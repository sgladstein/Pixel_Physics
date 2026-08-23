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
            "mix" => mix = v != "0",
            _ => panic!("unknown argument {arg:?}"),
        }
    }
    let (presets, err) = WorldgenPresets::load();
    if let Some(e) = err {
        panic!("{e}");
    }
    let name = if preset.is_empty() { presets.default_name() } else { preset.clone() };
    let Some(params) = presets.get(&name) else { panic!("unknown preset {name:?}") };

    // **The harness names its own parameters** — a log that does not say
    // which seed or which world size it ran is a log written by a binary
    // that may never have had the knob (`CLAUDE.md`, the megastudy that was
    // three populations wearing 24 files).
    println!("flora_census preset={name} seeds={seeds} frames={frames} world={}x{}", w + 1, h + 1);

    if terrain {
        report_terrain(params, seeds, w, h);
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
        println!("  seed {seed:>3}  gen {gen_ms:>6.0}ms  run {run_s:>6.1}s  {}", line.join(" | "));
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
    let mut aridity: Vec<f32> = Vec::new();
    let mut elev: Vec<f32> = Vec::new();
    let mut depth: Vec<f32> = Vec::new();
    let mut table: Vec<f32> = Vec::new();
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
