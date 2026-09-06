//! **Do roots climb out of the ground?** — §W6's remaining question, asked
//! of the scene the owner actually reported it in.
//!
//! The owner, by eye on a `grove` sheet: *"there is an issue where the roots
//! are growing into the tree/sky."* `open-bugs-handoff.md` §W6 traced the
//! chain and fixed the last link — a `RootTip` may now take an `EMPTY` cell
//! only if ground touches it — but the *sighting* was never confirmed gone,
//! and a unit scene cannot confirm it: a root cannot enter `Liquid`, so the
//! climb path has to be **air** while the attractor sits above it, and
//! standing water falls into that path unless something props it up, which
//! then blocks the climb. Only real weather over open terrain has both.
//!
//! ```text
//! cargo run --release --example root_sky -- seeds=6 frames=24000
//! PIXEL_PHYSICS_ROOT_SUBSTRATE=off cargo run --release --example root_sky -- seeds=6 frames=24000
//! ```
//!
//! # What it counts, and why not the obvious thing
//!
//! **Root cells with open sky above them.** Walk up from the cell; if
//! nothing that is not living tissue is met before the top of the world, the
//! root is out of the ground.
//!
//! The obvious metric — *a root cell with no soil touching it* — is wrong,
//! and wrong in a way that looks right. It was tried first and reported **23
//! of 55 root cells "standing in open air"** in a plain walled bed with
//! nothing wrong with it: a root ball dense enough that every neighbour of an
//! interior cell is its own tissue, and the soil it grew through was
//! displaced on the way in. That number reached a code comment before the
//! control caught it. Sky-above cannot make that mistake, because a buried
//! root ball has ground over it.
//!
//! # Paired on the ablation, not across a rebuild
//!
//! `PIXEL_PHYSICS_ROOT_SUBSTRATE=off` restores the pre-fix rule, so both
//! arms come from **one binary** on the same seeds. This line has twice been
//! caught comparing across a rebuild — once when a merge moved the baseline
//! 4,449 -> 2,751 cells with the mechanism still off — and a switch is the
//! only thing that removes the confound rather than measuring around it.

mod common;

use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::frame;
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::organism::{self, CellType};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}="))?.parse().ok())
}

/// Is there open sky above `(x, y)`? Living tissue does not roof anything —
/// a root under its own trunk is still out of the ground.
fn under_open_sky(world: &World, x: i32, y: i32) -> bool {
    let top = world.bounds().map_or(0, |b| b.min_y);
    let mut cy = y - 1;
    while cy >= top {
        let cell = world.get(x, cy);
        if cell.organism_id() == 0
            && matches!(world.materials.kind(cell.material), MaterialKind::Solid | MaterialKind::Powder)
        {
            return false;
        }
        cy -= 1;
    }
    true
}

fn main() {
    let seeds: u64 = arg("seeds").unwrap_or(6);
    let seed0: u64 = arg("seed0").unwrap_or(1);
    let frames: u64 = arg("frames").unwrap_or(24_000);
    let plants: usize = arg("plants").unwrap_or(common::PlantScene::default().trees);

    // Echo the parameters, and the arm. A knob nobody can see the value of
    // is a knob nobody can tell is disconnected -- and this one is an
    // environment variable, which is the easiest kind to run without.
    let rule = !matches!(std::env::var("PIXEL_PHYSICS_ROOT_SUBSTRATE").as_deref(), Ok("off"));
    println!(
        "root_sky: seeds={seed0}..{} frames={frames} plants={plants} \
         arm={}",
        seed0 + seeds - 1,
        if rule { "roots need substrate (shipped)" } else { "OFF -- pre-fix behaviour" }
    );

    let (mut tot_roots, mut tot_sky, mut worst) = (0usize, 0usize, 0usize);
    let (mut tot_above, mut worst_rise_all) = (0usize, 0i32);
    for s in 0..seeds {
        let seed = seed0 + s;
        let mut world = common::PlantScene { trees: plants, seed: Some(seed), ..Default::default() }.build();
        // The shipped driver, not a hand-rolled loop: weather, the field and
        // the plants all have to run or the rain this question is about
        // never falls.
        let mut particles = ParticleSystem::new();
        let mut blasts = Blasts::new();
        let tuning = player::Tuning::default();
        for _ in 0..frames {
            frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        }

        let (mut roots, mut sky) = (0usize, 0usize);
        let mut highest: Option<(i32, i32)> = None;
        // **The other reading of the complaint.** "Growing into the
        // tree/sky" can mean root tissue *in the air*, which `sky` counts,
        // or root tissue *inside the trunk* -- root-type cells well above
        // the plant's own collar, where shoot tissue belongs. They are
        // different defects and they look alike in a sentence, which is
        // exactly the ambiguity `CLAUDE.md` says to resolve by measuring
        // both rather than by picking one and building on it.
        let (mut above_collar, mut worst_rise) = (0usize, 0i32);
        for id in world.live_organism_ids() {
            let Some(state) = world.organism(id) else { continue };
            let collar = state.collar_y;
            let cells: Vec<(i32, i32)> = state.cells.keys().copied().collect();
            for (cx, cy) in cells {
                let cell = world.get(cx, cy);
                if cell.organism_id() != id {
                    continue;
                }
                let is_root = matches!(organism::cell_type(cell.aux()), Some(CellType::RootTip))
                    || world.materials.get(cell.material).reinforces_powder;
                if !is_root {
                    continue;
                }
                roots += 1;
                if let Some(collar) = collar {
                    // Above the collar by more than a couple of cells: a
                    // root mat straddles the collar row normally, so a small
                    // rise is the anatomy, not the defect.
                    let rise = collar - cy;
                    if rise > 2 {
                        above_collar += 1;
                        worst_rise = worst_rise.max(rise);
                    }
                }
                if under_open_sky(&world, cx, cy) {
                    sky += 1;
                    if highest.is_none_or(|(_, hy)| cy < hy) {
                        highest = Some((cx, cy));
                    }
                }
            }
        }
        let pct = if roots > 0 { 100.0 * sky as f64 / roots as f64 } else { 0.0 };
        println!(
            "  seed {seed:>3}: {roots:>6} root cells, {sky:>5} under open sky ({pct:>5.1}%)  highest {highest:?}  \
| {above_collar:>4} inside the shoot, worst {worst_rise:>3} cells above the collar"
        );
        tot_above += above_collar;
        worst_rise_all = worst_rise_all.max(worst_rise);
        tot_roots += roots;
        tot_sky += sky;
        worst = worst.max(sky);
    }
    let pct = if tot_roots > 0 { 100.0 * tot_sky as f64 / tot_roots as f64 } else { 0.0 };
    println!(
        "\n  TOTAL {tot_roots} root cells, {tot_sky} under open sky ({pct:.2}%), worst single seed {worst}\n\
  TOTAL {tot_above} root cells inside the shoot (>2 above the collar), worst rise {worst_rise_all} cells"
    );
}
