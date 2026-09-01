//! **Does a colony's nest stand in the lab's own bed?** — the owner's
//! playtest report, reproduced or refuted.
//!
//! Reported 2026-08-30, playing the merged lab: *"it looks to me like tunnels
//! that ants are digging are collapsing or soil is filling and they get stuck
//! underground."*
//!
//! `burrow_probe` already says a *hand-carved lined* gallery holds at 100%
//! and that a colony on a **hand-built soil bank** leaves 130 roofed cells
//! against 0 ablated. Neither of those is the scene the owner is playing.
//! The lab's bed is different in three ways that each bear on this, and
//! `CLAUDE.md`'s standing rule is that a scene which contradicts the code
//! looks exactly like a bug in the code:
//!
//! 1. **It is built at exactly `SOIL_FIELD_CAPACITY`** (`scene.rs`'s
//!    `with_aux`), and the un-pack rule fires at `aux > SOIL_FIELD_CAPACITY`.
//!    That is a margin of **one unit**. If `update_soil_water` ever pushes a
//!    wall cell one unit over — and redistributing water around a *void* is
//!    exactly the case nobody measured — the lining dissolves and the
//!    tunnel comes down. The bank scenes `burrow_probe` uses do not sit on
//!    that line.
//! 2. **It is 96 rows deep**, so an ant can be far below the surface.
//! 3. **It is sealed and lit through the shell**, so nothing dries it out.
//!
//! # The two halves of the report get separate columns, because they have
//! # different fixes
//!
//! *"Tunnels collapsing / soil filling"* is a **structure** question:
//! `roofed` (void with ground above it) and `packed` (lining still standing)
//! over time. A lining that appears and then vanishes is hypothesis 1; a
//! lining that never appears is a dig problem.
//!
//! *"They get stuck underground"* is a **motion** question, and it needs its
//! own counters or it cannot be told from "they are fine and just out of
//! sight". `buried` counts ants with ground on every side; `blocked_frac` is
//! `moves_blocked / (moves + moves_blocked)`. An ant that is walking a
//! gallery and one that is entombed both read "below the surface".
//!
//! **Read the counters, not the totals.** `CLAUDE.md`: a bank with no holes
//! in it and a colony that never dug are the same picture. `digs` and
//! `packed` are printed on every row for exactly that reason.
//!
//! ```text
//! cargo run --release --example labnest
//! cargo run --release --example labnest -- frames=12000 seeds=3
//! cargo run --release --example labnest -- dry=1     # the isolating control
//! ```

use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::material::MaterialId;
use pixel_physics::sim::{frame, material, player, World};

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses"))
    })
}

/// Void with ground above it, inside the bed. **Roofed, not standing** —
/// `CLAUDE.md`'s metric trap: a colony quarries the open face as well as
/// tunnelling, and a pit is standing void, so censusing bare emptiness
/// scored the build with no roof at all *higher* than the one whose tunnels
/// stand. What a player calls a nest has ground over it.
fn roofed(world: &World, x0: i32, x1: i32, y0: i32, y1: i32) -> usize {
    let mut n = 0;
    for x in x0..x1 {
        for y in y0..y1 {
            if world.get(x, y).material != material::EMPTY {
                continue;
            }
            // Ground somewhere above in this column, inside the bed.
            if (y0..y).rev().any(|uy| world.get(x, uy).material != material::EMPTY) {
                n += 1;
            }
        }
    }
    n
}

/// Cells of standing lining. The quantity hypothesis 1 predicts will rise
/// and then fall.
fn packed_cells(world: &World, id: MaterialId, x0: i32, x1: i32, y0: i32, y1: i32) -> usize {
    let mut n = 0;
    for x in x0..x1 {
        for y in y0..y1 {
            if world.get(x, y).material == id {
                n += 1;
            }
        }
    }
    n
}

/// **Wall cells that have gone over the un-pack line.** The direct test of
/// hypothesis 1, and the reason this harness exists rather than a re-run of
/// `burrow_probe`: it reports the *margin* rather than the outcome, so a
/// bed sitting one unit under the threshold reads differently from one
/// sitting a hundred under it.
fn over_capacity(world: &World, x0: i32, x1: i32, y0: i32, y1: i32) -> (usize, u16) {
    let (mut n, mut worst) = (0, 0u16);
    for x in x0..x1 {
        for y in y0..y1 {
            let c = world.get(x, y);
            if c.material == material::EMPTY {
                continue;
            }
            if world.materials.get(c.material).water_capacity > 0 {
                worst = worst.max(c.aux());
                if c.aux() > material::SOIL_FIELD_CAPACITY {
                    n += 1;
                }
            }
        }
    }
    (n, worst)
}

