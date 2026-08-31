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
use crate::sim::particle::ParticleSystem;
use crate::sim::player;
use crate::sim::world::World;

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
                out.push(PlannedRun { index: out.len(), setting_index: si, setting: *setting, replicate: j, spec });
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

/// One run, with its spec already resolved.
#[derive(Clone, Debug)]
pub struct PlannedRun {
    pub index: usize,
    pub setting_index: usize,
    pub setting: Option<f32>,
    pub replicate: u32,
    pub spec: LabBox,
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

/// Live counts, read by the interface while the batch runs.
#[derive(Clone, Copy, Debug, Default)]
pub struct Progress {
    pub total: usize,
    pub finished: usize,
    pub failed: usize,
    pub held: usize,
    pub elapsed: Duration,
    pub cancelled: bool,
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
        };
        let shared = Arc::new(Shared {
            done: Mutex::new(Vec::new()),
            finished: AtomicUsize::new(0),
            failed: AtomicUsize::new(0),
            held: AtomicUsize::new(0),
            kept_bytes: AtomicU64::new(0),
            cancel: AtomicBool::new(false),
        });
        let sink = Arc::clone(&shared);
        let handle = std::thread::spawn(move || drive(runs, frames, keep_bytes, &sink, &start));
        Self { handle: Some(handle), shared, started: Instant::now(), total, spec }
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
    let mut world = match start {
        Start::Fresh => {
            let mut w = run.spec.build();
            super::earth_toned_nest(&mut w);
            w
        }
        // Cloned per worker rather than per plan — see `start_runs_from`.
        // The palette repaint came with the template.
        Start::Copy(template) => (**template).clone(),
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
    while ran < frames {
        frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        ran += 1;
        // **Inside the loop.** `Stats::observe` gates on `frame >= last +
        // interval` — a `>=`, so it never skips *and never catches up*.
        // Called once per N ticks it would yield one sample spaced N apart
        // rather than N/interval samples, and the strip's x-axis would become
        // the call cadence instead of simulated time, which is the one thing
        // `stats.rs` says it must never be.
        stats.observe(&world);
        if ran.is_multiple_of(CANCEL_CHECK_EVERY) && shared.cancel.load(Ordering::Relaxed) {
            break;
        }
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
        BatchSpec { base: bed(), replicates, sweep, frames: 300, seed0: 1, keep_bytes: u64::MAX }
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
}
