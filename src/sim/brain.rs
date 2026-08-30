//! A **fixed-scaffold, evolvable brain**: a small network whose topology
//! never changes and whose weights are the genome.
//!
//! # Why the topology is caged
//!
//! `Reports/creature-direction.md` D4. NEAT-style topology evolution was
//! rejected, and every one of its downsides traced back to one thing —
//! mutating the *graph*. A variable-length graph genome needs speciation
//! machinery to protect innovations, needs hours of noise to bootstrap, and
//! produces networks nobody can read. So topology is what got caged, not
//! the brain: weights still evolve, freely, and every genome in the engine
//! shares one scaffold, which is also what keeps crossover cheap to add
//! later.
//!
//! # Generation zero must already behave
//!
//! The initial weights are **authored instincts** — a species `.ron` writes
//! plain taxis gains as a sparse wiring list, hidden units start silent,
//! and the colony forages on day one exactly as a brainless taxis design
//! would have. Evolution refines; it does not bootstrap. If the authored
//! ant does not forage convincingly, the fix is the instincts, not the
//! mutation rate.
//!
//! # No libm, anywhere
//!
//! `squash` is `x / (1 + |x|)`, not `tanh`. The choice function squares
//! rather than exponentiates. Headings come from a table rather than
//! `sin_cos`. Transcendentals are the named cross-platform determinism trap
//! (`Reports/emergent-world-architecture.md` §8d) and determinism is
//! required (`PLAN.md`), so they stay out of anything a decision reads
//! (P-19).

use serde::{Deserialize, Serialize};

pub const BRAIN_INPUTS: usize = 18;
pub const BRAIN_HIDDEN: usize = 4;
pub const BRAIN_OUTPUTS: usize = 11;

/// **Reserved storage dimensions.** The live counts above say how much of
/// the scaffold is wired; these say how much room the layout leaves it to
/// grow into. Both are needed, and conflating them is exactly what made the
/// 6 -> 9 output growth unlawful — see `GENOME_LEN`.
///
/// Sized to absorb any plausible growth on each axis before a migration, at 12,352
/// floats against the **268** a tight layout uses at today's live counts
/// (16x10 + 4x16 + 4 + 10x4): 2.3 KB per creature, ~9.6 MB at the
/// 4095-organism ceiling. Reserving more (16 outputs, 712 floats) buys no
/// additional lawfulness, only more headroom.
///
/// (That number read **248** until 2026-08-23, which was the tight layout
/// for `BRAIN_OUTPUTS = 9` — correct before the `Feed` verb was appended
/// and stale the moment it was. It is a derived quantity written by hand,
/// so it goes stale on exactly the appends this reservation exists to
/// make cheap; the reserved 12,352 is the number that does not move, and is
/// the one to reason with.)
pub const INPUT_SLOTS: usize = 64;
pub const HIDDEN_SLOTS: usize = 64;
pub const OUTPUT_SLOTS: usize = 64;

/// The genome: one flat `Vec<f32>` in four contiguous positional blocks,
/// **sized from the reserved dimensions rather than from the live counts**.
///
/// ```text
/// [0..4096)       input -> output   (64 x 64)  -- the authored "taxis gains"
/// [4096..8192)    input -> hidden   (64 x 64)
/// [8192..8256)    hidden self-recurrence     (64)
/// [8256..12352)   hidden -> output  (64 x 64)
/// ```
///
/// **Grown twice before this layout, and only the first growth was
/// lawful.** 168 (14x6) -> 188 (16x6, inputs 14 and 15 appended after the
/// lateral sensors turned out to be identically zero on a horizontal
/// surface -- see `BrainInput::PheroAAlong`) -> 248 (16x9, outputs 6, 7 and
/// 8 appended). The block was `input * BRAIN_OUTPUTS + output`, so
/// appending an output was an *insert into every row*: `(TempAboveAmb,
/// Turn)` moved from index 48 to 72, every weight with an input index >= 1
/// was renumbered, and `IO_END` moving 96 -> 144 shifted the two blocks
/// after it wholesale.
///
/// **Re-laying output-major was considered and is not the fix.** Under
/// `output * BRAIN_INPUTS + input` an output appends cleanly and an *input*
/// becomes the stride change -- the mirror image, on the axis this genome
/// has already grown along twice. There is no axis here that is safe to
/// sacrifice.
///
/// **And no arrangement inside a block is sufficient by itself**, which is
/// the part that is easy to miss: `IO_END`, `IH_END` and `HH_END` are
/// cumulative, so a block whose size comes from a *live* count shifts the
/// start of every block after it the moment that count grows. That is half
/// of what actually broke in the 248 growth, and re-ordering a block does
/// not touch it.
///
/// **So every dimension that can grow is reserved, and every block is sized
/// from the reserve.** Appending an input, an output or a hidden unit
/// lights up storage that already existed and was already zero: not one
/// existing weight moves and `GENOME_LEN` does not change. The law now
/// holds in all three directions until a reserve fills, at which point a
/// real migration is needed -- and `genome_manifest` is what makes that a
/// failing test rather than a silent reinterpretation of every individual
/// alive.
///
/// Ordering *within* a block is therefore a performance choice rather than
/// a correctness one, and it is output-major because `eval_brain` loops
/// `for o { for i }`, so each output's row is contiguous.
///
/// **Slots are positional and must never be renumbered or reordered.** The
/// same law the plant genome already lives under (`organism.rs`'s
/// `genotype_draws`), for the same reason: the slot index *is* the meaning.
/// A stored genome is a list of numbers with no labels, so moving a slot
/// silently reinterprets every individual that already exists as a
/// different animal.
pub const GENOME_LEN: usize = HH_END + OUTPUT_SLOTS * HIDDEN_SLOTS; // 12,352

const IO_END: usize = OUTPUT_SLOTS * INPUT_SLOTS; // 4096
const IH_END: usize = IO_END + HIDDEN_SLOTS * INPUT_SLOTS; // 8192
const HH_END: usize = IH_END + HIDDEN_SLOTS; // 8256

/// Where a connection lives, by name rather than by arithmetic. Every
/// caller outside `eval_brain`'s inner loops goes through these, so a
/// future re-lay is one edit rather than a hunt through hand-written index
/// expressions — which is how the 6 -> 9 growth got as far as it did.
#[inline]
pub fn io_slot(input: BrainInput, output: BrainOutput) -> usize {
    output as usize * INPUT_SLOTS + input as usize
}
#[inline]
pub fn ih_slot(input: BrainInput, hidden: usize) -> usize {
    IO_END + hidden * INPUT_SLOTS + input as usize
}
#[inline]
pub fn hh_slot(hidden: usize) -> usize {
    IH_END + hidden
}
#[inline]
pub fn ho_slot(hidden: usize, output: BrainOutput) -> usize {
    HH_END + output as usize * HIDDEN_SLOTS + hidden
}

/// Is this genome index wired to anything, or is it reserve?
///
/// One definition of "live", used by everything that walks a genome, so
/// `live_slots` and `reserve_is_zero` cannot disagree with each other.
pub fn is_live_slot(idx: usize) -> bool {
    if idx < IO_END {
        idx / INPUT_SLOTS < BRAIN_OUTPUTS && idx % INPUT_SLOTS < BRAIN_INPUTS
    } else if idx < IH_END {
        let rel = idx - IO_END;
        rel / INPUT_SLOTS < BRAIN_HIDDEN && rel % INPUT_SLOTS < BRAIN_INPUTS
    } else if idx < HH_END {
        idx - IH_END < BRAIN_HIDDEN
    } else {
        let rel = idx - HH_END;
        rel / HIDDEN_SLOTS < BRAIN_OUTPUTS && rel % HIDDEN_SLOTS < BRAIN_HIDDEN
    }
}

