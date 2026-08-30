//! Prints what a creature is actually sensing and deciding, per tick.
//!
//! **The companion every debug channel needs.** `render.rs`'s overlays and
//! `filmstrip`'s `channel=pheromone_a|pheromone_b` answer *what and where*;
//! they cannot answer *how much*, and this project has already paid twice
//! for reaching at an image with a quantitative question. A corrected
//! overlay was misread as "everything at the ramp floor" when the real
//! value was 40% of scale, because a one-cell-wide signal is genuinely
//! unjudgeable by eye (`CLAUDE.md`). An ant is one or two cells.
//!
//! So: when the question turns numeric — *was input 2 actually 0.03? is
//! the turn output doing anything? how many synapses is this brain paying
//! for?* — this is the tool, and `examples/plant_probe.rs` is the
//! precedent it follows.
//!
//! ```text
//! cargo run --release --example creature_probe
//! cargo run --release --example creature_probe -- frames=4000 every=500 ants=8
//! ```

use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material;
use pixel_physics::sim::pheromone::Channel;
use pixel_physics::sim::{parallel, Cell, World};

// **The names come from `brain.rs` rather than being restated here.** This
// file used to carry its own abbreviated copy, which is one enum edit away
// from a probe that prints the wrong label against the right number -- the
// exact failure mode `CLAUDE.md` records for debug readouts, and harder to
// spot than a blank overlay because it looks like data.
use pixel_physics::sim::brain::{INPUT_NAMES, OUTPUT_NAMES};

