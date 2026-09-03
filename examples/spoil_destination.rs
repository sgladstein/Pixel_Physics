//! **Where does the soil an ant digs actually end up — and does a tree's
//! root system keep growing once a colony has worked the ground under it?**
//!
//! Built from an owner playtest report, 2026-09-03: *"ants were constantly
//! digging under plants... they make big holes under each tree and pile up
//! dirt on top of and around the trees"*, and separately *"once they dig
//! under the tree roots cannot continue to grow"*. Two claims about the same
//! scene, so they are measured in the same scene.
//!
//! ```text
//! cargo run --release --example spoil_destination
//! cargo run --release --example spoil_destination -- seeds=8 frames=12000
//! cargo run --release --example spoil_destination -- grow=8000 ants=52
//! ```
//!
//! # Why the counters are split the way they are
//!
//! `act` has **two** places a pellet of spoil can go: an 8-neighbour of the
//! animal, or — if none of those will hold one — the first cell that will,
//! scanning as far as `SPOIL_LIFT` (160) rows straight up, with no check
//! that a path exists. `CreatureStats::spoil_dumped` counts both, so it
//! cannot answer this question at all: a colony laying tailings beside
//! itself and a colony posting them up through a canopy are the same number.
//! `spoil_lifted` and `spoil_lift_max` split them, and this probe is what
//! they were added for.
//!
//! The world census is the far side of that counter, which `CLAUDE.md`
//! requires: a lift that fired says a pellet moved, and only a census of
//! where `packedsoil` is standing says where it came to rest. **`packedsoil`
//! is a clean tracer** — soil `packs_into` it, and the only two writers are
//! the dig's own pellet and `line_burrow`'s wall lining, both of them ants.
//! Nothing in worldgen or the CA makes any.
//!
//! # The three arms, and which control each one is
//!
//! | arm | what it is for |
//! |---|---|
//! | `tree+ants` | the reproduction |
//! | `ants` (no tree) | **the positive control for elevated spoil.** A mound beside a shaft is a few rows tall; if the tree arm puts spoil forty rows up and this one does not, the tree is what the height is about |
//! | `tree` (no ants) | **the control for the root claim.** Roots that stall in both arms stalled for their own reasons, and the ants are exonerated |
//!
//! Without the second arm a tall `packedsoil` reading says nothing: ants
//! build mounds over their own nests by design, and that is the behaviour
//! `SPOIL_HEADROOM` exists to produce. The question is only ever whether the
//! tree changes it.
//!
//! # Frame-0 assertions
//!
//! `burrow_probe`'s lesson, and it is not optional here: this scene has to
//! grow a tree before it means anything, and a run whose tree never
//! germinated reads exactly like a colony that destroyed one. Every arm
//! asserts its own preconditions — the tree arms that there is a crown and a
//! root system before the ants arrive, the ant arms that ants were actually
//! placed — and names what it found when they fail.

use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::world::World;
use pixel_physics::sim::{material, parallel, rng};

const WIDTH: i32 = 512;
const HEIGHT: i32 = 320;
/// The original ground surface. Everything in the census is reported as rows
/// above or below this line, so it is the one number the whole report reads
/// against.
const GROUND_Y: i32 = 150;
/// Rows of soil under `GROUND_Y`, over a stone floor. Deep enough that the
/// colony is never digging against bedrock — `labsoil` measured a lab colony
/// already using 35 rows of a 40-row bed, so a shallower bed would report the
/// floor rather than the behaviour.
const SOIL_ROWS: i32 = 90;
const TREE_X: i32 = 256;

/// Echoed in the header, per `CLAUDE.md`'s 3.5-hour study that produced
/// byte-identical logs from a binary predating its own knob.
fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// The app's frame order, not a scheduler-only loop.
///
/// `creature::tests::how_high_does_an_ant_climb` records what the short
/// version costs: stepping only the scheduler gives a tree no light, so it
/// never grows, and the experiment reports a climb of 0 against a tree that
/// is not there.
fn live(w: &mut World, frames: usize) {
    for _ in 0..frames {
        parallel::step(w);
        w.step_active_sites();
        w.step_fields();
        w.step_pheromones();
    }
}

