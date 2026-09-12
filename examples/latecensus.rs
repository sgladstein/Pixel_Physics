//! **The late-game census** -- what the stand, the larder, the colony and the
//! nest's footprint each do on the played bed over a session and beyond.
//!
//! Built for `Reports/evolution-lab-late-game-design-2026-09-12.md` brief 0.
//! The owner's report is about the *late* game -- "when colonies got up to
//! hundreds of creatures ... they decimate all the plants, the food
//! disappears and then the colony dies", and "they dig large chambers
//! underground and piles of dirt/chambers above the nest, which creates an
//! area where plants don't grow". `labforage` censuses the larder and
//! `labnest` the nest, and neither runs the played bed to 500,000 frames
//! with both halves on one row. This does, and only that: one line per
//! stop, every column on it, so a boom, a crash and a dead zone can each be
//! dated to a frame.
//!
//! Three things on a row nothing else prints:
//!
//! * **the larder by kind** -- edible joules split into leaf, fruit (fruit +
//!   windfall), litter, seed (seed + pip), corpse and flower, at the
//!   founders' own gut through `creature::diet_yield`, so "what could a
//!   colony on fruit and carrion alone live on" is read off the bed rather
//!   than argued;
//! * **the nest's footprint** -- `roofed` void below the original surface
//!   (chambers), `pit` (void open to the sky), `packed` cells above and
//!   below the surface (tunnel lining and tamped spoil), `mound` (soil,
//!   packed soil or nest material standing above the original surface) and
//!   its highest row;
//! * **the dead zone** -- columns within `BAND` of a nest with no plant cell
//!   at all, against the same count outside the band, so "plants don't grow
//!   above the nest" is a ratio and not an impression.
//!
//! ```text
//! cargo run --release --example latecensus -- scenario=played_bed frames=500000 sample=20000 seed=1
//! cargo run --release --example latecensus -- scenario=played_bed frames=500000 sample=20000 seed=1 no_colony=1
//! cargo run --release --example latecensus -- control=selftest
//! ```
//!
//! `control=selftest` carves a known roofed cavity, a known open pit and a
//! known mound into a bare box and asserts each column reports exactly it --
//! `aloft` and `unvisited` were the two silently-always-zero columns in
//! `labforage`, and `roofed`/`pit`/`mound` are this file's equivalents.

use pixel_physics::lab::scenario::{Placement, Scenario};
use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::creature::{diet_yield, EAT_YIELD_THRESHOLD};
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::frame;
use pixel_physics::sim::material::{self, MaterialId, MaterialKind};
use pixel_physics::sim::organism::{self, DeathCause, TRAIT_GUT_BIAS};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

/// Half-width of the nest band, in columns, for the dead-zone ratio. The
/// played bed founds in the open ground 210..310 and a colony's home range
/// on it is a few tens of columns either side of the door, so 64 covers the
/// mound and the traffic without reaching the far stands.
const BAND: i32 = 64;

/// How far above the original surface ground still counts as the bed's own
/// (a mound, a heap of spoil) rather than the box's lid or a lamp.
const MOUND_REACH: i32 = 48;

/// The material ids the census buckets by, resolved once.
struct Ids {
    leaf: Vec<MaterialId>,
    fruit: Vec<MaterialId>,
    litter: Vec<MaterialId>,
    seed: Vec<MaterialId>,
    corpse: Vec<MaterialId>,
    flower: Vec<MaterialId>,
    ground: Vec<MaterialId>,
    packed: Option<MaterialId>,
}

impl Ids {
    fn resolve(world: &World) -> Self {
        let ids = |names: &[&str]| names.iter().filter_map(|n| world.materials.id_of(n)).collect::<Vec<_>>();
        Ids {
            leaf: ids(&["leaf", "grassblade", "moss"]),
            fruit: ids(&["fruit", "windfall"]),
            litter: ids(&["litter", "deadleaf"]),
            seed: ids(&["seed", "pip"]),
            corpse: ids(&["corpse"]),
            flower: ids(&["flower"]),
            ground: ids(&["soil", "packedsoil", "nest"]),
            packed: world.materials.id_of("packedsoil"),
        }
    }
}

