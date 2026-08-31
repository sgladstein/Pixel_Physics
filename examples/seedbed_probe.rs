//! **Can a seed start on what a dead plant leaves behind?**
//!
//! The owner's report, 2026-08-31: *"the ground gets covered in deadwood and
//! new seeds cannot germinate in deadwood and everything dies."* That is a
//! claim about a **gate**, not about mass, so `labmass` cannot answer it —
//! it follows matter, and this follows whether a seed on that matter ever
//! becomes a plant.
//!
//! ```text
//! cargo run --release --example seedbed_probe                    # every mat, against bare soil
//! cargo run --release --example seedbed_probe -- mat=litter      # one mat
//! cargo run --release --example seedbed_probe -- depth=1         # a thinner mat
//! ```
//!
//! # It is a positive control first
//!
//! `mat=none` is bare soil and **must** germinate. Every other arm is read
//! against it and means nothing without it: a probe where no arm germinates
//! is a broken probe, and reads exactly like a total blockade. `CLAUDE.md`
//! records six numbers that were arithmetically correct and about the wrong
//! thing, five of which needed the control that says the instrument can move
//! at all.
//!
//! # It reports the gate's inputs, not just its verdict
//!
//! "0 germinations" says a seed did not start. It does not say which of the
//! two gates stopped it, and they have different fixes:
//!
//! - **The germination gate** reads the *water* in the cell the seed rests
//!   on (`plant::Behavior::Seed`'s `soil_water_threshold`, against
//!   `update::plant_available_fraction`), and it is guarded on
//!   `water_capacity > 0` — a material that holds no water reads as bone dry
//!   however wet the world is.
//! - **The rooting gate** is `plant::growable` on the cell below, which
//!   takes a `Powder` only if its `penetration_resistance` is under the
//!   species' root `penetration_force`.
//!
//! So the probe prints, per mat, the water capacity and penetration
//! resistance it is up against and the root force pushing on it. Those are
//! the numbers a fix would change, and reading them beside the germination
//! count is what turns "it died" into "it died because".
//!
//! **The resistances are printed, not re-evaluated.** Re-implementing
//! `growable` here would make the probe agree with itself rather than with
//! the engine, which is the failure `CLAUDE.md` calls a debug readout that
//! is a function of the thing it debugs.

use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::Lab;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::material::EMPTY;
use pixel_physics::sim::update;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

fn tick(lab: &mut Lab) {
    pixel_physics::sim::frame::step(
        &mut lab.world,
        &mut lab.particles,
        &mut lab.blasts,
        pixel_physics::sim::player::PlayerInput::default(),
        &pixel_physics::sim::player::Tuning::default(),
    );
}

struct Arm {
    mat: String,
    /// Mat cells laid, still standing when the seeds arrive, and left at the
    /// end. A mat that has rotted away is not a mat a seed germinated
    /// through, and only these three numbers tell the two apart.
    mat_laid: usize,
    mat_at_sowing: usize,
    mat_at_end: usize,
    /// How many seeds were actually placed, so a germination count is read
    /// against the number of chances it had rather than against the request.
    sown: usize,
    /// What the seeds that are still seeds are resting on. The scene check:
    /// a mat arm whose seeds rest on `soil` is not measuring the mat.
    resting_on: std::collections::BTreeMap<String, usize>,
    germinated: u64,
    plants: usize,
    cells: usize,
    seeds_left: usize,
    water_capacity: u16,
    penetration: f32,
    /// Plant-available water in the cell each seed is resting on, averaged.
    /// This is the exact quantity the germination gate compares against
    /// `soil_water_threshold`.
    available: f32,
}

