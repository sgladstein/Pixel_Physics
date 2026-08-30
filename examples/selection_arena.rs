//! **Does this world punish a plant that is worse?**
//!
//! The teeth-test. Two genomes compete in **one** world for the same light,
//! water and space, and the readout is which one owns the world at the end.
//!
//! ```text
//! cargo run --release --example selection_arena -- arm=lethal seeds=18
//! cargo run --release --example selection_arena -- arm=same          # the control; must read ~0
//! cargo run --release --example selection_arena -- arm=nobranch relief=varied
//! ```
//!
//! # Why this exists, and what it is *not*
//!
//! `fate_viability` asks whether a mutant lives; `genome_drift` asks whether
//! a population's genome moves. Neither asks whether one genome **beats**
//! another, and `Reports/plant-fate-operator-gate-2026-08-29.md` §6 records
//! the gap in as many words: *"the gate asks whether a mutant lives, never
//! whether it wins, and no experiment in this repo yet asks the second
//! question for plants (that needs competing arms in one world, read at an
//! order statistic)"*. This is that experiment.
//!
//! **It is aimed at the environment, not at the genome.** The worry it was
//! built for is a false negative: run an evolution experiment in a world
//! whose selective pressures are incomplete, measure nothing, and write down
//! *"evolution does not work"* when the true finding is *"this world does not
//! select"*. Those are different failures and they are separable, because a
//! genome that is **known** to be worse gives the world something it must be
//! able to punish:
//!
//! - a known-bad arm loses -> the environment has teeth, and a later null on
//!   a subtle mutation means something;
//! - a known-bad arm does **not** lose -> the environment has no teeth, and
//!   no evolution result from this world can be trusted until that is fixed.
//!
//! So a null here is a finding about the *world*. That is the whole point.
//!
//! # The three things this is shaped around
//!
//! **1. One world, and the mirror run that makes it fair.** Competition needs
//! a shared resource pool, so both arms must stand in the same bed --
//! `divergence` deliberately uses two worlds and says why, and that choice is
//! right for its question and wrong for this one. But `divergence`'s reason
//! bites here too: genotype draws come from `(world seed, germination
//! coordinate)`, so a founder at x=100 and one at x=200 are **different
//! individuals** before either genome is applied, and a bed is not
//! left-right symmetric either. So every scene is run **twice** with the arm
//! assignment mirrored (A,B,A,B... then B,A,B,A...) and the pair pooled.
//! Each position is then occupied by both arms across the pair, at the
//! identical genotype draw, and both the position effect and the draw effect
//! cancel exactly rather than approximately.
//!
//! **2. The control comes first -- and it must be run `mirror=off`, because
//! mirrored it is vacuous.** Two identical genomes must read ~50/50. But
//! with `arm=same` the mirrored pair is *the same simulation with the labels
//! swapped*: arm A is the even founders in one run and the odd founders in
//! the other, so pooling gives `A == B` as an algebraic identity. Measured
//! on the first run of this harness -- **exactly 50.0% on every seed and
//! both metrics**, which is the tidiness `CLAUDE.md` warns is evidence of an
//! artifact before it is evidence of anything. It was not wrong, it was
//! **vacuous**: a control that cannot fail is not a control.
//!
//! So the control that means something is `arm=same mirror=off`, which
//! leaves the position and genotype confounds *in* and asks whether they
//! alone manufacture a winner. That number is also the size of the thing the
//! mirror exists to cancel, which is worth knowing on its own.
//!
//! **3. Attribution is by lineage, never by genome.** Every descendant
//! carries its founder's `OrganismState::lineage`, so an arm's share is
//! countable even when mutation has moved a descendant's genome -- and, the
//! case that actually forces it, even when the two arms are the *same
//! genome*, which is precisely the control above. Classifying by genome
//! would make the control unmeasurable, which is the one arm that must work.
//!
//! # Reading it
//!
//! The headline is **how many seeds moved the same way**, not a difference of
//! means: twelve identical trees from one genome span 31 to 153 cells, so a
//! mean over that spread is not a result. Per-seed shares are printed
//! individually and the quartiles sit beside the median.

mod common;

use pixel_physics::sim::organism::{CellType, FateGenome, FateWhen};
use pixel_physics::sim::parallel;
use pixel_physics::sim::plant;

fn arg<T: std::str::FromStr>(name: &str) -> Option<T>
where
    T::Err: std::fmt::Debug,
{
    std::env::args().find_map(|a| a.strip_prefix(&format!("{name}=")).map(|v| v.parse().expect(name)))
}

fn arg_str(name: &str) -> Option<String> {
    std::env::args().find_map(|a| a.strip_prefix(&format!("{name}=")).map(str::to_string))
}

