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

fn run(world: &mut World, frames: usize) {
    for _ in 0..frames {
        parallel::step(world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
    }
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
fn scene(w: i32, h: i32, floor: i32, ants: usize, food: bool) -> (World, usize) {
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
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
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "frames" => frames = v.parse().expect("frames"),
            other => panic!("unknown arg {other:?}; known: frames"),
        }
    }
    println!("forage_probe: frames={frames} FORAGE_TRIP_MIN={}", pixel_physics::sim::creature::FORAGE_TRIP_MIN);

    let (w, h) = (320i32, 120i32);
    let floor = h - 8;

    let (mut control, n) = scene(w, h, floor, 1, false);
    run(&mut control, frames);
    report("control: one ant, a nest, no food", &control, n);

    let (mut forage, n) = scene(w, h, floor, 55, true);
    run(&mut forage, frames);
    report("forage: 55 ants, a nest, food at range", &forage, n);
    println!("  (nest edge to food edge is {FOOD_GAP} cells -- the depth a real round trip has to reach)");
}