fn run(mat: &str, spec: &LabBox, depth: i32, seeds: usize, frames: u64, species: &str) -> Arm {
    let mut lab = Lab::new(spec.clone());
    let w = &mut lab.world;

    // Lay the mat across the middle of the bed, directly on the surface.
    // `ground_y` is the surface the founders are planted on, so the mat
    // occupies the rows just above it.
    // **Full width, not a patch.** A patch was tried first and the arms made
    // no sense: a seed is a `Powder` and rolls, so it leaves the mat and
    // germinates on the bare soil beside it -- which scores the mat as
    // permeable for a reason that has nothing to do with the gate. The
    // shell is 4 cells, so the mat runs wall to wall inside it.
    let (x0, x1) = (5, spec.width - 5);
    let (water_capacity, penetration);
    if mat != "none" {
        let id = w.materials.id_of(mat).unwrap_or_else(|| panic!("no material named {mat}"));
        water_capacity = w.materials.get(id).water_capacity;
        penetration = w.materials.get(id).penetration_resistance;
        let shades = w.materials.get(id).base_shades.max(1) as u32;
        for y in (spec.ground_y - depth)..spec.ground_y {
            for x in x0..x1 {
                let shade = w.rng.below(shades) as u8;
                w.set(x, y, Cell::new(id, shade));
            }
        }
    } else {
        let id = w.materials.id_of("soil").expect("soil");
        water_capacity = w.materials.get(id).water_capacity;
        penetration = w.materials.get(id).penetration_resistance;
    }

    let mat_laid = count_mat(&lab.world, spec, mat);
    // **Settle the mat before sowing.** A powder dropped in and immediately
    // seeded is still moving, so a seed can land in a hole that closes over
    // it -- which would make the mat look permeable for reasons that have
    // nothing to do with the gate under test.
    for _ in 0..600 {
        tick(&mut lab);
    }

    // Sow across the matted span, in the air above it.
    let step = ((x1 - x0) / seeds.max(1) as i32).max(1);
    let mut sown = 0;
    let mut x = x0 + step / 2;
    while sown < seeds && x < x1 {
        // A few rows up, so the seed falls onto the mat the way a shed one
        // would rather than being placed inside it.
        if lab.world.plant_tree_species(x, spec.ground_y - depth - 6, species) {
            sown += 1;
        }
        x += step;
    }

    // **How much mat is still there when the seeds arrive.** The scene
    // check that separates the two explanations for a permeable-looking
    // arm: a mat a seed can start on, and a mat that is no longer there.
    // Litter read 16/16 germinations against deadwood's 0/16 and it is this
    // number, not permeability, that says why.
    let mat_at_sowing = count_mat(&lab.world, spec, mat);
    let before = lab.world.germinations;
    for _ in 0..frames {
        tick(&mut lab);
    }

    // What each surviving seed is actually resting on, read as the gate
    // reads it.
    let mut avail = (0.0f32, 0usize);
    let mut resting_on: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut seeds_left = 0;
    let seed_id = lab.world.materials.id_of("seed");
    for y in 0..spec.height {
        for x in 0..spec.width {
            let c = lab.world.get(x, y);
            if Some(c.material) != seed_id || c.material == EMPTY {
                continue;
            }
            seeds_left += 1;
            let below = lab.world.get(x, y + 1);
            let holds = lab.world.materials.get(below.material).water_capacity > 0;
            avail.0 += if holds { update::plant_available_fraction(below) } else { 0.0 };
            avail.1 += 1;
            // **Is the seed on the thing under test?** `CLAUDE.md`: a scene
            // that contradicts the code looks like a bug in the code. A mat
            // arm in which the seeds are resting on soil is not measuring
            // the mat, and the first run of this probe was exactly that.
            *resting_on.entry(lab.world.materials.get(below.material).name.clone()).or_insert(0) += 1;
        }
    }

    // **Established, not merely alive.** The first version counted live
    // organisms, and an ungerminated seed *is* a live organism with one
    // cell -- so the bare-soil arm reported 168 "plants" of which 143 were
    // seeds that had done nothing. A plant that has started has grown past
    // its seed cell.
    let (mut plants, mut cells) = (0usize, 0usize);
    for id in lab.world.live_organism_ids() {
        let Some(st) = lab.world.organism(id) else { continue };
        if lab.world.species.get(st.species).creature.is_some() {
            continue;
        }
        if st.cells.len() <= 1 {
            continue;
        }
        plants += 1;
        cells += st.cells.len();
    }

    let mat_at_end = count_mat(&lab.world, spec, mat);
    Arm {
        mat_laid,
        mat_at_sowing,
        mat_at_end,
        resting_on,
        sown,
        mat: mat.to_string(),
        germinated: lab.world.germinations - before,
        plants,
        cells,
        seeds_left,
        water_capacity,
        penetration,
        available: avail.0 / avail.1.max(1) as f32,
    }
}