/// **How arm B differs from the species it is founded on.**
///
/// Each is a deliberate handicap of known direction except `Same` (no
/// change) and `Early` (genuinely ambiguous, and included because a ladder
/// whose every rung points the same way cannot show where the world stops
/// discriminating).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Handicap {
    /// No change at all. **The control.** Must read a ~50/50 share.
    Same,
    /// The shoot's `Grew` rule builds a `Seed` instead of a shoot child, so
    /// the axis cannot extend. `fate_viability` uses this exact edit as its
    /// mandatory negative control and measures it at 0 of 3 establishing, so
    /// it is the strongest handicap available and the minimum the world must
    /// be able to punish.
    ///
    /// Found by `when == Grew` rather than by index, for the reason that
    /// harness records: a determinate base carries an extra rule ahead of it
    /// and a control that poisons the wrong rule fails **open**.
    Lethal,
    /// A dormant bud matures instead of flushing into a growing tip, so the
    /// shoot cannot branch. Alive, and should carry much less leaf.
    NoBranch,
    /// The same handicap on the root: laterals are placed as `MatureBody`,
    /// so there is one descending axis and no spread.
    NoRootBranch,
    /// The determinate node fires at 2 metamers instead of its authored 8 --
    /// a much smaller plant that reproduces much sooner. **Direction
    /// unknown**, which is why it is here.
    Early,
}

impl Handicap {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "same" => Self::Same,
            "lethal" => Self::Lethal,
            "nobranch" => Self::NoBranch,
            "norootbranch" => Self::NoRootBranch,
            "early" => Self::Early,
            _ => return None,
        })
    }

    /// Apply this handicap to a base genome, returning the arm-B genome and
    /// whether the edit actually found its target.
    ///
    /// **The `applied` flag is not decoration.** An edit that silently
    /// matches nothing produces an arm-B identical to arm-A, which reads as
    /// a clean 50/50 -- indistinguishable from "the world has no teeth", and
    /// the more comforting of the two readings. `main` refuses to run when
    /// it comes back false.
    fn apply(self, base: FateGenome) -> (FateGenome, bool) {
        if self == Self::Same {
            return (base, true);
        }
        let mut table = base.to_table();
        let mut applied = false;
        for (owner, rules) in table.iter_mut() {
            for f in rules.iter_mut() {
                let hit = match self {
                    Self::Same => false,
                    Self::Lethal => *owner == CellType::GrowingTip && f.when == FateWhen::Grew,
                    // **The bud, not the shoot's lateral.** Two vacuous
                    // arms were spent before this was measured: a `herb`
                    // shoot never places a lateral at all (editing its type
                    // is byte-identical to the control on every seed) while
                    // the *root* does, so the two halves of one plant reach
                    // branching by different routes. A shoot branches only
                    // when a `DormantBud` flushes into a tip, so that is the
                    // rule to poison.
                    Self::NoBranch => *owner == CellType::DormantBud && f.when == FateWhen::Flush,
                    Self::NoRootBranch => *owner == CellType::RootTip && f.lateral.is_some(),
                    Self::Early => {
                        *owner == CellType::GrowingTip && f.when == FateWhen::Node && f.after_metamers.is_some()
                    }
                };
                if !hit {
                    continue;
                }
                match self {
                    Self::Same => {}
                    Self::Lethal => f.child = Some(CellType::Seed),
                    // **A lateral made of mature tissue, not `None`.**
                    // `None` is not "no lateral" and this cost two vacuous
                    // arms before it was read: `plant.rs`'s Grow arm does
                    // `fate.and_then(|f| f.lateral).unwrap_or(cell_type)`, so
                    // clearing the field falls back to *the growing cell's
                    // own type* -- which on a `GrowingTip` is exactly the
                    // `Some(GrowingTip)` that was already there. The field
                    // says what a lateral IS, never whether there is one.
                    // Creating it as `MatureBody` is the real handicap: the
                    // lateral is placed and is a dead end, so the axis cannot
                    // branch.
                    Self::NoBranch => f.becomes = CellType::MatureBody,
                    // **A lateral made of mature tissue, not `None`.**
                    // `None` is not "no lateral": `plant.rs`'s Grow arm does
                    // `fate.and_then(|f| f.lateral).unwrap_or(cell_type)`, so
                    // clearing the field falls back to *the growing cell's
                    // own type* -- on a `RootTip` exactly the
                    // `Some(RootTip)` already there. The field says what a
                    // lateral IS, never whether there is one.
                    Self::NoRootBranch => f.lateral = Some(CellType::MatureBody),
                    Self::Early => f.after_metamers = Some(2),
                }
                applied = true;
            }
        }
        (FateGenome::from_table(&table), applied)
    }
}

/// One arm's tally at one instant.
#[derive(Default, Clone, Copy)]
struct Tally {
    organisms: usize,
    cells: usize,
    seeds_set: u64,
}

