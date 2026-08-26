//! **Which species a generated world actually contains, per seed.**
//!
//! `life_scatter` sowed one hardcoded species name for the whole life of
//! this project (`Reports/plant-project-review-2026-08-23.md` §2.0), and
//! nothing measured it: the pass reports a cell *total*, and a total cannot
//! say that conifer, shrub and creeper have never been planted in a
//! generated world at all. Same shape as the `i32::MIN` spacing bug that
//! test `the_world_arrives_with_both_moss_and_trees_in_it` was written for
//! — a healthy-looking number over a species count of zero.
//!
//! Two numbers per species, and they answer different questions:
//!
//! - **sown** — organisms the generator created. Says the *rule* fired.
//! - **established** — organisms still alive with a grown (non-`seed`) cell
//!   after `frames`. Says the rule put the species somewhere it can live.
//!
//! Sowing without establishment is the failure mode this exists to catch:
//! a species dropped into country whose soil water never reaches its own
//! `soil_water_threshold` is sown forever and germinates never, and the
//! panorama looks exactly like a world where that species is rare.
//!
//! Worlds are procedural, so a single seed is a sample from a wide
//! distribution and never an answer (`CLAUDE.md`). Everything here is
//! reported per seed *and* as an order statistic across the sweep.
//!
//! ```text
//! cargo run --release --example flora_census                       # 8 seeds, generation only
//! cargo run --release --example flora_census -- frames=4000        # ...and let them germinate
//! cargo run --release --example flora_census -- preset=arid seeds=8
//! cargo run --release --example flora_census -- w=8192 h=2560 seeds=2 frames=2000
//! ```
//!
//! **Rebuild before trusting a run** (`cargo build --release --examples`):
//! species files are `include_str!`d, so an edited `.ron` and a prebuilt
//! example produce a bit-identical "run" that swept nothing.

use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::world::World;
use pixel_physics::worldgen::{self, Spec, WorldgenPresets};
use std::collections::BTreeMap;

/// One world's flora, by species name.
#[derive(Default)]
struct Census {
    /// Live organisms, by species.
    organisms: BTreeMap<String, usize>,
    /// Live organisms that have any cell of a material other than `seed`,
    /// by species — i.e. that germinated and are still standing.
    established: BTreeMap<String, usize>,
    /// Cells owned by each species' organisms.
    cells: BTreeMap<String, usize>,
}

/// Every live organism, resolved to its species name.
///
/// Scanned from *cells* rather than from the organism table, because the
/// organism table is `pub(crate)` and because a census that reads the same
/// grid the picture is drawn from cannot disagree with the picture.
fn census(world: &World) -> Census {
    let seed_material = world.materials.id_of("seed");
    let bounds = world.bounds().expect("bounded world");
    let mut grown: BTreeMap<u16, bool> = BTreeMap::new();
    let mut cells: BTreeMap<u16, usize> = BTreeMap::new();
    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            let cell = world.get(x, y);
            let id = cell.organism_id();
            if id == 0 {
                continue;
            }
            *cells.entry(id).or_default() += 1;
            let is_grown = Some(cell.material) != seed_material;
            let entry = grown.entry(id).or_default();
            *entry |= is_grown;
        }
    }
    let mut out = Census::default();
    for (id, cell_count) in &cells {
        // A stale id whose organism is gone resolves to `None` rather than
        // to some other organism — the generational check exists for this.
        let Some(state) = world.organism(*id) else { continue };
        let name = world.species.get(state.species).name.clone();
        *out.organisms.entry(name.clone()).or_default() += 1;
        *out.cells.entry(name.clone()).or_default() += cell_count;
        if grown.get(id).copied().unwrap_or(false) {
            *out.established.entry(name).or_default() += 1;
        }
    }
    out
}