/// Every genome index a wired slot occupies, ascending.
///
/// **Anything that writes a genome wholesale — a random draw, a mutation
/// operator — iterates this rather than `0..GENOME_LEN`.** A perturbed
/// reserved slot is invisible for exactly as long as its slot is unnamed,
/// and then springs to life as a connection nobody authored, in every
/// individual descended from the one that was perturbed.
pub fn live_slots() -> impl Iterator<Item = usize> {
    (0..GENOME_LEN).filter(|&i| is_live_slot(i))
}

/// True while every reserved slot is still exactly zero.
pub fn reserve_is_zero(g: &[f32]) -> bool {
    g.iter().enumerate().all(|(i, &w)| w == 0.0 || is_live_slot(i))
}

/// Slot names, in slot order. They exist so `genome_manifest` can hash
/// *meanings* rather than only sizes: a rename or a reorder changes the
/// manifest, which is the whole point.
pub const INPUT_NAMES: [&str; BRAIN_INPUTS] = [
    "Bias",
    "PheroAFront",
    "PheroALateral",
    "PheroBFront",
    "PheroBLateral",
    "MoistureFront",
    "MoistureLateral",
    "LightHere",
    "TempAboveAmb",
    "FoodAdjacent",
    "AtNest",
    "Energy",
    "Carrying",
    "Crowding",
    "PheroAAlong",
    "PheroBAlong",
    "PreyNear",
    "PreyBearing",
];
pub const OUTPUT_NAMES: [&str; BRAIN_OUTPUTS] = ["Turn", "Move", "EmitA", "EmitB", "Dig", "Drop", "Persist", "Tumble", "Caution", "Feed", "Impulse"];

fn fnv(h: u32, b: u8) -> u32 {
    (h ^ b as u32).wrapping_mul(0x0100_0193)
}

/// A hash of everything a stored genome's meaning depends on: the six
/// dimensions and the ordered slot names.
///
/// **This is the backstop the old layout did not have.**
/// `the_block_layout_exactly_fills_the_genome` checks that the blocks are
/// self-consistent, which stayed true through a growth that renumbered
/// every weight in the world. A manifest that a test pins to a literal
/// turns "somebody reordered a slot" from a silent reinterpretation of
/// every individual alive into a failing build.
pub fn genome_manifest() -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for d in [BRAIN_INPUTS, BRAIN_OUTPUTS, BRAIN_HIDDEN, INPUT_SLOTS, OUTPUT_SLOTS, HIDDEN_SLOTS, GENOME_LEN] {
        for b in (d as u32).to_le_bytes() {
            h = fnv(h, b);
        }
    }
    for name in INPUT_NAMES.iter().chain(OUTPUT_NAMES.iter()) {
        for b in name.bytes() {
            h = fnv(h, b);
        }
        h = fnv(h, 0);
    }
    h
}

/// A weight whose magnitude is below this is **no connection**: skipped in
/// evaluation *and* exempt from the synapse tax.
///
/// This is what gives evolution a real way to *delete* a connection. Without
/// it a weight can only ever get small, never absent, so a brain accumulates
/// vestigial links that each cost a multiply and a metabolic charge forever.
pub const W_EPS: f32 = 0.01;

/// **How far one point mutation can move a weight, at the bottom of the
/// scale.** The absolute half of `width = MUT_ABS_FLOOR + MUT_REL * |w|`
/// (`Reports/creature-evolution-plan.md` §2.6, decision E6).
///
/// **This is the term that lets a zero weight become a connection.**
/// Proportional-only mutation of zero is zero forever, so a purely
/// relative operator can prune a brain and can never grow one — every
/// lineage's wiring would then be a subset of its ancestor's, which is
/// not evolution, it is decay. Sized against `W_EPS` (0.01) rather than
/// against any authored weight: a single mutation of a dead slot has to
/// be able to clear the "is this a connection at all" threshold, or the
/// floor is decorative.
pub const MUT_ABS_FLOOR: f32 = 0.04;

/// The proportional half of the same width.
///
/// **This is the term that lets the ±30 homing gate move at a sensible
/// rate.** `ant.ron` authors weights spanning 0.2 to 30, and §13l measured
/// both ends of the single-step-size failure: one global step either never
/// shifts the gate or shreds everything else. At 0.10 the gate moves by
/// about 3 per mutation and a 0.3 crowding weight moves by about 0.07 —
/// the same *fraction* of each, which is the only sense in which one rate
/// can be fair across two orders of magnitude.
pub const MUT_REL: f32 = 0.10;

/// The magnitude no weight may exceed, in either direction.
///
/// **40, not §7a's 4.0, and the difference is the whole of E6.** The ant's
/// homing gate lives at ±30 in a genome whose other authored weights are
/// 0.2–2.5; a ±4 clamp destroys that gate on the first birth, so the
/// design that specified it predates the gate it would have deleted. 40
/// leaves the authored gate a third of a decade of headroom without
/// letting a runaway weight saturate every downstream unit permanently.
pub const MUT_CLAMP: f32 = 40.0;

/// **Point-mutate a genome in place**, one live slot at a time.
///
/// `rate` is the per-slot probability that a slot is touched at all, and
/// it is a knob rather than 1.0 for a reason the width formula alone does
/// not cover: with 268 live slots and an absolute floor large enough to
/// cross `W_EPS`, mutating *every* slot every birth turns a sparse
/// authored brain (11 connections) into a dense one (268) inside a
/// generation or two. Density is not free here — `CreatureDef::
/// synapse_fraction` charges per active synapse per tick — so an operator
/// with no rate would select hard for whatever survives a metabolic bill
/// nobody authored, and the sparsity pressure this module exists to
/// create would be measuring the operator instead of the world.
///
/// **`live_slots` rather than `0..GENOME_LEN`**, as that function's own
/// doc requires: a perturbed reserved slot is invisible for exactly as
/// long as its slot is unnamed and then springs to life as a connection
/// nobody authored, in every individual descended from the one that was
/// perturbed.
///
/// **Uniform in ±width, not Gaussian.** There is no libm in anything
/// determinism-relevant (see the module doc), and a Box-Muller pair would
/// buy tail shape this operator has no use for: what matters is that the
/// step scales with the weight and that zero is reachable from either
/// side, both of which a symmetric uniform gives.
///
/// **The caller owns the stream.** This draws from the `Rng` it is handed
/// and takes a variable number of draws, so handing it a generator the
/// caller goes on using would shift every subsequent draw by an amount
/// that depends on how many slots mutated — `CLAUDE.md`'s shared-`Rng`
/// gotcha, which stayed green through both of its guards. Every caller
/// here builds a dedicated `rng::stream`.
///
/// Returns how many slots actually moved, so a caller can print "did it
/// fire at all" beside the picture rather than inferring it.
pub fn mutate(genome: &mut [f32], rate: f32, rng: &mut crate::sim::rng::Rng) -> u32 {
    assert_eq!(genome.len(), GENOME_LEN, "a genome is always exactly GENOME_LEN; a short one means a slot layout changed");
    if rate <= 0.0 {
        return 0;
    }
    let mut moved = 0;
    for i in live_slots() {
        if rng.unit_f32() >= rate {
            continue;
        }
        let w = genome[i];
        let width = MUT_ABS_FLOOR + MUT_REL * w.abs();
        let step = (rng.unit_f32() * 2.0 - 1.0) * width;
        let next = (w + step).clamp(-MUT_CLAMP, MUT_CLAMP);
        if next != w {
            moved += 1;
        }
        genome[i] = next;
    }
    moved
}

/// Map an output in `(-1, 1)` onto `(0, scale)` — for the outputs that are
/// *magnitudes* rather than signed drives (`Persist`, `Tumble`,
/// `Caution`). A silent connection reads 0 and lands at half scale, so a
/// creature with nothing authored still behaves rather than freezing.
#[inline]
pub fn unit_scale(out: f32, scale: f32) -> f32 {
    ((out + 1.0) * 0.5).clamp(0.0, 1.0) * scale
}