fn main() {
    let depth: i32 = arg("depth").unwrap_or(3);
    let seeds: usize = arg("seeds").unwrap_or(16);
    let frames: u64 = arg("frames").unwrap_or(6_000);
    let species: String = arg("species").unwrap_or_else(|| "herb".to_string());
    let seed: u64 = arg("seed").unwrap_or(1);
    let mats: Vec<String> = arg::<String>("mat")
        .map(|m| vec!["none".to_string(), m])
        .unwrap_or_else(|| ["none", "litter", "deadwood", "soil"].iter().map(|s| s.to_string()).collect());

    let spec = LabBox { founders: 0, colonies: 0, seed, ..LabBox::default() };
    println!(
        "seedbed_probe: mats={mats:?} depth={depth} seeds={seeds} frames={frames} species={species} seed={seed}"
    );

    // The species' root push, which is what `penetration_resistance` is
    // measured against. Printed once because it is the other half of every
    // row below and is not guessable from the material files.
    let force = root_force(&Lab::new(spec.clone()).world, &species);
    println!("  {species} root penetration_force = {force}\n");
    println!(
        "  {:<10} {:>5} {:>6} {:>7} {:>7} {:>7} | {:>9} {:>8} {:>9}",
        "mat", "sown", "germ", "estab", "cells", "seeds", "water_cap", "resist", "avail"
    );

    let mut arms = Vec::new();
    for mat in &mats {
        let a = run(mat, &spec, depth, seeds, frames, &species);
        println!(
            "  {:<10} {:>5} {:>6} {:>7} {:>7} {:>7} | {:>9} {:>8.1} {:>9.3}  resting on {:?}",
            a.mat,
            a.sown,
            a.germinated,
            a.plants,
            a.cells,
            a.seeds_left,
            a.water_capacity,
            a.penetration,
            a.available,
            a.resting_on
        );
        println!(
            "               mat cells: {} laid -> {} when the seeds landed -> {} at the end",
            a.mat_laid, a.mat_at_sowing, a.mat_at_end
        );
        arms.push(a);
    }

    println!("\n=== controls ===");
    let mut all = true;
    let mut ok = |name: &str, pass: bool, said: String| {
        println!("  [{}] {name}: {said}", if pass { "PASS" } else { "FAIL" });
        all &= pass;
    };
    // **The positive control.** Bare soil is the case whose answer is known
    // to be non-zero. Without it a total blockade and a broken probe are the
    // same picture.
    match arms.iter().find(|a| a.mat == "none") {
        Some(bare) => {
            ok("bare soil germinates", bare.germinated > 0, format!("{} germinations", bare.germinated));
            ok("bare soil establishes", bare.plants > 0, format!("{} plants, {} cells", bare.plants, bare.cells));
        }
        None => ok("the positive control ran", false, "no mat=none arm".to_string()),
    }
    for a in arms.iter().filter(|a| a.mat != "none") {
        println!(
            "  [{}] a seed on {} : {} germinations, {} plants",
            if a.germinated > 0 { "starts" } else { "BLOCKED" },
            a.mat,
            a.germinated,
            a.plants
        );
    }
    println!("VERDICT: {}", if all { "controls held" } else { "A CONTROL FAILED" });
}

/// Mat cells still standing anywhere in the bed.
fn count_mat(world: &World, spec: &LabBox, mat: &str) -> usize {
    if mat == "none" {
        return 0;
    }
    let Some(id) = world.materials.id_of(mat) else { return 0 };
    let mut n = 0;
    for y in 0..spec.height {
        for x in 0..spec.width {
            if world.get(x, y).material == id {
                n += 1;
            }
        }
    }
    n
}

/// The species' `RootTip` growth force, which `penetration_resistance` is
/// compared against. Read off the species rather than hardcoded, so the
/// table stays true if a species is retuned.
fn root_force(world: &World, species: &str) -> f32 {
    use pixel_physics::sim::organism::{Behavior, CellType};
    let Some(id) = world.species.id_of(species) else { return 0.0 };
    world
        .species
        .get(id)
        .behaviors(CellType::RootTip)
        .iter()
        .find_map(|b| match b {
            Behavior::Grow { penetration_force, .. } => Some(*penetration_force),
            _ => None,
        })
        .unwrap_or(0.0)
}
