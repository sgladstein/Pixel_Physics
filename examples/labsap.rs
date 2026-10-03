//! **Does water flowing through plants change the lab, and what does it
//! cost?** — the instrument for `World::sap_flow` (`plant::sap_flow`).
//!
//! Runs a lab scenario (default `played_bed`) headless and prints a `STOP`
//! line every `sample=` frames: the stand (growing plants, cells, leaves),
//! its water (mean stomatal term, plants short of water, drought sheds),
//! **where the shortfall lands** (leaf dryness by path length: the far
//! quarter against the near quarter of each plant's leaves), the soil's
//! water, the ants, and the frame cost over the window.
//!
//! ```text
//! PIXEL_PHYSICS_SAP_FLOW=off cargo run --release --example labsap -- seed=3 growth=2
//! cargo run --release --example labsap -- seed=3 growth=2 dry_from=60000 dry_for=60000
//! ```
//!
//! - `growth=N` plants N times slower (`Clock::growth_slowdown`; the lab's
//!   default is about to be 2).
//! - `dry_from=`/`dry_for=` stop the mister for a spell (lane 17's
//!   `labdefence` knob, same meaning).
//! - `colony=0` runs the garden with no ants.
//! - `parch=F` is a hard drought at frame `F`: every soil cell is dried to
//!   just above the wilting point, standing water is removed, and the mister
//!   stays off from then on. Built because a dry spell alone does nothing in
//!   the lab: 90,000 rainless frames on a quarter of the usual soil left
//!   plant-available water at 0.87 of full, because the lidded box gives
//!   transpired water back as condensation (`weather::condense_under_a_lid`).
//!
//! One process is one arm: the switch is read from the environment when the
//! world is built, and the header line echoes it so a stale arm cannot pass
//! for the other.

use pixel_physics::lab::scenario::{Placement, Scenario};
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::frame;
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::organism::{self, CellType};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::update;
use pixel_physics::sim::world::World;
use std::time::Instant;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

fn main() {
    let frames: u64 = arg("frames").unwrap_or(300_000);
    let sample: u64 = arg("sample").unwrap_or(6_000);
    let name: String = arg("scenario").unwrap_or_else(|| "played_bed".to_string());
    let with_colony = arg::<u32>("colony").unwrap_or(1) != 0;
    let dry_from: u64 = arg("dry_from").unwrap_or(u64::MAX);
    let dry_for: u64 = arg("dry_for").unwrap_or(0);
    let parch_at: u64 = arg("parch").unwrap_or(u64::MAX);
    let mut sc = Scenario::load(&name).unwrap_or_else(|e| {
        eprintln!("scenario {name}: {e}");
        std::process::exit(2);
    });
    if let Some(sd) = arg::<u64>("seed") {
        sc.bed.seed = sd;
    }
    if !with_colony {
        sc.timeline.retain(|e| !matches!(e.what, Placement::Colony { .. } | Placement::Colonies { .. }));
    }
    let spec = sc.bed.clone();
    let (mut world, _, _) = sc.build();
    if let Some(n) = arg::<u32>("growth") {
        world.clock.set_rates(0, |c| c.growth_slowdown = n);
    }
    println!(
        "labsap: scenario={name} seed={} frames={frames} sample={sample} colony={} sap_flow={} growth_slowdown={} dry_from={dry_from} dry_for={dry_for} parch={parch_at}",
        spec.seed,
        if with_colony { "on" } else { "OFF" },
        if world.sap_flow { "ON" } else { "off" },
        world.clock.growth_slowdown,
    );
    let rain = spec.rain;
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    let (mut ants_peak, mut alive_at_end) = (0usize, 0usize);
    let mut window = Instant::now();
    let mut total_ms = 0f64;
    for f in 0..=frames {
        pixel_physics::lab::scenario::tick_timeline(&sc, &mut world, &spec);
        if f % sample == 0 {
            let ms = window.elapsed().as_secs_f64() * 1000.0;
            total_ms += ms;
            let ants = stop(&world, f, (spec.width, spec.height), if f == 0 { 0.0 } else { ms / sample as f64 });
            ants_peak = ants_peak.max(ants);
            alive_at_end = ants;
            window = Instant::now();
        }
        if f == parch_at {
            parch(&mut world, (spec.width, spec.height));
        }
        if f < frames {
            frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
            if !(f >= dry_from && f < dry_from.saturating_add(dry_for)) && f < parch_at {
                pixel_physics::lab::rain::tick(&mut world, &spec, rain);
            }
        }
    }
    println!(
        "SAP_SUMMARY seed={} sap_flow={} colony={} frames={frames} ants_peak={ants_peak} ants_end={alive_at_end} shed_drought={} shed_shade={} ms_per_frame={:.3}",
        spec.seed,
        world.sap_flow as u8,
        with_colony as u8,
        world.shed_drought,
        world.shed_shade,
        total_ms / frames.max(1) as f64,
    );
}