/// Fast sigmoid, output in `(-1, 1)`.
///
/// **Not `tanh`.** See the module doc: no libm in anything determinism
/// relevant. This has the same shape and saturating behaviour, is
/// bit-identical across platforms, and is faster.
#[inline]
pub fn squash(x: f32) -> f32 {
    x / (1.0 + x.abs())
}

/// Which input slot a wiring entry refers to. **Positional and
/// append-only** — see `GENOME_LEN`. The names exist so a species file can
/// be read by a human; the numbers are the contract.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub enum BrainInput {
    /// Always 1.0 — the constant term every gain is offset against.
    Bias = 0,
    /// Channel A concentration at the forward sensor.
    PheroAFront = 1,
    /// Channel A, right sensor minus left. Positive = stronger to the
    /// right. **This pairing is the single best interface idea in the
    /// surveyed sims** (Bibites' `PheroSense` + `PheroAngle`): concentration
    /// plus a direction hint makes trail-following reachable by *one*
    /// connection, from a lateral input to the turn output.
    PheroALateral = 2,
    PheroBFront = 3,
    PheroBLateral = 4,
    MoistureFront = 5,
    MoistureLateral = 6,
    /// Light where the head is.
    LightHere = 7,
    /// Degrees above ambient where the head is, scaled.
    TempAboveAmb = 8,
    /// 1.0 if something on the species' food list is in the head's
    /// 8-neighbourhood.
    FoodAdjacent = 9,
    /// 1.0 if nest material is in the head's 8-neighbourhood.
    AtNest = 10,
    /// Energy as a fraction of the species' starting energy.
    Energy = 11,
    /// 1.0 if carrying something.
    Carrying = 12,
    /// Creature cells within r=2 of the head, over 8.
    ///
    /// **The negative-feedback term, and it is not optional.** Without a
    /// crowding input a colony ossifies on the first path it finds, and
    /// P-12 is explicit that *more evaporation will not fix it*. If an
    /// authored genome ever has a zero gain here, that is the first thing
    /// to suspect when trails stop adapting.
    Crowding = 13,
    /// **Pheromone gradient along the heading**: how much more channel A
    /// there is at the forward sensor than underfoot, normalized to
    /// `-1..1`, so a weak trail and a strong one both give a usable
    /// reading.
    ///
    /// **This exists because the lateral inputs do not work on a surface.**
    /// `PheroALateral`/`PheroBLateral` sample ahead-left and ahead-right at
    /// the full sensor offset, which in a side-view world where creatures
    /// walk on the ground puts both of them in open air — measured with
    /// `examples/creature_probe.rs`, an ant standing on a cell holding
    /// `A = 27` read `pheroA_lr` of exactly 0.000. The Jones/Physarum
    /// sensor triad assumes agents in open 2D; this engine is not that.
    ///
    /// A scalar "is it getting better in the direction I am already going"
    /// is what a surface-dweller can actually measure, and it is enough:
    /// paired with a tumble on a failed move it gives run-and-tumble
    /// chemotaxis, which is how bacteria solve exactly this problem without
    /// being able to steer at all.
    ///
    /// The lateral inputs are kept, unwired, rather than removed. They are
    /// correct for anything moving in open space (a flier, a swimmer) and
    /// removing a slot is the one thing the positional law forbids.
    PheroAAlong = 14,
    PheroBAlong = 15,
    /// **How close the nearest prey animal this creature can actually see
    /// is**, as `1 - distance / sight_range`: 1.0 in contact, 0.0 at the
    /// limit of the eye, and exactly 0.0 when nothing is in sight.
    ///
    /// **The first input in this scaffold that reports another animal at a
    /// distance at all**, and that absence is why E15 exists. Before it,
    /// `FoodAdjacent` and `AtNest` were the head's 8-neighbourhood,
    /// `Crowding` was r=2 and could not say *what* was near, and the two
    /// pheromone planes were the only distal sense — measured unusable for
    /// hunting: mean beetle-to-nearest-trail 46 cells against a 6-cell
    /// sensor span, and the beetle's two sensor reads differing 1.3% of the
    /// time. A predator with no way to find prey moved no counter at all:
    /// `beetles=0` and `beetles=9` ran **bit-identical** over 6,000 frames.
    ///
    /// **Zero for every species that has not authored `sight_range`**, and
    /// that is the opt-in rather than a default — see
    /// `CreatureDef::sight_range`. Sized at 64 cells and cast all-round by
    /// `Reports/creature-vision-sizing-2026-08-30.md`, which measured the
    /// reach, the shape, what occludes it and what it costs before any of
    /// it was built.
    ///
    /// **Paired with `PreyBearing` deliberately**, the same way
    /// `PheroAFront` is paired with `PheroALateral`: a magnitude says
    /// *there is something*, a direction says *that way*, and the pair
    /// makes pursuit reachable by one connection from each. On its own this
    /// input can only gate speed.
    PreyNear = 16,
    /// **Which way to turn to face that prey**, as the signed angle from
    /// the current heading normalized to `-1..1`. **Positive = to the
    /// right**, the same convention `PheroALateral` states, so an authored
    /// pursuit instinct is a *negative* weight into `Turn` (which biases
    /// left when positive — see `creature.rs`'s candidate scoring).
    ///
    /// `0.0` means dead ahead *and* means nothing in sight; `PreyNear` is
    /// what separates those, which is the other half of why the pair is a
    /// pair.
    ///
    /// **A full-circle bearing, not a lateral difference of two sensor
    /// reads**, and that is not a style choice. `+-1` is prey directly
    /// behind, so an animal that has walked past its target turns hard
    /// rather than reading the 0 a left-minus-right sensor would give it at
    /// exactly the moment the sense matters most. It also keeps this input
    /// clear of `CLAUDE.md`'s coarse-field degeneracy — hit four times on
    /// three lines and never once caught by a test — because it is not a
    /// difference of two samples of a block-nearest field at all: it is one
    /// bearing to one cell found by a ray traced at CA resolution.
    PreyBearing = 17,
}

