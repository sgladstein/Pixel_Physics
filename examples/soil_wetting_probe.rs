//! **Does a decay-created soil cell wet up from the damp soil below it, or
//! does it stay bone dry for ever?**
//!
//! `Reports/instruments.md`'s `seedbed_probe` entry and
//! `Reports/where-a-dead-plant-goes-2026-08-31.md` both report a fresh
//! decay-created soil cell reading available water 0.000 after 6,600 frames.
//! That measurement predates two changes the same report names as landing
//! the same day: the capillary rest threshold made conditional on the
//! drainable band (so unsaturated soil equalises at a gap of
//! `SOIL_CAPILLARY_REST_UNSATURATED` = 60 rather than the old 380), and the
//! evaporative sink bounded so the fix does not just drain the bed back out.
//! This probe re-measures directly against current `HEAD`, using the exact
//! cell-creation call `decay.rs` performs, so the answer does not depend on
//! trusting either report's standing.
//!
//! ```text
//! cargo build --release --example soil_wetting_probe
//! cargo run --release --example soil_wetting_probe
//! ```
//!
//! **Every test column is walled in stone on both sides, for its whole
//! height.** A vertical column of loose soil one cell wide, standing in open
//! air, is not a stable scene in a falling-sand engine -- `soil.ron`'s
//! `friction_angle: 33.0` is far short of the 90-degree slope a bare 1-wide
//! tower presents, so ordinary powder movement (not capillary) immediately
//! slumps it sideways and scrambles the geometry before capillary has
//! anything to measure. The first version of this probe hit exactly that and
//! read a uniform zero everywhere; walling each column in stone removes the
//! open diagonal a grain could slide into, at every row, so the only way a
//! cell's `aux` can change is the mechanism under test.
//!
//! Four things are measured, in order:
//! 1. **Fidelity** — call the real `decay::tick` dispatch on a litter cell
//!    and confirm its product cell is bit-for-bit what this probe's "decay"
//!    arm replicates by calling `Cell::new` directly.
//! 2. **Main comparison**, one world, `parallel::step` (the driver the app
//!    runs) — three isolated arms: a decay-created cell over field-capacity
//!    soil, a hand-placed one over the same, and a hand-placed one with the
//!    wet donor beside it rather than below. Bare `parallel::step` excludes
//!    `world.step_active_sites()`, so this isolates capillary + drainage
//!    from evaporation. `world.soil_water_stats` is printed for the first
//!    few frames as the positive control: `changed` must be nonzero or the
//!    scene still is not exercising the mechanism.
//! 3. **Full pipeline** — the hand-placed arm re-run through `frame::step`,
//!    which does include active sites (and therefore evaporation), to check
//!    whether the fuller picture undoes what capillary alone does.
//! 4. **The other driver** — the same hand-placed arm stepped with the
//!    serial `update::step` instead (CLAUDE.md: "two drivers, and the app
//!    runs the parallel one"). `update.rs:100` calls `world.step_soil_water()`
//!    from this driver too ("both drivers, deliberately"), so this is a
//!    parity check, not a "does it move at all" question -- measured, it
//!    converges to the same value as the parallel driver.

use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::decay;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::material::{self, MaterialId, STONE};
use pixel_physics::sim::parallel;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::scheduler::{ActiveKind, ActiveSite};
use pixel_physics::sim::update;
use pixel_physics::sim::world::World;

const CHECKPOINTS: [u64; 5] = [0, 100, 600, 3000, 12000];
const HERB_THRESHOLD: f32 = 0.15;
const GRASS_THRESHOLD: f32 = 0.10;

fn report(label: &str, c: Cell) -> String {
    format!(
        "{label:<28} aux={:>4} avail={:>6.3}",
        update::soil_moisture(c),
        update::plant_available_fraction(c)
    )
}

/// Drive a due `ActiveKind::Decay` site until the cell at `(x, y)` becomes
/// `product`, feeding each call's rescheduled site back into the next.
/// Bounded so a probe bug reads as a panic rather than a hang.
fn force_decay(world: &mut World, x: i32, y: i32, product: MaterialId) -> u32 {
    let mut site = ActiveSite { x, y, kind: ActiveKind::Decay, next_frame: 0 };
    for tries in 1..=20_000u32 {
        let resched = decay::tick(world, &site);
        if world.get(x, y).material == product {
            return tries;
        }
        site = resched
            .into_iter()
            .next()
            .unwrap_or(ActiveSite { x, y, kind: ActiveKind::Decay, next_frame: world.frame });
    }
    panic!("litter at ({x},{y}) never decayed into the product material");
}