fn main() {
    let (mut seeds, mut frames, mut preset) = (8usize, 0usize, String::new());
    let (mut w, mut h) = (2047i32, 639i32);
    let mut terrain = false;
    let mut where_mode = false;
    let mut window: i32 = 512;
    let mut at: Option<i32> = None;
    let mut focus: Option<String> = None;
    let (mut tree_density, mut grass_density, mut moss_density): (Option<f32>, Option<f32>, Option<f32>) = (None, None, None);
    let mut mix = false;
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "seeds" => seeds = v.parse::<usize>().expect("seeds=N").max(1),
            "frames" => frames = v.parse().expect("frames=N"),
            "preset" => preset = v.to_string(),
            "w" => w = v.parse::<i32>().expect("w=N") - 1,
            "h" => h = v.parse::<i32>().expect("h=N") - 1,
            "terrain" => terrain = v != "0",
            // **Density overrides, so a control can be run without editing a
            // preset.** The question these exist for: grass establishes at
            // 96% and is down to 3 of 40 by frame 45,000 — is that the
            // woody canopy closing over it (succession, and correct), or
            // grass failing on its own? Only the same world with the woody
            // layer switched off can separate those, and before this the
            // comparison could not be expressed at all.
            "treedensity" => tree_density = Some(v.parse().expect("treedensity")),
            "grassdensity" => grass_density = Some(v.parse().expect("grassdensity")),
            "mossdensity" => moss_density = Some(v.parse().expect("mossdensity")),
            "mix" => mix = v != "0",
            // See `report_where`: `where=1` says which columns a species is
            // actually in, `window=` how wide a camera is, `at=` audits a
            // window that has already been rendered.
            "where" => where_mode = v != "0",
            "window" => window = v.parse().expect("window"),
            "at" => at = Some(v.parse().expect("at")),
            "focus" => focus = Some(v.to_string()),
            _ => panic!("unknown argument {arg:?}"),
        }
    }
    let (presets, err) = WorldgenPresets::load();
    if let Some(e) = err {
        panic!("{e}");
    }
    let name = if preset.is_empty() { presets.default_name() } else { preset.clone() };
    let Some(params) = presets.get(&name) else { panic!("unknown preset {name:?}") };
    let mut overridden = params.clone();
    if let Some(v) = tree_density {
        overridden.tree_density = v;
    }
    if let Some(v) = grass_density {
        overridden.grass_density = v;
    }
    if let Some(v) = moss_density {
        overridden.moss_density = v;
    }
    let params = &overridden;

    // **The harness names its own parameters** — a log that does not say
    // which seed or which world size it ran is a log written by a binary
    // that may never have had the knob (`CLAUDE.md`, the megastudy that was
    // three populations wearing 24 files). The three densities are echoed
    // whether or not they were overridden, for the same reason: a control
    // run and the run it controls have to be distinguishable from their logs
    // alone, months later.
    println!(
        "flora_census preset={name} seeds={seeds} frames={frames} world={}x{} tree_density={} grass_density={} moss_density={}",
        w + 1,
        h + 1,
        params.tree_density,
        params.grass_density,
        params.moss_density
    );

    if terrain {
        report_terrain(params, seeds, w, h);
        return;
    }
    if where_mode {
        report_where(params, seeds, w, h, frames, &WhereOpts { window, at, focus });
        return;
    }
    if mix {
        report_mix(params, seeds, w, h);
        return;
    }

    let mut per_species: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut per_species_est: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for seed in 1..=seeds as u64 {
        let mut world = World::new(Rect::new(0, 0, w, h));
        let t = std::time::Instant::now();
        worldgen::generate(&mut world, Spec::Generated { params, seed });
        let gen_ms = t.elapsed().as_secs_f64() * 1000.0;
        let sown = census(&world);
        let t = std::time::Instant::now();
        for _ in 0..frames {
            pixel_physics::sim::parallel::step(&mut world);
            world.step_active_sites();
            world.step_fields();
        }
        let run_s = t.elapsed().as_secs_f64();
        let after = census(&world);
        let names: Vec<String> = sown.organisms.keys().chain(after.organisms.keys()).cloned().collect();
        let mut seen: Vec<String> = Vec::new();
        for n in names {
            if !seen.contains(&n) {
                seen.push(n);
            }
        }
        seen.sort();
        let line: Vec<String> = seen
            .iter()
            .map(|n| {
                let s = sown.organisms.get(n).copied().unwrap_or(0);
                let e = after.established.get(n).copied().unwrap_or(0);
                let c = after.cells.get(n).copied().unwrap_or(0);
                per_species.entry(n.clone()).or_default().push(s);
                per_species_est.entry(n.clone()).or_default().push(e);
                format!("{n}: sown {s} est {e} cells {c}")
            })
            .collect();
        // **The slot ceiling, printed rather than assumed.** Grass is the
        // one sown species that breeds fast enough to matter to the 4,095
        // organism-slot ceiling, and what happens past it is silent id
        // corruption rather than a visible failure — a world at the ceiling
        // and a world where nothing is breeding read identically in every
        // other column of this line. `high-water` is the peak of
        // concurrently-live organisms and `refused` the births the ceiling
        // turned away; both are free from the allocator.
        let (high_water, ceiling) = world.organism_slot_high_water();
        let refused = world.organisms_refused();
        println!(
            "  seed {seed:>3}  gen {gen_ms:>6.0}ms  run {run_s:>6.1}s  slots {high_water}/{ceiling} refused {refused}  {}",
            line.join(" | ")
        );
    }

    // The order statistic, not the mean: outcomes here are chaotic in the
    // seed, so which world is worst reshuffles on any legitimate change and
    // a per-seed baseline gets rubber-stamped (`CLAUDE.md`).
    println!("\n  species        sown min/med/max   established min/med/max   seeds with any");
    let mut names: Vec<String> = per_species.keys().cloned().collect();
    names.sort();
    for n in names {
        let stat = |v: &mut Vec<usize>| {
            v.sort_unstable();
            (v[0], v[v.len() / 2], v[v.len() - 1])
        };
        let (s_lo, s_mid, s_hi) = stat(per_species.get_mut(&n).unwrap());
        let est = per_species_est.get_mut(&n).unwrap();
        let with_any = est.iter().filter(|&&e| e > 0).count();
        let (e_lo, e_mid, e_hi) = stat(est);
        println!("  {n:<12}   {s_lo:>4}/{s_mid:>4}/{s_hi:>4}          {e_lo:>4}/{e_mid:>4}/{e_hi:>4}              {with_any}/{seeds}");
    }
}

