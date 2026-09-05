//! **Does this world punish an animal that is worse?**
//!
//! The creature Gate 2, and it did not exist. `selection_arena` asks this
//! question for **plants only**, and the lab coordinator note has carried
//! the same line through five rounds:
//!
//! > *Gate 2, does selection have teeth in this bed, has still never been
//! > run, and `selection_arena`'s whole finding is that a null there is a
//! > statement about the world rather than about the genome. Until it
//! > passes, every evolution result measured in this bed is unvalidated.*
//!
//! ```text
//! cargo run --release --example creature_arena -- arm=same mirror=off   # the control that means something
//! cargo run --release --example creature_arena -- arm=lethal seeds=12   # the mandatory negative control
//! cargo run --release --example creature_arena -- arm=random seeds=12
//! ```
//!
//! # Why an instrument comes before the mechanism
//!
//! `Reports/creature-genome-flexibility-2026-09-02.md` §0 and §9. The noise
//! floor in this bed is measured: `labbatch`, 12 seeds at 9,000 frames, puts
//! the **world seed alone** at **2.42x-3.12x** across the lab census with no
//! true effect present. So if the de-hardcoded ancestor stops foraging, we
//! cannot currently tell *the mechanism failed* from *this bed never
//! selected for anything* — and the creature line has now ended three times
//! with the finding that the answer was the ecology and not the creature.
//!
//! # The four things this is shaped around
//!
//! **1. One bed, and the mirror run that makes it fair.** Both arms stand in
//! one `LabBox` and compete for the same plants, water and space. But a
//! colony is founded left to right along the ground, so a founder at the end
//! of the row is not interchangeable with one in the middle: it has fewer
//! neighbours, different ground, and a different distance to the nest patch.
//! So every scene is run **twice** with the arm assignment mirrored
//! (A,B,A,B... then B,A,B,A...) and the pair pooled, which cancels the
//! position effect exactly rather than approximately.
//!
//! **2. The control comes first — and mirrored it is vacuous.** With
//! `arm=same` the mirrored pair is *the same simulation with the labels
//! swapped*, so pooling gives `A == B` as an algebraic identity and its
//! exact 50.0% says nothing. That was the first result the plant harness
//! produced and it was worthless. The control that means something is
//! `arm=same mirror=off`, which leaves the position confound *in* and asks
//! whether it alone manufactures a winner. That number is also the size of
//! the thing the mirror exists to cancel.
//!
//! **3. Attribution is by `OrganismState::lineage`, never by genome and
//! never by position.** A creature's genome mutates at every birth, so
//! classifying by genome would lose the descendants; and — the reason this
//! harness cannot inherit the plant one's reasoning — **animals move**. Two
//! plants stay where they were planted, so position is nearly a label for
//! them; two colonies of ants mix within a few hundred frames. `lineage` is
//! carried through `Origin::Bud` unchanged, which is what makes an arm's
//! share countable when the two arms are the *same* genome, which is
//! precisely the control that must work.
//!
//! **4. `arm=` is a ladder, not a switch**, so it reports *where* the world
//! stops discriminating rather than yes/no. `lethal` is the mandatory
//! negative control and **must** be detected or the harness is blind;
//! `random` is the one that actually matters, because it asks whether this
//! bed rewards the authored instinct over noise, which is Gate 2's question
//! in one word.
//!
//! # Reading it
//!
//! **The headline is how many seeds moved the same way, not a difference of
//! means.** The lab census spans 2.42x-3.12x on the seed alone, so a mean
//! over that spread is not a result. Per-seed shares are printed and the
//! direction statistic sits under them.

use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::brain;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::{frame, player};

fn arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{name}=")).map(|v| v.parse().ok().expect("parses")))
}

fn arg_str(name: &str) -> Option<String> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{name}=")).map(|v| v.to_string()))
}

