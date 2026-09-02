//! **What an individual is, in sentences rather than numbers.**
//!
//! The cell page already prints an individual's genome and explains each row,
//! and that is still the right readout for *"what exactly is this number"*.
//! It is the wrong one for *"what kind of animal is this"*: a column of
//! `GUT BIAS +0.000 / BIRTH GRANT -0.200` and twelve thousand synapse weights
//! is a description only if you already know the answer. The owner asked for
//! the other reading -- *"instead of just a bunch of numbers it could say this
//! creature has this movement pattern and strong dig strength, and no vision"*
//! -- and that is this file.
//!
//! **Three rules it is built on.**
//!
//! **An absence is a sentence.** *"No vision"* is the owner's own example and
//! it is the hardest thing to read off a number, because the number is
//! missing. Two different absences wear that name: the species may have no
//! eyes at all (`sight_range == 0`, which is every shipped species but the
//! beetle), or this individual may have eyes and no live weight from them.
//! Those are different animals and they get different sentences.
//!
//! **A species' capability is never described as an individual's.** Only
//! three body traits and the brain's weights are heritable per individual.
//! `dig_force`, `sight_range`, `crop_capacity` and the rest belong to every
//! member of the species alike, and a page implying otherwise would be
//! claiming a gene that does not exist -- which, on a page whose whole
//! purpose is telling two individuals apart, is the worst thing it could say.
//! Everything from that half is prefixed with the species' name.
//!
//! **It works off the genetics, not off a live organism**, so the same
//! sentences describe a jar on the shelf. That is where the question
//! *"what did I keep?"* is actually asked, and a shelf full of names is not
//! an answer.

use crate::sim::brain::{self, BrainInput, BrainOutput};
use crate::sim::organism::{self, SpeciesId};
use crate::sim::specimen::Genetics;
use crate::sim::world::World;

/// One line of the readout: what it says, and the number it came from.
///
/// The number is carried rather than dropped because the page is a *summary*
/// and not a replacement -- a player who wants to know why it said "digs
/// hard" gets the weight, in the hover note, without leaving the sentence.
#[derive(Clone, Debug)]
pub struct Phrase {
    pub text: String,
    pub detail: String,
}

impl Phrase {
    fn new(text: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { text: text.into(), detail: detail.into() }
    }
}

/// **Below this share of the individual's own strongest drive, a connection
/// is not worth a sentence.**
///
/// A share rather than an absolute, because the two layers of this genome are
/// on different scales by design: the ant's authored direct weights run
/// 0.2-2.5 and its hidden-layer weights 6-75. One absolute cut-off would
/// either drown the page in the hidden layer or say nothing about the direct
/// one.
const WORTH_SAYING: f32 = 0.25;

/// **How wide a sentence may be.**
///
/// The cell page sizes itself to its widest row and is then clamped onto the
/// screen, so a long sentence does not wrap -- it makes the whole page wider
/// and slides it left over whatever it was opened from. Measured: a
/// thirty-character phrase pushed the page to 250 px and hid three of the
/// roster's eight columns behind it.
///
/// So the sentences are written to fit, and `every_phrase_fits_the_column`
/// holds them to it. Twenty-six characters is 155 px, which is about what the
/// numeric rows beside them already run to.
pub const PHRASE_COLUMNS: usize = 26;

/// **At most this many drives, strongest first.**
///
/// A cap on the *page*, not on the reading: everything below the cut is still
/// in the numeric `GENOME` group two headings down, and the share is printed
/// beside each phrase so a reader can see where the list was cut. The shipped
/// ant has eight direct weights above `WORTH_SAYING`, and eight sentences is
/// a paragraph rather than a summary -- which is the thing this page exists
/// not to be.
const MOST_DRIVES: usize = 5;

/// **What a connection from `input` to `output` makes an animal do.**
///
/// Two sentences per pair, because **the sign is a different behaviour and
/// not a weaker one**: a positive `Crowding -> Move` is an animal that hurries
/// when hemmed in, a negative one is an animal that stops. Reading a negative
/// weight as "a bit less of the positive sentence" would describe the
/// opposite animal.
///
/// Pairs with no entry fall through to `generic`, which names both ends
/// rather than inventing a sentence for a combination nobody has thought
/// about. That is deliberate: a wrong sentence is worse than a plain one.
fn phrasebook(input: BrainInput, output: BrainOutput) -> Option<(&'static str, &'static str)> {
    use BrainInput as I;
    use BrainOutput as O;
    Some(match (input, output) {
        // **The two pheromone channels are not here, and that is the point.**
        // `brain.rs` gives A and B no meaning whatever -- they are two
        // anonymous planes, and what a channel *means* is decided entirely by
        // which weights emit onto it. Naming A "the food trail" in a table
        // like this one is an assumption dressed as a fact, and for the
        // shipped ant it is probably backwards: every ant lays A all the time
        // (`Bias -> EmitA`), which pools it wherever the colony is, while
        // only a laden ant lays B (`Carrying -> EmitB`), which marks the way
        // back from food. `scent_phrase` derives the label from the
        // individual's own emissions instead.

        // -- food.
        (I::FoodAdjacent, O::Move) => ("HURRIES PAST FOOD", "STOPS DEAD ON FOOD"),
        (I::FoodAdjacent, O::Feed) => ("EATS WHAT IT WALKS INTO", "WALKS PAST FOOD UNFED"),
        (I::FoodAdjacent, O::Dig) => ("DIGS INTO FOOD IT FINDS", "WILL NOT DIG AT FOOD"),
        (I::Bias, O::Feed) => ("EATS BY DEFAULT", "RARELY EATS"),
        (I::Energy, O::Feed) => ("EATS MOST WHEN FED", "EATS MOST WHEN HUNGRY"),
        (I::Energy, O::Move) => ("TRAVELS WHILE IT CAN", "TRAVELS ONLY WHEN HUNGRY"),
        (I::Energy, O::Dig) => ("DIGS WHILE WELL FED", "DIGS WHEN HUNGRY"),

        // -- the nest, and the round trip.
        (I::AtNest, O::Drop) => ("UNLOADS AT THE NEST", "CARRIES PAST THE NEST"),
        (I::AtNest, O::Move) => ("LEAVES THE NEST AT ONCE", "SETTLES AT THE NEST"),
        (I::Carrying, O::Drop) => ("DROPS WHAT IT PICKS UP", "HOLDS ON TO ITS LOAD"),
        (I::Carrying, O::Move) => ("CARRIES ITS LOAD FAST", "SLOWS DOWN WHEN LADEN"),

        // -- digging.
        (I::Bias, O::Dig) => ("DIGS BY DEFAULT", "AVOIDS DIGGING"),
        (I::Crowding, O::Dig) => ("DIGS WHERE THE CROWD IS", "DIGS ONLY WHEN ALONE"),

        // -- crowding, the negative-feedback term.
        (I::Crowding, O::Move) => ("PUSHES THROUGH A CROWD", "STOPS WHEN HEMMED IN"),
        (I::Crowding, O::Turn) => ("TURNS INTO A CROWD", "TURNS OUT OF A CROWD"),
        (I::Crowding, O::Tumble) => ("RE-AIMS WHEN CROWDED", "HOLDS LINE WHEN CROWDED"),

        // -- the weather and the ground.
        (I::TempAboveAmb, O::Turn) => ("TURNS TOWARD WARMTH", "TURNS AWAY FROM HEAT"),
        (I::TempAboveAmb, O::Move) => ("MOVES FASTER WHEN WARM", "SLOWS DOWN WHEN WARM"),
        (I::MoistureFront, _) => ("SEEKS DAMP GROUND", "AVOIDS DAMP GROUND"),
        (I::MoistureLateral, O::Turn) => ("STEERS TOWARD DAMP", "STEERS AWAY FROM DAMP"),
        (I::LightHere, O::Move) => ("TRAVELS IN THE LIGHT", "TRAVELS IN THE DARK"),
        (I::LightHere, O::Turn) => ("TURNS TOWARD THE LIGHT", "TURNS AWAY FROM LIGHT"),

        // -- hunting. The only distal sense in this scaffold.
        (I::PreyNear, O::Move) => ("CHARGES PREY IT SEES", "BACKS OFF FROM PREY"),
        (I::PreyNear, O::Feed) => ("ATTACKS PREY IT SEES", "WILL NOT ATTACK PREY"),
        (I::PreyNear, O::Persist) => ("LOCKS ON TO PREY", "BREAKS OFF FROM PREY"),
        (I::PreyBearing, O::Turn) => ("TURNS TO FACE ITS PREY", "TURNS AWAY FROM PREY"),
        (I::PreyNear, O::Impulse) => ("POUNCES", "STAYS DOWN NEAR PREY"),

        // -- how it walks, which is the half that was hardcoded until S13.
        (I::Bias, O::Move) => ("WALKS RATHER THAN WAITS", "WAITS RATHER THAN WALKS"),
        (I::Bias, O::Persist) => ("COMMUTES IN STRAIGHT LINES", "MILLS ABOUT"),
        (I::Bias, O::Tumble) => ("RE-AIMS WHEN BLOCKED", "SHOVES AT AN OBSTACLE"),
        (I::Bias, O::Caution) => ("KEEPS ITS FEET ON GROUND", "WALKS OUT OVER OPEN AIR"),
        (I::Bias, O::Turn) => ("VEERS CONSTANTLY", "HOLDS ONE HEADING"),
        (I::Bias, O::Drop) => ("PUTS THINGS DOWN ANYWHERE", "NEVER LETS GO"),
        (I::Bias, O::Impulse) => ("JUMPS BY DEFAULT", "STAYS ON THE GROUND"),
        _ => return None,
    })
}