/// Wall a 1-wide vertical column in stone on both sides, for `top..=bottom`
/// -- removes every open diagonal a resting powder could slide into, so the
/// column cannot collapse under ordinary gravity/slump physics regardless of
/// its height. See the module doc: this is the fix for the bug the first
/// version of this probe had.
fn wall_column(w: &mut World, x: i32, top: i32, bottom: i32) {
    for y in top..=bottom {
        w.set(x - 1, y, Cell::new(STONE, 0));
        w.set(x + 1, y, Cell::new(STONE, 0));
    }
}

fn main() {
    // Only touches decay.rs's own yield roll (unrelated to the capillary
    // code under test) -- makes step 1's fidelity check deterministic-ish
    // instead of a 1-in-20 shot on top of the decay-chance roll it already
    // has to clear. `decay::decay_yield_override` reads this once via a
    // `OnceLock`, so it must be set before anything touches decay.rs.
    std::env::set_var("DECAY_YIELD", "1");

    // === 1. Fidelity: what does decay.rs's real product cell look like? ===
    let (soil, litter, decay_product_aux) = {
        let mut w = World::new(Rect::new(0, 0, 9, 9));
        let soil = w.materials.id_of("soil").expect("soil is compiled in");
        let litter = w.materials.id_of("litter").expect("litter is compiled in");
        w.set(5, 5, Cell::new(litter, 0));
        let tries = force_decay(&mut w, 5, 5, soil);
        let aux = w.get(5, 5).aux();
        println!("=== 1. fidelity: decay::tick's real product ===");
        println!("  litter -> soil in {tries} decay::tick call(s); product aux = {aux}");
        println!(
            "  (this probe's \"decay\" arm below replicates this with Cell::new(soil, shade), aux={})",
            Cell::new(soil, 0).aux()
        );
        (soil, litter, aux)
    };
    assert_eq!(decay_product_aux, 0, "decay.rs's real product must start bone dry (aux 0), matching Cell::new's default");

    // === 2. Main comparison: one sealed world, parallel::step ==============
    let (min_x, min_y, max_x, max_y) = (0, 0, 89, 39);
    let mut w = World::new(Rect::new(min_x, min_y, max_x, max_y));
    // Seal the box in stone so weather cannot land on any test column --
    // parallel::step runs weather::step every frame, and this probe wants
    // capillary in isolation, not "did it rain on it".
    for x in min_x..=max_x {
        w.set(x, min_y, Cell::new(STONE, 0));
        w.set(x, max_y, Cell::new(STONE, 0));
    }
    for y in min_y..=max_y {
        w.set(min_x, y, Cell::new(STONE, 0));
        w.set(max_x, y, Cell::new(STONE, 0));
    }

    // The wet bed's soil column sits directly on the floor (max_y - 1) so
    // the powder column is fully supported from the first frame, and
    // `wall_column` below walls it in so it cannot slump sideways either.
    let bed_top = 25;
    let bed_bottom = max_y - 1;
    let surface_y = bed_top - 1;
    let wet_bed = |w: &mut World, x: i32| {
        for y in bed_top..=bed_bottom {
            w.set(x, y, Cell::new(soil, 0).with_aux(material::SOIL_FIELD_CAPACITY));
        }
        wall_column(w, x, surface_y, bed_bottom);
    };

    // Arm "decay": a genuine decay::tick product cell over a field-capacity
    // bed -- the exact path decay.rs uses, not a replica of it.
    let ax = 10;
    wet_bed(&mut w, ax);
    w.set(ax, surface_y, Cell::new(litter, 0));
    force_decay(&mut w, ax, surface_y, soil);
    assert_eq!(update::soil_moisture(w.get(ax, surface_y)), 0, "decay product over the bed must start bone dry");

    // Arm "hand-placed": the same `Cell::new(soil, shade)` decay.rs writes
    // (`decay.rs:220`), called directly instead of through decay::tick --
    // the positive control for "is a decay-created cell any different from
    // one the scene placed".
    let bx = 30;
    wet_bed(&mut w, bx);
    w.set(bx, surface_y, Cell::new(soil, 0));

    // Arm "wet-left": dry cell with the wet donor beside it rather than
    // below. Both sit on a stone shelf and the whole 2-wide alcove is walled
    // in stone (left of the donor, right of the receiver, and the full row
    // beneath both, covering the diagonals) so neither can slide anywhere --
    // isolates the lateral capillary term with no vertical or sideways
    // escape route.
    let cx = 60;
    w.set(cx - 1, surface_y, Cell::new(soil, 0).with_aux(material::SOIL_FIELD_CAPACITY)); // donor
    w.set(cx, surface_y, Cell::new(soil, 0)); // receiver, bone dry
    for x in (cx - 2)..=(cx + 1) {
        w.set(x, surface_y + 1, Cell::new(STONE, 0));
    }
    w.set(cx - 2, surface_y, Cell::new(STONE, 0));
    w.set(cx + 1, surface_y, Cell::new(STONE, 0));

    let arms = [("decay", ax), ("hand-placed", bx), ("wet-left(donor beside)", cx)];
    let mut herb_cross = [None::<u64>; 3];
    let mut grass_cross = [None::<u64>; 3];

    println!("\n=== 2. main comparison: parallel::step (the app's driver), sealed box, no rain ===");
    println!("frame 0:");
    for (name, x) in &arms {
        println!("  {}", report(name, w.get(*x, surface_y)));
    }
    for f in 1..=12_000u64 {
        parallel::step(&mut w);
        if f <= 3 || f == 10 {
            println!("  [positive control] frame {f}: soil_water_stats = {:?}", w.soil_water_stats);
        }
        for (i, (_, x)) in arms.iter().enumerate() {
            let avail = update::plant_available_fraction(w.get(*x, surface_y));
            if herb_cross[i].is_none() && avail >= HERB_THRESHOLD {
                herb_cross[i] = Some(f);
            }
            if grass_cross[i].is_none() && avail >= GRASS_THRESHOLD {
                grass_cross[i] = Some(f);
            }
        }
        if CHECKPOINTS.contains(&f) {
            println!("frame {f}:");
            for (name, x) in &arms {
                println!("  {}", report(name, w.get(*x, surface_y)));
            }
        }
    }
    println!("\nthreshold crossing (first frame plant_available_fraction >= threshold; None = never in 12,000 frames):");
    for (i, (name, _)) in arms.iter().enumerate() {
        println!(
            "  {:<28} herb(>=0.15): {:?}   grass(>=0.10): {:?}",
            name, herb_cross[i], grass_cross[i]
        );
    }

    // === 3. Full pipeline: does evaporation undo it? ======================
    // Same "hand-placed" shape, but driven by frame::step -- which adds
    // world.step_active_sites() (evaporation included) on top of
    // parallel::step. Reports/dead-ends.md's capillary-fix entry warned the
    // threshold fix alone drains a bed to the wilting point via unbounded
    // evaporation; two more changes (#189, #191) are recorded as the
    // follow-up. This checks whether that follow-up holds at HEAD.
    let mut w2 = World::new(Rect::new(min_x, min_y, max_x, max_y));
    for x in min_x..=max_x {
        w2.set(x, min_y, Cell::new(STONE, 0));
        w2.set(x, max_y, Cell::new(STONE, 0));
    }
    for y in min_y..=max_y {
        w2.set(min_x, y, Cell::new(STONE, 0));
        w2.set(max_x, y, Cell::new(STONE, 0));
    }
    for y in bed_top..=bed_bottom {
        w2.set(bx, y, Cell::new(soil, 0).with_aux(material::SOIL_FIELD_CAPACITY));
    }
    w2.set(bx, surface_y, Cell::new(soil, 0));
    wall_column(&mut w2, bx, surface_y, bed_bottom);
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    println!("\n=== 3. full pipeline: frame::step (adds active sites / evaporation) on the hand-placed arm ===");
    println!("frame 0: {}", report("hand-placed, full pipeline", w2.get(bx, surface_y)));
    for f in 1..=12_000u64 {
        pixel_physics::sim::frame::step(&mut w2, &mut particles, &mut blasts, player::PlayerInput::default(), &player::Tuning::default());
        if CHECKPOINTS.contains(&f) {
            println!("frame {f}: {}", report("hand-placed, full pipeline", w2.get(bx, surface_y)));
        }
    }

    // === 4. The other driver: does moisture move under serial update::step? ===
    let mut w3 = World::new(Rect::new(min_x, min_y, max_x, max_y));
    for x in min_x..=max_x {
        w3.set(x, min_y, Cell::new(STONE, 0));
        w3.set(x, max_y, Cell::new(STONE, 0));
    }
    for y in min_y..=max_y {
        w3.set(min_x, y, Cell::new(STONE, 0));
        w3.set(max_x, y, Cell::new(STONE, 0));
    }
    for y in bed_top..=bed_bottom {
        w3.set(bx, y, Cell::new(soil, 0).with_aux(material::SOIL_FIELD_CAPACITY));
    }
    w3.set(bx, surface_y, Cell::new(soil, 0));
    wall_column(&mut w3, bx, surface_y, bed_bottom);
    println!("\n=== 4. the other driver: update::step (serial), same hand-placed setup, default env ===");
    println!("frame 0: {}", report("hand-placed, serial", w3.get(bx, surface_y)));
    for _ in 0..3000 {
        update::step(&mut w3);
    }
    println!("frame 3000: {}", report("hand-placed, serial", w3.get(bx, surface_y)));
}
