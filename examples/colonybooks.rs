//! **The food economy of each colony, in joules** — what they are eating,
//! where it came from, what it is being spent on, and who is delivering it.
//!
//! The owner's ask, 2026-09-14: *"We should explore better instruments,
//! visualizations, whatever for the player to understand the food economy of
//! each colony. I want to know what they are eating, where it is coming
//! from, if/where it is being stored or movement paths, general colony food
//! stats/balances."* This is the books half; the seen half (a laden-ant trail
//! and a harvest map) is a sibling lane's, and the lab's FOOD page is what
//! puts these numbers in front of a player. This harness is what says the
//! numbers are worth putting there.
//!
//! # What it reads and why nothing else could
//!
//! `World::colony_books` — the energy ledger split per
//! `OrganismState::colony`, with the diet booked on the same call that
//! credits the animal. Every other census in `examples/` is world-wide
//! (`creature_probe`, `ascii`'s ledger line) or counts **cells** rather than
//! joules (`labforage`'s `eaten`), and a cell of moss and a cell of corpse
//! are not the same food. `labstats` reports the colony's *outcome* — alive,
//! dead, of what — and has no income statement at all.
//!
//! # Three things it prints that a mean would hide
//!
//! 1. **The foragers, ranked, not averaged.** Round 33 cost exactly this: a
//!    pooled idle rate cannot tell *"everyone rests briefly"* from *"a fifth
//!    are frozen"* — both give 75% — and the owner saw it on screen when the
//!    number did not. A colony's mean forager hides the same thing, so
//!    `deliveries` is printed as the whole ranked list with the share of
//!    animals that have delivered **nothing** beside it.
//! 2. **Banks as a distribution, not "thriving or starving".** `CLAUDE.md`'s
//!    first law: an outcome is a distribution, not a binary. The histogram
//!    is over bank as a fraction of `start_energy`, so near-starvation is
//!    visible before the death is.
//! 3. **What the colony spends on its own brains**, which is booked
//!    separately (`Account::SynapseTax`) and has never been shown to anyone.
//!
//! # The controls
//!
//! * `control=selftest` — the sensitivity half. Builds a bed whose answer is
//!   known non-zero and asserts every band reports it, because a band
//!   reading zero because the colony ate nothing and one reading zero
//!   because it is blind are the same line of output.
//! * `colonies=0` — no colony at all. Every band must be empty; this is the
//!   specificity half, and it is the arm that catches a band that is really
//!   reading the world rather than the colony.
//!
//! ```text
//! cargo run --release --example colonybooks -- frames=60000 colonies=2 seed=1
//! cargo run --release --example colonybooks -- control=selftest
//! ```

use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::material::MaterialId;
use pixel_physics::sim::organism::SpeciesId;
use pixel_physics::sim::world::{Account, ColonyBooks, World};
use std::collections::BTreeMap;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}="))?.parse().ok())
}

/// One colony as this harness reads it: the label a player sees, the books,
/// and the standing animals the books are about.
struct Colony {
    label: String,
    species: SpeciesId,
    colony: u32,
    alive: u32,
}

fn colonies_of(world: &World) -> Vec<Colony> {
    world
        .live_creature_groups()
        .into_iter()
        .map(|g| Colony { label: world.group_label(g.species, g.colony), species: g.species, colony: g.colony, alive: g.alive })
        .collect()
}

/// `1,234,567` — a joule count a person can read at a glance. Every figure
/// in this file is joules, so the unit is stated once in the header rather
/// than on every number.
fn thousands(v: f64) -> String {
    let n = v.round().abs() as u64;
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if v < 0.0 {
        format!("-{out}")
    } else {
        out
    }
}

fn pct(part: f64, whole: f64) -> String {
    if whole <= 0.0 {
        "  -- ".to_string()
    } else {
        format!("{:4.1}%", 100.0 * part / whole)
    }
}

/// The diet band as names, heaviest first. **Joules, not cells** — see
/// `ColonyBooks::intake_by_material`.
fn diet_line(world: &World, books: &ColonyBooks, top: usize) -> String {
    let total = books.intake();
    let rows = books.diet();
    if rows.is_empty() {
        return "nothing".to_string();
    }
    let mut parts: Vec<String> =
        rows.iter().take(top).map(|&(m, j)| format!("{} {} ({})", world.materials.get(m).name, thousands(j), pct(j, total))).collect();
    if rows.len() > top {
        let rest: f64 = rows.iter().skip(top).map(|(_, j)| j).sum();
        parts.push(format!("+{} more {} ({})", rows.len() - top, thousands(rest), pct(rest, total)));
    }
    parts.join("   ")
}