/// **What this animal's two scent channels mean to it.**
///
/// `brain.rs` deliberately says nothing about A and B: they are two planes
/// with no semantics, and a channel becomes "the food trail" or "the colony
/// scent" only because of who lays it and when. So the label is read off this
/// individual's own emit weights rather than written down here.
///
/// The reasoning per case, and it is the same reasoning a player would use
/// watching the box: a channel only laid **while carrying** is laid by
/// animals walking away from food, so it marks the way back to it. One laid
/// **all the time** pools wherever the animals are, which is around the nest.
/// One laid **at the nest** marks the nest. One nobody lays is still
/// followable and has no name.
fn scent_label(w: &brain::Wiring, channel: BrainOutput) -> &'static str {
    let mut best: Option<(BrainInput, f32)> = None;
    for i in w.instincts.iter().filter(|i| i.1 == channel && i.2.abs() > brain::W_EPS) {
        if best.is_none_or(|(_, m)| i.2.abs() > m) {
            best = Some((i.0, i.2.abs()));
        }
    }
    match best {
        Some((BrainInput::Carrying, _)) => "FOOD ROUTE",
        Some((BrainInput::AtNest, _)) => "NEST MARK",
        Some((BrainInput::Bias, _)) => "HOME SCENT",
        Some(_) => "A SCENT",
        None => "A SCENT",
    }
}

/// The sentence for a pheromone weight, with the channel named for what this
/// animal actually uses it for.
fn scent_phrase(w: &brain::Wiring, input: BrainInput, output: BrainOutput) -> Option<(String, String)> {
    use BrainInput as I;
    use BrainOutput as O;
    let channel = match input {
        I::PheroAFront | I::PheroALateral | I::PheroAAlong => O::EmitA,
        I::PheroBFront | I::PheroBLateral | I::PheroBAlong => O::EmitB,
        _ => {
            // The other half: laying it, rather than following it.
            return match (input, output) {
                (_, O::EmitA) | (_, O::EmitB) => {
                    let name = scent_label(w, output);
                    // The condition leads, matching the gated sentences the
                    // hidden layer produces, so the two read as one list.
                    let who = match input {
                        I::Bias => "ALWAYS LAYS".to_string(),
                        I::Carrying => "LADEN: LAYS".to_string(),
                        I::AtNest => "AT NEST: LAYS".to_string(),
                        I::FoodAdjacent => "ON FOOD: LAYS".to_string(),
                        // A rare driver gets no condition rather than a
                        // twenty-nine character one: the explanation carries
                        // it in full, and an over-wide row widens the page.
                        _ => "SOMETIMES LAYS".to_string(),
                    };
                    Some((format!("{who} {name}"), format!("STOPS LAYING {name}")))
                }
                _ => None,
            };
        }
    };
    let name = scent_label(w, channel);
    Some(match output {
        O::Move => (format!("FOLLOWS {name}"), format!("AVOIDS {name}")),
        O::Turn => (format!("STEERS TO {name}"), format!("STEERS OFF {name}")),
        _ => (format!("{name} > {}", brain::OUTPUT_NAMES[output as usize].to_uppercase()), format!("{name} < {}", brain::OUTPUT_NAMES[output as usize].to_uppercase())),
    })
}

/// **One conditional behaviour, decoded out of a hidden unit.**
struct Gated {
    /// The condition, already in words: `LADEN`, `EMPTY`, `AT NEST`.
    when: &'static str,
    sensor: BrainInput,
    output: BrainOutput,
    /// Sign of the sensor's effect on the output through this unit.
    effect: f32,
}

/// **Inputs that read as a state rather than as a measurement**, and the two
/// words for being in that state or not.
///
/// A hidden unit gated on one of these is the engine's way of saying "only
/// while ...", and these are the only inputs that make sense as a condition:
/// they are the 0-or-1 and bank-fraction ones. A unit gated on a *gradient*
/// is not a conditional, it is a second-order term, and this reports nothing
/// about it rather than inventing a condition it does not have.
fn gate_words(input: BrainInput) -> Option<(&'static str, &'static str)> {
    Some(match input {
        BrainInput::Carrying => ("LADEN", "EMPTY"),
        BrainInput::AtNest => ("AT NEST", "AWAY FROM NEST"),
        BrainInput::FoodAdjacent => ("ON FOOD", "OFF FOOD"),
        BrainInput::Energy => ("WELL FED", "HUNGRY"),
        BrainInput::Crowding => ("IN A CROWD", "ALONE"),
        _ => return None,
    })
}

/// **How much larger the gate has to be than the sensor beside it.**
///
/// A unit whose two inputs are comparable is mixing them, not gating on one.
/// The ant's are 75 against 6 -- twelve and a half times -- so this is not a
/// tight fit; it is there to stop a random genome being described as a
/// conditional it does not implement.
const GATE_DOMINANCE: f32 = 3.0;