/// **How arm B's genome differs from the one the species authors.**
///
/// Each rung is a handicap of known direction except `Same` (no change) and
/// `Random` (genuinely ambiguous, and the interesting one).
#[derive(Clone, Copy, PartialEq)]
enum Arm {
    /// The control. Both arms carry `ant.ron`'s authored instincts.
    Same,
    /// **The mandatory negative control**: every weight zeroed. A brain with
    /// no connections emits nothing, so the animal never chooses to move,
    /// eat, dig or breed and lives exactly as long as its grant. If the bed
    /// cannot punish *this*, nothing measured in it means anything.
    Lethal,
    /// Every weight into the `Feed` output zeroed: it can still walk, still
    /// pick things up, still lay trail — it just never swallows. A milder
    /// rung than `Lethal` and still of known direction.
    NoFeed,
    /// Every weight from the four pheromone inputs zeroed. It can eat and
    /// walk; it cannot follow a trail, which is the whole of central-place
    /// foraging.
    NoTrail,
    /// A freshly drawn random genome against the authored one. **The rung
    /// that asks Gate 2's real question**: does this bed reward the instinct
    /// we wrote over noise? A world in which it does not is a world where
    /// evolving the instinct was never possible.
    Random,
    /// **Every weight out of one named input, zeroed** — `arm=ablate
    /// input=SurfaceCurvature`. The generalisation of `NoFeed`/`NoTrail`,
    /// and the reason it exists is that the fixed rungs answer only the
    /// questions somebody thought to hard-code.
    ///
    /// **This is the shape of the question "does this environment select for
    /// X?"** — take the sense or the verb X needs, remove it, and see
    /// whether the bed punishes the animal that lost it. A bed that does not
    /// is a bed in which X can never evolve, at any population size and any
    /// number of generations, which is the finding that outranks tuning.
    ///
    /// **It cannot separate what the genome does not separate**, and that is
    /// a limit of the animal rather than of this arm — see `NoTrail`'s own
    /// doc for the case that taught it.
    AblateInput(brain::BrainInput),
    /// **One (input, output) weight, zeroed** — `arm=ablate
    /// input=Crowding output=Dig`. The sharpest form: a single edge of the
    /// brain, which is the unit selection actually acts on.
    AblateEdge(brain::BrainInput, brain::BrainOutput),
}

/// Look an input up by the name `brain::INPUT_NAMES` gives it, so the
/// command line speaks the same vocabulary as the species files.
fn input_by_name(name: &str) -> Option<brain::BrainInput> {
    brain::INPUTS.iter().copied().find(|i| brain::INPUT_NAMES[*i as usize].eq_ignore_ascii_case(name))
}

fn output_by_name(name: &str) -> Option<brain::BrainOutput> {
    brain::OUTPUTS.iter().copied().find(|o| brain::OUTPUT_NAMES[*o as usize].eq_ignore_ascii_case(name))
}

impl Arm {
    fn parse(s: &str) -> Option<Arm> {
        Some(match s {
            "same" => Arm::Same,
            "lethal" => Arm::Lethal,
            "nofeed" => Arm::NoFeed,
            "notrail" => Arm::NoTrail,
            "random" => Arm::Random,
            "ablate" => {
                let input = arg_str("input").expect("arm=ablate needs input=<name>, e.g. input=Crowding");
                let input = input_by_name(&input)
                    .unwrap_or_else(|| panic!("unknown input {input:?}; known: {:?}", brain::INPUT_NAMES));
                match arg_str("output") {
                    Some(o) => {
                        let output = output_by_name(&o)
                            .unwrap_or_else(|| panic!("unknown output {o:?}; known: {:?}", brain::OUTPUT_NAMES));
                        Arm::AblateEdge(input, output)
                    }
                    None => Arm::AblateInput(input),
                }
            }
            _ => return None,
        })
    }

