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
    // **Widen the world rather than crowding more trees into it.** Tree
    // count is nearly free (measured: 8 trees and 32 trees both run 30,000
    // frames in ~101 s, because rayon parallelises across chunks and a
    // sparse stand leaves most cores idle), but packing 32 trees into the
    // default 512 columns puts them 15 cells apart instead of 57, which is
    // a different experiment -- crown shyness is exactly what that spacing
    // decides. Scaling width with tree count keeps the spacing fixed and
    // buys the extra samples honestly.
    let width: i32 = std::env::args()
        .find_map(|a| a.strip_prefix("width=").map(|v| v.parse().expect("width")))
        .unwrap_or_else(|| {
            let d = common::PlantScene::default();
            d.width * (trees as i32).max(1) / d.trees as i32
        });
    let species: String = std::env::args().find_map(|a| a.strip_prefix("species=").map(str::to_string)).unwrap_or_else(|| "tree".to_string());
    let scene = common::PlantScene { ground_y: ground_y(), trees, width, species, ..Default::default() };
    let (width, height) = (scene.width, scene.height);
    let mut w = scene.build();
    // Different worlds grow different individuals: genotypes are drawn
    // from (world seed, germination coordinate), so a genetic-variability
    // study replicates by varying this. Applied before any stepping --
    // germination, where draws happen, has not run yet.
    if let Some(seed) = std::env::args().find_map(|a| a.strip_prefix("worldseed=").map(|v| v.parse().expect("worldseed"))) {
        w.seed = seed;
    }

    // **Echo what this run was actually given, first line of every log.**
    //
    // A three-and-a-half hour megastudy — 3 species x 8 world seeds x 16
    // plants x 45,000 frames — produced eight *byte-identical* logs per
    // species, because it ran a release binary built fourteen minutes
    // before `worldseed=` was added to this file. An unknown argument is
    // silently ignored, so the study looked exactly like a study: 24 runs,
    // 24 logs, sensible numbers, and 3 distinct populations in place of 24.
    //
    // `CLAUDE.md` already carries the asset-side form of this ("editing a
    // `.ron` does nothing until the next build; identical output across
    // settings is the tell"). This is the same rule one level up — the
    // *harness* is as stale-able as the assets it reads — and the cheap
    // defence is not discipline, it is this line: a log that does not name
    // its own seed was written by a binary that never had one.
    println!("plant_probe: species={} trees={trees} frames={frames} worldseed={} width={width}", scene.species, w.seed);

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

        // **Shape, not size — and until now this harness measured only
        // size.** Every figure above is a magnitude: cells, height,
        // thickness, leaves. Three species that differ *only* in scale
        // score as three clearly different species on all of them, which
        // is precisely why the numbers could neither confirm nor refute
        // the owner's reading that "the shrub is a small version of the
        // same tree". A study cannot answer a question it does not
        // measure, and the genetic-variability megastudy was built
        // without these.
        //
        // Two descriptors, both **scale-free by construction** — ratios
        // taken within one individual, so neither can be satisfied by
        // growing the plant bigger:
        //
        //  - **crown profile**: foliage width in five height bands, top
        //    band first, each as a percentage of that plant's widest band.
        //    A fir is wide at the bottom (descending), a bare-boled
        //    broadleaf is top-heavy (ascending), a shrub is flat.
        //  - **foliage centre**: mean leaf height as a fraction of the
        //    plant's own vertical span, 0 at the collar and 1 at the apex.
        //    A bole-then-crown tree sits high; a mound sits mid.
        //  - **foliage share**: leaves as a percentage of the plant's
        //    cells. Not a shape number, but the one that governs whether
        //    the silhouette is set by foliage or by twig, and it was
        //    measured at 3-6% across all three species.
        const BANDS: usize = 5;
        let mut profile: std::collections::BTreeMap<u16, [(i32, i32, usize); BANDS]> = std::collections::BTreeMap::new();
        let mut leaf_centre: std::collections::BTreeMap<u16, (i64, usize)> = std::collections::BTreeMap::new();
        for y in 0..height {
            for x in 0..width {
                let c = w.get(x, y);
                let id = c.organism_id();
                if id == 0 || organism::cell_type(c.aux()) != Some(organism::CellType::Leaf) {
                    continue;
                }
                // **The span is the *shoot*, collar to apex -- not the whole
                // organism.** `per_organism`'s max_y is taken over every cell
                // the organism owns, which includes its roots, so the five
                // bands used to run from canopy top to root tip and the
                // bottom one or two were underground. That is why every
                // species reported exactly 0 in the last band: a constant
                // across three species with different habits was the tell,
                // and it was the metric measuring the wrong object rather
                // than a shared bare bole.
                let Some(&(_, _, min_y, _)) = per_organism.get(&id) else { continue };
                let Some(collar) = w.organism_state(id).and_then(|s| s.collar_y) else { continue };
                if y > collar {
                    continue;
                }
                let span = (collar - min_y + 1).max(1);
                // y grows downward, so band 0 is the top of the plant.
                let band = (((y - min_y) * BANDS as i32) / span).clamp(0, BANDS as i32 - 1) as usize;
                let slot = profile.entry(id).or_insert([(i32::MAX, i32::MIN, 0); BANDS]);
                slot[band].0 = slot[band].0.min(x);
                slot[band].1 = slot[band].1.max(x);
                slot[band].2 += 1;
                let centre = leaf_centre.entry(id).or_insert((0, 0));
                centre.0 += (collar - y) as i64;
                centre.1 += 1;
            }
        }
        // Median across the ensemble of each plant's own normalised
        // profile — a mean would let one huge individual write the shape.
        let mut per_band: [Vec<usize>; BANDS] = Default::default();
        for slot in profile.values() {
            let widest = slot.iter().map(|&(lo, hi, _)| if hi >= lo { (hi - lo + 1) as usize } else { 0 }).max().unwrap_or(0).max(1);
            for (b, &(lo, hi, _)) in slot.iter().enumerate() {
                let wide = if hi >= lo { (hi - lo + 1) as usize } else { 0 };
                per_band[b].push(100 * wide / widest);
            }
        }
        let med = |v: &mut Vec<usize>| {
            v.sort_unstable();
            if v.is_empty() { 0 } else { v[v.len() / 2] }
        };
        let shape: Vec<usize> = per_band.iter_mut().map(med).collect();
        println!("  crown profile (top->bottom, % of each plant's widest band, median): {shape:?}");
        let mut centres: Vec<usize> = per_organism
            .keys()
            .map(|k| {
                let Some(&(sum, n)) = leaf_centre.get(k) else { return 0 };
                let Some(&(_, _, min_y, _)) = per_organism.get(k) else { return 0 };
                let Some(collar) = w.organism_state(*k).and_then(|s| s.collar_y) else { return 0 };
                let span = (collar - min_y + 1).max(1) as i64;
                if n == 0 { 0 } else { (100 * sum / (n as i64 * span)) as usize }
            })
            .collect();
        let foliage_share: Vec<usize> = per_organism.values().map(|v| 100 * v.1 / v.0.max(1)).collect();
        let mut share = foliage_share.clone();
        println!("  foliage centre (0 = collar, 100 = apex, median): {}", med(&mut centres));
        println!("  foliage share  (% of cells that are leaf, median): {}", med(&mut share));

        // **One run is a population now, and that is worth more than a
        // parameter sweep.** `genotype_variance` gives every individual its
        // own draw on four traits, so a stand of N trees is N genomes
        // sharing one scene -- which makes a single run a natural
        // experiment rather than one sample. Printed per tree so the traits
        // can be regressed against the outcome, which is the only way to
        // see *interactions* between them; a one-knob-at-a-time sweep
        // structurally cannot.
        //
        // Slots match `genotype_variance`'s own order: 0 branch_chance,
        // 1 upward_weight, 2 plastochron, 3 turgor_per_cell, 4 pipe_ratio,
        // 5 light_weight.
        let variance = w
            .species
            .get(w.species.id_of("tree").expect("tree"))
            .behaviors(organism::CellType::GrowingTip)
            .iter()
            .find_map(|b| match b {
                organism::Behavior::Grow { genotype_variance, .. } => Some(*genotype_variance),
                _ => None,
            })
            .unwrap_or([0.0; 6]);
        println!("
  per-tree genotype and outcome (variance {variance:?}):");
        println!("  {:>4}  {:>6} {:>6} {:>6} {:>6} {:>6} {:>6}   {:>6} {:>6} {:>6} {:>6}", "id", "branch", "up", "plast", "turgor", "pipe", "light", "cells", "leaves", "height", "stem");
        let mut ids: Vec<u16> = per_organism.keys().copied().collect();
        ids.sort_unstable();
        for id in ids {
            let g = |slot: usize| pixel_physics::sim::plant::genotype(&w, id, slot, variance[slot]);
            println!(
                "  {id:>4}  {:>6.3} {:>6.3} {:>6.3} {:>6.3} {:>6.3} {:>6.3}   {:>6} {:>6} {:>6} {:>6}",
                g(0),
                g(1),
                g(2),
                g(3),
                g(4),
                g(5),
                per_organism.get(&id).map_or(0, |v| v.0),
                per_organism.get(&id).map_or(0, |v| v.1),
                per_organism.get(&id).map_or(0, |v| (v.3 - v.2 + 1) as usize),
                thickest_above_base.get(&id).copied().unwrap_or(0),
            );
        }

        // The architectural event counters, printed beside the picture
        // because a sheet cannot show whether a mechanism fired -- a
        // sympodial run whose counter reads zero is a monopodial tree that
        // happened to fork, and a "conifer" with zero plagiotropic steps
        // is a mislabelled poplar.
        let mut ids: Vec<u16> = per_organism.keys().copied().collect();
        ids.sort_unstable();
        let counters: Vec<String> = ids
            .iter()
            .filter_map(|&id| {
                w.organism_state(id).map(|s| {
                    // The *achieved* mean departure angle, not a count of
                    // how often the scoring ran -- see
                    // `OrganismState::departure_angle_sum`. An 8-neighbour
                    // lattice cannot always give a species the angle it
                    // asked for, and this is the number that says so.
                    let mean_angle = if s.lateral_departures > 0 { s.departure_angle_sum / s.lateral_departures as f32 } else { 0.0 };
                    format!(
                        "{id}: forks {} plag {} rigid {} lat {} angle {mean_angle:.0}deg",
                        s.sympodial_forks, s.plagiotropic_steps, s.rigid_steps, s.lateral_departures
                    )
                })
            })
            .collect();
        println!("  architecture events  [{}]", counters.join(", "));
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

    // **The palette-band counter, and it exists because a picture cannot
    // answer "did this fire".** A banded stand and an unbanded one differ
    // by a few colour bytes per cell; at the zoom a contact sheet is read
    // at they are indistinguishable, which is the exact failure mode that
    // once had a collapse read as "chunks are working" while the body
    // count was zero for the whole run. If a species declares
    // `foliage_bands: (first: 4, count: 2)` and this prints one band, the
    // draw is not reaching the cells.
    let mut band_counts: std::collections::BTreeMap<(String, u8), usize> = std::collections::BTreeMap::new();
    for y in 0..height {
        for x in 0..width {
            let c = w.get(x, y);
            if c.organism_id() == 0 {
                continue;
            }
            let mat = w.materials.get(c.material);
            // **The band a cell actually renders as**, which is not
            // `shade / PALETTE_BAND`: `render.rs` wraps the shade modulo the
            // palette length, so on a four-entry palette (`rootwood`) band 1
            // draws exactly band 0's colours. Reporting the unwrapped index
            // would show `rootwood` split across two bands it does not have
            // and read as a bug in a mechanism that is behaving correctly.
            let effective = (c.shade as usize % mat.palette.len().max(1)) as u8 / organism::PALETTE_BAND;
            *band_counts.entry((mat.name.clone(), effective)).or_insert(0) += 1;
        }
    }
    println!("  palette bands in use (material, band -> cells):");
    for ((name, band), n) in &band_counts {
        println!("    {name:<10} band {band:>2}  {n:>6}");
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

    // What leaves actually read, in noon-equivalent light -- the number
    // every light-driven decision (income, bud break, abscission) is made
    // from. `channel=light` shows *where* the shade is; this says *how
    // much*, which is the half a threshold can be chosen from. Written the
    // first time a shade_death sweep collapsed the stand at every value
    // tried and nobody could say what a healthy leaf reads.
    {
        let mut leaf_light: Vec<f32> = cells
            .iter()
            .filter(|c| c.2 == Some(organism::CellType::Leaf))
            .map(|c| pixel_physics::sim::plant::ambient_light_above(&w, c.0, c.1))
            .collect();
        if !leaf_light.is_empty() {
            leaf_light.sort_by(f32::total_cmp);
            let pct = |p: f32| leaf_light[((leaf_light.len() - 1) as f32 * p) as usize];
            let below = |t: f32| leaf_light.iter().filter(|&&l| l < t).count();
            println!(
                "\nleaf light (noon-equivalent): min {:.2}  p10 {:.2}  p50 {:.2}  p90 {:.2}  max {:.2}",
                leaf_light[0], pct(0.1), pct(0.5), pct(0.9), leaf_light[leaf_light.len() - 1]
            );
            println!(
                "  below 0.5: {}   below 1.0: {}   below 1.5: {}   of {} leaves",
                below(0.5), below(1.0), below(1.5), leaf_light.len()
            );
        }
    }

    let max_canopy = cells.iter().map(|c| c.4).fold(0.0f32, f32::max);
    let max_resource = cells.iter().map(|c| c.3).fold(0.0f32, f32::max);
    println!("\nmax resource {max_resource:.3} / {:.1}", organism::RESOURCE_SCALE);
    println!("max canopy   {max_canopy:.3} / {:.1}", organism::CANOPY_DENSITY_SCALE);
    // The 4-bit packing this used to report on is gone -- `canopy_density`
    // is a plain `f32` on `OrganismCell` since the sidecar migration, so
    // there is no quantum and the decay reaches zero. What is worth
    // printing instead is what a *fresh* deposit is worth, since the
    // interesting reading is how far above that floor a crowded spot sits.
    println!("one fresh canopy deposit is 1.500 (GROW_CANOPY_DEPOSIT), decaying by half per organism tick");
}