/// Read the hidden layer as conditionals.
///
/// **The push-pull pairs collapse to one sentence.** The ant wires each
/// condition twice with opposite signs on both the sensor and the output --
/// `PheroAAlong +6 -> h0 -> Move +2.5` beside `-6 -> h1 -> Move -2.5` -- which
/// is one gradient-follower built out of two half-wave units, not two
/// behaviours. Reporting both would say the same thing twice and read as a
/// contradiction, since one of the pair has both signs negative.
fn gated(w: &brain::Wiring) -> Vec<Gated> {
    let mut out: Vec<Gated> = Vec::new();
    for unit in 0..brain::BRAIN_HIDDEN as u8 {
        let outs: Vec<&brain::OutputWire> = w.outputs.iter().filter(|o| o.0 == unit && o.2.abs() > brain::W_EPS).collect();
        if outs.is_empty() {
            continue;
        }
        let ins: Vec<&brain::HiddenWire> = w.hidden.iter().filter(|h| h.1 == unit && h.2.abs() > brain::W_EPS).collect();
        let bias: f32 = ins.iter().filter(|h| h.0 == BrainInput::Bias).map(|h| h.2).sum();
        let mut rest: Vec<&brain::HiddenWire> = ins.iter().copied().filter(|h| h.0 != BrainInput::Bias).collect();
        rest.sort_by(|a, b| b.2.abs().total_cmp(&a.2.abs()));
        let (Some(gate), Some(sensor)) = (rest.first(), rest.get(1)) else { continue };
        if gate.2.abs() < sensor.2.abs() * GATE_DOMINANCE {
            continue;
        }
        let Some((yes, no)) = gate_words(gate.0) else { continue };
        // **The gate only gates if it changes the sign of the unit's own
        // resting level.** `bias + gate` against `bias`: the ant's h0 sits at
        // -45 and reaches +30 while carrying, so carrying is what switches it
        // on. A unit already on either way is not conditional on anything.
        let on_with = bias + gate.2 > 0.0;
        let on_without = bias > 0.0;
        if on_with == on_without {
            continue;
        }
        let when = if on_with { yes } else { no };
        for o in outs {
            let effect = sensor.2 * o.2;
            let already = out.iter().any(|g| {
                g.when == when && g.sensor == sensor.0 && g.output == o.1 && (g.effect >= 0.0) == (effect >= 0.0)
            });
            if !already {
                out.push(Gated { when, sensor: sensor.0, output: o.1, effect });
            }
        }
    }
    out
}

/// A last-resort sentence naming both ends, for a pair the phrasebook has no
/// opinion about. Plain rather than invented -- see `phrasebook`'s doc.
fn generic(input: BrainInput, output: BrainOutput) -> (String, String) {
    let i = brain::INPUT_NAMES[input as usize].to_uppercase();
    let o = brain::OUTPUT_NAMES[output as usize].to_uppercase();
    // **`>` and `<`, because the English form does not fit.** "LESS EMITB
    // WHEN MOISTURELATERAL IS HIGH" is thirty-nine characters against a
    // twenty-six character column, and an over-wide row widens the whole cell
    // page rather than wrapping. The longest pair the scaffold can produce is
    // MOISTURELATERAL (15) against IMPULSE (7), which is 25 with the arrow --
    // so this form always fits, and `every_phrase_fits_the_column` proves it
    // over every combination rather than over the ones I thought of.
    (format!("{i} > {o}"), format!("{i} < {o}"))
}

/// **Describe the individual `id` in `world`.**
pub fn describe(world: &World, id: u16) -> Vec<Phrase> {
    let Some(state) = world.organism(id) else { return Vec::new() };
    let species = state.species;
    if world.species.get(species).creature.is_some() {
        creature(world, species, &state.genome, &state.traits)
    } else {
        plant(world, species, &state.alleles, &state.genotype_draws)
    }
}

/// The same, off a jar rather than a live organism -- so the shelf can answer
/// *"what did I keep?"* in the same words.
pub fn describe_genetics(world: &World, species: SpeciesId, g: &Genetics) -> Vec<Phrase> {
    match g {
        Genetics::Creature(c) => {
            let genome = brain::genome_from_wiring(&c.instincts, &c.hidden, &c.outputs, &c.recurrence);
            creature(world, species, &genome, &c.traits)
        }
        Genetics::Plant(p) => {
            let mut alleles = [0u8; organism::DISCRETE_LOCI];
            for (i, a) in p.alleles.iter().enumerate().take(alleles.len()) {
                alleles[i] = *a;
            }
            let mut draws = [0.0f32; organism::GENOTYPE_TRAITS];
            for (i, d) in p.draws.iter().enumerate().take(draws.len()) {
                draws[i] = *d;
            }
            plant(world, species, &alleles, &draws)
        }
    }
}

// ----------------------------------------------------------------- animals

