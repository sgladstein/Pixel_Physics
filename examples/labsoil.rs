//! **What does deepening the lab's soil cost the crop?** — the real
//! `lab::scene::LabBox` bed at several soil depths across many world seeds,
//! censused at equal *ticks*, with the paired per-seed direction statistic
//! the answer actually turns on.
//!
//! # Why this is a harness and not a `labshot` argument
//!
//! `labshot` already prints every column this one does — cells, biggest,
//! roots reach, light at the bench — and it takes `soil=`. What it does not
//! take is a **world seed**, and one seed is not an answer here: `CLAUDE.md`
//! records six seeds reading 1.64x and the next twelve reading 1.08x on one
//! change, pooling to a per-seed median of zero. A single-seed reading that
//! deeper soil costs the stand is a sample from a wide distribution, and the
//! whole point of this file is to say whether that sample was the
//! distribution or the draw. It also renders nothing: the machine this runs
//! on has other agents compiling on it, and `CLAUDE.md`'s standing rule is
//! that counters survive a loud box where a clock does not.
//!
//! # Equal ticks, not equal wall time
//!
//! Deeper soil is 2.4x the soil cells, and `labbox_cost` measured 40 -> 240
//! rows at **1.9x the frame**. So a study that gave each arm the same
//! *seconds* would hand the deep arm fewer ticks and then report a smaller
//! stand — a performance result wearing a biology label. Every arm here runs
//! the identical frame count, so that confound cannot arise; the wall clock
//! is printed for information and is explicitly not the answer.
//!
//! # The two readings, and why both are printed
//!
//! Frame 0 and the final frame, for light and for soil water. A difference
//! present at frame 0 is the **scene** — the box was built differently — and
//! one that only appears at the end is the **run**. `CLAUDE.md`: a scene that
//! contradicts the code looks exactly like a bug in the code, and the light
//! figure in particular cannot move at build time unless the shell moved,
//! because the sky walk attenuates through what is *above* the bench and soil
//! is below it.
//!
//! # Controls
//!
//! `control=empty` is the specificity half — no founders, so every plant
//! column must read 0; a non-zero there is the census counting something
//! that is not the crop. `control=dark` is the sensitivity half — the same
//! bed with the sky held at the bottom of the day's curve, which must
//! crater the stand. A sweep whose knob is disconnected and a sweep whose
//! knob does nothing are the same table of numbers, and only a control that
//! is known to move tells them apart.
//!
//! ```text
//! cargo run --release --example labsoil -- soils=40,64,96,128 seeds=12 frames=9000
//! cargo run --release --example labsoil -- control=dark soils=40 seeds=3 frames=3600
//! ```

use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::field;
use pixel_physics::sim::frame;
use pixel_physics::sim::organism::{self, CellType};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::update;
use pixel_physics::sim::world::World;

/// Rows below the surface counted as the rooting zone for the water census.
///
/// Not the whole bed on purpose: the founder herb's roots stop at about
/// twelve rows (`scene.rs`'s `DEFAULT_SOIL_DEPTH` doc), so pooling moisture
/// over a 128-row bed averages the crop's own water with 116 rows nothing
/// has ever drunk from — which would report a deep bed as *wetter* precisely
/// because its roots are failing to reach anything.
const ROOT_ZONE_ROWS: i32 = 16;

/// One arm's census. Counters only.
#[derive(Clone, Copy, Default, Debug)]
struct Census {
    cells: usize,
    biggest: usize,
    roots: i32,
    plants: usize,
    ants: usize,
    seeds: u32,
    births: u64,
    fruit: usize,
    /// Mean light at the bench over the founder columns, as a fraction of
    /// `field::MAX_LIGHT`.
    light: f32,
    /// The dimmest founder column's share of the same.
    dim: f32,
    /// Mean `plant_available_fraction` over the rooting zone under the
    /// founders — the quantity a root actually reads, not raw moisture.
    avail: f32,
    /// The same over every soil row in the bed, which is the number that
    /// separates "the bed is drier" from "the water went somewhere the
    /// roots are not".
    avail_all: f32,
}