/// A hard drought: soil to just above the wilting point, standing water gone.
pub fn parch(world: &mut World, (width, height): (i32, i32)) {
    use pixel_physics::sim::cell::Cell;
    use pixel_physics::sim::material;
    for y in 0..height {
        for x in 0..width {
            let c = world.get(x, y);
            if c.organism_id() != 0 {
                continue;
            }
            match world.materials.kind(c.material) {
                MaterialKind::Liquid if c.material == material::WATER => world.set(x, y, Cell::EMPTY),
                MaterialKind::Powder if world.materials.get(c.material).water_capacity > 0 => {
                    let held = update::soil_moisture(c);
                    world.set(x, y, c.with_aux(held.min(material::SOIL_WILTING_POINT + 60)));
                }
                _ => {}
            }
        }
    }
}

fn quantile(v: &mut [f32], q: f32) -> f32 {
    if v.is_empty() {
        return f32::NAN;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[((v.len() as f32 - 1.0) * q).round() as usize]
}

/// One stop line; returns the ant count.
fn stop(world: &World, f: u64, (width, height): (i32, i32), ms_per_frame: f64) -> usize {
    let (mut ants, mut plants, mut cells, mut leaves, mut thirsty) = (0usize, 0usize, 0usize, 0usize, 0usize);
    let mut status_sum = 0f32;
    // Leaf dryness, near quarter and far quarter of each thirsty plant's
    // leaves by path length -- does the shortfall land on the far tips?
    let (mut near_dry, mut far_dry, mut nf_n) = (0f32, 0f32, 0usize);
    let mut leaf_dry: Vec<f32> = Vec::new();
    let mut flux_max = 0f32;
    for id in world.live_organism_ids() {
        let Some(s) = world.organism(id) else { continue };
        if world.species.get(s.species).creature.is_some() {
            ants += 1;
            continue;
        }
        if s.brood.is_some() || s.cells.len() <= 1 {
            continue;
        }
        plants += 1;
        cells += s.cells.len();
        status_sum += s.water_status;
        if s.water_desiccation > 0.0 {
            thirsty += 1;
        }
        let mut by_path: Vec<(u16, f32)> = Vec::new();
        for (&(x, y), oc) in &s.cells {
            flux_max = flux_max.max(oc.sap_flux);
            if organism::cell_type(world.get(x, y).aux()) == Some(CellType::Leaf) {
                leaves += 1;
                let d = world.desiccation_at(x, y);
                leaf_dry.push(d);
                by_path.push((oc.path_len, d));
            }
        }
        if s.water_desiccation > 0.0 && by_path.len() >= 8 {
            by_path.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.partial_cmp(&b.1).unwrap()));
            let q = by_path.len() / 4;
            near_dry += by_path[..q].iter().map(|p| p.1).sum::<f32>() / q as f32;
            far_dry += by_path[by_path.len() - q..].iter().map(|p| p.1).sum::<f32>() / q as f32;
            nf_n += 1;
        }
    }
    // Soil water: plant-available fraction summed over powder cells that
    // hold water, and the standing free water.
    let (mut soil_avail, mut soil_cells, mut free_water) = (0f64, 0usize, 0usize);
    for y in 0..height {
        for x in 0..width {
            let c = world.get(x, y);
            match world.materials.kind(c.material) {
                MaterialKind::Powder if world.materials.get(c.material).water_capacity > 0 => {
                    soil_cells += 1;
                    soil_avail += update::plant_available_fraction(c) as f64;
                }
                MaterialKind::Liquid => free_water += 1,
                _ => {}
            }
        }
    }
    let mean_dry = if leaf_dry.is_empty() { 0.0 } else { leaf_dry.iter().sum::<f32>() / leaf_dry.len() as f32 };
    println!(
        "STOP f={f} ants={ants} plants={plants} cells={cells} leaves={leaves} status={:.3} thirsty={thirsty} leaf_dry_mean={:.3} leaf_dry_p90={:.3} near_dry={:.3} far_dry={:.3} nf_plants={nf_n} shed_drought={} soil_avail={:.3} free_water={free_water} flux_max={:.2} ms_per_frame={ms_per_frame:.3}",
        status_sum / plants.max(1) as f32,
        mean_dry,
        quantile(&mut leaf_dry, 0.9),
        if nf_n > 0 { near_dry / nf_n as f32 } else { f32::NAN },
        if nf_n > 0 { far_dry / nf_n as f32 } else { f32::NAN },
        world.shed_drought,
        soil_avail / soil_cells.max(1) as f64,
        flux_max,
    );
    ants
}
