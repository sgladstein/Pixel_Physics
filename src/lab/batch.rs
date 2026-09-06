//! **Run a rack of chambers headless, in the background, and compare them.**
//!
//! The owner's brief, 2026-08-31: *"if we actually want to run evolution
//! experiments, explore the ability to set up a test chamber and then make
//! 10-100 copies of it, let them all run for x amount of time (without
//! visual) then we can go and compare them."*
//!
//! # The seed is the whole thing, and getting it wrong is silent
//!
//! Every random draw in this engine is a pure function of `(world.seed,
//! identity, position-or-frame)` — `rng::stream` is a splitmix64 finaliser
//! over four identity values with no state, and `rng.rs` forbids adding an
//! entropy-seeded constructor. **So a hundred copies of a chamber, copied as
//! they are, are a hundred bit-identical runs**: one sample wearing a hundred
//! labels. That is not a hypothetical, it is `CLAUDE.md`'s *"3 populations
//! wearing 24 logs"*, where a 3.5-hour study produced eight byte-identical
//! logs per species because a seed argument never reached the binary.
//!
//! `LabBox::seed` is the one field that fixes it, and it reaches every plant
//! founding genotype (keyed on the germination *coordinate*) and every
//! creature draw. Vary it and the problem inverts: the spread becomes
//! enormous. Measured in this repo with **no true effect present**, two
//! copies of one genome gave one arm between **40.9% and 80.5%** of a bed
//! across seeds, and twelve identical trees from one genome span **31 to 153
//! cells**. So a comparison read off one run per condition is a lottery
//! ticket.
//!
//! Both of those facts have the same design consequence and it is why
//! [`Sweep`] is optional rather than mandatory:
//!
//! - **Prospecting** — *make copies, see which got interesting, go explore
//!   it.* Here the spread **is** the content. `sweep: None`.
//! - **Experiment** — *does deeper soil win?* Here the spread is noise and
//!   you need replicates per setting, read at an order statistic.
//!
//! One mechanism serves both: prospecting is the one-setting case.
//!
//! # The seed varies with the replicate, never with the setting
//!
//! [`BatchSpec::runs`] is `settings x replicates`, and run `(setting s,
//! replicate j)` takes seed `seed0 + j` — **not** `seed0 + run_index`. So
//! setting A's replicate 3 and setting B's replicate 3 are *the same world*
//! apart from the knob, which is what makes the comparison paired. This is
//! `examples/divergence.rs`'s own rule: *"two separately-built worlds at the
//! same seed... are founded by the same individuals, and the only difference
//! between the runs is the axis."* Seeding by run index instead would
//! confound the knob with the seed and hand back that 40.9-80.5% noise as if
//! it were the effect.
//!
//! # Why this is threads and not a rewrite
//!
//! Both halves already exist in this repo and are copied rather than
//! redesigned:
//!
//! - **`examples/creature_space.rs`** fans N independent worlds over
//!   `std::thread::scope`, chunked by `available_parallelism` so a large
//!   sweep does not spawn hundreds of contending threads. Measured on this
//!   box, 4 independent runs: **29.3 s sequential against 9.0 s on 4
//!   threads, 3.27x**.
//! - **`src/main.rs`'s `Loading`** runs worldgen on a worker while the window
//!   keeps drawing, polled with `is_finished()` and never a blocking join,
//!   and turns a worker panic into a message rather than taking the window
//!   down. `LabBox::build` carries a runtime `assert!`, so a bad spec panics
//!   its thread and `join()` hands back `Err` — that path is handled, not
//!   assumed away.
//!
//! Runs share no state, so the order threads finish in cannot reach a result.
//!
//! # What this deliberately does not do
//!
//! It does not read a clock to decide anything. `frames` is simulated ticks,
//! the same unit the speed dial multiplies, so **a batch result is the same
//! result the player would have got watching it** — exact, not an
//! approximation. `Lab`'s own `advance` has a wall-clock budget so the window
//! keeps answering; a headless run has no window and must not inherit it, or
//! a slow machine would silently produce shorter runs and compare them
//! against long ones.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::sim::explosion::Blasts;
use crate::sim::frame;
use crate::sim::organism;
use crate::sim::particle::ParticleSystem;
use crate::sim::player;
use crate::sim::world::World;

use super::scenario::Scenario;
use super::scene::LabBox;
use super::{params, stats};

/// How often a run looks at the cancel flag, in ticks.
///
/// **Not every tick.** `CLAUDE.md`: *guard hot-path work at the call site* —
/// an atomic load per tick on every worker is a shared cache line bouncing
/// between cores for a question whose answer changes at most once a session.
/// 256 ticks is under a fiftieth of a second of simulated time at the dial's
/// top, so a cancel still feels immediate.
const CANCEL_CHECK_EVERY: u64 = 256;

/// One axis to vary, beside the seed.
///
/// `field` is a `LabBox` field name as [`params::write_bed`] spells it —
/// **the same table the parameters panel writes through**, so a sweep cannot
/// name a knob the panel does not have or write it differently.
#[derive(Clone, Debug)]
pub struct Sweep {
    pub field: String,
    pub values: Vec<f32>,
}

/// What to run.
#[derive(Clone, Debug)]
pub struct BatchSpec {
    /// The chamber every run is a copy of.
    pub base: LabBox,
    /// Seeded replicates **per setting**. `1` with no sweep is a single run.
    pub replicates: u32,
    /// `None` is prospecting: one setting, `replicates` seeds of it.
    pub sweep: Option<Sweep>,
    /// Simulated ticks per run.
    pub frames: u64,
    /// Replicate `j` takes `seed0 + j`. See the module doc.
    pub seed0: u64,
    /// How many bytes of finished worlds to hold before dropping them and
    /// keeping only the record. See [`RunResult::world`].
    pub keep_bytes: u64,
    /// **The scenario `base` was opened from, if one.** Carried here rather
    /// than only on `base` itself, because a scenario is *behaviour*
    /// (settings, placements, a running timeline) and `base` is only ever
    /// read as geometry (`LabBox::build`/`build_counted`) -- `runs()` reads
    /// this to hand every `PlannedRun` its own copy, and `run_one` is what
    /// actually applies it. `BatchSpec` has no `Serialize`/`Deserialize` of
    /// its own to keep in step with `Scenario`'s.
    pub scenario: Option<Scenario>,
}