/// The outcome of one world: arm A's tally and arm B's.
struct Outcome {
    a: Tally,
    b: Tally,
    /// Descendants ever seen, per arm, accumulated over samples.
    ever: (usize, usize),
    /// **The frequency trajectory**: `(population mean generation, arm B's
    /// share of biomass)` at each sample.
    ///
    /// This is the half of the harness that can see a *small* selection
    /// coefficient, and the endpoint share above is the half that cannot.
    /// The arithmetic, which is why it is worth carrying:
    ///
    /// Under neutral drift the log-odds of an arm's frequency random-walks
    /// with `sd ~= sqrt(g / Ne)` after `g` generations, while selection at
    /// `s` per generation moves the mean by `s * g`. So the signal-to-noise
    /// ratio is `s * sqrt(g * Ne)` -- it grows with the *length* of the run,
    /// not with how many worlds are run. Detecting `s` at |z|>2 needs
    /// `g * Ne > 4 / s^2`:
    ///
    /// | s | generations at Ne=500 |
    /// |---|---|
    /// | 0.05 | 3 |
    /// | 0.02 | 20 |
    /// | **0.01** | **80** |
    ///
    /// At herb's ~6,400 frames per generation that is a single ~512,000-frame
    /// run for `s = 1%`, against the **620 worlds** the endpoint share needs
    /// for the same coefficient. Sampling more often does not help -- samples
    /// along one trajectory are autocorrelated; running *longer* does.
    traj: Vec<(f64, f64)>,
    /// The population's mean generation at the last sample -- the x-value the
    /// trajectory's slope is measured against, carried out so the two halves
    /// of this harness can be checked against each other.
    final_gen: f64,
}

/// **Least-squares slope of `logit(freq)` against generation** -- the
/// selection coefficient per generation, signed so that negative means arm B
/// is losing.
///
/// Log-odds rather than the raw share because that is the scale selection is
/// linear on: a constant fitness ratio per generation moves log-odds by a
/// constant amount, while the share itself saturates near 0 and 1 and would
/// make a strong coefficient read as a decelerating one.
///
/// Samples at exactly 0 or 1 are dropped rather than clamped -- an arm that
/// has been eliminated has no defined log-odds, and clamping invents a finite
/// value whose magnitude is set by the clamp rather than by the data.
/// Returns `None` below three usable samples.
fn logit_slope(traj: &[(f64, f64)]) -> Option<(f64, f64)> {
    let pts: Vec<(f64, f64)> = traj
        .iter()
        .filter(|&&(_, p)| p > 1e-6 && p < 1.0 - 1e-6)
        .map(|&(g, p)| (g, (p / (1.0 - p)).ln()))
        .collect();
    if pts.len() < 3 {
        return None;
    }
    let n = pts.len() as f64;
    let mx = pts.iter().map(|p| p.0).sum::<f64>() / n;
    let my = pts.iter().map(|p| p.1).sum::<f64>() / n;
    let sxx: f64 = pts.iter().map(|p| (p.0 - mx).powi(2)).sum();
    let sxy: f64 = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    // Returns `(slope, intercept)`. **The intercept is not decoration**: both
    // arms start equal, so the log-odds at generation 0 is 0 by construction,
    // and a fitted intercept far from 0 means the trajectory is *curved* --
    // arm B losing fast during establishment and then levelling, say -- so a
    // single slope is a poor summary of it and `s` is not constant.
    //
    // Omitting it produced a false alarm the first time this harness
    // cross-checked itself: `slope x generations` was compared against the
    // endpoint log-odds as though the line passed through the origin, read
    // -0.676 against -0.426, and looked like the two readouts disagreeing.
    // They did not; the line simply has an intercept.
    (sxx.abs() > 1e-9).then(|| (sxy / sxx, my - (sxy / sxx) * mx))
}

