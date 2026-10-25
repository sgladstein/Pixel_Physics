//! **How does a grown lab colony treat a beetle that sits in its nest?** Step 0
//! of the guest seat (`Reports/teaching-the-colony-2026-10-25/seat-guest.md`,
//! "Look at the gate before building"): before any rule is built, count what
//! today's ants do to a passive visitor -- touches, bites landed, how long it
//! lives -- under each of the routes the design asks the owner to choose
//! between.
//!
//! ```text
//! cargo run --release --example guestprobe -- brain=old colony=stranger scent=far seed=1
//! ```
//!
//! - `brain=old|std`: the lab's hand-built ant (the shipped default) or the
//!   trained standard pool (ANT BRAIN: NEW).
//! - `colony=stranger|host`: the beetle founds its own colony number (what any
//!   beetle is today) or joins the ant colony's number.
//! - `scent=far|near`: the beetle's authored scent (0.8, 0.8, 0.8), or set to
//!   the nest's own odour on landing.
//! - `grow=N` frames of colony before the beetle lands (20,000), `after=N`
//!   frames watched after (6,000), `place=door|pile` (on the mound beside the
//!   door, or on the deepest brood found).
//!
//! The beetle is marked steered with an empty input, so it stands where it was
//! put and never bites: what happens to it is the colony's doing alone.
//!
//! Prints one `RESULT` line: `alive` frames lived (of `after`), `touch` ant
//! ticks spent touching it, `bites` gnaw increments it took, `cells_lost`,
//! `stranger_touches` and `mind_bites` (colony-wide deltas over the window),
//! `fed` energy it gained from anyone (it never eats, so a rise is a share).

use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::Lab;
use pixel_physics::sim::cell::OrganismId;
use pixel_physics::sim::creature;
use pixel_physics::sim::creature::guest;
use pixel_physics::sim::organism::{TRAIT_SCENT_A, TRAIT_SCENT_B, TRAIT_SCENT_C};
use pixel_physics::sim::scheduler::ActiveKind;
use pixel_physics::sim::steer::{self, SteerInput};

fn arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{name}=")).and_then(|v| v.parse().ok()))
}

