//! **What the plant-side soil switches would change in the lab box, and
//! what they cost** -- `PIXEL_PHYSICS_DRY_ROOT_COST` (dry ground is harder
//! for a root) and `PIXEL_PHYSICS_EMERGENCE_DEPTH` (a seed buried past its
//! reserve stays down). Both are read once per process, so an A/B is two
//! runs of this binary with the same arguments, one with the variable set.
//!
//! Builds the shipped `LabBox` (one colony, the default plants and light
//! rain) and runs it through `lab::tick_world`, the lab's own sequence, so
//! the mister rains here as it does in the game. At each stop it prints one
//! line per seed:
//!
//! - **plants / roots / deepest**: live organisms, root cells, and the
//!   deepest root's rows under the surface -- what a re-tune would move.
//! - **germ**: every germination so far (`World::germinations`).
//! - **buried d0-2 / d3-6 / d7+**: seeds standing as `buriedseed`, by the
//!   cover over them -- the population the emergence limit acts on, so a
//!   zero here means the switch has nothing to hold.
//! - **front dry / part / wet**: water-holding soil cells touching a root,
//!   by `plant_available_fraction` (0, between, 1) -- where roots would grow
//!   next and so where the dryness term would bill them. **mean@2** is the
//!   mean multiplier `on` (k = 2) would charge over that front: 1.00 says
//!   the switch would cost the economy nothing at this stop.
//! - **s**: wall-clock seconds for the run so far, for the paired frame cost.
//!
//! The front is a census of standing state, not of growth steps: it says
//! what the next step would pay, not what past steps paid.
//!
//! `seeds=1,2,3 stops=20000,40000,60000 colonies=1 rain=light` (`off`,
//! `light`, `steady`, `heavy`: the dryness term only has dry ground to bill
//! when the mister is off).

use pixel_physics::lab::rain::Rain;
use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::material::{self, MaterialKind};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::update::plant_available_fraction;
use pixel_physics::sim::{player, World};
use std::time::Instant;

fn arg<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter().find_map(|a| a.strip_prefix(key).and_then(|r| r.strip_prefix('=')))
}

fn list(v: &str) -> Vec<u64> {
    v.split(',').map(|s| s.parse().unwrap_or_else(|_| panic!("not a number: {s}"))).collect()
}

fn census(w: &World, ground_y: i32) -> String {
    let ids = w.live_organism_ids();
    let (mut plants, mut roots, mut deepest) = (0usize, 0usize, 0i32);
    let (mut dry, mut part, mut wet, mut bill) = (0usize, 0usize, 0usize, 0.0f64);
    let mut front = std::collections::HashSet::new();
    for id in &ids {
        let Some(state) = w.organism(*id) else { continue };
        let Some(root) = w.materials.id_of(&w.species.get(state.species).root_material) else { continue };
        plants += 1;
        for &(x, y) in state.cells.keys() {
            if w.get(x, y).material != root {
                continue;
            }
            roots += 1;
            deepest = deepest.max(y - ground_y + 1);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let c = w.get(x + dx, y + dy);
                    let m = w.materials.get(c.material);
                    if c.organism_id() == 0 && m.kind == MaterialKind::Powder && m.water_capacity > 0 && m.penetration_resistance > 0.0 {
                        front.insert((x + dx, y + dy));
                    }
                }
            }
        }
    }
    for &(x, y) in &front {
        let f = plant_available_fraction(w.get(x, y));
        if f <= 0.0 {
            dry += 1;
        } else if f >= 1.0 {
            wet += 1;
        } else {
            part += 1;
        }
        bill += 1.0 + f64::from(1.0 - f);
    }
    let mean = if front.is_empty() { 1.0 } else { bill / front.len() as f64 };
    let mut buried = [0usize; 3];
    if let Some(bs) = w.materials.id_of("buriedseed") {
        let b = w.bounds().expect("a lab box is bounded");
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                if w.get(x, y).material != bs {
                    continue;
                }
                let mut cover = 0;
                while cover < 64 && w.get(x, y - 1 - cover).material != material::EMPTY {
                    cover += 1;
                }
                buried[if cover <= 2 { 0 } else if cover <= 6 { 1 } else { 2 }] += 1;
            }
        }
    }
    format!(
        "plants {plants:3} roots {roots:5} deepest {deepest:3} germ {:4} | buried d0-2 {:3} d3-6 {:3} d7+ {:3} | front dry {dry:4} part {part:4} wet {wet:4} mean@2 {mean:.2}",
        w.germinations, buried[0], buried[1], buried[2]
    )
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seeds = list(arg(&args, "seeds").unwrap_or("1,2,3"));
    let stops = list(arg(&args, "stops").unwrap_or("20000,40000,60000"));
    let colonies: usize = arg(&args, "colonies").map_or(1, |v| v.parse().expect("colonies"));
    let rain = match arg(&args, "rain").unwrap_or("light") {
        "off" => Rain::Off,
        "light" => Rain::Light,
        "steady" => Rain::Steady,
        "heavy" => Rain::Heavy,
        other => panic!("rain={other}: off, light, steady or heavy"),
    };
    let env = |k: &str| std::env::var(k).unwrap_or_else(|_| "off".into());
    println!(
        "soil_switches seeds={seeds:?} stops={stops:?} colonies={colonies} rain={rain:?} DRY_ROOT_COST={} EMERGENCE_DEPTH={}",
        env("PIXEL_PHYSICS_DRY_ROOT_COST"),
        env("PIXEL_PHYSICS_EMERGENCE_DEPTH")
    );
    let tuning = player::Tuning::default();
    for &seed in &seeds {
        let spec = LabBox { colonies, seed, rain, ..LabBox::default() };
        let mut w = spec.build();
        let (mut p, mut bl) = (ParticleSystem::default(), Blasts::default());
        let t0 = Instant::now();
        let mut done = 0u64;
        for &stop in &stops {
            while done < stop {
                pixel_physics::lab::tick_world(&mut w, &mut p, &mut bl, &tuning, &spec, None);
                done += 1;
            }
            println!("seed {seed} frame {stop:6} | {} | s {:.1}", census(&w, spec.ground_y), t0.elapsed().as_secs_f64());
        }
    }
}
