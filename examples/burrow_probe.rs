//! **Does a tunnel dug in soil survive?** — the measurement behind the
//! evolution lab's digging question, and a direct check on a claim
//! `wiki/ants.md` makes today: *"Turn a colony loose on a soil bank and it
//! hollows it out, leaving the stone beneath untouched."*
//!
//! `Reports/evolution-lab-design-guide-2026-08-30.md` §2b records the owner's
//! decision to decline collapsing tunnels, and reads the cost of that as the
//! structural scheduler's 16%. It then says a repose angle is harmless —
//! *"a dug wall that slumps a little is available and free; a roof that falls
//! in is what was declined."* **That is the thing this harness tests**, and
//! it is testable because the two mechanisms are separable: the structural
//! scheduler is not what closes a hole in a powder. `update_powder`'s
//! straight-down rule is, and it runs in the CA sweep whether or not any
//! structural code is linked.
//!
//! Three arms, each a bed with the same excavation cut into it:
//!
//! | arm | bed |
//! |---|---|
//! | `soil` | the lab's own bed — `soil`, a `Powder` |
//! | `sand` | `sand`, the loosest shipped powder — the *negative* control, expected worse |
//! | `stone` | `stone`, a `Solid` — the **positive control** |
//!
//! **The positive control is the point.** `CLAUDE.md`: *a null looks the same
//! whether the mechanism is quiet or the probe never reached it*, and *run the
//! positive control — construct the case whose answer you know is non-zero and
//! check the instrument reports it.* A tunnel in stone must read 100%
//! surviving at every frame, or this harness is measuring its own scene
//! construction and not the physics.
//!
//! The excavation is what an ant would actually dig, not an abstract cavity:
//! a vertical shaft from the surface, a horizontal gallery off it, and a
//! chamber at the end. Each is censused separately, because they fail for
//! different reasons and a single pooled number would hide that — a shaft is
//! a vertical face (the repose rule), a gallery has a roof (the straight-down
//! rule), and a chamber is both with a longer span.
//!
//! ```text
//! cargo run --release --example burrow_probe
//! cargo run --release --example burrow_probe -- frames=3600 seeds=4
//! cargo run --release --example burrow_probe -- arms=soil width=256
//! ```

mod common;

use common::PlantScene;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::weather::Pin;
use pixel_physics::sim::{frame, material, player, Cell, World};

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses"))
    })
}

/// One dug void, censused on its own. `cells` is what was carved; the census
/// counts how many of them are still materially empty.
struct Void {
    name: &'static str,
    cells: Vec<(i32, i32)>,
}

impl Void {
    /// **Raw material equality, not `Cell::is_empty`.** `is_empty` is
    /// managed-aware and answers "is this position available", which is a
    /// different question from "is there material here" (`CLAUDE.md`'s
    /// gotcha). A tunnel refilled with soil must not read as empty.
    fn open(&self, world: &World) -> usize {
        self.cells.iter().filter(|(x, y)| world.get(*x, *y).material == material::EMPTY).count()
    }
}