#[derive(Default, Clone, Copy, Debug)]
struct Sample {
    ants: usize,
    plants: usize,
    seed_bank: usize,
    edible: usize,
    worth: f64,
    leaf_j: f64,
    fruit_j: f64,
    litter_j: f64,
    seed_j: f64,
    corpse_j: f64,
    flower_j: f64,
    other_j: f64,
    standing_flowers: usize,
    /// Void below the original surface with ground somewhere above it in
    /// the same column -- a chamber or a gallery.
    roofed: usize,
    /// Void below the original surface open to the sky -- a pit.
    pit: usize,
    packed_below: usize,
    packed_above: usize,
    /// Soil, packed soil or nest cells standing above the original surface.
    mound: usize,
    /// Rows above the original surface of the highest such cell.
    mound_high: i32,
    /// Columns within `BAND` of a nest holding no plant cell at all, and the
    /// band's width; the same outside it.
    bare_in_band: usize,
    band_cols: usize,
    bare_outside: usize,
    outside_cols: usize,
    plant_cells_in_band: usize,
    plant_cells_outside: usize,
}

fn is_waiting_seed(world: &World, state: &organism::OrganismState) -> bool {
    state.cells.len() == 1
        && state
            .cells
            .keys()
            .next()
            .map(|(x, y)| organism::cell_type(world.get(*x, *y).aux()) == Some(organism::CellType::Seed))
            .unwrap_or(false)
}

fn census(world: &World, spec: &LabBox, gut: f32, nest_cols: &[i32], ids: &Ids) -> Sample {
    let mut s = Sample { mound_high: 0, ..Sample::default() };
    for id in world.live_organism_ids() {
        let Some(state) = world.organism(id) else { continue };
        if world.species.get(state.species).creature.is_some() {
            s.ants += 1;
        } else if is_waiting_seed(world, state) {
            s.seed_bank += 1;
        } else {
            s.plants += 1;
        }
    }
    let mut has_plant = vec![false; spec.width as usize];
    let mut plant_cells = vec![0usize; spec.width as usize];
    for x in 0..spec.width {
        let mut covered = false;
        for y in 0..spec.height {
            let cell = world.get(x, y);
            let kind = world.materials.kind(cell.material);
            let is_ground = cell.material != material::EMPTY
                && matches!(kind, MaterialKind::Powder | MaterialKind::Solid)
                && cell.organism_id() == 0;
            if kind == MaterialKind::Plant || (cell.organism_id() != 0 && world.organism(cell.organism_id()).is_some_and(|st| world.species.get(st.species).creature.is_none())) {
                has_plant[x as usize] = true;
                plant_cells[x as usize] += 1;
            }
            // **Cover is ground near the surface, not the lid.** The lab box
            // is sealed, so row 0 is a wall in every column and a plain
            // "anything solid above" read the whole bed as roofed (selftest:
            // a shaft open to the sky counted as a chamber). A mound is a few
            // rows; the lid is 160 up.
            if is_ground && y >= spec.ground_y - MOUND_REACH {
                covered = true;
                if y < spec.ground_y && ids.ground.contains(&cell.material) {
                    s.mound += 1;
                    s.mound_high = s.mound_high.max(spec.ground_y - y);
                }
                if ids.packed == Some(cell.material) {
                    if y < spec.ground_y {
                        s.packed_above += 1;
                    } else {
                        s.packed_below += 1;
                    }
                }
            } else if y >= spec.ground_y && world.is_empty(x, y) {
                if covered {
                    s.roofed += 1;
                } else {
                    s.pit += 1;
                }
            }
            if ids.flower.contains(&cell.material) {
                s.standing_flowers += 1;
            }
            let yielded = diet_yield(world, cell, gut);
            if yielded <= EAT_YIELD_THRESHOLD {
                continue;
            }
            if world.organism(cell.organism_id()).is_some_and(|st| world.species.get(st.species).creature.is_some()) {
                continue;
            }
            s.edible += 1;
            s.worth += yielded as f64;
            let m = cell.material;
            let j = yielded as f64;
            if ids.leaf.contains(&m) {
                s.leaf_j += j;
            } else if ids.fruit.contains(&m) {
                s.fruit_j += j;
            } else if ids.litter.contains(&m) {
                s.litter_j += j;
            } else if ids.seed.contains(&m) {
                s.seed_j += j;
            } else if ids.corpse.contains(&m) {
                s.corpse_j += j;
            } else if ids.flower.contains(&m) {
                s.flower_j += j;
            } else {
                s.other_j += j;
            }
        }
    }
    for x in 0..spec.width {
        let d = nest_cols.iter().map(|c| (c - x).abs()).min().unwrap_or(i32::MAX);
        let bare = !has_plant[x as usize];
        if d <= BAND {
            s.band_cols += 1;
            s.plant_cells_in_band += plant_cells[x as usize];
            if bare {
                s.bare_in_band += 1;
            }
        } else {
            s.outside_cols += 1;
            s.plant_cells_outside += plant_cells[x as usize];
            if bare {
                s.bare_outside += 1;
            }
        }
    }
    s
}