    /// Arm B's genome, and **how many live slots it actually changed**.
    ///
    /// The count is returned rather than assumed, and it is checked at the
    /// call site: an `arm=` that matched no slot produces two identical arms,
    /// which read as a clean 50/50 — *indistinguishable from the finding this
    /// harness exists to make*. The plant arena refuses to run for exactly
    /// this reason and so does this one.
    fn apply(self, base: &[f32], seed: u64) -> (Vec<f32>, usize) {
        let mut g = base.to_vec();
        let mut moved = 0usize;
        match self {
            Arm::Same => {}
            Arm::Lethal => {
                for i in brain::live_slots() {
                    if g[i] != 0.0 {
                        g[i] = 0.0;
                        moved += 1;
                    }
                }
            }
            Arm::NoFeed => {
                let zero = |slot: usize, g: &mut Vec<f32>, moved: &mut usize| {
                    if g[slot] != 0.0 {
                        g[slot] = 0.0;
                        *moved += 1;
                    }
                };
                for &input in brain::INPUTS.iter() {
                    zero(brain::io_slot(input, brain::BrainOutput::Feed), &mut g, &mut moved);
                }
                for h in 0..brain::BRAIN_HIDDEN {
                    zero(brain::ho_slot(h, brain::BrainOutput::Feed), &mut g, &mut moved);
                }
            }
            Arm::NoTrail => {
                use brain::BrainInput as I;
                let zero = |slot: usize, g: &mut Vec<f32>, moved: &mut usize| {
                    if g[slot] != 0.0 {
                        g[slot] = 0.0;
                        *moved += 1;
                    }
                };
                for input in [I::PheroAFront, I::PheroALateral, I::PheroBFront, I::PheroBLateral, I::PheroAAlong, I::PheroBAlong] {
                    for &output in brain::OUTPUTS.iter() {
                        zero(brain::io_slot(input, output), &mut g, &mut moved);
                    }
                    for h in 0..brain::BRAIN_HIDDEN {
                        zero(brain::ih_slot(input, h), &mut g, &mut moved);
                    }
                }
            }
            Arm::Random => {
                g = brain::random_genome(seed);
                moved = brain::live_slots().filter(|&i| g[i] != base[i]).count();
            }
            Arm::AblateInput(input) => {
                // Every route out of that sense: straight to a verb, and
                // through every hidden unit. Missing the hidden half is how
                // an ablation reports "changed nothing" about an input the
                // species wires entirely through hidden units -- which is
                // exactly how `ant.ron` wires its pheromone senses.
                for &output in brain::OUTPUTS.iter() {
                    let slot = brain::io_slot(input, output);
                    if g[slot] != 0.0 {
                        g[slot] = 0.0;
                        moved += 1;
                    }
                }
                for h in 0..brain::BRAIN_HIDDEN {
                    let slot = brain::ih_slot(input, h);
                    if g[slot] != 0.0 {
                        g[slot] = 0.0;
                        moved += 1;
                    }
                }
            }
            Arm::AblateEdge(input, output) => {
                let slot = brain::io_slot(input, output);
                if g[slot] != 0.0 {
                    g[slot] = 0.0;
                    moved += 1;
                }
            }
        }
        (g, moved)
    }
}

/// One arm's tally at one instant.
#[derive(Default, Clone, Copy)]
struct Tally {
    /// Animals alive now.
    animals: usize,
    /// Body cells standing now — the ink-on-screen measure, which moves
    /// before the head count does when an arm is starving.
    cells: usize,
    /// Deepest generation reached. A lineage that never breeds stays at 0
    /// however long it survives, and that is a different failure from dying.
    deepest_gen: u16,
}

/// The outcome of one world.
struct Outcome {
    a: Tally,
    b: Tally,
    /// Distinct lineages **ever seen**, per arm. Births, in effect, and the
    /// number that separates "arm B lost" from "arm B never bred".
    ever: (usize, usize),
    /// **How many frames a founder lives on its grant doing nothing at
    /// all**, from the species' own constants. Carried out of the run
    /// because it is the bar the horizon has to clear -- see `idle_life`.
    idle_life: u64,
}

