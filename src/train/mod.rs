//! **The training rig: behaviour found by challenge and selection rather
//! than written by hand.** Built for the brain-training thread (2026-10-17),
//! on the owner's ruling that this line is a new approach and is not bound by
//! the hand-designed ant's rules.
//!
//! A [`Challenge`] is a small arena with a score. The rig runs a population of
//! brain genomes through it generation after generation: every genome is
//! scored on fresh layouts, the best quarter are kept, and the rest of the
//! population is refilled with mutated (optionally crossed) copies of them.
//! Two selection units, because some behaviours belong to an animal and some
//! only exist in a colony:
//!
//! * [`Unit::Colony`] -- every ant in the arena carries the same genome and the
//!   arena's score is the genome's. Foraging and digging are colony work.
//! * [`Unit::Ant`] -- each ant in a shared arena carries its own genome and is
//!   scored alone. Following a trail is one animal's skill.
//!
//! **What keeps a score honest is the part this module is for.** The probe
//! before it (`examples/brainfit.rs`) found a way to cheat every score it was
//! given inside three generations: "food gone from the heap" was won by
//! scattering the heap, "food at home" by hopping through the air, and a
//! walking-only colony that was not paid for staying alive halved itself. So
//! every run carries, beside the score being selected on:
//!
//! * **the start genome as a control**, scored on the same layouts every
//!   generation, so a rising best can be told from easier layouts;
//! * **a hold-out** on layouts training never sees, every few generations:
//!   the champion, the start genome, and **the champion with its senses cut**
//!   (every weight from an input but `Bias` zeroed). If the blind champion
//!   scores as well as the sighted one, what was learned is a motor habit,
//!   not a response to the world;
//! * **a list of moves the challenge forbids** ([`Challenge::banned`]): their
//!   brain rows are zeroed and never mutated, so a verb meant for another
//!   species cannot become the answer;
//! * **a picture** of the champion on the first hold-out layout.
//!
//! Mutation inside the world stays off (`creature::mutation_off_for_measuring`):
//! the only variation is the one this loop makes between runs, which is the
//! owner's ruling (2026-10-17) on how the rig and the frozen-mutation rule fit.

pub mod challenges;

use crate::sim::brain::{self, BrainInput, BrainOutput};
use crate::sim::World;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Who a score belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    /// Every ant in the arena is a clone; one score per arena.
    Colony,
    /// Every ant has its own genome; one score per ant.
    Ant,
}

/// One arena. `k` is the layout's index within its batch (counted across
/// generations in training, so an odd `layouts` still alternates), so a
/// challenge can alternate a side or a mirror with it and "always walk east"
/// scores half.
///
/// `blind` asks the arena to cut the **body's** senses as well as the
/// brain's, for the senses-cut control: the homeward re-roll
/// (`CreatureDef::home_bias`) reads the home direction outside the brain, and
/// the shipped walk (`walk=game`) reads trails and home directly. A blinded
/// genome alone left both on, so a "blind" forage colony could still find
/// home (coordinator review of PR 707).
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub seed: u64,
    pub k: u64,
    pub blind: bool,
}

/// The same layouts with the body's senses cut too; see [`Layout::blind`].
pub fn blind(layouts: &[Layout]) -> Vec<Layout> {
    layouts.iter().map(|&l| Layout { blind: true, ..l }).collect()
}

/// A score and the numbers beside it. `parts` lines up with
/// [`Challenge::parts`] and is written to the progress file, so the number
/// the score could be faked with is always printed next to it.
#[derive(Clone, Debug, Default)]
pub struct Score {
    pub fit: f64,
    pub parts: Vec<f64>,
}

pub trait Challenge: Sync {
    fn name(&self) -> &'static str;
    fn unit(&self) -> Unit;
    /// Ants in one arena.
    fn ants(&self) -> usize;
    /// Names of [`Score::parts`], in order.
    fn parts(&self) -> &'static [&'static str];
    /// Outputs this challenge forbids. Their rows are zeroed in every genome
    /// and never mutated.
    fn banned(&self) -> &'static [BrainOutput];
    /// Run one arena. `genomes` has one entry for [`Unit::Colony`] and
    /// [`Challenge::ants`] entries for [`Unit::Ant`]; the result has as many
    /// scores as `genomes`. `look` sees the world after every frame, for
    /// pictures.
    fn run(&self, layout: Layout, genomes: &[&[f32]], look: &mut dyn FnMut(u64, &World)) -> Vec<Score>;
    /// Frames one arena runs, for the picture's snapshot times.
    fn frames(&self) -> u64;
    /// Set any reference scores from the start genome before training.
    /// Only a suite has any.
    fn calibrate(&mut self, _start: &[f32], _layouts: &[Layout], _threads: usize) {}
    /// Whether odd layouts mirror even ones (the food or the trail on the
    /// other side). A mirrored challenge is scored by the worse side of each
    /// pair ([`fold`]), not the mean.
    fn mirrored(&self) -> bool {
        false
    }
}