fn census(world: &World, spec: &LabBox) -> Census {
    let mut c = Census::default();
    let ids = world.live_organism_ids();
    for id in &ids {
        let Some(state) = world.organism(*id) else { continue };
        if world.species.get(state.species).creature.is_some() {
            c.ants += 1;
            continue;
        }
        c.plants += 1;
        c.cells += state.cells.len();
        c.biggest = c.biggest.max(state.cells.len());
        c.seeds += state.seeds_set;
        // Material-keyed, not type-keyed: a root matures into `MatureBody`
        // while keeping the species' root material, so cell type alone sees
        // only the growing ends (`labshot`'s own note, and `root_contact`'s
        // before it).
        let root = world.materials.id_of(&world.species.get(state.species).root_material);
        let Some(root) = root else { continue };
        for &(x, y) in state.cells.keys() {
            if world.get(x, y).material == root {
                c.roots = c.roots.max(y - spec.ground_y + 1);
            }
        }
    }
    c.births = world.creature_stats.births;

    // Standing fruit as cells in the grid rather than as `seeds_set`: they
    // are different quantities, and the second cannot be walked to by an ant.
    for y in 0..spec.height {
        for x in 0..spec.width {
            let cell = world.get(x, y);
            if cell.organism_id() != 0 && organism::cell_type(cell.aux()) == Some(CellType::Fruit) {
                c.fruit += 1;
            }
        }
    }

    let cols = spec.founder_columns();
    let mut lit = 0.0f32;
    let mut dimmest = f32::INFINITY;
    for &x in &cols {
        let v = world.field_at(x, spec.ground_y - 2).light / field::MAX_LIGHT;
        lit += v;
        dimmest = dimmest.min(v);
    }
    c.light = lit / cols.len().max(1) as f32;
    c.dim = if dimmest.is_finite() { dimmest } else { 0.0 };

    // Water the roots can actually take up, which is `plant_available_fraction`
    // and not raw moisture: below the wilting point it is exactly zero however
    // much water the cell nominally holds.
    let mut zone = (0.0f32, 0usize);
    for &x in &cols {
        for y in spec.ground_y..(spec.ground_y + ROOT_ZONE_ROWS).min(spec.ground_y + spec.soil_depth)
        {
            zone.0 += update::plant_available_fraction(world.get(x, y));
            zone.1 += 1;
        }
    }
    c.avail = zone.0 / zone.1.max(1) as f32;

    let mut all = (0.0f32, 0usize);
    for y in spec.ground_y..(spec.ground_y + spec.soil_depth) {
        for x in 0..spec.width {
            all.0 += update::plant_available_fraction(world.get(x, y));
            all.1 += 1;
        }
    }
    c.avail_all = all.0 / all.1.max(1) as f32;
    c
}

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

fn list(key: &str, fallback: &str) -> Vec<i32> {
    let raw: String = arg(key).unwrap_or_else(|| fallback.to_string());
    raw.split(',').map(|s| s.trim().parse().expect("a number")).collect()
}