/// Which output slot. Positional and append-only, as above.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub enum BrainOutput {
    /// Added to the ahead-left and ahead-right candidate scores with
    /// opposite signs.
    Turn = 0,
    /// P(move this tick).
    Move = 1,
    /// Deposit rate on channel A.
    EmitA = 2,
    /// Deposit rate on channel B.
    EmitB = 3,
    /// Gates the dig/pick-up action.
    Dig = 4,
    /// Gates the drop action.
    Drop = 5,
    /// How strongly to keep going straight, against the turn bias.
    ///
    /// **These last three were constants in `creature.rs`, and moving them
    /// here is the point rather than a tidy-up.** An ablation over the
    /// authored genome found eight of ten instincts produced *bit-identical*
    /// behaviour: the brain was riding along while hardcoded locomotion
    /// policy did the deciding. A genome that cannot change what a creature
    /// does is a genome selection cannot act on, so evolution would have
    /// been a random walk on a plateau — and we would not have been able to
    /// tell that from evolution being broken.
    ///
    /// This one was an anonymous `0.15` in the candidate scoring, and it is
    /// the single number that decides milling versus commuting — the
    /// behaviour actually failing (median net displacement was 2% of path
    /// length). It had no business being a literal.
    Persist = 6,
    /// P(re-orient | the move roll failed). Was `TUMBLE_ON_FAILED_MOVE`.
    /// As an output rather than a constant it can be *modulated* — tumble
    /// more when crowded, less when on a good gradient — which a constant
    /// cannot express at all.
    Tumble = 7,
    /// How strongly a foothold is preferred over open air. Was
    /// `FOOTING_BONUS`, and at 0.6 against a turn signal capped at 1.0 it
    /// was swamping the steering it competed with.
    Caution = 8,
    /// P(take the food in reach) — eat it or pick it up.
    ///
    /// **Split out of `Dig`, which used to gate excavating and feeding with
    /// one weight.** §13d added `(Bias, Dig, 0.4)` because ants never dug,
    /// and it silently raised the baseline *eating* probability at the same
    /// time; nothing could separate the two, so a burrower and a grazer are
    /// not distinguishable points in the genome — there is no gene to tell
    /// them apart with. That is a hard precondition for the divergence S5
    /// and S7 are about, and it is why this ships in a milestone with no
    /// genes in it.
    ///
    /// **The first lawful output append**, and the thing S2 was built for:
    /// under the reserved layout this lights up a row that already existed
    /// and was already zero, so not one weight of any other slot moved.
    /// Authored at the Dig weights it inherited, so generation-zero
    /// behaviour is unchanged and evolution starts from where the hand
    /// tuning left off rather than from a hole.
    Feed = 9,
    /// **Leave the ground.** P(launch this tick), read raw and gated on
    /// strictly positive — never through `unit_scale`, and that is the
    /// difference between a verb and a default.
    ///
    /// `squash(0.0)` is exactly 0.0, so a species that has never authored a
    /// weight into this row reads 0.0, takes no RNG draw, and is
    /// bit-identical to the tree before the slot existed. `unit_scale` would
    /// have read 0.5 for the same silent row and every ant in the world
    /// would have started jumping — which is `Persist`'s mid-scale default
    /// being right for a *modulator* and wrong for an *action*. `Move`,
    /// `Dig`, `Drop` and `Feed` are all read the same raw way for the same
    /// reason.
    ///
    /// **The verb says only "go"; the body says what that means.** There is
    /// no jump height here and no per-species table: the launch speed is one
    /// impulse divided by the body's mass, and the descent is a drag law
    /// over the body's own bounding box. A 2-cell chain hops; a 9-cell slab
    /// barely leaves the ground and then glides; a 9-cell block of the same
    /// mass drops at 2.3x the slab's speed. See `creature::launch` and
    /// `creature::step_flight` — `Reports/creature-motion-design.md` §5 is
    /// the argument, and §2d is why this ships with a falls-per-move guard.
    ///
    /// **The second lawful output append.** Like `Feed` it lights up a row
    /// of the reserve that already existed and was already zero: not one
    /// weight of any other slot moves, `GENOME_LEN` does not change, and
    /// every stored species file still means what it meant. What it does
    /// change is `live_slots()` — 268 -> 288 — so `random_genome` draws 20
    /// more values and a sampled genome at a given seed is a *different*
    /// animal than it was. That is the real, unavoidable cost of a live
    /// verb (`creature-motion-design.md` §3): the reserve is free, the wiring
    /// is not.
    ///
    /// **The slot after this one stays unnamed on purpose** — §4d and the
    /// owner's call 2. The condition for spending it is written down in §4b:
    /// the day something in the world moves a creature against its will, a
    /// grip verb has a benefit to point at. Naming it now and changing it
    /// later renumbers nothing but does mislead every species file that
    /// authored the name.
    Impulse = 10,
}

/// One authored connection, as a species file writes it:
/// `(PheroBLateral, Turn, 0.9)`.
///
/// A **sparse** list rather than 168 raw floats, because 168 raw floats is
/// not something anyone can review, and the whole value of authored
/// instincts is that a human can see what the animal starts out believing.
/// Anything not listed is 0.0, which under `W_EPS` means "no connection".
#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct Instinct(pub BrainInput, pub BrainOutput, pub f32);

/// One authored connection into a hidden unit: `(Carrying, 0, 5.0)`.
#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct HiddenWire(pub BrainInput, pub u8, pub f32);

/// One authored connection out of a hidden unit: `(0, Turn, 1.0)`.
#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct OutputWire(pub u8, pub BrainOutput, pub f32);

/// One hidden unit's **self-recurrence** weight: `(0, 0.8)` — how much of
/// unit 0's own previous activation it reads back this tick.
///
/// **This is the block the three lists above could not reach, and it went
/// unnoticed because nothing ever wrote a genome out.** `eval_brain` reads
/// `hh[h]`, `is_live_slot` counts all four of those slots as live, and
/// `random_genome` fills them — so a sampled brain, or from S6 a mutated
/// one, can carry recurrence, and until this existed there was no authored
/// form that could express it. A species file could therefore describe
/// every evolved individual except one whose *memory* had evolved, and the
/// loss would have been silent: the reloaded animal would simply have no
/// memory, which looks like a slightly different animal rather than like a
/// bug.
///
/// Authored by **index**, not by name, because a hidden unit has no name —
/// see `SpeciesDef::genome_manifest` for what guards that.
#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct Recurrence(pub u8, pub f32);

/// Expand sparse wiring lists into the dense genome.
///
/// The hidden blocks default to **all zero** (silent), which is what
/// "evolution refines rather than bootstraps" means in practice. But a
/// species that needs a hidden unit on generation zero authors it as
/// weights, in data, exactly like any other instinct — it does not get a
/// special case in code. `Reports/creature-direction.md` §4c names this as
/// the honest fallback for the one thing a single layer provably cannot
/// express, and the ant needed it:
///
/// **"Follow B when empty-handed, A when laden" is a product, and a linear
/// layer has no product.** The additive approximation was tried first, on
/// the report's own reasoning that food trails exist where food is and nest
/// scent where the nest is, so the two gains would rarely compete. They
/// compete constantly. Measured: 28 ants picked food up and *all 28 were
/// still holding it* at the end of the run, with a mean position of x=208
/// against a nest at x<120 — carriers walking steadily away from home,
/// because the strongest thing they could smell was the food trail they
/// were themselves laying.
///
/// A single sigmoid unit cannot AND a *signed* signal with a gate either.
/// What does work, and what `ant.ron` authors, is a **symmetric pair** per
/// gated channel: two units with the same gate and opposite signal
/// polarity, wired to the output with opposite signs. Ungated, both
/// saturate to the same value and their contributions cancel exactly;
/// gated, they reinforce. Four hidden units, two channels, no code.
pub fn genome_from_wiring(instincts: &[Instinct], hidden: &[HiddenWire], outputs: &[OutputWire], recurrence: &[Recurrence]) -> Vec<f32> {
    let mut g = vec![0.0; GENOME_LEN];
    for &Instinct(input, output, weight) in instincts {
        g[io_slot(input, output)] = weight;
    }
    for &HiddenWire(input, h, weight) in hidden {
        assert!((h as usize) < BRAIN_HIDDEN, "hidden unit {h} does not exist; there are {BRAIN_HIDDEN}");
        g[ih_slot(input, h as usize)] = weight;
    }
    for &OutputWire(h, output, weight) in outputs {
        assert!((h as usize) < BRAIN_HIDDEN, "hidden unit {h} does not exist; there are {BRAIN_HIDDEN}");
        g[ho_slot(h as usize, output)] = weight;
    }
    for &Recurrence(h, weight) in recurrence {
        assert!((h as usize) < BRAIN_HIDDEN, "hidden unit {h} does not exist; there are {BRAIN_HIDDEN}");
        g[hh_slot(h as usize)] = weight;
    }
    debug_assert!(reserve_is_zero(&g), "authored wiring wrote into a reserved slot");
    g
}

/// The input→output-only form, for callers with nothing in the hidden layer.
pub fn genome_from_instincts(instincts: &[Instinct]) -> Vec<f32> {
    genome_from_wiring(instincts, &[], &[], &[])
}