#[allow(clippy::too_many_arguments)]
fn run_world(
    species: &str,
    founders: usize,
    width: i32,
    soil_depth: i32,
    moisture: u16,
    relief_varied: bool,
    worldseed: u64,
    frames: u64,
    every: u64,
    handicap: Handicap,
    mirror: bool,
) -> Outcome {
    let base_scene =
        if relief_varied { common::PlantScene::varied() } else { common::PlantScene::default() };
    let scene = common::PlantScene {
        trees: founders,
        width,
        species: species.to_string(),
        soil_depth,
        soil_moisture: moisture,
        ..base_scene
    };
    let (w_width, w_height) = (scene.width, scene.height);
    let mut w = scene.build();
    w.seed = worldseed;

    let species_id = w.species.id_of(species).expect("species is compiled in");
    let base = FateGenome::from_table(w.species.get(species_id).fate_table());
    let (arm_b, _) = handicap.apply(base);

    // **Find the founders in x order.** `PlantScene` plants them evenly and
    // in index order, but `Relief::Varied` shoves each one, so x order and
    // plant order are not the same thing -- and it is x order that decides
    // who neighbours whom, which is what the mirror has to invert.
    let mut founder_cells: Vec<(i32, u16)> = Vec::new();
    for y in 0..w_height {
        for x in 0..w_width {
            let id = w.get(x, y).organism_id();
            if id != 0 && !founder_cells.iter().any(|&(_, seen)| seen == id) {
                founder_cells.push((x, id));
            }
        }
    }
    founder_cells.sort_by_key(|&(x, _)| x);

    // Assign arms alternately, and let `mirror` invert the assignment.
    let mut arm_of_lineage: std::collections::HashMap<u32, bool> = std::collections::HashMap::new();
    let (mut n_a, mut n_b) = (0usize, 0usize);
    for (i, &(_, id)) in founder_cells.iter().enumerate() {
        let is_b = (i % 2 == 1) != mirror;
        if is_b {
            assert!(w.set_organism_fates(id, arm_b), "the founder must be live when its arm is assigned");
            n_b += 1;
        } else {
            n_a += 1;
        }
        let lineage = w.organism(id).expect("founder is live").lineage;
        arm_of_lineage.insert(lineage, is_b);
    }
    assert!(
        n_a.abs_diff(n_b) <= 1,
        "arms must be balanced: {n_a} A against {n_b} B. An odd founder count leaves one arm ahead, \
         which the mirror run cancels -- but a larger gap means founders failed to plant and the \
         comparison is not what it claims"
    );

    let mut ever_a: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
    let mut ever_b: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
    let mut traj: Vec<(f64, f64)> = Vec::new();
    let mut last = (Tally::default(), Tally::default());

    for f in 0..frames {
        parallel::step(&mut w);
        w.step_active_sites();
        w.step_fields();
        if !(f + 1).is_multiple_of(every) && f + 1 != frames {
            continue;
        }
        let mut a = Tally::default();
        let mut b = Tally::default();
        let mut counted: std::collections::BTreeSet<u16> = std::collections::BTreeSet::new();
        let (mut gen_sum, mut gen_n) = (0u64, 0u64);
        for y in 0..w_height {
            for x in 0..w_width {
                let id = w.get(x, y).organism_id();
                if id == 0 {
                    continue;
                }
                let Some(state) = w.organism(id) else { continue };
                let Some(&is_b) = arm_of_lineage.get(&state.lineage) else { continue };
                let t = if is_b { &mut b } else { &mut a };
                t.cells += 1;
                if counted.insert(id) {
                    t.organisms += 1;
                    t.seeds_set += state.seeds_set as u64;
                    gen_sum += state.generation as u64;
                    gen_n += 1;
                    // `u16` handles are recycled, so an id is only unique
                    // among the live; pair it with the lineage so a reused
                    // handle in a different arm cannot be double-counted.
                    let key = (u32::from(id) << 1) | u32::from(is_b);
                    if is_b {
                        ever_b.insert(key);
                    } else {
                        ever_a.insert(key);
                    }
                }
            }
        }
        // The x-axis is the population's mean generation, not the frame
        // number: selection acts per generation, and two runs that breed at
        // different rates would otherwise be put on incomparable axes.
        let tot = a.cells + b.cells;
        if gen_n > 0 && tot > 0 {
            traj.push((gen_sum as f64 / gen_n as f64, b.cells as f64 / tot as f64));
        }
        last = (a, b);
    }
    let final_gen = traj.last().map_or(0.0, |&(g, _)| g);
    Outcome { a: last.0, b: last.1, ever: (ever_a.len(), ever_b.len()), traj, final_gen }
}

