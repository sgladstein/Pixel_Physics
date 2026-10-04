//! **How many ants does it take to kill a beetle?**
//!
//! One beetle on a bare stone floor, N ants placed on both sides of it, every
//! animal handed a bank so nobody starves out of the fight. Runs until the
//! beetle is gone or the budget runs out, per seed, and prints one line per
//! ant count: beetles killed, the median frame of the kill, and ants lost.
//!
//! The question the owner asked on 2026-10-03: *"They are easily killed by a
//! bunch of ants. I am not sure what the ideal number of ants should be
//! required (on average) to kill a beetle."* This measures the answer as it
//! stands, so a change to the beetle has a number to move.
//!
//! The scene copies `creature.rs`'s `a_swarm_gets_through_what_one_mouth_cannot`
//! test (same floor, same spacing rule, same `test_world` settings: brood off,
//! budding anywhere, ant `scent_spread` 0) so the two agree on what a fight is.
//!
//! Args: `ants=1,2,4,8` `seeds=8` `frames=3000` `plate=<beetle
//! penetration_resistance, default the shipped material's>` `armour=<beetle
//! TRAIT_ARMOUR allele, default 0>`. Echoes every parameter it used.
//!
//! `cargo run --release --example beetle_duel -- ants=1,2,4,8 seeds=8`

use pixel_physics::sim::brain::{BrainInput, BrainOutput};
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::cell::OrganismId;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::creature::{plant_creature_seed, probe_full};
use pixel_physics::sim::organism::TRAIT_ARMOUR;
use pixel_physics::sim::scheduler;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{name}=")).and_then(|v| v.parse().ok()))
}

fn run(w: &mut World, frames: usize) {
    for _ in 0..frames {
        w.begin_step();
        scheduler::step(w);
        w.end_step();
    }
}

fn spawn(w: &mut World, species: &str, x: i32, y: i32) -> OrganismId {
    match plant_creature_seed(w, x, y, species) {
        Some(site) => {
            w.schedule_active_site(site);
            w.get(x, y).organism_id()
        }
        None => 0,
    }
}

fn world(seed: u64, plate: Option<f32>) -> World {
    let mut w = World::new(Rect::new(0, 0, 199, 199));
    w.brood = Some(false);
    w.bud_at_nest = Some(false);
    w.seed = 1234 + seed * 7919;
    if let Some(id) = w.species.id_of("ant") {
        if let Some(def) = w.species.get(id).creature.as_ref() {
            let mut def = def.clone();
            def.scent_spread = 0.0;
            w.species.set_creature(id, def);
        }
    }
    if let (Some(p), Some(m)) = (plate, w.materials.id_of("beetle")) {
        w.materials.get_mut(m).penetration_resistance = p;
    }
    let floor = w.materials.id_of("stone").expect("stone");
    for x in 40..160 {
        w.set(x, 120, Cell::new(floor, 0).with_attached(true));
    }
    w
}

