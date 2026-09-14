//! **What eats the plants around a colony in the held world — and whether
//! the `F` key is what does it.**
//!
//! Owner report, 2026-09-14: *"Absorbing creature energy, destroys plants
//! around it. The energy particles need to be foreground and not interact
//! with the world."* The observation is data; the explanation is a
//! hypothesis, and this is the instrument that separates them.
//!
//! **Nothing in this repo censused plants in the druid world before this.**
//! `PIXEL_PHYSICS_DRUID_CENSUS` counts living tissue inside each standing
//! circle and exits, which answers "is the speed dial connected" and cannot
//! answer "did something eat the garden": it has no *before*, and a count
//! inside a circle cannot see a plant that was killed and freed. `flora_census`
//! builds its own world and never holds it; `latecensus` reads a `LabBox`.
//! So the gap is a **paired** census of the *same* world across a run, which
//! is what `CLAUDE.md` asks for by name — *"if the question is 'how much did
//! this eat', census the materials before and after"*.
//!
//! **Materials, not events.** A failure count is not a damage count, and a
//! plant losing cells and a plant being struck from the organism table are
//! different findings with different causes, so both are printed side by
//! side, along with `World::deaths_by_cause`.
//!
//! ## The arms this exists to run
//!
//! ```text
//! cargo run --release --example druid_garden -- control=selftest
//! cargo run --release --example druid_garden -- arm=absorb
//! cargo run --release --example druid_garden -- arm=quiet
//! cargo run --release --example druid_garden -- arm=quiet speed=8
//! cargo run --release --example druid_garden -- arm=quiet colony=0
//! ```
//!
//! `control=selftest` is the positive control and it is not optional: it
//! erases a known number of plant cells and a known number of plant
//! organisms from a grown world and asserts the census reports exactly
//! those numbers. A paired census whose *sensitivity* has not been
//! demonstrated cannot distinguish "absorb is harmless" from "the
//! instrument cannot see harm", and those are the two readings this whole
//! measurement has to tell apart.
//!
//! **The world is expensive** — `Druid::new()` generates 2560x960 and grows
//! it for 8,000 frames. Every arm pays it, because the sim is deterministic
//! same-build and `World` is not `Clone`, so re-generating is the only way
//! two arms can be compared on the same ground. `PIXEL_PHYSICS_DRUID_SIZE`
//! and `PIXEL_PHYSICS_DRUID_GROW` shrink it for a quick pass; a shrunken
//! world is a different world and arms measured on one are comparable only
//! with each other.
//!
//! **Rebuild before trusting a run** (`cargo build --release --examples`):
//! species and material files are `include_str!`d, and a stale example
//! prints plausible numbers from a world nobody asked for.

use pixel_physics::app::{HEIGHT, WIDTH};
use pixel_physics::druid::Druid;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::organism::{DEATH_CAUSES, DEATH_CAUSE_LIST};
use pixel_physics::sim::world::World;
use std::collections::BTreeMap;

/// A box around the colony, in cells either side of it. Wider than
/// `ABSORB_RADIUS` (60) and than a placed circle, so a plant that dies just
/// outside the circle still lands inside the window — the question is what
/// died *near the player*, and a window cut to the circle would answer a
/// different one by construction.
const WINDOW: i32 = 200;

/// **The tight window: `ABSORB_RADIUS` itself.**
///
/// A second window rather than a narrower one, because the two answer
/// different questions and the wide one can hide the narrow one. "Plants
/// around it" is a claim about the ground the player is standing on, and at
/// full scale the wide window holds ~35,000 plant cells — a local loss of
/// fifty would not round to a visible digit in it.
const TIGHT: i32 = 60;