/// **Wilcoxon signed-rank against a 50% null, two-sided.**
///
/// Reported beside the seed count because the seed count throws away most of
/// what each seed measured. A sign test is about **64% efficient** against a
/// t-test where signed-rank is ~95%, so switching statistic is worth roughly
/// 1.5x the effective sample size for no extra compute -- and compute is the
/// binding constraint here, since one seed is two mirrored 20,000-frame runs.
///
/// **Legitimate here specifically because the design is paired.** The mirror
/// makes each seed's share a within-world contrast at the identical genotype
/// draw, so the per-seed values are exchangeable under the null in the way
/// signed-rank needs. It would not be legitimate on unmirrored data, where
/// the position effect is still in every value.
///
/// **It does not rescue the power problem, and must not be read as if it
/// does.** Measured against this harness's own control spread (~9.3
/// share-points per seed), 18 seeds resolves an effect of roughly 7.5 points
/// and is blind below about 5; a selection coefficient small enough to be
/// evolutionarily interesting needs hundreds of seeds by any statistic. The
/// answer to that is a different design -- a frequency trajectory over many
/// generations rather than one endpoint share -- not a better test.
///
/// Returns `(w_plus, z, p)`. Normal approximation with a continuity
/// correction; exact enough from about n = 10 and this is never run below
/// that in earnest.
fn signed_rank(shares: &[f64]) -> (f64, f64, f64) {
    let mut d: Vec<f64> = shares.iter().map(|s| s - 50.0).filter(|x| x.abs() > 1e-9).collect();
    let n = d.len();
    if n < 2 {
        return (f64::NAN, f64::NAN, f64::NAN);
    }
    let mut idx: Vec<usize> = (0..n).collect();
    idx.sort_by(|&i, &j| d[i].abs().partial_cmp(&d[j].abs()).expect("no NaN"));
    // Average ranks over ties, or a run of equal magnitudes biases W.
    let mut rank = vec![0.0f64; n];
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j + 1 < n && (d[idx[j + 1]].abs() - d[idx[i]].abs()).abs() < 1e-9 {
            j += 1;
        }
        let avg = ((i + 1) + (j + 1)) as f64 / 2.0;
        for k in i..=j {
            rank[idx[k]] = avg;
        }
        i = j + 1;
    }
    // `+ 0.0` normalises the negative zero an empty sum can produce, which
    // otherwise prints as `W+=-0.0` and reads as a bug.
    let w_plus: f64 = (0..n).filter(|&i| d[i] > 0.0).map(|i| rank[i]).sum::<f64>() + 0.0;
    let nn = n as f64;
    let mean = nn * (nn + 1.0) / 4.0;
    let sd = (nn * (nn + 1.0) * (2.0 * nn + 1.0) / 24.0).sqrt();
    let z = if sd > 0.0 { ((w_plus - mean).abs() - 0.5) / sd } else { f64::NAN };
    // Two-sided normal tail, via the same erf-free approximation the rest of
    // this file avoids needing: Abramowitz & Stegun 7.1.26 on erfc.
    let p = {
        let x = z / std::f64::consts::SQRT_2;
        let t = 1.0 / (1.0 + 0.327_591_1 * x.abs());
        let poly = t * (0.254_829_592 + t * (-0.284_496_736 + t * (1.421_413_741 + t * (-1.453_152_027 + t * 1.061_405_429))));
        let erfc = poly * (-x * x).exp();
        erfc.clamp(0.0, 1.0)
    };
    d.clear();
    (w_plus, z, p)
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

