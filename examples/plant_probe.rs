//! Dumps every organism-owned cell's per-cell channels for a grown tree.
//!
//! The companion to `filmstrip`'s organism overlay, and deliberately second
//! to it: the sheet answers *what and where* (canopy density reads as the
//! ramp floor on every cell of every tile), and this answers *how much* and
//! *why*. Written because the sheet's answer — "zero everywhere, always" —
//! is the same picture a genuinely-zero channel and a channel that is
//! merely decaying faster than it is deposited would both produce, and
//! those need different fixes.
//!
//! ```text
//! cargo run --release --example plant_probe -- frames=200
//! ```

use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::organism;
use pixel_physics::sim::world::World;
use pixel_physics::sim::{material, parallel};

const WIDTH: i32 = 512;
const HEIGHT: i32 = 320;
/// Matches `filmstrip`'s `TREE_GROUND_Y` — see that constant's doc for why
/// the depth is pinned to `field.rs`'s light profile rather than chosen.
const GROUND_Y: i32 = 40;

fn main() {
    let frames: u64 = std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix("frames=").map(|v| v.parse().expect("frames")))
        .unwrap_or(400);

    let mut w = World::new(Rect::new(0, 0, WIDTH - 1, HEIGHT - 1));
    for x in 0..WIDTH {
        for y in GROUND_Y..(GROUND_Y + 6) {
            w.set(x, y, Cell::new(material::STONE, 0));
        }
    }
    // Must stay identical to `filmstrip`'s own `tree` scene, including the
    // puddle, and that is not a cosmetic detail. Without it, row `GROUND_Y
    // - 1` is open air for the tree's whole life and `thicken()` -- which
    // only ever grows left/right into an *empty* neighbour -- spreads along
    // it unopposed, reporting a 19-cell contiguous run at the base. With
    // the puddle, water occupies that row, `world.is_empty` refuses, and
    // the same tree bases out at 2-3 cells.
    //
    // So `SecondaryThicken`'s ground-level behaviour is decided by whatever
    // happens to be lying on the row beside the trunk. Worth knowing before
    // reading any thickness number off this probe, and worth fixing when
    // `thicken()` is next touched: growing sideways along one open row is
    // a pancake, not a trunk.
    // `trees=N` plants N well-separated trees in one world and reports the
    // spread of outcomes rather than one number.
    //
    // **This is not a nicety.** Swapping `plant.rs` from the shared
    // `World::rng` to a per-organism stream -- a change that alters *which*
    // numbers a tree draws, not how many or how they are distributed --
    // moved this scene from 69 cells and 18 leaves to 19 and 6. Same
    // species file, same scene, same frame count. A single run therefore
    // cannot tell "this parameter is better" from "this run drew luckier
    // numbers", which is exactly what `examples/debug_tree_variants.rs`
    // does today with n=1 per variant, and exactly what
    // `Reports/population-dynamics-research.md` §8 argues about ecologies
    // ("single runs prove nothing... acceptance must be over an ensemble").
    // The plant work needs it first, so it grows here.
    let trees: usize = std::env::args()
        .find_map(|a| a.strip_prefix("trees=").map(|v| v.parse().expect("trees")))
        .unwrap_or(1);
    let spacing = WIDTH / (trees as i32 + 1);
    for i in 0..trees as i32 {
        w.plant_tree(spacing * (i + 1), GROUND_Y - 1);
    }
    if trees == 1 {
        w.paint_circle(150, GROUND_Y - 4, 7, material::WATER);
    }

    let mut awake_frames = 0u64;
    for _ in 0..frames {
        parallel::step(&mut w);
        w.step_active_sites();
        w.step_fields();
        if w.active_chunk_count() > 0 {
            awake_frames += 1;
        }
    }

    println!("after {frames} frames: {} active sites, {} awake chunks", w.active_site_count(), w.active_chunk_count());
    println!(
        "chunks were awake on {awake_frames}/{frames} frames ({:.1}%) -- this is how often `diffuse_resource` ran at all, \
since it is dispatched from the CA sweep and the sweep skips settled chunks",
        100.0 * awake_frames as f32 / frames as f32
    );

    let mut cells = Vec::new();
    let mut per_organism: std::collections::BTreeMap<u16, (usize, usize)> = std::collections::BTreeMap::new();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let c = w.get(x, y);
            if c.organism_id() == 0 {
                continue;
            }
            let (ty, resource) = organism::unpack_aux(c.aux());
            let entry = per_organism.entry(c.organism_id()).or_insert((0, 0));
            entry.0 += 1;
            if ty == Some(organism::CellType::Leaf) {
                entry.1 += 1;
            }
            cells.push((x, y, ty, resource, organism::canopy_density(c.aux())));
        }
    }

    if trees > 1 {
        let sizes: Vec<usize> = per_organism.values().map(|v| v.0).collect();
        let leaves: Vec<usize> = per_organism.values().map(|v| v.1).collect();
        let stat = |v: &[usize]| {
            let mut s = v.to_vec();
            s.sort_unstable();
            let sum: usize = s.iter().sum();
            (s.first().copied().unwrap_or(0), s[s.len() / 2], s.last().copied().unwrap_or(0), sum as f32 / s.len().max(1) as f32)
        };
        let (smin, smed, smax, smean) = stat(&sizes);
        let (lmin, lmed, lmax, lmean) = stat(&leaves);
        println!("\nensemble of {} trees, identical species and scene:", per_organism.len());
        println!("  cells  min {smin:>4}  median {smed:>4}  max {smax:>4}  mean {smean:>7.1}");
        println!("  leaves min {lmin:>4}  median {lmed:>4}  max {lmax:>4}  mean {lmean:>7.1}");
        println!("  per-tree sizes  {sizes:?}");
        println!("  per-tree leaves {leaves:?}");
    }

    println!("\n{} organism cells", cells.len());

    // Histogram before the dump: on a tree of any size the per-cell listing
    // is unreadable, and "how many leaves are there" is the question this
    // phase is actually asking. Counted by `CellType` rather than by
    // material, since every one of these paints as plain `wood` today --
    // which is the whole reason the overlay exists.
    let mut counts = std::collections::BTreeMap::new();
    for (_, _, ty, _, _) in &cells {
        *counts.entry(format!("{ty:?}")).or_insert(0usize) += 1;
    }
    for (ty, n) in &counts {
        println!("  {ty:<20} {n:>5}");
    }
    if let (Some(min_x), Some(max_x), Some(min_y), Some(max_y)) = (
        cells.iter().map(|c| c.0).min(),
        cells.iter().map(|c| c.0).max(),
        cells.iter().map(|c| c.1).min(),
        cells.iter().map(|c| c.1).max(),
    ) {
        println!("  bounding box x {min_x}..={max_x}, y {min_y}..={max_y}");
    }

    // How thick the trunk actually is: the longest **contiguous** run of
    // same-organism cells on any one row, which is what `thicken()`'s own
    // `width` term measures and what "one cell thick" means.
    //
    // The first version of this counted every organism cell on a row
    // instead, and reported 22 on a tree that is visibly one cell thick
    // everywhere -- it was counting two separate branches that happen to
    // cross the same height as if they were one wide trunk. Recorded rather
    // than quietly fixed, because a metric that answers a *different*
    // question than its own name claims is the exact failure `CLAUDE.md`
    // catalogues, and this one would have reported `SecondaryThicken` as
    // working when the picture plainly showed it was not.
    let thickest = (0..HEIGHT)
        .map(|y| {
            let (mut best, mut run) = (0usize, 0usize);
            for x in 0..WIDTH {
                if w.get(x, y).organism_id() != 0 {
                    run += 1;
                    best = best.max(run);
                } else {
                    run = 0;
                }
            }
            best
        })
        .max()
        .unwrap_or(0);
    println!("  thickest contiguous run on one row: {thickest} cells");

    if std::env::args().any(|a| a == "dump") {
        println!("{:>5} {:>5}  {:<12} {:>9} {:>9}", "x", "y", "type", "resource", "canopy");
        for (x, y, ty, resource, canopy) in &cells {
            println!("{x:>5} {y:>5}  {:<12} {resource:>9.3} {canopy:>9.3}", format!("{ty:?}"));
        }
    }

    let max_canopy = cells.iter().map(|c| c.4).fold(0.0f32, f32::max);
    let max_resource = cells.iter().map(|c| c.3).fold(0.0f32, f32::max);
    println!("\nmax resource {max_resource:.3} / {:.1}", organism::RESOURCE_SCALE);
    println!("max canopy   {max_canopy:.3} / {:.1}", organism::CANOPY_DENSITY_SCALE);
    // `with_canopy_density` packs into 4 bits, so 15 steps span the scale --
    // the number to compare the decay ladder above against, and the concrete
    // version of `plant-substrate-v2-design.md` §3a's claim that this channel
    // is quantization-limited rather than behaviour-limited.
    println!(
        "one quantization step of canopy density is {:.3} (4 bits, 15 steps)",
        organism::CANOPY_DENSITY_SCALE / 15.0
    );
}
