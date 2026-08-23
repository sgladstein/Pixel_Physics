//! Does the colony actually *range*, and how far?
//!
//! **Nothing measured foraging range before this, and the counter that
//! looked like it did was measuring loitering.**
//! `CreatureStats::nest_visits` increments on any move made while
//! nest-adjacent, guarded on `OrganismState::since_nest > 0` — but
//! `since_nest` is incremented unconditionally every tick, so that guard is
//! false exactly once in a creature's life. An ant that never leaves home
//! scores one per tick. `examples/ascii.rs`'s `assert!(st.nest_visits > 0,
//! "no ant ever reached the nest")` is therefore not the sessility guard it
//! reads as: a colony that never leaves passes it trivially.
//!
//! The fix is not to repair `since_nest`. It cannot be repaired: it also
//! accumulates while an ant stands *on* the nest, and it counts ticks while
//! `tick_interval` is 6, so its scale is a species constant rather than a
//! distance. `CreatureStats::forage_trips` is a **spatial** excursion depth
//! that re-anchors at every nest contact — see `OrganismState::forage_anchor`.
//!
//! # The two scenes, and why both
//!
//! - **`control`** — one ant, a nest, and no food anywhere. Nothing to
//!   forage for, nowhere to go. This is the arm that the old counter fails:
//!   it reads hundreds of "nest visits" from an ant that has been home the
//!   whole time.
//! - **`forage`** — a colony, a nest, and a food pile a known distance away.
//!   The arm that proves the new counter is not merely always-zero. A probe
//!   that cannot produce the thing it counts has already cost this project
//!   four wrong conclusions; a metric only trustworthy when it reads 0 is
//!   one of them waiting to happen.
//!
//! Read them as a pair. Neither is worth anything alone.
//!
//! ```text
//! cargo run --release --example forage_probe
//! cargo run --release --example forage_probe -- frames=8000
//! ```

use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material;
use pixel_physics::sim::{parallel, Cell, World};

/// Distance from the near edge of the nest patch to the near edge of the
/// food pile, in the `forage` scene. Printed beside the measured depth so
/// the number has something to be judged against: a colony whose deepest
/// excursion falls well short of this never reached the food at all.
const FOOD_GAP: i32 = 135 - 48;

/// The world seed this probe sweeps from (`BASE_SEED + i`).
///
/// The scene is hand-built, so there used to be no seed here at all and the
/// world simply kept `DEFAULT_WORLD_SEED`. That is not the same as having no
/// randomness: every creature draw is `rng::stream(world.seed, organism,
/// frame, slot)`, so the seed decides what the colony does, and a probe that
/// neither sets nor prints it was reporting one sample from a wide
/// distribution as though it were the number.
///
/// **`DEFAULT_WORLD_SEED`, not a fresh literal**, so `seeds=1` reproduces
/// every figure measured before this change bit for bit -- that is what the
/// hand-built scene was silently running on. Picking any other base would
/// have quietly invalidated the record while looking like a tidy-up: it moved
/// the control's moves from 64 to 68 on the first attempt here.
const BASE_SEED: u64 = pixel_physics::sim::world::DEFAULT_WORLD_SEED;

/// One run's headline figures, so `seeds=N` can order-statistic them.
///
/// House rule for a guard over a procedural system is an order statistic over
/// N seeds rather than any single seed, because which seed is worst reshuffles
/// on any legitimate change (`CLAUDE.md`, Conventions).
#[derive(Clone, Copy)]
struct Row {
    deliveries: u64,
    trips: u64,
    deepest: u64,
    moves: u64,
    blocked: u64,
}