/// Ants with ground on all four sides — entombed rather than merely
/// underground. The second half of the report needs its own number.
fn buried(world: &World) -> usize {
    world
        .live_organism_ids()
        .into_iter()
        .filter_map(|id| world.organism_state(id))
        .filter(|state| world.species.get(state.species).creature.is_some())
        .filter(|state| {
            // Every cell of the body walled in on all four sides. A body
            // with one open face is in a gallery; one with none is
            // entombed, and only the second is the owner's complaint.
            state.cells.keys().all(|(x, y)| {
                [(0, -1), (0, 1), (-1, 0), (1, 0)].iter().all(|(dx, dy)| {
                    let n = (x + dx, y + dy);
                    world.get(n.0, n.1).material != material::EMPTY
                        || state.cells.contains_key(&n)
                })
            })
        })
        .count()
}

fn main() {
    let frames: u64 = arg("frames").unwrap_or(9_000);
    let seeds: u64 = arg("seeds").unwrap_or(2);
    // **The isolating control for hypothesis 1.** Same bed, built a long way
    // under field capacity instead of exactly on it. If the lining survives
    // here and dissolves in the default bed, the margin is the bug and the
    // fix is the threshold, not the digging.
    let dry: bool = arg::<u32>("dry").unwrap_or(0) == 1;

    println!("labnest: frames={frames} seeds={seeds} dry={dry}");
    println!(
        "\n{:>5} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>6} {:>7} {:>7} {:>6} {:>8}",
        "seed", "frame", "roofed", "packed", "overcap", "wettest", "ants", "buried", "digs",
        "dumped", "laden", "blocked%"
    );

    for seed in 1..=seeds {
        let spec = LabBox { colonies: 1, founders: 0, seed, ..LabBox::default() };
        let mut world = spec.build();
        let packed_id = world.materials.id_of("packedsoil").expect("packedsoil is compiled in");

        if dry {
            let soil = world.materials.id_of("soil").expect("soil is compiled in");
            for x in 0..spec.width {
                for y in 0..spec.height {
                    let c = world.get(x, y);
                    if c.material == soil {
                        world.set(x, y, c.with_aux(material::SOIL_WILTING_POINT * 2));
                    }
                }
            }
        }

        let (y0, y1) = (spec.ground_y, spec.ground_y + spec.soil_depth);
        let (x0, x1) = (0, spec.width);

        let mut particles = ParticleSystem::default();
        let mut blasts = Blasts::default();
        let tuning = player::Tuning::default();

        let report = |world: &World, f: u64| {
            let (over, wettest) = over_capacity(world, x0, x1, y0, y1);
            let m = world.creature_stats.moves;
            let b = world.creature_stats.moves_blocked;
            // **`dumped` and `laden` beside `digs`, which is a call counter.**
            // Digging takes one cell into the mandibles and `dumped` counts
            // the ones that came out again, so `digs - dumped` is what is in
            // flight -- and `laden`, the animals holding one right now, is
            // what says whether that is traffic or a jam. A colony that has
            // stopped digging because every ant is stuck holding a pellet it
            // cannot put down reads exactly like one that has lost interest,
            // and only this pair tells them apart.
            let laden = world
                .live_organism_ids()
                .into_iter()
                .filter(|&id| world.organism(id).is_some_and(|s| s.spoil.is_some()))
                .count();
            println!(
                "{seed:>5} {f:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>6} {:>7} {:>7} {laden:>6} {:>7.1}%",
                roofed(world, x0, x1, y0, y1),
                packed_cells(world, packed_id, x0, x1, y0, y1),
                over,
                wettest,
                world.live_creature_count(),
                buried(world),
                world.creature_stats.digs,
                world.creature_stats.spoil_dumped,
                100.0 * b as f64 / (m + b).max(1) as f64,
            );
        };

        report(&world, 0);
        for f in 1..=frames {
            frame::step(
                &mut world,
                &mut particles,
                &mut blasts,
                player::PlayerInput::default(),
                &tuning,
            );
            if f % (frames / 9).max(1) == 0 {
                report(&world, f);
            }
        }
    }
}