/// Slot enums in slot order, so a genome index can be turned back into the
/// name a species file writes.
///
/// **Positional, exactly as `INPUT_NAMES` is**, and the pair is checked
/// against the discriminants by `the_slot_tables_agree_with_the_names` — a
/// table that drifted from the enum would relabel every exported weight,
/// which is the one failure an export can have that still produces a file
/// the loader accepts.
pub const INPUTS: [BrainInput; BRAIN_INPUTS] = [
    BrainInput::Bias,
    BrainInput::PheroAFront,
    BrainInput::PheroALateral,
    BrainInput::PheroBFront,
    BrainInput::PheroBLateral,
    BrainInput::MoistureFront,
    BrainInput::MoistureLateral,
    BrainInput::LightHere,
    BrainInput::TempAboveAmb,
    BrainInput::FoodAdjacent,
    BrainInput::AtNest,
    BrainInput::Energy,
    BrainInput::Carrying,
    BrainInput::Crowding,
    BrainInput::PheroAAlong,
    BrainInput::PheroBAlong,
    BrainInput::PreyNear,
    BrainInput::PreyBearing,
];
/// See [`INPUTS`].
pub const OUTPUTS: [BrainOutput; BRAIN_OUTPUTS] = [
    BrainOutput::Turn,
    BrainOutput::Move,
    BrainOutput::EmitA,
    BrainOutput::EmitB,
    BrainOutput::Dig,
    BrainOutput::Drop,
    BrainOutput::Persist,
    BrainOutput::Tumble,
    BrainOutput::Caution,
    BrainOutput::Feed,
    BrainOutput::Impulse,
];

/// A genome written back out as the four sparse lists a species file
/// authors — the exact inverse of [`genome_from_wiring`].
///
/// **This is the dev-tool exit** (`Reports/creature-evolution-plan.md`
/// decision E8: *"we can use it to create new creatures that get saved and
/// added to the game"*). An evolved individual is 12,352 floats; the loader
/// only ever reads sparse wiring lists, so the way to write an individual
/// into `assets/species/` is to invert the expansion rather than to invent
/// a second genome format the loader would then have to learn.
///
/// It is also why the export stays **reviewable**: this module's own
/// argument for the sparse form is that "168 raw floats is not something
/// anyone can review", and that argument does not stop applying because a
/// machine wrote the numbers.
#[derive(Clone, Default)]
pub struct Wiring {
    pub instincts: Vec<Instinct>,
    pub hidden: Vec<HiddenWire>,
    pub outputs: Vec<OutputWire>,
    pub recurrence: Vec<Recurrence>,
}

/// Decompose a dense genome into the sparse lists that reproduce it.
///
/// **Every nonzero live slot is emitted, including weights below
/// [`W_EPS`].** A sub-`W_EPS` weight is "no connection" to `eval_brain`,
/// so dropping it would look free — and it is not, for the one reason that
/// matters to a dev tool: mutation is relative as well as absolute
/// (`creature-evolution-plan.md` §2.6, `width = MUT_ABS_FLOOR + MUT_REL *
/// |w|`), so a 0.004 weight is a connection one birth away from existing
/// and its sign is inherited. Rounding it out on the way through would be
/// a silent edit to the animal's *descendants* rather than to the animal,
/// which is precisely the kind of loss a round-trip test written against
/// behaviour would not catch.
///
/// **Reserved slots are never emitted, and cannot be**: they have no name
/// to emit them under. `reserve_is_zero` is asserted on the way in, so a
/// genome that has somehow perturbed its reserve fails loudly here rather
/// than losing that perturbation quietly — which is what `live_slots`' own
/// doc asks for.
pub fn wiring_from_genome(g: &[f32]) -> Wiring {
    assert_eq!(g.len(), GENOME_LEN, "a genome is always exactly GENOME_LEN; a short one means a slot layout changed");
    assert!(reserve_is_zero(g), "genome carries a weight in a reserved slot; there is no authored form for it");
    let mut w = Wiring::default();
    for (o, &output) in OUTPUTS.iter().enumerate() {
        for (i, &input) in INPUTS.iter().enumerate() {
            let weight = g[o * INPUT_SLOTS + i];
            if weight != 0.0 {
                w.instincts.push(Instinct(input, output, weight));
            }
        }
    }
    for h in 0..BRAIN_HIDDEN {
        for (i, &input) in INPUTS.iter().enumerate() {
            let weight = g[IO_END + h * INPUT_SLOTS + i];
            if weight != 0.0 {
                w.hidden.push(HiddenWire(input, h as u8, weight));
            }
        }
    }
    for h in 0..BRAIN_HIDDEN {
        let weight = g[hh_slot(h)];
        if weight != 0.0 {
            w.recurrence.push(Recurrence(h as u8, weight));
        }
    }
    for (o, &output) in OUTPUTS.iter().enumerate() {
        for h in 0..BRAIN_HIDDEN {
            let weight = g[HH_END + o * HIDDEN_SLOTS + h];
            if weight != 0.0 {
                w.outputs.push(OutputWire(h as u8, output, weight));
            }
        }
    }
    w
}

/// [`Wiring`] expanded back to a dense genome — `genome_from_wiring` with
/// the four lists already grouped, so the round trip can be asserted
/// without reaching through the RON layer.
pub fn genome_from_wiring_struct(w: &Wiring) -> Vec<f32> {
    genome_from_wiring(&w.instincts, &w.hidden, &w.outputs, &w.recurrence)
}

/// A random genome, sparse the way an authored one is.
///
/// **Lives here rather than in a harness because two harnesses need to
/// agree on it.** `examples/creature_space.rs` samples genomes and reports
/// what they scored; `examples/filmstrip.rs` draws what one of them looks
/// like. If those two disagreed by one line of arithmetic the contact sheet
/// would be a picture of a *different animal* than the row above it, and
/// nothing on screen would say so -- the same class of trap as an
/// `include_str!` asset edit that never reached the binary.
///
/// **Sparsity is itself sampled**, 2% to 15% of connections live, rather
/// than fixed: a fixed density is one more assumption smuggled into the
/// "random" arm, and a dense random brain is not a creature, it is noise,
/// since every input drives every output at once.
/// **Live slots only.** Drawing over `0..GENOME_LEN` would seed the reserve
/// with weights that are dead now and would wake up as authored-looking
/// connections the day their slot is named (`live_slots`). It also means
/// the rows this produces moved when the layout was re-laid, because the
/// stream is consumed in slot order — expected, and not a regression: the
/// `zero` and `authored` references are what pin the re-lay.
pub fn random_genome(seed: u64) -> Vec<f32> {
    let mut r = crate::sim::rng::stream(seed, 0, 0, 0);
    let density = 0.02 + r.unit_f32() * 0.13;
    let mut g = vec![0.0; GENOME_LEN];
    for idx in live_slots() {
        if r.unit_f32() < density {
            g[idx] = (r.unit_f32() * 2.0 - 1.0) * 3.0;
        }
    }
    debug_assert!(reserve_is_zero(&g));
    g
}

/// The seed `examples/creature_space.rs` labels genome `rNNN` with, so a
/// row in that sweep and a picture in `filmstrip` can be matched up by
/// name. Off-by-one here would be silent and ruinous.
pub fn sweep_genome_seed(index: u64) -> u64 {
    0x5EED + index
}