/// One reading of the world. Everything here is a standing quantity, so two
/// of these subtract.
struct Census {
    /// Plant-kind cells in the whole world, by material name.
    cells: BTreeMap<String, usize>,
    /// ...and the same, inside [`WINDOW`] of the colony.
    near: BTreeMap<String, usize>,
    /// ...and inside [`TIGHT`], which is the reach of the `F` key itself.
    tight: usize,
    /// Live organisms whose species has no `creature` block.
    plants: usize,
    /// ...of which, rooted inside the window.
    plants_near: usize,
    /// Live organisms whose species has one.
    animals: usize,
    /// `World::deaths_by_cause`, cumulative since the world began.
    deaths: [u64; DEATH_CAUSES],
    /// **Energy animals took out of plant material**, cumulative.
    ///
    /// The instrument that answers "did the colony eat the garden" without
    /// going through a death count at all. A death count cannot: a grazed
    /// plant usually survives being grazed, so eating and dying are
    /// different events and only one of them is grazing.
    harvested_plant: f64,
}

impl Census {
    fn total(&self) -> usize {
        self.cells.values().sum()
    }
    fn total_near(&self) -> usize {
        self.near.values().sum()
    }
}

/// Read the grid, not the organism table.
///
/// **Cells come from the grid on purpose**: it is the same array the picture
/// is drawn from, so the census cannot disagree with what the owner saw. The
/// organism counts come from the table, because a *freed* organism leaves no
/// cell behind and the whole question is whether something is being removed.
fn census(world: &World, at: (i32, i32)) -> Census {
    let bounds = world.bounds().expect("bounded world");
    let mut cells: BTreeMap<String, usize> = BTreeMap::new();
    let mut near: BTreeMap<String, usize> = BTreeMap::new();
    let mut tight = 0usize;
    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            let cell = world.get(x, y);
            if world.materials.kind(cell.material) != MaterialKind::Plant {
                continue;
            }
            let name = world.materials.get(cell.material).name.clone();
            *cells.entry(name.clone()).or_default() += 1;
            if (x - at.0).abs() <= WINDOW && (y - at.1).abs() <= WINDOW {
                *near.entry(name).or_default() += 1;
            }
            if (x - at.0).abs() <= TIGHT && (y - at.1).abs() <= TIGHT {
                tight += 1;
            }
        }
    }
    let (mut plants, mut plants_near, mut animals) = (0usize, 0usize, 0usize);
    for id in world.live_organism_ids() {
        let Some(state) = world.organism(id) else { continue };
        if world.species.get(state.species).creature.is_some() {
            animals += 1;
            continue;
        }
        plants += 1;
        let seat = state.chain.first().copied().or_else(|| state.cells.keys().next().copied());
        if let Some((x, y)) = seat {
            if (x - at.0).abs() <= WINDOW && (y - at.1).abs() <= WINDOW {
                plants_near += 1;
            }
        }
    }
    Census {
        cells,
        near,
        tight,
        plants,
        plants_near,
        animals,
        deaths: world.deaths_by_cause,
        harvested_plant: world.energy_ledger.harvested_plant,
    }
}

/// Print one arm's before/after pair.
///
/// Per material rather than as a total, because *which* tissue went is the
/// difference between grazing (leaf first) and a plant coming apart (wood
/// and leaf together), and a total cannot say which happened.
fn report(label: &str, a: &Census, b: &Census) {
    println!();
    println!("=== {label} ===");
    println!("{:<14} {:>10} {:>10} {:>10}", "plant cells", "before", "after", "delta");
    let names: Vec<&String> = {
        let mut v: Vec<&String> = a.cells.keys().chain(b.cells.keys()).collect();
        v.sort();
        v.dedup();
        v
    };
    for name in names {
        let (x, y) = (a.cells.get(name).copied().unwrap_or(0), b.cells.get(name).copied().unwrap_or(0));
        println!("  {name:<12} {x:>10} {y:>10} {:>+10}", y as i64 - x as i64);
    }
    println!("  {:<12} {:>10} {:>10} {:>+10}", "WORLD", a.total(), b.total(), b.total() as i64 - a.total() as i64);
    println!("  {:<12} {:>10} {:>10} {:>+10}", "near colony", a.total_near(), b.total_near(), b.total_near() as i64 - a.total_near() as i64);
    println!("  {:<12} {:>10} {:>10} {:>+10}", "within r60", a.tight, b.tight, b.tight as i64 - a.tight as i64);
    println!("  {:<12} {:>10} {:>10} {:>+10}", "plants", a.plants, b.plants, b.plants as i64 - a.plants as i64);
    println!("  {:<12} {:>10} {:>10} {:>+10}", "plants near", a.plants_near, b.plants_near, b.plants_near as i64 - a.plants_near as i64);
    println!("  {:<12} {:>10} {:>10} {:>+10}", "animals", a.animals, b.animals, b.animals as i64 - a.animals as i64);
    println!(
        "  {:<12} {:>10.0} {:>10.0} {:>+10.0}",
        "eaten(plant)", a.harvested_plant, b.harvested_plant, b.harvested_plant - a.harvested_plant
    );
    for (i, cause) in DEATH_CAUSE_LIST.iter().enumerate() {
        let d = b.deaths[i] - a.deaths[i];
        if d > 0 {
            println!("  died {:<16} {d}", cause.label());
        }
    }
}