fn main() {
    let frames: u64 = arg("frames").unwrap_or(1_800);
    let width: i32 = arg("width").unwrap_or(256);
    let height: i32 = arg("height").unwrap_or(320);
    // **The bed has to fit in the world, and the builder will not say so.**
    // `PlantScene` writes its stone floor at `ground_y + soil + STONE_DEPTH`
    // and `World::set` silently drops anything past the bottom edge, so a bed
    // deeper than the world produces a bed with no floor *and* an excavation
    // carved into rows that do not exist -- which reads as "the tunnel closed
    // instantly" at frame 0, before a single tick has run. That is
    // `CLAUDE.md`'s scene-error trap exactly, and it is why the frame-0 row
    // below is asserted rather than merely printed.
    let ground: i32 = arg("ground").unwrap_or(60);
    let soil: i32 = arg("soil").unwrap_or(200);
    let seeds: u64 = arg("seeds").unwrap_or(1);
    let want: String = arg("arms").unwrap_or_else(|| "soil,sand,stone".to_string());

    println!(
        "burrow_probe: frames={frames} width={width}x{height} soil={soil} seeds={seeds} arms={want}"
    );
    println!(
        "\nan excavation is cut into each bed and censused as it fills in. \
         `stone` is the positive control and must read 100% at every frame."
    );

    for arm in ["soil", "sand", "stone"].iter().filter(|a| want.split(',').any(|w| &w == *a)) {
        println!("\n=== arm {arm} ===");
        println!(
            "{:>8}  {:>7}  {:>18}  {:>18}  {:>18}",
            "seed", "frame", "shaft open", "gallery open", "chamber open"
        );

        for seed in 1..=seeds {
            let mut scene = PlantScene { species: "herb".to_string(), ..PlantScene::default() };
            scene.width = width;
            scene.height = height;
            scene.ground_y = ground;
            scene.soil_depth = soil;
            scene.trees = 0;
            scene.seed = Some(seed);
            let mut world = scene.build();
            // No weather, and a held light: the lab's own operating point.
            // Rain into an open shaft is a real hazard and a different
            // experiment; it must not ride along inside this one.
            world.set_weather_pin(Pin::Clear);

            // **Repaint the bed in this arm's material.** The scene builder
            // only makes soil, so `sand` and `stone` are written over the
            // soil rows it produced -- same geometry, same stone floor
            // underneath, one material changed. That is the A/B `CLAUDE.md`
            // asks for: two arms differing in one thing.
            if *arm != "soil" {
                let id = world
                    .materials
                    .id_of(arm)
                    .unwrap_or_else(|| panic!("{arm} is a compiled-in material"));
                for x in 0..width {
                    for y in ground..(ground + soil) {
                        world.set(x, y, Cell::new(id, 0));
                    }
                }
            }

            // The excavation. A shaft down from the surface, a gallery
            // running off its foot, and a chamber at the gallery's end --
            // 3 cells tall, which is what an ant fits through.
            let shaft_x = width / 3;
            let shaft_bottom = ground + soil / 2;
            let gallery_y = shaft_bottom;
            let gallery_end = shaft_x + 60;

            let mut shaft = Void { name: "shaft", cells: Vec::new() };
            for y in ground..shaft_bottom {
                for x in shaft_x..(shaft_x + 3) {
                    shaft.cells.push((x, y));
                }
            }
            let mut gallery = Void { name: "gallery", cells: Vec::new() };
            for x in shaft_x..gallery_end {
                for y in gallery_y..(gallery_y + 3) {
                    gallery.cells.push((x, y));
                }
            }
            let mut chamber = Void { name: "chamber", cells: Vec::new() };
            for x in gallery_end..(gallery_end + 16) {
                for y in (gallery_y - 4)..(gallery_y + 4) {
                    chamber.cells.push((x, y));
                }
            }

            let voids = [&shaft, &gallery, &chamber];
            for v in voids {
                for (x, y) in &v.cells {
                    world.set(*x, *y, Cell::EMPTY);
                }
            }
            let carved: Vec<usize> = voids.iter().map(|v| v.cells.len()).collect();

            let mut particles = ParticleSystem::default();
            let mut blasts = Blasts::default();
            let tuning = player::Tuning::default();

            let report = |world: &World, f: u64| {
                let cols: Vec<String> = voids
                    .iter()
                    .zip(&carved)
                    .map(|(v, n)| {
                        let open = v.open(world);
                        format!("{open:>5}/{n:<5} {:>5.1}%", 100.0 * open as f64 / *n as f64)
                    })
                    .collect();
                println!(
                    "{seed:>8}  {f:>7}  {:>18}  {:>18}  {:>18}",
                    cols[0], cols[1], cols[2]
                );
            };

            // **The scene check, as an assertion.** Every carved cell must be
            // open before any tick runs. If it is not, the excavation is not
            // where the harness thinks it is and every number below is about
            // the scene rather than about the physics.
            report(&world, 0);
            for (v, n) in voids.iter().zip(&carved) {
                assert_eq!(
                    v.open(&world),
                    *n,
                    "{} was not fully carved at frame 0 -- the excavation is outside the bed \
                     (ground={ground} soil={soil} height={height}); every number after this \
                     would be a measurement of the scene",
                    v.name
                );
            }
            let marks = [1u64, 5, 30, 120, 600, frames];
            for f in 1..=frames {
                frame::step(
                    &mut world,
                    &mut particles,
                    &mut blasts,
                    player::PlayerInput::default(),
                    &tuning,
                );
                if marks.contains(&f) {
                    report(&world, f);
                }
            }
        }
    }
}