/// SplitMix64. The rig's only randomness, so a run is a function of its seed.
#[derive(Clone)]
pub struct Rng(pub u64);
impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    /// A step with mean 0 and standard deviation 1, uniform on
    /// `[-sqrt 3, sqrt 3]`. **Not a Gaussian, on purpose**: Box-Muller needs
    /// `ln` and `cos`, whose last bit is the platform libm's, so a run in the
    /// cloud could pick different children from a local one with the same
    /// seed. `brain::mutate` steps uniformly for the same reason. `sqrt` is
    /// correctly rounded everywhere, and this one is a constant anyway.
    pub fn step(&mut self) -> f32 {
        (self.unit() * 2.0 - 1.0) * 3f32.sqrt()
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// End of the brain's weights; the developmental trait slots after it are
/// not the rig's to move.
pub const BRAIN_END: usize = brain::GENOME_LEN - brain::TRAIT_SLOTS;

/// The output a slot feeds directly, if it is an input->output or
/// hidden->output weight.
pub fn output_of(slot: usize) -> Option<usize> {
    // Both blocks are output-major with 64-wide rows (`brain::io_slot`,
    // `brain::ho_slot`); the slot helpers give the block edges by name.
    let io_end = brain::ih_slot(BrainInput::Bias, 0);
    let ho_start = brain::ho_slot(0, BrainOutput::Turn);
    if slot < io_end {
        Some(slot / 64)
    } else if (ho_start..BRAIN_END).contains(&slot) {
        Some((slot - ho_start) / 64)
    } else {
        None
    }
}

/// The hidden unit a slot feeds, if it is an input->hidden or the
/// hidden->hidden (self-recurrence) weight.
pub fn hidden_of(slot: usize) -> Option<usize> {
    let ih_start = brain::ih_slot(BrainInput::Bias, 0);
    let hh_start = brain::hh_slot(0);
    if (ih_start..hh_start).contains(&slot) {
        Some((slot - ih_start) / 64)
    } else if (hh_start..brain::ho_slot(0, BrainOutput::Turn)).contains(&slot) {
        Some(slot - hh_start)
    } else {
        None
    }
}

/// The input a slot reads directly, if it is an input->output or
/// input->hidden weight.
pub fn input_of(slot: usize) -> Option<usize> {
    if slot < brain::hh_slot(0) {
        Some(slot % 64)
    } else {
        None
    }
}

/// The slots the rig may move for a challenge: live brain weights, minus the
/// rows of every banned output.
pub fn trainable(banned: &[BrainOutput]) -> Vec<usize> {
    let banned: Vec<usize> = banned.iter().map(|&o| o as usize).collect();
    brain::live_slots()
        .filter(|&i| i < BRAIN_END && !output_of(i).is_some_and(|o| banned.contains(&o)))
        .collect()
}

/// Zero every banned output's rows.
pub fn apply_bans(g: &mut [f32], banned: &[BrainOutput]) {
    for (i, w) in g.iter_mut().enumerate().take(BRAIN_END) {
        if output_of(i).is_some_and(|o| banned.iter().any(|&b| b as usize == o)) {
            *w = 0.0;
        }
    }
}

/// **The senses-cut control**: every weight from an input other than `Bias`
/// zeroed, so the brain sees nothing of the world and runs on its bias and
/// its own recurrence alone.
pub fn blinded(g: &[f32]) -> Vec<f32> {
    let mut b = g.to_vec();
    for (i, w) in b.iter_mut().enumerate().take(BRAIN_END) {
        if input_of(i).is_some_and(|inp| inp != BrainInput::Bias as usize) {
            *w = 0.0;
        }
    }
    b
}

/// Where a run starts.
pub enum Start {
    /// Every brain weight zero: from scratch. Variation comes only from the
    /// rig's mutations, so different run seeds are different starts.
    Zero,
    /// Every trainable weight drawn from N(0, sd), seeded by the run seed.
    Random(f32),
    /// The species file's genome, as shipped.
    Species,
    /// A genome file written by this rig (or `brainfit`'s one-per-line form).
    File(PathBuf),
}

/// Write a genome sparse: a header line, then `slot value` for every non-zero
/// slot. A champion with forty live weights is forty lines, not 12,416.
pub fn save_genome(path: &Path, g: &[f32]) -> std::io::Result<()> {
    let mut s = format!("genome {}\n", g.len());
    for (i, &w) in g.iter().enumerate() {
        if w != 0.0 {
            s.push_str(&format!("{i} {w}\n"));
        }
    }
    std::fs::write(path, s)
}

/// Read either form: the sparse one [`save_genome`] writes, or one value per
/// line (`examples/brainfit.rs`).
pub fn load_genome(path: &Path, len: usize) -> Vec<f32> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("genome file {}: {e}", path.display()));
    let mut lines = text.lines();
    if text.starts_with("genome ") {
        lines.next();
        let mut g = vec![0.0; len];
        for l in lines {
            let (i, w) = l.split_once(' ').expect("slot value");
            g[i.parse::<usize>().unwrap()] = w.parse().unwrap();
        }
        g
    } else {
        let g: Vec<f32> = lines.map(|l| l.parse().unwrap()).collect();
        assert_eq!(g.len(), len, "genome length");
        g
    }
}