fn creature(world: &World, species: SpeciesId, genome: &[f32], traits: &[f32]) -> Vec<Phrase> {
    let def = world.species.get(species);
    let Some(c) = def.creature.as_ref() else { return Vec::new() };
    let name = def.name.to_uppercase();
    let mut out = Vec::new();

    // -- diet, which is the one body gene that reads as a kind of animal.
    let gut = traits.get(organism::TRAIT_GUT_BIAS).copied().unwrap_or(0.0);
    out.push(Phrase::new(
        if gut <= -0.4 {
            "EATS PLANTS"
        } else if gut >= 0.4 {
            "EATS FLESH"
        } else {
            "EATS ANYTHING"
        },
        format!("GUT BIAS {gut:+.2}, ON A SCALE FROM -1 PLANT MATTER TO +1 FLESH. INHERITED WITH JITTER, SO IT IS THIS ANIMAL'S OWN AND NOT ITS SPECIES'."),
    ));

    // -- how it breeds. Two genes, one sentence each, because they are
    //    different decisions: when to have young, and what to send them out
    //    with.
    let at = traits.get(organism::TRAIT_REPRODUCE_AT).copied().unwrap_or(0.0);
    out.push(Phrase::new(
        if at <= -0.4 {
            "BREEDS AS SOON AS IT CAN"
        } else if at >= 0.4 {
            "HOARDS BEFORE BREEDING"
        } else {
            "BREEDS AT ITS OWN BAR"
        },
        format!("REPRODUCE-AT {at:+.2}: HOW MUCH WEALTH IT INSISTS ON BEFORE BUDDING, FROM THE EARLIEST THE ARITHMETIC ALLOWS TO TWICE ITS SPECIES' THRESHOLD."),
    ));
    let grant = traits.get(organism::TRAIT_BIRTH_GRANT).copied().unwrap_or(0.0);
    out.push(Phrase::new(
        if grant <= -0.4 {
            "YOUNG GET ALMOST NOTHING"
        } else if grant >= 0.4 {
            "PROVISIONS ITS YOUNG WELL"
        } else {
            "MIDDLING START FOR YOUNG"
        },
        format!("BIRTH GRANT {grant:+.2}: THE SHARE OF A FULL STARTING BUDGET A NEWBORN GETS OUT OF THIS ONE'S OWN BANK. GENEROUS YOUNG COST THE PARENT AND SURVIVE BETTER."),
    ));

    // -- what it can sense at all. **The species half, and it is labelled.**
    if c.sight_range <= 0 {
        out.push(Phrase::new(
            format!("{name}S HAVE NO EYES"),
            "SIGHT RANGE 0 FOR THIS SPECIES: IT CANNOT SEE ANOTHER ANIMAL AT ANY DISTANCE, AND NO AMOUNT OF BREEDING WILL GIVE IT EYES -- SIGHT RANGE IS THE SPECIES' AND NOT THE INDIVIDUAL'S. IT STILL SMELLS TRAILS, FEELS CROWDING AND TOUCHES FOOD IT WALKS INTO.".to_string(),
        ));
    } else {
        out.push(Phrase::new(
            format!("{}S SEE {} CELLS", name, c.sight_range),
            format!("SIGHT RANGE {} FOR THIS SPECIES, CAST ALL ROUND. THE ONLY SENSE IN THIS ENGINE THAT REPORTS ANOTHER ANIMAL AT A DISTANCE.", c.sight_range),
        ));
    }

    // -- the brain, as behaviour.
    //
    // **Guarded, because an organism can exist without one.**
    // `World::push_organism` allocates the slot and `place_creature` fills in
    // the genome afterwards, so there is a window -- and a test that pushes a
    // bare organism sits in it permanently. `wiring_from_genome` *asserts* its
    // length, by design, so an unguarded call here turns a readout into a
    // panic. A page must be able to describe a half-built thing.
    if genome.len() != brain::GENOME_LEN {
        out.push(Phrase::new(
            "NO BRAIN YET",
            format!("THIS ANIMAL HAS {} SYNAPSE WEIGHTS WHERE THE SCAFFOLD HAS {}. IT HAS BEEN ALLOCATED AND NOT YET FILLED IN, WHICH LASTS ONE TICK IN A RUNNING BOX.", genome.len(), brain::GENOME_LEN),
        ));
        return out;
    }
    let wiring = brain::wiring_from_genome(genome);
    let peak = wiring.instincts.iter().map(|i| i.2.abs()).fold(0.0f32, f32::max);
    if peak <= brain::W_EPS {
        out.push(Phrase::new(
            "HAS NO INSTINCTS AT ALL",
            "EVERY DIRECT WEIGHT IN THIS BRAIN IS BELOW THE ENGINE'S OWN NO-CONNECTION THRESHOLD, SO NOTHING IT SENSES REACHES ANYTHING IT DOES. IT WILL STAND STILL.".to_string(),
        ));
    } else {
        let mut ranked: Vec<&brain::Instinct> =
            wiring.instincts.iter().filter(|i| i.2.abs() > brain::W_EPS).collect();
        // Sorted by strength, ties broken on the slot pair so the order is
        // total: the page is re-rendered every frame and a list that
        // reshuffled under the cursor would be unreadable.
        ranked.sort_by(|a, b| {
            b.2.abs()
                .total_cmp(&a.2.abs())
                .then((a.0 as usize).cmp(&(b.0 as usize)))
                .then((a.1 as usize).cmp(&(b.1 as usize)))
        });
        let mut said = 0;
        for inst in ranked {
            let share = inst.2.abs() / peak;
            if share < WORTH_SAYING || said >= MOST_DRIVES {
                continue;
            }
            said += 1;
            let (pos, neg) = scent_phrase(&wiring, inst.0, inst.1)
                .or_else(|| phrasebook(inst.0, inst.1).map(|(p, n)| (p.to_string(), n.to_string())))
                .unwrap_or_else(|| generic(inst.0, inst.1));
            let body = if inst.2 >= 0.0 { pos } else { neg };
            out.push(Phrase::new(
                body,
                format!(
                    "{} TO {}, WEIGHT {:+.2}. THAT IS {:.0}% OF THIS ANIMAL'S STRONGEST DRIVE. THE SIGN IS A DIFFERENT BEHAVIOUR AND NOT A WEAKER ONE.",
                    brain::INPUT_NAMES[inst.0 as usize].to_uppercase(),
                    brain::OUTPUT_NAMES[inst.1 as usize].to_uppercase(),
                    inst.2,
                    share * 100.0
                ),
            ));
        }
    }

    // -- **what the hidden layer does, which is where the conditionals
    //    live.** Skipping it was the summary's biggest inaccuracy: the ant's
    //    twelve direct weights are all unconditional, and the *whole foraging
    //    loop* -- follow the food trail while laden, follow the nest scent
    //    while empty -- is twelve hidden weights this page used to say
    //    nothing about. A summary silent on the most important thing an
    //    animal does is worse than a short one.
    for g in gated(&wiring) {
        let (pos, neg) = scent_phrase(&wiring, g.sensor, g.output)
            .or_else(|| phrasebook(g.sensor, g.output).map(|(p, n)| (p.to_string(), n.to_string())))
            .unwrap_or_else(|| generic(g.sensor, g.output));
        let body = if g.effect >= 0.0 { pos } else { neg };
        let full = format!("{}: {body}", g.when);
        // The gate is dropped rather than wrapped when the pair is a long
        // one -- the behaviour phrase alone is still true, just less
        // specific, and the explanation always carries the whole condition.
        let text = if full.chars().count() <= PHRASE_COLUMNS { full } else { body.clone() };
        out.push(Phrase::new(
            text,
            format!(
                "THROUGH A HIDDEN UNIT: WHILE {}, {} DRIVES {} ({}). THE DIRECT WEIGHTS ABOVE APPLY ALL THE TIME; THIS ONE ONLY WHILE THAT CONDITION HOLDS, WHICH IS WHAT A HIDDEN UNIT IS FOR.",
                g.when,
                brain::INPUT_NAMES[g.sensor as usize].to_uppercase(),
                brain::OUTPUT_NAMES[g.output as usize].to_uppercase(),
                if g.effect >= 0.0 { "UP" } else { "DOWN" },
            ),
        ));
    }

    // -- **the absences, which are half the value.** A sense the animal has
    //    and does not use is a different animal from one that cannot sense at
    //    all, and only this half can say so.
    let uses = |input: BrainInput| -> bool {
        wiring.instincts.iter().any(|i| i.0 == input && i.2.abs() > brain::W_EPS)
            || wiring.hidden.iter().any(|h| h.0 == input && h.2.abs() > brain::W_EPS)
    };
    if c.sight_range > 0 && !uses(BrainInput::PreyNear) && !uses(BrainInput::PreyBearing) {
        out.push(Phrase::new(
            "HAS EYES, IGNORES THEM",
            "ITS SPECIES CASTS A SIGHT RAY AND THIS INDIVIDUAL'S BRAIN CARRIES NO LIVE WEIGHT FROM EITHER PREY INPUT. THE SENSE IS PAID FOR AND WIRED TO NOTHING -- A HUNTER THAT CANNOT ACT ON WHAT IT SEES.".to_string(),
        ));
    }
    if !uses(BrainInput::PheroAAlong) && !uses(BrainInput::PheroAFront) && !uses(BrainInput::PheroALateral) {
        out.push(Phrase::new(
            "CANNOT FOLLOW A TRAIL",
            "NO LIVE WEIGHT FROM ANY FOOD-SCENT INPUT. IT MAY STILL LAY ONE FOR OTHERS -- LAYING AND FOLLOWING ARE DIFFERENT WEIGHTS -- BUT IT WILL NOT FIND ITS WAY BACK ALONG IT.".to_string(),
        ));
    }
    if !uses(BrainInput::Crowding) {
        out.push(Phrase::new(
            "DOES NOT NOTICE A CROWD",
            "NO LIVE WEIGHT FROM CROWDING, WHICH IS THE NEGATIVE-FEEDBACK TERM. WITHOUT IT A COLONY SETTLES ON THE FIRST PATH IT FINDS AND NEVER LOOKS FOR A BETTER ONE.".to_string(),
        ));
    }
    if !uses(BrainInput::Energy) {
        out.push(Phrase::new(
            "CANNOT FEEL HUNGER",
            "NO LIVE WEIGHT FROM ENERGY: NOTHING IT DOES CHANGES AS ITS BANK FALLS. IT WILL FORAGE THE SAME WAY FULL AND STARVING.".to_string(),
        ));
    }

    // -- the rest of the body, all of it the species'.
    out.push(Phrase::new(
        format!("{name}S {}", dig_word(c.dig_force)),
        format!("DIG FORCE {:.2} FOR THIS SPECIES -- HOW HARD GROUND IT CAN SHIFT. IT IS THE SPECIES' AND NOT THIS INDIVIDUAL'S: BREEDING CANNOT MOVE IT.", c.dig_force),
    ));
    if c.eats_kin {
        out.push(Phrase::new(
            format!("{name}S EAT THEIR OWN KIND"),
            "THE SPECIES' OWN FOOD LIST INCLUDES ITSELF, SO A CROWDED BOX CAN TURN CANNIBAL WITHOUT ANYTHING NEW BEING INTRODUCED.".to_string(),
        ));
    }
    out
}