fn strip_colony(world: &mut World) -> usize {
    let mut cleared = 0usize;
    for id in world.live_organism_ids() {
        let Some(state) = world.organism(id) else { continue };
        if world.species.get(state.species).creature.is_none() {
            continue;
        }
        let positions: Vec<(i32, i32)> = state.cells.keys().copied().collect();
        for (x, y) in positions {
            world.set(x, y, Cell::EMPTY);
        }
        cleared += 1;
    }
    cleared
}

fn ant_gut_bias(world: &World) -> f32 {
    world
        .live_organism_ids()
        .iter()
        .filter_map(|id| world.organism(*id))
        .find(|s| world.species.get(s.species).creature.is_some())
        .map(|s| s.traits[TRAIT_GUT_BIAS])
        .unwrap_or(0.0)
}

/// The colony species' deaths by cause, colonies rolled up.
fn colony_deaths(world: &World, species: &str) -> (u64, u64, u64) {
    let (mut starved, mut killed, mut other) = (0, 0, 0);
    for g in &world.group_deaths {
        if world.species.get(g.species).name != species {
            continue;
        }
        for (i, n) in g.by_cause.iter().enumerate() {
            if i == DeathCause::Starved.index() || i == DeathCause::StarvedInFlight.index() {
                starved += n;
            } else if i == DeathCause::Killed.index() {
                killed += n;
            } else {
                other += n;
            }
        }
    }
    (starved, killed, other)
}

fn selftest() {
    let spec = LabBox { colonies: 0, founders: 0, ..LabBox::default() };
    let mut world = spec.build();
    let ids = Ids::resolve(&world);
    let base = census(&world, &spec, 0.0, &[spec.width / 2], &ids);
    println!("latecensus selftest: bare box reads roofed {} pit {} mound {} edible {} (all must be 0)", base.roofed, base.pit, base.mound, base.edible);
    assert_eq!((base.roofed, base.pit, base.mound, base.edible), (0, 0, 0, 0), "a bare box must read nothing");
    assert_eq!(base.bare_in_band, base.band_cols, "a bare box has no plant in any column");
    // A 3x3 chamber ten rows under the surface, roofed by the soil above it.
    let (cx, cy) = (spec.width / 2, spec.ground_y + 10);
    for dy in 0..3 {
        for dx in 0..3 {
            world.set(cx + dx, cy + dy, Cell::EMPTY);
        }
    }
    // A five-deep shaft from the surface, open to the sky, twenty columns away.
    for dy in 0..5 {
        world.set(cx + 20, spec.ground_y + dy, Cell::EMPTY);
    }
    // A two-high heap of packed soil on the surface, thirty columns away.
    let packed = ids.packed.expect("packedsoil is registered");
    world.set(cx + 30, spec.ground_y - 1, Cell::new(packed, 0));
    world.set(cx + 30, spec.ground_y - 2, Cell::new(packed, 0));
    let s = census(&world, &spec, 0.0, &[cx], &ids);
    println!(
        "  carved: roofed {} (must be 9) pit {} (must be 5) mound {} (must be 2) high {} (must be 2) packed_above {} (must be 2)",
        s.roofed, s.pit, s.mound, s.mound_high, s.packed_above
    );
    assert_eq!(s.roofed, 9, "a 3x3 chamber under intact soil is nine roofed cells");
    assert_eq!(s.pit, 5, "a five-deep shaft open to the sky is five pit cells");
    assert_eq!((s.mound, s.mound_high, s.packed_above), (2, 2, 2), "two packed cells on the surface are a two-high mound");
    println!("latecensus selftest: PASS -- every footprint column moves for a case whose answer is known");
}