/// **Frames a founder survives on its founding grant while doing nothing.**
///
/// `start_energy / (idle_cost_per_cell * cells)` ticks, times
/// `tick_interval` frames. For the shipped ant that is
/// `200 / (0.05 * 2) * 6` = **12,000 frames**.
///
/// This is printed, and checked against the run length, because it is the
/// trap this harness fell into on its first real run. `labbatch`'s horizon
/// is 9,000 frames -- chosen for plants -- and at 9,000 the zeroed-brain arm
/// read **52 of 52 alive, generation 0, and 65.8% of the animals**, i.e. it
/// *beat* the authored ant on four seeds out of four. That is not a bed with
/// no teeth; it is a window shorter than the endowment, in which not
/// spending is strictly the better strategy and starving is not yet
/// possible. **A negative control that cannot lose inside the horizon is not
/// a negative control**, and the finding it manufactures ("this world does
/// not select") is exactly the one the harness exists to make honestly.
///
/// The general form, for any harness racing arms of anything that starts
/// with a stock: **the run has to outlast the endowment**, or what is being
/// measured is who was given more, not who earned more.
fn idle_life(def: &pixel_physics::sim::organism::CreatureDef) -> u64 {
    let per_tick = def.idle_cost_per_cell * def.body.len() as f32;
    if per_tick <= 0.0 {
        return u64::MAX;
    }
    (def.start_energy / per_tick) as u64 * def.tick_interval.max(1)
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|p, q| p.partial_cmp(q).expect("no NaN"));
    if v.is_empty() {
        return f64::NAN;
    }
    v[v.len() / 2]
}

fn quartiles(v: &mut [f64]) -> (f64, f64) {
    v.sort_by(|p, q| p.partial_cmp(q).expect("no NaN"));
    if v.is_empty() {
        return (f64::NAN, f64::NAN);
    }
    (v[v.len() / 4], v[(3 * v.len() / 4).min(v.len() - 1)])
}

/// **How many seeds moved the same way**, which is the headline.
///
/// Not a mean: the lab census spans 2.42x-3.12x on the world seed alone, so
/// a mean over that spread is a sample of the noise. Returns
/// `(below, above, ties)` against a 50% null.
fn direction(shares: &[f64]) -> (usize, usize, usize) {
    let below = shares.iter().filter(|&&s| s < 50.0 - 1e-9).count();
    let above = shares.iter().filter(|&&s| s > 50.0 + 1e-9).count();
    (below, above, shares.len() - below - above)
}

