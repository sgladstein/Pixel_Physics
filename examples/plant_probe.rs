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

mod common;

use pixel_physics::sim::organism;
use pixel_physics::sim::parallel;


/// Soil surface row. Defaults to `common::PlantScene`'s, which is chosen
/// so the *plant* is the limit rather than the world's top edge -- see that
/// type's doc for the three times a ceiling-bound scene produced a wrong
/// conclusion. Override with `ground=N` only to deliberately test a
/// constrained world, and check `canopy top` in the output before trusting
/// any shape number from it.
fn ground_y() -> i32 {
    std::env::args()
        .find_map(|a| a.strip_prefix("ground=").map(|v| v.parse().expect("ground")))
        .unwrap_or(common::PlantScene::default().ground_y)
}

fn main() {
    let frames: u64 = std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix("frames=").map(|v| v.parse().expect("frames")))
        .unwrap_or(400);

    // **One scene, shared with `filmstrip`** -- see `common::PlantScene`.
    // These two harnesses used to build their own worlds and were compared
    // as if they matched; they did not, in tree count, spacing, seed
    // placement and soil depth.
    let trees: usize = std::env::args()
        .find_map(|a| a.strip_prefix("trees=").map(|v| v.parse().expect("trees")))
        .unwrap_or(1);
    let scene = common::PlantScene { ground_y: ground_y(), trees, ..Default::default() };
    let (width, height) = (scene.width, scene.height);
    let mut w = scene.build();

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
        for y in 0..height {
            for x in 0..width {
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
    for y in 0..height {
        for x in 0..width {
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
    for y in 0..height {
        let (mut run, mut owner) = (0usize, 0u16);
        for x in 0..=width {
            // Woody cells only. A leaf now sits *beside* the stem it
            // grew from, so counting every organism cell makes a bare
            // stem-plus-leaf pair read as a two-cell-thick trunk -- which
            // took the "rows wider than one cell" figure from 25% to 88%
            // on a change that added no wood at all. Thickness is a
            // question about the woody stem, so `Leaf` is excluded.
            let id = if x < width {
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
        // **The ceiling detector, and the first thing to read.** A run
        // whose canopy reaches row 0 is contaminated: trees that cannot go
        // up spread sideways, which is the "canopies merge into a slab"
        // symptom that was chased as a plant bug across two sessions and
        // three scenes (`forest` at 40 rows of sky, `grove` at 96, and
        // `ground=200`, where median shoot height still measured 203).
        // Discard the shape numbers rather than interpreting them.
        match common::canopy_top(&w) {
            Some(top) if top <= 0 => println!("  canopy top  row {top}   *** CEILING HIT -- shape numbers from this run are void ***"),
            Some(top) => println!("  canopy top  row {top}   ({top} rows of clearance remain)"),
            None => println!("  canopy top  -- nothing grew"),
        }
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
    // **Split above and below ground, and that is not cosmetic.** This
    // scanned every row in the world, so a horizontal *root mat* -- which
    // spreads through soil by design, and which `MAX_ROOT_FRACTION` already
    // bounds -- counted as a canopy slab. Reported as one number it moved
    // for root reasons and was read as a canopy result. That is the third
    // plant metric on this branch to measure something other than its own
    // name; see `PLAN.md` on `rows >1 cell wide` and the vein contrast.
    let widest_run = |rows: std::ops::Range<i32>| -> usize {
        rows.map(|y| {
            let (mut best, mut run) = (0usize, 0usize);
            for x in 0..width {
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
        .unwrap_or(0)
    };
    println!("  thickest contiguous run, above ground: {} cells", widest_run(0..ground_y()));
    println!("  thickest contiguous run, below ground: {} cells (roots spread by design)", widest_run(ground_y()..height));

    if std::env::args().any(|a| a == "dump") {
        println!("{:>5} {:>5}  {:<12} {:>9} {:>9}", "x", "y", "type", "resource", "canopy");
        for (x, y, ty, resource, canopy) in &cells {
            println!("{x:>5} {y:>5}  {:<12} {resource:>9.3} {canopy:>9.3}", format!("{ty:?}"));
        }
    }

    // Vein conductance -- the companion number to `filmstrip channel=vein`.
    // The sheet answers "is there a strand hierarchy and where"; this
    // answers "how much", which a one-cell-wide twig on a ramp cannot.
    {
        let mut conductances: Vec<f32> = Vec::new();
        for &(x, y, _, _, _) in &cells {
            if let Some(cell) = w.organism_cell(x, y) {
                conductances.push(cell.carbon_conductance.iter().copied().fold(f32::MIN, f32::max));
            }
        }
        if !conductances.is_empty() {
            conductances.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
            let at = |q: f32| conductances[((conductances.len() - 1) as f32 * q) as usize];
            let basal = conductances.iter().filter(|c| **c < organism::CONDUCTANCE_MIN * 1.05).count();
            println!(
                "
vein conductance (max face per cell), {}..{}:",
                organism::CONDUCTANCE_MIN,
                organism::CONDUCTANCE_MAX
            );
            println!(
                "  min {:.2}  p50 {:.2}  p90 {:.2}  p99 {:.2}  max {:.2}",
                conductances[0],
                at(0.5),
                at(0.9),
                at(0.99),
                conductances[conductances.len() - 1]
            );
            println!(
                "  {basal}/{} cells still at the basal floor ({:.0}%) -- undifferentiated tissue",
                conductances.len(),
                100.0 * basal as f32 / conductances.len() as f32
            );
            // **Differentiation, not a ratio of percentiles.**
            //
            // `p99/p50` was the first thing reported here and it is
            // actively misleading: it fell from 6.1x to 2.4x across a
            // change that made canalization *better*, because a thicker
            // trunk means more cells legitimately carry flux, which lifts
            // the median. The strands were unchanged -- p99 sat at the
            // ceiling throughout. A ratio against the median measures how
            // much *undifferentiated* tissue happens to be lying around,
            // which is a fact about tree shape, not about the mechanism.
            //
            // What canalization actually claims is a *split*: some tissue
            // becomes vascular and some does not. So report the shape of
            // the distribution -- how much sits at each end -- and let the
            // reader see the split directly.
            let span = organism::CONDUCTANCE_MAX - organism::CONDUCTANCE_MIN;
            let near_floor = conductances.iter().filter(|c| **c < organism::CONDUCTANCE_MIN + span * 0.1).count();
            let near_ceiling = conductances.iter().filter(|c| **c > organism::CONDUCTANCE_MIN + span * 0.9).count();
            let middle = conductances.len() - near_floor - near_ceiling;
            let pct = |n: usize| 100.0 * n as f32 / conductances.len() as f32;
            println!(
                "  differentiation: {:.0}% undifferentiated / {:.0}% partial / {:.0}% vascular",
                pct(near_floor),
                pct(middle),
                pct(near_ceiling)
            );
            println!(
                "  strand strength, p99 vs the basal floor: {:.2}x of a possible {:.0}x",
                at(0.99) / organism::CONDUCTANCE_MIN,
                organism::CANALIZATION_CONTRAST
            );
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
