//! **The biosphere page, in text and in a picture, with the controls that
//! say whether its numbers can move.**
//!
//! `src/lab/stats.rs` is a page of counters over a living box, which is the
//! exact shape `CLAUDE.md` says lies most readily: *"a number that is
//! arithmetically correct and answers a different question than the one asked
//! looks exactly like a result"*, six occurrences across two sessions. So
//! this harness is built around **positive controls** rather than around a
//! render — each `control=` mode constructs a box whose answer is known in
//! advance, and the run either reports it or the instrument is broken.
//!
//! ```text
//! cargo run --release --example labstats                        # the standard bed, dumped
//! cargo run --release --example labstats -- control=empty       # no founders, no colony
//! cargo run --release --example labstats -- control=plants      # plants only
//! cargo run --release --example labstats -- control=ants        # animals only
//! cargo run --release --example labstats -- control=cull        # kill half the stand mid-run
//! cargo run--release --example labstats -- control=steady       # is anything riding a day-length cycle
//! cargo run --release --example labstats -- control=cost        # what one census costs
//! cargo run --release --example labstats -- png=page.png frames=9000
//! ```
//!
//! **It echoes its own parameters on the first line.** `CLAUDE.md`'s harness
//! rule, from a 3.5-hour study that produced eight byte-identical logs
//! because the binary predated the argument: a knob nobody can see the value
//! of is a knob nobody can tell is disconnected.

use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::stats::{self, Stats};
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// Set one trait allele on a species' ancestral vector **and** on every
/// standing animal of it -- the two halves `beetlesight=` once shipped one
/// of (see the closure of the same shape in `main`). A free function so a
/// later block of `main` can reach it after that closure's borrow has ended.
fn set_allele_on(lab: &mut Lab, species: &str, slot: usize, v: f32) -> usize {
    let Some(sid) = lab.world.species.id_of(species) else { return 0 };
    if let Some(def) = lab.world.species.get(sid).creature.as_ref() {
        let mut def = def.clone();
        def.traits[slot] = v;
        lab.world.species.set_creature(sid, def);
    }
    let living: Vec<u16> = lab
        .world
        .live_organism_ids()
        .into_iter()
        .filter(|id| lab.world.organism(*id).is_some_and(|st| lab.world.species.get(st.species).name == species))
        .collect();
    for id in &living {
        lab.world.set_organism_trait(*id, slot, v);
    }
    living.len()
}

