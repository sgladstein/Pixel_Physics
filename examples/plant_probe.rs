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
/// Overridable with `ground=N`, because the default is low enough to cap
/// the answer: at `ground=40` there are only 40 rows of sky, and trees
/// median 35 of them — so a height reading there measures the *scene*, not
/// the plant. `field.rs`'s `LIGHT_DECAY` puts `Germinate`'s `0.1` crossing
/// around 75 cells below open sky, so ~70 is the deepest ground that still
/// germinates, and the widest window this question can be asked in.
fn ground_y() -> i32 {
    std::env::args()
        .find_map(|a| a.strip_prefix("ground=").map(|v| v.parse().expect("ground")))
        .unwrap_or(40)
}

fn main() {
    let frames: u64 = std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix("frames=").map(|v| v.parse().expect("frames")))
        .unwrap_or(400);

    let mut w = World::new(Rect::new(0, 0, WIDTH - 1, HEIGHT - 1));
    let soil = w.materials.id_of("soil").expect("soil is compiled in");
    for x in 0..WIDTH {
        for y in (ground_y() + 30)..(ground_y() + 36) {
            w.set(x, y, Cell::new(material::STONE, 0));
        }
        for y in ground_y()..(ground_y() + 30) {
            w.set(x, y, Cell::new(soil, 0).with_aux(material::SOIL_FIELD_CAPACITY));
        }
    }
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
        w.plant_tree(spacing * (i + 1), ground_y() - 1);
    }
    if trees == 1 {
        // The single-tree case must stay identical to `filmstrip`'s own
        // `tree` scene, puddle included, and that is not a cosmetic detail.
        // Without it, row `ground_y() - 1` is open air for the tree's whole
        // life and `thicken()` -- which only ever grows left/right into an
        // *empty* neighbour -- spreads along it unopposed, reporting a
        // 19-cell contiguous run at the base. With the puddle, water
        // occupies that row, `world.is_empty` refuses, and the same tree
        // bases out at 2-3 cells.
        //
        // So `SecondaryThicken`'s ground-level behaviour is decided by
        // whatever happens to be lying on the row beside the trunk. Worth
        // knowing before reading any thickness number off this probe, and
        // worth fixing when `thicken()` is next touched: growing sideways
        // along one open row is a pancake, not a trunk. The ensemble runs
        // without a puddle, so its thickness figures are the *unopposed*
        // case and should be read as an upper bound.
        w.paint_circle(150, ground_y() - 4, 7, material::WATER);
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
    // Soil water profile: what a growing stand actually does to the ground.
    {
        let mut held: Vec<u16> = Vec::new();
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let c = w.get(x, y);
                if w.materials.get(c.material).water_capacity > 0 {
                    held.push(pixel_physics::sim::update::soil_moisture(c));
                }
            }
        }
        if !held.is_empty() {
            held.sort_unstable();
            let sum: u64 = held.iter().map(|&v| v as u64).sum();
            let dry = held.iter().filter(|&&v| v <= 180).count();
            println!(
                "soil: {} cells, min {} median {} max {}, mean {:.0}; {} cells at or below wilting point ({:.0}%)",
                held.len(), held[0], held[held.len() / 2], held[held.len() - 1],
                sum as f64 / held.len() as f64, dry, 100.0 * dry as f64 / held.len() as f64
            );
        }
    }
    println!(
        "chunks were awake on {awake_frames}/{frames} frames ({:.1}%) -- this is how often `diffuse_resource` ran at all, \
since it is dispatched from the CA sweep and the sweep skips settled chunks",
        100.0 * awake_frames as f32 / frames as f32
    );

    let mut cells = Vec::new();
    // Per organism: cells, leaves, min y, max y — height and thickness are
    // the two numbers the owner's original complaint was actually about
    // ("one cell thick", "still a tiny tree"), so they get reported per
    // tree across the ensemble rather than as one run's figure.
    let mut per_organism: std::collections::BTreeMap<u16, (usize, usize, i32, i32)> = std::collections::BTreeMap::new();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let c = w.get(x, y);
            if c.organism_id() == 0 {
                continue;
            }
            let ty = organism::cell_type(c.aux());
            let resource = w.carbon_at(x, y);
            let entry = per_organism.entry(c.organism_id()).or_insert((0, 0, i32::MAX, i32::MIN));
            entry.0 += 1;
            if ty == Some(organism::CellType::Leaf) {
                entry.1 += 1;
            }
            entry.2 = entry.2.min(y);
            entry.3 = entry.3.max(y);
            cells.push((x, y, ty, resource, w.canopy_density_at(x, y)));
        }
    }

    // Thickest contiguous run per organism, so a wide row is only counted
    // when it belongs to one tree -- the same distinction the whole-world
    // measure got wrong once already (see `thickest` below).
    let mut thickest_per_organism: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
    // Same measure, but ignoring the three rows sitting on the ground. The
    // whole-tree figure is dominated by `thicken()` spreading sideways
    // along the open row at the trunk's foot, which is a pancake rather
    // than a trunk; this one answers "how thick is the stem *above* its
    // base", which is what "one cell thick" was ever about.
    let mut thickest_above_base: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
    // "Thickest" is a *max over rows*: a tree with one 5-wide row and sixty
    // 1-wide rows scores 5, which reads as "5 cells thick" and is not what
    // anyone means by it. This counts what share of a tree's occupied rows
    // are wider than one cell -- the difference between a tapered trunk and
    // a whip with a lump on it.
    let mut rows_total: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
    let mut rows_wide: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
    for y in 0..HEIGHT {
        let (mut run, mut owner) = (0usize, 0u16);
        for x in 0..=WIDTH {
            // Woody cells only. A leaf now sits *beside* the stem it
            // grew from, so counting every organism cell makes a bare
            // stem-plus-leaf pair read as a two-cell-thick trunk -- which
            // took the "rows wider than one cell" figure from 25% to 88%
            // on a change that added no wood at all. Thickness is a
            // question about the woody stem, so `Leaf` is excluded.
            let id = if x < WIDTH {
                let c = w.get(x, y);
                if organism::cell_type(c.aux()) == Some(organism::CellType::Leaf) { 0 } else { c.organism_id() }
            } else {
                0
            };
            if id != 0 && id == owner {
                run += 1;
            } else {
                if owner != 0 {
                    let e = thickest_per_organism.entry(owner).or_insert(0);
                    *e = (*e).max(run);
                    if y < ground_y() - 3 {
                        let e = thickest_above_base.entry(owner).or_insert(0);
                        *e = (*e).max(run);
                    }
                    *rows_total.entry(owner).or_insert(0) += 1;
                    if run > 1 {
                        *rows_wide.entry(owner).or_insert(0) += 1;
                    }
                }
                owner = id;
                run = 1;
            }
        }
    }

    if trees > 1 {
        let sizes: Vec<usize> = per_organism.values().map(|v| v.0).collect();
        let leaves: Vec<usize> = per_organism.values().map(|v| v.1).collect();
        let heights: Vec<usize> = per_organism.values().map(|v| (v.3 - v.2 + 1) as usize).collect();
        let thicks: Vec<usize> = per_organism.keys().map(|k| thickest_per_organism.get(k).copied().unwrap_or(0)).collect();
        let thicks_up: Vec<usize> = per_organism.keys().map(|k| thickest_above_base.get(k).copied().unwrap_or(0)).collect();
        let stat = |v: &[usize]| {
            let mut s = v.to_vec();
            s.sort_unstable();
            let sum: usize = s.iter().sum();
            (s.first().copied().unwrap_or(0), s[s.len() / 2], s.last().copied().unwrap_or(0), sum as f32 / s.len().max(1) as f32)
        };
        let (smin, smed, smax, smean) = stat(&sizes);
        let (lmin, lmed, lmax, lmean) = stat(&leaves);
        println!("\nensemble of {} trees, identical species and scene:", per_organism.len());
        let (hmin, hmed, hmax, hmean) = stat(&heights);
        let (tmin, tmed, tmax, tmean) = stat(&thicks);
        println!("  cells     min {smin:>4}  median {smed:>4}  max {smax:>4}  mean {smean:>7.1}");
        println!("  leaves    min {lmin:>4}  median {lmed:>4}  max {lmax:>4}  mean {lmean:>7.1}");
        println!("  height    min {hmin:>4}  median {hmed:>4}  max {hmax:>4}  mean {hmean:>7.1}   (scene ceiling: {} rows of sky)", ground_y());
        let wide_pct: Vec<usize> = per_organism
            .keys()
            .map(|k| {
                let t = rows_total.get(k).copied().unwrap_or(1).max(1);
                100 * rows_wide.get(k).copied().unwrap_or(0) / t
            })
            .collect();
        let (wmin, wmed, wmax, wmean) = stat(&wide_pct);
        let (umin, umed, umax, umean) = stat(&thicks_up);
        println!("  thickness min {tmin:>4}  median {tmed:>4}  max {tmax:>4}  mean {tmean:>7.1}   (whole tree, incl. ground-level spread)");
        println!("  stem thick min {umin:>3}  median {umed:>4}  max {umax:>4}  mean {umean:>7.1}   (above the base -- the real trunk)");
        println!("  rows >1 cell wide, as % of each tree's occupied rows: min {wmin}  median {wmed}  max {wmax}  mean {wmean:.0}");
        println!("  per-tree sizes     {sizes:?}");
        println!("  per-tree heights   {heights:?}");
        println!("  per-tree thickness {thicks:?}");
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
