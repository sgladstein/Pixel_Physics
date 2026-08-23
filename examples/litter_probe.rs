//! Where shed litter actually comes to rest, and whether it rots.
//!
//! **The count `Reports/plant-implementation-plan.md`'s WP-B2 deliberately
//! did not run.** That package landed litter, switched all three abscission
//! sites over and closed the decay-site blocker, then recorded: *"Not run:
//! the edible-cells-near-surface count. It is a creature-side quantity and
//! the creature branch sets its own bar when it consumes this."* This is
//! that harness.
//!
//! The question is not "is there litter" — a total says yes and means
//! nothing. `plant.rs::shed_to_litter` writes litter **in place**, at the
//! leaf's own position, and lets the powder fall from there. A leaf shed in
//! the middle of a crown therefore falls onto the first branch under it, not
//! to the floor. So the question is *where it stopped*, and the split that
//! answers it is *what is holding each cell up*:
//!
//!   - **on terrain** — it reached the ground. A walking creature can touch
//!     it; a forest floor accumulates.
//!   - **on plant** — it is sitting on a branch, metres up. It keeps its
//!     chunk's decay schedule alive and feeds nothing that cannot climb.
//!   - **airborne** — still falling this frame.
//!
//! A plain height histogram cannot make that call, because a low branch and
//! a deep floor pile occupy the same rows. Walking down the pile to whatever
//! is actually under it can, and it is the verb in the complaint: *did this
//! come to rest on a branch?*
//!
//! Also prints the 3-rows-above-terrain count, which is the form the
//! creature side's earlier reading took, so the two are comparable.
//!
//! **`canopy top` is printed first and is a gate, not decoration.** Four
//! conclusions in this project came from harnesses that did not contain the
//! situation they claimed to measure, one of them a tree that never grew
//! because the runner omitted the light field. If nothing grew, every number
//! below it is void — and `litter 0` from a bare world is indistinguishable
//! from `litter 0` from a working one.
//!
//! `out=x.png` writes the classification as a picture: **litter resting on a
//! branch in magenta, litter resting on the ground in cyan**, over a world
//! dimmed to a quarter. A plain screenshot cannot show this — litter's
//! palette is browns and so is wood's, deliberately (`litter.ron` keeps the
//! shed leaves close in value so a layer reads as ground texture), and
//! WP-B2 already flagged that the two may be too close to tell apart. So
//! this is a **full replace on fixed colours**, not a tint: a magnitude
//! blend into a brown cell was tried once elsewhere in this engine and
//! produced a sheet that read as blank.
//!
//! ```text
//! cargo run --release --example litter_probe
//! cargo run --release --example litter_probe -- frames=12000 every=2000 trees=8
//! cargo run --release --example litter_probe -- frames=12000 out=/tmp/litter.png
//! ```

mod common;

use pixel_physics::sim::material;
use pixel_physics::sim::parallel;
use pixel_physics::sim::World;

/// What is underneath a litter cell, once you walk down through whatever
/// litter is stacked below it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Rest {
    Terrain,
    Plant,
    Airborne,
}

/// Walk down from `(x, y)` through contiguous litter and report what the
/// bottom of that column is standing on.
///
/// **Through the pile, not one cell.** The cell directly below a litter cell
/// is usually more litter, so a one-cell test answers "is this a pile"
/// rather than "what is holding the pile up". A drift resting on a branch
/// and the same drift on the floor look identical until the walk bottoms
/// out.
fn rest_of(world: &World, litter: pixel_physics::sim::material::MaterialId, x: i32, y: i32, max_y: i32) -> Rest {
    let mut yy = y;
    while yy < max_y {
        let below = world.get(x, yy + 1);
        if below.material == litter {
            yy += 1;
            continue;
        }
        // Raw `material == EMPTY`, not `is_empty()`: the managed-aware
        // helper reads a promoted liquid body's container cells as
        // not-empty, and the question here is "is there material here".
        if below.material == material::EMPTY {
            return Rest::Airborne;
        }
        if below.organism_id() != 0 {
            return Rest::Plant;
        }
        return Rest::Terrain;
    }
    // Bottomed out against the world edge, which is terrain for this
    // purpose — nothing can be resting on a branch down there.
    Rest::Terrain
}

/// Topmost terrain row in column `x` — soil or stone, never plant and never
/// litter.
///
/// Litter is excluded on purpose: a mat of it *becomes* the walkable surface,
/// so including it would make a deep pile measure as "0 rows above the
/// surface" by definition and the metric could never report a pile at all.
fn terrain_top(world: &World, soil: material::MaterialId, x: i32, y0: i32, y1: i32) -> Option<i32> {
    (y0..=y1).find(|&y| {
        let m = world.get(x, y).material;
        m == material::STONE || m == soil
    })
}