fn main() {
    let control: String = arg("control").unwrap_or_else(|| "run".to_string());
    let frames: u64 = arg("frames").unwrap_or(9_000);
    let png: Option<String> = arg("png");
    let seed: u64 = arg("seed").unwrap_or(1);

    // Each control names the box it needs, so the case whose answer is known
    // and the case being reported are the same object.
    let (founders, colonies) = match control.as_str() {
        "empty" => (0, 0),
        "plants" => (arg("founders").unwrap_or(8), 0),
        "ants" => (0, arg("colonies").unwrap_or(1)),
        _ => (arg("founders").unwrap_or(8), arg("colonies").unwrap_or(1)),
    };
    let spec = LabBox {
        width: arg("width").unwrap_or(512),
        height: arg("height").unwrap_or(320),
        soil_depth: arg("soil").unwrap_or(80),
        founders,
        colonies,
        compartments: arg("walls").unwrap_or(1),
        // **Predators, which this harness could not place** -- so the one
        // question a predator-prey bed exists to answer could not be asked
        // of it at all. `LabBox::default()` is 0.
        predators: arg("predators").unwrap_or(0),
        seed,
        ..LabBox::default()
    };
    println!(
        "labstats: control={control} frames={frames} founders={founders} colonies={colonies} walls={} soil={} seed={seed} png={}",
        spec.compartments,
        spec.soil_depth,
        png.as_deref().unwrap_or("-")
    );

    if control == "cost" {
        return cost(spec, frames);
    }

    let mut lab = Lab::new(spec);
    // **The verb prices, patched into the live registry.** They are
    // `CreatureDef` fields compiled in via `include_str!`, so editing
    // `ant.ron` and re-running a prebuilt binary gives bit-identical "runs"
    // -- `CLAUDE.md` records three of those. Echoed on its own line whatever
    // it is set to, per the harness rule: a knob nobody can see the value of
    // is a knob nobody can tell is disconnected.
    {
        let dig_cost: Option<f32> = arg("digcost");
        let emit_cost: Option<f32> = arg("emitcost");
        let spoil_weight: Option<f32> = arg("spoilweight");
        let exposure: Option<f32> = arg("exposure");
        // **The beetle's breeding switch, as the control arm.**
        // `reproduce_threshold: 0.0` is the exogenous beetle exactly as it
        // was before 2026-09-05 -- a fixed stock that can only overshoot or
        // be swamped. Without this the new behaviour has no null to be read
        // against, and "beetles bred once in 40,000 frames" cannot be told
        // from "nothing changed".
        if let Some(v) = arg::<f32>("beetlebreed") {
            if let Some(bid) = lab.world.species.id_of("beetle") {
                if let Some(def) = lab.world.species.get(bid).creature.as_ref() {
                    let mut def = def.clone();
                    def.reproduce_threshold = v;
                    println!("labstats: beetle reproduce_threshold = {v}");
                    lab.world.species.set_creature(bid, def);
                }
            }
        }
        // **The sight allele, on the one shipped species that has an eye.**
        // The ant is authored blind, so a bed of ants cannot demonstrate that
        // `TRAIT_SIGHT_RANGE` reaches a running world at all -- the beetle
        // can, and it is the honest positive control for the gene rather
        // than a claim from the arithmetic.
        // **Setting a gene means setting it on the animals that are already
        // standing, not only on what the next one inherits.**
        //
        // `Lab::new` has already placed the bed by the time these overrides
        // run, and `place_creature` copies the species vector into each
        // `OrganismState` at founding -- so a species-only override changes
        // nothing a short run can see. `beetlesight=` shipped with that as a
        // printed caveat, which was a limitation of this harness dressed as a
        // fact about the engine. `World::set_organism_trait` closes it, and
        // this closure does both halves so no future gene has to remember to.
        let mut set_allele = |species: &str, slot: usize, v: f32| -> usize {
            let Some(sid) = lab.world.species.id_of(species) else { return 0 };
            if let Some(def) = lab.world.species.get(sid).creature.as_ref() {
                let mut def = def.clone();
                def.traits[slot] = v;
                lab.world.species.set_creature(sid, def);
            }
            let living: Vec<u16> = lab
                .world
                .live_organism_ids()
                .into_iter()
                .filter(|id| lab.world.organism(*id).is_some_and(|st| lab.world.species.get(st.species).name == species))
                .collect();
            for id in &living {
                lab.world.set_organism_trait(*id, slot, v);
            }
            living.len()
        };
        if let Some(v) = arg::<f32>("beetlesight") {
            let n = set_allele("beetle", pixel_physics::sim::organism::TRAIT_SIGHT_RANGE, v);
            println!("labstats: beetle sight allele {v}, applied to {n} standing beetle(s) and to what they breed");
        }
        // **The pace allele, on the ants, because it is the one gene a person
        // can watch.** A quick ant scurries and a slow one plods, and the
        // counters below say what that costs: every levy is charged once per
        // decision, so turns and joules move together or the gene is a free
        // speed-up.
        if let Some(v) = arg::<f32>("pace") {
            let n = set_allele("ant", pixel_physics::sim::organism::TRAIT_PACE, v);
            println!("labstats: ant pace allele {v}, applied to {n} standing ant(s) and to what they breed");
        }
        let id = lab.world.species.id_of("ant").expect("ant species");
        let mut def = lab.world.species.get(id).creature.as_ref().expect("creature").clone();
        if let Some(v) = dig_cost {
            def.dig_cost_in_moves = v;
        }
        if let Some(v) = emit_cost {
            def.emit_cost_in_moves = v;
        }
        // **The arm that separates a predator from a meal.** A beetle authors
        // `penetration_resistance: 0.8` and an ant bites at `dig_force: 1.0`,
        // so ants EAT beetles -- adding beetles to a bed adds danger and food
        // in the same act, and no count of ants can tell the two apart.
        // Dropping the ant's bite below 0.8 makes the beetle inedible while
        // leaving it exactly as dangerous, which is the only arm in which
        // "predation" means predation alone.
        // **The arms-race arm.** Bite and armour are both priced and both
        // heritable, so one lineage can pay for a harder mouth and the other
        // for a thicker shell -- every tick, for ever. Red Queen races end
        // with both sides spending more for the same outcome, which is
        // realistic and is also a way to impoverish a bed that already
        // starves its colony. This is the knob that starts one.
        if let Some(v) = arg::<f32>("beetlearmour") {
            if let Some(bid) = lab.world.species.id_of("beetle") {
                if let Some(def) = lab.world.species.get(bid).creature.as_ref() {
                    let mut def = def.clone();
                    def.traits[pixel_physics::sim::organism::TRAIT_ARMOUR] = v;
                    lab.world.species.set_creature(bid, def);
                }
            }
            let living: Vec<u16> = lab
                .world
                .live_organism_ids()
                .into_iter()
                .filter(|id| lab.world.organism(*id).is_some_and(|st| lab.world.species.get(st.species).name == "beetle"))
                .collect();
            for id in &living {
                lab.world.set_organism_trait(*id, pixel_physics::sim::organism::TRAIT_ARMOUR, v);
            }
            println!("labstats: beetle armour allele {v}, applied to {} standing beetle(s) and to what they breed", living.len());
        }
        if let Some(v) = arg::<f32>("antbite") {
            def.bite_force = Some(v);
            println!("labstats: ant bite_force = {v} (beetle armour is 0.8; below that a beetle cannot be eaten)");
        }
        if let Some(v) = spoil_weight {
            def.spoil_weight_cells = v;
        }
        if let Some(v) = exposure {
            def.exposure_cost_per_cell = v;
        }
        println!(
            "labstats: prices dig_cost_in_moves={} emit_cost_in_moves={} spoil_weight_cells={}",
            def.dig_cost_in_moves, def.emit_cost_in_moves, def.spoil_weight_cells
        );
        lab.world.species.set_creature(id, def);
    }
    // **The scent dials -- `tolerance=`, `spread=`, `drift=`, `crosskin=` --
    // and `rivalry=1`, which is the name the §2 table of the groups report
    // was measured under and now means its own narrow end: every click a
    // stranger (`tolerance=-1 spread=1`), one point per colony (`drift=0`).**
    // The switch retired into the tolerance slot (`organism::TRAIT_TOLERANCE`),
    // so this alias is how that table is re-run on the new mechanism as its
    // positive control. `tolerance=` is an allele and lands on the standing
    // ants as well as on what they breed, like `pace=`; `spread=` is felt at
    // founding, so it is applied by re-founding: the bed is rebuilt with the
    // species' spread set, since a colony's offset is drawn when its label
    // is claimed. Echoed either way, per the harness rule: a bed with two
    // colonies that never bite each other is the same picture whether the
    // dial is off or disconnected.
    {
        let rivalry = arg::<i32>("rivalry").is_some_and(|v| v != 0);
        let tolerance: Option<f32> = arg::<f32>("tolerance").or(rivalry.then_some(-1.0));
        let spread: Option<f32> = arg::<f32>("spread").or(rivalry.then_some(1.0));
        let drift: Option<f32> = arg::<f32>("drift");
        let crosskin: Option<i32> = arg::<i32>("crosskin");
        if let Some(v) = spread {
            // The offset is drawn at founding, keyed on the seed and the
            // label (`creature::colony_scent_offset`), so applying the same
            // draw to every standing animal of each label is byte-identical
            // to having founded the bed at this spread -- and the species'
            // spread is set for anything founded later.
            if let Some(id) = lab.world.species.id_of("ant") {
                let mut def = lab.world.species.get(id).creature.as_ref().expect("creature").clone();
                def.scent_spread = v;
                lab.world.species.set_creature(id, def);
            }
            let world_seed = lab.world.seed;
            let living: Vec<(u16, u32)> = lab
                .world
                .live_organism_ids()
                .into_iter()
                .filter_map(|id| lab.world.organism(id).map(|st| (id, st.colony, st.species)))
                .filter(|(_, _, sp)| lab.world.species.get(*sp).name == "ant")
                .map(|(id, col, _)| (id, col))
                .collect();
            for &(id, col) in &living {
                let off = pixel_physics::sim::creature::colony_scent_offset(world_seed, col, v);
                let traits = lab.world.organism(id).expect("live").traits;
                for (i, slot) in pixel_physics::sim::organism::SCENT_SLOTS.iter().enumerate() {
                    lab.world.set_organism_trait(id, *slot, (traits[*slot] + off[i]).clamp(-1.0, 1.0));
                }
            }
            println!("labstats: ant scent_spread = {v}, each colony's founding offset applied to {} standing ant(s)", living.len());
        }
        if let Some(v) = tolerance {
            let n = set_allele_on(&mut lab, "ant", pixel_physics::sim::organism::TRAIT_TOLERANCE, v);
            println!("labstats: ant tolerance allele {v} (radius {}), applied to {n} standing ant(s) and to what they breed", v + 1.0);
        }
        // **`tolerance2=` sets the allele on colony label 2's standing ants
        // only** -- the adoption/raid arm: one tolerant colony beside one
        // intolerant one, which is the asymmetry `TRAIT_TOLERANCE` is
        // built around and the design report's §5.5.
        if let Some(v) = arg::<f32>("tolerance2") {
            let living: Vec<u16> = lab
                .world
                .live_organism_ids()
                .into_iter()
                .filter(|id| lab.world.organism(*id).is_some_and(|st| st.colony == 2 && lab.world.species.get(st.species).name == "ant"))
                .collect();
            for id in &living {
                lab.world.set_organism_trait(*id, pixel_physics::sim::organism::TRAIT_TOLERANCE, v);
            }
            println!("labstats: ANT 2 tolerance allele {v} (radius {}), applied to {} standing ant(s) of colony 2 and to what they breed", v + 1.0, living.len());
        }
        if let Some(v) = drift {
            if let Some(id) = lab.world.species.id_of("ant") {
                let mut def = lab.world.species.get(id).creature.as_ref().expect("creature").clone();
                def.scent_drift = v;
                lab.world.species.set_creature(id, def);
            }
            println!("labstats: ant scent_drift = {v}");
        }
        if let Some(v) = crosskin {
            if let Some(id) = lab.world.species.id_of("ant") {
                let mut def = lab.world.species.get(id).creature.as_ref().expect("creature").clone();
                def.kin_crosses_kinds = v != 0;
                lab.world.species.set_creature(id, def);
            }
            println!("labstats: ant kin_crosses_kinds = {}", v != 0);
        }
        if rivalry {
            println!("labstats: rivalry=1 is the tolerance dial's narrow end (tolerance -1, spread 1): every click a stranger");
        }
    }
    {
        let w = &lab.world;
        let ant = w.species.id_of("ant");
        let scents: Vec<(u32, [f32; 3], f32)> = w
            .live_creature_groups()
            .iter()
            .filter_map(|g| {
                let first = w.live_organism_ids().into_iter().filter_map(|id| w.organism(id)).find(|s| s.species == g.species && s.colony == g.colony)?;
                Some((g.colony, pixel_physics::sim::creature::scent_of(&first.traits), pixel_physics::sim::creature::tolerance_radius(&first.traits)))
            })
            .collect();
        println!(
            "labstats: colonies placed = {:?} | scent per colony (label, scent, radius) = {:?} | ant scent_drift = {:?}",
            w.live_creature_groups().iter().map(|g| (g.colony, g.alive)).collect::<Vec<_>>(),
            scents,
            ant.and_then(|id| w.species.get(id).creature.as_ref().map(|d| d.scent_drift))
        );
    }
    // The cull control needs a moment of stand to cull; everything else runs
    // straight through.
    let cull_at = if control == "cull" { frames / 2 } else { u64::MAX };
    let mut culled = 0usize;

    for f in 0..=frames {
        if f == cull_at {
            culled = cull_half(&mut lab.world);
            println!("  frame {f:>7}: CULLED {culled} of the living organisms");
        }
        // `Lab::advance` is the real path and it is wall-clock bounded, which
        // makes a headless run non-deterministic. Ticking directly and calling
        // `observe` by hand runs the identical sequence with the clock taken
        // out -- the page never sees the difference, since it samples on
        // `world.frame`.
        // The label follows the scent before the page samples it, exactly
        // as `Lab::advance` does; the count is the "did a split fire"
        // counter, printed when it does.
        let minted = lab.world.regroup_by_scent();
        if minted > 0 {
            println!("  frame {f:>7}: {minted} group(s) named off a drifted lineage -> {:?}", lab.world.live_creature_groups().iter().map(|g| (lab.world.group_label(g.species, g.colony), g.alive)).collect::<Vec<_>>());
        }
        lab.stats.observe(&lab.world);
        if f % 900 == 0 || f == frames {
            line(&lab.stats, &lab.world);
        }
        if f < frames {
            tick(&mut lab);
        }
    }

    // **The verb-price accounts, and what they are a share of.** Sizing a
    // price needs the denominator beside it: `dig_energy` alone says the
    // charge fired, and only `dig_energy / (metabolized + moved)` says
    // whether it is a rounding error or the whole animal's budget. That
    // ratio is the number a default is derived from -- `CLAUDE.md`'s "set
    // bars from measurement with headroom", where the measurement is a share
    // rather than a joule count that means nothing on its own.
    {
        let l = &lab.world.energy_ledger;
        let st = &lab.world.creature_stats;
        let burn = l.metabolized + l.moved + l.synapse_tax;
        let share = |x: f64| if burn > 0.0 { 100.0 * x / burn } else { 0.0 };
        println!(
            "\n--- verb accounts --- digs {} spoil_dumped {} | dig_energy {:.1} ({:.1}% of burn) emit_energy {:.1} ({:.1}%) \
             | burn {:.1} = metabolized {:.1} + moved {:.1} + synapse {:.1}",
            st.digs,
            st.spoil_dumped,
            st.dig_energy,
            share(st.dig_energy),
            st.emit_energy,
            share(st.emit_energy),
            burn,
            l.metabolized,
            l.moved,
            l.synapse_tax
        );
        // **The four prices the "everything should be priced" ruling added,
        // itemised.** They all land inside `metabolized`, so without this
        // line the only way to see what any of them costs is to turn it off
        // and diff -- and the whole point of pricing a lever is being able
        // to read what it is worth. The digestive overhead is the odd one
        // out and is printed against INTAKE rather than burn, because it is
        // food that never arrived rather than energy that was spent.
        let intake = l.harvested_plant + l.harvested_corpse;
        // **Split, because the sum cannot answer the question a predator
        // raises.** Adding beetles to a bed does two opposite things at once:
        // it kills ants, and it leaves corpses that ants eat. Both raise ant
        // births, and a combined intake figure reports them identically -- so
        // "beetles changed the colony" would not say whether predation is a
        // PRESSURE or a FOOD SUPPLY, which are opposite answers to whether
        // anything should evolve to resist it.
        println!(
            "--- where the food came from --- plant {:.0} ({:.0}%) corpse {:.0} ({:.0}%)",
            l.harvested_plant,
            if intake > 0.0 { 100.0 * l.harvested_plant / intake } else { 0.0 },
            l.harvested_corpse,
            if intake > 0.0 { 100.0 * l.harvested_corpse / intake } else { 0.0 },
        );
        println!(
            "--- the priced levers --- curvature {:.1} ({:.2}% of burn) force {:.1} ({:.2}%) armour {:.1} ({:.2}%) exposure {:.1} ({:.2}%) \
             | digest overhead {:.1} of {:.1} intake ({:.2}%) | ground felt {} cells",
            st.curvature_energy,
            share(st.curvature_energy),
            st.force_energy,
            share(st.force_energy),
            st.armour_energy,
            share(st.armour_energy),
            st.exposure_energy,
            share(st.exposure_energy),
            st.digest_overhead_energy,
            intake,
            if intake > 0.0 { 100.0 * st.digest_overhead_energy / (intake + st.digest_overhead_energy) } else { 0.0 },
            st.curvature_cells_read,
        );
        // **Gnawing, beside eating, because one without the other is the
        // finding.** A colony whose `gnaws` climbs while `eats` stays flat is
        // chewing on something it will never get through -- which is what the
        // graded bite makes possible and the old binary could not express.
        println!("--- biting --- eats {} gnaws {} bites_refused {}", st.eats, st.gnaws, st.bites_refused);
        // **How much of an animal's life is spent in the open** -- the number
        // that decides whether an exposure price can select for anything at
        // all. A colony outdoors on essentially every tick has no sheltering
        // behaviour for a hazard to reward, however steep the hazard: the
        // price is then a flat tax on being alive, which selects for nothing.
        let ticks = st.ticks.max(1);
        println!(
            "--- shelter --- exposed on {} of {} creature ticks ({:.1}%) | exposure_energy {:.1} ({:.1}% of burn)",
            st.exposed_ticks,
            st.ticks,
            100.0 * st.exposed_ticks as f64 / ticks as f64,
            st.exposure_energy,
            share(st.exposure_energy)
        );
        println!(
            "--- eyes --- sight casts {} | cells read {} | sightings {} | threat sightings {}",
            st.sight_casts, st.sight_cells_read, st.sightings, st.threat_sightings
        );
    }

    // **Per group: who is left, what killed the rest, and who did the
    // killing.** The population line says *that* a colony fell; only this
    // says whether it starved or was eaten, and by whom -- which is the
    // whole question a rivalry bed is run to answer. A group with nothing
    // alive still prints if it has dead, so a wiped-out colony is a row
    // reading `alive 0` rather than a row that vanished.
    {
        let w = &lab.world;
        let name = |sp: pixel_physics::sim::organism::SpeciesId, col: u32| w.group_label(sp, col);
        let mut groups: Vec<(pixel_physics::sim::organism::SpeciesId, u32, u32)> =
            w.live_creature_groups().iter().map(|g| (g.species, g.colony, g.alive)).collect();
        for d in &w.group_deaths {
            if !groups.iter().any(|(sp, col, _)| *sp == d.species && *col == d.colony) {
                groups.push((d.species, d.colony, 0));
            }
        }
        groups.sort_unstable_by_key(|(sp, col, _)| (*col, sp.0));
        println!("\n--- groups ---");
        for (sp, col, alive) in groups {
            let mut line = format!("  {:<10} alive {alive:>4}", name(sp, col));
            if let Some(d) = w.group_deaths_of(sp, col) {
                for cause in pixel_physics::sim::organism::DEATH_CAUSE_LIST {
                    let n = d.by_cause[cause.index()];
                    if n > 0 {
                        line.push_str(&format!("  {} {n}", cause.label().to_lowercase()));
                    }
                }
                for (asp, acol, n) in &d.killed_by {
                    line.push_str(&format!("  | killed by {} x{n}", name(*asp, *acol)));
                }
            }
            println!("{line}");
        }
    }

    println!("\n--- the page, as text ---");
    for row in stats::dump(&lab.stats, &lab.world) {
        println!("  {row}");
    }

    check(&control, &lab, culled);

    if let Some(path) = png {
        let mut buf = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        // **`page=ants` draws the whole lab frame with the ANTS page open**
        // -- the bar, the per-group graph and the legend that names a
        // split-off group `ANT 1b` -- through `Lab::draw`, the path the
        // player sees. The default is the biosphere overlay over the bare
        // world, as before. A group's colour in the box and its legend row
        // come from one function (`render::group_colour`), so this is the
        // picture the ANTS page's naming is judged by.
        let page: Option<String> = arg("page");
        if page.as_deref() == Some("ants") {
            // The start-up help page would cover the box; a harness frame
            // is the box. (`PIXEL_PHYSICS_LAB_HELP=0` does the same.)
            lab.show_help = false;
            // The biosphere overlay is up in a fresh headless lab and would
            // cover the page; `Lab::act` closes it when a page opens, and
            // this is that rule by hand.
            if lab.stats.showing() {
                lab.stats.toggle();
            }
            lab.ui.toggle_panel(pixel_physics::lab::ui::Panel::Ants);
            lab.ui.observe(&lab.world);
            lab.draw(&mut buf, 60.0);
        } else {
            let touched = lab.world.take_touched_chunks();
            lab.renderer.draw(&lab.world, &lab.particles, &touched, &mut buf, (WIDTH, HEIGHT), true);
            // A cursor over the first row, so the hover the page carries is in
            // the picture rather than described in a caption.
            let hover: Option<i32> = arg("hover");
            match hover {
                Some(row) => lab.stats.draw_at(&mut buf, &lab.world, Some((WIDTH as i32 - 200, row))),
                None => lab.stats.draw(&mut buf, &lab.world),
            }
        }
        // **Nearest-neighbour, integer factor.** The page is 5x7 glyphs on a
        // 512-wide framebuffer and the review queue's own note is that the
        // stills the owner has been able to judge are 700-950 px across; a
        // smoothing resize would turn a 5x7 glyph into a smear, which is the
        // exact failure `Reports/lanes/creature-lane-g.md` spent an hour on.
        let zoom: u32 = arg("zoom").unwrap_or(2);
        let (zw, zh) = (WIDTH * zoom, HEIGHT * zoom);
        let mut big = vec![0u8; (zw * zh * 4) as usize];
        for y in 0..zh {
            for x in 0..zw {
                let src = (((y / zoom) * WIDTH + (x / zoom)) * 4) as usize;
                let dst = ((y * zw + x) * 4) as usize;
                big[dst..dst + 4].copy_from_slice(&buf[src..src + 4]);
            }
        }
        image::save_buffer(&path, &big, zw, zh, image::ColorType::Rgba8)
            .expect("writing the page");
        println!("wrote {path} ({zw}x{zh}, {zoom}x)");
    }
}

