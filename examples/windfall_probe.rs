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

use pixel_physics::lab::scene::LabBox;
use pixel_physics::render::Renderer;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::creature::{birth_cost, diet_yield, food_value, reproduce_at};
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::frame;
use pixel_physics::sim::organism::{self, CellType, TRAIT_GUT_BIAS};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;

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
}

fn census(world: &World, ground_y: i32, width: i32, windfall_id: Option<pixel_physics::sim::material::MaterialId>) -> Sample {
    let mut s = Sample {
        windfall: 0, windfall_floor: 0, fruit: 0, flower: 0,
        ant_high: 0, ants_aloft: 0, ants: 0, organ_low: i32::MAX, organ_high: 0,
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

fn main() {
    let frames: u64 = arg("frames").unwrap_or(24_000);
    let sample_every: u64 = arg("sample").unwrap_or(30);
    let gut: f32 = arg("gut").unwrap_or(f32::NAN);
    let handout: u64 = arg("handout").unwrap_or(0);
    // **A picture of the bed at the end**, because "a colony that breeds
    // eats the stand" is a judge-by-eye claim and this project's rule is to
    // post the artifact rather than describe it. The counts that go beside
    // it are the `SUMMARY` line below.
    let png: String = arg("png").unwrap_or_default();
    let spec = LabBox {
        founders: arg("founders").unwrap_or(8),
        colonies: arg("colonies").unwrap_or(1),
        compartments: arg("walls").unwrap_or(1),
        seed: arg("seed").unwrap_or(1),
        ..LabBox::default()
    };
    // **The bed is built with no ants in it, and the colonies are founded
    // afterwards at the same columns `LabBox` would have used.** An ant's
    // `gut_bias` is read off the *organism* (`creature::gut_of`), and
    // `place_creature` copies it from the species def at the moment of
    // placement — so a species-level write after the founders are standing
    // reaches nobody, and the run measures the neutral gut while the header
    // says otherwise. `stamp_probe` records paying for exactly that failure.
    // Deferring the colony is the same scene, one step later.
    let bare = LabBox { colonies: 0, ..spec.clone() };
    let (mut world, mut placed) = bare.build_counted();

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
    for x in spec.colony_columns() {
        placed.ants += world.found_colony(x, spec.ground_y - 2);
    }
    let def = world.species.get(world.species.id_of("ant").expect("ant")).creature.clone().expect("creature");
    let founder_gut = world
        .live_organism_ids()
        .iter()
        .filter_map(|id| world.organism(*id))
        .find(|s| world.species.get(s.species).creature.is_some())
        .map(|s| s.traits[TRAIT_GUT_BIAS]);

    let windfall_id = world.materials.id_of("windfall");
    let bar = birth_cost(&def);

    println!(
        "windfall probe: {frames} frames, sample every {sample_every} | founders {}/{} ants {} colonies {} seed {} handout {handout}",
        placed.planted, placed.asked, placed.ants, spec.colonies, spec.seed
    );
    println!(
        "  ant: start_energy {:.0} hunger_fraction {:.2} body_energy {:.0} x {} cells | bar {bar:.0} (buds at {:.0}) | gut {:+.2} (founder reads {})",
        def.start_energy,
        def.hunger_fraction,
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
            println!(
                "    {name:<9} face {:>6.0}  to this gut {y:>6.0}  -> bank ceiling {:>6.0} against a bar of {bar:.0}  {}",
                food_value(&world, cell),
                def.hunger_fraction * def.start_energy + y,
                if def.hunger_fraction * def.start_energy + y >= bar + 1.0 { "PAYS FOR A CHILD" } else { "short" },
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
    let mut handed_out = 0u64;
    // The colony's own columns, so a handout lands where the ants are rather
    // than somewhere they would first have to find.
    let colony_cols = spec.colony_columns();

    for f in 0..=frames {
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
        }
        if handout > 0 && f > 0 && f % handout == 0 {
            if let Some(wid) = windfall_id {
                let x = colony_cols[(handed_out as usize) % colony_cols.len().max(1)];
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
    let produced = world.fruit_dropped as f64;
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
    println!("    windfalls created (fruit let go)       {}", world.fruit_dropped);
    println!("    handed out by this harness             {handed_out}");
    println!("    mean standing: flower {:.1}  fruit {:.1}  windfall {mean_wf:.2} ({mean_wf_floor:.2} on the floor)",
        flower_sum as f64 / samples.max(1) as f64, fruit_sum as f64 / samples.max(1) as f64);
    println!("    peak standing windfall {wf_max} ({wf_floor_max} on the floor)");
    println!(
        "    organ height above the soil: lowest ever {} rows, highest ever {organ_high_max} rows | ants aloft, mean {:.2} of {} at the end",
        if organ_low_min == i32::MAX { "none stood".to_string() } else { format!("{organ_low_min}") },
        aloft_sum as f64 / samples.max(1) as f64,
        live_ants,
    );
    println!("    mean time a windfall stands: {life}  (on the floor: {life_floor})");
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
         dropped={} handed={handed_out} meanwf={mean_wf:.3} meanwffloor={mean_wf_floor:.3} maxwf={wf_max} \
         organlow={organ_low_min} organhigh={organ_high_max} aloft={:.2} \
         deliveries={} larder={} \
         births={} deaths={} liveants={live_ants} plants={plants} gen={deepest} eats={} bestoffer={:.0} bestbite={:.0} peakbank={:.0} bar={bar:.0} anthigh={ant_high_max}",
        spec.seed, placed.planted, placed.ants, world.fruit_dropped,
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
