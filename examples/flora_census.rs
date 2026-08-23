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
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "seeds" => seeds = v.parse::<usize>().expect("seeds=N").max(1),
            "frames" => frames = v.parse().expect("frames=N"),
            "preset" => preset = v.to_string(),
            "w" => w = v.parse::<i32>().expect("w=N") - 1,
            "h" => h = v.parse::<i32>().expect("h=N") - 1,
            "terrain" => terrain = v != "0",
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
