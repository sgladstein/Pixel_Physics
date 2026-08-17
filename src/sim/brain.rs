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
pub const BRAIN_OUTPUTS: usize = 6;

/// The genome: one flat `Vec<f32>` in four contiguous positional blocks.
///
/// ```text
/// [0..96)     input -> output   (16 x 6)  -- the authored "taxis gains"
/// [96..160)   input -> hidden   (16 x 4)
/// [160..164)  hidden self-recurrence      (4)
/// [164..188)  hidden -> output  (4 x 6)
/// ```
///
/// **Grown once, by appending inputs 14 and 15**, after the lateral
/// sensors turned out to be identically zero on a horizontal surface (see
/// `BrainInput::PheroAAlong`). Every pre-existing slot kept its index and
/// its meaning, which is what the law below actually protects; the flat
/// block offsets moved, which is unavoidable when the scaffold grows and
/// is harmless only because nothing persists a genome yet. **Once stage 4
/// puts heritable genomes in flight this becomes a migration, not an
/// edit.**
///
/// **Slots are positional and must never be renumbered or reordered.** The
/// same law the plant genome already lives under (`organism.rs`'s
/// `genotype_draws`), for the same reason: the slot index *is* the meaning.
/// A stored genome is a list of numbers with no labels, so moving a slot
/// silently reinterprets every individual that already exists as a
/// different animal. Growing the scaffold later means **appending** blocks,
/// never inserting.
pub const GENOME_LEN: usize = 188;

const IO_END: usize = BRAIN_INPUTS * BRAIN_OUTPUTS; // 96
const IH_END: usize = IO_END + BRAIN_INPUTS * BRAIN_HIDDEN; // 160
const HH_END: usize = IH_END + BRAIN_HIDDEN; // 164

/// A weight whose magnitude is below this is **no connection**: skipped in
/// evaluation *and* exempt from the synapse tax.
///
/// This is what gives evolution a real way to *delete* a connection. Without
/// it a weight can only ever get small, never absent, so a brain accumulates
/// vestigial links that each cost a multiply and a metabolic charge forever.
pub const W_EPS: f32 = 0.01;

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
        g[input as usize * BRAIN_OUTPUTS + output as usize] = weight;
    }
    for &HiddenWire(input, h, weight) in hidden {
        assert!((h as usize) < BRAIN_HIDDEN, "hidden unit {h} does not exist; there are {BRAIN_HIDDEN}");
        g[IO_END + input as usize * BRAIN_HIDDEN + h as usize] = weight;
    }
    for &OutputWire(h, output, weight) in outputs {
        assert!((h as usize) < BRAIN_HIDDEN, "hidden unit {h} does not exist; there are {BRAIN_HIDDEN}");
        g[HH_END + h as usize * BRAIN_OUTPUTS + output as usize] = weight;
    }
    g
}

/// The input→output-only form, for callers with nothing in the hidden layer.
pub fn genome_from_instincts(instincts: &[Instinct]) -> Vec<f32> {
    genome_from_wiring(instincts, &[], &[])
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
            let w = ih[i * BRAIN_HIDDEN + h];
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
            let w = io[i * BRAIN_OUTPUTS + o];
            if w.abs() >= W_EPS {
                sum += w * input;
                active += 1;
            }
        }
        for (h, &hidden) in hidden.iter().enumerate() {
            let w = ho[h * BRAIN_OUTPUTS + o];
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
        assert_eq!(IO_END, 96);
        assert_eq!(IH_END, 160);
        assert_eq!(HH_END, 164);
        assert_eq!(GENOME_LEN, HH_END + BRAIN_HIDDEN * BRAIN_OUTPUTS);
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
        g[BrainInput::Bias as usize * BRAIN_OUTPUTS + BrainOutput::Move as usize] = W_EPS * 0.5;
        let (out, active) = eval_brain(&g, &[1.0; BRAIN_INPUTS], &mut zero_state());
        assert_eq!(active, 0, "a sub-epsilon weight must not be charged for");
        assert_eq!(out[BrainOutput::Move as usize], 0.0, "and must not contribute");

        g[BrainInput::Bias as usize * BRAIN_OUTPUTS + BrainOutput::Move as usize] = W_EPS * 2.0;
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
        g[IO_END + BrainInput::PheroAFront as usize * BRAIN_HIDDEN] = 1.0; // input -> hidden 0
        g[IH_END] = 0.9; // hidden 0 self-recurrence
        g[HH_END + BrainOutput::Move as usize] = 1.0; // hidden 0 -> Move

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
        assert_eq!(g[BrainInput::AtNest as usize * BRAIN_OUTPUTS + BrainOutput::Drop as usize], 0.9);
        assert_eq!(g.iter().filter(|w| **w != 0.0).count(), 1, "an authored list must write exactly the connections it names");
        assert!(g[IO_END..].iter().all(|w| *w == 0.0), "hidden blocks must start silent");
    }
}