fn bed(seed: u64) -> World {
    let mut w = World::new(Rect::new(0, 0, WIDTH - 1, HEIGHT - 1));
    w.seed = seed;
    let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");
    for x in 0..WIDTH {
        for y in GROUND_Y..(GROUND_Y + SOIL_ROWS) {
            // Field capacity, matching `filmstrip`'s forest scene: bone-dry
            // soil sits below the wilting point where `Absorb` correctly
            // credits nothing, which would stall the roots for a reason that
            // has nothing to do with ants.
            w.set(x, y, Cell::new(soil, (rng::jitter(x, y) * 255.0) as u8).with_aux(material::SOIL_FIELD_CAPACITY));
        }
        for y in (GROUND_Y + SOIL_ROWS)..(GROUND_Y + SOIL_ROWS + 6) {
            w.set(x, y, Cell::new(material::STONE, 0));
        }
    }
    w
}

fn count_material(w: &World, name: &str) -> usize {
    let Some(id) = w.materials.id_of(name) else { return 0 };
    let mut n = 0;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if w.get(x, y).material == id {
                n += 1;
            }
        }
    }
    n
}

/// Every standing `packedsoil` cell, as rows **above** `GROUND_Y` (so a
/// positive number is spoil sitting proud of the original surface and a
/// negative one is lining down inside the workings), plus how many of them
/// are touching living plant tissue.
struct SpoilCensus {
    heights: Vec<i32>,
    touching_plant: usize,
    /// `(rows above ground, x, touching plant)` for every standing pellet,
    /// so `dump=` can say *where* the high ones are. A height alone cannot
    /// distinguish spoil posted into a canopy from spoil on a mound that
    /// happens to be tall, and that is the whole question here.
    cells: Vec<(i32, i32, bool)>,
}

fn census_spoil(w: &World) -> SpoilCensus {
    let Some(packed) = w.materials.id_of("packedsoil") else {
        return SpoilCensus { heights: Vec::new(), touching_plant: 0, cells: Vec::new() };
    };
    let mut heights = Vec::new();
    let mut touching_plant = 0;
    let mut cells = Vec::new();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if w.get(x, y).material != packed {
                continue;
            }
            heights.push(GROUND_Y - y);
            // 8-neighbour, matching the neighbourhood the digger and the
            // pellet placement both use.
            let touches = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)]
                .iter()
                .any(|&(dx, dy)| w.materials.kind(w.get(x + dx, y + dy).material) == MaterialKind::Plant);
            if touches {
                touching_plant += 1;
            }
            cells.push((GROUND_Y - y, x, touches));
        }
    }
    SpoilCensus { heights, touching_plant, cells }
}

/// Deciles rather than a mean.
///
/// `spoil_curvature`'s own header makes this argument and it applies
/// unchanged: a population pinned at one end has a perfectly reasonable
/// mean, and the whole question here is about the tail.
fn quantiles(v: &mut [i32]) -> String {
    if v.is_empty() {
        return "none".into();
    }
    v.sort_unstable();
    let at = |q: f64| v[((v.len() - 1) as f64 * q) as usize];
    format!("p10 {:+} p50 {:+} p90 {:+} p99 {:+} max {:+}", at(0.10), at(0.50), at(0.90), at(0.99), v[v.len() - 1])
}

struct Arm {
    tree: bool,
    ants: bool,
}

struct Outcome {
    digs: u64,
    dumped: u64,
    lifted: u64,
    lift_max: u32,
    spoil_above: usize,
    spoil_total: usize,
    touching_plant: usize,
    heights: String,
    roots_before: usize,
    roots_after: usize,
    plant_before: usize,
    plant_after: usize,
    top: Vec<(i32, i32, bool)>,
}