fn main() {
    creature::mutation_off_for_measuring();
    let brain: String = arg("brain").unwrap_or_else(|| "old".into());
    let colony_arm: String = arg("colony").unwrap_or_else(|| "stranger".into());
    let scent_arm: String = arg("scent").unwrap_or_else(|| "far".into());
    let place: String = arg("place").unwrap_or_else(|| "door".into());
    // `body=beetle|rove`: the shipped 2x2 beetle, or the guest seat's
    // ant-sized rove beetle.
    let body: String = arg("body").unwrap_or_else(|| "beetle".into());
    let seed: u64 = arg("seed").unwrap_or(1);
    let grow: u64 = arg("grow").unwrap_or(20_000);
    let after: u64 = arg("after").unwrap_or(6_000);
    let scenario: String = arg("scenario").unwrap_or_else(|| "steady_income".into());
    assert!(matches!(brain.as_str(), "old" | "std"), "brain= is old or std");
    assert!(matches!(colony_arm.as_str(), "stranger" | "host"), "colony= is stranger or host");
    assert!(matches!(scent_arm.as_str(), "far" | "near"), "scent= is far or near");
    assert!(matches!(place.as_str(), "door" | "pile"), "place= is door or pile");
    println!("guestprobe: body={body} brain={brain} colony={colony_arm} scent={scent_arm} place={place} seed={seed} grow={grow} after={after} scenario={scenario}");

    let mut sc = Scenario::load(&scenario).unwrap_or_else(|e| panic!("scenario {scenario}: {e}"));
    sc.bed.seed = seed;
    let mut lab = Lab::new(sc.bed.clone());
    println!("  {}", lab.load_scenario(sc));
    if brain == "std" {
        println!("  {}", lab.set_new_ant(true));
    }
    for _ in 0..grow {
        lab.tick_for_harness();
    }
    let ant = lab.world.species.id_of("ant").expect("ant ships");
    let ants: Vec<OrganismId> = lab.world.live_organism_ids().into_iter().filter(|&id| lab.world.organism(id).is_some_and(|s| s.species == ant)).collect();
    let host = ants.iter().filter_map(|&id| lab.world.organism(id).map(|s| s.colony)).max_by_key(|&c| ants.iter().filter(|&&i| lab.world.colony_of(i) == c).count()).expect("a colony grew");
    let brood = lab.world.live_brood_ids().len();
    let nest = lab.world.nest_sites.first().cloned().expect("the colony has a nest site");
    println!("  frame {}: {} ants of colony {host}, {brood} brood, nest at ({}, {}) scent {:?}", lab.world.frame, ants.len(), nest.x, nest.y, nest.scent);

    // Where it lands: beside the door on the mound, or on the deepest brood.
    // The brood cells, found on the grid (a brood item is a stage cell, so
    // it has no chain to read a position from).
    let brood_mat = lab.world.materials.id_of("brood").expect("brood ships");
    let mut brood_cells: Vec<(i32, i32)> = Vec::new();
    for y in nest.surface - 4..nest.surface + 60 {
        for x in nest.x - 80..=nest.x + 80 {
            if lab.world.get(x, y).material == brood_mat {
                brood_cells.push((x, y));
            }
        }
    }
    // **Who could feed a beggar** (review of plan v1, finding 6): the adults
    // touching brood, their banks against 0.7 of the grant, and their crops.
    {
        let grant = lab.world.species.get(ant).creature.as_ref().map_or(200.0, |d| d.start_energy);
        let mut seen: Vec<OrganismId> = Vec::new();
        for &(bx, by) in &brood_cells {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let o = lab.world.get(bx + dx, by + dy).organism_id();
                    if o != 0 && !seen.contains(&o) && lab.world.organism(o).is_some_and(|s| s.species == ant && s.brood.is_none()) {
                        seen.push(o);
                    }
                }
            }
        }
        let banks: Vec<f32> = seen.iter().filter_map(|&o| lab.world.organism(o).map(|s| s.energy / grant)).collect();
        let crops: Vec<f32> = seen.iter().filter_map(|&o| lab.world.organism(o).map(|s| s.crop.map_or(0.0, |c| c.worth()))).collect();
        let donors = banks.iter().filter(|&&b| b > 0.7).count();
        let mean = |v: &[f32]| if v.is_empty() { 0.0 } else { v.iter().sum::<f32>() / v.len() as f32 };
        let all_banks: Vec<f32> = ants.iter().filter_map(|&o| lab.world.organism(o).map(|s| s.energy / grant)).collect();
        println!(
            "DONORS seed={seed} brood_cells={} adults_at_brood={} over_0.7={donors} mean_bank={:.2} mean_crop={:.0} | all adults mean_bank={:.2} over_0.7={}",
            brood_cells.len(),
            seen.len(),
            mean(&banks),
            mean(&crops),
            mean(&all_banks),
            all_banks.iter().filter(|&&b| b > 0.7).count()
        );
    }
    let site = match place.as_str() {
        "pile" => brood_cells.iter().copied().max_by_key(|&(_, y)| y),
        _ => None,
    }
    .unwrap_or((nest.x + arg::<i32>("dx").unwrap_or(4), nest.surface - 2));
    let mut placed = None;
    let col = (colony_arm == "host").then_some(host);
    if place == "pile" {
        'near: for r in 1..6 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if let Some(s) = creature::plant_creature_seed_in(&mut lab.world, site.0 + dx, site.1 + dy, &body, col) {
                        if let ActiveKind::Creature { organism } = s.kind {
                            lab.world.schedule_active_site(s);
                            placed = Some(organism);
                            break 'near;
                        }
                    }
                }
            }
        }
    }
    'search: for off in [0, 2, -2, 4, -4, 6, -6, 8, -8, 10, -10] {
        if placed.is_some() {
            break;
        }
        for lift in 0..24 {
            if let Some(s) = creature::plant_creature_seed_in(&mut lab.world, site.0 + off, site.1 - lift, &body, col) {
                if let ActiveKind::Creature { organism } = s.kind {
                    lab.world.schedule_active_site(s);
                    placed = Some(organism);
                    break 'search;
                }
            }
        }
    }
    let guest = placed.expect("the beetle fits somewhere above the site");
    if scent_arm == "near" {
        for (slot, v) in [TRAIT_SCENT_A, TRAIT_SCENT_B, TRAIT_SCENT_C].into_iter().zip(nest.scent) {
            lab.world.set_organism_trait(guest, slot, v);
        }
    }
    // `drive=right|bite`: the steer check. `right` holds the arrow east the
    // whole window (does the steer hook move a beetle?); `bite` holds B (does
    // the beetle's own bite take brood or ants beside it?).
    let drive: String = arg("drive").unwrap_or_else(|| "none".into());
    let held = match drive.as_str() {
        "right" => SteerInput { dir: Some((1, 0)), ..Default::default() },
        "down" => SteerInput { dir: Some((0, 1)), ..Default::default() },
        "downright" => SteerInput { dir: Some((1, 1)), ..Default::default() },
        "bite" => SteerInput { bite: true, ..Default::default() },
        _ => SteerInput::default(),
    };
    assert!(steer::mark(&mut lab.world, guest, Some(held)));
    // `ladder=1 gap=G`: register the guest with the smell ladder, its scent
    // set G (in tolerance units of 1.0) from the nest's odour and fading back
    // to there, so it stays at G unless a verb moves it.
    let ladder = arg::<u8>("ladder").unwrap_or(0) == 1;
    if ladder {
        let want: f32 = arg("gap").unwrap_or(2.0);
        let (home, got) = guest::scent_away_from(nest.scent, want);
        guest::register(&mut lab.world, ant, guest, home);
        println!("  ladder on: guest scent {home:?}, {got:.2} from the nest odour (asked {want})");
    }
    let brood0 = lab.world.live_brood_ids().len();
    let g = lab.world.organism(guest).expect("live").clone();
    println!("  beetle {guest} colony {} at {:?}, {} cells, energy {:.0}", g.colony, g.chain.first(), g.chain.len(), g.energy);

    if arg::<u8>("look").unwrap_or(0) == 1 {
        let c = if arg::<u8>("nestlook").unwrap_or(0) == 1 { (nest.x, nest.surface + 14) } else { g.chain.first().copied().unwrap_or(site) };
        look(&lab.world, guest, ant, c, arg("half").unwrap_or(8));
    }
    let s0 = lab.world.creature_stats;
    let mb0 = lab.world.minds.as_ref().map_or(0, |m| m.counts.bites);
    let e0 = g.energy;
    let (mut touch, mut bites, mut alive, mut peak_e) = (0u64, 0u64, 0u64, e0);
    let mut last_gnaw = g.gnawed;
    let cells0 = g.chain.len();
    let mut last_cells = cells0;
    // `script=1`: a fixed key script for the guest's verbs (needs `ladder=1`):
    // rub whoever is beside you every 20 frames for the first 1,200 frames,
    // soak throughout, then beg every 90 frames. `script=2` adds a larva
    // killed at frame 3,000 and E pressed every 30 frames after it.
    let script: u8 = arg("script").unwrap_or(0);
    for f in 0..after {
        if script > 0 {
            if let Some(v) = lab.world.visitors.as_mut() {
                v.input.soak = true;
                if f < 1200 && f % 20 == 0 {
                    v.input.rub = true;
                }
                if f >= 1200 && f % 90 == 0 {
                    v.input.beg = true;
                }
                if script == 2 && f == 3000 {
                    v.input.kill = true;
                }
                if script == 2 && f > 3000 && f % 30 == 0 {
                    v.input.eat = true;
                }
            }
            if f % 600 == 0 {
                if let Some(v) = lab.world.visitors.as_ref() {
                    let c = v.counts;
                    println!(
                        "    f+{f}: gap {:.2} energy {:.0} rolls {} ign {} ant {} disp {} nip {} rubs {} fed {} ({:.0} J) refused {} empty {}",
                        v.list.first().map_or(f32::NAN, |x| x.gap),
                        lab.world.organism(guest).map_or(0.0, |s| s.energy),
                        c.rolls, c.ignored, c.antennated, c.displayed, c.nipped, c.rubs, c.fed, c.fed_j, c.refused, c.empty
                    );
                }
            }
        }
        lab.tick_for_harness();
        let Some(st) = lab.world.organism(guest) else {
            println!("  frame {}: the beetle died after {f} frames", lab.world.frame);
            break;
        };
        alive = f + 1;
        if drive != "none" && f % 48 == 0 {
            println!("    f+{f}: head {:?} heading {} airborne {}", st.chain.first(), st.heading, st.flight.is_some());
        }
        if st.gnawed > last_gnaw + 1e-6 || st.chain.len() < last_cells {
            bites += 1;
        }
        last_gnaw = st.gnawed;
        last_cells = st.chain.len();
        peak_e = peak_e.max(st.energy);
        let chain = st.chain.clone();
        let mut toucher: Vec<OrganismId> = Vec::new();
        for &(x, y) in &chain {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let o = lab.world.get(x + dx, y + dy).organism_id();
                    if o != 0 && o != guest && lab.world.organism(o).is_some_and(|s| s.species == ant) && !toucher.contains(&o) {
                        toucher.push(o);
                    }
                }
            }
        }
        touch += toucher.len() as u64;
    }
    if arg::<u8>("look").unwrap_or(0) == 1 {
        if let Some(&c) = lab.world.organism(guest).and_then(|s| s.chain.first()) {
            println!("  at the end:");
            look(&lab.world, guest, ant, c, arg("half").unwrap_or(8));
        }
    }
    let s1 = &lab.world.creature_stats;
    let mb1 = lab.world.minds.as_ref().map_or(0, |m| m.counts.bites);
    let cells_lost = cells0 - lab.world.organism(guest).map_or(0, |s| s.chain.len());
    println!(
        "RESULT body={body} brain={brain} colony={colony_arm} scent={scent_arm} place={place} seed={seed} alive={alive} touch={touch} bites={bites} cells_lost={cells_lost} stranger_touches={} contests={} mind_bites={} fed={:.0}",
        s1.stranger_touches - s0.stranger_touches,
        s1.contests - s0.contests,
        mb1 - mb0,
        peak_e - e0
    );
    if let Some(v) = lab.world.visitors.as_ref() {
        let c = v.counts;
        println!(
            "LADDER seed={seed} gap={} alive={alive} episodes={} rolls={} ignore={} antennate={} display={} nip={} cells_lost={} deaths={} rubs={} begs={} fed={} fed_j={:.0} refused={} empty={} larvae_killed={} eaten_j={:.0}",
            arg::<f32>("gap").unwrap_or(2.0),
            c.episodes, c.rolls, c.ignored, c.antennated, c.displayed, c.nipped, c.cells_lost, c.deaths, c.rubs, c.begs, c.fed, c.fed_j, c.refused, c.empty, c.larvae_killed, c.eaten_j
        );
    }
    println!(
        "  drive={drive}: head now {:?}, brood {brood0} -> {}, kills booked by the beetle's colony {:?}",
        lab.world.organism(guest).and_then(|s| s.chain.first().copied()),
        lab.world.live_brood_ids().len(),
        lab.world.creature_stats.attack_kills
    );
}

/// A crude look at the ground round a point: '@' the guest, 'a' an ant, 'o'
/// any other organism, '#' solid or powder, '~' liquid, ',' gas, '.' air.
fn look(world: &pixel_physics::sim::world::World, guest: OrganismId, ant: pixel_physics::sim::organism::SpeciesId, (hx, hy): (i32, i32), half: i32) {
    for y in hy - half..=hy + half {
        let row: String = (hx - 30..=hx + 30)
            .map(|x| {
                let c = world.get(x, y);
                let o = c.organism_id();
                if o == guest {
                    '@'
                } else if o != 0 && world.organism(o).is_some_and(|s| s.species == ant) {
                    'a'
                } else if o != 0 {
                    'o'
                } else if c.material == pixel_physics::sim::material::EMPTY {
                    '.'
                } else {
                    match world.materials.kind(c.material) {
                        pixel_physics::sim::material::MaterialKind::Liquid => '~',
                        pixel_physics::sim::material::MaterialKind::Gas => ',',
                        _ => '#',
                    }
                }
            })
            .collect();
        println!("    {y:4} {row}");
    }
}