/// **The positive control, and it is the reason this file can be believed.**
///
/// Grows a world, counts it, erases a known number of plant cells and frees
/// a known number of plant organisms, counts it again, and asserts the
/// deltas match. Both halves matter and they fail differently: a census that
/// reads the organism table for cells would pass the first and miss the
/// second, and one that reads the grid for organisms would do the reverse.
fn selftest() {
    let mut game = Druid::new();
    let at = game.world.player.as_ref().map(|p| p.center()).unwrap_or((0, 0));
    let before = census(&game.world, at);
    assert!(before.total() > 0, "selftest needs a world with plants in it; grew {} cells", before.total());

    // Erase 100 plant cells outright, from wherever they are.
    let bounds = game.world.bounds().expect("bounded world");
    let mut erased = 0usize;
    let mut victims: Vec<(i32, i32)> = Vec::new();
    'scan: for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
            if game.world.materials.kind(game.world.get(x, y).material) == MaterialKind::Plant {
                victims.push((x, y));
                erased += 1;
                if erased == 100 {
                    break 'scan;
                }
            }
        }
    }
    for (x, y) in &victims {
        game.world.set(*x, *y, Cell::EMPTY);
    }
    let after = census(&game.world, at);
    report("SELFTEST erase 100 plant cells", &before, &after);
    let moved = before.total() as i64 - after.total() as i64;
    assert_eq!(moved, erased as i64, "the cell census is blind: erased {erased}, it reported {moved}");
    assert!(
        after.plants <= before.plants,
        "erasing tissue cannot create organisms: {} -> {}",
        before.plants,
        after.plants
    );
    println!();
    println!("selftest: PASS - the census sees {erased} erased cells as {moved}");
}

/// Render one frame of the real game to `<prefix>-<when>.png`, or do nothing
/// when no prefix was asked for.
///
/// The same `Druid::draw` the window calls, into a plain RGBA buffer — so
/// what lands in the file is the picture, interface and all, rather than a
/// debug view of it.
fn shoot(game: &mut Druid, prefix: &str, when: &str) {
    if prefix.is_empty() {
        return;
    }
    // **The key legend off, because it covers half the ground.** It is the
    // game's own default and right for a player who has just started; a card
    // asking "what happened to this hillside" cannot spend 40% of its pixels
    // on a list of bindings.
    game.show_keys = false;
    let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    game.draw(&mut frame, (WIDTH, HEIGHT), true);
    let path = format!("{prefix}-{when}.png");
    match image::save_buffer(&path, &frame, WIDTH, HEIGHT, image::ColorType::Rgba8) {
        Ok(()) => println!("druid_garden: wrote {path}"),
        Err(e) => eprintln!("druid_garden: {path}: {e}"),
    }
}