/// One world, one mirror setting.
fn run_world(spec: &LabBox, frames: u64, arm: Arm, mirror: bool, arm_seed: u64) -> Outcome {
    let mut w = spec.build();
    let species_id = w.species.id_of(&spec.colony_species).expect("colony species is compiled in");
    // **The economy, as arguments — because "does this environment select
    // for X" is a question about the environment, and an arena that can only
    // vary the *genome* can only ever answer half of it.**
    //
    // These are `CreatureDef` fields compiled in via `include_str!`, so
    // editing the `.ron` and re-running a prebuilt binary gives bit-identical
    // "runs" (`CLAUDE.md` records three of those). Patching the live registry
    // is the only way to sweep them, and it is what makes the paired control
    // possible: the same ablation, run against a world where the verb costs
    // something and one where it is free, is how you tell a *selective
    // pressure* from a coincidence.
    if let Some(def) = w.species.get(species_id).creature.as_ref() {
        let mut def = def.clone();
        if let Some(v) = arg::<f32>("digcost") {
            def.dig_cost_in_moves = v;
        }
        if let Some(v) = arg::<f32>("emitcost") {
            def.emit_cost_in_moves = v;
        }
        if let Some(v) = arg::<f32>("spoilweight") {
            def.spoil_weight_cells = v;
        }
        if let Some(v) = arg::<f32>("exposure") {
            def.exposure_cost_per_cell = v;
        }
        w.species.set_creature(species_id, def);
    }
    let life = idle_life(w.species.get(species_id).creature.as_ref().expect("the colony species is a creature"));
    let base = w.species.get(species_id).genome.clone();
    assert_eq!(base.len(), brain::GENOME_LEN, "the colony species carries no genome; there is nothing to race");

    // **Arm B's genome is a function of the world seed and nothing else.**
    // Drawing it from the world's own generator would put the two mirror
    // runs on different draws, and the mirror's whole job is that the pair
    // differs in the arm assignment and in nothing else.
    let (arm_b, moved) = arm.apply(&base, 0x_A470_0000 ^ arm_seed);
    if arm != Arm::Same {
        assert!(moved > 0, "arm= matched no live slot, so both arms carry one genome. Two identical arms read as a clean 50/50, which is indistinguishable from the finding this harness exists to make");
    }

    // **Founders in x order**, because that is what decides who neighbours
    // whom and where the nest patch is relative to each animal, and it is
    // what the mirror has to invert. `found_colony_of` hands back a count
    // and not handles, so they are recovered from the grid.
    let mut founders: Vec<(i32, u16)> = Vec::new();
    for y in 0..spec.height {
        for x in 0..spec.width {
            let id = w.get(x, y).organism_id();
            if id == 0 || founders.iter().any(|&(_, seen)| seen == id) {
                continue;
            }
            if w.organism(id).is_some_and(|s| s.species == species_id) {
                founders.push((x, id));
            }
        }
    }
    founders.sort_by_key(|&(x, _)| x);
    assert!(founders.len() >= 8, "{} founders placed; a bed with almost no animals in it cannot race two arms", founders.len());

    let mut arm_of_lineage: std::collections::HashMap<u32, bool> = std::collections::HashMap::new();
    let (mut n_a, mut n_b) = (0usize, 0usize);
    for (i, &(_, id)) in founders.iter().enumerate() {
        let is_b = (i % 2 == 1) != mirror;
        if is_b {
            assert!(w.set_organism_genome(id, arm_b.clone()), "the founder must be live when its arm is assigned");
            n_b += 1;
        } else {
            n_a += 1;
        }
        let lineage = w.organism(id).expect("founder is live").lineage;
        arm_of_lineage.insert(lineage, is_b);
    }
    assert!(n_a.abs_diff(n_b) <= 1, "arms must be balanced: {n_a} A against {n_b} B");

    let (mut particles, mut blasts, tuning) = (ParticleSystem::default(), Blasts::default(), player::Tuning::default());
    let mut ever_a: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
    let mut ever_b: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
    let mut last = (Tally::default(), Tally::default());

    for f in 1..=frames {
        frame::step(&mut w, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        // Sample every 500 frames as well as at the end, so `ever` catches
        // a lineage that lived and died inside the run. A census taken only
        // at the end cannot see a line that bred and was wiped out, which is
        // exactly the shape of a losing arm.
        if !f.is_multiple_of(500) && f != frames {
            continue;
        }
        let (mut a, mut b) = (Tally::default(), Tally::default());
        for id in w.live_organism_ids() {
            let Some(state) = w.organism(id) else { continue };
            if state.species != species_id {
                continue;
            }
            let Some(&is_b) = arm_of_lineage.get(&state.lineage) else { continue };
            let t = if is_b { &mut b } else { &mut a };
            t.animals += 1;
            t.cells += state.chain.len();
            t.deepest_gen = t.deepest_gen.max(state.generation);
            // **Keyed on the lineage, not the `u16` handle.** Handles are
            // recycled after sixteen reuses and would silently merge two
            // unrelated lines; `claim_lineage` counts monotonically. But a
            // *lineage* label is shared by a whole descent, so this counts
            // founder lines that still exist rather than births -- paired
            // with `generation` below, which is what says a line bred.
            if is_b { &mut ever_b } else { &mut ever_a }.insert(state.lineage);
        }
        last = (a, b);
    }
    Outcome { a: last.0, b: last.1, ever: (ever_a.len(), ever_b.len()), idle_life: life }
}

fn main() {
    let arm_name = arg_str("arm").unwrap_or_else(|| "same".into());
    let arm = Arm::parse(&arm_name).unwrap_or_else(|| panic!("unknown arm={arm_name}; try same, lethal, nofeed, notrail, random"));
    let seeds: u64 = arg("seeds").unwrap_or(12);
    let frames: u64 = arg("frames").unwrap_or(9_000);
    let mirror: bool = arg::<String>("mirror").as_deref() != Some("off");
    let ants: i32 = arg("ants").unwrap_or(LabBox::default().colony_ants);
    // **Which animal is in the box.** `ancestor` is the nest-free lab
    // founder (`assets/species/ancestor.ron`), and the question stage 3 asks
    // of it is not "does it beat the ant" -- they are different species in
    // different worlds -- but *does it do anything at all*, which is
    // `species=ancestor arm=lethal`: itself against a zeroed brain.
    let species = arg_str("species").unwrap_or_else(|| LabBox::default().colony_species);
    let founders: usize = arg("founders").unwrap_or(LabBox::default().founders);

    println!("creature_arena: species={species} arm={arm_name} seeds={seeds} frames={frames} mirror={} ants={ants} founders={founders}", if mirror { "on" } else { "off" });
    if arm == Arm::Same && mirror {
        println!("  NOTE: arm=same with mirror=on is an ALGEBRAIC IDENTITY -- one simulation with the labels swapped.");
        println!("        It must read exactly 50.0%, and that says only that the harness runs. Use mirror=off for the control that means something.");
    }

    let mut share_animals: Vec<f64> = Vec::new();
    let mut share_cells: Vec<f64> = Vec::new();
    println!("\n{:>5} {:>8} {:>8} {:>9} {:>8} {:>8} {:>9} {:>7} {:>7}", "seed", "A alive", "B alive", "B share", "A cells", "B cells", "B cells%", "A gen", "B gen");
    for seed in 1..=seeds {
        // **Predators, which this arena could not place** -- so the one
        // hazard the engine already has could not be put on the other side
        // of an ablation. A beetle authors `dig_force: 0.3` against soil's
        // 0.8, so it cannot cut ground: if a gallery is a refuge at all, it
        // is a refuge *already*, and `arm=ablate input=Bias output=Dig` with
        // and without beetles is the whole test.
        let spec = LabBox {
            colonies: 1,
            founders,
            colony_ants: ants,
            colony_species: species.clone(),
            predators: arg("predators").unwrap_or(0),
            seed,
            ..LabBox::default()
        };
        let runs = if mirror { vec![false, true] } else { vec![false] };
        let (mut a, mut b) = (Tally::default(), Tally::default());
        let (mut ea, mut eb) = (0usize, 0usize);
        for m in runs {
            let o = run_world(&spec, frames, arm, m, seed);
            if seed == 1 && !m {
                println!("  founding grant lasts {} frames of doing nothing; this run is {frames}. {}", o.idle_life, if frames >= o.idle_life {
                    "The horizon outlasts the endowment, so an arm that never feeds must die inside it."
                } else {
                    "*** THE HORIZON IS SHORTER THAN THE ENDOWMENT. An arm that does nothing cannot starve inside this run, so it cannot lose, and a negative control that cannot lose is not one. Raise frames= above the grant. ***"
                });
            }
            a.animals += o.a.animals;
            b.animals += o.b.animals;
            a.cells += o.a.cells;
            b.cells += o.b.cells;
            a.deepest_gen = a.deepest_gen.max(o.a.deepest_gen);
            b.deepest_gen = b.deepest_gen.max(o.b.deepest_gen);
            ea += o.ever.0;
            eb += o.ever.1;
        }
        let sa = 100.0 * b.animals as f64 / (a.animals + b.animals).max(1) as f64;
        let sc = 100.0 * b.cells as f64 / (a.cells + b.cells).max(1) as f64;
        share_animals.push(sa);
        share_cells.push(sc);
        println!(
            "{seed:>5} {:>8} {:>8} {sa:>8.1}% {:>8} {:>8} {sc:>8.1}% {:>7} {:>7}   (lines surviving A {ea} B {eb})",
            a.animals, b.animals, a.cells, b.cells, a.deepest_gen, b.deepest_gen
        );
    }

    for (label, v) in [("animals", &mut share_animals), ("cells", &mut share_cells)] {
        let (below, above, ties) = direction(v);
        let mut c = v.clone();
        let (q1, q3) = quartiles(&mut c);
        println!(
            "\narm B share of {label}: median {:.1}%  (q1 {q1:.1}% q3 {q3:.1}%)  |  seeds below 50%: {below}, above: {above}, tied: {ties}",
            median(&mut c)
        );
    }
    println!("\nRead the seed count, not the median. The lab census spans 2.42x-3.12x on the world seed alone with no true effect present.");
    if arm == Arm::Lethal {
        let (below, _, _) = direction(&share_animals);
        println!(
            "arm=lethal is the MANDATORY negative control: {below} of {} seeds put the zeroed brain behind. {}",
            share_animals.len(),
            if below * 4 >= share_animals.len() * 3 { "The bed has teeth." } else { "THE BED DOES NOT DISCRIMINATE AGAINST A BRAIN WITH NO CONNECTIONS. Nothing measured in it is interpretable." }
        );
    }
}