fn tick(lab: &mut Lab) {
    // The same sequence `Lab::advance` runs, reached directly so a headless
    // run is deterministic. `frame::step` is the one shared tick both
    // binaries call.
    pixel_physics::sim::frame::step(
        &mut lab.world,
        &mut lab.particles,
        &mut lab.blasts,
        pixel_physics::sim::player::PlayerInput::default(),
        &pixel_physics::sim::player::Tuning::default(),
    );
}

/// One greppable line per stop: everything the page's headline says, plus the
/// counters the rates are built from.
fn line(stats: &Stats, world: &World) {
    let Some(c) = stats.census() else {
        println!("  (no census)");
        return;
    };
    // **Split the animals by species, because one total cannot show a
    // cycle.** A predator-prey question is about two curves and the phase
    // between them; `c.animals` is their *sum*, in which a rise in one
    // against a fall in the other is exactly invisible. Printed as its own
    // short line rather than widened into the row above, which is already at
    // the width a terminal will wrap.
    let mut by_species: std::collections::BTreeMap<&str, usize> = Default::default();
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        let sp = world.species.get(st.species);
        if sp.creature.is_none() {
            continue;
        }
        *by_species.entry(sp.name.as_str()).or_default() += 1;
    }
    if by_species.len() > 1 {
        let split: Vec<String> = by_species.iter().map(|(k, v)| format!("{k} {v}")).collect();
        println!("  frame {:>7}: animals by species -- {}", c.frame, split.join(", "));
    }
    println!(
        "  frame {:>7}: plants {:>5} ({:>6} cells, size {:>3}/{:>3}/{:>4})  bank {:>5}  animals {:>4} ({:>5} cells)  senescent {:>4} | borne {:>5} sprouted {:>5} | born {:>4} died {:>4} refused {:>3} | gen p{} a{} {:?} | slots {}/{} | lines {} top {:.0}%",
        c.frame,
        c.plants,
        c.plant_cells,
        c.plant_size.low as i64,
        c.plant_size.mid as i64,
        c.plant_size.high as i64,
        // **The bank beside the stand, never pooled into it.** Measured
        // 2026-09-01 on the standard bed: at frame 30,000 the pooled figure
        // was 467 over a bed holding 48 plants and 419 ungerminated seeds,
        // which is the owner's *"5-7 obvious plants, but the count is way
        // higher like 200+"* with the arithmetic behind it.
        c.seed_bank,
        c.animals,
        c.animal_cells,
        c.senescent,
        c.seeds_borne,
        c.germinations,
        world.creature_stats.births,
        world.creature_stats.deaths,
        c.refused,
        c.plant_generation,
        c.animal_generation,
        c.generations,
        c.slots_used,
        c.slots_ceiling,
        c.lineages,
        c.top_lineage * 100.0,
    );
}