impl BatchSpec {
    /// Every run this batch will perform, in order, each with its seed and
    /// its setting already applied.
    ///
    /// **The one place run specs are made.** A second copy of this loop is a
    /// second answer to "what seed did run 7 have", and the two would agree
    /// right up until somebody changed one.
    pub fn runs(&self) -> Vec<PlannedRun> {
        let settings: Vec<Option<f32>> = match &self.sweep {
            Some(s) => s.values.iter().map(|v| Some(*v)).collect(),
            None => vec![None],
        };
        let mut out = Vec::with_capacity(settings.len() * self.replicates.max(1) as usize);
        for (si, setting) in settings.iter().enumerate() {
            for j in 0..self.replicates.max(1) {
                let mut spec = self.base.clone();
                // Seed by replicate, never by run index — see the module doc.
                spec.seed = self.seed0.wrapping_add(j as u64);
                if let (Some(sw), Some(v)) = (&self.sweep, setting) {
                    // Refuses rather than silently sweeping a constant.
                    assert!(
                        params::write_bed(&mut spec, &sw.field, *v),
                        "sweep names a bed field that does not exist: {:?}",
                        sw.field
                    );
                }
                out.push(PlannedRun {
                    index: out.len(),
                    frames: None,
                    setting_index: si,
                    setting: *setting,
                    replicate: j,
                    spec,
                    scenario: self.scenario.clone(),
                });
            }
        }
        out
    }

    /// Bytes one finished world of this shape occupies, near enough to
    /// budget with.
    ///
    /// **Chunk-granular, and it counts the pheromone planes**, which are
    /// eagerly allocated by `World::new` and are a third of the bill again —
    /// `Pheromones` holds two planes of a front and a back `Vec<u8>` over the
    /// whole bounds, so that is `4 * w * h` bytes nobody expects.
    pub fn world_bytes(spec: &LabBox) -> u64 {
        let chunk = crate::sim::chunk::CHUNK_SIZE as u64;
        let (w, h) = (spec.width.max(1) as u64, spec.height.max(1) as u64);
        let chunks = w.div_ceil(chunk) * h.div_ceil(chunk);
        let cells = chunks * chunk * chunk * std::mem::size_of::<crate::sim::cell::Cell>() as u64;
        let pheromones = 4 * w * h;
        cells + pheromones
    }
}

/// The trait name the parameters page labels heritable slot `slot` with --
/// reused from `params::TRAIT_ROWS` rather than re-derived, so the S1
/// compartment table and the page it mirrors cannot drift apart into two
/// answers for "what is slot 3 called". `pub` (unlike `TRAIT_ROWS` itself)
/// because a harness outside this crate's own modules -- `examples/
/// labbatch.rs` -- is exactly who needs to label the table it prints.
pub fn trait_name(slot: usize) -> &'static str {
    params::TRAIT_ROWS.iter().find(|(s, ..)| *s == slot).map(|(_, name, _)| *name).unwrap_or("?")
}

/// A copy still in flight: enough to draw a rack row for it.
#[derive(Clone, Copy, Debug)]
pub struct LiveRun {
    pub index: usize,
    pub seed: u64,
    pub setting: Option<f32>,
    pub ticks: u64,
}

/// One run, with its spec already resolved.
#[derive(Clone, Debug)]
pub struct PlannedRun {
    pub index: usize,
    /// Ticks for this run alone, overriding the batch's own count.
    ///
    /// **Extending needs it and a sweep does not.** Resuming a chamber runs
    /// the EXTRA ticks; rebuilding a row whose world was dropped has to run
    /// `frames it already had + extra` to reach the same place, because a
    /// spec plus a seed reproduces a run only from the beginning. One batch
    /// therefore holds runs of different lengths, which `frames` alone
    /// cannot express.
    pub frames: Option<u64>,
    pub setting_index: usize,
    pub setting: Option<f32>,
    pub replicate: u32,
    pub spec: LabBox,
    /// The scenario `spec` was built from, if one -- cloned onto every run
    /// rather than looked up once per batch, because it is small (a bed
    /// plus a handful of placements) beside the `World` each run builds,
    /// and a run travels alone onto its own worker thread.
    pub scenario: Option<Scenario>,
}

/// What a finished run leaves behind.
pub struct RunResult {
    pub index: usize,
    pub setting: Option<f32>,
    pub replicate: u32,
    /// The exact spec, seed included — **this reproduces the run**, which is
    /// what makes dropping the world below affordable rather than lossy.
    pub spec: LabBox,
    /// Taken at the last tick through a fresh `Stats`, never read off the
    /// run's own cache.
    ///
    /// `Stats::observe` gates its census on `frame >= last + interval` and
    /// that interval **doubles permanently** as the history ring decimates,
    /// so the cached census on a long run can be hundreds of frames stale —
    /// and stale by an amount that depends on where the last sample happened
    /// to fall. Two runs compared on that are being compared partly on
    /// sampling phase, which is `CLAUDE.md`'s divide-out-the-oscillator trap
    /// wearing a different hat.
    pub census: stats::Census,
    /// The whole run's series, decimated by `Stats` itself.
    pub history: Vec<stats::Sample>,
    /// Ticks actually run — **separate from the batch's `frames`**, because a
    /// cancelled run must be visibly short rather than quietly compared
    /// against complete ones.
    pub ticks_run: u64,
    /// The finished world, if the batch was still inside `keep_bytes` when
    /// this run landed. `None` means the row is on record and openable by
    /// rebuilding from `spec`, which is exact.
    pub world: Option<World>,
}

/// **S5's reading, one species: its standing at the last sample, and the
/// first sample frame it dropped to zero after having been above it.**
///
/// `None` means the species never went extinct across `history` -- printed
/// as `alive`, never as a frame, so a species that is merely absent from the
/// run's *start* (a colony arriving on a scenario's timeline, say) cannot be
/// misread as one that died before the count ever moved.
#[derive(Clone, Debug, PartialEq)]
pub struct SpeciesRun {
    pub species: String,
    pub last_count: u32,
    pub extinct_at: Option<u64>,
}