fn dig_word(force: f32) -> &'static str {
    if force <= 0.0 {
        "CANNOT DIG"
    } else if force < 0.5 {
        "SCRAPE WEAKLY"
    } else if force < 1.5 {
        "DIG NORMALLY"
    } else {
        "DIG HARD"
    }
}

// ------------------------------------------------------------------ plants

fn plant(world: &World, species: SpeciesId, alleles: &[u8], draws: &[f32]) -> Vec<Phrase> {
    let mut out = Vec::new();

    // -- the six jumping genes. **Categorical, not scalar**: two plants that
    //    differ here are different shapes, not the same shape at different
    //    sizes, which is exactly what a sentence can say and a multiplier
    //    cannot.
    for (locus, table, note) in [
        (
            organism::LOCUS_LEAF_ECONOMY,
            &["GROWS FAST, SPENDS WATER", "GROWS SLOW, HOARDS WATER"][..],
            "LEAF ECONOMY: THE ACQUISITIVE ALLELE PHOTOSYNTHESISES HARDER AND TRANSPIRES MORE, THE CONSERVATIVE ONE DOES LESS OF BOTH. IT DECIDES WHETHER THIS PLANT WINS A GOOD YEAR OR SURVIVES A BAD ONE.",
        ),
        (
            organism::LOCUS_BRANCH_ANGLE,
            &["NARROW AND UPRIGHT", "BRANCHES AT ITS OWN ANGLE", "SPLAYED WIDE"][..],
            "BRANCH ANGLE: HOW FAR A CHILD SHOOT LEAVES ITS PARENT'S LINE. NARROW STACKS LEAF OVER LEAF AND SHADES ITSELF; SPLAYED CATCHES MORE LIGHT AND CARRIES MORE LOAD OUT ON A LEVER.",
        ),
        (
            organism::LOCUS_INTERNODE,
            &["TWIGGY, WITH SHORT RUNS", "RUNS AT ITS OWN LENGTH", "LONG CLEAN RUNS"][..],
            "INTERNODE: HOW FAR IT GROWS BETWEEN ONE BRANCHING POINT AND THE NEXT. SHORT RUNS MAKE A DENSE BUSH, LONG ONES A BARE STEM THAT REACHES.",
        ),
        (
            organism::LOCUS_SYMPODIAL,
            &["KEEPS ONE LEADING SHOOT", "FORKS INSTEAD OF LEADING"][..],
            "SYMPODIAL: WHETHER ONE SHOOT STAYS IN CHARGE OR EACH TIP HANDS OVER TO ITS OWN CHILDREN. A LEADER MAKES A TREE SHAPE, FORKING MAKES A SHRUB.",
        ),
        (
            organism::LOCUS_TROPISM,
            &["CLIMBS", "SPREADS SIDEWAYS"][..],
            "TROPISM: WHETHER A SHOOT SCORES UPWARDS OR ALONG. CLIMBING WINS THE LIGHT ABOVE IT; SPREADING TAKES GROUND BEFORE ANYTHING ELSE DOES.",
        ),
        (
            organism::LOCUS_WOOD_DENSITY,
            &["PIONEER WOOD: CHEAP, WEAK", "WOOD AT ITS OWN DENSITY", "DENSE WOOD: STRONG, DEAR"][..],
            "WOOD DENSITY: IT SCALES BOTH WHAT A CANTILEVER CAN CARRY AND WHAT A CELL COSTS TO BUILD, SO IT IS A REAL TRADE AND NOT A FREE STRENGTH KNOB.",
        ),
    ] {
        let a = alleles.get(locus).copied().unwrap_or(0) as usize;
        if let Some(word) = table.get(a) {
            out.push(Phrase::new(*word, format!("{note} ALLELE {a} OF {}.", table.len())));
        }
    }

    // -- the continuous genome, **only where its species gives the slot a
    //    width**. A slot with no variance is a draw with no consumer for this
    //    species, and printing it would be nine rows of noise around
    //    whichever two actually vary.
    for (slot, label, more, less) in PLANT_DRAWS {
        let Some(width) = crate::lab::params::genotype_variance_of(world, species, *slot) else { continue };
        if width <= 0.0 {
            continue;
        }
        let draw = draws.get(*slot).copied().unwrap_or(0.0);
        let factor = 1.0 + draw * width;
        // A tenth either way is inside the noise of what a plant grows into
        // anyway, so it is not worth a sentence. The threshold is on the
        // *expressed* multiplier rather than on the draw, because a wide slot
        // and a narrow one at the same draw are different plants.
        if (factor - 1.0).abs() < 0.1 {
            continue;
        }
        let word = if factor > 1.0 { more } else { less };
        out.push(Phrase::new(
            format!("{word} ({:+.0}%)", (factor - 1.0) * 100.0),
            format!("{label}, DRAWN WHEN THIS SEED GERMINATED AND CARRIED FOR LIFE. ITS SPECIES ALLOWS UP TO {:.0}% EITHER WAY, AND THIS ONE DREW {:+.0}%. THIS IS WHY TWO SEEDS OF ONE SPECIES DO NOT GROW INTO THE SAME PLANT.", width * 100.0, (factor - 1.0) * 100.0),
        ));
    }

    if out.len() <= organism::DISCRETE_LOCI {
        out.push(Phrase::new(
            "OTHERWISE TYPICAL",
            "EVERY CONTINUOUS GENE ITS SPECIES LETS VARY CAME OUT WITHIN A TENTH OF THE SPECIES VALUE, SO ITS SHAPE GENES ABOVE ARE THE WHOLE OF WHAT MAKES IT ITSELF.".to_string(),
        ));
    }
    out
}

