//! **Where does a seed land, why does it not start, and does spreading them
//! fix it?** — the reseeding funnel in the lab bed, stage by stage.
//!
//! The owner's report: *plants create lots of seeds, many germinate, but they
//! rarely grow into full plants — they drop under the parent, so they are
//! shaded, or piled on other seeds and not touching dirt, or they sprout
//! right beside the parent.* That is **four** claims about four different
//! stages, and every existing harness collapses them into one number. `labshot`
//! prints `seeds` and `orgs`; `windfall_probe` follows fruit to the floor for
//! the *ants*; `seedbed_probe` asks whether a seed can start on a named mat.
//! None of them says where a seed is relative to its parent, which of the two
//! germination gates is shut, or what a germinated seedling then dies of.
//!
//! ```text
//! cargo run --release --example reseed_probe -- frames=45000
//! cargo run --release --example reseed_probe -- frames=45000 scatter=1   # the positive control
//! cargo run --release --example reseed_probe -- frames=45000 colonies=1  # do the ants eat the crop
//! ```
//!
//! # `scatter=1` is the positive control, and it is the point of the binary
//!
//! `CLAUDE.md`'s worst-recurring failure is a number that is arithmetically
//! correct and answers a different question; its remedy is to construct the
//! case whose answer you already know. `scatter=1` takes every seed the world
//! creates, once, at the moment it appears, and sets it down on open ground at
//! a uniformly random column of the bed. Dispersal limitation is then gone and
//! **nothing else has changed** — same species, same economy, same light, same
//! germination gate, same decay clock. If the stand still does not spread, the
//! diagnosis was wrong and dispersal is not the block.
//!
//! # It reports the gate's inputs, not a re-implementation of its verdict
//!
//! For every standing seed the probe reads the three quantities
//! `plant::Behavior::Germinate` reads — is anything underneath, the ambient
//! light above (`plant::ambient_light_above`), and the plant-available water
//! in the cell below (`update::plant_available_fraction`, and whether that
//! cell holds water at all) — and prints them against the species' own
//! thresholds. Those are the numbers a fix would change. Re-deriving the
//! verdict here would make the probe agree with itself rather than with the
//! engine, which is the readout-is-a-function-of-the-thing-it-debugs trap.
//!
//! **`on a seed pile` is the owner's own hypothesis as a column.** A seed
//! resting on another seed or a windfall is the "big pile of seeds, not
//! touching dirt" case, and it is separable from plain shade only by asking
//! what the cell below is made of.

use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::frame;
use pixel_physics::sim::material::{self, MaterialId};
use pixel_physics::sim::organism::{self, Behavior, CellType};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::rng::Rng;
use pixel_physics::sim::update;
use pixel_physics::sim::world::World;
use std::collections::HashMap;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// A seed's own state at the moment it is looked at: which of the germination
/// gate's inputs is short, and what it is sitting on.
#[derive(Default, Clone, Copy)]
struct Seeds {
    total: usize,
    /// Nothing underneath — still in the air.
    falling: usize,
    /// Resting on a cell that holds no water at all (`water_capacity == 0`):
    /// stone, wood, litter, **another seed**. The gate reads bone dry here
    /// however wet the world is.
    on_dry_mat: usize,
    /// ...of which the mat is another seed or a windfall. The owner's pile.
    on_seed_pile: usize,
    /// ...of which the mat is living or dead plant tissue.
    on_plant: usize,
    /// Resting on something that holds water but is below the threshold.
    too_dry: usize,
    /// Water is fine, light is short.
    too_dark: usize,
    /// Every input satisfied — this one germinates on its next seed tick.
    ready: usize,
    /// Horizontal distance to the nearest cell of a *rooted* plant, bucketed
    /// 0-1 / 2-3 / 4-7 / 8-15 / 16+.
    near: [usize; 5],
}

/// The stand, split by how far a plant got.
#[derive(Default, Clone, Copy)]
struct Stand {
    /// Organisms holding a single ungerminated `Seed` cell.
    seeds: usize,
    /// Germinated, under `est` cells.
    seedlings: usize,
    /// Germinated, at or over `est` cells.
    established: usize,
    /// Cells over every germinated plant.
    cells: usize,
    biggest: usize,
    max_generation: u16,
    senescent: usize,
    /// Distinct columns holding rooted plant tissue -- the coverage question
    /// the cell count cannot answer, because a stand of 90 plants in one
    /// clump and a stand of 90 across the bed have the same cell count.
    columns: usize,
    /// Leftmost and rightmost rooted plant cell.
    span: (i32, i32),
    /// Rooted plants by how far their centroid sits from the nearest founder
    /// column: 0-3 / 4-7 / 8-15 / 16+. **A plant in the last bucket is one
    /// that reached ground its parents did not stand on.**
    from_founder: [usize; 4],
}

