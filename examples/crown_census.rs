//! **What are the brown cells in the crown?** — a census by material and by
//! height band, written to answer one owner complaint without guessing.
//!
//! The complaint, on card `20260824T014630073Z-a10698`: *"the soil build-up
//! in between the branches is horrible."* At contact-sheet zoom, soil,
//! litter, deadwood and thickened wood are all mid-brown speckle and the
//! four are indistinguishable by eye — which is exactly the case
//! `CLAUDE.md` says needs a counter rather than a picture.
//!
//! Prints every material present above the ground line, split into bands, so
//! "soil in the canopy" is a number rather than a reading of a colour.
//!
//! ```text
//! cargo run --release --example crown_census -- frames=28800 trees=8
//! ```
mod common;

use pixel_physics::sim::material;
use std::collections::BTreeMap;

fn main() {
    let arg = |k: &str| std::env::args().find_map(|a| a.strip_prefix(k).map(str::to_string));
    let frames: u64 = arg("frames=").map_or(28800, |v| v.parse().expect("frames=N"));
    let trees: usize = arg("trees=").map_or(8, |v| v.parse().expect("trees=N"));
    let scene = common::PlantScene { trees, ..common::PlantScene::default() };
    let ground = scene.ground_y;
    let mut w = scene.build();
    if let Some(seed) = arg("worldseed=") {
        w.seed = seed.parse().expect("worldseed=N");
    }
    println!("crown_census: trees={trees} frames={frames} worldseed={} ground={ground}", w.seed);
    while w.frame < frames {
        pixel_physics::sim::parallel::step(&mut w);
        w.step_active_sites();
        w.step_fields();
    }
    let b = w.bounds().expect("bounded world");
    // Bands of 40 rows above the ground line, so "in the canopy" and "on the
    // forest floor" are different rows rather than one number.
    let mut bands: BTreeMap<i32, BTreeMap<String, usize>> = BTreeMap::new();
    for y in b.min_y..ground {
        let band = (ground - y - 1) / 40;
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            if c.material == material::EMPTY {
                continue;
            }
            let name = w.materials.get(c.material).name.clone();
            *bands.entry(band).or_default().entry(name).or_insert(0) += 1;
        }
    }
    // **The row profile, because a band cannot tell a rising floor from a
    // blob lodged in a crown** — and those two want different fixes. A
    // contiguous run of rows all holding soil is a forest floor that has
    // grown upward; isolated rows with soil far above an empty gap are
    // material that never fell.
    let mut soil_by_row: Vec<(i32, usize)> = Vec::new();
    let soil_id = w.materials.id_of("soil");
    for y in b.min_y..ground {
        let n = (b.min_x..=b.max_x).filter(|&x| Some(w.get(x, y).material) == soil_id).count();
        if n > 0 {
            soil_by_row.push((y, n));
        }
    }
    if let (Some(&(top, _)), Some(&(bottom, _))) = (soil_by_row.first(), soil_by_row.last()) {
        let rows_with_soil = soil_by_row.len();
        let span = bottom - top + 1;
        println!(
            "\nsoil above the ground line: {} cells over {rows_with_soil} rows, from y {top} to y {bottom} (a span of {span} rows, so {} of them hold none)",
            soil_by_row.iter().map(|&(_, n)| n).sum::<usize>(),
            span as usize - rows_with_soil
        );
        println!("  highest ten rows holding any soil (row: cells):");
        for &(y, n) in soil_by_row.iter().take(10) {
            println!("    y {y:>3}: {n:>4}");
        }
    }
    println!("\nrows above the ground line, in bands of 40 (band 0 is the 40 rows just above the soil):");
    for (band, mats) in &bands {
        let total: usize = mats.values().sum();
        let mut v: Vec<_> = mats.iter().collect();
        v.sort_by(|a, b| b.1.cmp(a.1));
        let rows = format!("y {}..{}", ground - (band + 1) * 40, ground - band * 40);
        println!("  band {band} ({rows:>14}): {total:>6} cells   {}", v.iter().map(|(k, n)| format!("{k} {n}")).collect::<Vec<_>>().join(", "));
    }
}