/// The continuous slots worth a sentence, and the sentence at each end.
///
/// **Not all ten.** `GENOTYPE_SLOTS` in `params.rs` names every slot for the
/// numeric readout; this names the ones a player can act on, in the words
/// `wiki/plants.md` uses for them. A slot with no consumer yet (strain
/// response) is deliberately absent: a sentence about a gene nothing reads
/// would be the page's own version of a channel with a writer and no reader.
const PLANT_DRAWS: &[(usize, &str, &str, &str)] = &[
    (0, "SHOOT BRANCHING", "BRANCHES MORE", "BRANCHES LESS"),
    (1, "ROOT BRANCHING", "FINER ROOTS", "COARSER ROOTS"),
    (2, "SHOOT PLASTOCHRON", "SHOOTS SLOWLY", "SHOOTS QUICKLY"),
    (3, "TURGOR PER CELL", "HOLDS MORE WATER", "HOLDS LESS WATER"),
    (4, "PIPE RATIO", "THICKER STEMS", "THINNER STEMS"),
    (5, "ROOT TROPISM", "STEERS ROOTS HARD", "ROOTS WANDER"),
    (6, "ROOT:SHOOT BIAS", "ROOT OVER SHOOT", "SHOOT OVER ROOT"),
    (7, "STOMATAL CLOSURE", "SHUTS DOWN LATE", "SHUTS DOWN EARLY"),
    (8, "ROOT PENETRATION", "ROOTS PUSH HARDER", "ROOTS PUSH WEAKLY"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::chunk::Rect as WorldRect;

    fn world() -> World {
        World::new(WorldRect::new(0, 0, 63, 63))
    }

    /// A live animal of `name`, carrying its species' authored brain.
    fn animal(world: &mut World, name: &str) -> u16 {
        let species = world.species.id_of(name).unwrap_or_else(|| panic!("{name} must be loaded"));
        let id = world.push_organism(species).expect("slots free");
        let genome = world.species.get(species).genome.clone();
        let traits = world.species.get(species).creature.as_ref().expect("a creature").traits;
        let state = world.organism_mut(id).expect("just pushed");
        state.genome = genome;
        state.traits = traits;
        id
    }

    fn says(phrases: &[Phrase], want: &str) -> bool {
        phrases.iter().any(|p| p.text.contains(want))
    }

    /// **Every phrase class has a positive control**, and each of them is
    /// provable red by deleting the rule it names.
    ///
    /// This is the shape `CLAUDE.md` asks for and the reason it asks: a
    /// phrasebook is a pile of string literals, and a rule that is never
    /// reached produces a page that is merely *shorter* rather than wrong.
    /// Shorter looks fine.
    #[test]
    fn each_phrase_class_fires_on_a_genome_built_to_trigger_it() {
        let mut world = world();
        let id = animal(&mut world, "ant");
        let species = world.organism(id).expect("alive").species;

        // -- the shipped ant, unmodified. Its authored weights are the
        //    positive control for the ordinary case.
        let base = describe(&world, id);
        assert!(!base.is_empty(), "the shipped ant produced no description at all");
        assert!(says(&base, "EATS"), "no diet sentence: {base:#?}");
        assert!(says(&base, "BREEDS"), "no breeding sentence");
        assert!(says(&base, "NO EYES"), "the ant has sight_range 0 and the page did not say so");
        assert!(says(&base, "WALKS RATHER THAN WAITS"), "the ant's strongest Bias->Move weight produced no sentence");
        // The channels are named for what the animal does with them, so the
        // words to look for are those names rather than "TRAIL".
        assert!(says(&base, "HOME SCENT"), "the ant lays a scent everywhere it goes and the page said nothing about it");
        assert!(says(&base, "FOOD ROUTE"), "the ant marks the way back from food and the page said nothing about it");
        assert!(base.len() <= 4 + MOST_DRIVES + 4 + 2, "the summary has grown into a paragraph: {} lines", base.len());

        // -- a drive built to fire. `FoodAdjacent -> Feed` at a weight that
        //    dominates everything else must produce the eating sentence, at
        //    strength.
        let mut w2 = world;
        let genome = brain::genome_from_instincts(&[brain::Instinct(
            BrainInput::FoodAdjacent,
            BrainOutput::Feed,
            9.0,
        )]);
        w2.organism_mut(id).expect("alive").genome = genome;
        let fed = describe(&w2, id);
        assert!(says(&fed, "EATS WHAT IT WALKS INTO"), "a dominant FoodAdjacent->Feed weight said nothing: {fed:#?}");
        // **Rank is carried by order, not by a word.** "-- STRONGLY" cost
        // eleven characters on a twenty-six character line, which is the
        // difference between a page that fits beside the roster and one that
        // covers it. The share is in every row's own explanation instead, and
        // the drives are listed strongest first -- so this asserts the
        // ordering, which is the claim the page actually makes.
        let drives: Vec<f32> = fed
            .iter()
            .filter_map(|p| p.detail.split("THAT IS ").nth(1))
            .filter_map(|t| t.split('%').next())
            .filter_map(|t| t.trim().parse::<f32>().ok())
            .collect();
        assert!(!drives.is_empty(), "no drive carried its share in the explanation");
        assert!(
            drives.windows(2).all(|w| w[0] >= w[1]),
            "the drives are not listed strongest first, which is the only thing saying which one dominates: {drives:?}"
        );
        assert!(drives[0] > 99.0, "the dominant weight did not come out as ~100% of the strongest drive: {drives:?}");

        // -- **the sign is a different sentence, not a weaker one.**
        let genome = brain::genome_from_instincts(&[brain::Instinct(
            BrainInput::FoodAdjacent,
            BrainOutput::Feed,
            -9.0,
        )]);
        w2.organism_mut(id).expect("alive").genome = genome;
        let starved = describe(&w2, id);
        assert!(says(&starved, "WALKS PAST FOOD UNFED"), "a negative weight read as a weak positive: {starved:#?}");
        assert!(!says(&starved, "EATS WHAT IT WALKS INTO"), "both signs of one weight produced the same sentence");

        // -- **the absences.** A brain with one weight in it senses nothing
        //    else, and each absence has its own line.
        assert!(says(&starved, "CANNOT FOLLOW A TRAIL"), "a brain with no scent weights did not say so");
        assert!(says(&starved, "DOES NOT NOTICE A CROWD"), "a brain with no crowding weight did not say so");
        assert!(says(&starved, "CANNOT FEEL HUNGER"), "a brain with no energy weight did not say so");

        // -- **eyes that are wired to nothing are a different animal from no
        //    eyes**, and that distinction is the owner's own example.
        assert!(!says(&starved, "HAS EYES, IGNORES"), "the ant has no eyes, so this sentence must not appear");
        let mut sighted = w2;
        // Give this species eyes. `get_mut` is the registry's own seam and
        // `sight_range` is read live at every use, so nothing needs redrawing.
        sighted
            .species
            .get_mut(species)
            .creature
            .as_mut()
            .expect("an ant is a creature")
            .sight_range = 40;
        let blind_by_choice = describe(&sighted, id);
        assert!(says(&blind_by_choice, "HAS EYES, IGNORES THEM"), "a species with eyes and an individual with no prey weight said nothing");
        assert!(!says(&blind_by_choice, "NO EYES"), "an animal that can see was described as eyeless");

        // ...and one that uses them says the other thing.
        let genome = brain::genome_from_instincts(&[brain::Instinct(BrainInput::PreyNear, BrainOutput::Feed, 5.0)]);
        sighted.organism_mut(id).expect("alive").genome = genome;
        let hunter = describe(&sighted, id);
        assert!(says(&hunter, "ATTACKS PREY IT SEES"), "a live prey weight produced no hunting sentence: {hunter:#?}");
        assert!(!says(&hunter, "HAS EYES, IGNORES"), "an animal acting on what it sees was called blind to it");
    }

    /// **A scent channel is named for what this animal uses it for, and the
    /// engine gives it no name of its own.**
    ///
    /// `brain.rs` says nothing whatever about what channels A and B mean;
    /// they are two anonymous planes and the meaning comes from who lays
    /// them. The first version of this file hard-coded *"A is the food
    /// trail"* and, on the shipped ant, that was **backwards** -- every ant
    /// lays A all the time, which pools it around the nest, while only a
    /// laden ant lays B, which marks the way back from food. The page
    /// therefore described the foraging loop the wrong way round while
    /// reading perfectly.
    #[test]
    fn a_scent_is_named_for_what_the_animal_lays_on_it() {
        use BrainInput as I;
        use BrainOutput as O;
        let mut world = world();
        let id = animal(&mut world, "ant");

        // The shipped ant: A laid unconditionally, B only while carrying.
        let said = describe(&world, id);
        assert!(says(&said, "ALWAYS LAYS HOME SCENT"), "A is laid by every ant all the time and was not named for it: {said:#?}");
        assert!(says(&said, "LADEN: LAYS FOOD ROUTE"), "B is laid only while laden and was not named for it");
        // ...and the loop it implies, which lives entirely in the hidden
        // layer and which this page said nothing about until it read one.
        assert!(says(&said, "LADEN: FOLLOWS HOME SCENT"), "the laden half of the foraging loop is missing");
        assert!(says(&said, "EMPTY: FOLLOWS FOOD ROUTE"), "the empty-handed half of the foraging loop is missing");

        // **Swap which channel is laid how, and the names must swap with
        // it.** This is the control: if the labels were constants rather than
        // derived, this arm would produce the same words as the one above.
        let swapped = brain::genome_from_instincts(&[
            brain::Instinct(I::Bias, O::EmitB, 2.0),
            brain::Instinct(I::Carrying, O::EmitA, 2.5),
            brain::Instinct(I::PheroAAlong, O::Move, 1.0),
        ]);
        world.organism_mut(id).expect("alive").genome = swapped;
        let other = describe(&world, id);
        assert!(says(&other, "ALWAYS LAYS HOME SCENT"), "the always-laid channel is B here and was not named for it: {other:#?}");
        assert!(says(&other, "LADEN: LAYS FOOD ROUTE"), "the laden-laid channel is A here and was not named for it");
        assert!(says(&other, "FOLLOWS FOOD ROUTE"), "following A must now read as the food route, since A is what this animal lays while laden");
        assert!(!says(&other, "FOLLOWS HOME SCENT"), "following A still read as the home scent, so the label is a constant rather than derived");
    }

    /// **The hidden layer is read, and its conditionals are not invented.**
    #[test]
    fn a_hidden_unit_is_reported_as_a_condition_only_when_it_gates() {
        use BrainInput as I;
        use BrainOutput as O;
        let mut world = world();
        let id = animal(&mut world, "ant");

        // A unit whose gate cannot switch it: bias and gate agree in sign, so
        // it is on either way and is not a conditional at all.
        let ungated = brain::genome_from_wiring(
            &[],
            &[brain::HiddenWire(I::Bias, 0, 45.0), brain::HiddenWire(I::Carrying, 0, 75.0), brain::HiddenWire(I::PheroAAlong, 0, 6.0)],
            &[brain::OutputWire(0, O::Move, 2.5)],
            &[],
        );
        world.organism_mut(id).expect("alive").genome = ungated;
        assert!(
            !describe(&world, id).iter().any(|p| p.text.starts_with("LADEN:") || p.text.starts_with("EMPTY:")),
            "a unit that is on whether or not the gate fires was reported as a condition"
        );

        // ...and the same wiring with the bias flipped is a real gate.
        let gated_genome = brain::genome_from_wiring(
            &[],
            &[brain::HiddenWire(I::Bias, 0, -45.0), brain::HiddenWire(I::Carrying, 0, 75.0), brain::HiddenWire(I::PheroAAlong, 0, 6.0)],
            &[brain::OutputWire(0, O::Move, 2.5)],
            &[],
        );
        world.organism_mut(id).expect("alive").genome = gated_genome;
        assert!(
            describe(&world, id).iter().any(|p| p.text.starts_with("LADEN:")),
            "flipping the bias made the gate real and it was still not reported"
        );
    }

    /// **A species capability is never claimed as an individual's.**
    ///
    /// Only three body traits and the brain's weights are heritable per
    /// individual; `dig_force` and `sight_range` belong to every member of
    /// the species alike. On a page whose whole purpose is telling two
    /// individuals apart, claiming a gene that does not exist is the worst
    /// thing it could say -- so every species sentence names the species.
    #[test]
    fn a_species_capability_is_named_as_the_species() {
        let mut world = world();
        let id = animal(&mut world, "ant");
        let name = {
            let species = world.organism(id).expect("alive").species;
            world.species.get(species).name.to_uppercase()
        };
        let phrases = describe(&world, id);
        for marker in ["EYES", "DIG", "SCRAPE"] {
            for p in phrases.iter().filter(|p| p.text.contains(marker)) {
                // Every sentence about a species capability opens with the
                // species' own name; every sentence about a gene does not.
                let about_the_species = p.text.starts_with(&format!("{name}S "));
                let about_a_gene = p.text.contains("DIGS INTO FOOD") || p.text.contains("DIGS BY DEFAULT") || p.text.contains("DIGS WHERE") || p.text.contains("DIGS WHILE") || p.text.contains("DIGS WHEN");
                assert!(
                    about_the_species || about_a_gene,
                    "{:?} mentions a capability without saying whose it is",
                    p.text
                );
            }
        }
        assert!(
            phrases.iter().any(|p| p.text.starts_with(&format!("{name}S "))),
            "no species-scoped sentence at all, so this guard is checking nothing"
        );
    }

    /// **Every phrase is drawable.** `hud::draw_text` renders a character
    /// outside its 5x7 set as a **silent blank**, and `CLAUDE.md` records
    /// that trap as having shipped three times. This module generates strings
    /// rather than authoring all of them, so a species name or a formatted
    /// percentage is a live route for one.
    #[test]
    fn every_phrase_is_drawable() {
        let mut world = world();
        let mut checked = 0;
        let check = |p: &Phrase, checked: &mut usize| {
            for c in p.text.chars() {
                assert!(crate::hud::has_glyph(c), "no glyph for {c:?} in {:?}", p.text);
            }
            for c in p.detail.chars() {
                assert!(crate::hud::has_glyph(c), "no glyph for {c:?} in the note {:?}", p.detail);
            }
            *checked += 1;
        };

        // Every creature species the build ships, at its authored genome.
        let creatures: Vec<&str> = ["ant", "worm", "beetle"]
            .into_iter()
            .filter(|n| world.species.id_of(n).is_some_and(|id| world.species.get(id).creature.is_some()))
            .collect();
        assert!(!creatures.is_empty(), "no creature species loaded, so the animal half of this sweep runs on nothing");
        for name in creatures {
            let id = animal(&mut world, name);
            for p in &describe(&world, id) {
                check(p, &mut checked);
            }
            // ...and at a swept set of random genomes, which is where a
            // generic fallback sentence naming two raw slot names comes from.
            for seed in 0..16u64 {
                world.organism_mut(id).expect("alive").genome = brain::random_genome(brain::sweep_genome_seed(seed));
                for p in &describe(&world, id) {
                    check(p, &mut checked);
                }
            }
        }

        // Every plant species, over every allele combination its loci allow
        // and both ends of every continuous draw -- which is where the
        // percentage formatting is exercised.
        let plants: Vec<u16> = (0..world.species.len() as u16)
            .filter(|i| world.species.get(SpeciesId(*i)).creature.is_none())
            .collect();
        for sp in plants {
            let species = SpeciesId(sp);
            let id = world.push_organism(species).expect("slots free");
            for locus in 0..organism::DISCRETE_LOCI {
                for allele in 0..organism::LOCUS_ALLELES[locus] {
                    for draw in [-1.0f32, 0.0, 1.0] {
                        let state = world.organism_mut(id).expect("just pushed");
                        state.alleles[locus] = allele;
                        state.genotype_draws = [draw; organism::GENOTYPE_TRAITS];
                        for p in &describe(&world, id) {
                            check(p, &mut checked);
                        }
                    }
                }
            }
        }
        assert!(checked > 500, "only {checked} phrases checked -- the sweep is not reaching the generated strings");
    }

    /// A throwaway readout, run with `--nocapture`, that prints the ant's
    /// whole genome beside what the page says about it. Kept because the
    /// question *"is the summary accurate?"* is one a reader will ask again,
    /// and answering it from the source beats answering it from memory.
    #[test]
    #[ignore = "a readout, not an assertion -- cargo test -- --ignored --nocapture the_ants_wiring_against_what_the_page_says"]
    fn the_ants_wiring_against_what_the_page_says() {
        let mut world = world();
        let id = animal(&mut world, "ant");
        let genome = world.organism(id).expect("alive").genome.clone();
        let w = brain::wiring_from_genome(&genome);
        let mut direct: Vec<&brain::Instinct> = w.instincts.iter().collect();
        direct.sort_by(|a, b| b.2.abs().total_cmp(&a.2.abs()));
        println!("== DIRECT ({}) ==", direct.len());
        for i in direct {
            println!("  {:>16} -> {:<8} {:+.2}", brain::INPUT_NAMES[i.0 as usize], brain::OUTPUT_NAMES[i.1 as usize], i.2);
        }
        println!("== INPUT -> HIDDEN ({}) ==", w.hidden.len());
        for h in &w.hidden {
            println!("  {:>16} -> h{}       {:+.2}", brain::INPUT_NAMES[h.0 as usize], h.1, h.2);
        }
        println!("== HIDDEN -> OUTPUT ({}) ==", w.outputs.len());
        for o in &w.outputs {
            println!("  h{} -> {:<8} {:+.2}", o.0, brain::OUTPUT_NAMES[o.1 as usize], o.2);
        }
        println!("== RECURRENCE ({}) ==", w.recurrence.len());
        for r in &w.recurrence {
            println!("  h{} self {:+.2}", r.0, r.1);
        }
        println!("== THE PAGE ==");
        for p in describe(&world, id) {
            println!("  {}", p.text);
        }
    }

    /// **No sentence is wider than the column it is drawn in.**
    ///
    /// The page cannot narrow itself after the fact: `page_rect` sizes to the
    /// widest row and then clamps onto the screen, so an over-wide phrase
    /// slides the whole cell page left over whatever it was opened from. This
    /// is the guard that keeps the phrasebook honest as it grows -- a new
    /// sentence that does not fit fails here rather than on a contact sheet.
    #[test]
    fn every_phrase_fits_the_column() {
        let mut world = world();
        let mut widest = 0usize;
        let mut worst = String::new();
        let check = |phrases: &[Phrase], widest: &mut usize, worst: &mut String| {
            for p in phrases {
                let n = p.text.chars().count();
                if n > *widest {
                    *widest = n;
                    *worst = p.text.clone();
                }
            }
        };
        for name in ["ant", "worm", "beetle"] {
            if world.species.id_of(name).is_none_or(|id| world.species.get(id).creature.is_none()) {
                continue;
            }
            let id = animal(&mut world, name);
            check(&describe(&world, id), &mut widest, &mut worst);
            for seed in 0..24u64 {
                world.organism_mut(id).expect("alive").genome = brain::random_genome(brain::sweep_genome_seed(seed));
                check(&describe(&world, id), &mut widest, &mut worst);
            }
        }
        let plants: Vec<u16> = (0..world.species.len() as u16)
            .filter(|i| world.species.get(SpeciesId(*i)).creature.is_none())
            .collect();
        for sp in plants {
            let id = world.push_organism(SpeciesId(sp)).expect("slots free");
            for locus in 0..organism::DISCRETE_LOCI {
                for allele in 0..organism::LOCUS_ALLELES[locus] {
                    for draw in [-1.0f32, -0.5, 0.5, 1.0] {
                        let state = world.organism_mut(id).expect("just pushed");
                        state.alleles[locus] = allele;
                        state.genotype_draws = [draw; organism::GENOTYPE_TRAITS];
                        check(&describe(&world, id), &mut widest, &mut worst);
                    }
                }
            }
        }
        assert!(widest > 0, "no phrases were measured, so this guard is checking nothing");
        assert!(
            widest <= PHRASE_COLUMNS,
            "{worst:?} is {widest} characters against a {PHRASE_COLUMNS}-character column -- it will widen the cell page and slide it over the roster"
        );
    }

    /// **The same individual, described live and out of a jar, reads the
    /// same.** The shelf is where *"what did I keep?"* is actually asked, and
    /// a jar that described itself differently from the animal it was taken
    /// from would be answering a different question.
    #[test]
    fn a_jar_describes_what_it_was_taken_from() {
        let mut world = world();
        let id = animal(&mut world, "ant");
        let species = world.organism(id).expect("alive").species;
        let live = describe(&world, id);
        let jar = crate::sim::specimen::capture(&world, id, "probe").expect("an ant can be kept");
        let kept = describe_genetics(&world, species, &jar.genetics);
        assert!(!live.is_empty(), "the positive control: the live animal has something to say");
        assert_eq!(
            live.iter().map(|p| p.text.clone()).collect::<Vec<_>>(),
            kept.iter().map(|p| p.text.clone()).collect::<Vec<_>>(),
            "a jar and the animal it was taken from describe themselves differently"
        );
    }

    /// **A plant's shape genes each say something, and the six are distinct.**
    ///
    /// A phrasebook that mapped two loci to the same sentence would produce a
    /// page that reads fine and tells you nothing, which is the failure mode
    /// a string table has.
    #[test]
    fn every_plant_locus_has_its_own_sentence() {
        let mut world = world();
        let species = world.species.id_of("tree").expect("tree loaded");
        let id = world.push_organism(species).expect("slots free");
        let mut seen: Vec<String> = Vec::new();
        for locus in 0..organism::DISCRETE_LOCI {
            let mut here: Vec<String> = Vec::new();
            for allele in 0..organism::LOCUS_ALLELES[locus] {
                let state = world.organism_mut(id).expect("just pushed");
                state.alleles = [0; organism::DISCRETE_LOCI];
                state.alleles[locus] = allele;
                state.genotype_draws = [0.0; organism::GENOTYPE_TRAITS];
                let said = describe(&world, id);
                assert_eq!(said.len(), organism::DISCRETE_LOCI + 1, "locus {locus} allele {allele}: a plant at the species mean should say one thing per locus plus the typical-otherwise line");
                here.push(said[locus].text.clone());
            }
            assert_eq!(
                here.len(),
                here.iter().collect::<std::collections::HashSet<_>>().len(),
                "locus {locus}: two of its alleles produced the same sentence, so the page cannot tell them apart"
            );
            seen.extend(here);
        }
        assert_eq!(
            seen.len(),
            seen.iter().collect::<std::collections::HashSet<_>>().len(),
            "two different loci produced the same sentence"
        );
    }
}