/// Every species that ever appears in `history`, sorted by name, each
/// reduced to [`SpeciesRun`]. **Pure over one run's samples** -- no `World`,
/// no `RunResult` -- so this is the whole of what a guard needs to construct
/// by hand.
pub fn species_runs(history: &[stats::Sample]) -> Vec<SpeciesRun> {
    let mut names: Vec<String> = Vec::new();
    for s in history {
        for (name, _) in &s.by_species {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
    }
    names.sort();
    names
        .into_iter()
        .map(|species| {
            let mut was_alive = false;
            let mut extinct_at = None;
            let mut last_count = 0u32;
            for s in history {
                let n = s.by_species.iter().find(|(name, _)| *name == species).map(|(_, n)| *n).unwrap_or(0);
                last_count = n;
                if n > 0 {
                    was_alive = true;
                } else if was_alive && extinct_at.is_none() {
                    // **First**, and never overwritten -- a species that
                    // dies and is later re-founded (a fresh colony on a
                    // repeating timeline event, say) still reads the frame
                    // it *first* went to zero at, which is the question S5
                    // asks.
                    extinct_at = Some(s.frame);
                }
            }
            SpeciesRun { species, last_count, extinct_at }
        })
        .collect()
}

/// **S5's per-setting summary**: one row per species per setting, the
/// min/median/max of its extinction frame across that setting's runs (an
/// `alive` run counts its own `ticks_run`, so a longer horizon does not
/// silently outrank a shorter one that never lost the species either), and
/// how many of the setting's runs it survived to the end of.
#[derive(Clone, Debug, PartialEq)]
pub struct ExtinctionSummary {
    pub setting: Option<f32>,
    pub species: String,
    pub min: u64,
    pub median: u64,
    pub max: u64,
    pub survived: usize,
    pub of: usize,
}

/// Built from `&[RunResult]` and nothing else -- a guard constructs its
/// input by hand, with `world: None` throughout, and never runs a tick.
pub fn extinction_summary(rows: &[RunResult]) -> Vec<ExtinctionSummary> {
    let per_run: Vec<(Option<f32>, u64, Vec<SpeciesRun>)> = rows.iter().map(|r| (r.setting, r.ticks_run, species_runs(&r.history))).collect();
    let mut settings: Vec<Option<f32>> = per_run.iter().map(|(s, ..)| *s).collect();
    settings.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    settings.dedup();

    let mut out = Vec::new();
    for setting in settings {
        let group: Vec<&(Option<f32>, u64, Vec<SpeciesRun>)> = per_run.iter().filter(|(s, ..)| *s == setting).collect();
        let mut species: Vec<String> = Vec::new();
        for (_, _, sr) in &group {
            for s in sr {
                if !species.contains(&s.species) {
                    species.push(s.species.clone());
                }
            }
        }
        species.sort();

        for name in species {
            let mut frames: Vec<u64> = Vec::new();
            let mut survived = 0usize;
            for (_, ticks_run, sr) in &group {
                let alive = match sr.iter().find(|s| s.species == name) {
                    // Present and never extinct, or never present at all --
                    // either way there is nothing to call "extinct", so this
                    // run counts toward survival and reads its own length.
                    Some(s) => s.extinct_at.is_none(),
                    None => true,
                };
                let frame = sr.iter().find(|s| s.species == name).and_then(|s| s.extinct_at).unwrap_or(*ticks_run);
                if alive {
                    survived += 1;
                }
                frames.push(frame);
            }
            frames.sort_unstable();
            let n = frames.len();
            out.push(ExtinctionSummary {
                setting,
                species: name,
                min: frames[0],
                median: frames[n / 2],
                max: frames[n - 1],
                survived,
                of: group.len(),
            });
        }
    }
    out
}

/// **S1's reading, one run, one compartment**: how many animals stood in it
/// (by head cell x) and the mean of every heritable trait slot over them.
/// `None` in `means` where no animal stood there to average.
#[derive(Clone, Debug, PartialEq)]
pub struct CompartmentMeans {
    pub setting: Option<f32>,
    pub replicate: u32,
    pub compartment: usize,
    pub animals: usize,
    pub means: Vec<Option<f32>>,
}

/// Read `world`'s live animals into `spec.compartment_spans()`. **Needs a
/// live `World`** -- a trait is per-organism state, not a sampled count, so
/// unlike [`species_runs`] this cannot be answered from `history` alone.
pub fn compartment_means(world: &World, spec: &LabBox, setting: Option<f32>, replicate: u32) -> Vec<CompartmentMeans> {
    let spans = spec.compartment_spans();
    let mut sums = vec![[0f32; organism::CREATURE_TRAITS]; spans.len()];
    let mut counts = vec![0usize; spans.len()];
    for id in world.live_organism_ids() {
        let Some(state) = world.organism(id) else { continue };
        if world.species.get(state.species).creature.is_none() {
            continue;
        }
        let Some(&(hx, _)) = state.chain.first() else { continue };
        let Some(ci) = spans.iter().position(|&(lo, hi)| hx >= lo && hx <= hi) else { continue };
        counts[ci] += 1;
        for (slot, sum) in sums[ci].iter_mut().enumerate() {
            *sum += state.traits[slot];
        }
    }
    (0..spans.len())
        .map(|ci| CompartmentMeans {
            setting,
            replicate,
            compartment: ci,
            animals: counts[ci],
            means: (0..organism::CREATURE_TRAITS).map(|slot| (counts[ci] > 0).then(|| sums[ci][slot] / counts[ci] as f32)).collect(),
        })
        .collect()
}

/// **S1's per-setting summary**: per compartment, the median across
/// replicates of each trait's mean. A trait with no animal in some
/// replicate's compartment is excluded from that trait's median rather than
/// counted as zero -- an empty compartment is not evidence the trait is low,
/// it is evidence nothing stood there to measure.
///
/// Pure over `&[CompartmentMeans]` -- a guard builds these by hand, no
/// `World` involved at all.
#[derive(Clone, Debug, PartialEq)]
pub struct CompartmentMedians {
    pub setting: Option<f32>,
    pub compartment: usize,
    /// How many of the group's runs actually had an animal in this
    /// compartment to average -- `0/N` on a row says the row is a gap, not
    /// a real reading of "no trait".
    pub runs: usize,
    pub medians: Vec<Option<f32>>,
}

pub fn compartment_medians(rows: &[CompartmentMeans]) -> Vec<CompartmentMedians> {
    let mut settings: Vec<Option<f32>> = rows.iter().map(|r| r.setting).collect();
    settings.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    settings.dedup();

    let mut out = Vec::new();
    for setting in settings {
        let group: Vec<&CompartmentMeans> = rows.iter().filter(|r| r.setting == setting).collect();
        let mut compartments: Vec<usize> = group.iter().map(|r| r.compartment).collect();
        compartments.sort_unstable();
        compartments.dedup();
        for ci in compartments {
            let same: Vec<&&CompartmentMeans> = group.iter().filter(|r| r.compartment == ci).collect();
            let runs = same.iter().filter(|r| r.animals > 0).count();
            let medians: Vec<Option<f32>> = (0..organism::CREATURE_TRAITS)
                .map(|slot| {
                    let mut vals: Vec<f32> = same.iter().filter_map(|r| r.means[slot]).collect();
                    if vals.is_empty() {
                        None
                    } else {
                        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                        Some(vals[vals.len() / 2])
                    }
                })
                .collect();
            out.push(CompartmentMedians { setting, compartment: ci, runs, medians });
        }
    }
    out
}

/// Live counts, read by the interface while the batch runs.
#[derive(Clone, Debug, Default)]
pub struct Progress {
    pub total: usize,
    pub finished: usize,
    pub failed: usize,
    pub held: usize,
    pub elapsed: Duration,
    pub cancelled: bool,
    /// Ticks simulated so far across the whole batch, against what was
    /// planned. The pair is the completion figure; `finished/total` is the
    /// coarser one beside it.
    pub ticks: u64,
    pub ticks_planned: u64,
    /// The copies still running, newest reading of each.
    pub live: Vec<LiveRun>,
}

impl Progress {
    /// Rough time remaining, from the mean run so far. `None` until at least
    /// one run has landed — an estimate from zero samples is a made-up
    /// number, and this one is shown to a person deciding whether to wait.
    pub fn remaining(&self) -> Option<Duration> {
        let done = self.finished + self.failed;
        if done == 0 || done >= self.total {
            return None;
        }
        let per = self.elapsed.as_secs_f64() / done as f64;
        Some(Duration::from_secs_f64(per * (self.total - done) as f64))
    }
}

struct Shared {
    done: Mutex<Vec<RunResult>>,
    /// Ticks completed across every run, finished and in flight.
    ///
    /// **Runs done is not progress.** Fifty copies of 9,000 ticks report
    /// `0/50` for the whole of the first minute while 200,000 ticks have
    /// actually been simulated, so the only honest completion figure is this
    /// one over the planned total.
    ticks: AtomicU64,
    /// What each copy still running has reached, so a rack shows its rows
    /// filling rather than staying empty until they land. Published at the
    /// cancel-check cadence, never per tick.
    live: Mutex<Vec<LiveRun>>,
    finished: AtomicUsize,
    failed: AtomicUsize,
    held: AtomicUsize,
    kept_bytes: AtomicU64,
    cancel: AtomicBool,
}

/// **What every copy starts from.**
///
/// The distinction the owner found the hard way: *"I added some plants and
/// ants to my chamber, hit F4, tried to run copies of the same room, but all
/// of the copies were empty."* Exactly right, and the fault was here — a copy
/// was built from the chamber's **recipe**, and the binary opens on
/// `founders: 0, colonies: 0` because the box starts empty and you stock it.
/// Everything the player plants lives in the *world*, which the recipe has
/// never heard of.
///
/// So a copy now starts from the world itself, and `Fresh` is the exception
/// rather than the rule.
pub enum Start {
    /// **Each run carries on from a world it is handed**, keyed by run index.
    ///
    /// What `EXTEND` runs on: the chamber's own world, moved in rather than
    /// rebuilt, so "another 20,000 ticks" continues the experiment instead of
    /// starting a new one from the same recipe.
    Resume(Mutex<std::collections::HashMap<usize, World>>),
    /// Build the bed from its recipe. Only what the recipe describes is in
    /// it — which is nothing at all on the shipped opening.
    Fresh,
    /// **Copy the box exactly as it stands**, then give the copy its own
    /// seed.
    ///
    /// Cloning is what makes this possible and it was not free: `World` had
    /// no `Clone` at all, and the batch was written around that absence.
    /// Twelve plain-data types needed the derive and nothing needed a hand
    /// written impl.
    ///
    /// **The seed change reaches the future and not the past**, which is
    /// precisely the experiment: every copy holds the same plants, the same
    /// ants, in the same places, with the same genomes — and from the next
    /// tick on, every draw they make differs. Same starting population,
    /// different futures.
    ///
    /// **Boxed**, because a `World` is enormous beside the unit `Fresh` and
    /// every value of this type would otherwise be sized for the larger arm.
    Copy(Box<World>),
}

/// A batch in flight. Held by `Lab`; polled once per displayed frame.
pub struct Batch {
    handle: Option<std::thread::JoinHandle<()>>,
    shared: Arc<Shared>,
    started: Instant,
    total: usize,
    /// Ticks each copy was asked for, so `Progress` can state the planned
    /// total rather than the caller having to remember it.
    frames_each: u64,
    pub spec: BatchSpec,
}

impl Batch {
    /// Start `spec` on background threads and return immediately.
    ///
    /// Nothing about this blocks: the caller is a frame loop.
    pub fn start(spec: BatchSpec) -> Self {
        let runs = spec.runs();
        let mut batch = Self::start_runs(runs, spec.frames, spec.keep_bytes);
        // `start_runs` reconstructs a spec from its runs, which cannot know
        // about a sweep; the real one is the caller's.
        batch.spec = spec;
        batch
    }

    /// Start an already-planned list of runs.
    ///
    /// The seam [`Batch::start`] goes through, exposed because a harness
    /// wants to construct its own arms — notably *the same run N times*,
    /// which no `BatchSpec` can express and which is the control that says
    /// whether any of this is reproducible.
    pub fn start_runs(runs: Vec<PlannedRun>, frames: u64, keep_bytes: u64) -> Self {
        Self::start_runs_from(runs, frames, keep_bytes, Start::Fresh)
    }

    /// As [`Batch::start_runs`], with what every copy starts from.
    ///
    /// The template is cloned **once** into the batch and then cloned again
    /// per run on the worker that needs it, rather than N times up front: at
    /// 2.5 MB a world, a fifty-copy batch would otherwise hold 125 MB before
    /// a single tick had run.
    pub fn start_runs_from(runs: Vec<PlannedRun>, frames: u64, keep_bytes: u64, start: Start) -> Self {
        let total = runs.len();
        let spec = BatchSpec {
            base: runs.first().map(|r| r.spec.clone()).unwrap_or_default(),
            replicates: total as u32,
            sweep: None,
            frames,
            seed0: runs.first().map(|r| r.spec.seed).unwrap_or(0),
            keep_bytes,
            scenario: runs.first().and_then(|r| r.scenario.clone()),
        };
        let shared = Arc::new(Shared {
            done: Mutex::new(Vec::new()),
            finished: AtomicUsize::new(0),
            failed: AtomicUsize::new(0),
            held: AtomicUsize::new(0),
            kept_bytes: AtomicU64::new(0),
            cancel: AtomicBool::new(false),
            ticks: AtomicU64::new(0),
            live: Mutex::new(Vec::new()),
        });
        let sink = Arc::clone(&shared);
        let handle = std::thread::spawn(move || drive(runs, frames, keep_bytes, &sink, &start));
        Self { handle: Some(handle), shared, started: Instant::now(), total, frames_each: frames, spec }
    }

    /// Take every run that has landed since the last call.
    ///
    /// Drains rather than accumulates, so the caller owns the worlds and this
    /// does not hold a rack of them alive behind a mutex.
    pub fn drain(&self) -> Vec<RunResult> {
        let mut done = self.shared.done.lock().expect("batch mutex poisoned only by a panicking worker");
        std::mem::take(&mut *done)
    }

    pub fn progress(&self) -> Progress {
        Progress {
            total: self.total,
            finished: self.shared.finished.load(Ordering::Relaxed),
            failed: self.shared.failed.load(Ordering::Relaxed),
            held: self.shared.held.load(Ordering::Relaxed),
            elapsed: self.started.elapsed(),
            cancelled: self.shared.cancel.load(Ordering::Relaxed),
            ticks: self.shared.ticks.load(Ordering::Relaxed),
            ticks_planned: self.total as u64 * self.frames_each,
            live: self.shared.live.lock().map(|l| l.clone()).unwrap_or_default(),
        }
    }

    /// Ask every run to stop at its next check. Runs already finished keep
    /// their results; a run in flight lands short, with its real `ticks_run`.
    pub fn cancel(&self) {
        self.shared.cancel.store(true, Ordering::Relaxed);
    }

    /// Whether the worker has stopped. **`is_finished`, never a blocking
    /// join** — `main.rs`'s `poll_loading` rule: a join in the frame loop
    /// freezes the window for the rest of the batch, which is the whole thing
    /// this mechanism exists to avoid.
    pub fn is_finished(&self) -> bool {
        self.handle.as_ref().is_none_or(|h| h.is_finished())
    }

    /// Reap the worker once it has stopped. Returns whether it panicked.
    ///
    /// A bool rather than a `Result`: there is exactly one failure and it
    /// carries nothing — the owner thread's own panic payload is not
    /// something a caller can act on, and the per-run failures it was
    /// responsible for are already counted in [`Progress::failed`].
    pub fn join(&mut self) -> bool {
        match self.handle.take() {
            Some(h) => h.join().is_ok(),
            None => true,
        }
    }
}

/// The owner thread: chunked `thread::scope` over the planned runs.
///
/// Chunked rather than one thread per run so that a hundred-chamber batch
/// does not spawn a hundred contending threads — `creature_space`'s rule,
/// inherited with its reasoning.
fn drive(runs: Vec<PlannedRun>, frames: u64, keep_bytes: u64, shared: &Arc<Shared>, start: &Start) {
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    for chunk in runs.chunks(threads) {
        if shared.cancel.load(Ordering::Relaxed) {
            break;
        }
        let landed: Vec<Option<RunResult>> = std::thread::scope(|scope| {
            let handles: Vec<_> = chunk.iter().map(|run| scope.spawn(|| run_one(run, frames, shared, start))).collect();
            // `.ok()` — a panicking worker is a bad spec, not a reason to
            // take the game down. `LabBox::build` asserts its floor sits
            // below the bed, and `main.rs` handles a panicking generator the
            // same way: count it, say so, carry on.
            handles.into_iter().map(|h| h.join().ok()).collect()
        });
        let mut keep: Vec<RunResult> = Vec::new();
        for landed in landed {
            match landed {
                Some(mut r) => {
                    // The budget is applied here, on the owner thread, so it
                    // is a single running total rather than N racing ones.
                    let bytes = BatchSpec::world_bytes(&r.spec);
                    let held = shared.kept_bytes.load(Ordering::Relaxed);
                    if r.world.is_some() && held + bytes <= keep_bytes {
                        shared.kept_bytes.store(held + bytes, Ordering::Relaxed);
                        shared.held.fetch_add(1, Ordering::Relaxed);
                    } else {
                        r.world = None;
                    }
                    shared.finished.fetch_add(1, Ordering::Relaxed);
                    keep.push(r);
                }
                None => {
                    shared.failed.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        shared.done.lock().expect("batch mutex poisoned only by a panicking worker").extend(keep);
    }
}

/// One chamber, built and run headless.
fn run_one(run: &PlannedRun, frames: u64, shared: &Arc<Shared>, start: &Start) -> RunResult {
    let frames = run.frames.unwrap_or(frames);
    let mut world = match start {
        Start::Fresh => {
            let mut w = run.spec.build();
            // **The run's own (swept) spec, not the batch's `base`.** A
            // sweep over `compartments` -- say -- has already written the
            // swept value into `run.spec` by the time `runs()` handed this
            // plan out, so applying the scenario against a fresh clone of
            // *that* spec is what keeps a scenario's placements landing on
            // the bed this particular run actually has, not the template
            // every other setting also started from.
            if let Some(scenario) = &run.scenario {
                let mut spec = run.spec.clone();
                scenario.apply(&mut w, &mut spec);
            }
            super::earth_toned_nest(&mut w);
            w
        }
        // Cloned per worker rather than per plan — see `start_runs_from`.
        // The palette repaint came with the template.
        Start::Copy(template) => {
            let mut w = (**template).clone();
            // **The copy's narrative starts when the copy does.** `World` is
            // `Clone`, so a copy arrives holding everything its parent
            // logged, and fifty of them would be fifty transcripts of one
            // shared past with fifty divergent futures appended. The
            // *counters* are deliberately kept -- they describe the shared
            // starting population, which is the experiment -- but the log is
            // a story about a particular run, and this is a different one.
            // Not done for `Resume`, which is EXTEND: that is the same run
            // carrying on, and its history is its own.
            w.run_log.clear();
            w
        }
        // **Taken, not cloned.** The world came out of the rack and is going
        // back into it; cloning here would double a fifty-chamber extension's
        // peak memory for nothing. A row with no entry -- an on-record row,
        // whose world was dropped for the budget -- falls back to building
        // from its spec, which reproduces it exactly, and its `frames` is the
        // full total rather than the extra.
        Start::Resume(table) => table
            .lock()
            .ok()
            .and_then(|mut t| t.remove(&run.index))
            .unwrap_or_else(|| {
                let mut w = run.spec.build();
                // Same reasoning as `Start::Fresh` just above: this is the
                // rebuild-from-recipe fallback for a row whose world was not
                // in the table (an on-record row extended past its budget),
                // and it needs the scenario applied for the identical
                // reason a fresh run does.
                if let Some(scenario) = &run.scenario {
                    let mut spec = run.spec.clone();
                    scenario.apply(&mut w, &mut spec);
                }
                super::earth_toned_nest(&mut w);
                w
            }),
    };
    // **After the clone, so it reaches the copy's future and not its past.**
    // Everything already standing in the box is identical across copies; from
    // here on every draw they make differs.
    world.seed = run.spec.seed;
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    let mut stats = stats::Stats::new();
    let mut ran = 0u64;
    let mut published = 0u64;
    if let Ok(mut live) = shared.live.lock() {
        live.push(LiveRun { index: run.index, seed: run.spec.seed, setting: run.setting, ticks: 0 });
    }
    while ran < frames {
        frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        // Same call, same timing contract, as `Lab::tick`'s own -- see
        // `scenario::tick_timeline`'s doc. Cheap when the timeline is empty
        // or there is no scenario at all, which is every setting sweep run
        // that has ever used this function until now.
        if let Some(scenario) = &run.scenario {
            super::scenario::tick_timeline(scenario, &mut world, &run.spec);
        }
        ran += 1;
        // **Inside the loop.** `Stats::observe` gates on `frame >= last +
        // interval` — a `>=`, so it never skips *and never catches up*.
        // Called once per N ticks it would yield one sample spaced N apart
        // rather than N/interval samples, and the strip's x-axis would become
        // the call cadence instead of simulated time, which is the one thing
        // `stats.rs` says it must never be.
        stats.observe(&world);
        // **Published on the cancel check's own cadence, never per tick.**
        // `CLAUDE.md`'s *guard hot-path work at the call site*: this rides a
        // modulo the loop already computes, so the aggregate counter and the
        // live row cost one relaxed add and one short lock every 256 ticks
        // rather than anything per frame.
        if ran.is_multiple_of(CANCEL_CHECK_EVERY) {
            shared.ticks.fetch_add(CANCEL_CHECK_EVERY, Ordering::Relaxed);
            published += CANCEL_CHECK_EVERY;
            if let Ok(mut live) = shared.live.lock() {
                if let Some(row) = live.iter_mut().find(|r| r.index == run.index) {
                    row.ticks = ran;
                }
            }
            if shared.cancel.load(Ordering::Relaxed) {
                break;
            }
        }
    }
    // The tail the cadence did not cover, so the aggregate lands on the true
    // total rather than a multiple of the check interval.
    shared.ticks.fetch_add(ran.saturating_sub(published), Ordering::Relaxed);
    if let Ok(mut live) = shared.live.lock() {
        live.retain(|r| r.index != run.index);
    }
    // A **fresh** `Stats` for the final census, never `stats.census()` — see
    // `RunResult::census` for the staleness this avoids. Its history is
    // empty, so its first `observe` censuses unconditionally.
    let mut final_stats = stats::Stats::new();
    final_stats.observe(&world);
    let census = final_stats.census().cloned().expect("a fresh Stats censuses on its first observe");
    RunResult {
        index: run.index,
        setting: run.setting,
        replicate: run.replicate,
        spec: run.spec.clone(),
        census,
        history: stats.history().to_vec(),
        ticks_run: ran,
        world: Some(world),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bed small enough to run several of in a test, and still alive.
    /// `ground_y` and `soil_depth` scale with `height` — `lab_resolution`
    /// records what happens when they do not: the soil sits in the top
    /// quarter and the rest is void, a scene error wearing a result.
    fn bed() -> LabBox {
        LabBox { width: 256, height: 192, ground_y: 96, soil_depth: 48, founders: 4, colonies: 0, ..LabBox::default() }
    }

    fn spec(replicates: u32, sweep: Option<Sweep>) -> BatchSpec {
        BatchSpec { base: bed(), replicates, sweep, frames: 300, seed0: 1, keep_bytes: u64::MAX, scenario: None }
    }

    /// **The seed varies with the replicate and not with the setting.**
    ///
    /// The pairing the whole sweep rests on: setting A's replicate 3 and
    /// setting B's replicate 3 must be the same world apart from the knob, or
    /// the knob is confounded with a seed difference whose own spread —
    /// measured elsewhere in this repo at 40.9% to 80.5% with no true effect
    /// — swamps anything a sweep could find.
    #[test]
    fn a_sweep_pairs_its_settings_on_the_seed() {
        let sw = Sweep { field: "soil_depth".into(), values: vec![32.0, 64.0, 96.0] };
        let runs = spec(4, Some(sw)).runs();
        assert_eq!(runs.len(), 12, "three settings by four replicates");

        for j in 0..4u32 {
            let same: Vec<&PlannedRun> = runs.iter().filter(|r| r.replicate == j).collect();
            assert_eq!(same.len(), 3, "every replicate appears once per setting");
            let seed = same[0].spec.seed;
            assert!(same.iter().all(|r| r.spec.seed == seed), "replicate {j} is not one seed across settings");
            // ...and the knob is the only other thing that moved.
            let depths: Vec<i32> = same.iter().map(|r| r.spec.soil_depth).collect();
            assert_eq!(depths, vec![32, 64, 96], "the swept field did not take its values");
        }
        // Distinct seeds across replicates: without this the pairing above is
        // satisfied by every run sharing one seed, which is the failure the
        // whole module doc is about.
        let seeds: std::collections::HashSet<u64> = runs.iter().map(|r| r.spec.seed).collect();
        assert_eq!(seeds.len(), 4, "four replicates must be four different worlds, not one four times");
    }

    /// Prospecting is the one-setting case, not a second mechanism.
    #[test]
    fn no_sweep_is_one_setting_of_seeded_replicates() {
        let runs = spec(5, None).runs();
        assert_eq!(runs.len(), 5);
        assert!(runs.iter().all(|r| r.setting.is_none()));
        let seeds: std::collections::HashSet<u64> = runs.iter().map(|r| r.spec.seed).collect();
        assert_eq!(seeds.len(), 5, "five copies must be five worlds");
    }

    /// A sweep naming a field that does not exist must refuse rather than
    /// sweep a constant — the `include_str!` failure, where three "runs" came
    /// back bit-identical because the knob was never connected.
    #[test]
    #[should_panic(expected = "sweep names a bed field that does not exist")]
    fn a_sweep_over_a_field_that_does_not_exist_refuses() {
        let sw = Sweep { field: "not_a_field".into(), values: vec![1.0, 2.0] };
        let _ = spec(1, Some(sw)).runs();
    }

    /// **Both halves of the runner's own control, and the answer to the
    /// question the feature was asked to settle.**
    ///
    /// `CLAUDE.md`'s most-recurring failure is a number that is
    /// arithmetically right and about the wrong thing, and this feature has
    /// an obvious one waiting: **"50 runs completed" is true of 50 runs of
    /// the same world.** So both arms run here, and either alone would be
    /// green on a broken build — `same` passes for a runner that ignores the
    /// seed entirely, `seeded` for one whose runs share nothing with their
    /// base.
    ///
    /// **900 frames rather than 300, and the reason is the trap this test
    /// exists to avoid.** Founders are placed as seeds and have not sprouted
    /// by frame 300, so every census would read the same *whatever* the seed
    /// was, and the `seeded` arm would fail for a reason that has nothing to
    /// do with the seed. Measured with `labstats`: 8 sprouted and 208 plant
    /// cells by frame 900, against 8 cells at frame 0.
    #[test]
    fn identical_seeds_give_identical_runs_and_different_seeds_do_not() {
        const FRAMES: u64 = 900;
        let census_of = |runs: Vec<PlannedRun>| -> Vec<stats::Census> {
            let shared = Arc::new(Shared {
                done: Mutex::new(Vec::new()),
                finished: AtomicUsize::new(0),
                failed: AtomicUsize::new(0),
                held: AtomicUsize::new(0),
                kept_bytes: AtomicU64::new(0),
                cancel: AtomicBool::new(false),
                ticks: AtomicU64::new(0),
                live: Mutex::new(Vec::new()),
            });
            runs.iter().map(|r| run_one(r, FRAMES, &shared, &Start::Fresh).census).collect()
        };

        // Specificity: one seed, three copies. Anything but "identical" here
        // means the engine is not reproducible and no rack comparison means
        // anything at all.
        let one = spec(1, None).runs().remove(0);
        let same = census_of((0..3).map(|i| PlannedRun { index: i, ..one.clone() }).collect());
        let first = &same[0];
        for (i, c) in same.iter().enumerate() {
            assert_eq!(
                (c.plants, c.plant_cells, c.animals, c.seeds_borne, c.germinations),
                (first.plants, first.plant_cells, first.animals, first.seeds_borne, first.germinations),
                "copy {i} of one seed came out different -- the engine is not reproducible, \
                 so every chamber-vs-chamber comparison would be reporting its own noise"
            );
        }

        // Sensitivity: three seeds. Identical here means the seed never
        // reached the copy, and a rack of replicates is one world wearing
        // many labels -- `CLAUDE.md`'s "3 populations wearing 24 logs".
        let seeded = census_of(spec(3, None).runs());
        let distinct: std::collections::HashSet<(usize, usize, u64)> =
            seeded.iter().map(|c| (c.plants, c.plant_cells, c.germinations)).collect();
        assert!(
            distinct.len() > 1,
            "three different seeds produced identical censuses -- the seed is not reaching \
             the copies, so a batch would be one run wearing N labels. Got: {:?}",
            seeded.iter().map(|c| (c.plants, c.plant_cells, c.germinations)).collect::<Vec<_>>()
        );
    }

    /// The memory estimate must count the pheromone planes, which
    /// `World::new` allocates eagerly and which are a third of the bill.
    #[test]
    fn the_world_estimate_counts_more_than_the_cells() {
        let b = LabBox { width: 512, height: 320, ..LabBox::default() };
        let bytes = BatchSpec::world_bytes(&b);
        let cells = 40 * 64 * 64 * 12; // 8x5 chunks of 12-byte cells
        assert!(bytes > cells, "the estimate is cells only: {bytes} vs {cells}");
        assert!(bytes < cells * 2, "the estimate has run away: {bytes} vs {cells}");
    }

    /// **A batch copy starts its own run log.**
    ///
    /// `World` is `Clone` and the run log is part of it, so without the clear
    /// in `run_one` every copy opens holding its parent's whole narrative and
    /// then appends a *different* future to it. Fifty chambers would each
    /// read as one shared past with fifty contradictory continuations, and
    /// the page has no way to tell which lines were inherited.
    ///
    /// Provable red by deleting `w.run_log.clear()` from the `Copy` arm: the
    /// planted line comes back in every copy's log.
    ///
    /// The `Resume` half is the other side of the same rule and is asserted
    /// here too, because it is the case where clearing would be the bug --
    /// EXTEND is the same run carrying on, so its history must survive.
    #[test]
    fn a_batch_copy_starts_its_own_run_log_and_an_extension_keeps_its_own() {
        let shared = Arc::new(Shared {
            done: Mutex::new(Vec::new()),
            finished: AtomicUsize::new(0),
            failed: AtomicUsize::new(0),
            held: AtomicUsize::new(0),
            kept_bytes: AtomicU64::new(0),
            cancel: AtomicBool::new(false),
            ticks: AtomicU64::new(0),
            live: Mutex::new(Vec::new()),
        });
        // A world with a line of history nothing in the run could produce:
        // frame 999_999 is past anything a 60-frame run reaches, so finding
        // it in a copy's log can only mean it was inherited.
        let planted = crate::sim::world::LogEvent {
            frame: 999_999,
            id: 7,
            born_frame: 0,
            species: crate::sim::organism::SpeciesId(0),
            kind: crate::sim::world::LogKind::Born,
            other: 0,
        };
        let mut parent = bed().build();
        crate::lab::earth_toned_nest(&mut parent);
        parent.run_log.push(planted);
        assert_eq!(parent.run_log.len(), 1, "the plant did not take -- the rest of this proves nothing");

        let run = spec(1, None).runs().remove(0);
        let copied = run_one(&run, 60, &shared, &Start::Copy(Box::new(parent.clone())));
        let log = &copied.world.as_ref().expect("the budget is u64::MAX, so the world is kept").run_log;
        assert!(
            !log.recent().any(|e| e.frame == 999_999),
            "a batch copy opened holding its parent's history: {} line(s) inherited",
            log.recent().filter(|e| e.frame == 999_999).count()
        );

        // ...and the extension keeps it. Same planted world, handed in
        // through the table EXTEND uses.
        let table = Mutex::new(std::collections::HashMap::from([(run.index, parent)]));
        let extended = run_one(&run, 60, &shared, &Start::Resume(table));
        let log = &extended.world.as_ref().expect("the budget is u64::MAX, so the world is kept").run_log;
        assert!(
            log.recent().any(|e| e.frame == 999_999),
            "EXTEND threw away the chamber's own history -- that is the same run carrying on, not a copy"
        );
    }

    /// A hand-made [`Sample`](stats::Sample) at `frame`, carrying only
    /// `by_species` -- every other field is zero, which nothing under test
    /// here reads.
    fn sample(frame: u64, by_species: &[(&str, u32)]) -> stats::Sample {
        stats::Sample { frame, by_species: by_species.iter().map(|(n, c)| (n.to_string(), *c)).collect(), ..Default::default() }
    }

    /// A hand-made [`RunResult`], `world: None` throughout -- these guards
    /// never build a world or run a tick.
    fn run_result(index: usize, setting: Option<f32>, replicate: u32, ticks_run: u64, history: Vec<stats::Sample>) -> RunResult {
        RunResult { index, setting, replicate, spec: bed(), census: stats::Census::default(), history, ticks_run, world: None }
    }

    /// **The guard for item 2b/S5, and its own positive control.** Three
    /// hand-made runs, one setting, `world: None` throughout -- this proves
    /// the reading without a tick ever running, per `CLAUDE.md`'s "does the
    /// planned step actually demonstrate itself" and the coordinator's own
    /// ask that this not run the whole scenario.
    ///
    /// ANT goes extinct at 3,000 in run 0, at 2,000 in run 1, and never in
    /// run 2 (5 standing at the last sample) -- min 2,000, and at n=3 this
    /// median convention (`frames[len/2]`, the same `Spread::of` uses)
    /// lands on 3,000, survived 1 of 3. **BEETLE is the positive control**:
    /// never zero in any run, so every run reads `alive` and contributes its
    /// own `ticks_run` (3,000 in all three) rather than a manufactured
    /// frame -- min = median = max = 3,000, survived 3 of 3.
    #[test]
    fn the_extinction_summary_reads_the_frame_each_species_first_hit_zero() {
        let run0 = run_result(
            0,
            None,
            0,
            3000,
            vec![
                sample(0, &[]),
                sample(1000, &[("ANT", 26)]),
                sample(2000, &[("ANT", 10), ("BEETLE", 4)]),
                sample(3000, &[("ANT", 0), ("BEETLE", 4)]),
            ],
        );
        let run1 = run_result(
            1,
            None,
            1,
            3000,
            vec![
                sample(0, &[]),
                sample(1000, &[("ANT", 26)]),
                sample(2000, &[("ANT", 0), ("BEETLE", 4)]),
                sample(3000, &[("ANT", 0), ("BEETLE", 4)]),
            ],
        );
        let run2 = run_result(
            2,
            None,
            2,
            3000,
            vec![
                sample(0, &[]),
                sample(1000, &[("ANT", 26)]),
                sample(2000, &[("ANT", 15), ("BEETLE", 4)]),
                sample(3000, &[("ANT", 5), ("BEETLE", 4)]),
            ],
        );

        // Per-run reading first -- the layer under the summary, checked on
        // its own so a wrong summary cannot hide a right per-run answer or
        // vice versa.
        let sr0 = species_runs(&run0.history);
        let ant0 = sr0.iter().find(|s| s.species == "ANT").expect("ANT appears in run 0");
        assert_eq!(ant0.extinct_at, Some(3000), "run 0's ANT hit zero at frame 3000");
        assert_eq!(ant0.last_count, 0);
        let beetle2 = species_runs(&run2.history).into_iter().find(|s| s.species == "BEETLE").expect("BEETLE appears in run 2");
        assert_eq!(beetle2.extinct_at, None, "BEETLE never hit zero in run 2 -- it must read alive, not a frame");
        assert_eq!(beetle2.last_count, 4);

        let summary = extinction_summary(&[run0, run1, run2]);
        let ant = summary.iter().find(|s| s.species == "ANT").expect("ANT has a summary row");
        assert_eq!((ant.min, ant.median, ant.max), (2000, 3000, 3000), "ANT's extinction spread: {ant:?}");
        assert_eq!(ant.survived, 1, "only run 2's ant colony was still standing: {ant:?}");
        assert_eq!(ant.of, 3);

        let beetle = summary.iter().find(|s| s.species == "BEETLE").expect("BEETLE has a summary row");
        assert_eq!((beetle.min, beetle.median, beetle.max), (3000, 3000, 3000), "the positive control: an always-alive species reads its own ticks_run, not a manufactured extinction: {beetle:?}");
        assert_eq!(beetle.survived, 3, "BEETLE survived every run: {beetle:?}");
    }

    /// **The guard for item 2c/S1's aggregation half** -- pure over
    /// hand-made [`CompartmentMeans`], no `World` and no tick. The
    /// `World`-reading half ([`compartment_means`] itself) is exercised live
    /// by the `two_larders` smoke run, which is the one thing here that
    /// cannot be faked: a trait mean is a property of live organism state,
    /// not of a sampled count.
    #[test]
    fn the_compartment_medians_take_the_median_across_replicates_and_skip_empty_compartments() {
        let mut means_a = vec![None; organism::CREATURE_TRAITS];
        means_a[0] = Some(1.0);
        let mut means_b = vec![None; organism::CREATURE_TRAITS];
        means_b[0] = Some(3.0);
        // Compartment 1's replicate 0 has no animal in it at all -- its
        // slot-0 mean is `None`, and the median must skip it rather than
        // read it as zero.
        let means_c = vec![None; organism::CREATURE_TRAITS];
        let mut means_d = vec![None; organism::CREATURE_TRAITS];
        means_d[0] = Some(5.0);

        let rows = vec![
            CompartmentMeans { setting: None, replicate: 0, compartment: 0, animals: 2, means: means_a },
            CompartmentMeans { setting: None, replicate: 1, compartment: 0, animals: 2, means: means_b },
            CompartmentMeans { setting: None, replicate: 0, compartment: 1, animals: 0, means: means_c },
            CompartmentMeans { setting: None, replicate: 1, compartment: 1, animals: 1, means: means_d },
        ];
        let medians = compartment_medians(&rows);

        let c0 = medians.iter().find(|m| m.compartment == 0).expect("compartment 0 has a row");
        assert_eq!(c0.medians[0], Some(3.0), "median of [1.0, 3.0] at n=2 (frames[len/2] convention) is 3.0: {c0:?}");
        assert_eq!(c0.runs, 2);

        let c1 = medians.iter().find(|m| m.compartment == 1).expect("compartment 1 has a row");
        assert_eq!(c1.medians[0], Some(5.0), "compartment 1's empty replicate must be skipped, not read as 0: {c1:?}");
        assert_eq!(c1.runs, 1, "only one of compartment 1's two replicates actually had an animal in it");
    }
}