/// **What country the sowing rule is actually choosing between.**
///
/// Written before any niche threshold was, per `CLAUDE.md`'s "look before
/// you measure": every band in `life_scatter` is a cut through one of these
/// distributions, and a cut placed from an aspiration rather than from the
/// spread is a band that is either empty or the whole world. Restricted to
/// the columns the pass can actually plant in — soil footing with sky above
/// — because the distribution over *all* columns is a different question and
/// answering it would set the thresholds wrong.
fn report_terrain(params: &pixel_physics::worldgen::WorldgenParams, seeds: usize, w: i32, h: i32) {
    use pixel_physics::worldgen::column::Terrain;
    use pixel_physics::worldgen::passes;
    let mut aridity: Vec<f32> = Vec::new();
    let mut elev: Vec<f32> = Vec::new();
    let mut depth: Vec<f32> = Vec::new();
    let mut table: Vec<f32> = Vec::new();
    // **The woody sum, which is the fact the grass band is cut through.**
    // Grass is the ground layer of open country, and "open" is not a species
    // being absent — it is the whole woody preference summing low. Printed
    // here, unclamped, because `life_scatter`'s own `budget` is
    // `min(1.0, sum)` and that saturates across most of the world.
    let mut woody: Vec<f32> = Vec::new();
    let (mut plantable, mut columns) = (0usize, 0usize);
    for seed in 1..=seeds as u64 {
        let mut world = World::new(Rect::new(0, 0, w, h));
        worldgen::generate(&mut world, Spec::Generated { params, seed });
        let soil = world.materials.id_of("soil").expect("soil");
        let soil_tan = world.materials.get(soil).friction_angle.to_radians().tan();
        let sand = world.materials.id_of("sand").expect("sand");
        let sand_tan = world.materials.get(sand).friction_angle.to_radians().tan();
        let t = Terrain::new(seed, params, w + 1, h + 1, soil_tan, sand_tan);
        for x in 0..=w {
            columns += 1;
            let plan = t.plan(x);
            let ground = plan.surface_y;
            if ground - 1 < 0 || world.get(x, ground - 1).material != pixel_physics::sim::material::EMPTY {
                continue;
            }
            if world.get(x, ground).material != soil {
                continue;
            }
            plantable += 1;
            let ch = t.character(x);
            aridity.push(ch.aridity);
            elev.push(ch.elev);
            depth.push(plan.soil_depth as f32);
            table.push((plan.table_y - ground) as f32);
            woody.push(passes::woody_budget(&passes::Site::new(ch.aridity, ch.elev, plan.soil_depth)));
        }
    }
    let q = |v: &mut Vec<f32>, name: &str| {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let at = |f: f32| v[((v.len() - 1) as f32 * f) as usize];
        println!("  {name:<12} min {:>7.2}  p10 {:>7.2}  p50 {:>7.2}  p90 {:>7.2}  max {:>7.2}", at(0.0), at(0.1), at(0.5), at(0.9), at(1.0));
    };
    println!("  plantable columns (soil footing, sky above): {plantable} of {columns}");
    q(&mut aridity, "aridity");
    q(&mut elev, "elev");
    q(&mut depth, "soil_depth");
    q(&mut table, "table-below");
    q(&mut woody, "woody-sum");
}