fn run(world: &mut World, frames: usize) {
    for _ in 0..frames {
        parallel::step(world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
    }
}

fn row(world: &World) -> Row {
    let st = world.creature_stats;
    Row { deliveries: st.deliveries, trips: st.forage_trips, deepest: st.forage_depth_max, moves: st.moves, blocked: st.moves_blocked }
}

/// min / median / max of one column, printed rather than a mean.
///
/// A mean hides which end moved; the spread is the whole reason this sweeps
/// seeds at all.
fn order_stats(mut v: Vec<f64>) -> (f64, f64, f64) {
    v.sort_by(f64::total_cmp);
    let n = v.len();
    let median = if n == 0 {
        0.0
    } else if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    };
    (*v.first().unwrap_or(&0.0), median, *v.last().unwrap_or(&0.0))
}

fn report(label: &str, world: &World, ants: usize) {
    let st = world.creature_stats;
    println!("\n--- {label} ({ants} ant(s)) ---");
    println!(
        "  moves {} | blocked {} | pickups {} | drops {} | deliveries {} | deaths {}",
        st.moves, st.moves_blocked, st.pickups, st.drops, st.deliveries, st.deaths
    );
    // **The old counter and the new one on the same line, on the same run.**
    // Two mechanisms that look identical in a summary are exactly what this
    // pair exists to separate, so they are never printed apart.
    let ratio = if st.moves > 0 { st.nest_visits as f64 / st.moves as f64 } else { 0.0 };
    println!("  nest_visits {} ({ratio:.3} of moves -- loitering, NOT trips)", st.nest_visits);
    let mean = if st.forage_trips > 0 { st.forage_depth_sum as f64 / st.forage_trips as f64 } else { 0.0 };
    println!(
        "  forage_trips {} (bar {} cells) | mean depth {mean:.1} cells | deepest {} cells",
        st.forage_trips,
        pixel_physics::sim::creature::FORAGE_TRIP_MIN,
        st.forage_depth_max
    );
    // **The profile, always, next to the count.** The count is one point on
    // this curve and cannot show its shape; the shape is what says whether
    // a colony hops or ranges.
    let prof: Vec<String> = pixel_physics::sim::creature::FORAGE_REACH_BUCKETS
        .iter()
        .zip(&st.forage_reach)
        .map(|(edge, n)| format!(">={edge}: {n}"))
        .collect();
    println!("  excursions reaching at least N cells -- {}", prof.join("  "));
    // **Per ant, because the two scenes do not have the same number of
    // them.** A raw count from 55 ants against a raw count from 1 compares
    // colony size, not foraging, and reading it as foraging is how a jammed
    // colony looks busy.
    println!(
        "  per ant: {:.2} moves, {:.3} trips, deepest {} cells",
        st.moves as f64 / ants as f64,
        st.forage_trips as f64 / ants as f64,
        st.forage_depth_max
    );
}

/// Stone floor, a nest patch, and nothing else. `plant_ant` needs solid
/// ground under it or the ant spends the run falling, which is its own
/// wrong conclusion.
fn scene(w: i32, h: i32, floor: i32, ants: usize, food: bool, seed: u64) -> (World, usize) {
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    // Set before anything is planted: every creature draw keys off it.
    world.seed = seed;
    let nest = world.materials.id_of("nest").expect("nest is compiled in");
    let corpse = world.materials.id_of("corpse").expect("corpse is compiled in");
    for x in 0..w {
        for y in floor..h {
            world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
        }
    }
    for x in 16..48 {
        world.set(x, floor, Cell::new(nest, 0).with_attached(true));
    }
    if food {
        for x in 135..170 {
            for y in (floor - 5)..floor {
                world.set(x, y, Cell::new(corpse, 0));
            }
        }
    }
    for i in 0..ants {
        world.plant_ant(20 + i as i32 * 2, floor - 1);
    }
    (world, ants)
}