/// Evaluate the network. Returns the outputs and the count of **active**
/// synapses, which the caller charges energy for (`SYNAPSE_COST`).
///
/// The active count comes back from here rather than being recomputed
/// because it is free: the eval already tests every weight against `W_EPS`.
///
/// **Recurrence reads last tick's activations** (P-17). `state` is updated
/// *after* the hidden layer is computed from its previous contents. Getting
/// this backwards leaves everything apparently working — the network still
/// produces plausible outputs — while the memory the recurrence exists to
/// provide silently does not exist, and it is a slower feed-forward pass.
pub fn eval_brain(g: &[f32], inputs: &[f32; BRAIN_INPUTS], state: &mut [f32; BRAIN_HIDDEN]) -> ([f32; BRAIN_OUTPUTS], u32) {
    debug_assert_eq!(g.len(), GENOME_LEN, "a genome is always exactly GENOME_LEN; a short one means a slot layout changed");
    let mut active = 0u32;
    let (io, ih, hh, ho) = (&g[0..IO_END], &g[IO_END..IH_END], &g[IH_END..HH_END], &g[HH_END..GENOME_LEN]);

    let mut hidden = [0.0f32; BRAIN_HIDDEN];
    for (h, slot) in hidden.iter_mut().enumerate() {
        let mut sum = hh[h] * state[h];
        if hh[h].abs() >= W_EPS {
            active += 1;
        }
        for (i, &input) in inputs.iter().enumerate() {
            let w = ih[h * INPUT_SLOTS + i];
            if w.abs() >= W_EPS {
                sum += w * input;
                active += 1;
            }
        }
        *slot = squash(sum);
    }

    let mut out = [0.0f32; BRAIN_OUTPUTS];
    for (o, slot) in out.iter_mut().enumerate() {
        let mut sum = 0.0;
        for (i, &input) in inputs.iter().enumerate() {
            let w = io[o * INPUT_SLOTS + i];
            if w.abs() >= W_EPS {
                sum += w * input;
                active += 1;
            }
        }
        for (h, &hidden) in hidden.iter().enumerate() {
            let w = ho[o * HIDDEN_SLOTS + h];
            if w.abs() >= W_EPS {
                sum += w * hidden;
                active += 1;
            }
        }
        *slot = squash(sum);
    }

    *state = hidden;
    (out, active)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zero_state() -> [f32; BRAIN_HIDDEN] {
        [0.0; BRAIN_HIDDEN]
    }

    #[test]
    fn the_block_layout_exactly_fills_the_genome() {
        // The slot-layout law made mechanical: if someone changes a size
        // const without re-deriving the blocks, the genome silently gains
        // or loses a tail and every stored individual is reinterpreted.
        assert_eq!(IO_END, 4096);
        assert_eq!(IH_END, 8192);
        assert_eq!(HH_END, 8256);
        assert_eq!(GENOME_LEN, HH_END + OUTPUT_SLOTS * HIDDEN_SLOTS);
        assert_eq!(GENOME_LEN, 12352);
        // Every block sized from the *reserve*, never from a live count --
        // this is the assertion that fails if someone "tidies" a stride
        // back to BRAIN_INPUTS/BRAIN_OUTPUTS, which is what made the
        // 6 -> 9 output growth renumber every weight in the world.
        assert_eq!(IO_END, OUTPUT_SLOTS * INPUT_SLOTS);
        assert_eq!(IH_END - IO_END, HIDDEN_SLOTS * INPUT_SLOTS);
        assert_eq!(HH_END - IH_END, HIDDEN_SLOTS);
        const { assert!(BRAIN_INPUTS <= INPUT_SLOTS && BRAIN_OUTPUTS <= OUTPUT_SLOTS && BRAIN_HIDDEN <= HIDDEN_SLOTS) };
    }

    #[test]
    fn appending_a_slot_on_any_axis_moves_no_existing_weight() {
        // The law itself, tested rather than asserted in a comment: recompute
        // every live index as it would be with one more input, one more
        // output and one more hidden unit, and require that the indices the
        // *current* slots occupy are unchanged. This is what a growth is,
        // and it is the check the old layout could not have passed.
        let here: Vec<usize> = live_slots().collect();
        let grown = |ins: usize, outs: usize, hid: usize| -> Vec<usize> {
            let mut v = Vec::new();
            v.extend((0..outs).flat_map(|o| (0..ins).map(move |i| o * INPUT_SLOTS + i)));
            v.extend((0..hid).flat_map(|h| (0..ins).map(move |i| IO_END + h * INPUT_SLOTS + i)));
            v.extend((0..hid).map(|h| IH_END + h));
            v.extend((0..outs).flat_map(|o| (0..hid).map(move |h| HH_END + o * HIDDEN_SLOTS + h)));
            v
        };
        for (ins, outs, hid) in [
            (BRAIN_INPUTS + 1, BRAIN_OUTPUTS, BRAIN_HIDDEN),
            (BRAIN_INPUTS, BRAIN_OUTPUTS + 1, BRAIN_HIDDEN),
            (BRAIN_INPUTS, BRAIN_OUTPUTS, BRAIN_HIDDEN + 1),
        ] {
            let after = grown(ins, outs, hid);
            for slot in &here {
                assert!(after.contains(slot), "growing to ({ins}, {outs}, {hid}) moved slot {slot}");
            }
        }
    }

    #[test]
    fn the_authored_ant_writes_nothing_into_the_reserve() {
        let g = super::genome_from_wiring(
            &[Instinct(BrainInput::Crowding, BrainOutput::Caution, 1.0)],
            &[HiddenWire(BrainInput::Carrying, 3, 30.0)],
            &[OutputWire(3, BrainOutput::Caution, -2.5)],
            &[Recurrence(3, 0.5)],
        );
        assert!(reserve_is_zero(&g));
        assert!(is_live_slot(BrainOutput::Caution as usize * INPUT_SLOTS + BrainInput::Crowding as usize));
        // The corner cases: the last live column of a row, and the first
        // reserved one beside it.
        assert!(is_live_slot(BRAIN_INPUTS - 1));
        assert!(!is_live_slot(BRAIN_INPUTS));
        assert!(!is_live_slot(BRAIN_OUTPUTS * INPUT_SLOTS));
        assert_eq!(live_slots().count(), BRAIN_OUTPUTS * BRAIN_INPUTS + BRAIN_HIDDEN * BRAIN_INPUTS + BRAIN_HIDDEN + BRAIN_OUTPUTS * BRAIN_HIDDEN);
    }

    #[test]
    fn the_slot_tables_agree_with_the_discriminants() {
        // `INPUTS`/`OUTPUTS` are what turns a genome index back into the
        // name a species file writes. If either drifted from the enum,
        // every exported weight would be relabelled -- and the file would
        // still parse, still load, and describe a different animal. That
        // is the one export failure with no other tell, so it gets a test
        // rather than a comment.
        for (i, &input) in INPUTS.iter().enumerate() {
            assert_eq!(input as usize, i, "INPUTS[{i}] is not slot {i}");
            assert_eq!(io_slot(input, BrainOutput::Turn), i, "INPUTS[{i}] does not index slot {i}");
        }
        for (o, &output) in OUTPUTS.iter().enumerate() {
            assert_eq!(output as usize, o, "OUTPUTS[{o}] is not slot {o}");
        }
        assert_eq!(INPUTS.len(), INPUT_NAMES.len());
        assert_eq!(OUTPUTS.len(), OUTPUT_NAMES.len());
    }

    #[test]
    fn a_genome_survives_being_written_out_as_wiring_and_read_back() {
        // The property the whole dev-tool exit rests on: `wiring_from_
        // genome` is the exact inverse of `genome_from_wiring` over every
        // live slot. Run over random genomes rather than the authored ant,
        // because an authored genome is sparse in a *particular* way --
        // nothing in the hidden self-recurrence block, no negative weights
        // in some blocks -- and the export has to survive whatever
        // evolution produces, not what a human wrote.
        for seed in 0..64u64 {
            let g = random_genome(seed);
            let back = genome_from_wiring_struct(&wiring_from_genome(&g));
            assert_eq!(g, back, "seed {seed} did not survive the round trip");
        }
    }

    #[test]
    fn the_recurrence_block_is_reachable_only_through_the_fourth_list() {
        // The gap this found, made mechanical. `eval_brain` reads `hh[h]`
        // and `is_live_slot` calls those four slots live, so a sampled or
        // mutated brain can carry memory -- and the three original wiring
        // lists have no way to say so. Without `Recurrence`, exporting such
        // an animal loses its memory silently: the file loads, the animal
        // spawns, and it is simply a different creature.
        let mut g = vec![0.0f32; GENOME_LEN];
        g[hh_slot(1)] = 0.75;
        g[io_slot(BrainInput::Bias, BrainOutput::Move)] = 2.0;

        let w = wiring_from_genome(&g);
        assert_eq!(w.recurrence.len(), 1, "the recurrence weight was not emitted");
        assert_eq!(genome_from_wiring_struct(&w), g);

        // The sensitivity control -- drop the new list and watch the round
        // trip go red, so this test is evidence about the mechanism rather
        // than about itself.
        let without = genome_from_wiring(&w.instincts, &w.hidden, &w.outputs, &[]);
        assert_ne!(without, g, "dropping the recurrence list changed nothing; this guard cannot fail");
        assert_eq!(without[hh_slot(1)], 0.0);
    }

    #[test]
    fn a_weight_too_small_to_be_a_connection_is_still_written_out() {
        // Sub-`W_EPS` weights are "no connection" to `eval_brain`, so
        // dropping them on export looks free. It is not: mutation is
        // partly proportional, so a 0.004 weight is one birth away from
        // being a connection and its sign is inherited. Rounding it out
        // would edit the animal's descendants rather than the animal --
        // invisible to any test written against this generation's
        // behaviour, which is exactly why it is asserted on the bytes.
        let mut g = vec![0.0f32; GENOME_LEN];
        let tiny = W_EPS * 0.4;
        g[io_slot(BrainInput::Crowding, BrainOutput::Tumble)] = -tiny;
        let w = wiring_from_genome(&g);
        assert_eq!(w.instincts.len(), 1, "a sub-W_EPS weight was dropped on the way out");
        assert_eq!(genome_from_wiring_struct(&w), g);
    }

    #[test]
    fn the_genome_manifest_is_pinned() {
        // Not a checksum for its own sake: this is the thing that fails when
        // a slot is renamed or reordered, which is otherwise a silent
        // reinterpretation of every stored individual. If this assertion
        // fires, either put the slot back or accept that every genome in
        // flight now means something else -- and say which in the commit.
        // **Moved 2026-08-29 by the `Impulse` append, and this is the
        // lawful kind of move.** The manifest hashes the dimensions and the
        // ordered slot names, so *adding* a name at the end changes it while
        // renumbering nothing: every weight of every stored genome still
        // means what it meant, because `BRAIN_OUTPUTS` indexes into a
        // reserve of 64 that has not moved. A change here that came with a
        // changed `GENOME_LEN` or a reordered name would be the other kind.
        //
        // **Moved again 2026-08-30 by the `PreyNear`/`PreyBearing` append**
        // (E15, the sight sense), and lawfully for the same reason one
        // level up: `BRAIN_INPUTS` 16 -> 18 lights up two columns of a
        // 64-wide reserve that already existed and were already zero,
        // `GENOME_LEN` is unchanged at 12,352, and not one existing weight
        // moves. This is exactly the append S2 reserved the dimensions for.
        assert_eq!(genome_manifest(), 1_520_499_525);
    }

    #[test]
    fn a_zero_genome_produces_zero_output_and_costs_nothing() {
        let g = vec![0.0; GENOME_LEN];
        let mut state = zero_state();
        let (out, active) = eval_brain(&g, &[1.0; BRAIN_INPUTS], &mut state);
        assert_eq!(out, [0.0; BRAIN_OUTPUTS]);
        assert_eq!(active, 0, "a brain with no connections must not be taxed for any");
    }

    #[test]
    fn one_authored_connection_moves_its_own_output_in_its_own_direction() {
        let g = genome_from_instincts(&[Instinct(BrainInput::PheroBLateral, BrainOutput::Turn, 0.9)]);
        let mut inputs = [0.0; BRAIN_INPUTS];
        inputs[BrainInput::PheroBLateral as usize] = 1.0;

        let mut state = zero_state();
        let (out, active) = eval_brain(&g, &inputs, &mut state);
        assert!(out[BrainOutput::Turn as usize] > 0.0, "a positive gain on a positive input should turn positive");
        for o in [BrainOutput::Move, BrainOutput::EmitA, BrainOutput::EmitB, BrainOutput::Dig, BrainOutput::Drop] {
            assert_eq!(out[o as usize], 0.0, "{o:?} should be untouched by a connection that does not reach it");
        }
        assert_eq!(active, 1);

        // And the sign follows the input, not just the weight.
        inputs[BrainInput::PheroBLateral as usize] = -1.0;
        let (out, _) = eval_brain(&g, &inputs, &mut zero_state());
        assert!(out[BrainOutput::Turn as usize] < 0.0);
    }

    #[test]
    fn sub_epsilon_weights_are_neither_evaluated_nor_taxed() {
        // The delete-a-connection mechanism. A weight below W_EPS must be
        // *absent*, not merely small, or evolution can never remove one.
        let mut g = vec![0.0; GENOME_LEN];
        g[io_slot(BrainInput::Bias, BrainOutput::Move)] = W_EPS * 0.5;
        let (out, active) = eval_brain(&g, &[1.0; BRAIN_INPUTS], &mut zero_state());
        assert_eq!(active, 0, "a sub-epsilon weight must not be charged for");
        assert_eq!(out[BrainOutput::Move as usize], 0.0, "and must not contribute");

        g[io_slot(BrainInput::Bias, BrainOutput::Move)] = W_EPS * 2.0;
        let (out, active) = eval_brain(&g, &[1.0; BRAIN_INPUTS], &mut zero_state());
        assert_eq!(active, 1);
        assert!(out[BrainOutput::Move as usize] > 0.0);
    }

    #[test]
    fn recurrence_reads_last_ticks_activations_not_this_ticks() {
        // **P-17, and it is the one that fails invisibly.** With the state
        // updated too early the network still runs and still produces
        // plausible outputs; it is simply feed-forward, and the memory the
        // recurrence exists to provide never happens.
        //
        // Built as a pulse: one tick of input, then silence. A brain with
        // working memory still has a nonzero hidden activation on the
        // silent ticks; a feed-forward one is back to exactly zero
        // immediately.
        let mut g = vec![0.0; GENOME_LEN];
        g[ih_slot(BrainInput::PheroAFront, 0)] = 1.0; // input -> hidden 0
        g[hh_slot(0)] = 0.9; // hidden 0 self-recurrence
        g[ho_slot(0, BrainOutput::Move)] = 1.0; // hidden 0 -> Move

        let mut state = zero_state();
        let mut pulse = [0.0; BRAIN_INPUTS];
        pulse[BrainInput::PheroAFront as usize] = 1.0;
        let (during, _) = eval_brain(&g, &pulse, &mut state);
        assert!(during[BrainOutput::Move as usize] > 0.0, "the pulse itself should drive the output");

        let (after, _) = eval_brain(&g, &[0.0; BRAIN_INPUTS], &mut state);
        assert!(after[BrainOutput::Move as usize] > 0.0, "with the input gone, only memory can hold this above zero -- recurrence is reading the wrong tick");
        assert!(after[BrainOutput::Move as usize] < during[BrainOutput::Move as usize], "and it should be decaying, not held");

        // Several more silent ticks: it must fade rather than latch.
        let mut last = after[BrainOutput::Move as usize];
        for _ in 0..20 {
            let (o, _) = eval_brain(&g, &[0.0; BRAIN_INPUTS], &mut state);
            assert!(o[BrainOutput::Move as usize] < last);
            last = o[BrainOutput::Move as usize];
        }
    }

    #[test]
    fn the_active_count_matches_a_hand_counted_sparse_genome() {
        // The tax has to be charged on what the brain actually is. Three
        // authored input->output links, one input->hidden, one recurrence,
        // one hidden->output: six.
        let mut g = genome_from_instincts(&[
            Instinct(BrainInput::Bias, BrainOutput::Move, 0.7),
            Instinct(BrainInput::FoodAdjacent, BrainOutput::Move, -0.8),
            Instinct(BrainInput::Carrying, BrainOutput::EmitB, 0.9),
        ]);
        // **Through the named accessors, not hand arithmetic.** These three
        // lines were `IO_END + input * BRAIN_HIDDEN + 1`, `IH_END + 1` and
        // `HH_END + BRAIN_OUTPUTS + output` — stride expressions that did not
        // match `eval_brain`'s indexing and survived only because the old
        // constants made them land somewhere live by coincidence. The
        // 64/64/64 re-lay broke them, which is precisely what `io_slot` and
        // friends exist to prevent: *"every caller outside `eval_brain`'s
        // inner loops goes through these, so a future re-lay is one edit
        // rather than a hunt through hand-written index expressions."*
        g[ih_slot(BrainInput::Crowding, 1)] = 0.5;
        g[hh_slot(1)] = -0.4;
        g[ho_slot(1, BrainOutput::Turn)] = 0.6;

        let (_, active) = eval_brain(&g, &[1.0; BRAIN_INPUTS], &mut zero_state());
        assert_eq!(active, 6);
    }

    #[test]
    fn squash_saturates_without_leaving_its_range() {
        assert_eq!(squash(0.0), 0.0);
        for x in [-1000.0f32, -4.0, -0.5, 0.5, 4.0, 1000.0] {
            let y = squash(x);
            assert!(y > -1.0 && y < 1.0, "squash({x}) = {y} left (-1, 1)");
            assert_eq!(y.signum(), x.signum());
        }
        assert!(squash(4.0) > squash(1.0), "and it must stay monotone across the clamp range mutation uses");
    }

    #[test]
    fn instincts_land_in_the_slots_their_names_claim() {
        // The wiring list is only readable if the names mean what they say;
        // an off-by-one in the index arithmetic would produce a working
        // brain that does something else entirely.
        let g = genome_from_instincts(&[Instinct(BrainInput::AtNest, BrainOutput::Drop, 0.9)]);
        assert_eq!(g[io_slot(BrainInput::AtNest, BrainOutput::Drop)], 0.9);
        assert_eq!(g.iter().filter(|w| **w != 0.0).count(), 1, "an authored list must write exactly the connections it names");
        assert!(g[IO_END..].iter().all(|w| *w == 0.0), "hidden blocks must start silent");
    }

    // --- the mutation operator (S6, decision E6) ------------------------

    /// **A mutation never touches a reserved slot.**
    ///
    /// `live_slots`' own doc states the failure: a perturbed reserve is
    /// invisible for exactly as long as its slot is unnamed, and then
    /// springs to life as a connection nobody authored, in every
    /// individual descended from the one that was perturbed. Put the fault
    /// back by iterating `0..GENOME_LEN` and this goes red immediately —
    /// at rate 1.0 essentially every reserved slot moves.
    #[test]
    fn mutation_leaves_every_reserved_slot_at_zero() {
        let mut g = vec![0.0f32; GENOME_LEN];
        let mut rng = crate::sim::rng::Rng::new(0xA17);
        let moved = mutate(&mut g, 1.0, &mut rng);
        assert!(moved > 0, "a rate of 1.0 moved nothing at all; the operator did not run");
        assert!(reserve_is_zero(&g), "mutation wrote into a reserved slot");
        assert_eq!(g.iter().filter(|w| **w != 0.0).count() as u32, moved, "`moved` must count the slots that actually changed");
    }

    /// **The width is `MUT_ABS_FLOOR + MUT_REL * |w|`, and the clamp is
    /// ±`MUT_CLAMP`.**
    ///
    /// A tight assertion on a deterministic function, so it is exempt from
    /// the put-the-fault-back rule by `CLAUDE.md`'s own second exemption —
    /// but it is the one that would catch §7a's retired ±4.0 clamp coming
    /// back, which would delete the ant's ±30 homing gate on the first
    /// birth.
    #[test]
    fn one_mutation_moves_a_weight_by_at_most_its_own_width() {
        let big = live_slots().next().expect("a genome has live slots");
        let small = live_slots().nth(1).expect("a genome has at least two live slots");
        for seed in 1..200u64 {
            let mut g = vec![0.0f32; GENOME_LEN];
            g[big] = 30.0;
            g[small] = 0.3;
            let mut rng = crate::sim::rng::Rng::new(seed);
            mutate(&mut g, 1.0, &mut rng);
            let width_big = MUT_ABS_FLOOR + MUT_REL * 30.0;
            let width_small = MUT_ABS_FLOOR + MUT_REL * 0.3;
            assert!((g[big] - 30.0).abs() <= width_big + 1e-4, "the ±30 gate moved {} on seed {seed}, wider than {width_big}", g[big] - 30.0);
            assert!((g[small] - 0.3).abs() <= width_small + 1e-4, "a 0.3 weight moved {} on seed {seed}, wider than {width_small}", g[small] - 0.3);
            assert!(g.iter().all(|w| w.abs() <= MUT_CLAMP + 1e-4), "a weight escaped the clamp on seed {seed}");
        }
    }

    /// **A zero weight can become a connection, and that is what the
    /// absolute floor is for** (P-18).
    ///
    /// Proportional-only mutation of zero is zero for ever, so without
    /// this a lineage's wiring could only ever be a subset of its
    /// ancestor's. The bar is `W_EPS`, because below it `eval_brain` skips
    /// the slot entirely — a weight that never crosses it is not a
    /// connection however non-zero it is.
    #[test]
    fn a_dead_slot_can_be_woken_by_one_mutation() {
        let slot = live_slots().next().expect("a genome has live slots");
        let mut woken = 0;
        for seed in 1..500u64 {
            let mut g = vec![0.0f32; GENOME_LEN];
            let mut rng = crate::sim::rng::Rng::new(seed);
            mutate(&mut g, 1.0, &mut rng);
            if g[slot].abs() >= W_EPS {
                woken += 1;
            }
        }
        assert!(woken > 100, "only {woken} of 499 single mutations of a dead slot cleared W_EPS; the absolute floor is decorative");
    }

    /// **The rate changes how many draws this takes, which is why the
    /// caller must never share its generator.**
    ///
    /// `mutate` rolls once per live slot to decide whether to touch it and
    /// once more for each slot it does, so the generator it is handed ends
    /// in a rate-dependent state. A caller that passed in the generator it
    /// goes on using would therefore have every subsequent decision
    /// shifted by an amount depending on how many slots happened to
    /// mutate — `CLAUDE.md`'s shared-`Rng` gotcha, whose last instance in
    /// this repo stayed green through *both* of its guards. Nothing can
    /// stop a future caller doing that; what this test does is make the
    /// property it would violate explicit and checked, so the comment at
    /// `creature::RNG_SLOT_BIRTH` is standing on a measurement.
    #[test]
    fn the_mutation_rate_changes_how_many_draws_it_takes() {
        let state_after = |rate: f32| {
            let mut g = vec![0.0f32; GENOME_LEN];
            let mut rng = crate::sim::rng::Rng::new(0x5EED);
            mutate(&mut g, rate, &mut rng);
            rng.next_u64()
        };
        assert_ne!(
            state_after(0.0),
            state_after(0.5),
            "mutate left the caller's generator in the same state at two rates; either it takes no draws or this test is not exercising it"
        );
    }

    /// **A rate of zero is a clone, exactly.**
    ///
    /// This is the clonal control the whole lineage-share measurement is
    /// read against, so "no mutation" has to mean bit-identical rather
    /// than approximately unchanged.
    #[test]
    fn a_zero_rate_returns_the_genome_untouched() {
        let mut g = genome_from_instincts(&[Instinct(BrainInput::AtNest, BrainOutput::Drop, 0.9)]);
        let before = g.clone();
        let mut rng = crate::sim::rng::Rng::new(7);
        assert_eq!(mutate(&mut g, 0.0, &mut rng), 0);
        assert_eq!(g, before, "a zero mutation rate changed the genome");
    }
}
