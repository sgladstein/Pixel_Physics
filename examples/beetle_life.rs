//! **How does each beetle in the lab die?** One row per beetle death in a
//! `labstats`-style bed (`predators=6`), traced per individual
//! (`CLAUDE.md`: "why did it do that" is never a population statistic).
//!
//! Every frame it reads each living beetle's `gnawed` and cell count. A rise
//! in `gnawed` or a lost cell is a bite landing. Bites less than `gap=`
//! frames apart are one fight. Per death it prints: frame, age, energy,
//! cells over its life, how many fights and bites it took, and for the
//! final fight how many cells it started with and the wound it carried in.
//! "Carried in" is the question wound healing answers: if beetles walk into
//! the last fight whole, healing cannot save them.
//!
//! Args: `seed=1 frames=60000 predators=6 gap=300 head=<beetle head_armour>`. Honours
//! `PIXEL_PHYSICS_WOUND_HEAL` like everything else.
//!
//! `cargo run --release --example beetle_life -- seed=1 frames=60000`

use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::Lab;
use pixel_physics::sim::cell::OrganismId;
use std::collections::BTreeMap;

fn arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{name}=")).and_then(|v| v.parse().ok()))
}

#[derive(Default)]
struct Life {
    born: u64,
    cells_max: usize,
    cells: usize,
    gnawed: f32,
    energy: f32,
    bites: u32,
    fights: u32,
    last_bite: u64,
    // At the first bite of the current fight.
    fight_start_cells: usize,
    fight_start_wound: f32,
    fight_start_frame: u64,
    fight_bites: u32,
    quiet_before: u64,
}

fn main() {
    let seed: u64 = arg("seed").unwrap_or(1);
    let frames: u64 = arg("frames").unwrap_or(60_000);
    let predators: u32 = arg("predators").unwrap_or(6);
    let gap: u64 = arg("gap").unwrap_or(300);
    println!(
        "beetle_life: seed={seed} frames={frames} predators={predators} gap={gap} heal={:?} head={:?}",
        pixel_physics::sim::creature::wound_heal_frames(),
        arg::<f32>("head")
    );
    let spec = LabBox { predators: predators as _, seed, ..LabBox::default() };
    let mut lab = Lab::new(spec);
    let beetle = lab.world.species.id_of("beetle").expect("beetle");
    // `head=` overrides `CreatureDef::head_armour` for the run.
    if let Some(v) = arg::<f32>("head") {
        let mut def = lab.world.species.get(beetle).creature.clone().expect("creature");
        def.head_armour = v;
        lab.world.species.set_creature(beetle, def);
    }
    let mut lives: BTreeMap<OrganismId, Life> = BTreeMap::new();
    for f in 0..=frames {
        pixel_physics::sim::frame::step(
            &mut lab.world,
            &mut lab.particles,
            &mut lab.blasts,
            pixel_physics::sim::player::PlayerInput::default(),
            &pixel_physics::sim::player::Tuning::default(),
        );
        let w = &lab.world;
        let mut seen = Vec::new();
        for id in w.live_organism_ids() {
            let Some(st) = w.organism(id) else { continue };
            if st.species != beetle {
                continue;
            }
            seen.push(id);
            let l = lives.entry(id).or_insert_with(|| Life { born: f, ..Default::default() });
            let cells = st.chain.len();
            let bitten = st.gnawed > l.gnawed + 1e-6 || (l.cells > 0 && cells < l.cells);
            if bitten {
                if l.bites == 0 || f - l.last_bite > gap {
                    l.fights += 1;
                    l.fight_start_cells = l.cells.max(cells);
                    l.fight_start_wound = l.gnawed;
                    l.fight_start_frame = f;
                    l.fight_bites = 0;
                    l.quiet_before = if l.bites == 0 { f - l.born } else { f - l.last_bite };
                }
                l.bites += 1;
                l.fight_bites += 1;
                l.last_bite = f;
            }
            l.cells = cells;
            l.cells_max = l.cells_max.max(cells);
            l.gnawed = st.gnawed;
            l.energy = st.energy;
        }
        let dead: Vec<OrganismId> = lives.keys().copied().filter(|id| !seen.contains(id)).collect();
        for id in dead {
            let l = lives.remove(&id).unwrap();
            let in_fight = l.bites > 0 && f - l.last_bite <= gap;
            println!(
                "DEATH f={f} id={id} age={} energy={:.0} cells_last={} cells_max={} bites={} fights={} {}",
                f - l.born,
                l.energy,
                l.cells,
                l.cells_max,
                l.bites,
                l.fights,
                if in_fight {
                    format!(
                        "last_fight: start_cells={} wound_in={:.2} bites={} lasted={} quiet_before={}",
                        l.fight_start_cells,
                        l.fight_start_wound,
                        l.fight_bites,
                        f - l.fight_start_frame,
                        l.quiet_before
                    )
                } else {
                    format!("not_in_fight (last bite {} frames before)", if l.bites > 0 { f - l.last_bite } else { f - l.born })
                }
            );
        }
        if f % 10_000 == 0 {
            println!("  frame {f}: beetles {}", lives.len());
        }
    }
}