fn main() {
    let mut frames = 6000usize;
    let mut every = 1000usize;
    let mut ants = 6usize;
    // **The hand-built floor is a stone slab with no water anywhere in it**,
    // so every field channel reads exactly 0 for every ant in it — which is
    // a statement about this scene and not about the game. `terrain=world`
    // generates the shipped world instead, the same seam
    // `ant_ablation` uses and for the same stated reason: a result that
    // only holds on the hand-built profile is a result about the profile.
    // It matters here specifically because the moisture pre-flight's whole
    // question is what a sensor reads with air on one side and *ground* on
    // the other, and the slab has no wet ground to be one side of.
    let mut world_terrain = false;
    let mut seed = 0xA17u64;
    // **The four numbers S6's reachability question turns on**, overridable
    // in-process rather than by editing `ant.ron`, because assets are
    // `include_str!`ed and a sweep that edits one and re-runs a prebuilt
    // binary produces bit-identical "runs" — the gotcha that has already
    // produced whole invalid sweeps here. `-1` means "leave the species
    // file alone", so the default run measures the shipped animal.
    let mut start_energy = -1.0f32;
    let mut body_energy = -1.0f32;
    let mut threshold = -1.0f32;
    let mut hunger = -1.0f32;
    let mut mutation_rate = -1.0f32;
    // The ancestral `TRAIT_BIRTH_GRANT` allele, taken as a **fraction of
    // `start_energy`** rather than as its `-1..=1` axis position, because
    // the fraction is the quantity the arithmetic is written in. Negative
    // means "leave the species file alone".
    let mut grant = -1.0f32;
    // **Chain length, and the reason it is a knob rather than a species
    // file.** Extent is the only measured lever on whether a creature can
    // be found in the picture (`creature-appearance-design.md` §2: decoys
    // fall 127 -> 15 -> 0 across 2, 9 and 16 cells), and since 2026-08-30 it
    // is also the thing `idle_cost_per_cell` and `move_cost_per_cell` are
    // charged against. Nothing could vary it in-process before, so the
    // economics of a bigger body had never been measured at all -- the
    // `ant_long` species file exists but is `include_str!`ed, which is the
    // gotcha that has produced whole invalid sweeps here. 0 means "leave
    // the species file alone".
    let mut body = 0usize;
    // **The two prices, per cell, so `body=` can be run against the bill it
    // used to pay.** Without these a `body=6` run confounds two changes at
    // once -- the body got bigger *and* it started paying for itself -- and
    // `CLAUDE.md`'s rule about a sweep whose settings all fail the same way
    // is precisely that a rider travelling with the mechanism is part of
    // every data point. Setting `idle=0.016667` at `body=6` reproduces the
    // pre-2026-08-30 total of 0.10 per tick, which is the control.
    // Negative means "leave the species file alone".
    let mut idle = -1.0f32;
    let mut mv = -1.0f32;
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "frames" => frames = v.parse().expect("frames"),
            "every" => every = v.parse().expect("every"),
            "ants" => ants = v.parse().expect("ants"),
            "terrain" => world_terrain = v == "world",
            "seed" => seed = v.parse().expect("seed"),
            "start_energy" => start_energy = v.parse().expect("start_energy"),
            "body_energy" => body_energy = v.parse().expect("body_energy"),
            "threshold" => threshold = v.parse().expect("threshold"),
            "hunger" => hunger = v.parse().expect("hunger"),
            "mutation_rate" => mutation_rate = v.parse().expect("mutation_rate"),
            "grant" => grant = v.parse().expect("grant"),
            "body" => body = v.parse().expect("body"),
            "idle" => idle = v.parse().expect("idle"),
            "move" => mv = v.parse().expect("move"),
            other => panic!(
                "unknown arg {other:?}; known: frames, every, ants, terrain, seed, start_energy, body_energy, threshold, hunger, mutation_rate, grant, body, idle, move"
            ),
        }
    }

    let (w, h) = if world_terrain { (512i32, 320i32) } else { (320i32, 120i32) };
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    world.seed = seed;
    let floor = h - 8;
    let nest = world.materials.id_of("nest").expect("nest is compiled in");
    let corpse = world.materials.id_of("corpse").expect("corpse is compiled in");
    let ant = world.materials.id_of("ant").expect("ant is compiled in");

    // **The overrides, applied through `set_creature`** — the same seam
    // `creature_space` cuts `START_ENERGY` with, and the same reason: a
    // knob that has to rebuild between points cannot hold everything else
    // fixed within one process.
    //
    // `body_energy` moves the ant *material*'s `food_energy` with it. That
    // equality is deliberate and is `ant.ron`'s own: a bitten live ant must
    // be worth to the biter exactly what its stamp took out of the world,
    // or scavenging becomes an energy pump. It is no longer *load-bearing*
    // for the ledger — S6 booked the living-flesh stamp to `meat_lost` —
    // but it is still the honest price, so the harness holds it rather
    // than opening a pump it would then measure.
    let species = world.species.id_of("ant").expect("ant species");
    {
        let mut def = world.species.get(species).creature.clone().expect("ant is a creature");
        if start_energy >= 0.0 {
            def.start_energy = start_energy;
        }
        if body_energy >= 0.0 {
            def.body_energy = body_energy;
            for name in ["ant", "corpse"] {
                if let Some(id) = world.materials.id_of(name) {
                    world.materials.get_mut(id).food_energy = body_energy;
                }
            }
        }
        if threshold >= 0.0 {
            def.reproduce_threshold = threshold;
        }
        if grant >= 0.0 {
            def.traits[pixel_physics::sim::organism::TRAIT_BIRTH_GRANT] = grant * 2.0 - 1.0;
        }
        if hunger >= 0.0 {
            def.hunger_fraction = hunger;
        }
        if mutation_rate >= 0.0 {
            def.mutation_rate = mutation_rate;
        }
        if body > 0 {
            def.body = pixel_physics::sim::organism::BodyPlan::Chain(body as u8);
        }
        if idle >= 0.0 {
            def.idle_cost_per_cell = idle;
        }
        if mv >= 0.0 {
            def.move_cost_per_cell = mv;
        }
        world.species.set_creature(species, def);
    }
    let def = world.species.get(species).creature.clone().expect("ant is a creature");
    // Echo every parameter, so a log that does not name its terrain or its
    // prices was written by a binary that never had the knob (`CLAUDE.md`:
    // a knob nobody can see the value of is a knob nobody can tell is
    // disconnected). The two derived numbers are printed beside them
    // because they are what the reachability question is actually about.
    // **The best mouthful *this gut* can actually digest, not the fattest
    // number in the material table.** The first version of this line used
    // `body_energy` and overstated the ceiling by 6x on the shipped ant:
    // S5's matched filter pays `worth * (1 - |gut - class|/2)^2`, so the
    // neutral gut draws 120 from a 480 leaf, and a readout quoting 480
    // would have said REACHABLE where the animal measurably banks 568
    // against a 1,860 bar. A debug readout that is a function of the wrong
    // quantity is the failure this project has paid for twice; the
    // authority is the measured `richest bank` below, and this is the
    // arithmetic that has to agree with it -- and **measured 2026-08-29, it
    // does not**: 616 banked against 540 printed. The two channels it misses
    // are named at the readout further down; the authority is still the
    // measurement, which is the point.
    let gut = def.traits[pixel_physics::sim::organism::TRAIT_GUT_BIAS];
    let best_mouthful = (0..world.materials.len())
        .map(|i| pixel_physics::sim::material::MaterialId(i as u16))
        .map(|id| {
            // A corpse carries its worth per cell in `aux`; everything else
            // reads its material's face value, so the probe cell has to
            // carry a stamp or the whole carrion half reads as worthless.
            let aux = if world.materials.get(id).worth_in_aux { def.body_energy.round().clamp(0.0, 65535.0) as u16 } else { 0 };
            pixel_physics::sim::creature::diet_yield(&world, Cell::new(id, 0).with_aux(aux), gut)
        })
        .fold(0.0f32, f32::max);
    let ceiling = def.hunger_fraction * def.start_energy + best_mouthful;
    println!(
        "creature probe: {frames} frames, reporting {ants} ants every {every} | terrain={} seed={seed:#x} body={} cells",
        if world_terrain { "world" } else { "slab" },
        def.body.len()
    );
    println!(
        "  metabolism: idle {:.4}/cell x {} cells = {:.4} per tick, move {:.4}/cell = {:.4} per step",
        def.idle_cost_per_cell,
        def.body.len(),
        def.idle_cost_per_cell * def.body.len() as f32,
        def.move_cost_per_cell,
        def.move_cost_per_cell * def.body.len() as f32
    );
    println!(
        "  economy: start_energy {:.0} body_energy {:.0} hunger_fraction {:.2} reproduce_threshold {:.0} mutation_rate {:.3} birth_grant {:.2} (= {:.0} energy)",
        def.start_energy,
        def.body_energy,
        def.hunger_fraction,
        def.reproduce_threshold,
        def.mutation_rate,
        pixel_physics::sim::creature::grant_fraction(def.traits[pixel_physics::sim::organism::TRAIT_BIRTH_GRANT]),
        pixel_physics::sim::creature::birth_grant(&def, &def.traits)
    );
    println!(
        "  reachability: birth costs {:.0}, and an ant banks at most about {:.0} (hunger_fraction * start_energy + best digestible mouthful {best_mouthful:.0}) -- {}",
        pixel_physics::sim::creature::birth_cost(&def),
        ceiling,
        // **This can rule out and it can never rule in**, and the wording
        // has to say so. `best_mouthful` is the best cell in the whole
        // material table, not the best one an ant in this world can
        // actually reach -- 360 comes off a flower, which no ant here has
        // ever touched. So a ceiling over the bar is only the absence of a
        // proof: measured, arms at 495 against a 330 bar still produced
        // zero births, because the reachable food is worth a third of the
        // best food. Reading a bound as a verdict is the size-cap failure
        // in a readout's clothing -- exhausting it must not produce an
        // answer.
        //
        // **And the other direction is not a proof either, which this line
        // claimed until 2026-08-29.** It said "UNREACHABLE, and this is a
        // proof". Measured on the S6 gate control (`start_energy=200
        // body_energy=20 threshold=241 hunger=0.9 terrain=world
        // frames=24000`): printed ceiling **540**, measured `richest bank`
        // **616** -- the bound exceeded by 14%, and again at **561** with
        // `mutation_rate=0`, so it is not a one-off of a lucky lineage.
        // Two channels widen it past this arithmetic, and neither is in the
        // sum above:
        //
        // - **The gut it prices is the founder's, not the eater's.** This
        //   reads `def.traits[TRAIT_GUT_BIAS]` -- a species constant --
        //   while `creature.rs:1583` digests with the *organism's* own
        //   `s.traits[TRAIT_GUT_BIAS]`, which is heritable and mutates by
        //   `trait_variance`. A matched gut pays `worth` where the neutral
        //   founder pays `worth/4`, so 18 generations of selection eat food
        //   this line cannot price. Turning mutation off removed 55 of the
        //   76 excess, which is the measurement that names this channel.
        // - **A corpse is worth more than its stamp.** `creature.rs:3028`
        //   writes `(body_energy * cells + leftover) / cells` -- the dead
        //   animal's unspent bank rides into the meat -- where the probe
        //   cell below is stamped with `body_energy` alone.
        //
        // The shipped ant's conclusion survives this comfortably (bank 567
        // against a 1,860 bar, a 3.3x margin, and nothing dies in that
        // scene), which is *why* the wording is the thing being fixed
        // rather than the arithmetic: the number is still the most useful
        // one to print, it just does not license the word "proof".
        if ceiling > pixel_physics::sim::creature::birth_cost(&def) {
            "the bar is not ruled out by this bound (which is optimistic -- read `richest bank` below)"
        } else {
            "the bar is above this bound -- strong evidence of unreachability, but the bound is not a ceiling (measured 616 against a printed 540; see the note above) -- read `richest bank` below"
        }
    );

    if world_terrain {
        let (presets, _) = pixel_physics::worldgen::WorldgenPresets::load();
        let name = presets.default_name();
        let params = presets.get(&name).expect("the default preset exists");
        pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed });
        // Let the field settle before anything is sensed: a world read on
        // frame 0 is a world whose moisture has never been stepped, which
        // would answer the pre-flight's question about the initial
        // condition rather than about the world an ant lives in.
        for _ in 0..600 {
            parallel::step(&mut world);
            world.step_active_sites();
            world.step_fields();
        }
        // Ground, not "anything solid": a seed is a `Powder` and a blade a
        // `Solid`, so a naive scan puts ants into the vegetation.
        let surface_of = |world: &World, x: i32| -> i32 {
            (0..h)
                .find(|&y| {
                    world.get(x, y).organism_id() == 0
                        && matches!(world.materials.kind(world.get(x, y).material), material::MaterialKind::Solid | material::MaterialKind::Powder)
                })
                .unwrap_or(h - 1)
        };
        for x in 96..160 {
            let sy = surface_of(&world, x);
            world.set(x, sy, Cell::new(nest, 0).with_attached(true));
        }
        for i in 0..55 {
            let ax = 100 + i * 2;
            let sy = surface_of(&world, ax);
            world.plant_ant(ax, sy - 1);
        }
    } else {
        for x in 0..w {
            for y in floor..h {
                world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        for x in 16..48 {
            world.set(x, floor, Cell::new(nest, 0).with_attached(true));
        }
        for x in 16..120 {
            world.set(x, floor, Cell::new(nest, 0).with_attached(true));
        }
        for x in 135..170 {
            for y in (floor - 5)..floor {
                world.set(x, y, Cell::new(corpse, 0));
            }
        }
        for i in 0..55 {
            world.plant_ant(20 + i * 2, floor - 1);
        }
    }

    // **Frame cost, because §2.6 asks for it and the inherited number does
    // not transfer.** Creature work was measured free at 55 ants against a
    // 0-ant control; a breeding population is not 55, and the whole point
    // of reproduction is that nobody sets the population any more. Timed
    // over the same four phases `App::update` runs so this is a whole-frame
    // figure rather than a subsystem one — an isolated harness overstates,
    // and the phase a change is made cheaper *against* is the whole frame.
    //
    // Read the mean, and read the worst only against the ratio: `mean *
    // frames ~= worst` is what says an aggregate pins it. Here it does not
    // (thousands of comparable frames), so the worst is an order statistic
    // over a noisy box and the mean is the number to quote.
    let mut worst = 0.0f64;
    let mut total = 0.0f64;
    let mut peak_live = 0usize;
    for frame in 0..frames {
        let t0 = std::time::Instant::now();
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        total += ms;
        worst = worst.max(ms);
        if frame % 200 == 0 {
            peak_live = peak_live.max(world.live_creature_count());
        }
        if frame % every == 0 {
            report(&world, frame, ants, ant, w, h);
            sensor_census(&world, frame, ant, w, h);
        }
    }
    report(&world, frames, ants, ant, w, h);
    sensor_census(&world, frames, ant, w, h);

    let st = world.creature_stats;
    println!(
        "\ntotals: moves {} blocked {} falls {} | eats {} pickups {} digs {} drops {} deliveries {} nest-visits {} deaths {}",
        st.moves, st.moves_blocked, st.falls, st.eats, st.pickups, st.digs, st.drops, st.deliveries, st.nest_visits, st.deaths
    );
    // **The reproduction readout.** `births` alone cannot say why a run
    // produced none: a colony too poor to reach the bar and a birth path
    // that never fires read identically. `richest bank` is what separates
    // them, and the lineage columns are what say whether anything is left
    // to select on -- a population at one lineage has converged whatever
    // its genomes look like.
    let mut lineages: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    let mut deepest = 0u16;
    let mut richest = 0.0f32;
    let mut live = 0usize;
    for x in 0..w {
        for y in 0..h {
            let cell = world.get(x, y);
            if pixel_physics::sim::organism::cell_type(cell.aux()) != Some(pixel_physics::sim::organism::CellType::Head) {
                continue;
            }
            let Some(state) = world.organism(cell.organism_id()) else { continue };
            if world.species.get(state.species).creature.is_none() {
                continue;
            }
            live += 1;
            deepest = deepest.max(state.generation);
            richest = richest.max(state.energy);
            *lineages.entry(state.lineage).or_default() += 1;
        }
    }
    // **Share, not just count.** The count says how many lines are left;
    // the share of the biggest is what the clonal-drift band is a
    // distribution *of*, and the two come apart badly — 45 lines surviving
    // says nothing if one of them is 90% of the animals.
    let top_share = lineages.values().copied().max().unwrap_or(0) as f32 / live.max(1) as f32;
    println!(
        "frame cost: mean {:.3} ms, worst {:.3} ms over {frames} frames | peak creature population {}",
        total / frames as f64,
        worst,
        peak_live.max(live)
    );
    println!(
        "reproduction: births {} denied-no-space {} refused-no-slot {} | live {live} deepest generation {deepest} | lineages {} top share {top_share:.3} | richest bank {richest:.0} against a birth cost of {:.0}",
        st.births,
        st.births_denied_no_space,
        world.organisms_refused(),
        lineages.len(),
        pixel_physics::sim::creature::birth_cost(&def)
    );
    println!(
        "energy census: live {:.2} vs ledger {:.2} (delta {:.4})",
        world.live_creature_energy(),
        world.energy_ledger.expected_live_total(),
        world.live_creature_energy() - world.energy_ledger.expected_live_total()
    );
}

fn report(world: &World, frame: usize, ants: usize, ant_material: material::MaterialId, w: i32, h: i32) {
    println!("\n--- frame {frame} ---");
    // Head cells, found by scanning: the probe deliberately does not hold
    // handles across frames, because an ant that dies and has its slot
    // reused would otherwise be silently reported as the same individual.
    let heads: Vec<(i32, i32)> = (0..w)
        .flat_map(|x| (0..h).map(move |y| (x, y)))
        .filter(|&(x, y)| {
            let c = world.get(x, y);
            c.material == ant_material
                && pixel_physics::sim::organism::cell_type(c.aux()) == Some(pixel_physics::sim::organism::CellType::Head)
        })
        .collect();
    // **Carriers first.** The interesting ant is almost never a random one:
    // a colony with pickups and no deliveries has one question, and it is
    // "what does a laden ant think it should do".
    let mut heads = heads;
    heads.sort_by_key(|&(x, y)| {
        let carrying = world.organism(world.get(x, y).organism_id()).is_some_and(|s| s.carrying.is_some());
        (!carrying, x)
    });
    let heads: Vec<(i32, i32)> = heads.into_iter().take(ants).collect();

    for (x, y) in heads {
        let organism = world.get(x, y).organism_id();
        let Some(state) = world.organism(organism) else { continue };
        let Some(def) = world.species.get(state.species).creature.as_ref() else { continue };
        let (inputs, outputs, active) = pixel_physics::sim::creature::probe(world, x, y, organism, def);
        println!(
            "  ant {organism:>5} at ({x:>3},{y:>3}) heading {} energy {:>7.1} carrying {} since_nest {:>4} | {} active synapses",
            state.heading,
            state.energy,
            state.carrying.map_or("-", |_| "yes"),
            state.since_nest,
            active
        );
        let ins: Vec<String> = INPUT_NAMES.iter().zip(&inputs).map(|(n, v)| format!("{n} {v:+.3}")).collect();
        let outs: Vec<String> = OUTPUT_NAMES.iter().zip(&outputs).map(|(n, v)| format!("{n} {v:+.3}")).collect();
        println!("    in : {}", ins.join("  "));
        println!("    out: {}", outs.join("  "));
        println!(
            "    planes here: A {} B {}",
            world.pheromone_at(Channel::A, x, y),
            world.pheromone_at(Channel::B, x, y)
        );
        // **The number beside the picture.** `OrganismOverlay::FoodValue`
        // says what and where; on a one-cell mouthful it cannot say how
        // much, and the overlay-misread-as-empty failure has cost this
        // project a misdiagnosis twice. Anything nonzero here is food, and
        // a corpse reads its own stamped worth rather than a species
        // constant.
        let food: Vec<String> = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)]
            .iter()
            .map(|&(dx, dy)| (x + dx, y + dy))
            .map(|(nx, ny)| (world.materials.get(world.get(nx, ny).material).name.clone(), pixel_physics::sim::creature::food_value(world, world.get(nx, ny))))
            .filter(|(_, v)| *v > 0.0)
            .map(|(name, v)| format!("{name} {v:.0}"))
            .collect();
        println!("    food in reach: {}", if food.is_empty() { "none".to_string() } else { food.join("  ") });
    }
}