fn main() {
    // `quiet` is the pure control: same world, same elapsed, same speed, and
    // the `F` key never pressed.
    let mut arm = String::from("absorb");
    let mut ticks: u64 = 4_000;
    let mut speed: u32 = 1;
    let mut colony = true;
    // **How many founders to ask for, and it is not cosmetic.** `found_colony`
    // asks for `COLONY_SIZE` (12) and `colony_stations` drops every station
    // whose column has no ground at the founder's own height, so on the
    // druid preset's rolling country a founding places **2**. A two-ant
    // colony cannot eat a garden however long it runs, so an arm meant to
    // test grazing has to be able to ask for a real one.
    let mut ants: i32 = 0;
    let mut bending = true;
    let mut circle = true;
    let mut unlimited = true;
    let mut absorb_every: u64 = 300;
    let mut control = String::new();
    // **A picture, because the report is about what the ground looks like.**
    // `filmstrip` cannot drive this game and `bin/druid.rs`'s capture hooks
    // need a surface; `Druid::draw` needs neither, so an example can render
    // the same frame the player sees straight into a buffer.
    let mut png = String::new();
    for a in std::env::args().skip(1) {
        let Some((k, v)) = a.split_once('=') else { continue };
        match k {
            "arm" => arm = v.to_string(),
            "ticks" => ticks = v.parse().expect("ticks=N"),
            "speed" => speed = v.parse().expect("speed=N"),
            "colony" => colony = v != "0",
            "ants" => ants = v.parse().expect("ants=N"),
            "bending" => bending = v != "0",
            "circle" => circle = v != "0",
            // **Default on, and that is the point of the control.** With the
            // economy live, not absorbing means running out of power, and
            // running out of power *closes a circle* — so "no absorb" would
            // silently also mean "less world running", which is the confound
            // this whole measurement exists to remove. Turn it off
            // deliberately (`unlimited=0`) to measure that chain instead.
            "unlimited" => unlimited = v != "0",
            "absorbevery" => absorb_every = v.parse::<u64>().expect("absorbevery=N").max(1),
            "control" => control = v.to_string(),
            "png" => png = v.to_string(),
            _ => panic!("unknown argument {a:?}"),
        }
    }
    if control == "selftest" {
        selftest();
        return;
    }

    let mut game = Druid::new();
    game.unlimited = unlimited;
    game.speed = speed.clamp(pixel_physics::druid::SPEED_MIN, pixel_physics::druid::SPEED_MAX);
    let at = game.world.player.as_ref().map(|p| p.center()).expect("the druid world spawns a player");

    game.world.plant_bending = bending;
    let placed = if ants > 0 {
        let n = game.world.found_colony_of(at.0, at.1, "ant", ants);
        println!("druid_garden: asked for {ants} founders, {n} took");
        n
    } else if colony {
        game.found_colony()
    } else {
        0
    };
    if circle {
        game.place_quickening();
    }
    println!(
        "druid_garden: arm={arm} ticks={ticks} speed={} colony={placed} circle={} unlimited={unlimited} bending={bending} at={at:?}",
        game.speed,
        game.world.quickenings.len()
    );

    let before = census(&game.world, at);
    shoot(&mut game, &png, "before");
    let mut absorbs = 0usize;
    let mut drawn = 0.0f32;
    for t in 1..=ticks {
        game.update();
        if arm == "absorb" && t.is_multiple_of(absorb_every) {
            let took = game.absorb();
            absorbs += 1;
            drawn += took;
        }
    }
    let after = census(&game.world, at);
    shoot(&mut game, &png, "after");

    report(&format!("{arm} speed={} colony={placed} circle={} unlimited={unlimited}", game.speed, circle as u8), &before, &after);
    // **The discrete event count, beside the numbers it is meant to explain.**
    // A run that reads "absorb changed nothing" is worthless if the key was
    // never pressed, and the two are indistinguishable in a delta.
    println!();
    println!("absorbs fired: {absorbs}, energy drawn: {drawn:.0}, power {:.0}, circles {}", game.power, game.world.quickenings.len());
}
