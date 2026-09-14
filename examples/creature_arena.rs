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
//! # the developmental race: arm B's children are made at +1 and armour moves 0.8 per unit of it;
//! # plasticity=0 is the null (same genome, dial off), predators= is the pressure
//! cargo run --release --example creature_arena -- arm=wire wire=Bias:Provision:1.0 dev=armour:0.8 plasticity=1 predators=8 frames=24000
//! # a re-weighted hidden unit: `wire=` reaches the input->output block only, and `ant.ron`
//! # wires its whole trail circuit through hidden units (see `hidden_rider`)
//! cargo run --release --example creature_arena -- arm=same mirror=on seeds=6 frames=24000 \
//!     hidden=$(cargo run --release --example trailfollow -- gate=b2 spec)
//! # a bed from a scenario file rather than from the flags -- the only way to
//! # race two arms in a bed the flags cannot describe, a pond among them
//! cargo run --release --example creature_arena -- arm=lethal scenario=the_pond_shore seeds=6 frames=24000
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

use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::brain;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::{frame, player, World};

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
// `Clone` and not `Copy` since `Wire` carries its edge list; the harness
// passes the arm by reference and clones it once per world.
#[derive(Clone, PartialEq)]
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
    /// **Named direct weights SET on arm B** -- `arm=wire
    /// wire=ThreatBearing:Turn:-0.8,ThreatNear:Move:0.6`. The inverse of an
    /// ablation: instead of asking what the world does to an animal that has
    /// lost a pathway, it asks what the world does to one that has *gained*
    /// an instinct nothing shipped carries. Built for the threat sense, whose
    /// two slots no authored genome wires -- so `ablate` on them is a no-op
    /// by construction, and the only way to ask "does flight pay in this bed"
    /// is to give one arm a flight and race it.
    Wire(Vec<(brain::BrainInput, brain::BrainOutput, f32)>),
}

/// Look an input up by the name `brain::INPUT_NAMES` gives it, so the
/// command line speaks the same vocabulary as the species files.
fn input_by_name(name: &str) -> Option<brain::BrainInput> {
    brain::INPUTS.iter().copied().find(|i| brain::INPUT_NAMES[*i as usize].eq_ignore_ascii_case(name))
}

fn output_by_name(name: &str) -> Option<brain::BrainOutput> {
    brain::OUTPUTS.iter().copied().find(|o| brain::OUTPUT_NAMES[*o as usize].eq_ignore_ascii_case(name))
}

/// **`dev=trait:weight[,...]` -- developmental weights SET on arm B**, on top
/// of whatever `arm=` did to its brain. The developmental block
/// (`brain::TRAIT_SLOTS`) is the other half of the plasticity channel: a
/// weight here says how far a child's expressed trait moves per unit of
/// what its parent handed it, and `World::plasticity` (`plasticity=`
/// below) is the dial that lets any of it count. So the paired race this
/// exists for is `arm=wire wire=Bias:Provision:1.0 dev=armour:0.8` at
/// `plasticity=1` against the same arm at `plasticity=0`: the genome is
/// identical in both, and only the dial separates them.
///
/// Resolved through `batch::trait_name` rather than `params::TRAIT_ROWS`,
/// which is `pub(crate)` to `lab`; this binary is outside it.
fn dev_rider() -> Vec<(usize, f32)> {
    let Some(spec) = arg_str("dev") else { return Vec::new() };
    spec.split(',')
        .map(|pair| {
            let (name, w) = pair.split_once(':').unwrap_or_else(|| panic!("dev entry {pair:?} wants trait_name:weight, e.g. dev=armour:0.8"));
            let slot = (0..pixel_physics::sim::organism::CREATURE_TRAITS)
                .find(|&s| pixel_physics::lab::batch::trait_name(s).eq_ignore_ascii_case(name))
                .unwrap_or_else(|| panic!("dev trait {name:?} is not one of the CREATURE_TRAITS rows"));
            let w: f32 = w.parse().unwrap_or_else(|_| panic!("dev weight {w:?} does not parse"));
            (slot, w)
        })
        .collect()
}

/// **`hidden=<Input>:<unit>:<weight>[,...]` -- input-to-hidden weights SET on
/// arm B**, on top of whatever `arm=` did to the rest of its brain.
///
/// **`wire=` cannot express this, and that is not a gap in the parser.**
/// `Arm::Wire` writes `brain::io_slot`, the direct input-to-output block;
/// `ant.ron` wires its whole trail-following circuit through *hidden* units
/// instead, so every question about that circuit is a question about weights
/// `wire=` has no slot for. `Arm::AblateInput`'s own doc already records the
/// consequence in the other direction — an ablation that missed the hidden
/// half *"reports 'changed nothing' about an input the species wires entirely
/// through hidden units, which is exactly how `ant.ron` wires its pheromone
/// senses"*. This is the same fact from the constructive side.
///
/// A rider rather than an `Arm` rung, matching `dev=`: the gate races against
/// the shipped genome with `arm=same`, so the pair differs in the twelve
/// numbers under test and in nothing else.
fn hidden_rider() -> Vec<(brain::BrainInput, usize, f32)> {
    let Some(spec) = arg_str("hidden") else { return Vec::new() };
    spec.split(',')
        .map(|entry| {
            let bits: Vec<&str> = entry.split(':').collect();
            assert_eq!(bits.len(), 3, "hidden entry {entry:?} wants Input:unit:weight, e.g. hidden=PheroBAlong:2:6.0");
            let input = input_by_name(bits[0]).unwrap_or_else(|| panic!("unknown input {:?}; known: {:?}", bits[0], brain::INPUT_NAMES));
            let unit: usize = bits[1].parse().unwrap_or_else(|_| panic!("hidden unit {:?} does not parse", bits[1]));
            assert!(unit < brain::BRAIN_HIDDEN, "hidden unit {unit} is past BRAIN_HIDDEN ({})", brain::BRAIN_HIDDEN);
            let w: f32 = bits[2].parse().unwrap_or_else(|_| panic!("hidden weight {:?} does not parse", bits[2]));
            (input, unit, w)
        })
        .collect()
}