/// **The controls.** Each asserts what the box was built to guarantee, so a
/// counter that cannot move is caught here rather than believed on screen.
fn check(control: &str, lab: &Lab, culled: usize) {
    let Some(c) = lab.stats.census() else {
        println!("VERDICT: no census at all");
        return;
    };
    let history = lab.stats.history();
    let ok = |name: &str, pass: bool, said: String| {
        println!("  [{}] {name}: {said}", if pass { "PASS" } else { "FAIL" });
        pass
    };
    println!("\n--- control: {control} ---");
    let mut all = true;
    match control {
        // A box with nothing in it. Every population figure must be zero --
        // this is the specificity half: does the instrument stay quiet when
        // there is genuinely nothing to report.
        "empty" => {
            all &= ok("no plants", c.plants == 0, format!("plants {}", c.plants));
            all &= ok("no animals", c.animals == 0, format!("animals {}", c.animals));
            all &= ok("no biomass", c.biomass() == 0, format!("biomass {}", c.biomass()));
            all &= ok("no seeds", c.germinations == 0, format!("sprouted {}", c.germinations));
            all &= ok("no generations", c.generations.iter().sum::<u32>() == 0, format!("{:?}", c.generations));
        }
        // Plants only. The plant side must be non-zero and the animal side
        // must be zero -- the sensitivity half and the specificity half in
        // one box, which is what stops "animals 0" being read as the census
        // being blind.
        "plants" => {
            all &= ok("plants alive", c.plants > 0, format!("plants {}", c.plants));
            all &= ok("plant biomass", c.plant_cells > 0, format!("{} cells", c.plant_cells));
            all &= ok("no animals", c.animals == 0, format!("animals {}", c.animals));
            all &= ok("no animal biomass", c.animal_cells == 0, format!("{} cells", c.animal_cells));
            all &= ok("no breed margin", c.breed.is_none(), format!("{:?}", c.breed));
        }
        // Animals only. The mirror.
        "ants" => {
            all &= ok("animals alive", c.animals > 0, format!("animals {}", c.animals));
            all &= ok("animal biomass", c.animal_cells > 0, format!("{} cells", c.animal_cells));
            all &= ok("no plants", c.plants == 0, format!("plants {}", c.plants));
            all &= ok("a breed margin", c.breed.is_some(), format!("{:?}", c.breed));
            all &= ok("a species name", c.animal_species.is_some(), format!("{:?}", c.animal_species));
        }
        // **The sensitivity control the rest cannot give.** A count that is
        // right about a settled box may still be a constant; killing half the
        // stand mid-run and watching the strip fall is the one check that says
        // the number moves with the thing it names.
        "cull" => {
            let peak = history.iter().map(|s| s.plants).max().unwrap_or(0);
            all &= ok("something was culled", culled > 0, format!("{culled} organisms"));
            all &= ok(
                "the strip fell",
                (c.plants as u32) < peak,
                format!("peak {peak} -> {} now", c.plants),
            );
            let peak_cells = history.iter().map(|s| s.plant_cells).max().unwrap_or(0);
            all &= ok(
                "biomass fell",
                (c.plant_cells as u32) < peak_cells,
                format!("peak {peak_cells} -> {} now", c.plant_cells),
            );
        }
        // **Is anything in a sealed, sky-held box riding a day-length
        // cycle?** `CLAUDE.md`: a designed oscillator must be divided out of
        // every number it reaches, and the lab holds the light so the biggest
        // one is gone by construction. This measures whether that is true
        // rather than assuming it: the plant count is read at the same phase
        // of successive nominal days and the spread across them reported.
        "steady" => {
            let day = 3_600u64;
            let at = |f: u64| history.iter().min_by_key(|s| s.frame.abs_diff(f)).map(|s| s.plants);
            let phases: Vec<(u64, Option<u32>)> =
                (1..=(frames_of(history) / day)).map(|k| (k * day, at(k * day))).collect();
            let quarters: Vec<(u64, Option<u32>)> =
                (1..=(frames_of(history) / day)).map(|k| (k * day + day / 2, at(k * day + day / 2))).collect();
            println!("    on the nominal day boundary: {phases:?}");
            println!("    half a day later:            {quarters:?}");
            println!("    (a systematic gap between the two rows would be a cycle to divide out)");
            all &= ok("history spans a day", frames_of(history) >= day, format!("{} frames", frames_of(history)));
        }
        _ => {
            println!("  (no control asserted -- this is the standard bed)");
        }
    }
    println!("VERDICT: {}", if all { "controls held" } else { "A CONTROL FAILED" });
}