fn main() {
    let control: String = arg("control").unwrap_or_else(|| "run".to_string());
    if control == "selftest" {
        selftest();
        return;
    }
    let frames: u64 = arg("frames").unwrap_or(500_000);
    // **`no_colony=1` -- the paired unfed control**, `labforage`'s own shape:
    // the scenario's timeline still founds the colony (nothing on the
    // scenario side can suppress it), and this clears every creature's cells
    // the frame they land, so the bed's own production -- litter shed, seeds
    // borne, fruit dropped -- is read with nothing eating it.
    let no_colony: bool = arg::<u32>("no_colony").unwrap_or(0) != 0;
    let sample_every: u64 = arg("sample").unwrap_or(20_000);
    let scenario_name: String = arg("scenario").unwrap_or_else(|| "played_bed".to_string());
    let mut scenario = Scenario::load(&scenario_name).unwrap_or_else(|e| {
        eprintln!("scenario {scenario_name}: {e}");
        std::process::exit(2);
    });
    if let Some(sd) = arg::<u64>("seed") {
        scenario.bed.seed = sd;
    }
    let spec = scenario.bed.clone();
    println!(
        "latecensus: scenario={} seed={} frames={frames} sample={sample_every} band={BAND} threads={}",
        scenario.name,
        spec.seed,
        std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "default".into())
    );
    let (mut world, planted, placed) = scenario.build();
    println!(
        "  bed: {} of {} founders planted; scenario placed {} cells, {} plants, {} animals",
        planted.planted, planted.asked, placed.cells, placed.plants, placed.animals
    );
    let nest_cols: Vec<i32> = {
        let mut v: Vec<i32> = scenario
            .placements
            .iter()
            .chain(scenario.timeline.iter().map(|e| &e.what))
            .filter_map(|p| match p {
                Placement::Colony { x, .. } => Some(*x),
                _ => None,
            })
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    println!("  nests at {nest_cols:?}; band = +-{BAND} columns of a nest\n");
    let ids = Ids::resolve(&world);
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    let mut gut = 0.0f32;
    println!(
        "{:>7} {:>5} {:>5} {:>5} {:>6} {:>9} | {:>8} {:>7} {:>7} {:>6} {:>6} {:>7} {:>7} {:>5} | {:>5} {:>5} {:>5} {:>5} {:>4} {:>6} {:>6} {:>7} | {:>6} {:>5} {:>6} {:>6} {:>4} | {:>4}/{:<3} {:>4}/{:<3} {:>6} {:>6}",
        "frame", "ants", "plnts", "bank", "edible", "worth(J)",
        "leafJ", "fruitJ", "littrJ", "seedJ", "crpsJ", "flowrJ", "otherJ", "flwrs",
        "born", "died", "strvd", "killd", "othr", "eats", "digs", "delivs",
        "roofed", "pit", "pack<", "pack^", "mnd", "bare", "band", "bare", "out", "pcIn", "pcOut"
    );
    println!("        (then, cumulative production: shed=leaves shed, borne=seeds borne, germ=germinations, fdrop=fruit dropped)");
    for f in 0..=frames {
        let arrived = pixel_physics::lab::scenario::tick_timeline(&scenario, &mut world, &spec);
        if arrived.animals > 0 {
            gut = ant_gut_bias(&world);
            println!("  frame {f}: {} animal(s) arrived on the timeline, founder gut_bias {gut}", arrived.animals);
            if no_colony {
                let cleared = strip_colony(&mut world);
                println!("  frame {f}: no_colony=1, cleared {cleared} colony animal(s) right back off the bed");
            }
        }
        if f % sample_every == 0 {
            let s = census(&world, &spec, gut, &nest_cols, &ids);
            let st = world.creature_stats;
            let (starved, killed, other) = colony_deaths(&world, &spec.colony_species);
            println!(
                "{f:>7} {:>5} {:>5} {:>5} {:>6} {:>9.0} | {:>8.0} {:>7.0} {:>7.0} {:>6.0} {:>6.0} {:>7.0} {:>7.0} {:>5} | {:>5} {:>5} {:>5} {:>5} {:>4} {:>6} {:>6} {:>7} | {:>6} {:>5} {:>6} {:>6} {:>4} | {:>4}/{:<3} {:>4}/{:<3} {:>6} {:>6}",
                s.ants, s.plants, s.seed_bank, s.edible, s.worth,
                s.leaf_j, s.fruit_j, s.litter_j, s.seed_j, s.corpse_j, s.flower_j, s.other_j, s.standing_flowers,
                st.births, st.deaths, starved, killed, other, st.eats, st.digs, st.deliveries,
                s.roofed, s.pit, s.packed_below, s.packed_above, s.mound_high,
                s.bare_in_band, s.band_cols, s.bare_outside, s.outside_cols, s.plant_cells_in_band, s.plant_cells_outside
            );
            // **Production, cumulative, so a rate is a difference of two
            // rows.** The standing larder above is stock, which is
            // production times residence, and only the rate says what a
            // colony living on the bed's surplus could sustain.
            println!(
                "        shed={} borne={} germ={} fdrop={}",
                world.shed_shade as u64 + world.shed_drought as u64 + world.shed_stranded as u64,
                world.seeds_borne,
                world.germinations,
                world.fruit_dropped
            );
        }
        if f < frames {
            frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        }
    }
    let st = world.creature_stats;
    println!(
        "\nSUMMARY scenario={} seed={} frames={frames} born={} died={} eats={} digs={} spoil_dumped={} deliveries={} nectar_paid={:.0}",
        scenario.name, spec.seed, st.births, st.deaths, st.eats, st.digs, st.spoil_dumped, st.deliveries, world.nectar_paid
    );
}