impl Arm {
    fn parse(s: &str) -> Option<Arm> {
        Some(match s {
            "same" => Arm::Same,
            "lethal" => Arm::Lethal,
            "nofeed" => Arm::NoFeed,
            "notrail" => Arm::NoTrail,
            "random" => Arm::Random,
            "wire" => {
                let spec = arg_str("wire").expect("arm=wire needs wire=<Input>:<Output>:<weight>[,...]");
                let mut edges = Vec::new();
                for part in spec.split(',') {
                    let bits: Vec<&str> = part.split(':').collect();
                    assert_eq!(bits.len(), 3, "wire entry {part:?} wants Input:Output:weight");
                    let input = input_by_name(bits[0]).unwrap_or_else(|| panic!("unknown input {:?}; known: {:?}", bits[0], brain::INPUT_NAMES));
                    let output = output_by_name(bits[1]).unwrap_or_else(|| panic!("unknown output {:?}; known: {:?}", bits[1], brain::OUTPUT_NAMES));
                    let w: f32 = bits[2].parse().expect("a weight");
                    edges.push((input, output, w));
                }
                Arm::Wire(edges)
            }
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
            Arm::Wire(ref edges) => {
                for &(input, output, w) in edges {
                    let slot = brain::io_slot(input, output);
                    if g[slot] != w {
                        g[slot] = w;
                        moved += 1;
                    }
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
    /// **The frame the last founder was seated on**, which is 0 for every
    /// bed the flags can build and is the scenario's own founding frame
    /// under `scenario=`.
    ///
    /// **It is here because the grant check above is otherwise wrong by
    /// exactly this much.** `idle_life` is how long a founder lives doing
    /// nothing *from the moment it is founded*, so the quantity that has to
    /// clear it is the run's length **after** founding, not the run. A bed
    /// founding at 6,000 and run for 13,000 frames gives its colony 7,000 --
    /// under a 12,000-frame grant, so the negative control cannot starve and
    /// cannot lose -- while `frames >= idle_life` reads 13,000 against
    /// 12,000 and reports the horizon as sufficient. Measured on
    /// `the_pond_shore` at `frames=9000`: 52 founders, both arms alive at
    /// 34/34 and 38/40, `arm=lethal` tied.
    founded_at: u64,
    /// **What the scenario's build-time placements actually managed to
    /// put in the box**, `Placed::default()` without one.
    ///
    /// **The founder asserts do not cover this and it is the half that
    /// would go quiet.** They say animals arrived; they say nothing about
    /// the larder, and a race run in a bed whose plants all failed to place
    /// is two arms starving together -- a clean, symmetrical, entirely
    /// uninterpretable 50/50 that looks exactly like `arm=same`. A scene
    /// that contradicts the code looks like a bug in the code
    /// (`CLAUDE.md`), so the scene gets a counter.
    scenario_placed: pixel_physics::lab::scenario::Placed,
    /// **The run's final `world.creature_stats.threat_sightings`** -- sight
    /// casts, world-wide, that found an animal whose gut could digest the
    /// looker and whose bite could open the looker's armour
    /// (`src/sim/world.rs`'s own doc on the field). This is the positive
    /// control `Reports/creature-groups-and-combat-design-2026-09-06.md`
    /// §4a's flight-vs-null race was run without: a race of a
    /// threat-response arm against this sitting at zero never once had
    /// anything in front of it to flee from, so the result is a finding
    /// about the BED, not about the instinct being raced.
    ///
    /// **Whole-box, not per-arm.** Both arms are cut from the one
    /// `CreatureDef` `sight=N` patches, so they share one eye -- a non-zero
    /// total says the sense had *something* to report during the run, and
    /// cannot say which arm did the seeing. Read beside `a.animals` /
    /// `b.animals`: an arm sitting at zero could not have been the one
    /// looking.
    threat_sightings: u64,
    /// **The denominator.** Total sight casts made, world-wide, this run.
    /// `threat_sightings / sight_casts` is the duty cycle -- how often the
    /// eye had a hunter to report, the quantity
    /// `Reports/creature-vision-sizing-2026-08-30.md` §3 sized the sight
    /// radius from. A raw `threat_sightings` count with no denominator
    /// cannot be told apart from an eye that was barely ever open.
    sight_casts: u64,
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

/// `hits / casts` as a percentage -- the duty cycle, "how often the eye had
/// something to report". `"n/a"` rather than `0.0%` when `casts` is itself
/// zero: that is an eye that was never opened, a different fact from an eye
/// that opened and saw nothing, and the two must not print identically.
fn duty_cycle(hits: u64, casts: u64) -> String {
    if casts == 0 { "n/a".to_string() } else { format!("{:.1}%", 100.0 * hits as f64 / casts as f64) }
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

/// `padarm=on` -- see `pad_arm_a_to_match`'s doc for the mechanism this
/// switches on. Off by default: unlike the count print below, which is
/// read-only, this changes the actual genome arm A races, so it has to be
/// asked for.
fn pad_arm_flag() -> bool {
    arg::<String>("padarm").as_deref() == Some("on")
}

/// **The count `synapse_fraction` actually bills, read from the source of
/// truth rather than re-derived.** `brain::eval_brain`'s second return
/// value is the number of weights whose magnitude cleared `brain::W_EPS`
/// that tick -- the exact quantity `creature.rs` multiplies by
/// `synapse_fraction * start_energy` to get the per-tick tax -- so calling
/// it directly can never drift from what an animal is actually charged
/// the way a hand-written re-count of the four genome blocks could.
///
/// **Independent of `inputs` and `state`, which is what makes one dummy
/// call sufficient.** Every term in `eval_brain`'s loop tests the WEIGHT's
/// own magnitude against `W_EPS` -- never the value multiplied through it
/// -- so the count it returns is a pure function of the genome alone.
fn active_synapses(g: &[f32]) -> u32 {
    brain::eval_brain(g, &[0.0; brain::BRAIN_INPUTS], &mut [0.0; brain::BRAIN_HIDDEN]).1
}

/// A hidden unit nothing downstream can act on: every one of its weights
/// into every output sits below `W_EPS`. `eval_brain` still bills an
/// input wired into a unit like this -- the ih weight is tested on its own
/// magnitude, same as any other -- so such a unit is exactly where a
/// weight can be active (billed) and inert (unreachable) at once, which is
/// the pad this file needs.
fn hidden_output_is_silent(g: &[f32], h: usize) -> bool {
    brain::OUTPUTS.iter().all(|&o| g[brain::ho_slot(h, o)].abs() < brain::W_EPS)
}

/// Write inert weights into `g` until `active_synapses(g) == target`.
///
/// **Which hidden unit is free is read from `g`, never assumed.** `ant.ron`
/// wires units 0-3 for its trail laterals and 4 as the nest odometer,
/// which leaves 5-7 free today -- but `wire=`/`species=` can point this
/// harness at any genome, a later species file could wire a sixth unit,
/// and this function has no business knowing either. It asks
/// `hidden_output_is_silent` instead, which reads the one fact that
/// actually matters: whether anything downstream of a unit can act.
///
/// **Verified, not trusted**: `active_synapses` is called again after
/// writing, through the exact `eval_brain` path `synapse_fraction` bills
/// from, rather than assumed from how many slots this function touched. A
/// shortfall panics naming the capacity actually available -- a genome
/// that comes back still short is exactly the unbalanced race this
/// mechanism exists to prevent, so it does not return one quietly.
fn pad_with_inert_weights(g: &mut [f32], target: u32) {
    let before = active_synapses(g);
    assert!(before <= target, "pad_with_inert_weights asked to shrink {before} active synapses to {target}; it only ever adds");
    let need = (target - before) as usize;
    if need == 0 {
        return;
    }
    let free_units: Vec<usize> = (0..brain::BRAIN_HIDDEN).filter(|&h| hidden_output_is_silent(g, h)).collect();
    assert!(
        !free_units.is_empty(),
        "padarm=on: no hidden unit is free -- all {} already carry a live hidden->output weight, so any input wired into one now would reach an output and change behaviour rather than only cost. Refusing rather than padding a live unit.",
        brain::BRAIN_HIDDEN
    );
    let mut slots = Vec::with_capacity(need);
    'search: for &h in &free_units {
        for &input in brain::INPUTS.iter() {
            let slot = brain::ih_slot(input, h);
            if g[slot].abs() < brain::W_EPS {
                slots.push(slot);
                if slots.len() == need {
                    break 'search;
                }
            }
        }
    }
    assert_eq!(
        slots.len(),
        need,
        "padarm=on: only {} unused input slot(s) across free hidden unit(s) {free_units:?}, need {need} more to take {before} to {target}. Wire fewer edges, or free another hidden unit in the species genome.",
        slots.len()
    );
    for slot in slots {
        g[slot] = 1.0; // any magnitude >= W_EPS; the unit's silent output row makes the value irrelevant
    }
    let after = active_synapses(g);
    assert_eq!(
        after, target,
        "padarm=on: wrote {need} slot(s) into hidden unit(s) {free_units:?} and active_synapses now reads {after}, not {target} -- eval_brain's W_EPS rule diverged from what this function assumed"
    );
}

/// **Bring arm A's active-synapse count up to arm B's, with weights that
/// cannot act.** The fix for the confound this file's module doc opens
/// with: `synapse_fraction` bills every tick per active synapse
/// regardless of what a weight reaches, so `arm=wire wire=In:Out:w,...`
/// handing arm B named weights arm A never had also hands B that tax
/// forever, on top of whatever the wiring itself is worth -- and a race
/// decided by that measured wiring *size*, not wiring *shape*.
///
/// **Only ever pads arm A.** Arm B is the thing under study -- exactly the
/// genome `wire=`/`ablate=`/`dev=` asked for -- and this function never
/// touches it. Padding the control instead of the experiment is what
/// keeps "arm B" meaning what the caller typed.
///
/// **Refuses rather than running an unbalanced race quietly** if arm A is
/// already the heavier of the two: padding only adds, and this function
/// will not reach for arm B to compensate. (`wire=` can zero an authored
/// weight as easily as it can add one -- `wire=Bias:Move:0.0` deletes the
/// ant's baseline restlessness gain -- so "B is always heavier" is a fact
/// about the examples in this file's doc comment, not a guarantee.) It
/// also refuses if `pad_with_inert_weights` cannot find the capacity, for
/// the same reason: better to stop loudly than hand back a genome that is
/// still short.
fn pad_arm_a_to_match(arm_a: &mut [f32], arm_b: &[f32]) -> (u32, u32) {
    let count_a = active_synapses(arm_a);
    let count_b = active_synapses(arm_b);
    assert!(
        count_a <= count_b,
        "padarm=on cannot equalise: arm A already carries {count_a} active synapses against arm B's {count_b}, and this flag only ever pads arm A upward -- it never rewires arm B to compensate. Check wire=/ablate= for an entry that zeroed an authored weight instead of adding one."
    );
    if count_a < count_b {
        pad_with_inert_weights(arm_a, count_b);
    }
    (active_synapses(arm_a), count_b)
}

/// Both arms' active-synapse counts, padding arm A first if `pad_arm` is
/// set. Shared by the run header (one representative read, for the count
/// print every run gets) and `run_world` (one real read per seed x mirror
/// pair, because `arm=random` draws a fresh arm B every seed and a count
/// printed once in the header cannot speak for a genome it never saw).
fn synapse_counts(arm_a: &mut [f32], arm_b: &[f32], pad_arm: bool) -> (u32, u32) {
    if pad_arm { pad_arm_a_to_match(arm_a, arm_b) } else { (active_synapses(arm_a), active_synapses(arm_b)) }
}

/// **Give an arm to every founder that has just arrived, however late it
/// arrives.** Returns how many it labelled.
///
/// **Why this is not simply a straight line after `build`, which is what it
/// was until `scenario=` existed.** Every bed this harness could describe
/// before then had its colony standing at frame 0, so the founders were
/// there to be found the moment the world was built. A scenario's need not
/// be: the owner's 2026-09-09 correction is that a colony dropped onto
/// seedlings at frame 0 collapses at once and never recovers where one
/// founded on a grown bed holds (5 ants against 39, same seed), so every bed
/// written since founds its ants from the **timeline** -- `the_pond_shore.
/// ron` and `the_pond_stocked.ron` at frame 6,000. A frame-0 scan of one of
/// those finds **zero** animals and trips the `>= 8` assert before the run
/// has begun, which is why pointing this harness at a pond needed more than
/// a `scenario=` flag.
///
/// **"Whose lineage is not yet labelled" is exactly "the founders that were
/// just placed", and that is a fact about `lineage` rather than a
/// convenience.** `Origin::Bud` carries a lineage through unchanged -- which
/// is the same property that lets `run_world` attribute a whole descent to
/// an arm -- so a creature born since the last call already has a label and
/// is skipped, and only `found_colony_of` mints an unlabelled one. The scan
/// therefore cannot pick up a descendant and mistake it for a founder, and
/// it does not need to know which frame the colony arrived on.
///
/// `next` runs on across calls so a bed founding in two events still
/// alternates across both, which is what keeps the arms balanced when a
/// single call cannot see every founder.
#[allow(clippy::too_many_arguments)]
fn assign_new_founders(
    w: &mut World,
    spec: &LabBox,
    species_id: pixel_physics::sim::organism::SpeciesId,
    mirror: bool,
    arm_a: &[f32],
    a_padded: bool,
    arm_b: &[f32],
    arm_of_lineage: &mut std::collections::HashMap<u32, bool>,
    next: &mut usize,
    n_a: &mut usize,
    n_b: &mut usize,
) -> usize {
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
            let Some(state) = w.organism(id) else { continue };
            if state.species == species_id && !arm_of_lineage.contains_key(&state.lineage) {
                founders.push((x, id));
            }
        }
    }
    founders.sort_by_key(|&(x, _)| x);
    for &(_, id) in &founders {
        let is_b = (*next % 2 == 1) != mirror;
        *next += 1;
        if is_b {
            assert!(w.set_organism_genome(id, arm_b.to_vec()), "the founder must be live when its arm is assigned");
            *n_b += 1;
        } else {
            // Left at the founding default (`base`, unmutated) unless
            // `padarm=on` actually changed arm A -- so a run with the flag
            // present but nothing to equalise never touches this founder
            // at all, and cannot be told apart from one where the flag was
            // never passed.
            if a_padded {
                assert!(w.set_organism_genome(id, arm_a.to_vec()), "the founder must be live when its arm is assigned");
            }
            *n_a += 1;
        }
        let lineage = w.organism(id).expect("founder is live").lineage;
        arm_of_lineage.insert(lineage, is_b);
    }
    assert!(n_a.abs_diff(*n_b) <= 1, "arms must be balanced: {n_a} A against {n_b} B");
    founders.len()
}

/// One world, one mirror setting.
fn run_world(spec: &LabBox, scenario: Option<&Scenario>, frames: u64, arm: &Arm, mirror: bool, arm_seed: u64) -> Outcome {
    // **The scenario's own build, which is `spec.build()` plus its
    // placements** -- the walls, water and plants a `LabBox` has no field
    // for. `spec` is the scenario's own `bed` in that case (set in `main`,
    // with the sweep's seed written onto it there and not here), so
    // everything below that reads `spec` reads the same box either way.
    let (mut w, scenario_placed) = match scenario {
        Some(s) => {
            let (w, _planted, placed) = s.build();
            (w, placed)
        }
        None => (spec.build(), pixel_physics::lab::scenario::Placed::default()),
    };
    let species_id = w.species.id_of(&spec.colony_species).expect("colony species is compiled in");
    // **The plasticity dial**, read exactly where `World::plasticity`'s own
    // doc says it is. It ships at 1 (`creature::PLASTICITY_DEFAULT`); at 0 a
    // developmental block on arm B is carried and never expressed, which is
    // the null the `dev=` race needs.
    if let Some(v) = arg::<f32>("plasticity") {
        w.plasticity = v;
    }
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
        // **`sight=N` gives BOTH arms an eye.** The shipped ant is blind, so
        // a threat instinct wired onto it is a weight on a constant zero; the
        // fair race is two eyed colonies, one of which also knows to run.
        if let Some(v) = arg::<i32>("sight") {
            def.sight_range = v;
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
    let (mut arm_b, mut moved) = arm.clone().apply(&base, 0x_A470_0000 ^ arm_seed);
    let dev = dev_rider();
    for &(slot, w) in &dev {
        let i = brain::dev_slot(slot);
        if arm_b[i] != w {
            arm_b[i] = w;
            moved += 1;
        }
    }
    let hidden = hidden_rider();
    for &(input, unit, w) in &hidden {
        let i = brain::ih_slot(input, unit);
        if arm_b[i] != w {
            arm_b[i] = w;
            moved += 1;
        }
    }
    if *arm != Arm::Same || !dev.is_empty() || !hidden.is_empty() {
        assert!(moved > 0, "arm= matched no live slot, so both arms carry one genome. Two identical arms read as a clean 50/50, which is indistinguishable from the finding this harness exists to make");
    }

    // **`padarm=on` closes the confound this file's module doc names**: a
    // `wire=` spec that hands arm B weights arm A never had also hands it
    // their metabolism tax, forever, so the race measures size as well as
    // shape. Read fresh here rather than threaded in as a parameter,
    // matching `plasticity`/`digcost`/... above -- and it has to be read
    // per call rather than once in `main`, because `arm=random` draws a
    // different arm B every seed and a count taken once cannot speak for a
    // genome it never saw.
    let mut arm_a = base.clone();
    synapse_counts(&mut arm_a, &arm_b, pad_arm_flag());
    let a_padded = arm_a != base;

    // **The arms are handed out when the founders exist, which is frame 0
    // only when the flags built the bed** -- see `assign_new_founders`.
    // Without `scenario=` the colony is standing the moment `build` returns,
    // so this one call does the whole job right here and the run is
    // byte-identical to every run taken before `scenario=` existed.
    let mut arm_of_lineage: std::collections::HashMap<u32, bool> = std::collections::HashMap::new();
    let (mut n_a, mut n_b, mut next_founder) = (0usize, 0usize, 0usize);
    assign_new_founders(&mut w, spec, species_id, mirror, &arm_a, a_padded, &arm_b, &mut arm_of_lineage, &mut next_founder, &mut n_a, &mut n_b);
    if scenario.is_none() {
        assert!(n_a + n_b >= 8, "{} founders placed; a bed with almost no animals in it cannot race two arms", n_a + n_b);
    }
    // **The frame after which a scenario has no chances left to seat a
    // colony**, so a bed that never seats one fails there rather than at the
    // end of a 90,000-frame run that could not have measured anything.
    // Computed only when every event is one-shot: `every != 0` with
    // `until: 0` repeats for ever, so no such frame exists, and a tripwire
    // that guessed one would fail runs that were going to work. Every
    // scenario shipped today founds one-shot, so it is live where it counts.
    let founding_deadline: Option<u64> = scenario.and_then(|s| {
        s.timeline.iter().all(|e| e.every == 0).then(|| s.timeline.iter().map(|e| e.at).max().unwrap_or(0))
    });
    if let Some(d) = founding_deadline {
        // **Refuse a run too short to reach the founding, at the door.** It
        // would seat nobody, so both arms would read zero for every seed --
        // and a table of zeros is indistinguishable from a bed where neither
        // arm survived, which is the finding this harness exists to make.
        assert!(
            frames >= d || n_a + n_b >= 8,
            "this run is {frames} frames and {} does not finish founding until frame {d}; no founder would be seated and both arms would read zero for every seed, which reads exactly like a bed where neither survived",
            scenario.map(|s| s.name.as_str()).unwrap_or_default()
        );
    }

    let mut founded_at = 0u64;
    let (mut particles, mut blasts, tuning) = (ParticleSystem::default(), Blasts::default(), player::Tuning::default());
    let mut ever_a: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
    let mut ever_b: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
    let mut last = (Tally::default(), Tally::default());

    for f in 1..=frames {
        frame::step(&mut w, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        // **The timeline, immediately after the step**, which is the order
        // `tick_timeline`'s own doc requires: `frame::step` increments
        // `world.frame` first thing, so an `Event` at `at: N` lands after
        // the Nth tick. Anything it founds is labelled here and now, on the
        // frame it appears and before it has taken a single step, so a
        // colony that arrives at 6,000 starts its race exactly the way a
        // frame-0 one does.
        if let Some(s) = scenario {
            let placed = pixel_physics::lab::scenario::tick_timeline(s, &mut w, spec);
            if placed.animals > 0 && assign_new_founders(&mut w, spec, species_id, mirror, &arm_a, a_padded, &arm_b, &mut arm_of_lineage, &mut next_founder, &mut n_a, &mut n_b) > 0 {
                founded_at = f;
            }
        }
        if founding_deadline.is_some_and(|d| f == d) {
            assert!(
                n_a + n_b >= 8,
                "the scenario's timeline is spent at frame {f} and has seated {} founders; a bed with almost no animals in it cannot race two arms",
                n_a + n_b
            );
        }
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
    // **The backstop for a timeline this cannot predict**: an event with
    // `every != 0` and `until: 0` repeats for ever, so there is no deadline
    // to check in the loop and only this can catch a bed that never seated
    // anybody.
    assert!(
        n_a + n_b >= 8,
        "{} founders were seated across the whole {frames}-frame run; a bed with almost no animals in it cannot race two arms",
        n_a + n_b
    );
    Outcome {
        a: last.0,
        b: last.1,
        ever: (ever_a.len(), ever_b.len()),
        idle_life: life,
        founded_at,
        scenario_placed,
        // Read after the frame loop above has run to completion, so this is
        // the run's final total rather than a mid-run sample -- and it is a
        // running total on `World` that nothing here resets, so "final" and
        // "whole run" are the same read.
        threat_sightings: w.creature_stats.threat_sightings,
        sight_casts: w.creature_stats.sight_casts,
    }
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
    // **`scenario=<name>` builds the bed from a saved scenario instead of
    // from the flags below**, which is the only way to race two arms in a
    // box a `LabBox` has no field for -- a pond among them. A bad name
    // refuses at load rather than silently opening the default bed, the
    // rule `bin/lab.rs`, `labshot` and `labbatch` all follow.
    //
    // **The seed is NOT applied here**, unlike every other harness that
    // takes one: this one sweeps `seeds=1..=N` rather than taking a single
    // `seed=`, so the scenario is re-stamped per seed inside the loop
    // below. It still has to happen on the *scenario* and not on `spec` --
    // `Scenario::build` reads `self.bed`, so a seed written only onto
    // `spec` reaches nothing and every seed silently runs the file's own
    // pinned bed (`labshot.rs` carries the same comment for the same bug).
    let scenario: Option<Scenario> = arg_str("scenario").map(|n| {
        Scenario::load(&n).unwrap_or_else(|e| {
            eprintln!("scenario {n}: {e}");
            std::process::exit(1);
        })
    });
    // **Which animal the arms are read off, and under `scenario=` the
    // scenario says so.** `run_world` attributes by `spec.colony_species`
    // and `spec` is the scenario's own bed there, so taking the header's
    // genome preview from the `species=` default instead would print one
    // species and race another.
    let species = arg_str("species").unwrap_or_else(|| match &scenario {
        Some(s) => s.bed.colony_species.clone(),
        None => LabBox::default().colony_species,
    });
    let founders: usize = arg("founders").unwrap_or(LabBox::default().founders);
    if let Some(s) = &scenario {
        // **A scenario's timeline carries its own species per event**, so a
        // bed whose `colony_species` says one thing and whose `Colony` event
        // founds another would label nobody: every founder's lineage would
        // be skipped, both arms would read zero, and a run that measured
        // nothing would look exactly like one where neither arm survived.
        // Refuse it at the door instead, where the message can say why.
        use pixel_physics::lab::scenario::Placement;
        let founds: Vec<&str> = s
            .placements
            .iter()
            .chain(s.timeline.iter().map(|e| &e.what))
            .filter_map(|p| match p {
                Placement::Colony { species, .. } | Placement::Colonies { species, .. } | Placement::Animal { species, .. } => Some(species.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            founds.contains(&species.as_str()),
            "scenario {} founds {:?} and this race reads {species:?}; nothing would be labelled and both arms would read zero, which is indistinguishable from a bed where neither survived. Pass species= to match, or point at a scenario that founds {species:?}",
            s.name,
            founds
        );
    }

    let dev = dev_rider();
    let hidden = hidden_rider();
    println!(
        "creature_arena: species={species} arm={arm_name} seeds={seeds} frames={frames} mirror={} padarm={} ants={ants} founders={founders} predators={} plasticity={} dev={:?} hidden={:?}{}",
        if mirror { "on" } else { "off" },
        if pad_arm_flag() { "on" } else { "off" },
        arg::<i32>("predators").unwrap_or(0),
        arg::<f32>("plasticity").unwrap_or(pixel_physics::sim::creature::PLASTICITY_DEFAULT),
        dev.iter().map(|&(slot, w)| format!("{}:{w}", pixel_physics::lab::batch::trait_name(slot))).collect::<Vec<_>>(),
        hidden.iter().map(|&(i, u, w)| format!("{}:{u}:{w}", brain::INPUT_NAMES[i as usize])).collect::<Vec<_>>(),
        // **A harness that does not echo its own parameters is a harness
        // nobody can tell was pointed anywhere** -- `CLAUDE.md`'s 3.5-hour
        // megastudy that turned out to be three populations wearing 24 logs.
        scenario.as_ref().map(|s| format!(" scenario={} ({})", s.name, s.question)).unwrap_or_default()
    );
    if let Some(s) = &scenario {
        println!("  NOTE: the bed comes from the scenario, so ants= founders= predators= plant= and the bed flags are ignored; seeds=1..={seeds} is stamped onto its own bed seed ({} in the file).", s.bed.seed);
    }
    if arm == Arm::Same && mirror && dev.is_empty() && hidden.is_empty() {
        println!("  NOTE: arm=same with mirror=on is an ALGEBRAIC IDENTITY -- one simulation with the labels swapped.");
        println!("        It must read exactly 50.0%, and that says only that the harness runs. Use mirror=off for the control that means something.");
    }

    // **The confound this file's module doc opens with, measured and
    // printed every run -- not only when padarm=on.** `synapse_fraction`
    // bills per active synapse per tick regardless of what a weight
    // reaches (`brain::eval_brain`'s `W_EPS` rule), so a `wire=` spec that
    // hands arm B weights arm A never had also hands it that tax forever.
    // A harness that pads silently is one nobody can check.
    //
    // **Representative, not per-seed**: `arm=random` draws a fresh arm B
    // every seed, so this reads the seed-1 pair off a throwaway species
    // registry -- no world, no terrain; `SpeciesRegistry::builtin()` is
    // exactly what `World::new` loads from, so this is the same genome
    // every seed's `run_world` will actually race. `padarm=on`, if set,
    // re-equalises for real inside `run_world`, once per seed x mirror
    // pair -- this print is a preview, not the decision.
    let pad_arm = pad_arm_flag();
    let header_species = pixel_physics::sim::organism::SpeciesRegistry::builtin();
    let header_species_id = header_species.id_of(&species).unwrap_or_else(|| panic!("unknown species={species}"));
    let header_base = header_species.get(header_species_id).genome.clone();
    let (header_arm_b, _) = arm.clone().apply(&header_base, 0x_A470_0000 ^ 1);
    let mut header_arm_a = header_base.clone();
    let (count_a, count_b) = synapse_counts(&mut header_arm_a, &header_arm_b, pad_arm);
    let header_padded = header_arm_a != header_base;
    println!(
        "active synapses billed per tick (synapse_fraction x eval_brain's W_EPS={} rule): arm A {count_a}, arm B {count_b}{}",
        brain::W_EPS,
        if count_a == count_b {
            if header_padded { "  -- arm A padded to match (padarm=on)".to_string() } else { "  -- already equal".to_string() }
        } else {
            format!(
                "  -- DIFFER by {}: this race bills the arms unequally regardless of what either wiring does. Pass padarm=on to equalise.",
                count_a.abs_diff(count_b)
            )
        }
    );
    if arm == Arm::Random {
        println!("  (arm=random draws a fresh arm B per seed; the pair above is seed 1's, for a read on typical size -- padarm=on, if set, re-equalises fresh for every seed x mirror pair inside run_world, not from this preview)");
    }

    let mut share_animals: Vec<f64> = Vec::new();
    let mut share_cells: Vec<f64> = Vec::new();
    // **World-wide, not per-arm** -- summed over every seed and both mirror
    // runs. See `Outcome::threat_sightings`'s doc for the limit this puts on
    // what the total below can say.
    let mut total_threat_sightings: u64 = 0;
    let mut total_sight_casts: u64 = 0;
    println!("\n{:>5} {:>8} {:>8} {:>9} {:>8} {:>8} {:>9} {:>7} {:>7}", "seed", "A alive", "B alive", "B share", "A cells", "B cells", "B cells%", "A gen", "B gen");
    for seed in 1..=seeds {
        // **Predators, which this arena could not place** -- so the one
        // hazard the engine already has could not be put on the other side
        // of an ablation. A beetle authors `dig_force: 0.3` against soil's
        // 0.8, so it cannot cut ground: if a gallery is a refuge at all, it
        // is a refuge *already*, and `arm=ablate input=Bias output=Dig` with
        // and without beetles is the whole test.
        // **This seed, stamped onto the scenario and not onto the bed it
        // hands back.** `Scenario::build` reads `self.bed`, so a seed
        // written only onto `spec` below would leave every seed of the
        // sweep running the file's own pinned bed -- digit-identical
        // columns across seeds, `CLAUDE.md`'s own tell for a knob that was
        // never connected.
        let seed_scenario: Option<Scenario> = scenario.as_ref().map(|s| {
            let mut s = s.clone();
            s.bed.seed = seed;
            s
        });
        let flag_spec = LabBox {
            colonies: 1,
            founders,
            colony_ants: ants,
            colony_species: species.clone(),
            // **`plant=<species>` -- which larder the two arms compete for.**
            // `LabBox::default()` is `herb`, and every race this harness has
            // ever run was on eight herbs spread evenly over 512 columns,
            // which is the least patchy larder the bed can hold. That is not
            // a neutral choice for any question about **recruitment**: a
            // trail pays when food is clumped and worth telling a nestmate
            // about, and on an even larder a colony that converges on a
            // patch converges on one it has already eaten. Measured 2026-09-09
            // (`open-bugs-handoff.md` §Z7): a working trail-following gate
            // loses 6 of 6 mirrored seeds on the herb bed. Whether that is a
            // statement about the gate or about the herbs is exactly what
            // this knob exists to ask, and it could not be asked before.
            species: arg_str("plant").unwrap_or_else(|| LabBox::default().species),
            predators: arg("predators").unwrap_or(0),
            seed,
            ..LabBox::default()
        };
        // The scenario's own bed, seed included, or the flags' one. Built
        // either way so the flag path is unchanged; a `LabBox` literal
        // builds no world, so the unused one costs nothing.
        let spec = match &seed_scenario {
            Some(s) => s.bed.clone(),
            None => flag_spec,
        };
        let runs = if mirror { vec![false, true] } else { vec![false] };
        let (mut a, mut b) = (Tally::default(), Tally::default());
        let (mut ea, mut eb) = (0usize, 0usize);
        let (mut seed_sightings, mut seed_casts) = (0u64, 0u64);
        for m in runs {
            let o = run_world(&spec, seed_scenario.as_ref(), frames, &arm, m, seed);
            if seed == 1 && !m {
                // **Measured from founding, not from frame 0** -- see
                // `Outcome::founded_at`. Identical to `frames` on every bed
                // the flags build, where the colony is standing at 0.
                if scenario.is_some() {
                    let p = o.scenario_placed;
                    println!("  scenario placed: {} cells, {} plants, {} animals, {} settings applied (before the timeline)", p.cells, p.plants, p.animals, p.settings);
                }
                let lived = frames.saturating_sub(o.founded_at);
                println!(
                    "  founding grant lasts {} frames of doing nothing; this colony is founded at {} and so gets {lived} of this {frames}-frame run. {}",
                    o.idle_life,
                    o.founded_at,
                    if lived >= o.idle_life {
                        "The horizon outlasts the endowment, so an arm that never feeds must die inside it."
                    } else {
                        "*** THE HORIZON IS SHORTER THAN THE ENDOWMENT. An arm that does nothing cannot starve inside this run, so it cannot lose, and a negative control that cannot lose is not one. Raise frames= above the grant PLUS the founding frame. ***"
                    }
                );
            }
            a.animals += o.a.animals;
            b.animals += o.b.animals;
            a.cells += o.a.cells;
            b.cells += o.b.cells;
            a.deepest_gen = a.deepest_gen.max(o.a.deepest_gen);
            b.deepest_gen = b.deepest_gen.max(o.b.deepest_gen);
            ea += o.ever.0;
            eb += o.ever.1;
            seed_sightings += o.threat_sightings;
            seed_casts += o.sight_casts;
        }
        let sa = 100.0 * b.animals as f64 / (a.animals + b.animals).max(1) as f64;
        let sc = 100.0 * b.cells as f64 / (a.cells + b.cells).max(1) as f64;
        share_animals.push(sa);
        share_cells.push(sc);
        total_threat_sightings += seed_sightings;
        total_sight_casts += seed_casts;
        // `seed_sightings`/`seed_casts` sit beside `a.animals`/`b.animals` on
        // this same line deliberately -- see `Outcome::threat_sightings`'s
        // doc: the sighting count is whole-box, and the animal counts beside
        // it are how a reader tells "one arm was extinct while the other did
        // the seeing" from "both arms were live and it still can't say which".
        let duty = duty_cycle(seed_sightings, seed_casts);
        println!(
            "{seed:>5} {:>8} {:>8} {sa:>8.1}% {:>8} {:>8} {sc:>8.1}% {:>7} {:>7}   (lines surviving A {ea} B {eb})  threat sightings {seed_sightings}/{seed_casts} ({duty})",
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

    // **The positive control `Reports/creature-groups-and-combat-design-
    // 2026-09-06.md` §4a's flight race was run without.** Printed
    // unconditionally, for every `arm=`, because whether the sense saw
    // anything is a property of the BED -- `sight=`, and whatever was
    // placed in the box to see -- not of which arm this particular run
    // happened to race.
    println!(
        "\nthreat sightings across all {seeds} seeds: {total_threat_sightings} of {total_sight_casts} sight casts saw a hunter -- duty cycle {} (\"how often the eye had a hunter to report\", the quantity `Reports/creature-vision-sizing-2026-08-30.md` §3 sized the sight radius from).",
        duty_cycle(total_threat_sightings, total_sight_casts)
    );
    println!(
        "  WHOLE BOX, NOT ONE ARM: both arms share sight=N's one eye, so this total cannot say which arm did the seeing -- only whether the sense saw anything all run. Read it against the per-seed table above: an arm whose A alive/B alive sits at zero could not have been the one looking."
    );
    if total_threat_sightings == 0 {
        println!(
            "\n*** THREAT_SIGHTINGS IS ZERO ACROSS ALL {seeds} SEEDS ({total_sight_casts} sight casts made, not one of them found a hunter). ***"
        );
        println!(
            "*** No animal, on arm A or arm B, ever once saw something that could eat it. A threat-response arm (arm=wire wiring ThreatBearing/ThreatNear) raced against a bed reading this is a null about the BED, not about flight -- no eyes were open (pass sight=N) or nothing threatening was ever in view (pass predators=N). Nothing this run says about flight is interpretable. This is the positive control that §4a's race was run without. ***"
        );
    }
    if arm == Arm::Lethal {
        let (below, _, _) = direction(&share_animals);
        println!(
            "arm=lethal is the MANDATORY negative control: {below} of {} seeds put the zeroed brain behind. {}",
            share_animals.len(),
            if below * 4 >= share_animals.len() * 3 { "The bed has teeth." } else { "THE BED DOES NOT DISCRIMINATE AGAINST A BRAIN WITH NO CONNECTIONS. Nothing measured in it is interpretable." }
        );
    }
}