/// Every setting of a run. `Default` is the setting the first runs used.
#[derive(Clone, Debug)]
pub struct Settings {
    pub gens: usize,
    pub pop: usize,
    /// Layouts per generation; every genome meets all of them.
    pub layouts: usize,
    /// Probability each trainable slot mutates in a child.
    pub pm: f32,
    /// Mutation step, standard deviation.
    pub sigma: f32,
    /// Probability a child is a uniform cross of two parents before mutation.
    pub cross: f32,
    /// Hold-out layouts (different seeds from every training layout).
    pub holdout: usize,
    /// Hold-out every this many generations, and always after the last.
    pub every: usize,
    pub seed: u64,
    pub threads: usize,
    pub pictures: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            gens: 30,
            pop: 24,
            layouts: 2,
            pm: 0.03,
            sigma: 1.0,
            cross: 0.3,
            holdout: 8,
            every: 5,
            seed: 1,
            threads: 4,
            pictures: true,
        }
    }
}

/// Training layouts are numbered from 1,000 + 100,000 x the run seed,
/// hold-out ones from 50,000 and calibration ones from 90,000. For run seed
/// 1 and up the training seeds start at 101,000, clear of both; run seed 0
/// stays clear for its first 49,000 training layouts. A suite's references are
/// set on the calibration set, never on the hold-out it is later judged
/// on: its keep floor is a threshold, so references taken from hold-out
/// scores let hold-out data steer training.
///
/// **Layouts come in mirrored pairs on one seed** (`k` and `k + 1`, `k`
/// even), training as well as hold-out, so a mirrored challenge's two sides
/// differ only by the side and [`fold`] can take the worse of them. An arena
/// that has no sides derives two beds from the pair itself ([`Dig`]'s
/// `bed_seed`), or every pair would be one run twice.
///
/// [`Dig`]: challenges::Dig
pub fn training_layouts(run_seed: u64, gen: usize, n: usize) -> Vec<Layout> {
    (0..n as u64)
        .map(|k| {
            let i = gen as u64 * n as u64 + k;
            Layout { seed: 1000 + run_seed * 100_000 + i / 2, k: i, blind: false }
        })
        .collect()
}
pub fn holdout_layouts(n: usize) -> Vec<Layout> {
    (0..n as u64).map(|k| Layout { seed: 50_000 + k / 2, k, blind: false }).collect()
}
pub fn calibration_layouts(n: usize) -> Vec<Layout> {
    (0..n as u64).map(|k| Layout { seed: 90_000 + k / 2, k, blind: false }).collect()
}

/// One arena's worth of work: which genomes go into it (indices into the
/// slice being scored) and on which layout.
struct Job {
    layout: usize,
    members: Vec<usize>,
    /// How many of `members` are this arena's own; the rest are repeats
    /// topping up an uneven last arena, and their scores are dropped. A
    /// first-come guard here instead let a repeat's score (from another
    /// arena, beside other neighbours) win a race against the genome's own,
    /// so a run was not a function of its seed: `pop=8 threads=4` gave six
    /// different progress files in six runs (coordinator review of PR 707).
    scored: usize,
}

/// **Score every genome on every layout.** Returns `[genome][layout]`.
///
/// Under [`Unit::Colony`] each (genome, layout) is one arena of clones.
/// Under [`Unit::Ant`] the genomes are shuffled into arenas of
/// [`Challenge::ants`] for each layout (a fresh shuffle per layout, so no
/// genome is always beside the same neighbours); a population that does not
/// divide evenly has its last arena topped up with repeats, whose scores
/// are discarded (only each arena's own members are recorded).
pub fn score_all(ch: &dyn Challenge, genomes: &[Vec<f32>], layouts: &[Layout], threads: usize, rng: &mut Rng) -> Vec<Vec<Score>> {
    let mut jobs = Vec::new();
    for l in 0..layouts.len() {
        match ch.unit() {
            Unit::Colony => jobs.extend((0..genomes.len()).map(|g| Job { layout: l, members: vec![g], scored: 1 })),
            Unit::Ant => {
                let mut order: Vec<usize> = (0..genomes.len()).collect();
                for i in (1..order.len()).rev() {
                    order.swap(i, rng.below(i + 1));
                }
                let n = ch.ants();
                for chunk in order.chunks(n) {
                    let mut members = chunk.to_vec();
                    let scored = members.len();
                    let mut fill = 0;
                    while members.len() < n {
                        members.push(order[fill % order.len()]);
                        fill += 1;
                    }
                    jobs.push(Job { layout: l, members, scored });
                }
            }
        }
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let out = std::sync::Mutex::new(vec![vec![None::<Score>; layouts.len()]; genomes.len()]);
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| loop {
                let j = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(job) = jobs.get(j) else { break };
                let gs: Vec<&[f32]> = job.members.iter().map(|&m| genomes[m].as_slice()).collect();
                let scores = ch.run(layouts[job.layout], &gs, &mut |_, _| {});
                let mut o = out.lock().unwrap();
                for (&m, sc) in job.members[..job.scored].iter().zip(scores) {
                    let slot = &mut o[m][job.layout];
                    assert!(slot.is_none(), "genome {m} scored twice on layout {}", job.layout);
                    *slot = Some(sc);
                }
            });
        }
    });
    out.into_inner()
        .unwrap()
        .into_iter()
        .map(|row| row.into_iter().map(|s| s.expect("every genome scored on every layout")).collect())
        .collect()
}