fn is_seed_material(world: &World, m: MaterialId, seed_id: Option<MaterialId>, windfall_id: Option<MaterialId>) -> bool {
    let _ = world;
    Some(m) == seed_id || Some(m) == windfall_id
}

fn census(world: &World, est: usize, seed_id: Option<MaterialId>, windfall_id: Option<MaterialId>, light_bar: f32, water_bar: f32, founders: &[i32]) -> (Stand, Seeds) {
    // `span` starts inverted so the first cell sets both ends; the rest of
    // the fields want their zeroes, hence the struct-update form (clippy
    // rejects assigning a field after `Default::default()`).
    let mut st = Stand { span: (i32::MAX, i32::MIN), ..Stand::default() };
    let mut cols: std::collections::HashSet<i32> = std::collections::HashSet::new();
    let mut sd = Seeds::default();
    // Rooted plant tissue, for the distance column. Collected first because
    // the distance question is asked of every seed against all of it.
    let mut tissue: Vec<(i32, i32)> = Vec::new();
    let mut seeds: Vec<(i32, i32)> = Vec::new();
    for id in world.live_organism_ids() {
        let Some(state) = world.organism(id) else { continue };
        if world.species.get(state.species).creature.is_some() {
            continue;
        }
        let only_seed = state.cells.len() == 1
            && state.cells.keys().all(|&(x, y)| organism::cell_type(world.get(x, y).aux()) == Some(CellType::Seed));
        if only_seed {
            st.seeds += 1;
            if let Some(&(x, y)) = state.cells.keys().next() {
                seeds.push((x, y));
            }
            continue;
        }
        st.cells += state.cells.len();
        st.biggest = st.biggest.max(state.cells.len());
        st.max_generation = st.max_generation.max(state.generation);
        st.senescent += usize::from(state.senescent);
        if state.cells.len() >= est {
            st.established += 1;
        } else {
            st.seedlings += 1;
        }
        let (mut sum_x, mut n) = (0i64, 0i64);
        for &(x, y) in state.cells.keys() {
            if organism::cell_type(world.get(x, y).aux()) != Some(CellType::Seed) {
                tissue.push((x, y));
                cols.insert(x);
                st.span.0 = st.span.0.min(x);
                st.span.1 = st.span.1.max(x);
                sum_x += x as i64;
                n += 1;
            }
        }
        if n > 0 {
            let cx = (sum_x / n) as i32;
            let d = founders.iter().map(|&fc| (fc - cx).abs()).min().unwrap_or(0);
            st.from_founder[match d {
                0..=3 => 0,
                4..=7 => 1,
                8..=15 => 2,
                _ => 3,
            }] += 1;
        }
    }
    st.columns = cols.len();
    for (x, y) in seeds {
        sd.total += 1;
        let below = world.get(x, y + 1);
        if below.material == material::EMPTY {
            sd.falling += 1;
        } else {
            let mat = world.materials.get(below.material);
            if mat.water_capacity == 0 {
                sd.on_dry_mat += 1;
                if is_seed_material(world, below.material, seed_id, windfall_id) {
                    sd.on_seed_pile += 1;
                } else if mat.kind == material::MaterialKind::Plant {
                    sd.on_plant += 1;
                }
            } else if update::plant_available_fraction(below) < water_bar {
                sd.too_dry += 1;
            } else if pixel_physics::sim::plant::ambient_light_above(world, x, y) < light_bar {
                sd.too_dark += 1;
            } else {
                sd.ready += 1;
            }
        }
        let d = tissue.iter().map(|&(tx, _)| (tx - x).abs()).min().unwrap_or(i32::MAX);
        let bucket = match d {
            0..=1 => 0,
            2..=3 => 1,
            4..=7 => 2,
            8..=15 => 3,
            _ => 4,
        };
        sd.near[bucket] += 1;
    }
    (st, sd)
}

/// The species' own germination thresholds, read off the loaded definition so
/// the probe's columns cannot drift from the `.ron`.
fn thresholds(world: &World, species: &str) -> (f32, f32) {
    let Some(id) = world.species.id_of(species) else { return (f32::NAN, f32::NAN) };
    for b in world.species.get(id).behaviors(CellType::Seed) {
        if let Behavior::Germinate { light_threshold, soil_water_threshold, .. } = b {
            return (*light_threshold, *soil_water_threshold);
        }
    }
    (f32::NAN, f32::NAN)
}