fn main() {
    let mut frames = 6000u64;
    let mut every = 1000u64;
    let mut out: Option<String> = None;
    let mut scene = common::PlantScene::default();
    // Echo every parameter below, so a log that does not name its settings
    // was written by a binary that never had them — the megastudy that
    // produced eight byte-identical logs is why this line exists.
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "frames" => frames = v.parse().expect("frames"),
            "every" => every = v.parse().expect("every"),
            "trees" => scene.trees = v.parse().expect("trees"),
            "species" => scene.species = v.to_string(),
            "ground" => scene.ground_y = v.parse().expect("ground"),
            "startframe" => scene.start_frame = v.parse().expect("startframe"),
            "out" => out = Some(v.to_string()),
            other => panic!("unknown arg {other:?}; known: frames, every, trees, species, ground, startframe, out"),
        }
    }
    println!(
        "litter_probe: species={} trees={} frames={} every={} ground={} start_frame={} world={}x{}",
        scene.species, scene.trees, frames, every, scene.ground_y, scene.start_frame, scene.width, scene.height
    );

    let mut world = scene.build();
    let litter = world.materials.id_of("litter").expect("litter is a compiled-in material");
    let soil = world.materials.id_of("soil").expect("soil is a compiled-in material");
    let (w, h) = (scene.width, scene.height);

    println!("  frame | canopy |  litter | on-terrain  on-plant  airborne | <=3 rows | rotted (damp/dry)");
    let sample = |world: &World| {
        let canopy = common::canopy_top(world).map(|y| y.to_string()).unwrap_or_else(|| "NONE".into());
        let (mut total, mut on_terrain, mut on_plant, mut airborne, mut near) = (0u32, 0u32, 0u32, 0u32, 0u32);
        for x in 0..w {
            let top = terrain_top(world, soil, x, 0, h - 1);
            for y in 0..h {
                if world.get(x, y).material != litter {
                    continue;
                }
                total += 1;
                match rest_of(world, litter, x, y, h - 1) {
                    Rest::Terrain => on_terrain += 1,
                    Rest::Plant => on_plant += 1,
                    Rest::Airborne => airborne += 1,
                }
                if let Some(t) = top {
                    if (t - y) <= 3 && y < t {
                        near += 1;
                    }
                }
            }
        }
        println!(
            "  {:>6} | {:>6} | {:>7} | {:>10} {:>9} {:>9} | {:>8} | {} / {}",
            world.frame, canopy, total, on_terrain, on_plant, airborne, near, world.decayed_damp, world.decayed_dry
        );
        (total, on_terrain, on_plant, near)
    };

    let mut run = 0u64;
    sample(&world);
    while run < frames {
        let chunk = every.min(frames - run);
        for _ in 0..chunk {
            parallel::step(&mut world);
            world.step_active_sites();
            world.step_fields();
        }
        run += chunk;
        sample(&world);
    }

    let (total, on_terrain, on_plant, near) = sample(&world);
    let rotted = world.decayed_damp + world.decayed_dry;
    println!();
    println!("  shed cells that ever existed (standing + rotted): {}", total + rotted);
    if total > 0 {
        println!(
            "  standing split: on-terrain {:.1}%  on-plant {:.1}%  |  within 3 rows of terrain {:.1}%",
            100.0 * on_terrain as f32 / total as f32,
            100.0 * on_plant as f32 / total as f32,
            100.0 * near as f32 / total as f32,
        );
    }
    println!("  rotted: {} damp + {} dry = {}", world.decayed_damp, world.decayed_dry, rotted);

    if let Some(path) = out {
        write_overlay(&world, litter, &path, w, h);
        println!("  overlay: {path} (magenta = resting on a branch, cyan = resting on the ground)");
    }
}

/// Paint the same classification the census counts, so the picture and the
/// number are the same quantity rather than two things that have to be
/// argued into agreement.
fn write_overlay(world: &World, litter: material::MaterialId, path: &str, w: i32, h: i32) {
    let mut buf = vec![0u8; (w * h * 3) as usize];
    for x in 0..w {
        for y in 0..h {
            let c = world.get(x, y);
            let i = ((y * w + x) * 3) as usize;
            let rgb = if c.material == litter {
                match rest_of(world, litter, x, y, h - 1) {
                    Rest::Plant => [255u8, 0, 220],
                    Rest::Terrain => [0, 230, 255],
                    Rest::Airborne => [255, 255, 255],
                }
            } else if c.material == material::EMPTY {
                [10, 12, 20]
            } else {
                // A quarter of the material's own colour: enough to read the
                // trunk, the crown and the ground line as context, dim
                // enough that nothing competes with the two markers.
                let pal = &world.materials.get(c.material).palette;
                let raw = pal[c.shade as usize % pal.len().max(1)];
                [raw[0] / 4, raw[1] / 4, raw[2] / 4]
            };
            buf[i..i + 3].copy_from_slice(&rgb);
        }
    }
    image::RgbImage::from_raw(w as u32, h as u32, buf)
        .expect("buffer is w*h*3")
        .save(path)
        .expect("write overlay png");
}