/// Median of a sample, by the low element of the middle pair on an even
/// count. Order statistics rather than means throughout: outcomes here are
/// chaotic in the seed, and `CLAUDE.md` asks for one explicitly.
fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn main() {
    let soils = list("soils", "40,64,96,128");
    let seeds: u64 = arg("seeds").unwrap_or(12);
    let seed0: u64 = arg("seed0").unwrap_or(1);
    let frames: u64 = arg("frames").unwrap_or(9000);
    // Long enough for the sky walk to have reached the bench and the soil to
    // have taken one pass, short enough that nothing has grown into it.
    let warm: u64 = arg("warm").unwrap_or(120);
    let control: String = arg("control").unwrap_or_else(|| "none".to_string());
    let height: i32 = arg("height").unwrap_or(LabBox::default().height);
    let ground: i32 = arg("ground").unwrap_or(LabBox::default().ground_y);
    let founders: usize = arg("founders").unwrap_or(LabBox::default().founders);
    let colonies: usize = arg("colonies").unwrap_or(LabBox::default().colonies);
    let walls: usize = arg("walls").unwrap_or(1);

    // Echo the parameters. A knob nobody can see the value of is a knob
    // nobody can tell is disconnected -- `CLAUDE.md`, from a 3.5-hour study
    // that turned out to be three populations wearing 24 logs.
    println!(
        "labsoil: soils={soils:?} seeds={seed0}..{} frames={frames} warm={warm} {height}x{} ground={ground} \
         founders={founders} colonies={colonies} walls={walls} control={control}",
        seed0 + seeds - 1,
        LabBox::default().width,
    );

    let mut rows: Vec<(i32, u64, Census, Census, f64)> = Vec::new();
    for &soil in &soils {
        for s in 0..seeds {
            let seed = seed0 + s;
            let spec = LabBox {
                height,
                ground_y: ground,
                soil_depth: soil,
                founders: if control == "empty" { 0 } else { founders },
                colonies: if control == "empty" { 0 } else { colonies },
                compartments: walls,
                seed,
                ..LabBox::default()
            };
            let (mut world, placed) = spec.build_counted();
            if control == "dark" {
                world.set_sky_hold(Some(pixel_physics::sky::frame_for_daylight(0.0)));
            }
            // The scene check made mechanical, which `burrow_probe` pays for
            // in its own header: a bed whose soil ran past the bottom of the
            // world is silently dropped by `World::set` and reads downstream
            // as a biology result. Assert the bed is the bed before a tick.
            //
            // Counted across the row rather than sampled at one column: the
            // colony is founded at the middle of the bed and the founders
            // are spread across it, so a single probe lands on a burrow or a
            // root and reports a scene error that is not one. Three quarters
            // is well clear of the shell, the partitions and everything the
            // builder plants.
            let soil_id = world.materials.id_of("soil").expect("soil is compiled in");
            for probe in [spec.ground_y, spec.ground_y + soil - 1] {
                let n = (0..spec.width).filter(|&x| world.get(x, probe).material == soil_id).count();
                assert!(
                    n * 4 >= spec.width as usize * 3,
                    "soil={soil} height={height} ground={ground}: row {probe} holds {n} soil \
                     cells of {}, so this bed is not the bed being reported",
                    spec.width
                );
            }
            assert_eq!(
                placed.planted,
                if control == "empty" { 0 } else { founders },
                "soil={soil} seed={seed}: the builder did not plant what it was asked for"
            );

            let mut particles = ParticleSystem::new();
            let mut blasts = Blasts::new();
            let tuning = player::Tuning::default();
            let t0 = std::time::Instant::now();
            // **The early reading is taken after a warmup, not at frame 0**,
            // and that is not a detail: the field has never been solved
            // before the first tick, so a frame-0 light census reads a flat
            // 0.000 in every arm — a number that is arithmetically correct,
            // identical across arms, and about nothing. `CLAUDE.md`'s
            // instrument rule, met on the first run of this harness.
            let mut first = Census::default();
            for f in 0..frames {
                if f == warm {
                    first = census(&world, &spec);
                }
                frame::step(
                    &mut world,
                    &mut particles,
                    &mut blasts,
                    player::PlayerInput::default(),
                    &tuning,
                );
            }
            let secs = t0.elapsed().as_secs_f64();
            let last = census(&world, &spec);
            println!(
                "  soil {soil:>4} seed {seed:>3}: cells {:>6} biggest {:>5} roots {:>4} \
                 plants {:>3} ants {:>4} seeds {:>5} fruit {:>4} | light {:.3} dim {:.3} \
                 | avail {:.3} all {:.3} | early light {:.3} avail {:.3} | {secs:.1}s",
                last.cells,
                last.biggest,
                last.roots,
                last.plants,
                last.ants,
                last.seeds,
                last.fruit,
                last.light,
                last.dim,
                last.avail,
                last.avail_all,
                first.light,
                first.avail,
            );
            rows.push((soil, seed, first, last, secs));
        }
    }

    // Per-depth order statistics.
    println!("\n  depth |  cells (med)   biggest    roots   seeds |  light   dim  avail |  s/run");
    for &soil in &soils {
        let of = |f: &dyn Fn(&Census) -> f64| -> f64 {
            median(rows.iter().filter(|r| r.0 == soil).map(|r| f(&r.3)).collect())
        };
        let secs = median(rows.iter().filter(|r| r.0 == soil).map(|r| r.4).collect());
        println!(
            "  {soil:>5} | {:>11.0} {:>9.0} {:>8.0} {:>7.0} | {:>6.3} {:>5.3} {:>6.3} | {secs:>6.1}",
            of(&|c| c.cells as f64),
            of(&|c| c.biggest as f64),
            of(&|c| c.roots as f64),
            of(&|c| c.seeds as f64),
            of(&|c| c.light as f64),
            of(&|c| c.dim as f64),
            of(&|c| c.avail as f64),
        );
    }

    // **Paired by seed, against the first depth in the list.** The median of
    // a column is a comparison of two populations; the sign count is a
    // comparison of two worlds that differ only in the knob, which is the
    // shape `CLAUDE.md` asks for and the one that survives a chaotic seed.
    let base = soils[0];
    println!("\n  paired against soil={base}, per seed (worse / same / better on plant cells):");
    for &soil in soils.iter().skip(1) {
        let (mut worse, mut same, mut better) = (0, 0, 0);
        let mut ratios = Vec::new();
        for s in 0..seeds {
            let seed = seed0 + s;
            let find = |d: i32| rows.iter().find(|r| r.0 == d && r.1 == seed).map(|r| r.3.cells);
            let (Some(b), Some(a)) = (find(base), find(soil)) else { continue };
            match a.cmp(&b) {
                std::cmp::Ordering::Less => worse += 1,
                std::cmp::Ordering::Equal => same += 1,
                std::cmp::Ordering::Greater => better += 1,
            }
            if b > 0 {
                ratios.push(a as f64 / b as f64);
            }
        }
        println!(
            "  soil {soil:>4}: {worse:>3} worse / {same:>3} same / {better:>3} better  \
             median ratio {:.3}",
            median(ratios)
        );
    }
}