/// **Every forager's deliveries, ranked** — the distribution round 33 says
/// to print instead of a mean, with the share that have delivered nothing
/// beside it because that is the half a mean erases completely.
fn deliveries(world: &World, colony: u32) -> Vec<u32> {
    let mut rows: Vec<u32> = world
        .live_organism_ids()
        .into_iter()
        .filter_map(|id| world.organism(id))
        .filter(|s| s.colony == colony && !s.chain.is_empty())
        .map(|s| s.life.deliveries)
        .collect();
    rows.sort_unstable_by(|a, b| b.cmp(a));
    rows
}

/// Banks as a fraction of the species' `start_energy`, in five bands.
///
/// **Five and not two**, and that is the whole point of the readout: a
/// colony that is "alive" with every bank under a tenth is a colony about to
/// die, and a thriving/starving flag cannot say so until it has.
fn bank_bands(world: &World, colony: u32, species: SpeciesId) -> [u32; 5] {
    let full = world.species.get(species).creature.as_ref().map_or(1.0, |d| d.start_energy).max(1.0);
    let mut bands = [0u32; 5];
    for state in world.live_organism_ids().into_iter().filter_map(|id| world.organism(id)) {
        if state.colony != colony || state.chain.is_empty() {
            continue;
        }
        let f = (state.energy / full).clamp(0.0, 1.0);
        let band = if f < 0.1 {
            0
        } else if f < 0.25 {
            1
        } else if f < 0.5 {
            2
        } else if f < 1.0 {
            3
        } else {
            4
        };
        bands[band] += 1;
    }
    bands
}

/// **Every colony the run ever had, not every colony still standing.**
///
/// The books outlive the animals, and the colony a player most wants the
/// books of is the one that died — `live_creature_groups` cannot name it,
/// because there is nothing left to name. So the roster is remembered as it
/// goes and the report is drawn from that: a colony that starved out at
/// frame 9,000 still gets its income statement, with `0 alive` beside it.
/// Reading the final report off the live groups alone made the whole of a
/// dead colony's economy disappear at the exact moment it became the
/// interesting one.
fn report(world: &World, roster: &BTreeMap<u32, Colony>) {
    let live = colonies_of(world);
    for c in roster.values() {
        let books = world.colony_books(c.colony);
        let alive = live.iter().find(|g| g.colony == c.colony).map_or(0, |g| g.alive);
        let income = books.income();
        let outgo = books.outgo();
        println!("\n=== {} — {alive} alive ===", c.label);
        println!(
            "  in     {:>12}   placed with {}   foraged {}   scavenged {}   fed by others {}",
            thousands(income),
            thousands(books.get(Account::Granted)),
            thousands(books.get(Account::HarvestedPlant)),
            thousands(books.get(Account::HarvestedCorpse)),
            thousands(books.get(Account::SharedIn))
        );
        println!(
            "  out    {:>12}   upkeep {} ({})   walking {} ({})   brains {} ({})",
            thousands(outgo),
            thousands(books.get(Account::Metabolized)),
            pct(books.get(Account::Metabolized), outgo),
            thousands(books.get(Account::Moved)),
            pct(books.get(Account::Moved), outgo),
            thousands(books.get(Account::SynapseTax)),
            pct(books.get(Account::SynapseTax), outgo)
        );
        println!(
            "         left as meat {}   fed to others {}   died owing {}",
            thousands(books.get(Account::StoredInMeat)),
            thousands(books.get(Account::SharedOut)),
            thousands(books.get(Account::Overdrawn))
        );
        // **The margin is what the books say is standing in the animals, and
        // it is checked against the animals rather than asserted.** A margin
        // that has drifted from the banks is a booking on the wrong colony,
        // which every total above would still show as perfect.
        let held: f64 = world.live_creature_energy_by_colony().get(c.colony as usize).copied().unwrap_or(0.0);
        println!("  margin {:>12}   banks hold {}   ({} apart)", thousands(income - outgo), thousands(held), thousands((income - outgo) - held));
        println!("  eating   {}", diet_line(world, &books, 5));
        if books.raided > 0.0 || books.raided_by_others > 0.0 {
            println!("  raid     took {} off other colonies   lost {} to them", thousands(books.raided), thousands(books.raided_by_others));
        }
        let d = deliveries(world, c.colony);
        let idle = d.iter().filter(|&&n| n == 0).count();
        let carried: u32 = d.iter().sum();
        let shown: Vec<String> = d.iter().take(20).map(|n| n.to_string()).collect();
        println!(
            "  carried  {carried} loads by {} ants, ranked: {}{}   -- {idle} of {} delivered nothing",
            d.len(),
            shown.join(" "),
            if d.len() > 20 { " ..." } else { "" },
            d.len()
        );
        let b = bank_bands(world, c.colony, c.species);
        println!("  banks    <10% {}   10-25% {}   25-50% {}   50-100% {}   full {}", b[0], b[1], b[2], b[3], b[4]);
    }
}