/// **The population's standing sensor distribution, one row per input.**
///
/// The per-ant dump above answers *what is this animal sensing right now*;
/// it cannot answer *does this channel carry a systematic constant across
/// the whole colony*, which is the question S6's first pre-flight asks
/// (`Reports/creature-evolution-plan.md` §2.6: on a surface the lateral
/// moisture pair samples one point in air and one in ground, so it likely
/// reads a large fixed offset — and **evolution finds and exploits any
/// constant**, so a genome wiring it to `Turn` gets a persistent turn bias
/// that looks like a strategy and is an instrument artifact).
///
/// A mean alone cannot tell a constant from a symmetric signal, so the sd
/// and the range print beside it: a spurious constant is `|mean|` large
/// with `sd` near zero, a live signal has `sd` comparable to `|mean|`, and
/// a dead channel is both at zero. `|mean|/sd` is printed as `bias` for
/// exactly that comparison, because reading it off two columns by eye is
/// how the canopy-density sheet got misread.
///
/// **Both controls ship in the same table, which is why every row prints
/// rather than the one under suspicion** (`CLAUDE.md`: ask what a number
/// says when nothing is wrong, *and* check that it can move). `Bias` is
/// wired to exactly 1.0 in `creature::sense`, so it must read
/// `mean +1.0000 sd 0.0000` — if it does not, the aggregation is wrong and
/// no other row means anything. The pheromone channels are the positive
/// control from the other side: a foraging colony demonstrably moves them,
/// so an sd of 0 there would say the sampler never caught a live ant
/// rather than that the channel is flat.
fn sensor_census(world: &World, frame: usize, ant_material: material::MaterialId, w: i32, h: i32) {
    let mut n = 0u64;
    let mut sum = [0.0f64; INPUT_NAMES.len()];
    let mut sq = [0.0f64; INPUT_NAMES.len()];
    let mut lo = [f32::INFINITY; INPUT_NAMES.len()];
    let mut hi = [f32::NEG_INFINITY; INPUT_NAMES.len()];
    for x in 0..w {
        for y in 0..h {
            let c = world.get(x, y);
            if c.material != ant_material
                || pixel_physics::sim::organism::cell_type(c.aux()) != Some(pixel_physics::sim::organism::CellType::Head)
            {
                continue;
            }
            let organism = c.organism_id();
            let Some(state) = world.organism(organism) else { continue };
            let Some(def) = world.species.get(state.species).creature.as_ref() else { continue };
            let (inputs, _, _) = pixel_physics::sim::creature::probe(world, x, y, organism, def);
            n += 1;
            for (i, &v) in inputs.iter().enumerate() {
                sum[i] += v as f64;
                sq[i] += (v as f64) * (v as f64);
                lo[i] = lo[i].min(v);
                hi[i] = hi[i].max(v);
            }
        }
    }
    println!("\n--- sensor census, frame {frame}: {n} heads ---");
    if n == 0 {
        println!("  (no live heads; nothing to census)");
        return;
    }
    for (i, name) in INPUT_NAMES.iter().enumerate() {
        let mean = sum[i] / n as f64;
        let sd = (sq[i] / n as f64 - mean * mean).max(0.0).sqrt();
        // `bias` is |mean|/sd -- large means the channel is mostly a
        // constant offset, which is the quantity the pre-flight is after.
        // Infinite is the honest answer for a channel with no spread at
        // all (Bias itself), so it is printed rather than clamped away.
        let ratio = if sd > 0.0 { format!("{:>7.2}", mean.abs() / sd) } else { "    inf".to_string() };
        println!("  {name:<14} mean {mean:+.4}  sd {sd:.4}  bias {ratio}  min {:+.3}  max {:+.3}", lo[i], hi[i]);
    }
}
