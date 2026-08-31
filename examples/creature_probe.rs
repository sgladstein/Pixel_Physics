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
    // **§R3's isolating control: is living ant flesh what is eating the
    // colony?** `open-bugs-handoff.md` §R3 records that no `Chain` above two
    // cells leaves a survivor, that the per-cell bill and the terrain are
    // both ruled out by paired arms, and that 22 of 34 animals go missing
    // without the death counter moving. Its leading hypothesis is
    // cannibalism, on the strength of a frame-0 dump reading `food in reach:
    // ant 480` — the richest food in the world — and of a longer chain
    // having proportionally more contact surface with its neighbours.
    //
    // **That dump is `food_value` alone and never applies the kin gate**
    // (`report` above), so it cannot distinguish flesh that is edible from
    // flesh that merely *would* be if the eater were a beetle. This knob is
    // the control that can: `kinfood=off` sets the living-ant materials'
    // `food_energy` to zero, which takes them under `EAT_YIELD_THRESHOLD`
    // for every gut, so no ant can draw anything from another ant's body
    // however the kin gate behaves.
    //
    // **Living flesh only — `corpse` is deliberately left alone.** The
    // question is whether ants eat *each other*, not whether they scavenge,
    // and carrion is the food an ant colony is supposed to run on. Zeroing
    // both would confound the control with a famine.
    //
    // In-process rather than in `assets/species/*.ron` for two reasons: the
    // assets are `include_str!`ed, so an edited `.ron` against a prebuilt
    // binary produces bit-identical "runs" (the gotcha that has already
    // produced whole invalid sweeps here), and `beetle.ron` eats ants
    // *deliberately* — a committed change to ant flesh would silently
    // disarm the only predator in the world. Nothing in this scene is a
    // beetle, so the override cannot reach one.
    let mut kinfood = true;
    // **The positive control for the knob above, and it is not optional.**
    // `kinfood=off` answers "does the colony live once kin cannot be eaten".
    // A null from it — the colony dies anyway — is only worth something if
    // the instrument could have reported the other answer, and `CLAUDE.md`
    // is explicit that a null is exactly where a counter hides: *"run the
    // positive control... construct the case whose answer you know is
    // non-zero and check the instrument reports it."*
    //
    // `eatskin=on` flips `CreatureDef::eats_kin`, which is the gate
    // `adjacent_food` gives living kin. It makes cannibalism *actually
    // happen*, so the arms read:
    //
    //     eatskin=on    cannibalism forced on   — must look catastrophic
    //     (default)     shipped behaviour       — the arm under test
    //     kinfood=off   kin flesh worthless     — the isolating control
    //
    // If the first and the last do not differ, this probe cannot see
    // cannibalism at all and no null it reports means anything.
    let mut eats_kin = false;
    // **The spacing the harness lays its founders out on, and the reason
    // it is suddenly a variable.** Both scenes below plant 55 founders at
    // `x = base + i * 2` — a **two-cell pitch** — and a `Chain(n)` is laid
    // out as *n* cells running left from its head. So at `body=2` the
    // bodies tile the row exactly, and at `body=3` every consecutive pair
    // overlaps by a cell and the second of each pair has nowhere to go.
    //
    // That is arithmetic, not ecology: the pitch was calibrated against the
    // shipped two-cell ant and silently became a *body-size filter* the
    // moment `body=` existed. §R3 reads the resulting 55 -> 28 on the slab
    // as "the site predicate gets harder as *n* grows" and files it as a
    // property of the engine; it is a property of this scene's founder
    // loop, which is `CLAUDE.md`'s "a scene that contradicts the code will
    // look like a bug in the code".
    //
    // `pitch=` is what tells the two apart. Default 2 reproduces every
    // number already filed; `pitch=4` gives a three-cell body the same
    // clearance a two-cell one has always had.
    let mut pitch = 2usize;
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "frames" => frames = v.parse().expect("frames"),
            "every" => every = v.parse().expect("every"),
            "ants" => ants = v.parse().expect("ants"),
            "terrain" => world_terrain = v == "world",
            // **Hex as well as decimal, because this probe echoes hex.**
            // The parameter line prints `seed={seed:#x}`, so a report
            // quoting its own output writes `seed=0xA17` -- which
            // `str::parse::<u64>` rejects outright. `open-bugs-handoff.md`
            // §R3 does exactly that, and its headline reproduction command
            // panicked on the argument rather than running. Accepting the
            // form the harness prints is the fix; the alternative is a
            // readout that cannot be pasted back in.
            "seed" => {
                seed = v
                    .strip_prefix("0x")
                    .or_else(|| v.strip_prefix("0X"))
                    .map_or_else(|| v.parse::<u64>(), |hex| u64::from_str_radix(hex, 16))
                    .expect("seed: decimal, or hex with an 0x prefix")
            }
            "start_energy" => start_energy = v.parse().expect("start_energy"),
            "body_energy" => body_energy = v.parse().expect("body_energy"),
            "threshold" => threshold = v.parse().expect("threshold"),
            "hunger" => hunger = v.parse().expect("hunger"),
            "mutation_rate" => mutation_rate = v.parse().expect("mutation_rate"),
            "grant" => grant = v.parse().expect("grant"),
            "body" => body = v.parse().expect("body"),
            "idle" => idle = v.parse().expect("idle"),
            "move" => mv = v.parse().expect("move"),
            "kinfood" => kinfood = v != "off",
            "eatskin" => eats_kin = v == "on",
            "pitch" => pitch = v.parse::<usize>().expect("pitch").max(1),
            other => panic!(
                "unknown arg {other:?}; known: frames, every, ants, terrain, seed, start_energy, body_energy, threshold, hunger, mutation_rate, grant, body, idle, move, kinfood, eatskin, pitch"
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
            def.digest_rate = hunger;
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
        // The positive control. Left as the species file has it unless asked.
        if eats_kin {
            def.eats_kin = true;
        }
        world.species.set_creature(species, def);
        // **Every living-ant material, not just `ant`.** The chain bodies
        // (`ant_long`, `ant_wide`, `ant_block`) and the two chitins are the
        // same living flesh wearing a different palette, and a control that
        // zeroed only `ant` would leave a hole exactly where a longer body
        // puts its extra cells. `corpse` is absent on purpose — see above.
        if !kinfood {
            for name in ["ant", "ant_long", "ant_wide", "ant_block", "chitin_mid", "chitin_pale"] {
                if let Some(id) = world.materials.id_of(name) {
                    world.materials.get_mut(id).food_energy = 0.0;
                }
            }
        }
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
    // The face value behind `best_mouthful`, so the gut's conversion factor
    // can be recovered: digestion turns face into body at `quality`.
    let best_face = (0..world.materials.len())
        .map(|i| pixel_physics::sim::material::MaterialId(i as u16))
        .map(|id| {
            let aux = if world.materials.get(id).worth_in_aux { def.body_energy.round().clamp(0.0, 65535.0) as u16 } else { 0 };
            pixel_physics::sim::creature::food_value(&world, Cell::new(id, 0).with_aux(aux))
        })
        .fold(0.0f32, f32::max);
    println!(
        "creature probe: {frames} frames, reporting {ants} ants every {every} | terrain={} seed={seed:#x} body={} cells pitch={pitch} | kin flesh {} eats_kin {}",
        if world_terrain { "world" } else { "slab" },
        def.body.len(),
        // Echoed because a knob nobody can see the value of is a knob
        // nobody can tell is disconnected (`CLAUDE.md`) -- and these two
        // are the whole of §R3's control, so a log that does not name them
        // cannot be read as either arm.
        if kinfood { "edible" } else { "WORTHLESS" },
        if def.eats_kin { "ON" } else { "off" }
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
        "  economy: start_energy {:.0} body_energy {:.0} crop {:.0} digest {:.2}/tick reproduce_threshold {:.0} mutation_rate {:.3} birth_grant {:.2} (= {:.0} energy)",
        def.start_energy,
        def.body_energy,
        def.crop_capacity,
        def.digest_rate,
        def.reproduce_threshold,
        def.mutation_rate,
        pixel_physics::sim::creature::grant_fraction(def.traits[pixel_physics::sim::organism::TRAIT_BIRTH_GRANT]),
        pixel_physics::sim::creature::birth_grant(&def, &def.traits)
    );
    // **A rate and a time, because the bound this used to print no longer
    // exists.** It read `hunger_fraction * start_energy + best mouthful` --
    // the roof an animal's bank could not pass because it stopped eating once
    // comfortable. A crop has no roof: an animal digests what it carries at
    // `digest_rate` and what limits it is how long it can keep feeding.
    //
    // **The old line's hard-won caveat survives the change and is why this
    // one is worded as it is.** That bound was measured to be *exceeded* --
    // printed 540 against a measured `richest bank` of 616 -- because it
    // priced the founder's gut while digestion uses the organism's own
    // heritable one, and because a corpse carries its occupant's unspent bank
    // on top of the stamp. Both channels are still outside the arithmetic
    // below, and it prices the best cell in the whole material table rather
    // than the best one an ant in this world can reach. So this can say a
    // child is a long way off and it cannot say one is close: read `richest
    // bank` below, which is the measurement rather than the model.
    let quality = if best_face > 0.0 { best_mouthful / best_face } else { 0.0 };
    let upkeep = def.idle_cost_per_cell * def.body.len() as f32;
    let net = def.digest_rate * quality - upkeep;
    println!(
        "  reachability: birth costs {:.0}; on the best mouthful in the table ({best_mouthful:.0}) an ant nets {net:+.3}/tick after {upkeep:.3} upkeep -- {}",
        pixel_physics::sim::creature::birth_cost(&def),
        if net > 0.0 {
            format!(
                "about {:.0} ticks of uninterrupted feeding per child, against an idle life of {:.0}. Optimistic: read `richest bank` below",
                pixel_physics::sim::creature::birth_cost(&def) / net,
                def.start_energy / upkeep.max(f32::EPSILON)
            )
        } else {
            "it cannot out-eat its own upkeep on any food in the table, so no amount of time produces a birth".to_string()
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
            let ax = 100 + (i * pitch) as i32;
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
            world.plant_ant(20 + (i * pitch) as i32, floor - 1);
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
    // **Where the bodies went, as accounts rather than as a death count.**
    // §R3's open question is that 22 of 34 animals disappear while `deaths`
    // moves by 12, and `CLAUDE.md` names that counter as one that has
    // already failed here — so the outcome measure has to come from
    // somewhere else. The ledger is that somewhere, and it partitions the
    // question three ways instead of leaving it as one absence:
    //
    // * `stamped` is meat created when a body is *built*, so
    //   `stamped / body_energy` is **how many animals were ever placed** —
    //   a direct read on §R3's effect 1 that does not go through peak
    //   population, which is a max over a trajectory and conflates
    //   placement with survival.
    // * `harvested_corpse` is meat **eaten off a corpse**: scavenging, the
    //   thing the colony is supposed to live on.
    // * `meat_lost` is meat **destroyed without being eaten** — the sweep
    //   burying or crushing it, and (since S6) the stamp of living flesh
    //   bitten off an animal that is still alive. A cannibalism story has
    //   to show up here or it is not happening.
    //
    // Printed unconditionally: these cost nothing, and a number nobody can
    // see is a number nobody can tell is flat.
    // **Two counts of "how many animals are left", and they are not the
    // same question.** `live` above counts **head cells standing in the
    // world**; `live_creature_count` counts **entries in the organism
    // registry**. A healthy colony has them equal. They come apart exactly
    // when a body is removed from the world without its organism being
    // retired — or the reverse — and §R3's "22 animals unaccounted for"
    // is that gap being read through a death counter that never sees it.
    //
    // Censusing the standing flesh alongside them closes the third corner:
    // `ant` cells with no head are a body that lost its head, and a
    // registry entry with no cells is a ghost. Which of the three numbers
    // moves says which of those it is, and no one of them can say it alone
    // (`CLAUDE.md`: pair every "it fired" counter with an effect counter
    // from the far side of the call).
    let registry = world.live_creature_count();
    let mut ant_cells = 0usize;
    let mut headless_ant_cells = 0usize;
    for x in 0..w {
        for y in 0..h {
            let cell = world.get(x, y);
            if cell.material != ant {
                continue;
            }
            ant_cells += 1;
            if world.organism(cell.organism_id()).is_none() {
                headless_ant_cells += 1;
            }
        }
    }
    println!(
        "population: {live} head cells vs {registry} registry entries (gap {}) | standing ant cells {ant_cells}, of which {headless_ant_cells} belong to no organism",
        registry as i64 - live as i64
    );
    // **Which cell type the standing flesh actually carries**, because
    // "no head cells" has two readings and they call for opposite fixes:
    // the heads were destroyed (a body-integrity bug), or they are standing
    // there wearing a different label (a marking bug, and every consumer
    // keyed on `CellType::Head` is then looking straight past a live
    // animal). A histogram separates them in one line; the population
    // counts above cannot.
    let mut types: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for x in 0..w {
        for y in 0..h {
            let cell = world.get(x, y);
            if cell.material != ant {
                continue;
            }
            let label = match pixel_physics::sim::organism::cell_type(cell.aux()) {
                Some(t) => format!("{t:?}"),
                None => format!("unrecognised(aux={})", cell.aux()),
            };
            *types.entry(label).or_default() += 1;
        }
    }
    // **The mechanism check, and it is a claim about `chain` rather than
    // about the world.** If the head is being *overwritten* by its own
    // trailing segment, the cause is visible in the position list itself:
    // `body_after_step` builds a chain's next body as `[head, chain[0],
    // ..., chain[n-2]]`, so a head that steps into a cell its own body
    // already occupies puts the **same position twice** in that list, and
    // `relocate_chain` writes it twice -- last write wins, and the last
    // write is a Segment.
    //
    // That needs three cells: at `Chain(2)` the list is `[head, chain[0]]`
    // and the two are distinct however the animal turns, which is exactly
    // the length threshold the histogram above shows. So a duplicate in a
    // live chain is the signature, and its absence would refute this.
    let mut dup_chains = 0usize;
    let mut headless_chains = 0usize;
    let mut chained = 0usize;
    for id in 1..4096u16 {
        let Some(state) = world.organism(id) else { continue };
        if world.species.get(state.species).creature.is_none() || state.chain.is_empty() {
            continue;
        }
        chained += 1;
        let mut seen = std::collections::HashSet::new();
        if !state.chain.iter().all(|p| seen.insert(*p)) {
            dup_chains += 1;
        }
        if let Some(&(hx, hy)) = state.chain.first() {
            if pixel_physics::sim::organism::cell_type(world.get(hx, hy).aux()) != Some(pixel_physics::sim::organism::CellType::Head) {
                headless_chains += 1;
            }
        }
    }
    println!("chain integrity: {chained} chains | {dup_chains} contain a repeated position | {headless_chains} have a non-Head cell at chain[0]");
    println!(
        "standing ant cell types: {}",
        if types.is_empty() { "none".to_string() } else { types.iter().map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>().join("  ") }
    );
    let l = &world.energy_ledger;
    println!(
        "ledger: granted {:.0} stamped {:.0} | harvested plant {:.0} corpse {:.0} | metabolized {:.0} moved {:.0} synapse {:.0} | stored_in_meat {:.0} meat_lost {:.0} dissipated {:.0} overdrawn {:.0}",
        l.granted, l.stamped, l.harvested_plant, l.harvested_corpse, l.metabolized, l.moved, l.synapse_tax, l.stored_in_meat, l.meat_lost, l.dissipated, l.overdrawn
    );
    // `stamped` is in joules; a body is `body_energy` per cell times its
    // length, so this divides out both and reports **animals**. It is the
    // placement counter §R3 asks for, and it is independent of
    // `births_denied_no_space` above rather than a restatement of it: that
    // one counts refusals, this one counts bodies that made it into the
    // world, and a founder placed by the harness is in this and not in that.
    let per_body = (def.body_energy as f64) * def.body.len() as f64;
    println!(
        "bodies ever built (stamped / body_energy / cells): {:.1} at {:.0} J each ({} cells x {:.0} J)",
        l.stamped / per_body.max(1.0),
        per_body,
        def.body.len(),
        def.body_energy
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
        let carrying = world.organism(world.get(x, y).organism_id()).is_some_and(|s| s.crop.is_some());
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
            state.crop.map_or("-", |_| "yes"),
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
