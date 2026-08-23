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
    // Soil depth, so the paired deep-vs-thin comparison is one flag rather
    // than a recompile -- see `PlantScene::soil_depth`.
    let soil_depth: i32 = std::env::args()
        .find_map(|a| a.strip_prefix("soil=").map(|v| v.parse().expect("soil")))
        .unwrap_or(common::SOIL_DEPTH);
    // Wet/dry is a flag rather than a recompile, and the frame pin makes
    // the weather reproducible -- see `PlantScene::soil_moisture` and
    // `start_frame` for why each matters.
    let soil_moisture: u16 = std::env::args()
        .find_map(|a| a.strip_prefix("moisture=").map(|v| v.parse().expect("moisture")))
        .unwrap_or(pixel_physics::sim::material::SOIL_FIELD_CAPACITY);
    let start_frame: u64 = std::env::args()
        .find_map(|a| a.strip_prefix("frame0=").map(|v| v.parse().expect("frame0")))
        .unwrap_or(0);
    let scene = common::PlantScene {
        ground_y: ground_y(),
        trees,
        width,
        species,
        soil_depth,
        soil_moisture,
        start_frame,
        ..Default::default()
    };
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
    println!(
        "plant_probe: species={} trees={trees} frames={frames} worldseed={} width={width} soil={}",
        scene.species, w.seed, scene.soil_depth
    );

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

    // **Population first, and the ensemble only over *established* plants.**
    //
    // Reproduction changed what `per_organism` contains. A stand of 8 planted
    // trees now holds hundreds of organisms, most of them single-cell seeds
    // lying on the ground waiting for light -- so every ensemble statistic
    // taken over the raw map reports the seed bank, not the trees. It read
    // `cells median 1` on a run whose canopy was plainly full.
    //
    // A seed is an organism and should be counted as one; it is just not a
    // *plant* yet, and mixing the two answers neither question. Population
    // and generations are reported on their own below, and the ensemble is
    // filtered to anything that actually grew.
    const ESTABLISHED: usize = 20;
    {
        let seeds = per_organism.values().filter(|v| v.0 < ESTABLISHED).count();
        let grown = per_organism.len() - seeds;
        let mut gens: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
        for id in per_organism.keys() {
            if let Some(st) = w.organism_state(*id) {
                *gens.entry(st.generation).or_insert(0) += 1;
            }
        }
        let total_seeds: u32 = per_organism.keys().filter_map(|id| w.organism_state(*id).map(|s| s.seeds_set)).sum();
        println!(
            "
population: {} organisms -- {grown} established (>= {ESTABLISHED} cells), {seeds} seeds or seedlings; {total_seeds} seeds set in total",
            per_organism.len()
        );
        let hist: Vec<String> = gens.iter().map(|(g, n)| format!("gen {g}: {n}")).collect();
        println!("  generations  [{}]", hist.join(", "));

        // **Epiphytes: plants rooted above the ground.** A seed is a
        // `Powder`, so it falls -- but it comes to rest on the first thing
        // that stops it, and in a closed stand that is very often a branch.
        // It then germinates there and grows from a collar high in another
        // plant's canopy, which reads on a sheet as "a tree growing out of a
        // tree" and is not something any rule intends.
        //
        // Counted rather than eyeballed because the two readings a contact
        // sheet cannot separate are "a tall tree behind a short one" and "a
        // tree standing on one". A collar well above the soil surface
        // settles it.
        let ground = ground_y();
        let mut epiphytes = 0usize;
        let mut deep = 0usize;
        // Established plants only. Counting every organism included seeds
        // still lying in the branches they landed on, which is a different
        // fact -- a perched seed is not an epiphyte until it germinates,
        // and conflating them made the fix look like it had done nothing.
        for (id, v) in per_organism.iter() {
            if v.0 < ESTABLISHED {
                continue;
            }
            if let Some(collar) = w.organism_state(*id).and_then(|s| s.collar_y) {
                if collar < ground - 3 {
                    epiphytes += 1;
                    if collar < ground - 25 {
                        deep += 1;
                    }
                }
            }
        }
        println!("  rooted above ground: {epiphytes} organisms ({deep} of them more than 25 rows up)");

        // **The clustering readout, and the whole point of discrete loci.**
        //
        // A continuous genome gives a Gaussian cloud however long it runs --
        // a spectrum, which is exactly what was not wanted. Discrete alleles
        // are supposed to make a population sit on a few combinations. That
        // claim is only checkable as a *distribution over genotypes*: a
        // healthy result is a handful of combinations holding most of the
        // population, and the failure it must distinguish is every
        // combination present in roughly equal numbers, which is a smear
        // wearing integers.
        //
        // Counted over established plants only -- the seed bank is mostly
        // one generation of unselected offspring and would swamp whatever
        // selection has actually kept.
        let mut morphs: std::collections::BTreeMap<[u8; organism::DISCRETE_LOCI], usize> = std::collections::BTreeMap::new();
        for (id, v) in per_organism.iter() {
            if v.0 < ESTABLISHED {
                continue;
            }
            if let Some(st) = w.organism_state(*id) {
                *morphs.entry(st.alleles).or_insert(0) += 1;
            }
        }
        let mut ranked: Vec<_> = morphs.iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(a.1));
        let established_total: usize = ranked.iter().map(|(_, n)| **n).sum();
        println!(
            "  morphs among established plants: {} distinct of {} plants  [foliage, angle, internode, sympodial, tropism]",
            ranked.len(),
            established_total
        );
        for (alleles, n) in ranked.iter().take(8) {
            println!("    {alleles:?}  x{n}");
        }
    }
    per_organism.retain(|_, v| v.0 >= ESTABLISHED);

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
        // Slots follow `organism::GENOTYPE_TRAITS`' map (positional
        // forever): 0 shoot branch, 1 root branch, 2 plastochron, 3
        // turgor, 4 pipe, 5 root tropism gain, 6 allocation bias, 7
        // stomatal closure, 8 penetration. Each column's variance comes
        // from the vector its consumer actually reads -- the shoot Grow
        // for 0/2/3/4/6/7, the RootTip Grow for 1/5/8 -- and from the
        // run's own species: this used to hardcode "tree", so every
        // conifer and shrub table printed multipliers scaled by the
        // wrong widths.
        let table_species = w.species.id_of(&scene.species).expect("species is compiled in");
        let vector_of = |ct: organism::CellType| {
            w.species
                .get(table_species)
                .behaviors(ct)
                .iter()
                .find_map(|b| match b {
                    organism::Behavior::Grow { genotype_variance, .. } => Some(*genotype_variance),
                    _ => None,
                })
                .unwrap_or([0.0; organism::GENOTYPE_TRAITS])
        };
        let shoot_v = vector_of(organism::CellType::GrowingTip);
        let root_v = vector_of(organism::CellType::RootTip);
        let variance: Vec<f32> =
            (0..organism::GENOTYPE_TRAITS).map(|s| if matches!(s, 1 | 5 | 8) { root_v[s] } else { shoot_v[s] }).collect();
        println!("
  per-tree genotype and outcome (variance {variance:?}):");
        println!(
            "  {:>4}  {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6}   {:>6} {:>6} {:>6} {:>6}",
            "id", "branch", "rootbr", "plast", "turgor", "pipe", "roottr", "alloc", "stoma", "penetr", "cells", "leaves", "height", "stem"
        );
        let mut ids: Vec<u16> = per_organism.keys().copied().collect();
        ids.sort_unstable();
        for id in ids {
            let g = |slot: usize| pixel_physics::sim::plant::genotype(&w, id, slot, variance[slot]);
            println!(
                "  {id:>4}  {:>6.3} {:>6.3} {:>6.3} {:>6.3} {:>6.3} {:>6.3} {:>6.3} {:>6.3} {:>6.3}   {:>6} {:>6} {:>6} {:>6}",
                g(0),
                g(1),
                g(2),
                g(3),
                g(4),
                g(5),
                g(6),
                g(7),
                g(8),
                per_organism.get(&id).map_or(0, |v| v.0),
                per_organism.get(&id).map_or(0, |v| v.1),
                per_organism.get(&id).map_or(0, |v| (v.3 - v.2 + 1) as usize),
                thickest_above_base.get(&id).copied().unwrap_or(0),
            );
        }

        // **The morph census.** Allele frequencies per discrete locus,
        // counts by allele index -- a cluster shows here long before any
        // sheet can show it, and a column that never moves across a long
        // breeding run is a lever selection is not pulling. Loci in slot
        // order (organism::LOCUS_*): economy, angle, internode, sympody,
        // tropism, density.
        let mut allele_counts = [[0u32; 3]; organism::DISCRETE_LOCI];
        for id in per_organism.keys() {
            if let Some(s) = w.organism_state(*id) {
                for (locus, &a) in s.alleles.iter().enumerate() {
                    allele_counts[locus][(a as usize).min(2)] += 1;
                }
            }
        }
        let census: Vec<String> = allele_counts
            .iter()
            .zip(["economy", "angle", "internode", "sympody", "tropism", "density"])
            .map(|(c, name)| format!("{name} {}/{}/{}", c[0], c[1], c[2]))
            .collect();
        println!("  alleles: {}", census.join("  "));

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
                        "{id}: gen {} seeds {} forks {} plag {} rigid {} lat {} angle {mean_angle:.0}deg",
                        s.generation, s.seeds_set, s.sympodial_forks, s.plagiotropic_steps, s.rigid_steps, s.lateral_departures
                    )
                })
            })
            .collect();
        println!("  architecture events  [{}]", counters.join(", "));
    }

    // **Can a number say what the owner's eye says? — §Z / review item C4.**
    //
    // The standing verdict on the stand is "everything has merged together
    // into a big mass. I cannot identify individual trees", and the metric
    // that reported success on the same picture was the widest unbroken run
    // of plant cells: 39 against a 56-cell founder spacing, i.e. no row is
    // continuous across two crowns. That number was correct and the
    // conclusion drawn from it was wrong -- crowns interleave with one- and
    // two-cell gaps, so every row breaks and the eye still reads one mass.
    // A contiguous-run metric measures whether crowns *touch*; it cannot
    // measure whether they are *distinguishable*.
    //
    // §Z names two candidates and neither had been built. Both are here,
    // because they fail differently: components can merge through a single
    // shared block, and a gap census cannot see a crown that is separate
    // but hidden behind another.
    //
    // **Calibration, and it is the whole point.** These are not reported as
    // good or bad; they are reported against the two answered cards. On the
    // default 8-tree stand at the frame the absolute card was taken, a
    // metric that tracks the eye must read *materially fewer than 8*. A
    // metric that reports 8 has reproduced the contiguous-run failure and
    // must be said so of, not tuned until it agrees.
    {
        // The field's own resolution (`field::FIELD_SCALE`), which is
        // §Z's proposal: at cell resolution two crowns a single empty
        // column apart are two components, and the eye does not see them
        // that way.
        const BLOCK: i32 = 8;
        let ground = ground_y();
        let (bw, bh) = ((width + BLOCK - 1) / BLOCK, (ground + BLOCK - 1) / BLOCK);
        let mut foliage = vec![0u32; (bw * bh) as usize];
        for y in 0..ground {
            for x in 0..width {
                let c = w.get(x, y);
                if c.organism_id() != 0 && organism::cell_type(c.aux()) == Some(organism::CellType::Leaf) {
                    foliage[((y / BLOCK) * bw + x / BLOCK) as usize] += 1;
                }
            }
        }
        // Threshold swept rather than chosen. One leaf in an 8x8 block is
        // the literal reading of "at field resolution", and it is also the
        // most fusible; if the count only drops below the founder count at
        // a threshold nobody can justify, that is the metric failing and
        // the sweep is what shows it.
        let components = |min_leaves: u32| -> (usize, usize) {
            let solid: Vec<bool> = foliage.iter().map(|&n| n >= min_leaves).collect();
            let mut seen = vec![false; solid.len()];
            let (mut n, mut largest) = (0usize, 0usize);
            for start in 0..solid.len() {
                if !solid[start] || seen[start] {
                    continue;
                }
                n += 1;
                let mut size = 0usize;
                let mut stack = vec![start];
                seen[start] = true;
                while let Some(i) = stack.pop() {
                    size += 1;
                    let (bx, by) = ((i as i32) % bw, (i as i32) / bw);
                    // 8-connected: a crown touching another only at a
                    // corner is one mass to the eye, and 4-connectivity
                    // would call it two.
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let (nx, ny) = (bx + dx, by + dy);
                            if nx < 0 || ny < 0 || nx >= bw || ny >= bh {
                                continue;
                            }
                            let j = (ny * bw + nx) as usize;
                            if solid[j] && !seen[j] {
                                seen[j] = true;
                                stack.push(j);
                            }
                        }
                    }
                }
                largest = largest.max(size);
            }
            (n, largest)
        };
        let blocks_with_any = foliage.iter().filter(|&&n| n > 0).count();
        println!("\ndistinguishability (§Z / C4), {trees} founders planted:");
        for t in [1u32, 2, 4, 8] {
            let (n, largest) = components(t);
            println!(
                "  canopy components at field resolution, >= {t:>2} leaf/block: {n:>3}   largest holds {:>3}% of canopy blocks",
                100 * largest / blocks_with_any.max(1)
            );
        }

        // **The sky-gap census, §Z's other candidate.**
        //
        // **Foliage, not any organism cell, and the first version got this
        // wrong.** Counting any plant cell in the column reported *zero*
        // interior gaps on a 4-founder stand whose render plainly shows sky
        // between three of its four crowns -- because the shed litter and
        // root mound at the foot of a stand is continuous across every
        // column, so the census was measuring the forest floor. `CLAUDE.md`:
        // look at the artifact before trusting the number, and ask what a
        // metric counts when nothing is wrong. Sky between crowns is a
        // question about crowns.
        //
        // Interior gaps only: the open world either side of the stand is
        // not a gap between trees, and counting it would put a floor of two
        // under every reading.
        let occupied: Vec<bool> = (0..width)
            .map(|x| {
                (0..ground).any(|y| {
                    let c = w.get(x, y);
                    c.organism_id() != 0 && organism::cell_type(c.aux()) == Some(organism::CellType::Leaf)
                })
            })
            .collect();
        let first = occupied.iter().position(|&o| o);
        let last = occupied.iter().rposition(|&o| o);
        let mut gaps: Vec<usize> = Vec::new();
        if let (Some(a), Some(b)) = (first, last) {
            let mut run = 0usize;
            for &o in &occupied[a..=b] {
                if o {
                    if run > 0 {
                        gaps.push(run);
                    }
                    run = 0;
                } else {
                    run += 1;
                }
            }
        }
        gaps.sort_unstable();
        let spacing = width / (trees as i32 + 1);
        println!(
            "  interior sky gaps: {} (a fully separate stand of {trees} shows {}), widths {gaps:?}",
            gaps.len(),
            trees.saturating_sub(1)
        );
        // **A one-cell gap is not a gap, and the threshold must not scale
        // with spacing.** §Z's own lesson is the first half: "crowns
        // interleave with one- and two-cell gaps: every row breaks, and the
        // eye still reads one mass". The raw count above duly finds a 1-cell
        // gap in the fused 8-founder stand.
        //
        // **CALIBRATED AGAINST THE OWNER'S EYE, 2026-08-23, AND IT FAILED.
        // These numbers are descriptive. They are not a substitute for a
        // card, and §Z stays judged by eye.**
        //
        // Three stands were rendered at frame 28,800 and put to the owner
        // with the founder counts withheld, asking only "how many separate
        // trees can you count?" (cards `20260823T092919055Z-ac816a` and
        // `...-87b3f5`, answered identically). Ground truth against every
        // reading this block computes:
        //
        //   founders  spacing | raw gaps (widths) | +1 | >=8-cell gaps +1 | fusion | OWNER
        //      8        56    |   1  ([1])        |  2 |        1         |   99%  |   2
        //      4       102    |   1  ([4])        |  2 |        1         |  100%  |   4
        //      3       128    |   2  ([1, 32])    |  3 |        2         |   38%  |   3
        //      2       170    |   1  ([13])       |  2 |        2         |   58%  |   -
        //
        // **The 4-founder stand is the case that settles it.** The owner
        // counts *all four*. Fusion reads **100%** — the strongest possible
        // "this is one mass" — and the gap census finds one 4-cell gap where
        // the eye finds three separations. The claim this block used to
        // make, that fusion "splits cleanly and in one place", is false: the
        // split it draws puts a stand the owner reads as four distinct trees
        // on the *fused* side.
        //
        // **Why no column census can fix this.** A gap is a fully empty
        // column. The eye separates crowns that overlap, using trunk
        // position and crown outline — cues that live in the *shape* of the
        // occupancy, not in whether any column is empty. At 102 cells apart
        // these crowns touch and are still four obvious trees. That is a
        // structural limit, not a threshold to retune.
        //
        // **The raw gap count is the least bad of the three and is still
        // wrong.** Gaps + 1 scores 2, 2, 3 against the owner's 2, 4, 3 —
        // exact on two of three, short by two on the fourth. Fusion misses
        // the same case worse. Reported for whoever picks this up; not
        // trusted.
        //
        // **A threshold made it strictly worse, twice, and both times I
        // invented it to explain away a reading I doubted.** First as a
        // quarter of the founder spacing, which scored two obviously
        // separate trees at zero because a quarter of 170 is 42. Then as an
        // absolute 8 cells, which scores **0 of 3** against the owner where
        // the unthresholded count scores 2 of 3 — because the 1-cell gap at
        // 8 founders that I discarded as noise is exactly the separation the
        // owner saw when they answered "2". Both thresholds were reasoning
        // about the picture instead of asking about it. The constant is kept
        // at 8 only so the printed line is comparable with the record above;
        // read the raw widths.
        //
        // What survives intact is the *negative* result, and it is worth
        // keeping: `thickest contiguous run` — the metric §Z records as
        // having been believed once and overturned — reads **36 to 51 across
        // this entire range**, and is *highest* on the stand the owner
        // counts as 2 of 8. It cannot tell a fused stand from a separate
        // one in either direction.
        //
        // Do **not** read the component count on its own either: it goes
        // above the founder count on a widely spaced stand, because a sparse
        // crown breaks into separate blocks.
        const CROWN_GAP: usize = 8;
        let real_gaps = gaps.iter().filter(|&&g| g >= CROWN_GAP).count();
        println!("  founder spacing is {spacing} cells; gaps at least {CROWN_GAP} cells wide: {real_gaps}");
        let largest_share = 100 * components(1).1 / blocks_with_any.max(1);
        println!(
            "  --> canopy fusion {largest_share}%, {} raw sky gaps ({real_gaps} at least {CROWN_GAP} cells) \
-- so this run reads {} distinguishable crowns of {trees} founders",
            gaps.len(),
            gaps.len() + 1
        );
        println!(
            "  (CALIBRATION FAILED against the owner's eye, 2026-08-23: on the three stands he counted,\n   \
these read 2 / 2 / 3 where he read 2 / 4 / 3, and fusion called the 4-founder stand 100% ONE MASS\n   \
when he counted all four. §Z is cards-only. Reports/open-bugs-handoff.md §Z has the table.)"
        );
    }

    // **Lineage turnover — the plan of record's Phase 0d, still unprinted
    // until now.**
    //
    // `Reports/plant-evolution-design.md` §5: "the count of
    // inherited-genome establishments per run is the plant equivalent of
    // births-per-generation, and if it reads ~0 at 30k frames, every
    // evolution claim at that horizon is about founders". A standing
    // population count cannot answer that -- slots are reused, so a flat
    // live count is equally consistent with a frozen stand and a fast
    // cycle. Births and deaths are cumulative (`World::organism_turnover`)
    // and separate them.
    //
    // Two different quantities, printed together because reading either
    // alone has already misled once: **births** counts every seed that ever
    // took a slot, most of which never grow; **establishments** counts the
    // ones that reached a plant, which is what selection can act on.
    {
        let (born, died) = w.organism_turnover();
        let (slots, live) = w.organism_slot_usage();
        let per_k = |n: u64| 1_000.0 * n as f64 / frames.max(1) as f64;
        println!(
            "\nlineage turnover over {frames} frames: {born} born, {died} died ({:.2} and {:.2} per 1,000 frames); \
{live} live in {slots} slots",
            per_k(born),
            per_k(died)
        );
        let descendants = per_organism.keys().filter(|id| w.organism_state(**id).is_some_and(|s| s.generation >= 1)).count();
        let deepest = per_organism.keys().filter_map(|id| w.organism_state(*id).map(|s| s.generation)).max().unwrap_or(0);
        println!(
            "  established plants carrying an inherited genome: {descendants} of {} (deepest generation {deepest}) \
-- at ~0 every claim from this run is about founders",
            per_organism.len()
        );
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

    // **The root readout, and why "thickest run below ground" was not it.**
    // That number is a *horizontal* run, so a root system that sprawls
    // sideways in the top three rows and one that also goes down report the
    // same 54 cells. The grove sheets show roots as pale lateral fences
    // hugging the surface and nothing had ever measured whether that
    // reading is right -- `CLAUDE.md`'s "an image says what and where; only
    // a number says how much". The band histogram is the one that settles
    // it: if every root cell lands in the top band, depth is the problem
    // and no amount of tuning lateral behaviour touches it.
    //
    // Root tissue is identified the same way `organism_upkeep` does it --
    // `reinforces_powder` (i.e. `rootwood`) or a live `RootTip` -- because a
    // retired root and a retired branch are both `MatureBody` and cell type
    // alone cannot tell them apart.
    {
        const ESTABLISHED: usize = 20;
        let surface = ground_y();
        let mut per_plant: std::collections::BTreeMap<u16, (usize, usize, i32, i32, i32, usize)> = std::collections::BTreeMap::new();
        let mut depth_bands = [0usize; 5];
        let mut root_total = 0usize;
        let mut depths: Vec<i32> = Vec::new();
        let (mut buried_seeds, mut surface_seeds) = (0usize, 0usize);
        for y in 0..height {
            for x in 0..width {
                let c = w.get(x, y);
                let id = c.organism_id();
                if id == 0 {
                    continue;
                }
                let is_root = w.materials.get(c.material).reinforces_powder
                    || organism::cell_type(c.aux()) == Some(organism::CellType::RootTip);
                // **Buried seeds, and whether they are a real leak.**
                // `Reproduce` runs on every `MatureBody` cell, and a retired
                // root cell is one -- so on the face of it a tree scatters
                // seed underground, where light is ~0 and germination can
                // never fire. `set_seed` needs an *empty* 8-neighbour
                // though, and underground every neighbour is soil, so the
                // path may be vacuous. Counted rather than reasoned about:
                // an exactly-zero reading means the gate already closes it.
                if organism::cell_type(c.aux()) == Some(organism::CellType::Seed) {
                    if y >= surface {
                        buried_seeds += 1;
                    } else {
                        surface_seeds += 1;
                    }
                }
                let e = per_plant.entry(id).or_insert((0, 0, i32::MAX, i32::MIN, i32::MIN, 0));
                e.0 += 1; // every cell, so root:shoot has a denominator
                if !is_root {
                    continue;
                }
                e.1 += 1;
                e.2 = e.2.min(x);
                e.3 = e.3.max(x);
                e.4 = e.4.max(y);
                // **Width, and the first version of this counted the wrong
                // thing.** "Has a root cell to its left or right" reads
                // *100%* for a purely horizontal one-cell filament, which is
                // the shape in question -- it measures continuity along the
                // run, not thickness across it. `CLAUDE.md`: ask what a
                // metric counts when nothing is wrong.
                //
                // The 4-neighbour root count separates them without knowing
                // which way the run points: an interior cell of a
                // one-cell-wide filament has exactly 2 whichever direction
                // it runs, an end has 1, and only a genuinely thickened root
                // has 3 or 4. This is the quantity `thicken` is supposed to
                // move.
                let neighbours = [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)]
                    .iter()
                    .filter(|&&(dx, dy)| {
                        let n = w.get(x + dx, y + dy);
                        n.organism_id() == id && w.materials.get(n.material).reinforces_powder
                    })
                    .count();
                if neighbours >= 3 {
                    e.5 += 1;
                }
                // Soil rows only. A cell past the soil bed would otherwise
                // clamp into the bottom band and read as "deep roots" when
                // it is a root that has left the soil entirely.
                if y >= surface && y < surface + soil_depth {
                    let band = (((y - surface) * 5) / soil_depth).clamp(0, 4) as usize;
                    depth_bands[band] += 1;
                    root_total += 1;
                }
                depths.push(y - surface);
            }
        }
        per_plant.retain(|_, v| v.0 >= ESTABLISHED);
        if !per_plant.is_empty() && root_total > 0 {
            let stat = |mut v: Vec<i64>| {
                v.sort_unstable();
                let sum: i64 = v.iter().sum();
                (v[0], v[v.len() / 2], v[v.len() - 1], sum as f32 / v.len() as f32)
            };
            let roots = stat(per_plant.values().map(|v| v.1 as i64).collect());
            let frac = stat(per_plant.values().map(|v| (v.1 * 100 / v.0.max(1)) as i64).collect());
            let depth = stat(per_plant.values().map(|v| (v.4 - surface + 1) as i64).collect());
            let spread = stat(per_plant.values().map(|v| (v.3 - v.2 + 1) as i64).collect());
            let wide: usize = per_plant.values().map(|v| v.5).sum();
            let root_cells: usize = per_plant.values().map(|v| v.1).sum();
            println!("\nroot system ({} established plants, soil is {} rows deep):", per_plant.len(), common::SOIL_DEPTH);
            println!("  root cells        min {:>4}  median {:>4}  max {:>4}  mean {:>7.1}", roots.0, roots.1, roots.2, roots.3);
            println!("  root share of plant  min {:>3}%  median {:>3}%  max {:>3}%  mean {:>5.1}%", frac.0, frac.1, frac.2, frac.3);
            println!("  deepest row below surface  min {:>3}  median {:>3}  max {:>3}  mean {:>5.1}", depth.0, depth.1, depth.2, depth.3);
            println!("  lateral spread (cells)     min {:>3}  median {:>3}  max {:>3}  mean {:>5.1}", spread.0, spread.1, spread.2, spread.3);
            depths.sort_unstable();
            println!(
                "  median root cell sits {} rows down; quartiles {} / {}",
                depths[depths.len() / 2],
                depths[depths.len() / 4],
                depths[depths.len() * 3 / 4]
            );
            println!(
                "  root cells with 3+ root neighbours: {}% (a 1-cell-wide run reads ~0 whichever way it points)",
                wide * 100 / root_cells.max(1)
            );
            let pct = |n: usize| n * 100 / root_total.max(1);
            println!(
                "  depth histogram, surface->bedrock (% of all root cells): [{}, {}, {}, {}, {}]",
                pct(depth_bands[0]),
                pct(depth_bands[1]),
                pct(depth_bands[2]),
                pct(depth_bands[3]),
                pct(depth_bands[4])
            );
            println!("  seeds standing: {surface_seeds} above the surface, {buried_seeds} buried (buried can never germinate)");

            // **The water balance, per organism, as a standing state.**
            // `CLAUDE.md`: when the complaint is about something visible
            // and persistent, measure the standing state rather than the
            // event rate. The stomatal term is the number that decides
            // whether roots matter at all -- it multiplies every credit --
            // so it is reported directly rather than inferred from mass.
            let mut stocks: Vec<f32> = Vec::new();
            let mut statuses: Vec<f32> = Vec::new();
            let mut uptakes: Vec<f32> = Vec::new();
            let mut demands: Vec<f32> = Vec::new();
            for id in per_plant.keys() {
                if let Some(st) = w.organism_state(*id) {
                    stocks.push(st.water);
                    statuses.push(st.water_status);
                    uptakes.push(st.water_uptake);
                    demands.push(st.water_demand);
                }
            }
            if !statuses.is_empty() {
                let q = |v: &mut Vec<f32>| {
                    v.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
                    (v[0], v[v.len() / 2], v[v.len() - 1], v.iter().sum::<f32>() / v.len() as f32)
                };
                let (smin, smed, smax, smean) = q(&mut stocks);
                let (wmin, wmed, wmax, wmean) = q(&mut statuses);
                println!("
water balance, per established plant:");
                println!("  stock          min {smin:>7.1} median {smed:>7.1} max {smax:>7.1} mean {smean:>7.1}");
                println!("  stomatal term  min {wmin:>7.2} median {wmed:>7.2} max {wmax:>7.2} mean {wmean:>7.2}   (1.0 = demand fully met)");
                let (umin, umed, umax, umean) = q(&mut uptakes);
                let (dmin, dmed, dmax, dmean) = q(&mut demands);
                println!("  uptake/tick    min {umin:>7.2} median {umed:>7.2} max {umax:>7.2} mean {umean:>7.2}");
                println!("  demand/tick    min {dmin:>7.2} median {dmed:>7.2} max {dmax:>7.2} mean {dmean:>7.2}");
            }

            // **What `support` actually reads on a healthy tree.** The
            // number changed meaning when the search was turned around to
            // run from the anchors outward: it used to answer "is there
            // ground within N hops of me" and now answers "how far out
            // along my own load path do I sit". `max_unsupported_span` is
            // read against it and was calibrated against the old meaning,
            // so this distribution is what a new value has to be set from.
            let mut supports: Vec<u16> = Vec::new();
            let mut unreached = 0usize;
            for y in 0..height {
                for x in 0..width {
                    if w.get(x, y).organism_id() == 0 {
                        continue;
                    }
                    if let Some(c) = w.organism_cell(x, y) {
                        if c.support == u16::MAX {
                            unreached += 1;
                        } else {
                            supports.push(c.support);
                        }
                    }
                }
            }
            if !supports.is_empty() {
                supports.sort_unstable();
                let at = |q: f32| supports[((supports.len() - 1) as f32 * q) as usize];
                println!(
                    "  support (cantilever reach from anchors): p50 {}  p90 {}  p99 {}  max {}; {} unreached",
                    at(0.5),
                    at(0.9),
                    at(0.99),
                    supports[supports.len() - 1],
                    unreached
                );
            }
        }
    }

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