fn run(arm: &Arm, seed: u64, grow: usize, frames: usize, ants: i32) -> Outcome {
    let mut w = bed(seed);

    if arm.tree {
        w.plant_tree(TREE_X, GROUND_Y - 1);
        live(&mut w, grow);
    }

    let roots_before = count_material(&w, "rootwood");
    let plant_before = (0..HEIGHT)
        .map(|y| (0..WIDTH).filter(|&x| w.materials.kind(w.get(x, y).material) == MaterialKind::Plant).count())
        .sum::<usize>();

    // **The scene check, made mechanical.** A tree arm whose seed never
    // germinated is a bare bed, and every number below it would read as "the
    // colony destroyed the tree".
    if arm.tree {
        assert!(
            roots_before > 0 && plant_before > 40,
            "seed={seed}: the tree arm grew no tree in {grow} frames ({plant_before} plant cells, {roots_before} root cells) \
             -- this is a scene fault, not a result"
        );
    }

    if arm.ants {
        let placed = w.found_colony_of(TREE_X, GROUND_Y - 1, "ant", ants);
        assert!(placed > 0, "seed={seed}: the colony placed no ants -- the scene is not showing what it claims to");
    }

    // Counters are read as a delta across the measured window, so the tree's
    // own growth phase cannot contribute to them.
    let base = w.creature_stats;
    live(&mut w, frames);

    let census = census_spoil(&w);
    let mut heights = census.heights.clone();
    let mut top = census.cells.clone();
    top.sort_unstable_by_key(|&(h, x, _)| (std::cmp::Reverse(h), x));
    top.truncate(12);
    Outcome {
        digs: w.creature_stats.digs - base.digs,
        dumped: w.creature_stats.spoil_dumped - base.spoil_dumped,
        lifted: w.creature_stats.spoil_lifted - base.spoil_lifted,
        lift_max: w.creature_stats.spoil_lift_max,
        spoil_above: census.heights.iter().filter(|&&h| h > 0).count(),
        spoil_total: census.heights.len(),
        touching_plant: census.touching_plant,
        heights: quantiles(&mut heights),
        roots_before,
        roots_after: count_material(&w, "rootwood"),
        plant_before,
        plant_after: (0..HEIGHT)
            .map(|y| (0..WIDTH).filter(|&x| w.materials.kind(w.get(x, y).material) == MaterialKind::Plant).count())
            .sum::<usize>(),
        top,
    }
}

fn main() {
    let seeds: u64 = arg("seeds").unwrap_or(4);
    let frames: usize = arg("frames").unwrap_or(9_000);
    let grow: usize = arg("grow").unwrap_or(8_000);
    let ants: i32 = arg("ants").unwrap_or(52);
    let dump: bool = arg::<i32>("dump").unwrap_or(0) != 0;

    println!(
        "spoil_destination: seeds={seeds} frames={frames} grow={grow} ants={ants} \
         world={WIDTH}x{HEIGHT} ground_y={GROUND_Y} soil_rows={SOIL_ROWS} tree_x={TREE_X}"
    );
    println!("  spoil heights are rows ABOVE the original surface: + is proud of the ground, - is down in the workings\n");

    for (label, arm) in [
        ("tree+ants", Arm { tree: true, ants: true }),
        ("ants     ", Arm { tree: false, ants: true }),
        ("tree     ", Arm { tree: true, ants: false }),
    ] {
        for seed in 1..=seeds {
            let o = run(&arm, seed, grow, frames, ants);
            println!(
                "{label} seed={seed} | digs {:5} dumped {:5} lifted {:5} ({:4.1}%) lift_max {:3} \
                 | spoil {:5} of which above ground {:5} touching plant {:4} | {} \
                 | roots {} -> {} | plant {} -> {}",
                o.digs,
                o.dumped,
                o.lifted,
                if o.dumped > 0 { 100.0 * o.lifted as f64 / o.dumped as f64 } else { 0.0 },
                o.lift_max,
                o.spoil_total,
                o.spoil_above,
                o.touching_plant,
                o.heights,
                o.roots_before,
                o.roots_after,
                o.plant_before,
                o.plant_after,
            );
            if dump && !o.top.is_empty() {
                let shown: Vec<String> = o
                    .top
                    .iter()
                    .map(|&(h, x, t)| format!("{:+}@dx{}{}", h, x - TREE_X, if t { "*" } else { "" }))
                    .collect();
                println!("      highest pellets (rows above ground @ x offset from the trunk, * = touching plant): {}", shown.join(" "));
            }
        }
        println!();
    }
}
