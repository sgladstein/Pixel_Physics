//! **Does the garden answer back?** One stop line every `sample=` frames on a
//! lab scenario (default `played_bed`): ants alive, growing plants, dormant
//! seeds, edible plant food standing (at face value and at what a mouth
//! actually gets after defence), and the spread of plant defence over the
//! growing plants and over the seed bank.
//!
//! Built 2026-10-03 with `OrganismState::defence` (plants that can evolve to
//! be worse to eat, paying for it in growth). The claim that mechanism makes
//! is *selection*: grazed gardens should climb in defence faster than the
//! same garden with no animals. Defence also drifts upward under no
//! selection at all (`organism::mutate_defence` reflects at zero), so the
//! comparison that means anything is `colony=1` against `colony=0` on the
//! same seed, never the colony arm against zero.
//!
//! ```text
//! cargo run --release --example labdefence -- seed=1 frames=300000
//! cargo run --release --example labdefence -- seed=1 frames=300000 colony=0
//! PLANT_DEFENCE=0 cargo run --release --example labdefence -- seed=1   # the off arm
//! ```
//!
//! `cost=` overrides nothing (the cost is a constant); `scale=` is the
//! positive control: it multiplies every newborn seed's defence jitter by
//! scale via the world's `mutation_sigma`, so `scale=4` should move defence
//! visibly faster than `scale=1` or the instrument is blind.
use pixel_physics::lab::scenario::{Placement, Scenario};
use pixel_physics::sim::creature;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::frame;
use pixel_physics::sim::organism;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}="))
            .map(|v| v.parse().ok().expect("parses"))
    })
}

fn quantile(v: &[f32], q: f32) -> f32 {
    if v.is_empty() {
        return f32::NAN;
    }
    v[((v.len() as f32 - 1.0) * q).round() as usize]
}

fn main() {
    let frames: u64 = arg("frames").unwrap_or(300_000);
    let sample: u64 = arg("sample").unwrap_or(6_000);
    let name: String = arg("scenario").unwrap_or_else(|| "played_bed".to_string());
    let with_colony = arg::<u32>("colony").unwrap_or(1) != 0;
    let scale: f32 = arg("scale").unwrap_or(1.0);
    let mut sc = Scenario::load(&name).unwrap_or_else(|e| {
        eprintln!("scenario {name}: {e}");
        std::process::exit(2);
    });
    if let Some(sd) = arg::<u64>("seed") {
        sc.bed.seed = sd;
    }
    if !with_colony {
        sc.timeline
            .retain(|e| !matches!(e.what, Placement::Colony { .. } | Placement::Colonies { .. }));
    }
    let spec = sc.bed.clone();
    let (mut world, _, _) = sc.build();
    if scale != 1.0 {
        world.mutation_sigma *= scale;
    }
    println!(
        "labdefence: scenario={name} seed={} frames={frames} sample={sample} colony={} defence={} drought_reach={} mutation_sigma={:.3} cost={}",
        spec.seed,
        if with_colony { "on" } else { "OFF" },
        if world.plant_defence { "on" } else { "OFF" },
        if world.drought_reach { "on" } else { "OFF" },
        world.mutation_sigma,
        organism::DEFENCE_COST,
    );
    let rain = spec.rain;
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    let mut ants_peak = 0usize;
    for f in 0..=frames {
        pixel_physics::lab::scenario::tick_timeline(&sc, &mut world, &spec);
        if f % sample == 0 {
            ants_peak = ants_peak.max(stop(&world, f, (spec.width, spec.height)));
        }
        if f < frames {
            frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
            pixel_physics::lab::rain::tick(&mut world, &spec, rain);
        }
    }
    println!(
        "DEFENCE_SUMMARY seed={} colony={} defence={} drought_reach={} frames={frames} ants_peak={ants_peak} deepest_generation={}",
        spec.seed,
        with_colony as u8,
        world.plant_defence as u8,
        world.drought_reach as u8,
        world.deepest_generation
    );
}

/// One stop line; returns the ant count.
fn stop(world: &World, f: u64, (width, height): (i32, i32)) -> usize {
    let mut ants = 0usize;
    let mut plant_def: Vec<f32> = Vec::new();
    // Thirst: the plant-wide shortfall over growing plants, and how many
    // are short at all -- what `DROUGHT_REACH` stretches by distance.
    let mut thirsty = 0usize;
    let mut thirst_sum = 0f32;
    let mut plant_cells = 0usize;
    let mut seed_def: Vec<f32> = Vec::new();
    for id in world.live_organism_ids() {
        let Some(s) = world.organism(id) else { continue };
        let def = world.species.get(s.species);
        if def.creature.is_some() {
            ants += 1;
            continue;
        }
        if s.brood.is_some() {
            continue;
        }
        // A dormant seed is a one-cell organism (labgarden's 2026-10-03
        // correction); kept apart from growing plants.
        let seed_only = s.cells.len() <= 1
            && s.cells.keys().all(|&(x, y)| {
                matches!(
                    world.materials.get(world.get(x, y).material).name.as_str(),
                    "seed" | "pip" | "windfall" | "reedseed"
                )
            });
        if seed_only {
            seed_def.push(s.defence);
        } else {
            plant_def.push(s.defence);
            plant_cells += s.cells.len();
            thirst_sum += s.water_desiccation;
            if s.water_desiccation > 0.0 {
                thirsty += 1;
            }
        }
    }
    plant_def.sort_by(|a, b| a.partial_cmp(b).unwrap());
    seed_def.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = |v: &[f32]| if v.is_empty() { f32::NAN } else { v.iter().sum::<f32>() / v.len() as f32 };
    // Edible plant food standing: plant-class food not owned by an animal.
    // `face` is the material's worth; `got` is `creature::food_value`, which
    // is what the defence discounts -- the gap between them is food the
    // plants are withholding.
    let (mut face_j, mut got_j) = (0f64, 0f64);
    for y in 0..height {
        for x in 0..width {
            let c = world.get(x, y);
            let mat = world.materials.get(c.material);
            if !(mat.food_energy > 0.0 && mat.food_class < 0.0) {
                continue;
            }
            if c.organism_id() != 0
                && world
                    .organism(c.organism_id())
                    .is_some_and(|s| world.species.get(s.species).creature.is_some() || s.brood.is_some())
            {
                continue;
            }
            face_j += mat.food_energy as f64;
            got_j += creature::food_value(world, c) as f64;
        }
    }
    println!(
        "STOP f={f} ants={ants} plants={} seeds={} edible_face_kJ={:.0} edible_got_kJ={:.0} def_plant_mean={:.3} def_plant_p50/90={:.3}/{:.3} def_seed_mean={:.3} deepest_gen={} plant_cells={plant_cells} thirsty={thirsty} thirst_mean={:.3} shed_drought={} shed_shade={}",
        plant_def.len(),
        seed_def.len(),
        face_j / 1000.0,
        got_j / 1000.0,
        mean(&plant_def),
        quantile(&plant_def, 0.5),
        quantile(&plant_def, 0.9),
        mean(&seed_def),
        world.deepest_generation,
        thirst_sum / plant_def.len().max(1) as f32,
        world.shed_drought,
        world.shed_shade,
    );
    ants
}