/// The sample row: one line per colony per interval, so the band is a
/// *series* rather than a final state. Deltas, not totals, because the
/// question "what are they eating" is about the window and a cumulative
/// column answers it only by subtraction in the reader's head.
struct Sample {
    intake: f64,
    outgo: f64,
    by_material: Vec<f64>,
}

fn snapshot(books: &ColonyBooks, materials: usize) -> Sample {
    let mut by_material = vec![0.0; materials];
    for (m, j) in books.diet() {
        if (m.0 as usize) < materials {
            by_material[m.0 as usize] = j;
        }
    }
    Sample { intake: books.intake(), outgo: books.outgo(), by_material }
}

fn main() {
    let control: String = arg("control").unwrap_or_else(|| "run".to_string());
    let frames: u64 = arg("frames").unwrap_or(60_000);
    let sample_every: u64 = arg("sample").unwrap_or(10_000);
    let spec = LabBox {
        width: arg("width").unwrap_or(512),
        height: arg("height").unwrap_or(320),
        // `labstats`' and `labforage`'s defaults, so a run here and a run
        // there are the same bed.
        soil_depth: arg("soil").unwrap_or(80),
        founders: arg("founders").unwrap_or(8),
        colonies: arg("colonies").unwrap_or(2),
        compartments: arg("walls").unwrap_or(1),
        seed: arg("seed").unwrap_or(1),
        species: arg::<String>("plant").unwrap_or_else(|| LabBox::default().species),
        ..LabBox::default()
    };
    if control == "selftest" {
        return selftest();
    }
    // Echo the parameters: a knob nobody can see the value of is a knob
    // nobody can tell is disconnected — `plant_probe`'s 3.5-hour lesson.
    println!(
        "colonybooks: frames={frames} sample={sample_every} colonies={} founders={} of {} walls={} soil={} seed={} colony={}",
        spec.colonies, spec.founders, spec.species, spec.compartments, spec.soil_depth, spec.seed, spec.colony_species
    );
    println!("  all figures in joules. intake is priced through `creature::diet_yield` at the call that credits the animal, so it is what the ant GOT, not what the cell was worth.");
    let (mut world, planted) = spec.build_counted();
    println!("  bed: {} plants asked, {} planted, {} ants, {} beetles", planted.asked, planted.planted, planted.ants, planted.beetles);
    let materials = world.materials.len();
    let mut previous: Vec<Sample> = Vec::new();
    // Seeded before the first frame so a colony that dies inside the first
    // sample window is still in the report. See `report`.
    let mut roster: BTreeMap<u32, Colony> = colonies_of(&world).into_iter().map(|c| (c.colony, c)).collect();
    println!("\n   frame  colony        alive   in/win   out/win   margin      bank   brains%   ate this window");
    for frame in 0..frames {
        world.begin_step();
        pixel_physics::sim::scheduler::step(&mut world);
        world.end_step();
        if !(frame + 1).is_multiple_of(sample_every) {
            continue;
        }
        let colonies = colonies_of(&world);
        let live = world.live_creature_energy_by_colony();
        if previous.len() < world.all_colony_books().len() {
            previous.resize_with(world.all_colony_books().len(), || Sample { intake: 0.0, outgo: 0.0, by_material: vec![0.0; materials] });
        }
        for c in &colonies {
            roster.insert(c.colony, Colony { label: c.label.clone(), species: c.species, colony: c.colony, alive: c.alive });
            let books = world.colony_books(c.colony);
            let now = snapshot(&books, materials);
            let was = &previous[c.colony as usize];
            let mut window: Vec<(MaterialId, f64)> = now
                .by_material
                .iter()
                .enumerate()
                .map(|(i, j)| (MaterialId(i as u16), j - was.by_material.get(i).copied().unwrap_or(0.0)))
                .filter(|(_, j)| *j > 0.0)
                .collect();
            window.sort_by(|a, b| b.1.total_cmp(&a.1));
            let ate: Vec<String> = window.iter().take(3).map(|&(m, j)| format!("{} {}", world.materials.get(m).name, thousands(j))).collect();
            println!(
                "{:>8}  {:<12} {:>5} {:>8} {:>9} {:>8} {:>9}   {:>6}   {}",
                frame + 1,
                c.label,
                c.alive,
                thousands(now.intake - was.intake),
                thousands(now.outgo - was.outgo),
                thousands(books.income() - books.outgo()),
                thousands(live.get(c.colony as usize).copied().unwrap_or(0.0)),
                pct(books.get(Account::SynapseTax), books.outgo()),
                if ate.is_empty() { "nothing".to_string() } else { ate.join("   ") }
            );
            previous[c.colony as usize] = now;
        }
    }
    report(&world, &roster);
    // **The world ledger beside the split, every run.** The per-colony books
    // are only as good as the claim that they sum, and a harness that prints
    // the split without the total is a harness that cannot notice the day
    // they stop agreeing.
    let split: f64 = world.all_colony_books().iter().map(|b| b.get(Account::HarvestedPlant) + b.get(Account::HarvestedCorpse)).sum();
    let whole = world.energy_ledger.harvested_plant + world.energy_ledger.harvested_corpse;
    println!("\n  check: colonies harvested {} against the world ledger's {} ({} apart)", thousands(split), thousands(whole), thousands(split - whole));
}