fn main() {
    let mut frames = 6000usize;
    let mut seeds = 1u64;
    for arg in std::env::args().skip(1) {
        // **A bare argument is an error, not a shrug.** This loop used to
        // `continue` past anything without an `=`, so `forage_probe 8` ran
        // the default and said nothing about it.
        let Some((k, v)) = arg.split_once('=') else {
            panic!("unknown arg {arg:?}; known: frames, seeds");
        };
        match k {
            "frames" => frames = v.parse().expect("frames"),
            "seeds" => seeds = v.parse().expect("seeds"),
            other => panic!("unknown arg {other:?}; known: frames, seeds"),
        }
    }
    println!(
        "forage_probe: frames={frames} seeds={seeds} base_seed={BASE_SEED:#x} FORAGE_TRIP_MIN={}",
        pixel_physics::sim::creature::FORAGE_TRIP_MIN
    );

    let (w, h) = (320i32, 120i32);
    let floor = h - 8;

    // One seed keeps the old single-run output verbatim, so every figure
    // measured before this still compares.
    if seeds == 1 {
        let (mut control, n) = scene(w, h, floor, 1, false, BASE_SEED);
        run(&mut control, frames);
        report("control: one ant, a nest, no food", &control, n);

        let (mut forage, n) = scene(w, h, floor, 55, true, BASE_SEED);
        run(&mut forage, frames);
        report("forage: 55 ants, a nest, food at range", &forage, n);
        println!("  (nest edge to food edge is {FOOD_GAP} cells -- the depth a real round trip has to reach)");
        return;
    }

    // **The instrument every foraging claim has to quote from here on.**
    // Both scenes per seed, per-seed rows, and an order statistic over them
    // -- not a mean, and not one seed. Outcomes here have a standard
    // deviation around 0.1, so a bar set from a single run is a sample from
    // a wide distribution and flakes in whichever direction that run landed.
    println!("\n  both scenes per seed; control is one ant with no food, forage is 55 ants with food at {FOOD_GAP} cells\n");
    // The offset, not the full 16-hex-digit seed: line one already names the
    // base, and `base+3` is what a reader needs to reproduce a row.
    println!("{:>6}  {:>10} {:>7} {:>8} {:>8} {:>8}  |  {:>10} {:>9} {:>8}", "seed", "deliveries", "trips", "deepest", "moves", "blocked", "ctl moves", "ctl trips", "ctl deep");
    let mut forage_rows: Vec<Row> = Vec::new();
    for i in 0..seeds {
        let seed = BASE_SEED + i;
        let (mut control, _) = scene(w, h, floor, 1, false, seed);
        run(&mut control, frames);
        let c = row(&control);

        let (mut forage, _) = scene(w, h, floor, 55, true, seed);
        run(&mut forage, frames);
        let f = row(&forage);
        println!(
            "{:>6}  {:>10} {:>7} {:>8} {:>8} {:>8}  |  {:>10} {:>9} {:>8}",
            format!("+{i}"),
            f.deliveries,
            f.trips,
            f.deepest,
            f.moves,
            f.blocked,
            c.moves,
            c.trips,
            c.deepest
        );
        forage_rows.push(f);
    }
    println!("\n{:>10}  {:>8} {:>8} {:>8}", "", "min", "median", "max");
    for (name, col) in [
        ("deliveries", forage_rows.iter().map(|r| r.deliveries as f64).collect::<Vec<_>>()),
        ("trips", forage_rows.iter().map(|r| r.trips as f64).collect()),
        ("deepest", forage_rows.iter().map(|r| r.deepest as f64).collect()),
        ("moves", forage_rows.iter().map(|r| r.moves as f64).collect()),
    ] {
        let (lo, med, hi) = order_stats(col);
        println!("{name:>10}  {lo:>8.1} {med:>8.1} {hi:>8.1}");
    }
    // Blocked-move fraction is the jam readout, and it is the pair the
    // traffic work (WP-9) has to quote beside any reach claim.
    let blocked: Vec<f64> = forage_rows.iter().map(|r| if r.moves > 0 { r.blocked as f64 / r.moves as f64 } else { 0.0 }).collect();
    let (lo, med, hi) = order_stats(blocked);
    println!("{:>10}  {lo:>8.3} {med:>8.3} {hi:>8.3}", "blocked/mv");
}
