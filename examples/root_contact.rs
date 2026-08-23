//! **How much of a root system is actually touching soil?**
//!
//! Written to size a mechanism *before* anyone builds it, per `CLAUDE.md`'s
//! "check that a planned step can demonstrate itself, before promising it
//! will".
//!
//! The owner's direction, on the card that showed two root treatments
//! converging into one dense mass by 43,200 frames:
//!
//! > *"There should be a disadvantage for growing a big blob of roots that
//! > fully fills in all space. If the root cell isnt touching soil it cannot
//! > benefit the plant and has a cost ... As usual we don't want to force the
//! > roots to grow a certain way but set up a system that leads to
//! > interesting and heterogenous results, not every plant root eventually
//! > grows into the same blob that is worst case"*
//!
//! That is a proposal about *uptake surface*: a root cell walled in by its
//! own siblings shares no face with soil, so it can absorb nothing, while
//! still costing carbon to build and maintain. If it is right, the interior
//! of a blob is dead weight and the blob is self-limiting without any rule
//! saying so.
//!
//! **Whether it bites is a number, not an opinion**, and this prints it:
//! the fraction of an organism's root cells with at least one *soil* face,
//! sampled as the system matures. A fraction that stays near 1.0 means the
//! proposed cost would touch almost nothing and the lever is a rounding
//! error; a fraction that falls as the mass grows means the cost lands
//! exactly where the owner wants it, and lands harder the blobbier the
//! plant gets.
//!
//! Four-neighbour contact is the headline, because an exchange crosses a
//! shared *face* — `CLAUDE.md`'s note on why `diffuse_resource` stays at 4
//! while growth places at 8. The eight-neighbour figure is printed beside it
//! so the choice is visible rather than assumed.
//!
//! ```text
//! cargo run --release --example root_contact
//! cargo run --release --example root_contact -- species=tree soil=100 frames=43200
//! ```
//!
//! **This measures; it does not implement.** No root code is changed here —
//! `Absorb` is `plant.rs`'s and belongs to the plant-core lane.

mod common;

use pixel_physics::sim::material;
use pixel_physics::sim::organism::{self, CellType};
use pixel_physics::sim::world::World;
use std::collections::BTreeMap;

/// One organism's root system, as an uptake surface rather than a mass.
struct Contact {
    cells: usize,
    /// Root cells sharing at least one face with a soil cell.
    touching_4: usize,
    /// ...and the same over all eight neighbours, for comparison.
    touching_8: usize,
    /// Root cells whose entire 4-neighbourhood is other root cells — the
    /// interior of the blob, and what the owner's proposal would charge for.
    walled_in: usize,
}

fn census(world: &World, soil: material::MaterialId) -> BTreeMap<u16, Contact> {
    let bounds = world.bounds().expect("bounded world");
    let mut out: BTreeMap<u16, Contact> = BTreeMap::new();
    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            let cell = world.get(x, y);
            let id = cell.organism_id();
            if id == 0 {
                continue;
            }
            // Root cells only. `RootTip` is the growing end; the rest of a
            // root matures into `MatureBody` while keeping the root
            // material, so material is the reliable discriminator and cell
            // type alone is not.
            let is_root = match organism::cell_type(cell.aux()) {
                Some(CellType::RootTip) => true,
                Some(_) => world.organism(id).is_some_and(|s| {
                    let sp = world.species.get(s.species);
                    world.materials.id_of(&sp.root_material) == Some(cell.material)
                }),
                None => false,
            };
            if !is_root {
                continue;
            }
            let faces = [(0, 1), (0, -1), (1, 0), (-1, 0)];
            let diagonals = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
            let soil_at = |dx: i32, dy: i32| world.get(x + dx, y + dy).material == soil;
            let root_at = |dx: i32, dy: i32| {
                let n = world.get(x + dx, y + dy);
                n.organism_id() != 0 && n.material == cell.material
            };
            let t4 = faces.iter().any(|&(dx, dy)| soil_at(dx, dy));
            let t8 = t4 || diagonals.iter().any(|&(dx, dy)| soil_at(dx, dy));
            let walled = faces.iter().all(|&(dx, dy)| root_at(dx, dy));
            let e = out.entry(id).or_insert(Contact { cells: 0, touching_4: 0, touching_8: 0, walled_in: 0 });
            e.cells += 1;
            e.touching_4 += usize::from(t4);
            e.touching_8 += usize::from(t8);
            e.walled_in += usize::from(walled);
        }
    }
    out
}

fn main() {
    let arg = |k: &str| std::env::args().find_map(|a| a.strip_prefix(k).map(str::to_string));
    let species = arg("species=").unwrap_or_else(|| "tree".to_string());
    let soil_depth: i32 = arg("soil=").map_or(100, |v| v.parse().expect("soil=N"));
    let frames: u64 = arg("frames=").map_or(43200, |v| v.parse().expect("frames=N"));
    let trees: usize = arg("trees=").map_or(8, |v| v.parse().expect("trees=N"));

    // The harness names its own parameters -- `CLAUDE.md`, the megastudy
    // that was three populations wearing 24 logs.
    println!("root_contact: species={species} trees={trees} frames={frames} soil={soil_depth}");

    let scene = common::PlantScene {
        species: species.clone(),
        trees,
        soil_depth,
        ..common::PlantScene::default()
    };
    let mut world = scene.build();
    let soil = world.materials.id_of("soil").expect("soil is compiled in");

    // Sampled as it matures rather than once at the end: the question is
    // whether contact *falls* as the mass grows, which a single reading
    // cannot answer.
    let checkpoints: Vec<u64> = [10_800u64, 25_200, 43_200].into_iter().filter(|&f| f <= frames).chain(Some(frames)).collect();
    let mut seen = Vec::new();
    println!("\n  frames   plants   root cells   touching soil (4-nbr)   (8-nbr)   walled in");
    for target in checkpoints {
        if seen.contains(&target) {
            continue;
        }
        seen.push(target);
        while world.frame < target {
            pixel_physics::sim::parallel::step(&mut world);
            world.step_active_sites();
            world.step_fields();
        }
        let c = census(&world, soil);
        let cells: usize = c.values().map(|v| v.cells).sum();
        let t4: usize = c.values().map(|v| v.touching_4).sum();
        let t8: usize = c.values().map(|v| v.touching_8).sum();
        let walled: usize = c.values().map(|v| v.walled_in).sum();
        let pct = |n: usize| 100.0 * n as f32 / cells.max(1) as f32;
        println!(
            "  {target:>6}   {:>6}   {cells:>10}   {:>19.1}%   {:>6.1}%   {:>8.1}%",
            c.len(),
            pct(t4),
            pct(t8),
            pct(walled)
        );
    }

    // Per plant at the final sample, because one plant is one draw and a
    // stand-wide percentage hides whether the blob is universal or is one
    // individual dragging the mean (`plant-species-authoring.md` §7).
    let c = census(&world, soil);
    let mut per: Vec<(usize, f32)> = c.values().map(|v| (v.cells, 100.0 * v.touching_4 as f32 / v.cells.max(1) as f32)).collect();
    per.sort_by(|a, b| a.0.cmp(&b.0));
    println!("\n  per plant at {frames}, smallest root system first:");
    for (cells, pct) in &per {
        println!("    {cells:>6} root cells   {pct:>5.1}% touching soil");
    }
}