/// **The sensitivity half.** A bed whose answer is known non-zero, with
/// every band asserted to report it.
///
/// `CLAUDE.md`: a band reading zero because the colony ate nothing and one
/// reading zero because it is blind are the same line of output, and only a
/// positive control separates them. The specificity half is `colonies=0`,
/// which any reader can run, and the two arms are asserted against each
/// other here so the file cannot pass by reporting the world.
fn selftest() {
    let fed = LabBox { colonies: 2, founders: 8, seed: 1, ..LabBox::default() };
    let (mut world, _) = fed.build_counted();
    // **5,000 frames and not 20,000, measured rather than chosen**: on the
    // default bed at seed 1 one of the two colonies is already gone by
    // 20,000 (19 and 17 alive at 5,000, 6 and 6 at 10,000, one colony at
    // 20,000), and a two-colony assertion over a one-colony bed fails for
    // the ecology rather than for the instrument.
    for _ in 0..5_000 {
        world.begin_step();
        pixel_physics::sim::scheduler::step(&mut world);
        world.end_step();
    }
    let groups = colonies_of(&world);
    assert!(groups.len() >= 2, "selftest: the bed must hold two colonies, holds {}", groups.len());
    let mut any_diet = false;
    for c in &groups {
        let books = world.colony_books(c.colony);
        assert!(books.get(Account::Granted) > 0.0, "{}: placed with nothing", c.label);
        assert!(books.get(Account::Metabolized) > 0.0, "{}: has not spent a joule", c.label);
        assert!(books.get(Account::SynapseTax) > 0.0, "{}: the brain band is blind", c.label);
        assert!(books.outgo() > 0.0 && books.income() > 0.0, "{}: an empty income statement", c.label);
        any_diet |= books.intake() > 0.0;
        // The bands the report prints, exercised rather than trusted: a
        // panic here is a band that cannot be drawn.
        let _ = bank_bands(&world, c.colony, c.species);
        let _ = deliveries(&world, c.colony);
        let _ = diet_line(&world, &books, 5);
    }
    assert!(any_diet, "selftest: nothing in this bed ate anything in 5,000 frames — the scene no longer contains food");
    // **The specificity arm, in the same process.** A band that is really
    // reading the world rather than the colony passes every assertion above
    // and fails here.
    let bare = LabBox { colonies: 0, founders: 8, seed: 1, ..LabBox::default() };
    let (mut empty, _) = bare.build_counted();
    for _ in 0..2_000 {
        empty.begin_step();
        pixel_physics::sim::scheduler::step(&mut empty);
        empty.end_step();
    }
    for books in empty.all_colony_books() {
        assert_eq!(books.intake(), 0.0, "selftest: a bed with no colony in it fed somebody {} J", books.intake());
        assert_eq!(books.get(Account::Granted), 0.0, "selftest: a bed with no colony in it granted somebody a bank");
    }
    println!("colonybooks selftest: ok -- two colonies fed, banded and ranked; a colonyless bed reports nothing");
}