fn main() {
    let frames: u64 = arg("frames").unwrap_or(45_000);
    let every: u64 = arg("every").unwrap_or(1_500);
    let est: usize = arg("est").unwrap_or(12);
    let scatter: u32 = arg("scatter").unwrap_or(0);
    let species: String = std::env::args().find_map(|a| a.strip_prefix("species=").map(str::to_string)).unwrap_or_else(|| "herb".to_string());

    let spec = LabBox {
        founders: arg("founders").unwrap_or(8),
        colonies: arg("colonies").unwrap_or(0),
        species: species.clone(),
        seed: arg("seed").unwrap_or(1),
        ..LabBox::default()
    };
    let (mut world, mut placed) = spec.build_counted();
    // **`col=` moves the single founder to a named column.** `spread(1)` puts
    // one founder at the bed's centre, which at `lamp_spacing 64` is exactly
    // half way between two fixtures -- so a one-founder run is not a smaller
    // eight-founder run, it is the same experiment at the darkest column in
    // the bed. That is a scene difference and it changes the answer, so it
    // has to be steerable rather than inherited.
    let mut founder_cols = spec.founder_columns();
    if let Some(col) = arg::<i32>("col") {
        for id in world.live_organism_ids() {
            let cells: Vec<(i32, i32)> = world.organism(id).map(|s| s.cells.keys().copied().collect()).unwrap_or_default();
            for (x, y) in cells {
                world.set(x, y, Cell::EMPTY);
            }
        }
        placed.planted = usize::from(world.plant_tree_species(col, spec.ground_y - 2, &species));
        founder_cols = vec![col];
    }
    let seed_id = world.materials.id_of("seed");
    let windfall_id = world.materials.id_of("windfall");
    let (light_bar, water_bar) = thresholds(&world, &species);

    // **The harness echoes its own parameters** -- a log that does not name
    // its arm was written by a binary that never had one (`CLAUDE.md`).
    println!(
        "reseed probe: {species} | {frames} frames, sample every {every} | founders {}/{} at {:?} | colonies {} ants {} | world seed {} | scatter {scatter} | established >= {est} cells",
        placed.planted,
        placed.asked,
        founder_cols,
        spec.colonies,
        placed.ants,
        spec.seed,
        );
    println!("  germination gate: light >= {light_bar:.3}, plant-available soil water >= {water_bar:.3}, and something underneath");

    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    // Its own stream, so the control's placement draws cannot be confused
    // with anything the world does.
    let mut rng = Rng::new(spec.seed ^ 0x5ca7_7e12);
    let mut moved: std::collections::HashSet<u16> = std::collections::HashSet::new();
    let mut scattered = 0u64;
    // `seeds_set` lives on the parent and dies with it, so the running total
    // is a delta walk. Sampled ten times per report so a parent that dies
    // between reports still has its last count counted.
    let mut seen: HashMap<u16, u32> = HashMap::new();
    let mut seeds_set_total: u64 = 0;
    // **The independent counter, from the far side of the call.** The delta
    // walk over `seeds_set` above disagreed with `World::germinations` by
    // more than 2x on one arm -- 79 against 164, which is impossible -- so
    // one of them is wrong and neither can be believed alone. This one
    // counts the *effect*: an organism id that was not live last frame and
    // is now a single `CellType::Seed` cell is a seed that has just been
    // created, whoever made it. `CLAUDE.md`: pair every "it fired" counter
    // with an effect counter from the far side.
    let mut live_last: std::collections::HashSet<u16> = std::collections::HashSet::new();
    let mut seeds_born: u64 = 0;

    let interior = (spec.width / 16, spec.width - spec.width / 16);

    // **The light profile along the bench, once the field has settled.**
    // Printed because a one-founder run and an eight-founder run put their
    // founders at different columns, and `spread(1)`'s centre column is the
    // darkest place in the bed -- which is a scene difference that decides
    // the answer, not a detail. Read against the germination bar above.
    let profile_at = arg::<u64>("profile").unwrap_or(600);
    for f in 0..=frames {
        if f == profile_at {
            let mut line = String::new();
            for x in (0..spec.width).step_by(8) {
                line.push_str(&format!("{:.2} ", pixel_physics::sim::plant::ambient_light_above(&world, x, spec.ground_y - 2)));
            }
            println!("  bench light at f={f}, every 8 columns from 0: {line}");
        }
        if scatter > 0 {
            // Every new single-cell seed organism, once, on to open ground at
            // a uniformly random column. Two `World::set` calls: that is the
            // one seam that keeps an organism's cell list correct.
            for id in world.live_organism_ids() {
                if moved.contains(&id) {
                    continue;
                }
                let Some(state) = world.organism(id) else { continue };
                if world.species.get(state.species).creature.is_some() || state.cells.len() != 1 {
                    continue;
                }
                let Some(&(sx, sy)) = state.cells.keys().next() else { continue };
                if organism::cell_type(world.get(sx, sy).aux()) != Some(CellType::Seed) {
                    continue;
                }
                moved.insert(id);
                let tx = interior.0 + rng.below((interior.1 - interior.0) as u32) as i32;
                // Lowest empty cell in that column above the soil: the seed is
                // set down on the ground rather than dropped, so travel time
                // is not part of the arm.
                let mut ty = None;
                for y in (0..spec.ground_y + 4).rev() {
                    if world.get(tx, y).material == material::EMPTY && world.get(tx, y + 1).material != material::EMPTY {
                        ty = Some(y);
                        break;
                    }
                }
                let Some(ty) = ty else { continue };
                if (tx, ty) == (sx, sy) {
                    continue;
                }
                let cell = world.get(sx, sy);
                world.set(sx, sy, Cell::EMPTY);
                world.set(tx, ty, cell);
                scattered += 1;
            }
        }
        // **Every frame, and that is a correction rather than caution.**
        // Sampled every `every/10` frames this undercounts badly: `seeds_set`
        // lives on the parent and a herb is an annual, so a plant that sets
        // seed and dies inside the gap takes its whole count with it. The
        // tell fired on its own -- one arm reported **164 germinations from
        // 79 seeds set**, which is impossible -- and it is `CLAUDE.md`'s
        // *ask what your number counts*: the ratio was a ratio of an exact
        // numerator to a lossy denominator.
        {
            for id in world.live_organism_ids() {
                let Some(state) = world.organism(id) else { continue };
                let e = seen.entry(id).or_insert(0);
                if state.seeds_set >= *e {
                    seeds_set_total += (state.seeds_set - *e) as u64;
                } else {
                    seeds_set_total += state.seeds_set as u64;
                }
                *e = state.seeds_set;
            }
        }
        {
            let mut live_now: std::collections::HashSet<u16> = std::collections::HashSet::new();
            for id in world.live_organism_ids() {
                live_now.insert(id);
                if live_last.contains(&id) {
                    continue;
                }
                let Some(state) = world.organism(id) else { continue };
                if state.cells.len() == 1
                    && state.cells.keys().all(|&(x, y)| organism::cell_type(world.get(x, y).aux()) == Some(CellType::Seed))
                {
                    seeds_born += 1;
                }
            }
            live_last = live_now;
        }
        if f % every == 0 {
            let (st, sd) = census(&world, est, seed_id, windfall_id, light_bar, water_bar, &founder_cols);
            println!(
                "  f {f:>6} | plants {:>3} (est {:>3}, seedling {:>3}, senescent {:>2}) cells {:>5} biggest {:>4} gen {:>2} \
                 | seeds {:>4} [air {:>3} dry-mat {:>3} (pile {:>3}, plant {:>3}) too-dry {:>3} dark {:>3} ready {:>3}] \
                 | to plant 0-1 {:>3} 2-3 {:>3} 4-7 {:>3} 8-15 {:>3} 16+ {:>3} \
                 | cols {:>3} span {:>3}-{:>3} from founder 0-3 {:>3} 4-7 {:>3} 8-15 {:>3} 16+ {:>3} \
                 | born {:>5} set {:>5} germ {:>5} fruit {:>4} | shed shade {:>5} drought {:>5} | refused {}",
                st.seedlings + st.established,
                st.established,
                st.seedlings,
                st.senescent,
                st.cells,
                st.biggest,
                st.max_generation,
                sd.total,
                sd.falling,
                sd.on_dry_mat,
                sd.on_seed_pile,
                sd.on_plant,
                sd.too_dry,
                sd.too_dark,
                sd.ready,
                sd.near[0],
                sd.near[1],
                sd.near[2],
                sd.near[3],
                sd.near[4],
                st.columns,
                if st.span.0 == i32::MAX { -1 } else { st.span.0 },
                if st.span.1 == i32::MIN { -1 } else { st.span.1 },
                st.from_founder[0],
                st.from_founder[1],
                st.from_founder[2],
                st.from_founder[3],
                world.seeds_borne,
                seeds_set_total,
                world.germinations,
                world.fruit_dropped,
                world.shed_shade,
                world.shed_drought,
                world.organisms_refused(),
            );
        }
        if f < frames {
            frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        }
    }

    let (st, _) = census(&world, est, seed_id, windfall_id, light_bar, water_bar, &founder_cols);
    println!(
        "FUNNEL {species} scatter={scatter} colonies={} seed={}: seed cells born {} (id-keyed undercount {seeds_born}, parents claim {seeds_set_total}, fruit {}) -> germinated {} -> alive {} (established {}) \
         | columns held {}/{} span {}-{} | plants 16+ cells from a founder column: {} | scattered {scattered}",
        spec.colonies,
        spec.seed,
        world.seeds_borne,
        world.fruit_dropped,
        world.germinations,
        st.seedlings + st.established,
        st.established,
        st.columns,
        spec.width,
        if st.span.0 == i32::MAX { -1 } else { st.span.0 },
        if st.span.1 == i32::MIN { -1 } else { st.span.1 },
        st.from_founder[3],
    );
}