fn main() {
    // Births inherit exactly unless PIXEL_PHYSICS_MUTATION=on (`creature::mutation_of`).
    pixel_physics::sim::creature::mutation_off_for_measuring();
    let ants: Vec<i32> = arg::<String>("ants")
        .unwrap_or_else(|| "1,2,4,8".into())
        .split(',')
        .filter_map(|s| s.parse().ok())
        .collect();
    let seeds: u64 = arg("seeds").unwrap_or(8);
    let budget: usize = arg("frames").unwrap_or(3000);
    let plate: Option<f32> = arg("plate");
    let armour: f32 = arg("armour").unwrap_or(0.0);
    // `bank=0` leaves every animal on its species' own starting energy. A
    // fed animal fights differently from a hungry one -- the beetle's
    // hunting is hunger-driven -- so the shipped test's 100,000 J bank is
    // one arm, not the answer.
    let bank: f32 = arg("bank").unwrap_or(100_000.0);
    let trace = arg::<u32>("trace").unwrap_or(0) != 0;
    let shipped = {
        let w = World::new(Rect::new(0, 0, 9, 9));
        w.materials
            .id_of("beetle")
            .map(|m| w.materials.get(m).penetration_resistance)
    };
    println!("beetle_duel: ants={ants:?} seeds={seeds} frames={budget} plate={} (shipped {shipped:?}) armour={armour} bank={bank}", plate.map_or("shipped".into(), |p| p.to_string()));

    for &n in &ants {
        let mut kills = Vec::new();
        let mut lost = Vec::new();
        for seed in 0..seeds {
            let mut w = world(seed, plate);
            let beetle = spawn(&mut w, "beetle", 100, 119);
            assert_ne!(
                beetle, 0,
                "the beetle did not place; the scene does not contain a fight"
            );
            w.set_organism_trait(beetle, TRAIT_ARMOUR, armour);
            if bank > 0.0 {
                w.set_organism_energy(beetle, bank);
            }
            let mut team = Vec::new();
            for i in 0..n {
                // Alternate sides, two columns apart, clear of the beetle's
                // own cells: a spawn onto an occupied cell fails.
                let x = if i % 2 == 0 {
                    92 - (i / 2) * 2
                } else {
                    104 + (i / 2) * 2
                };
                let a = spawn(&mut w, "ant", x, 119);
                if a != 0 {
                    if bank > 0.0 {
                        w.set_organism_energy(a, bank);
                    }
                    team.push(a);
                }
            }
            let mut died = None;
            // `trace=1`: the beetle's last frames before it dies -- what it
            // sensed (Crowding), what the gang gate made of it (hidden 0 and
            // 1), what it chose (Feed, Dig, Move, Tumble), how many ants'
            // cells touched it and its bank. Read the rows of every death,
            // not a population figure (`CLAUDE.md`, tracing individuals).
            let mut tail: std::collections::VecDeque<String> = std::collections::VecDeque::new();
            for f in 1..=budget {
                if trace {
                    if let Some(st) = w.organism(beetle) {
                        let (hx, hy) = st.chain[0];
                        let def = w.species.get(st.species).creature.clone().expect("creature");
                        let (i, h, o, _) = probe_full(&w, hx, hy, beetle, &def);
                        let mut touching = std::collections::BTreeSet::new();
                        for &(cx, cy) in &st.chain {
                            for dy in -1..=1 {
                                for dx in -1..=1 {
                                    let id = w.get(cx + dx, cy + dy).organism_id();
                                    if id != 0 && id != beetle {
                                        touching.insert(id);
                                    }
                                }
                            }
                        }
                        tail.push_back(format!(
                            "    f={f} alarm={:.2} crowd={:.2} h0={:.2} h1={:.2} feed={:.2} dig={:.2} move={:.2} tumble={:.2} ants_touching={} cells={} energy={:.0}",
                            i[BrainInput::Alarm as usize],
                            i[BrainInput::Crowding as usize],
                            h[0],
                            h[1],
                            o[BrainOutput::Feed as usize],
                            o[BrainOutput::Dig as usize],
                            o[BrainOutput::Move as usize],
                            o[BrainOutput::Tumble as usize],
                            touching.len(),
                            st.chain.len(),
                            st.energy
                        ));
                        if tail.len() > 12 {
                            tail.pop_front();
                        }
                    }
                }
                run(&mut w, 1);
                if w.organism(beetle).is_none() {
                    died = Some(f);
                    break;
                }
            }
            if trace && died.is_some() {
                for l in &tail {
                    println!("{l}");
                }
            }
            kills.push(died);
            lost.push(team.iter().filter(|a| w.organism(**a).is_none()).count());
            println!(
                "  ants={n} seed={seed} placed={} beetle_died_at={} ants_lost={}",
                team.len(),
                died.map_or("never".into(), |f| f.to_string()),
                lost.last().unwrap()
            );
        }
        let killed = kills.iter().filter(|k| k.is_some()).count();
        let mut frames: Vec<usize> = kills.iter().map(|k| k.unwrap_or(budget)).collect();
        frames.sort_unstable();
        let mut l = lost.clone();
        l.sort_unstable();
        println!(
            "SUMMARY ants={n} killed={killed}/{seeds} median_frame={} (budget {budget} counts as never) median_ants_lost={}",
            frames[frames.len() / 2],
            l[l.len() / 2]
        );
    }
}