/// **Where a species actually is, in world columns — the camera-pointing
/// instrument.**
///
/// Written after a review card came back *"I don't see a difference between
/// the images."* The card was a 512-column crop of the 8,192-column world,
/// and its `meta` honestly reported 71 established grass plants — **for the
/// whole world**. One plant every 115 columns means the rendered window held
/// about four of them, each a handful of cells, and "no difference" was the
/// correct reading of that picture. The counts were real and were the wrong
/// counts: a whole-world total cannot say whether the thing is *in the
/// frame*.
///
/// So this reports the densest `window`-wide span per species — where to
/// point a camera — and, with `at=X`, what a window you have already
/// rendered actually contained. Check the second before believing a card and
/// use the first before shooting one.
/// How the camera-pointing report is aimed: how wide the shot is, an
/// already-rendered window to audit, and a single species to rank windows
/// for. One struct rather than three more parameters -- these three are one
/// question ("which frame") and travel together.
struct WhereOpts {
    window: i32,
    at: Option<i32>,
    focus: Option<String>,
}

fn report_where(params: &pixel_physics::worldgen::WorldgenParams, seeds: usize, w: i32, h: i32, frames: usize, opts: &WhereOpts) {
    let WhereOpts { window, at, focus } = opts;
    let (window, at) = (*window, *at);
    for seed in 1..=seeds as u64 {
        let mut world = World::new(Rect::new(0, 0, w, h));
        worldgen::generate(&mut world, Spec::Generated { params, seed });
        for _ in 0..frames {
            pixel_physics::sim::parallel::step(&mut world);
            world.step_active_sites();
            world.step_fields();
        }
        let bounds = world.bounds().expect("bounded world");
        // Cells per (species, column). Cells rather than plants, because a
        // camera sees cells: four grass plants of five cells each and one
        // shrub of two hundred are not the same picture.
        let mut per_col: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for y in bounds.min_y..=bounds.max_y {
            for x in bounds.min_x..=bounds.max_x {
                let id = world.get(x, y).organism_id();
                if id == 0 {
                    continue;
                }
                let Some(state) = world.organism(id) else { continue };
                let name = world.species.get(state.species).name.clone();
                per_col.entry(name).or_insert_with(|| vec![0; (w + 1) as usize])[x as usize] += 1;
            }
        }
        // **`focus=NAME` — the top non-overlapping windows for one species,
        // with everything else that shares them.** The densest-window-per-
        // species listing below answers "where is this species", which is
        // not the same question as "where can I photograph this species":
        // grass's densest 128-column window on the shipped world holds 348
        // grass cells and 3,276 tree cells, and a picture of it is a picture
        // of trees. This ranks candidates and prints what a camera pointed
        // at each would actually contain.
        if let Some(target) = focus.as_deref() {
            let Some(cols) = per_col.get(target) else {
                println!("  seed {seed}: no {target} in this world at all");
                continue;
            };
            let total: usize = cols.iter().sum();
            let mut sums: Vec<(usize, i32)> = Vec::new();
            let mut run: usize = cols.iter().take(window.min(w + 1) as usize).sum();
            sums.push((run, 0));
            for x in window..=w {
                run += cols[x as usize];
                run -= cols[(x - window) as usize];
                sums.push((run, x - window + 1));
            }
            sums.sort_by_key(|s| std::cmp::Reverse(s.0));
            let mut taken: Vec<i32> = Vec::new();
            println!("  seed {seed}: top {window}-column windows for {target} ({total} cells world-wide)");
            for (count, x0) in sums {
                if count == 0 || taken.len() >= 6 {
                    break;
                }
                // Non-overlapping, or the list is six views of one patch.
                if taken.iter().any(|&t| (t - x0).abs() < window) {
                    continue;
                }
                taken.push(x0);
                let others: Vec<String> = per_col
                    .iter()
                    .filter(|(n, _)| n.as_str() != target)
                    .map(|(n, c)| {
                        let lo = x0.clamp(0, w) as usize;
                        let hi = (x0 + window).clamp(0, w + 1) as usize;
                        (n.clone(), c[lo..hi].iter().sum::<usize>())
                    })
                    .filter(|(_, v)| *v > 0)
                    .map(|(n, v)| format!("{n} {v}"))
                    .collect();
                // **And the rows it occupies**, because "where" is two
                // coordinates and a crop set from the wrong one shows bare
                // ground. A 70-row crop guessed from the surface line missed
                // a sward entirely once already.
                let (mut y0, mut y1) = (i32::MAX, i32::MIN);
                for y in bounds.min_y..=bounds.max_y {
                    for x in x0..(x0 + window).min(w + 1) {
                        let id = world.get(x, y).organism_id();
                        if id == 0 {
                            continue;
                        }
                        let Some(state) = world.organism(id) else { continue };
                        if world.species.get(state.species).name == target {
                            y0 = y0.min(y);
                            y1 = y1.max(y);
                        }
                    }
                }
                println!(
                    "    x={x0:<6} {target} {count:>5} cells  rows {y0}..{y1}  |  also: {}",
                    if others.is_empty() { "nothing else".to_string() } else { others.join(", ") }
                );
            }
            continue;
        }
        println!("  seed {seed}: densest {window}-column window per species (and what it holds)");
        for (name, cols) in &per_col {
            let total: usize = cols.iter().sum();
            // One pass with a running sum -- the world is 8,192 wide and this
            // runs per species per seed.
            let mut run: usize = cols.iter().take(window.min(w + 1) as usize).sum();
            let mut best = (0i32, run);
            for x in window..=w {
                run += cols[x as usize];
                run -= cols[(x - window) as usize];
                if run > best.1 {
                    best = (x - window + 1, run);
                }
            }
            let at_count = at.map(|a| {
                let lo = a.clamp(0, w) as usize;
                let hi = (a + window).clamp(0, w + 1) as usize;
                cols[lo..hi].iter().sum::<usize>()
            });
            match at_count {
                Some(c) => println!(
                    "    {name:<9} {total:>6} cells world-wide | densest window x={:<6} {:>5} cells ({:>4.1}%) | at x={:<6} {:>5} cells ({:>4.1}%)",
                    best.0,
                    best.1,
                    100.0 * best.1 as f64 / total.max(1) as f64,
                    at.unwrap_or(0),
                    c,
                    100.0 * c as f64 / total.max(1) as f64
                ),
                None => println!(
                    "    {name:<9} {total:>6} cells world-wide | densest window x={:<6} {:>5} cells ({:>4.1}%)",
                    best.0,
                    best.1,
                    100.0 * best.1 as f64 / total.max(1) as f64
                ),
            }
        }
    }
}