/// A genome's whole score at once: the mean over the layouts it met, fit and
/// parts alike.
pub fn mean_score(v: &[Score]) -> Score {
    let n = v.len().max(1) as f64;
    let parts = v.first().map_or(0, |s| s.parts.len());
    Score {
        fit: v.iter().map(|s| s.fit).sum::<f64>() / n,
        parts: (0..parts).map(|p| v.iter().map(|s| s.parts[p]).sum::<f64>() / n).collect(),
    }
}

/// **One genome's score over `layouts`**: the mean, or for a mirrored
/// challenge the mean over pairs of the **worse side** -- the whole [`Score`]
/// of whichever side fitted worse, parts and all, so the numbers beside it
/// describe the run that set it.
///
/// Why: the noise bar of 2026-10-17 (`rig-2026-10-17c/`) found a forage
/// champion scoring 21-38 on every east layout and 0 on every west one,
/// founders all dead there -- a one-way habit with a mean of 14.0, ahead of
/// the game walk's 8.3, which kept its founders on both sides. A mean over
/// mirrors pays a specialist for the side it happens to face; the worse side
/// pays only for reading the world. A layout without its partner (an odd
/// count) counts alone.
pub fn fold(ch: &dyn Challenge, v: &[Score], layouts: &[Layout]) -> Score {
    if !ch.mirrored() {
        return mean_score(v);
    }
    let mut worse: Vec<Score> = Vec::new();
    let mut i = 0;
    while i < v.len() {
        let pair = i + 1 < v.len() && layouts[i].k.is_multiple_of(2) && layouts[i + 1].k == layouts[i].k + 1;
        if pair {
            worse.push(if v[i + 1].fit < v[i].fit { v[i + 1].clone() } else { v[i].clone() });
            i += 2;
        } else {
            worse.push(v[i].clone());
            i += 1;
        }
    }
    mean_score(&worse)
}

/// Score a genome as a whole colony of clones on `layouts`, whatever the
/// challenge's unit -- how the hold-out reads a champion, so a per-ant skill
/// is judged on what a colony of it does. Folded by [`fold`].
pub fn score_clones(ch: &dyn Challenge, g: &[f32], layouts: &[Layout], threads: usize) -> Score {
    fold(ch, &score_each(ch, g, layouts, threads), layouts)
}

/// [`score_clones`] layout by layout, in `layouts` order: what the noise bar
/// is read from (`examples/train.rs`'s `noise=1`). A mean hides how far one
/// fixed brain's score swings between layouts, and that swing is the bar a
/// difference between two brains has to clear.
pub fn score_each(ch: &dyn Challenge, g: &[f32], layouts: &[Layout], threads: usize) -> Vec<Score> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let out = std::sync::Mutex::new(vec![Score::default(); layouts.len()]);
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| loop {
                let j = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(&l) = layouts.get(j) else { break };
                let gs: Vec<&[f32]> = vec![g; if ch.unit() == Unit::Ant { ch.ants() } else { 1 }];
                let r = mean_score(&ch.run(l, &gs, &mut |_, _| {}));
                out.lock().unwrap()[j] = r;
            });
        }
    });
    out.into_inner().unwrap()
}

/// The mutation operator: each trainable slot moves with probability `pm`
/// by a uniform step of standard deviation `sigma` ([`Rng::step`]), clamped
/// to the range the hand-written genomes use.
pub fn mutate(g: &[f32], slots: &[usize], pm: f32, sigma: f32, rng: &mut Rng) -> Vec<f32> {
    let mut c = g.to_vec();
    for &i in slots {
        if rng.unit() < pm {
            c[i] = (c[i] + sigma * rng.step()).clamp(-40.0, 40.0);
        }
    }
    c
}

/// Uniform crossover **by output row**: a child takes each output's whole
/// incoming row (its input weights and its hidden weights) from one parent
/// or the other. Whole rows, because a weight means something only beside
/// the others feeding the same output; a slot-by-slot cross breaks working
/// circuits far more often than it joins two.
pub fn cross(a: &[f32], b: &[f32], rng: &mut Rng) -> Vec<f32> {
    let take_b: Vec<bool> = (0..64).map(|_| rng.unit() < 0.5).collect();
    let hidden_b: Vec<bool> = (0..64).map(|_| rng.unit() < 0.5).collect();
    let mut c = a.to_vec();
    for i in 0..BRAIN_END {
        let from_b = match (output_of(i), hidden_of(i)) {
            (Some(o), _) => take_b[o],
            (None, Some(h)) => hidden_b[h],
            (None, None) => false,
        };
        if from_b {
            c[i] = b[i];
        }
    }
    c
}