fn frames_of(history: &[stats::Sample]) -> u64 {
    match (history.first(), history.last()) {
        (Some(a), Some(b)) => b.frame.saturating_sub(a.frame),
        _ => 0,
    }
}

/// Kill half the living organisms, oldest slot first.
///
/// `mark_organism_senescent` is the shipped experimental-disturbance seam,
/// and it produces the *graded* death `rot_remains` carries out rather than
/// erasing cells -- so the strip falls the way a real die-back falls, which
/// is the thing the control is checking the page can show.
fn cull_half(world: &mut World) -> usize {
    let ids = world.live_organism_ids();
    let mut killed = 0;
    for id in ids.iter().take(ids.len() / 2) {
        if world.mark_organism_senescent(*id) {
            killed += 1;
        }
    }
    killed
}

/// **What one census costs, and what it costs amortised over a frame.**
///
/// `CLAUDE.md`: the page must not cost the frame. Measured paired and
/// alternating on one binary in one process -- the census against a
/// same-shaped walk that does nothing -- because a single timing on a busy
/// box is the rest of the machine (two runs of a byte-identical `ascii`
/// once disagreed 2.42x).
fn cost(spec: LabBox, frames: u64) {
    let mut lab = Lab::new(spec);
    for _ in 0..frames {
        tick(&mut lab);
    }
    let organisms = lab.world.live_organism_count();
    let mut census_ns = 0u128;
    let mut idle_ns = 0u128;
    const REPS: u32 = 200;
    for _ in 0..REPS {
        // Alternating, so a drift in machine state falls on both arms.
        let t = std::time::Instant::now();
        let mut fresh = Stats::new();
        fresh.observe(&lab.world);
        census_ns += t.elapsed().as_nanos();
        let t = std::time::Instant::now();
        let ids = lab.world.live_organism_ids();
        std::hint::black_box(ids.len());
        idle_ns += t.elapsed().as_nanos();
    }
    // The paint, separately. It is a different bargain from the census --
    // every panel in this repo forces a full redraw while it is open, and
    // that cost belongs to the renderer rather than to this page -- but the
    // page's own painting is its to answer for.
    let mut draw_ns = 0u128;
    let mut buf = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    lab.stats.observe(&lab.world);
    for _ in 0..REPS {
        let t = std::time::Instant::now();
        lab.stats.draw(&mut buf, &lab.world);
        draw_ns += t.elapsed().as_nanos();
    }
    let census = census_ns as f64 / REPS as f64 / 1e6;
    let idle = idle_ns as f64 / REPS as f64 / 1e6;
    let draw = draw_ns as f64 / REPS as f64 / 1e6;
    let cells: usize = lab
        .world
        .live_organism_ids()
        .iter()
        .filter_map(|id| lab.world.organism(*id))
        .map(|s| s.cells.len())
        .sum();
    println!("  settled at frame {frames}: {organisms} live organisms, {cells} living cells");
    println!("  one census                        {census:.4} ms");
    println!("  the id walk alone (the floor)     {idle:.4} ms");
    println!("  one paint of the page             {draw:.4} ms");
    println!(
        "  amortised at 60 fps: census {:.5} ms/frame (one every {} frames) + paint {draw:.4} ms/frame while open",
        census / stats::SAMPLE_INTERVAL as f64,
        stats::SAMPLE_INTERVAL,
    );
}