/// **How segregated the woody species actually are along the world.**
///
/// Written after the owner's verdict on the first generated-world panorama:
/// *"Mostly more of the same"* — the four species were sown, established and
/// counted, and crossing the world still did not read as country changing.
/// A count says a species is present; only this says whether it is present
/// *somewhere in particular*.
///
/// Two readings, because they fail differently:
///
/// - **same-species neighbour fraction** — for each plant, how many of its
///   four nearest woody neighbours along x share its species. `1.0` is
///   perfect belts, and the baseline for "no spatial structure at all" is
///   not zero but each species' own share of the population, which is
///   printed beside it. A value at that baseline means the niche weights
///   are decorative and the cluster noise is doing all the placing.
/// - **run length** — consecutive plants of one species. Belts read as long
///   runs; a mixed thicket reads as runs of one or two however many plants
///   there are.
fn report_mix(params: &pixel_physics::worldgen::WorldgenParams, seeds: usize, w: i32, h: i32) {
    let mut pooled: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut pooled_runs: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for seed in 1..=seeds as u64 {
        let mut world = World::new(Rect::new(0, 0, w, h));
        worldgen::generate(&mut world, Spec::Generated { params, seed });
        // Every planted organism as (x, species), in world order.
        let bounds = world.bounds().expect("bounded world");
        let mut seen: BTreeMap<u16, (i32, String)> = BTreeMap::new();
        for y in bounds.min_y..=bounds.max_y {
            for x in bounds.min_x..=bounds.max_x {
                let id = world.get(x, y).organism_id();
                if id == 0 || seen.contains_key(&id) {
                    continue;
                }
                if let Some(st) = world.organism(id) {
                    seen.insert(id, (x, world.species.get(st.species).name.clone()));
                }
            }
        }
        // Moss is not part of this question -- it is a different kind of
        // thing and it is placed by the unoffset field, so including it
        // would dilute the very structure being measured.
        let mut plants: Vec<(i32, String)> = seen.into_values().filter(|(_, n)| n != "moss").collect();
        plants.sort();
        for i in 0..plants.len() {
            let (_, ref name) = plants[i];
            let lo = i.saturating_sub(2);
            let hi = (i + 3).min(plants.len());
            let (mut same, mut total) = (0, 0);
            for (j, (_, other)) in plants.iter().enumerate().take(hi).skip(lo) {
                if j == i {
                    continue;
                }
                total += 1;
                if other == name {
                    same += 1;
                }
            }
            let e = pooled.entry(name.clone()).or_default();
            e.0 += same;
            e.1 += total;
        }
        let mut i = 0;
        while i < plants.len() {
            let mut j = i;
            while j + 1 < plants.len() && plants[j + 1].1 == plants[i].1 {
                j += 1;
            }
            pooled_runs.entry(plants[i].1.clone()).or_default().push(j - i + 1);
            i = j + 1;
        }
    }
    let population: usize = pooled_runs.values().flatten().sum();
    println!("  pooled woody plants: {population} over {seeds} worlds\n");
    println!("  species     same-species neighbours   share of population   longest run   mean run");
    for (name, runs) in &pooled_runs {
        let (same, total) = pooled[name];
        let n: usize = runs.iter().sum();
        let share = n as f32 / population.max(1) as f32;
        let frac = same as f32 / total.max(1) as f32;
        let longest = runs.iter().copied().max().unwrap_or(0);
        let mean = n as f32 / runs.len().max(1) as f32;
        println!("  {name:<10}          {frac:>6.2}                {share:>6.2}            {longest:>4}       {mean:>5.2}");
    }
    println!("\n  Read `same-species neighbours` against `share of population`: equal means no");
    println!("  spatial structure at all, and the niche weights are decorative.");
}