/// A short readout of what a genome attends to: its non-zero weight count
/// and the five inputs with the most total weight.
pub fn describe(g: &[f32]) -> String {
    let mut by_input = [0.0f32; 64];
    let mut live = 0;
    for (i, &w) in g.iter().enumerate().take(BRAIN_END) {
        if w != 0.0 {
            live += 1;
            if let Some(inp) = input_of(i) {
                by_input[inp] += w.abs();
            }
        }
    }
    let mut top: Vec<(f32, usize)> = by_input.iter().enumerate().filter(|(_, &v)| v > 0.0).map(|(i, &v)| (v, i)).collect();
    top.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    let names: Vec<String> = top
        .iter()
        .take(5)
        .map(|&(v, i)| match brain::INPUTS.get(i) {
            Some(x) => format!("{x:?} {v:.1}"),
            None => format!("input{i} {v:.1}"),
        })
        .collect();
    format!("{live} weights; strongest inputs: {}", names.join(", "))
}

/// Build the start genome for a challenge.
pub fn start_genome(start: &Start, species_genome: &[f32], banned: &[BrainOutput], seed: u64) -> Vec<f32> {
    let mut g = match start {
        Start::Species => species_genome.to_vec(),
        Start::File(p) => load_genome(p, species_genome.len()),
        Start::Zero | Start::Random(_) => {
            let mut g = species_genome.to_vec();
            for w in g.iter_mut().take(BRAIN_END) {
                *w = 0.0;
            }
            if let Start::Random(sd) = start {
                let mut rng = Rng(seed ^ 0x5EED_5EED);
                for i in trainable(banned) {
                    g[i] = sd * rng.step();
                }
            }
            g
        }
    };
    apply_bans(&mut g, banned);
    g
}

/// **Run a whole training session.** Writes into `out`:
///
/// * `progress.csv` -- one row per generation: best, median and control fit,
///   then the best genome's parts;
/// * `holdout.csv` -- champion, start and blinded champion on the hold-out
///   layouts, every `every` generations;
/// * `champions/gen-NNN.txt` -- each generation's top genome;
/// * `best.txt` -- the last champion; `pic-gen-NNN.png` -- its picture;
/// * `curves.svg` -- the learning curves, redrawn at every hold-out;
/// * `summary.txt` -- what the run was and how it ended.
pub fn train(ch: &dyn Challenge, start: Vec<f32>, set: &Settings, out: &Path) -> std::io::Result<Vec<f32>> {
    std::fs::create_dir_all(out.join("champions"))?;
    let slots = trainable(ch.banned());
    let mut rng = Rng(set.seed.wrapping_mul(0xA24B_AED4_963E_E407) ^ 0x7EA1);
    let parts = ch.parts();
    let mut progress = std::fs::File::create(out.join("progress.csv"))?;
    writeln!(progress, "gen,best,median,control,secs,{}", parts.join(","))?;
    let mut holdout = std::fs::File::create(out.join("holdout.csv"))?;
    writeln!(holdout, "gen,who,fit,{}", parts.join(","))?;
    let head = format!(
        "challenge={} unit={:?} ants={} frames={} {:?} banned={:?} trainable_slots={}",
        ch.name(),
        ch.unit(),
        ch.ants(),
        ch.frames(),
        set,
        ch.banned(),
        slots.len()
    );
    println!("{head}");
    let hold = holdout_layouts(set.holdout);

    let mut popn: Vec<Vec<f32>> = (0..set.pop)
        .map(|k| if k == 0 { start.clone() } else { mutate(&start, &slots, set.pm, set.sigma, &mut rng) })
        .collect();
    let elites = (set.pop / 4).max(1);
    let t0 = Instant::now();
    let mut champion = start.clone();
    let mut last_hold = String::new();
    println!("gen   best  median  control  [{}]  secs", parts.join(" "));
    for gen in 0..set.gens {
        let tg = Instant::now();
        let layouts = training_layouts(set.seed, gen, set.layouts);
        let mut all = popn.clone();
        // The control rides in the same batch, on the same layouts. Under
        // `Unit::Ant` it is one more member of the shuffle, so it meets the
        // same neighbours the population does.
        all.push(start.clone());
        let scores = score_all(ch, &all, &layouts, set.threads, &mut rng);
        let means: Vec<Score> = scores.iter().map(|s| fold(ch, s, &layouts)).collect();
        let control = means[set.pop].fit;
        let mut ranked: Vec<(f64, usize)> = (0..set.pop).map(|g| (means[g].fit, g)).collect();
        ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then(a.1.cmp(&b.1)));
        let b = ranked[0].1;
        champion = popn[b].clone();
        let secs = tg.elapsed().as_secs_f64();
        let bp: Vec<String> = means[b].parts.iter().map(|p| format!("{p:.2}")).collect();
        writeln!(progress, "{gen},{:.3},{:.3},{:.3},{secs:.1},{}", ranked[0].0, ranked[set.pop / 2].0, control, bp.join(","))?;
        println!("{gen:3} {:7.2} {:7.2} {:7.2}  [{}]  {secs:.0}", ranked[0].0, ranked[set.pop / 2].0, control, bp.join(" "));
        save_genome(&out.join(format!("champions/gen-{gen:03}.txt")), &champion)?;

        if (gen + 1) % set.every.max(1) == 0 || gen + 1 == set.gens {
            let hold_blind = blind(&hold);
            for (who, g, lay) in [("champion", champion.clone(), &hold), ("start", start.clone(), &hold), ("blinded", blinded(&champion), &hold_blind)] {
                let s = score_clones(ch, &g, lay, set.threads);
                let p: Vec<String> = s.parts.iter().map(|p| format!("{p:.2}")).collect();
                writeln!(holdout, "{gen},{who},{:.3},{}", s.fit, p.join(","))?;
                println!("    HOLDOUT {who:8} {:7.2}  [{}]", s.fit, p.join(" "));
                if who == "champion" {
                    last_hold = format!("hold-out champion {:.2}", s.fit);
                } else {
                    last_hold.push_str(&format!(", {who} {:.2}", s.fit));
                }
            }
            println!("    {}", describe(&champion));
            if set.pictures {
                picture(ch, &champion, hold[0], &out.join(format!("pic-gen-{gen:03}.png")));
            }
            curves(out);
        }

        let parents: Vec<Vec<f32>> = ranked[..elites].iter().map(|&(_, g)| popn[g].clone()).collect();
        popn = parents.clone();
        while popn.len() < set.pop {
            let a = &parents[rng.below(elites)];
            let child = if elites > 1 && rng.unit() < set.cross {
                let b = &parents[rng.below(elites)];
                cross(a, b, &mut rng)
            } else {
                a.clone()
            };
            popn.push(mutate(&child, &slots, set.pm, set.sigma, &mut rng));
        }
    }
    save_genome(&out.join("best.txt"), &champion)?;
    let summary = format!(
        "{head}\n{} generations in {:.0} s\n{last_hold}\n{}\n",
        set.gens,
        t0.elapsed().as_secs_f64(),
        describe(&champion)
    );
    std::fs::write(out.join("summary.txt"), &summary)?;
    println!("{summary}");
    Ok(champion)
}

