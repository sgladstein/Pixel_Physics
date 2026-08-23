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

use serde::Deserialize;

pub const BRAIN_INPUTS: usize = 16;
pub const BRAIN_HIDDEN: usize = 4;
pub const BRAIN_OUTPUTS: usize = 10;

/// **Reserved storage dimensions.** The live counts above say how much of
/// the scaffold is wired; these say how much room the layout leaves it to
/// grow into. Both are needed, and conflating them is exactly what made the
/// 6 -> 9 output growth unlawful — see `GENOME_LEN`.
///
/// Sized to absorb three appends on each axis before a migration, at 584
/// floats against the 248 a tight layout uses: 2.3 KB per creature, ~9.6 MB
/// at the 4095-organism ceiling. Reserving more (16 outputs, 712 floats)
/// buys no additional lawfulness, only more headroom.
pub const INPUT_SLOTS: usize = 24;
pub const HIDDEN_SLOTS: usize = 8;
pub const OUTPUT_SLOTS: usize = 12;

/// The genome: one flat `Vec<f32>` in four contiguous positional blocks,
/// **sized from the reserved dimensions rather than from the live counts**.
///
/// ```text
/// [0..288)    input -> output   (12 x 24)  -- the authored "taxis gains"
/// [288..480)  input -> hidden   ( 8 x 24)
/// [480..488)  hidden self-recurrence       (8)
/// [488..584)  hidden -> output  (12 x  8)
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
pub const GENOME_LEN: usize = HH_END + OUTPUT_SLOTS * HIDDEN_SLOTS; // 584

const IO_END: usize = OUTPUT_SLOTS * INPUT_SLOTS; // 288
const IH_END: usize = IO_END + HIDDEN_SLOTS * INPUT_SLOTS; // 480
const HH_END: usize = IH_END + HIDDEN_SLOTS; // 488

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
];
pub const OUTPUT_NAMES: [&str; BRAIN_OUTPUTS] = ["Turn", "Move", "EmitA", "EmitB", "Dig", "Drop", "Persist", "Tumble", "Caution", "Feed"];

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
#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
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
}

/// Which output slot. Positional and append-only, as above.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
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
}

/// One authored connection, as a species file writes it:
/// `(PheroBLateral, Turn, 0.9)`.
///
/// A **sparse** list rather than 168 raw floats, because 168 raw floats is
/// not something anyone can review, and the whole value of authored
/// instincts is that a human can see what the animal starts out believing.
/// Anything not listed is 0.0, which under `W_EPS` means "no connection".
#[derive(Clone, Copy, Deserialize)]
pub struct Instinct(pub BrainInput, pub BrainOutput, pub f32);

/// One authored connection into a hidden unit: `(Carrying, 0, 5.0)`.
#[derive(Clone, Copy, Deserialize)]
pub struct HiddenWire(pub BrainInput, pub u8, pub f32);

/// One authored connection out of a hidden unit: `(0, Turn, 1.0)`.
#[derive(Clone, Copy, Deserialize)]
pub struct OutputWire(pub u8, pub BrainOutput, pub f32);

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
pub fn genome_from_wiring(instincts: &[Instinct], hidden: &[HiddenWire], outputs: &[OutputWire]) -> Vec<f32> {
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
    debug_assert!(reserve_is_zero(&g), "authored wiring wrote into a reserved slot");
    g
}

/// The input→output-only form, for callers with nothing in the hidden layer.
pub fn genome_from_instincts(instincts: &[Instinct]) -> Vec<f32> {
    genome_from_wiring(instincts, &[], &[])
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
        assert_eq!(IO_END, 288);
        assert_eq!(IH_END, 480);
        assert_eq!(HH_END, 488);
        assert_eq!(GENOME_LEN, HH_END + OUTPUT_SLOTS * HIDDEN_SLOTS);
        assert_eq!(GENOME_LEN, 584);
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
    fn the_genome_manifest_is_pinned() {
        // Not a checksum for its own sake: this is the thing that fails when
        // a slot is renamed or reordered, which is otherwise a silent
        // reinterpretation of every stored individual. If this assertion
        // fires, either put the slot back or accept that every genome in
        // flight now means something else -- and say which in the commit.
        assert_eq!(genome_manifest(), 2_369_832_241);
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
        g[IO_END + BrainInput::Crowding as usize * BRAIN_HIDDEN + 1] = 0.5;
        g[IH_END + 1] = -0.4;
        g[HH_END + BRAIN_OUTPUTS + BrainOutput::Turn as usize] = 0.6;

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
}