fn main() {
    let species = arg_str("species").unwrap_or_else(|| "herb".to_string());
    let handicap_name = arg_str("arm").unwrap_or_else(|| "lethal".to_string());
    let handicap = Handicap::parse(&handicap_name)
        .unwrap_or_else(|| panic!("arm={handicap_name} is not one of same|lethal|nobranch|norootbranch|early"));
    let seeds: u64 = arg("seeds").unwrap_or(18);
    let founders: usize = arg("founders").unwrap_or(16);
    let frames: u64 = arg("frames").unwrap_or(30_000);
    let every: u64 = arg("every").unwrap_or(5_000);
    let relief_varied = arg_str("relief").as_deref() == Some("varied");
    // `mirror=off` runs a single assignment instead of the cancelling pair.
    // The only reason to want it is the control above -- see the module doc.
    let mirrored = arg_str("mirror").as_deref() != Some("off");
    // `dump=1` prints the first seed's raw (generation, share) trajectory.
    // **Look at the curve before trusting any summary of it** -- the fitted
    // intercept said this trajectory is not a straight line in log-odds, and
    // a slope is only a fair summary of a straight one.
    let dump = arg_str("dump").as_deref() == Some("1");
    let soil_depth: i32 = arg("soil").unwrap_or(common::SOIL_DEPTH);
    let moisture: u16 = arg("moisture").unwrap_or(pixel_physics::sim::material::SOIL_FIELD_CAPACITY);
    let width: i32 = arg("width").unwrap_or_else(|| {
        let d = common::PlantScene::default();
        d.width * (founders as i32).max(1) / d.trees as i32
    });

    println!(
        "selection_arena: species={species} arm={handicap_name} seeds={seeds} founders={founders} \
         frames={frames} every={every} relief={} mirror={} width={width} soil={soil_depth} moisture={moisture}",
        if relief_varied { "varied" } else { "flat" },
        if mirrored { "on" } else { "OFF" }
    );
    println!(
        "selection_arena: fate_mutation_chance={} fate_lookup={:?}",
        plant::fate_mutation_chance(),
        plant::fate_lookup_mode()
    );

    // **The edit has to have bitten.** An arm-B genome identical to arm-A
    // reads as a clean 50/50, which is exactly what "no teeth" looks like --
    // the comforting reading of a broken harness. Checked before any run.
    {
        let probe = common::PlantScene { trees: 1, species: species.clone(), ..common::PlantScene::default() }.build();
        let id = probe.species.id_of(&species).expect("species is compiled in");
        let base = FateGenome::from_table(probe.species.get(id).fate_table());
        let (arm_b, applied) = handicap.apply(base);
        assert!(
            applied || handicap == Handicap::Same,
            "arm={handicap_name} matched no rule in {species}'s table, so both arms would be identical \
             and the run would report a clean 50/50 -- which is indistinguishable from the finding this \
             harness exists to make"
        );
        if handicap != Handicap::Same {
            assert_ne!(
                arm_b, base,
                "arm={handicap_name} left the genome byte-identical; see above for why that cannot be run"
            );
        }
        println!("selection_arena: arm B differs from arm A: {}", handicap != Handicap::Same);
    }

    println!("\nper seed, arm B's share of the final population (50% = no effect):");
    println!("  {:>5} {:>16} {:>16} {:>14}", "seed", "B share of orgs", "B share of cells", "B seeds set");

    let mut share_orgs: Vec<f64> = Vec::new();
    let mut share_cells: Vec<f64> = Vec::new();
    let mut b_lower = 0usize;
    let mut usable = 0usize;
    // **Silent-arm detector.** `fate_viability` had to split its outcomes
    // three ways -- viable / lethal / *silent* -- because a mutation can
    // change the genome and leave the plant identical, and counting those as
    // tolerance overstates it. The same class bites harder here: a silent
    // arm reads a clean 50/50 under the mirror, which is exactly what "the
    // world has no teeth" looks like. Under the mirror an arm that changes
    // nothing gives A and B *exactly* equal integer tallies, so it is
    // detectable rather than merely suspected.
    let mut exact_ties = 0usize;
    // Per-seed selection coefficient: the mean of the mirrored pair's two
    // slopes, so position and genotype draw cancel here exactly as they do
    // for the endpoint share.
    let mut slopes: Vec<f64> = Vec::new();
    let mut gens_final: Vec<f64> = Vec::new();
    let mut intercepts: Vec<f64> = Vec::new();
    // Every sample's generation, pooled, so the axis's own span can be read.
    let mut traj_span: Vec<f64> = Vec::new();
    for s in 0..seeds {
        // The mirror pair: same world, arm assignment inverted, pooled.
        let mut a = Tally::default();
        let mut b = Tally::default();
        let assignments: &[bool] = if mirrored { &[false, true] } else { &[false] };
        let mut pair_slopes: Vec<f64> = Vec::new();
        let mut pair_gens: Vec<f64> = Vec::new();
        let mut pair_intercepts: Vec<f64> = Vec::new();
        for &mirror in assignments {
            let o = run_world(
                &species, founders, width, soil_depth, moisture, relief_varied, s + 1, frames, every,
                handicap, mirror,
            );
            a.organisms += o.a.organisms;
            a.cells += o.a.cells;
            a.seeds_set += o.a.seeds_set;
            b.organisms += o.b.organisms;
            b.cells += o.b.cells;
            b.seeds_set += o.b.seeds_set;
            if dump && s == 0 && !mirror {
                println!("\n  raw trajectory, seed 1 (generation -> arm B share -> log-odds):");
                for &(g, f) in &o.traj {
                    let lo = if f > 1e-6 && f < 1.0 - 1e-6 { (f / (1.0 - f)).ln() } else { f64::NAN };
                    println!("    gen {g:>5.2}   share {:>5.1}%   logit {lo:+.3}", 100.0 * f);
                }
                println!();
            }
            traj_span.extend(o.traj.iter().map(|&(g, _)| g));
            if let Some((sl, ic)) = logit_slope(&o.traj) {
                pair_slopes.push(sl);
                pair_intercepts.push(ic);
            }
            pair_gens.push(o.final_gen);
            let _ = o.ever;
        }
        let tot_o = a.organisms + b.organisms;
        let tot_c = a.cells + b.cells;
        if tot_o == 0 || tot_c == 0 {
            println!("  {:>5} {:>16} {:>16} {:>14}", s + 1, "-- empty --", "-- empty --", b.seeds_set);
            continue;
        }
        if a.organisms == b.organisms && a.cells == b.cells {
            exact_ties += 1;
        }
        let so = 100.0 * b.organisms as f64 / tot_o as f64;
        let sc = 100.0 * b.cells as f64 / tot_c as f64;
        println!("  {:>5} {:>15.1}% {:>15.1}% {:>14}", s + 1, so, sc, b.seeds_set);
        share_orgs.push(so);
        share_cells.push(sc);
        if !pair_slopes.is_empty() {
            slopes.push(pair_slopes.iter().sum::<f64>() / pair_slopes.len() as f64);
        }
        if !pair_gens.is_empty() {
            gens_final.push(pair_gens.iter().sum::<f64>() / pair_gens.len() as f64);
        }
        if !pair_intercepts.is_empty() {
            intercepts.push(pair_intercepts.iter().sum::<f64>() / pair_intercepts.len() as f64);
        }
        usable += 1;
        if sc < 50.0 {
            b_lower += 1;
        }
    }

    if usable == 0 {
        println!("\nno usable seed -- every world came out empty. Nothing here is a statement about selection.");
        return;
    }

    let (qo_lo, qo_hi) = quartiles(&mut share_orgs.clone());
    let (qc_lo, qc_hi) = quartiles(&mut share_cells.clone());
    println!("\npooled over {usable} seeds (each the mean of a mirrored pair):");
    println!("  B share of organisms  median {:>5.1}%   quartiles {:>5.1}% .. {:>5.1}%", median(&mut share_orgs), qo_lo, qo_hi);
    println!("  B share of cells      median {:>5.1}%   quartiles {:>5.1}% .. {:>5.1}%", median(&mut share_cells), qc_lo, qc_hi);
    // **The headline, and it is a count of seeds rather than a mean.**
    // `divergence` records why: the within-genome spread is so wide that a
    // difference of two means over it is not a result, and "how many seeds
    // moved the same way" is the statistic that survives it.
    println!("\n  seeds where B held LESS than half the biomass: {b_lower} of {usable}");
    println!("  (half is the null. All of them is a world with teeth. Read this line, not the median.)");
    let (w, z, p) = signed_rank(&share_cells);
    println!("  Wilcoxon signed-rank on B's cell share vs 50%: W+={w:.1} z={z:.2} p={p:.4} (two-sided)");
    // **Power, stated for the n actually run rather than for 18.** The
    // control measured a per-seed spread of ~9.3 share-points with no true
    // effect present; 18 seeds resolve about 7.5 points, and the detectable
    // effect scales roughly as 1/sqrt(n) from there. An earlier version of
    // this line printed the 18-seed figure beside whatever `n` the run
    // actually used, which overstates a short run's reach.
    let resolves = 7.5 * (18.0 / usable as f64).sqrt();
    println!(
        "  POWER: the control's own spread is ~9.3 share-points per seed with no true effect, so\n  \
         {usable} seeds resolve an effect of roughly {resolves:.0} points and are blind well below it.\n  \
         An evolutionarily interesting selection coefficient is ~100x smaller than that and needs a\n  \
         different DESIGN -- a frequency trajectory over many generations, not an endpoint share --\n  \
         rather than more seeds. Note the test SATURATES once every seed points one way: at n=18 that\n  \
         floor is p=0.0002, so beyond it the magnitude lives in the median, not the p-value."
    );

    if mirrored && handicap != Handicap::Same && exact_ties == usable && usable > 0 {
        println!(
            "\n  *** ARM B IS SILENT: it changed the genome and did NOT change the plant. ***\n  \
             All {usable} seeds gave A and B exactly equal tallies, which under the mirror is the\n  \
             signature of an arm that behaves identically to the control. This is NOT a finding about\n  \
             the world -- it says the edited field is never read on this species. Do not report the\n  \
             50/50 above as an absence of selection; fix the arm and re-run.\n  \
             (Confirm with: `arm={handicap_name} mirror=off` against `arm=same mirror=off` at the same\n  \
             seeds -- byte-identical output is the proof.)"
        );
    }

    // **Is the generation axis actually a clock?** Measured 2026-08-30 and it
    // is not: over a 150,000-frame run the population's mean generation rose
    // to ~2.9 by frame ~50,000 and then FLATTENED and drifted back down
    // (2.88, 2.85, 2.77, 2.73, 2.63, 2.60). Mean generation is taken over
    // *living* organisms, and at steady state deaths of old plants balance
    // births of new ones, so it equilibrates rather than accumulating.
    //
    // That breaks this readout's premise outright. The power argument
    // (`g*Ne > 4/s^2`, ~80 generations for s=0.01) needs `g` to grow without
    // bound; against a saturated axis `s*g` cannot grow however long the run,
    // and a longer run buys nothing at all. The fix is a **cumulative**
    // generation clock -- deepest generation reached, or cumulative births
    // over standing population -- not a longer run.
    //
    // Detected rather than left for a reader to notice, because a slope
    // fitted against a saturated axis is a real number about nothing.
    let gen_span = {
        let xs: Vec<f64> = traj_span.clone();
        match (xs.iter().cloned().fold(f64::INFINITY, f64::min), xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max)) {
            (lo, hi) if lo.is_finite() && hi.is_finite() => hi - lo,
            _ => f64::NAN,
        }
    };
    if gen_span.is_finite() && gen_span < 3.0 {
        println!(
            "\n  *** THE GENERATION AXIS IS SATURATED: it spans only {gen_span:.2} generations. ***\n  \
             Mean generation is taken over LIVING organisms, so at steady state it equilibrates\n  \
             instead of accumulating -- it is not a clock, and a longer run does not lengthen it.\n  \
             Any slope below is fitted against an axis that does not move, and the g*Ne > 4/s^2\n  \
             power argument is unreachable on it. Needs a CUMULATIVE generation clock (deepest\n  \
             generation reached, or cumulative births over standing population), not more frames."
        );
    }

    // **The trajectory readout -- the half that can see a small coefficient.**
    if slopes.len() >= 3 {
        let mut sl = slopes.clone();
        let (q_lo, q_hi) = quartiles(&mut sl.clone());
        let med = median(&mut sl);
        // Offset by 50 so the same signed-rank helper (which tests against a
        // 50% null) tests these against zero.
        let (_, z, p) = signed_rank(&slopes.iter().map(|v| 50.0 + v).collect::<Vec<_>>());
        println!("\nselection coefficient per generation, from the frequency trajectory:");
        println!("  s (median over {} seeds)  {med:+.4}   quartiles {q_lo:+.4} .. {q_hi:+.4}", slopes.len());
        println!("  signed-rank vs 0: z={z:.2} p={p:.4}");
        println!("  Negative means arm B loses ground per generation. THIS is the readout that scales:");
        println!("  detecting s needs g*Ne > 4/s^2, so it improves with RUN LENGTH (generations reached)");
        println!("  where the endpoint share improves only with seed count. At Ne~500, s=0.05 needs ~3");
        println!("  generations and s=0.01 needs ~80 -- one long run against ~620 worlds.");
        if med.abs() > 1e-6 {
            let gens = 4.0 / med.powi(2) / 500.0;
            if gens < 1.0 {
                println!("  (a coefficient this size is resolvable in under one generation at Ne=500)");
            } else {
                println!("  (a coefficient this size is resolvable in ~{gens:.0} generations at Ne=500)");
            }
        }
        // **The two halves of this harness, checked against each other.**
        // The endpoint share and the trajectory slope are separate
        // measurements of one thing, so `slope x generations` must reproduce
        // the endpoint log-odds. They are not independent evidence -- they
        // come from the same runs -- but a disagreement means one of the two
        // is wrong, and without this line the discrepancy is invisible.
        //
        // It has already earned its place: the slope first read -0.35 against
        // a -0.23 predicted by hand from the endpoint, and the gap was
        // entirely a wrong guess at the mean generation (assumed ~2.0, really
        // ~1.3). Printing the generation makes that checkable instead of
        // inferred.
        if !gens_final.is_empty() {
            let mut g = gens_final.clone();
            let gbar = median(&mut g);
            let mut sc2 = share_cells.clone();
            let endp = median(&mut sc2) / 100.0;
            let mut ic = intercepts.clone();
            let icbar = if ic.is_empty() { 0.0 } else { median(&mut ic) };
            if endp > 1e-6 && endp < 1.0 - 1e-6 {
                let implied = icbar + med * gbar;
                let actual = (endp / (1.0 - endp)).ln();
                println!(
                    "  cross-check: intercept {icbar:+.3} + slope {med:+.4} x mean generation {gbar:.2}\n  \
                     = {implied:+.3}, against the endpoint share's own log-odds {actual:+.3}."
                );
                // Both arms start equal, so the honest intercept is 0. A large
                // one is the finding, not the arithmetic: it says the log-odds
                // trajectory is curved and a single `s` does not describe it.
                if icbar.abs() > 0.15 {
                    println!(
                        "  WARNING: the fitted intercept is {icbar:+.3}, not ~0. Both arms start equal, so\n  \
                         log-odds at generation 0 must be 0 -- a large intercept means the trajectory is\n  \
                         CURVED (arm B losing fast early, then levelling) and a single slope is a poor\n  \
                         summary of it. Treat `s` as an average over this run's generations, not a rate."
                    );
                }
            }
        }
    } else {
        println!("\nselection coefficient: only {} seeds gave a usable slope.", slopes.len());
        println!("  Needs >=3 samples per run with arm B strictly between 0 and 1: lower `every`, or the");
        println!("  arm is eliminated too fast for a log-odds slope to exist at all (this is `lethal`).");
    }

    if handicap == Handicap::Same && mirrored {
        println!(
            "\n  VACUOUS BY CONSTRUCTION: arm=same with the mirror on is one simulation with the labels\n  \
             swapped, so A == B is an algebraic identity and an exact 50.0% says nothing about this\n  \
             harness. Re-run it as `arm=same mirror=off` -- that is the control that can fail."
        );
    } else if handicap == Handicap::Same {
        println!(
            "\n  arm=same mirror=off is the REAL CONTROL: both arms are one genome, so any departure from\n  \
             50% is position and genotype draw alone -- this harness's own asymmetry, and the size of\n  \
             what the mirror cancels. A large number here does not invalidate the mirrored arms; it is\n  \
             the reason they are mirrored."
        );
    } else {
        println!(
            "\n  A null here is a finding about the WORLD, not about the genome: it says this bed does\n  \
             not discriminate against a plant known to be worse, so no evolution result measured in it\n  \
             can be trusted yet. Run arm=same first to rule out the harness."
        );
    }
}