/// Redraw `curves.svg` (the learning curves, `scripts/traincurves.py`) in
/// the run directory after every hold-out, so a run in progress can be
/// watched for a plateau. Best effort: no Python, no curves, no error.
fn curves(out: &Path) {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/traincurves.py");
    let svg = out.join("curves.svg");
    let _ = std::process::Command::new("python3")
        .arg(&script)
        .arg(out)
        .arg("--out")
        .arg(&svg)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

/// Five snapshots of one arena, stacked top to bottom, every ant painted
/// magenta so a colony of one-cell animals is visible at all.
pub fn picture(ch: &dyn Challenge, g: &[f32], layout: Layout, path: &Path) {
    let frames = ch.frames();
    let shots: Vec<u64> = (1..=5).map(|i| i * frames / 5).collect();
    let mut sheet: Vec<u8> = Vec::new();
    let mut size = (0u32, 0u32);
    let mut renderer = crate::render::Renderer::new();
    let particles = crate::sim::particle::ParticleSystem::new();
    let gs: Vec<&[f32]> = vec![g; if ch.unit() == Unit::Ant { ch.ants() } else { 1 }];
    ch.run(layout, &gs, &mut |f, w| {
        if !shots.contains(&f) {
            return;
        }
        let Some(b) = w.bounds() else { return };
        let (vw, vh) = ((b.max_x + 1) as u32, (b.max_y + 1) as u32);
        size = (vw, vh);
        let mut buf = vec![0u8; (vw * vh * 4) as usize];
        let mut ww = w.clone();
        let touched = ww.take_touched_chunks();
        renderer.draw(&ww, &particles, &touched, &mut buf, (vw, vh), true);
        // Trail scent tinted green in open cells, so a trail arena shows its
        // trail; a colony arena shows where its ants have laid one.
        for y in 0..vh as i32 {
            for x in 0..vw as i32 {
                let b = w.pheromone_at(crate::sim::pheromone::Channel::B, x, y);
                if b > 0 && w.get(x, y).material == crate::sim::material::EMPTY {
                    let i = ((y as u32 * vw + x as u32) * 4) as usize;
                    buf[i] /= 2;
                    buf[i + 1] = buf[i + 1] / 2 + 100;
                    buf[i + 2] /= 2;
                }
            }
        }
        for id in w.live_organism_ids() {
            let Some(st) = w.organism(id) else { continue };
            for &(x, y) in &st.chain {
                if x >= 0 && y >= 0 && (x as u32) < vw && (y as u32) < vh {
                    let i = ((y as u32 * vw + x as u32) * 4) as usize;
                    buf[i..i + 4].copy_from_slice(&[255, 0, 255, 255]);
                }
            }
        }
        sheet.extend_from_slice(&buf);
    });
    if size.0 > 0 {
        let rows = (sheet.len() / (size.0 as usize * 4)) as u32;
        if let Err(e) = image::save_buffer(path, &sheet, size.0, rows, image::ColorType::Rgba8) {
            eprintln!("picture {}: {e}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A per-ant arena whose every score depends on the neighbours, so a
    /// filler's score (from another arena) differs from the genome's own.
    /// Arenas whose genome total is in `slow` take a while, so the test can
    /// make each layout's first arena finish after the last one -- the one
    /// topped up with repeats of the first arena's members -- which is the
    /// order in which the old first-come guard recorded the wrong score.
    struct Neighbours {
        slow: Vec<f64>,
    }
    impl Challenge for Neighbours {
        fn name(&self) -> &'static str {
            "neighbours"
        }
        fn unit(&self) -> Unit {
            Unit::Ant
        }
        fn ants(&self) -> usize {
            3
        }
        fn parts(&self) -> &'static [&'static str] {
            &[]
        }
        fn banned(&self) -> &'static [BrainOutput] {
            &[]
        }
        fn run(&self, _: Layout, genomes: &[&[f32]], _: &mut dyn FnMut(u64, &World)) -> Vec<Score> {
            let arena: f64 = genomes.iter().map(|g| g[0] as f64).sum();
            if self.slow.contains(&arena) {
                std::thread::sleep(std::time::Duration::from_millis(40));
            }
            genomes.iter().map(|g| Score { fit: g[0] as f64 + 1000.0 * arena, parts: vec![] }).collect()
        }
        fn frames(&self) -> u64 {
            1
        }
    }

    /// A sided arena that returns a fixed score per side.
    struct Sided {
        east: f64,
        west: f64,
    }
    impl Challenge for Sided {
        fn name(&self) -> &'static str {
            "sided"
        }
        fn unit(&self) -> Unit {
            Unit::Colony
        }
        fn ants(&self) -> usize {
            1
        }
        fn parts(&self) -> &'static [&'static str] {
            &["side"]
        }
        fn banned(&self) -> &'static [BrainOutput] {
            &[]
        }
        fn mirrored(&self) -> bool {
            true
        }
        fn run(&self, l: Layout, _: &[&[f32]], _: &mut dyn FnMut(u64, &World)) -> Vec<Score> {
            let east = l.k.is_multiple_of(2);
            vec![Score { fit: if east { self.east } else { self.west }, parts: vec![if east { 1.0 } else { -1.0 }] }]
        }
        fn frames(&self) -> u64 {
            1
        }
    }

    /// **A one-way habit scores its worse side, not the mean of both**: the
    /// forage champion of 2026-10-17 scored 21-38 east and 0 west, and a
    /// mean put it ahead of a colony that worked both ways. The parts come
    /// from the side that set the score. Training layouts pair on one seed,
    /// so the two sides of a pair are the same arena mirrored.
    #[test]
    fn a_mirrored_arena_scores_the_worse_side() {
        let layouts = training_layouts(3, 2, 4);
        assert_eq!(layouts[0].seed, layouts[1].seed, "a pair shares its seed");
        assert_ne!(layouts[1].seed, layouts[2].seed, "pairs do not");
        let habit = Sided { east: 30.0, west: 0.0 };
        let s = score_clones(&habit, &[], &layouts, 2);
        assert_eq!(s.fit, 0.0, "the west side is the score");
        assert_eq!(s.parts, vec![-1.0], "...and its parts are the west side's");
        let both = Sided { east: 8.0, west: 7.0 };
        assert_eq!(score_clones(&both, &[], &layouts, 2).fit, 7.0);
        // An odd count leaves the last layout to count alone.
        assert_eq!(score_clones(&habit, &[], &layouts[..3], 2).fit, 15.0);
    }

    /// **Each genome keeps its own arena's score, whatever the threads do.**
    /// Seven genomes in arenas of three leave the last arena two short; the
    /// repeats topping it up must not record. Before the fix a first-come
    /// guard let a repeat's score win a race (`pop=8 threads=4` gave six
    /// different runs in six); with each layout's first arena held back here
    /// that guard fails every time, and the fix must not.
    #[test]
    fn uneven_arenas_score_each_genome_in_its_own_arena() {
        let genomes: Vec<Vec<f32>> = (0..7).map(|i| vec![i as f32 + 1.0]).collect();
        let layouts = training_layouts(1, 0, 2);
        // Replay `score_all`'s shuffle to know each layout's arenas.
        let mut rng = Rng(9);
        let mut arenas: Vec<Vec<(Vec<usize>, f64)>> = vec![];
        for _ in 0..layouts.len() {
            let mut order: Vec<usize> = (0..genomes.len()).collect();
            for i in (1..order.len()).rev() {
                order.swap(i, rng.below(i + 1));
            }
            let mut these = vec![];
            for chunk in order.chunks(3) {
                let mut arena: f64 = chunk.iter().map(|&m| genomes[m][0] as f64).sum();
                for fill in 0..3 - chunk.len() {
                    arena += genomes[order[fill % order.len()]][0] as f64;
                }
                these.push((chunk.to_vec(), arena));
            }
            arenas.push(these);
        }
        let ch = Neighbours { slow: arenas.iter().map(|a| a[0].1).collect() };
        let got = score_all(&ch, &genomes, &layouts, 4, &mut Rng(9));
        for (l, these) in arenas.iter().enumerate() {
            for (members, arena) in these {
                for &m in members {
                    assert_eq!(got[m][l].fit, genomes[m][0] as f64 + 1000.0 * arena, "genome {m} on layout {l} has a score from another arena");
                }
            }
        }
    }

    #[test]
    fn slot_maps_agree_with_brain_layout() {
        assert_eq!(output_of(brain::io_slot(BrainInput::Energy, BrainOutput::Move)), Some(BrainOutput::Move as usize));
        assert_eq!(input_of(brain::io_slot(BrainInput::Energy, BrainOutput::Move)), Some(BrainInput::Energy as usize));
        assert_eq!(output_of(brain::ho_slot(3, BrainOutput::Fly)), Some(BrainOutput::Fly as usize));
        assert_eq!(input_of(brain::ih_slot(BrainInput::Crowding, 5)), Some(BrainInput::Crowding as usize));
        assert_eq!(output_of(brain::ih_slot(BrainInput::Crowding, 5)), None);
        assert_eq!(input_of(brain::ho_slot(3, BrainOutput::Fly)), None);
        assert_eq!(input_of(brain::hh_slot(2)), None);
    }

    #[test]
    fn bans_remove_rows_from_training_and_genome() {
        let all = trainable(&[]);
        let some = trainable(&[BrainOutput::Fly, BrainOutput::Impulse]);
        // Two rows of 34 inputs + 8 hidden each.
        assert_eq!(all.len() - some.len(), 2 * (brain::BRAIN_INPUTS + brain::BRAIN_HIDDEN));
        let mut g = vec![1.0; brain::GENOME_LEN];
        apply_bans(&mut g, &[BrainOutput::Fly]);
        assert_eq!(g[brain::io_slot(BrainInput::Bias, BrainOutput::Fly)], 0.0);
        assert_eq!(g[brain::ho_slot(0, BrainOutput::Fly)], 0.0);
        assert_eq!(g[brain::io_slot(BrainInput::Bias, BrainOutput::Move)], 1.0);
    }

    #[test]
    fn blinded_keeps_only_bias_and_recurrence() {
        let g = vec![1.0; brain::GENOME_LEN];
        let b = blinded(&g);
        assert_eq!(b[brain::io_slot(BrainInput::Bias, BrainOutput::Move)], 1.0);
        assert_eq!(b[brain::io_slot(BrainInput::PheroBAlong, BrainOutput::Turn)], 0.0);
        assert_eq!(b[brain::ih_slot(BrainInput::Energy, 1)], 0.0);
        assert_eq!(b[brain::ih_slot(BrainInput::Bias, 1)], 1.0);
        assert_eq!(b[brain::hh_slot(1)], 1.0);
        assert_eq!(b[brain::ho_slot(1, BrainOutput::Move)], 1.0);
    }

    #[test]
    fn genome_round_trips_sparse() {
        let mut g = vec![0.0; brain::GENOME_LEN];
        g[7] = 1.5;
        g[9000] = -0.25;
        let p = std::env::temp_dir().join(format!("train-genome-{}.txt", std::process::id()));
        save_genome(&p, &g).unwrap();
        assert_eq!(load_genome(&p, g.len()), g);
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn mutation_touches_only_trainable_slots_and_is_seeded() {
        let slots = trainable(&[BrainOutput::Fly]);
        let g = vec![0.0; brain::GENOME_LEN];
        let a = mutate(&g, &slots, 0.5, 1.0, &mut Rng(3));
        let b = mutate(&g, &slots, 0.5, 1.0, &mut Rng(3));
        assert_eq!(a, b, "same seed, same child");
        for (i, &w) in a.iter().enumerate() {
            if w != 0.0 {
                assert!(slots.contains(&i), "slot {i} moved but is not trainable");
            }
        }
        assert!(a.iter().filter(|&&w| w != 0.0).count() > slots.len() / 4);
    }

    #[test]
    fn crossover_takes_whole_output_rows() {
        let a = vec![0.0; brain::GENOME_LEN];
        let b = vec![1.0; brain::GENOME_LEN];
        let c = cross(&a, &b, &mut Rng(9));
        for o in brain::OUTPUTS {
            let first = c[brain::io_slot(BrainInput::Bias, o)];
            for i in brain::INPUTS {
                assert_eq!(c[brain::io_slot(i, o)], first, "{o:?}'s row is split");
            }
            for h in 0..brain::BRAIN_HIDDEN {
                assert_eq!(c[brain::ho_slot(h, o)], first, "{o:?}'s hidden row is split");
            }
        }
    }
}
