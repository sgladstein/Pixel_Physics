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
//! * **a hold-out** on layouts training never sees, every generation (`every=`):
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
//!
//! **Two brains, told apart by genome length.** Today's brain (`brain.rs`,
//! [`SCORER`] slots and the blocks after it) and stage 2's general brain
//! (`creature::mind`, `mind::GENOME_LEN`): every function here that reads a
//! genome's layout asks [`is_mind`] first, and an arena puts a mind genome
//! into its ants as a mind ([`Challenge::minds`]). One run can score both
//! side by side, which is how a rung's champion is held against today's
//! brain trained the same way.

pub mod challenges;

use crate::sim::brain::{self, BrainInput, BrainOutput};
use crate::sim::creature::mind;
use crate::sim::creature::options::Verb;
use crate::sim::creature::targets;
use crate::sim::organism;
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
///
/// `legal` is the target scorer's **matched-rate legality control**: the
/// scorer still decides whether to act or to answer "none", and the cell is
/// then drawn evenly from the legal ones (`creature::targets::Targets::even`).
/// Zeroing the scorer's weights instead (the first version) changed how often
/// it declined as well as what it picked, so a champion could beat that row
/// by declining more or less often (ladder lane, `stage1-code-review.md` 4).
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub seed: u64,
    pub k: u64,
    pub blind: bool,
    pub legal: bool,
}

/// The same layouts with the body's senses cut too; see [`Layout::blind`].
pub fn blind(layouts: &[Layout]) -> Vec<Layout> {
    layouts.iter().map(|&l| Layout { blind: true, ..l }).collect()
}

/// The same layouts with every target drawn evenly from the legal ones; see
/// [`Layout::legal`].
pub fn legal(layouts: &[Layout]) -> Vec<Layout> {
    layouts.iter().map(|&l| Layout { legal: true, ..l }).collect()
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
    /// Set any reference scores from the start genome before training;
    /// `saved` is the start before the rung's entry ([`start_mind_saved`]),
    /// which a kept arena is measured on. Only a suite has any.
    fn calibrate(&mut self, _start: &[f32], _saved: &[f32], _layouts: &[Layout], _threads: usize) {}
    /// Whether a suite's forget floor for this arena, kept, is two standard
    /// errors under its reference rather than a share of it
    /// (`challenges::Suite`'s floor; R12).
    fn floor_in_se(&self) -> bool {
        false
    }
    /// Whether odd layouts mirror even ones (the food or the trail on the
    /// other side). A mirrored challenge is scored by the worse side of each
    /// pair ([`fold`]), not the mean.
    fn mirrored(&self) -> bool {
        false
    }
    /// **How a genome's per-layout scores become its score**: by default
    /// the mean, or the worse side of each mirrored pair ([`worse_side`]).
    /// A suite folds each of its arenas on its own first ([`fold`]).
    fn combine(&self, v: &[Score], layouts: &[Layout]) -> Score {
        worse_side(self.mirrored(), v, layouts)
    }
    /// Whether this arena's genomes carry an evolved target scorer after the
    /// brain ([`SCORER`]): which cell is dug and where a pellet goes down,
    /// in place of the hand rules (`creature::targets`).
    fn scorer(&self) -> bool {
        false
    }
    /// Cells to paint over a picture of this arena in fixed colours, so the
    /// picture shows what the score counted rather than only what the
    /// renderer draws: a full replace, never a blend into the cell's own
    /// colour (`CLAUDE.md`, *A debug readout must not be a function of the
    /// thing it debugs*). Dig paints its room, its pit and what it refused.
    fn marks(&self, _layout: Layout, _w: &World) -> Vec<((i32, i32), [u8; 3])> {
        Vec::new()
    }
    /// Whether this arena can run stage 2 minds (`creature::mind`): it puts
    /// a mind genome into its ants as a mind. Each arena opts in as its
    /// rung is built.
    fn minds(&self) -> bool {
        false
    }
    /// The verbs a mind may use here; the rest stay switched off and are
    /// never trained.
    fn verbs(&self) -> &'static [Verb] {
        &[Verb::Walk, Verb::TurnAround, Verb::Rest]
    }
    /// The scents a mind may lay here (A, B, alarm): A and B as the brain's
    /// `EmitA`/`EmitB` bans say, alarm off.
    fn mind_emits(&self) -> [bool; mind::EMITS] {
        let b = self.banned();
        [!b.contains(&BrainOutput::EmitA), !b.contains(&BrainOutput::EmitB), false]
    }
}

/// **Whether a genome is a stage 2 mind** (`creature::mind`), by its length.
pub fn is_mind(len: usize) -> bool {
    len == mind::GENOME_LEN
}

/// **A mind's start genome**: a preset by name (`mind::preset`: `blank` or
/// `zero`, `rest`, `follower`, `persist`, `forager`) over the arena's
/// verbs, or a genome file this rig wrote -- an earlier rung's champion,
/// which enters this rung by [`mind::enter_rung`] -- the arena's bans
/// applied. A file saved in an older layout is read into today's
/// ([`mind::pad`]), picking as it did.
pub fn start_mind(ch: &dyn Challenge, start: &str) -> Vec<f32> {
    load_mind(ch, start, true)
}

/// **The start as saved**: [`start_mind`] without the rung's entry
/// ([`mind::enter_rung`]), the arena's bans applied. A suite measures its
/// kept arenas on it (R12), so what the entry itself costs them shows as a
/// loss rather than becoming the baseline.
pub fn start_mind_saved(ch: &dyn Challenge, start: &str) -> Vec<f32> {
    load_mind(ch, start, false)
}

/// What one live link bills a tick at the ant's authored budget
/// (`synapse_fraction` x `start_energy`); a budget rescales the fraction so
/// that the bill stays this (`challenges::budget_to`).
fn link_price() -> f32 {
    let reg = organism::SpeciesRegistry::builtin();
    reg.id_of("ant").and_then(|id| reg.get(id).creature.as_ref()).map_or(0.0, |d| d.synapse_fraction * d.start_energy)
}

fn load_mind(ch: &dyn Challenge, start: &str, enter: bool) -> Vec<f32> {
    assert!(ch.minds(), "{} cannot run minds yet", ch.name());
    let name = if start == "zero" { "blank" } else { start };
    let mut g = match mind::preset(name, ch.verbs()) {
        Some(g) => g,
        None => {
            let p = Path::new(start);
            let len = genome_file_len(p);
            let mut g = mind::pad(load_genome(p, len)).unwrap_or_else(|| {
                panic!(
                    "{start}: a genome of {len} slots is not a mind ({}; {} saved before the job's hidden units, {} before PR 2's features); start= blank|rest|follower|persist|forager|<mind file>",
                    mind::GENOME_LEN,
                    mind::GENOME_LEN_V1,
                    mind::GENOME_LEN_V2
                )
            });
            // Saved before Pick up had its own act: once, whether it is
            // trained from or scored, so a champion runs as it was trained
            // to pick up (fix plan step 1).
            match genome_file_layout(p) {
                None => {
                    mind::split_pick_up(&mut g);
                    println!("{start}: saved before Pick up had its own act; Eat's links copied to Pick up, Eat quieted");
                }
                Some(l) => assert!(l <= mind::LAYOUT, "{start}: layout {l} is newer than this build's {}", mind::LAYOUT),
            }
            if enter {
                let live = |g: &[f32]| mind::MindGenome::new(g.to_vec()).live;
                let before = live(&g);
                let did = mind::enter_rung(&mut g, ch.verbs(), ch.mind_emits());
                if !did.is_empty() {
                    let after = live(&g);
                    println!(
                        "{start} enters {}: {}; live links {before} -> {after}, {:+.4} J a tick",
                        ch.name(),
                        did.join(", "),
                        (after as f32 - before as f32) * link_price()
                    );
                }
            }
            g
        }
    };
    mind::apply_bans(&mut g, ch.verbs(), ch.mind_emits());
    g
}

/// Where a genome's target scorer starts, when its arena has one: right after
/// the brain's own `brain::GENOME_LEN` slots, [`targets::WEIGHTS`] long.
pub const SCORER: usize = brain::GENOME_LEN;

/// **The walk genes**, a block after the brain (and after the scorer, where
/// there is one): one allele for each of `organism::WALK_SLOTS`, in its
/// order, then the scout's reserve (`creature::FoodTrail::reserve`). At all
/// zeros it is the shipped walk exactly. Opt in with `genes=walk`.
///
/// **Why the rig moves them.** A forager trained on today's brain brought
/// home 8x the game ant's food and kept 0.81 of its founders against 0.96:
/// its scouts on the side away from the food walked to the box wall and
/// starved coming back, and what decides when a scout gives up was code no
/// genome reached (ladder lane, `forage-founder-deaths.md`). Of the walk
/// genes, neither the scout pull nor patience was the lever there
/// (`forage-walk-genes.md`); the reserve was. The others ride along because
/// a lineage that can retune its brain should be able to retune its walk.
///
/// The alleles go into each ant's own traits, where the walk reads them
/// (`creature::traits_of`), so births and eggs inherit them; the reserve
/// is the world's (`World::food_trail`), so one genome per world: walk genes
/// need a colony arena ([`Unit::Colony`]).
///
/// **Only the game walk reads them** (`creature::chooser_step`): under the
/// brain's own walk, the blinded control and the trail arena an ant steps
/// by `step_chain`, which reads none of them, so `examples/train.rs` refuses
/// `genes=walk` without `walk=game` (review of PR 711, N2). The needs walk
/// (`creature::needs`) does not read the reserve either.
pub const WALK: usize = organism::WALK_SLOTS.len() + 1;

/// The walk block's names, in order, for `walkset=` and [`describe`].
pub const WALK_NAMES: [&str; WALK] = ["home", "trail", "away", "scout", "patience", "door", "ladenA", "ladenB", "emptyA", "emptyB", "memory", "reserve"];

/// Where a genome's blocks start, read off its length: the brain alone, the
/// brain and a scorer, the brain and walk genes, or all three. The four
/// lengths cannot collide (`targets::WEIGHTS` is not [`WALK`]), and any
/// other length is a file from somewhere else and stops the run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Blocks {
    pub scorer: Option<usize>,
    pub walk: Option<usize>,
}

pub fn blocks(len: usize) -> Blocks {
    if is_mind(len) {
        return Blocks { scorer: None, walk: None };
    }
    let (sc, wk) = (targets::WEIGHTS, WALK);
    match len.checked_sub(SCORER) {
        Some(0) => Blocks { scorer: None, walk: None },
        Some(n) if n == sc => Blocks { scorer: Some(SCORER), walk: None },
        Some(n) if n == wk => Blocks { scorer: None, walk: Some(SCORER) },
        Some(n) if n == sc + wk => Blocks { scorer: Some(SCORER), walk: Some(SCORER + sc) },
        _ => panic!("a genome of {len} slots: the brain is {SCORER}, a target scorer {sc} more, walk genes {wk} more"),
    }
}

/// A challenge's whole genome length: the brain, the scorer if it has one,
/// and the walk genes if the run carries them.
pub fn genome_len(ch: &dyn Challenge, walk: bool) -> usize {
    SCORER + if ch.scorer() { targets::WEIGHTS } else { 0 } + if walk { WALK } else { 0 }
}

/// The slots the rig may move for `ch`, in a genome `len` long:
/// [`trainable`], or, in an arena with a target scorer, **the scorer's
/// slots and not the brain's**; and the walk genes, where the genome
/// carries them.
///
/// The brain is frozen beside a scorer because stage 1 asks whether choosing
/// the *cell* beats the hand rules. Evolving the brain beside it let a
/// champion win by retuning how often it digs and drops without choosing any
/// better (ladder lane, `stage1-code-review.md` 3); what retuning the brain
/// alone buys is its own arm, today's brain trained under the hand rules.
/// Walk genes beside a scorer would confound it the same way, and a run that
/// asks for both gets both: the `walk-shipped` hold-out row says how much
/// of the score is theirs.
pub fn trainable_for(ch: &dyn Challenge, len: usize) -> Vec<usize> {
    if is_mind(len) {
        assert!(ch.minds(), "{} cannot run minds yet", ch.name());
        return mind::trainable(ch.verbs(), ch.mind_emits());
    }
    let b = blocks(len);
    let mut v: Vec<usize> = if ch.scorer() {
        let s = b.scorer.expect("a scorer arena's genome carries a scorer");
        (s..s + targets::WEIGHTS).collect()
    } else {
        assert!(b.scorer.is_none(), "a genome with a target scorer in an arena that has none");
        trainable(ch.banned())
    };
    if let Some(w) = b.walk {
        v.extend(w..w + WALK);
    }
    v
}

/// The genome with its walk genes at the shipped walk (all zeros), or as it
/// is if it carries none.
pub fn walk_shipped(g: &[f32]) -> Vec<f32> {
    let mut v = g.to_vec();
    if let Some(w) = blocks(g.len()).walk {
        v[w..w + WALK].fill(0.0);
    }
    v
}

/// The genome without its target scorer, for the `hand` row.
fn without_scorer(g: &[f32]) -> Vec<f32> {
    let b = blocks(g.len());
    let mut v = g[..SCORER].to_vec();
    if let Some(w) = b.walk {
        v.extend_from_slice(&g[w..w + WALK]);
    }
    v
}

/// **The hold-out rows**, every one on the same layouts: the genome, the
/// start, and the senses-cut control ([`blinded`] on [`blind`] layouts);
/// and, in an arena with a target scorer (stage 1's decision rows,
/// `ladder/stage1-code-review.md`):
/// * `legal` -- the genome on [`legal`] layouts: what the list of legal
///   targets gives for free, the floor a trained scorer must clear;
/// * `scorer-blind` -- the brain untouched, the scorer's features cut
///   ([`scorer_blinded`]), on the ordinary layouts. `blinded` also forces
///   the brain walk and cuts the home pull, which alone takes the game ant's
///   dig from about 77 to about 7, so a low `blinded` row cannot show the
///   scorer reads the world; this row can;
/// * `hand` -- the genome's brain on the hand rules, its scorer dropped,
///   which says how much of a score is the brain's.
///
/// And, in a genome with walk genes ([`WALK`]):
/// * `walk-shipped` -- the genome with its walk genes back at the shipped
///   walk, which says how much of a score is theirs.
pub fn controls(ch: &dyn Challenge, g: &[f32], start: &[f32], hold: &[Layout]) -> Vec<(&'static str, Vec<f32>, Vec<Layout>)> {
    let mut rows = vec![("genome", g.to_vec(), hold.to_vec()), ("start", start.to_vec(), hold.to_vec()), ("blinded", blinded(g), blind(hold))];
    if ch.scorer() {
        rows.push(("legal", g.to_vec(), legal(hold)));
        rows.push(("scorer-blind", scorer_blinded(g), hold.to_vec()));
        rows.push(("hand", without_scorer(g), hold.to_vec()));
    }
    if blocks(g.len()).walk.is_some() {
        rows.push(("walk-shipped", walk_shipped(g), hold.to_vec()));
    }
    rows
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
///
/// A target scorer, if the genome has one, is cut too ([`scorer_blinded`]).
pub fn blinded(g: &[f32]) -> Vec<f32> {
    if is_mind(g.len()) {
        return mind::blinded(g);
    }
    let mut b = scorer_blinded(g);
    for (i, w) in b.iter_mut().enumerate().take(BRAIN_END) {
        if input_of(i).is_some_and(|inp| inp != BrainInput::Bias as usize) {
            *w = 0.0;
        }
    }
    b
}

/// **The scorer's senses cut, and nothing else**: every weight from a
/// feature zeroed, keeping the verb's own row, its bias and its "none"
/// constants, so it scores every target of a verb alike. The brain is left
/// as it is.
pub fn scorer_blinded(g: &[f32]) -> Vec<f32> {
    let mut b = g.to_vec();
    if let Some(s) = blocks(b.len()).scorer {
        for f in 0..targets::FEATURES {
            for h in 0..targets::HIDDEN {
                b[s + targets::w_in(targets::in_feature(f), h)] = 0.0;
            }
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
/// slot. A champion with forty live weights is forty lines, not 12,416. A
/// mind's header names its layout (`genome 2797 layout=4`,
/// [`mind::LAYOUT`]), so a reader knows whether it was saved before Pick up
/// had its own act; a binary from before the mark refuses the file rather
/// than read it as its own. **An unmarked file is read as old and converted**
/// (review of PR 724, N3), so a mind file written by hand, or in
/// `brainfit`'s one-per-line form, needs `layout=4` in its header or its
/// Pick up row is overwritten by Eat's.
pub fn save_genome(path: &Path, g: &[f32]) -> std::io::Result<()> {
    let mut s = if is_mind(g.len()) {
        format!("genome {} layout={}\n", g.len(), mind::LAYOUT)
    } else {
        format!("genome {}\n", g.len())
    };
    for (i, &w) in g.iter().enumerate() {
        if w != 0.0 {
            s.push_str(&format!("{i} {w}\n"));
        }
    }
    std::fs::write(path, s)
}

/// How long the genome in a file is: the sparse form's header, or its line
/// count.
pub fn genome_file_len(path: &Path) -> usize {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("genome file {}: {e}", path.display()));
    match text.lines().next().and_then(|l| l.strip_prefix("genome ")) {
        Some(n) => n
            .split_whitespace()
            .next()
            .and_then(|n| n.parse().ok())
            .unwrap_or_else(|| panic!("genome file {}: bad header", path.display())),
        None => text.lines().count(),
    }
}

/// **The layout a genome file names** (`layout=N` in its header,
/// [`save_genome`]), or `None` for one saved before the mark.
pub fn genome_file_layout(path: &Path) -> Option<u32> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("genome file {}: {e}", path.display()));
    let head = text.lines().next()?.strip_prefix("genome ")?;
    head.split_whitespace().find_map(|t| t.strip_prefix("layout=")).map(|n| {
        n.parse().unwrap_or_else(|_| panic!("genome file {}: bad layout {n}", path.display()))
    })
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
    /// Every generation by default: the owner reads the hold-out curve
    /// generation by generation from the start (2026-10-17), and a curve
    /// checked every fifth generation hid how the champion moves between.
    /// It costs about a tenth more a run (three genomes on the hold-out
    /// beside a population on the training layouts) and draws nothing from
    /// the training stream, so a run is the same run at any cadence.
    pub every: usize,
    pub seed: u64,
    pub threads: usize,
    pub pictures: bool,
    /// Probability that a mutation landing on a live weight sets it to 0
    /// instead of stepping it: a brain weight, or a target scorer's where
    /// the genome carries one; never a walk gene, which has its own axis.
    /// 0 is the operator the first runs used, draw for draw; 0.3, the
    /// default since 2026-10-17, is the trail rung's chosen setting beside
    /// [`Settings::wire`].
    ///
    /// **Why**: a step never lands on exactly 0, so a brain could only grow.
    /// The forage champion grew to 419 live weights, and at
    /// `brain::SYNAPSE_TAX`'s price that was 31% of each ant's burn; priced
    /// like the game ant's 8x sparser brain, the same champion kept 0.97 of
    /// its founders against 0.81 and scored 13% more (ladder lane,
    /// `forage-walk-genes.md`). Selection already pays for a dense brain in
    /// deaths; this gives it a move that makes one sparser.
    pub prune: f32,
    /// **The design's cost per connection** (rev3, *Size, cost and
    /// growth*): selection ranks each genome by its fit less `|fit|` times
    /// `1 - exp(-wire * connections)`, its share lost to [`connections`].
    /// A share rather than a sum so one setting means the same on the
    /// trail's 0-1 score and a colony's head count; at 5e-5 a thousand
    /// connections cost about 5% of the score and a hundred 0.5%. Selection
    /// only: the printed and saved scores are the arena's own.
    ///
    /// **Why**: an arena that cannot starve its ants never charges for a
    /// brain. The trail arena raises the budget to 1e9 and bills each tax
    /// at the authored budget's size, about 1 a tick for a full mind, which
    /// an f32 at 1e9 (one step is 64) never takes off (review of PR 713 at
    /// `d8a55651`, N1). Its champions grew to about 1,000 connections, and
    /// started the forage rung paying four times an idle ant's bill in
    /// brain tax. 0 is the rig as before, draw for draw.
    ///
    /// **1e-3, the default since 2026-10-17** (with `prune` 0.3 and `grow`
    /// 1): on the trail rung from blank it held a mind to 383 connections
    /// at 0.89 against 1,160 at 0.94 with no cost, and the lean champions
    /// it made are the ones the forage rung starts from
    /// (`brain-training/stage2/trail-wire`, `trail-lean`). Owner, 2026-10-17:
    /// keep the brain's cost because it helps, not because it was there.
    pub wire: f64,
    /// Probability that a mutation landing on an empty slot makes it a
    /// connection; otherwise the slot stays empty. 1, the default, is the
    /// operator as before, draw for draw.
    ///
    /// **Why**: a step on an empty slot always made a connection, and two
    /// thirds of a mind's slots start empty, so each child carried about
    /// 45 new connections a generation and a mind grew by 25-30 a
    /// generation whatever it scored: 63-82 at generation 0 of the trail
    /// rung, 1,095-1,160 at 39 (`brain-training/stage2/trail-fixed`).
    /// Mutation alone drifts a brain towards `grow / prune` live per empty
    /// slot; at 1 with no pruning that is every slot. A cost per connection
    /// ([`Settings::wire`]) pushes against the drift; this sets how hard
    /// the drift pushes.
    ///
    /// **Under 1 from a blank start, the trail rung can stall standing
    /// still** (trail, seed 1, 40 generations, 2026-10-17,
    /// `brain-training/stage2/trail-wire`): a mind that never moves scores
    /// exactly 0, and one that moves at random scores just below 0 on the
    /// worse side of a mirrored pair, so while children are near-blank the
    /// one that stands still wins. At `grow` 0.1 (wire 2e-4) and 0.3 (wire
    /// 1e-3) every champion stood still for all 40 generations; at 0.3
    /// with wire 2e-4 it left at generation 16 and reached 0.93 at 466
    /// connections. At 1 with `prune` 0.3 every cost tried left by
    /// generation 1: wire 2e-4 0.89 at 710 connections, 5e-4 0.92 at 594,
    /// 1e-3 0.89 at 383, 2e-3 0.75 at 383, against 0.94 at 1,160 with no
    /// cost. Those all ran at `pm` 0.03, population 48 and crossover 0.3;
    /// the design's smaller steps (`pm` 0.007, population 100, layouts 8,
    /// no crossover) at `grow` 1, `prune` 0.3 and wire 2e-4 stood still
    /// for all 40 generations at 63 connections (`trail-wire/P1`), so the
    /// trap is about how near-blank the children stay, not `grow` alone. A
    /// rung that starts from a champion that already moves is not exposed
    /// to this.
    pub grow: f32,
    /// **Stop a run once its hold-out has gone flat** ([`flat`]): after
    /// this many generations with no new hold-out best, and never before
    /// [`STOP_MIN`] or before the flat stretch is as long as the climb to
    /// the best. 0 runs every generation; 30 unless `patience=` sets it --
    /// on by default, as features here are (owner, 2026-09-27).
    pub patience: usize,
    /// **Carry on a run from its `state.txt`** ([`RunState`]) in the same
    /// `out` directory, as if it had never stopped: the generation count,
    /// the population's parents, the random stream, the hold-out best and
    /// [`flat`]'s clock all pick up where the last whole generation left
    /// them. Raise `gens` to run more; a run that stopped flat stops again
    /// at its next hold-out unless `patience` is 0. At `every` above 1 a
    /// leg's last generation is held out, as every run's is, where a run
    /// straight through may not hold that one out: the population is the
    /// same either way, the hold-out best may not be. A state saved on
    /// another arena ([`Settings::arena`]), with other parts or another
    /// `holdout`, or one cut short, is refused (review of PR 726, S1, S2,
    /// N10); other settings only warn ([`RunState::load`]).
    ///
    /// **Why**: a run saved only each generation's champion, so carrying
    /// one on meant a fresh population of one ant's children, a dip, and
    /// `patience`'s clock back at 0 (owner, 2026-10-19: "restart one
    /// exactly where it was and just add an extra 50 generations"). A
    /// cloud container that restarts mid-run lost its run the same way.
    pub resume: bool,
    /// **The arena's own arguments**, as the command line gave them
    /// (`examples/train.rs`: challenge, arenas, step, shaft, frames, dist,
    /// budget and the rest), for [`RunState`]: a resume under another
    /// arena is refused rather than carrying this one's hold-out best,
    /// [`flat`]'s clock and `hold-best.txt` into it (review of PR 726, S1:
    /// `step=3` on a step-4 state went on silently and mixed both rungs in
    /// one `holdout.csv`). The challenge's own name, ants, frames and banned
    /// outputs are stored beside it whatever it holds, so empty is safe.
    pub arena: String,
}

/// No run stops before this many generations, however flat ([`flat`]).
pub const STOP_MIN: usize = 40;

/// A hold-out best counts only above the last one by this share of it
/// ([`flat`]).
pub const STOP_GAIN: f64 = 0.02;

/// **Whether a run has gone flat**: `patience` generations since the last
/// hold-out best (`best_gen`, a best being [`STOP_GAIN`] above the last),
/// at least [`STOP_MIN`] run, and the flat stretch at least as long as the
/// climb, counted in generations run, so a best at generation 25 (the 26th
/// run) runs to 51 at least (owner, 2026-10-18: "make sure we have a smart
/// measure ... take into account variability").
///
/// **Why these numbers.** Curves here are staircases, not power laws: a
/// climb in the first 5-20 generations, then long flats with a step now
/// and then. Over the 23 past runs that went 40 generations
/// (`brain-training/stage2`), today's brain on trail sat flat for 29
/// generations and then gained 4%, two mind forage runs 19-20 and then
/// 3-5%, and a forage seed started at 0.17 and reached 5.3 by generation
/// 22. A plain 20-generation stop would have ended 9 of the 23, one before
/// its last 4% step. Those runs were all 40 generations, so [`STOP_MIN`]
/// alone keeps this rule from ending any of them early, and that backtest
/// could not have failed (review of PR 717, N17). **The margin is thin**:
/// the trail staircase's 4% came at generation 38, and with its best at 8
/// this rule would have stopped it at 39. Steps under 2% are inside a
/// champion's own spread there (0-20% between successive champions on a
/// flat stretch).
pub fn flat(gen: usize, best_gen: usize, patience: usize) -> bool {
    patience > 0 && gen + 1 >= STOP_MIN && gen - best_gen >= patience && gen > 2 * best_gen
}

/// Whether `fit` is a new hold-out best over `best` ([`STOP_GAIN`]).
pub fn new_best(fit: f64, best: f64) -> bool {
    !best.is_finite() || fit > best + STOP_GAIN * best.abs()
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
            every: 1,
            seed: 1,
            threads: 4,
            pictures: true,
            prune: 0.3,
            wire: 1e-3,
            grow: 1.0,
            patience: 30,
            resume: false,
            arena: String::new(),
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
/// differ only by the side and [`fold`] can take the worse of them. Dig's
/// beds have sides too: one bed per pair, drawn from the seed, with its
/// one-sided features (the stone lens, the step) mirrored on odd `k`
/// ([`challenges::DigBed`]), so a pair is one bed met from both sides.
///
/// **The dig hold-out draws few beds.** 8 hold-out layouts are 4 beds,
/// with soil depth 29-39 and never the 41 lamp spacing; 12 are 6 beds.
/// Training draws a new bed per pair per generation, so it sees the whole
/// range; a champion's hold-out score is on the narrower set (coordinator
/// review of PR 708 cf9b4350). [`sealed_layouts`] is a further 6.
///
/// [`Dig`]: challenges::Dig
pub fn training_layouts(run_seed: u64, gen: usize, n: usize) -> Vec<Layout> {
    (0..n as u64)
        .map(|k| {
            let i = gen as u64 * n as u64 + k;
            Layout { seed: 1000 + run_seed * 100_000 + i / 2, k: i, blind: false, legal: false }
        })
        .collect()
}
pub fn holdout_layouts(n: usize) -> Vec<Layout> {
    (0..n as u64).map(|k| Layout { seed: 50_000 + k / 2, k, blind: false, legal: false }).collect()
}
/// **A second hold-out nobody has looked at**, for the final rows once a
/// champion has been chosen by its hold-out score (picking among seeds or
/// checkpoints by the hold-out is selection on it). `sealed=1` in the
/// example scores on these instead (ladder lane's stage 1 review, s3).
pub fn sealed_layouts(n: usize) -> Vec<Layout> {
    (0..n as u64).map(|k| Layout { seed: 70_000 + k / 2, k, blind: false, legal: false }).collect()
}
pub fn calibration_layouts(n: usize) -> Vec<Layout> {
    (0..n as u64).map(|k| Layout { seed: 90_000 + k / 2, k, blind: false, legal: false }).collect()
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
///
/// The challenge decides how ([`Challenge::combine`]): a suite folds each
/// arena on its own before it multiplies them (`challenges::Suite`).
pub fn fold(ch: &dyn Challenge, v: &[Score], layouts: &[Layout]) -> Score {
    ch.combine(v, layouts)
}

/// The units a fold averages over, in order: each mirrored pair (`k` even
/// and `k + 1`) as two indices, and a layout without its partner alone;
/// every layout alone when nothing is mirrored.
pub fn sides(mirrored: bool, layouts: &[Layout]) -> Vec<(usize, Option<usize>)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < layouts.len() {
        let pair = mirrored && i + 1 < layouts.len() && layouts[i].k.is_multiple_of(2) && layouts[i + 1].k == layouts[i].k + 1;
        if pair {
            out.push((i, Some(i + 1)));
            i += 2;
        } else {
            out.push((i, None));
            i += 1;
        }
    }
    out
}

/// [`fold`]'s default: the mean of `v`, or of the worse side of each
/// mirrored pair, parts and all ([`sides`]).
pub fn worse_side(mirrored: bool, v: &[Score], layouts: &[Layout]) -> Score {
    let worse: Vec<Score> = sides(mirrored, layouts)
        .into_iter()
        .map(|(a, b)| match b {
            Some(b) if v[b].fit < v[a].fit => v[b].clone(),
            _ => v[a].clone(),
        })
        .collect();
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
    score_each_rows(ch, &[(g, layouts)], threads).remove(0)
}

/// [`score_each`] for several genomes, each on its own layouts, as **one
/// pool of jobs**: how the hold-out scores [`controls`]' rows, so the
/// champion's layouts and the blinded control's share the threads rather
/// than taking turns. One row at a time, 12 hold-out layouts kept at most
/// 12 threads busy and cost one arena length a row: 22 s of every
/// 6-generation trail run at 16, 32 and 64 vCPU alike, 46% of the run at 64
/// (cloud lane, `cpu-use-and-the-serial-part-2026-10-19.md`). Row `k`'s
/// scores come back in its layouts' order, as [`score_each`] gives them,
/// and each job is the run [`score_each`] makes, so no score moves.
pub fn score_each_rows(ch: &dyn Challenge, rows: &[(&[f32], &[Layout])], threads: usize) -> Vec<Vec<Score>> {
    let jobs: Vec<(usize, usize)> = rows.iter().enumerate().flat_map(|(k, r)| (0..r.1.len()).map(move |j| (k, j))).collect();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let out = std::sync::Mutex::new(rows.iter().map(|r| vec![Score::default(); r.1.len()]).collect::<Vec<_>>());
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(&(k, j)) = jobs.get(i) else { break };
                let (g, layouts) = rows[k];
                let gs: Vec<&[f32]> = vec![g; if ch.unit() == Unit::Ant { ch.ants() } else { 1 }];
                let r = mean_score(&ch.run(layouts[j], &gs, &mut |_, _| {}));
                out.lock().unwrap()[k][j] = r;
            });
        }
    });
    out.into_inner().unwrap()
}

/// [`score_clones`] for each of `rows`, every row's layouts in one pool
/// ([`score_each_rows`]).
pub fn score_rows(ch: &dyn Challenge, rows: &[(&[f32], &[Layout])], threads: usize) -> Vec<Score> {
    score_each_rows(ch, rows, threads).iter().zip(rows).map(|(each, r)| fold(ch, each, r.1)).collect()
}

/// The mutation operator: each trainable slot moves with probability `pm`
/// by a uniform step of standard deviation `sigma` ([`Rng::step`]), clamped
/// to the range the hand-written genomes use; a live weight is set to 0
/// instead with probability `prune` ([`Settings::prune`]), and an empty one
/// made live only with probability `grow` ([`Settings::grow`]).
///
/// **A walk gene moves on its own terms** ([`WALK_PM`], [`WALK_SIGMA`]):
/// its axis is ±1 where a weight's is ±40, so a weight's step would cross
/// it in one go, and at a weight's `pm` a twelve-gene block would see one
/// move every third child. The reserve is held to `0..=1`, the rest to
/// `±1`, where the walk itself clamps a gain (`creature::walk_gain`).
pub fn mutate(g: &[f32], slots: &[usize], pm: f32, sigma: f32, prune: f32, grow: f32, rng: &mut Rng) -> Vec<f32> {
    let mut c = g.to_vec();
    let walk = blocks(g.len()).walk;
    for &i in slots {
        if let Some(w) = walk.filter(|&w| i >= w) {
            if rng.unit() < WALK_PM {
                let lo = if i - w == WALK - 1 { 0.0 } else { -1.0 };
                c[i] = (c[i] + WALK_SIGMA * rng.step()).clamp(lo, 1.0);
            }
        } else if rng.unit() < pm {
            // A mind's settings are not connections: trimmed, a verb's
            // switch goes to 0, which is off. The lean trail champion's
            // resting switch reads exactly 0, which only this writes
            // (`mind::FIRST_LINK`). They mutate as any slot does.
            if prune > 0.0 && c[i] != 0.0 && rng.unit() < prune && (!is_mind(g.len()) || i >= mind::FIRST_LINK) {
                c[i] = 0.0;
            } else if grow < 1.0 && c[i] == 0.0 && rng.unit() >= grow {
                // A connection not made: the slot stays empty.
            } else {
                c[i] = (c[i] + sigma * rng.step()).clamp(-40.0, 40.0);
            }
        }
    }
    c
}

/// How often each walk gene mutates in a child: about one of the twelve.
pub const WALK_PM: f32 = 0.08;
/// A walk gene's mutation step, standard deviation, on its ±1 axis.
pub const WALK_SIGMA: f32 = 0.2;

/// Uniform crossover **by output row**: a child takes each output's whole
/// incoming row (its input weights and its hidden weights) from one parent
/// or the other. Whole rows, because a weight means something only beside
/// the others feeding the same output; a slot-by-slot cross breaks working
/// circuits far more often than it joins two.
pub fn cross(a: &[f32], b: &[f32], rng: &mut Rng) -> Vec<f32> {
    if is_mind(a.len()) {
        return cross_mind(a, b, rng);
    }
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

/// A mind's crossover: each of `mind::crossover_group`'s groups (a scorer
/// unit's whole fan-in, a job row, a memory row with its rate) whole from
/// one parent, drawn in slot order so a run is a function of its seed.
fn cross_mind(a: &[f32], b: &[f32], rng: &mut Rng) -> Vec<f32> {
    let mut take_b: std::collections::BTreeMap<usize, bool> = std::collections::BTreeMap::new();
    let mut c = a.to_vec();
    for i in 0..a.len() {
        let g = mind::crossover_group(i);
        if *take_b.entry(g).or_insert_with(|| rng.unit() < 0.5) {
            c[i] = b[i];
        }
    }
    c
}

/// **What a genome's ants pay brain tax on**: a mind's live weights
/// (`mind::MindGenome::live`), today's brain's non-zero weights (as
/// [`describe`] counts them). What [`Settings::wire`] charges for.
pub fn connections(g: &[f32]) -> usize {
    if is_mind(g.len()) {
        mind::MindGenome::new(g.to_vec()).live as usize
    } else {
        g.iter().take(BRAIN_END).filter(|&&w| w != 0.0).count()
    }
}

/// A fit less its [`Settings::wire`] share, as selection ranks it.
pub fn wired(fit: f64, links: usize, wire: f64) -> f64 {
    fit - fit.abs() * (1.0 - (-wire * links as f64).exp())
}

/// A short readout of what a genome attends to: its non-zero weight count,
/// the five inputs with the most total weight, and its walk genes off the
/// shipped walk.
pub fn describe(g: &[f32]) -> String {
    if is_mind(g.len()) {
        return mind::describe(g);
    }
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
    let mut s = format!("{live} weights; strongest inputs: {}", names.join(", "));
    if let Some(w) = blocks(g.len()).walk {
        let off: Vec<String> = (0..WALK).filter(|&k| g[w + k] != 0.0).map(|k| format!("{} {:+.2}", WALK_NAMES[k], g[w + k])).collect();
        s.push_str(&format!("; walk genes: {}", if off.is_empty() { "shipped".to_string() } else { off.join(", ") }));
    }
    s
}

/// Build the start genome for a challenge.
///
/// `scorer`, for an arena with a target scorer, is the scorer the brain
/// starts beside; it is appended to the brain, and a genome file written
/// for the brain alone takes it too. An empty `scorer` means **the start
/// file's own** (`targets=own`), and a start file that carries a scorer
/// takes nothing else: the first version kept the file's scorer whatever
/// `targets=` said, while the settings line printed what was asked for
/// (ladder lane, `stage1-code-review.md` 8, the trap of review-rev2 N10).
///
/// `walk` gives the genome walk genes ([`WALK`]): the start file's own if it
/// carries them, else the shipped walk. A start file that carries walk genes
/// a run did not ask for stops it, for the same reason.
pub fn start_genome(start: &Start, species_genome: &[f32], banned: &[BrainOutput], seed: u64, scorer: Option<&[f32]>, walk: bool) -> Vec<f32> {
    let file = match start {
        Start::File(p) => {
            let g = load_genome(p, genome_file_len(p));
            let b = blocks(g.len());
            Some((p, g, b))
        }
        _ => None,
    };
    let mut g = start_brain(start, species_genome, banned, seed);
    match (scorer, &file) {
        (None, Some((p, _, b))) => assert!(b.scorer.is_none(), "{} carries a target scorer, and this arena has none", p.display()),
        (None, None) => {}
        (Some(sc), Some((p, f, Blocks { scorer: Some(at), .. }))) => {
            assert!(sc.is_empty(), "{} carries its own target scorer, which targets= would not replace: pass targets=own", p.display());
            g.extend_from_slice(&f[*at..*at + targets::WEIGHTS]);
        }
        (Some(sc), _) => {
            assert!(!sc.is_empty(), "targets=own needs a start file that carries a target scorer");
            g.extend_from_slice(sc);
        }
    }
    match &file {
        Some((_, f, Blocks { walk: Some(at), .. })) if walk => g.extend_from_slice(&f[*at..*at + WALK]),
        Some((p, _, Blocks { walk: Some(_), .. })) => panic!("{} carries walk genes, which this run would drop: pass genes=walk", p.display()),
        _ if walk => g.extend_from_slice(&[0.0; WALK]),
        _ => {}
    }
    g
}

/// The brain alone, [`SCORER`] slots, bans applied.
fn start_brain(start: &Start, species_genome: &[f32], banned: &[BrainOutput], seed: u64) -> Vec<f32> {
    let mut g = match start {
        Start::Species => species_genome.to_vec(),
        Start::File(p) => {
            let mut g = load_genome(p, genome_file_len(p));
            g.truncate(SCORER);
            g
        }
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
/// * `progress.csv` -- one row per generation: best, median and control fit
///   (the arena's own, before [`Settings::wire`]), the best genome's parts,
///   and its [`connections`];
/// * `holdout.csv` -- champion, start and blinded champion on the hold-out
///   layouts, every `every` generations;
/// * `champions/gen-NNN.txt` -- each generation's top genome;
/// * `best.txt` -- the last champion; `pic-gen-NNN.png` -- its picture;
/// * `hold-best.txt` -- the champion that scored highest on the hold-out,
///   which a flat stop can leave well above the last (review of PR 717,
///   N16): the one to start a next stage from;
/// * `curves.svg` -- the learning curves, redrawn at every hold-out;
/// * `state.txt` -- what [`Settings::resume`] carries on from
///   ([`RunState`]), overwritten every generation;
/// * `summary.txt` -- what the run was and how it ended.
pub fn train(ch: &dyn Challenge, start: Vec<f32>, set: &Settings, out: &Path) -> std::io::Result<Vec<f32>> {
    std::fs::create_dir_all(out.join("champions"))?;
    let slots = trainable_for(ch, start.len());
    assert!(blocks(start.len()).walk.is_none() || ch.unit() == Unit::Colony, "walk genes need a colony arena: the scout's reserve is the world's");
    let mut rng = Rng(set.seed.wrapping_mul(0xA24B_AED4_963E_E407) ^ 0x7EA1);
    let parts = ch.parts();
    let resumed = set.resume.then(|| RunState::load(&out.join(STATE_FILE), ch, set, &start));
    let first = resumed.as_ref().map_or(0, |r| r.next);
    let mut progress = rows_before(&out.join("progress.csv"), &format!("gen,best,median,control,secs,{},links", parts.join(",")), first)?;
    let mut holdout = rows_before(&out.join("holdout.csv"), &format!("gen,who,fit,{}", parts.join(",")), first)?;
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

    let elites = (set.pop / 4).max(1);
    let t0 = Instant::now();
    let mut champion = start.clone();
    let mut last_hold = String::new();
    // The hold-out best so far and its generation, for [`flat`], and the
    // highest hold-out champion, which a creep under [`STOP_GAIN`] can lift
    // past the best [`flat`] counts.
    let (mut best_hold, mut best_gen) = (f64::NEG_INFINITY, 0);
    let (mut top, mut top_gen, mut top_genome) = (f64::NEG_INFINITY, 0, start.clone());
    let mut ran = set.gens;
    // **How often each floor vetoes** (ladder lane, `dig-rung-design.md`
    // rev 2.4): a suite's `_under` parts, 1 where a kept arena is under its
    // floor. Counted over the population each generation and over the
    // hold-out champions, since a gate that vetoes often is a noise source
    // of its own.
    let gates: Vec<(usize, &str)> =
        parts.iter().enumerate().filter_map(|(i, p)| p.strip_suffix("_under").map(|n| (i, n))).collect();
    let mut vetoed_pop = vec![0usize; gates.len()];
    let mut vetoed_hold = vec![0usize; gates.len()];
    let (mut scored_pop, mut scored_hold) = (0usize, 0usize);
    // A veto is x0.1, not exclusion, so hold-best.txt can be a vetoed
    // genome; the summary says which floors it was under (review of PR 723,
    // N3). Print-only: nothing reads it back, so it has no guard (review of
    // PR 724, N4).
    let mut top_under: Vec<&str> = Vec::new();
    let mut secs_before = 0.0;
    let mut start_hold: Option<Score> = None;
    let mut popn: Vec<Vec<f32>> = match resumed {
        Some(r) => {
            println!("resumed at generation {} from {}", r.next, out.join(STATE_FILE).display());
            rng = Rng(r.rng);
            (best_hold, best_gen, top, top_gen, top_genome) = (r.best_hold, r.best_gen, r.top, r.top_gen, r.top_genome);
            (champion, last_hold, secs_before) = (r.champion, r.last_hold, r.secs);
            (scored_pop, scored_hold) = (r.scored_pop, r.scored_hold);
            if r.vetoed_pop.len() == gates.len() && r.vetoed_hold.len() == gates.len() {
                (vetoed_pop, vetoed_hold) = (r.vetoed_pop, r.vetoed_hold);
            }
            top_under = gates.iter().filter(|g| r.top_under.iter().any(|n| n == g.1)).map(|g| g.1).collect();
            breed(&r.parents, set, &slots, &mut rng)
        }
        None => (0..set.pop)
            .map(|k| if k == 0 { start.clone() } else { mutate(&start, &slots, set.pm, set.sigma, set.prune, set.grow, &mut rng) })
            .collect(),
    };
    println!("gen   best  median  control  [{}]  secs", parts.join(" "));
    for gen in first..set.gens {
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
        let links: Vec<usize> = popn.iter().map(|g| connections(g)).collect();
        let mut ranked: Vec<(f64, usize)> = (0..set.pop).map(|g| (wired(means[g].fit, links[g], set.wire), g)).collect();
        ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then(a.1.cmp(&b.1)));
        let b = ranked[0].1;
        champion = popn[b].clone();
        let secs = tg.elapsed().as_secs_f64();
        let bp: Vec<String> = means[b].parts.iter().map(|p| format!("{p:.2}")).collect();
        let (best, median) = (means[b].fit, means[ranked[set.pop / 2].1].fit);
        writeln!(progress, "{gen},{best:.3},{median:.3},{control:.3},{secs:.1},{},{}", bp.join(","), links[b])?;
        println!("{gen:3} {best:7.2} {median:7.2} {control:7.2}  [{}]  {secs:.0}  links {}", bp.join(" "), links[b]);
        if !gates.is_empty() {
            let shares: Vec<String> = gates
                .iter()
                .zip(vetoed_pop.iter_mut())
                .map(|(&(i, name), n)| {
                    let k = means[..set.pop].iter().filter(|m| m.parts[i] > 0.0).count();
                    *n += k;
                    format!("{name} {:.0}%", 100.0 * k as f64 / set.pop as f64)
                })
                .collect();
            scored_pop += set.pop;
            println!("    vetoed by a floor: {}", shares.join(", "));
        }
        save_genome(&out.join(format!("champions/gen-{gen:03}.txt")), &champion)?;

        let mut stop = false;
        if (gen + 1) % set.every.max(1) == 0 || gen + 1 == set.gens {
            // The rows in one pool (`score_rows`), and `start` scored once:
            // the same genome on the same hold-out layouts every generation
            // is the same score every generation, and re-running it was a
            // third of the hold-out's time (cloud lane, 2026-10-19). A
            // resume scores it again, once.
            let rows = controls(ch, &champion, &start, &hold);
            let todo: Vec<(&[f32], &[Layout])> =
                rows.iter().filter(|r| !(r.0 == "start" && start_hold.is_some())).map(|r| (r.1.as_slice(), r.2.as_slice())).collect();
            let mut fresh = score_rows(ch, &todo, set.threads).into_iter();
            for &(who, _, _) in &rows {
                let s = match &start_hold {
                    Some(s) if who == "start" => s.clone(),
                    _ => fresh.next().expect("a score for every row not reused"),
                };
                if who == "start" {
                    start_hold = Some(s.clone());
                }
                let who = if who == "genome" { "champion" } else { who };
                let p: Vec<String> = s.parts.iter().map(|p| format!("{p:.2}")).collect();
                writeln!(holdout, "{gen},{who},{:.3},{}", s.fit, p.join(","))?;
                println!("    HOLDOUT {who:8} {:7.2}  [{}]", s.fit, p.join(" "));
                if who == "champion" {
                    scored_hold += 1;
                    for (&(i, _), n) in gates.iter().zip(vetoed_hold.iter_mut()) {
                        *n += usize::from(s.parts[i] > 0.0);
                    }
                    last_hold = format!("hold-out champion {:.2}", s.fit);
                    if new_best(s.fit, best_hold) {
                        (best_hold, best_gen) = (s.fit, gen);
                    }
                    if s.fit > top {
                        (top, top_gen, top_genome) = (s.fit, gen, champion.clone());
                        top_under = gates
                            .iter()
                            .filter(|&&(i, _)| s.parts[i] > 0.0)
                            .map(|&(_, n)| n)
                            .collect();
                    }
                } else {
                    last_hold.push_str(&format!(", {who} {:.2}", s.fit));
                }
            }
            println!("    {}", describe(&champion));
            stop = flat(gen, best_gen, set.patience);
            if set.pictures && ((gen + 1) % PICTURE_EVERY == 0 || gen + 1 == set.gens || stop) {
                picture(ch, &champion, hold[0], &out.join(format!("pic-gen-{gen:03}.png")));
            }
            curves(out);
        }

        let parents: Vec<Vec<f32>> = ranked[..elites].iter().map(|&(_, g)| popn[g].clone()).collect();
        // Saved before the children are drawn, with the stream where they
        // will be drawn from: a resume breeds these parents from this
        // stream, so its generation `gen + 1` is this run's.
        let state = RunState {
            next: gen + 1,
            rng: rng.0,
            start: fingerprint(&start),
            arena: RunState::arena(ch, set),
            parts: parts.join(","),
            best_hold,
            best_gen,
            top,
            top_gen,
            top_under: top_under.iter().map(|n| n.to_string()).collect(),
            scored_pop,
            scored_hold,
            vetoed_pop: vetoed_pop.clone(),
            vetoed_hold: vetoed_hold.clone(),
            secs: secs_before + t0.elapsed().as_secs_f64(),
            last_hold: last_hold.clone(),
            settings: RunState::settings(set),
            champion: champion.clone(),
            top_genome: top_genome.clone(),
            parents,
        };
        state.save(&out.join(STATE_FILE))?;
        if stop {
            println!("STOPPED flat: no hold-out best {STOP_GAIN} above the last since generation {best_gen} ({best_hold:.3}), {} generations, patience {}", gen + 1, set.patience);
            ran = gen + 1;
            break;
        }
        popn = breed(&state.parents, set, &slots, &mut rng);
    }
    save_genome(&out.join("best.txt"), &champion)?;
    save_genome(&out.join("hold-best.txt"), &top_genome)?;
    let vetoes = if gates.is_empty() {
        String::new()
    } else {
        let each: Vec<String> = gates
            .iter()
            .enumerate()
            .map(|(k, &(_, name))| {
                format!(
                    "{name} {} of {scored_hold} hold-out champions, {:.1}% of the population",
                    vetoed_hold[k],
                    100.0 * vetoed_pop[k] as f64 / scored_pop.max(1) as f64
                )
            })
            .collect();
        let hb = if top_under.is_empty() {
            "under no floor".to_string()
        } else {
            format!("under {}", top_under.join(", "))
        };
        format!("vetoed by a floor: {}\nhold-best.txt: {hb}\n", each.join("; "))
    };
    let summary = format!(
        "{head}\n{ran} of {} generations in {:.0} s (hold-out best {top:.3} at generation {top_gen}, hold-best.txt; last gain over {STOP_GAIN} at generation {best_gen})\n{last_hold}\n{vetoes}{}\n",
        set.gens,
        secs_before + t0.elapsed().as_secs_f64(),
        describe(&champion)
    );
    std::fs::write(out.join("summary.txt"), &summary)?;
    println!("{summary}");
    Ok(champion)
}

/// **The next generation from its parents**: the parents themselves, then
/// children -- each a parent, or a uniform cross of two with probability
/// `cross`, mutated -- until the population is full.
fn breed(parents: &[Vec<f32>], set: &Settings, slots: &[usize], rng: &mut Rng) -> Vec<Vec<f32>> {
    let elites = parents.len();
    let mut popn = parents.to_vec();
    while popn.len() < set.pop {
        let a = &parents[rng.below(elites)];
        let child = if elites > 1 && rng.unit() < set.cross {
            let b = &parents[rng.below(elites)];
            cross(a, b, rng)
        } else {
            a.clone()
        };
        popn.push(mutate(&child, slots, set.pm, set.sigma, set.prune, set.grow, rng));
    }
    popn
}

/// Where [`train`] keeps its [`RunState`], in the run's directory.
pub const STATE_FILE: &str = "state.txt";

/// **A run between two generations** ([`Settings::resume`]): everything
/// [`train`] carries from one generation to the next. One file a run,
/// overwritten every generation by a rename, so a run killed mid-write
/// keeps the last whole one, and a run adds one file to the shared
/// folder's cap, not one a generation.
///
/// Every number is written as its bits: a resumed run is the run that
/// never stopped only if the weights, the scores [`flat`] compares and the
/// random stream come back to the last bit, and a decimal print can drop
/// one (a weight of -0.0 among them, which [`save_genome`] leaves out).
#[derive(Debug, PartialEq)]
pub struct RunState {
    /// The generation a resume runs first.
    pub next: usize,
    /// The random stream where the children of `parents` are drawn from.
    pub rng: u64,
    /// The start genome's [`fingerprint`]: the control, and what a suite's
    /// references were taken from, so a state saved from another start is
    /// another run.
    pub start: u64,
    /// The arena ([`RunState::arena`]) and its parts, the columns of
    /// `progress.csv` and `holdout.csv`: a resume must share both.
    pub arena: String,
    pub parts: String,
    pub best_hold: f64,
    pub best_gen: usize,
    pub top: f64,
    pub top_gen: usize,
    /// The floors the hold-out top was under, for the summary. It, the
    /// veto counts and `last_hold` feed only `summary.txt`, and the test
    /// arena has no floors, so nothing guards their restore (review of
    /// PR 726, N3).
    pub top_under: Vec<String>,
    pub scored_pop: usize,
    pub scored_hold: usize,
    pub vetoed_pop: Vec<usize>,
    pub vetoed_hold: Vec<usize>,
    /// Seconds run so far, for the summary.
    pub secs: f64,
    pub last_hold: String,
    /// The settings a resume must share to be this run ([`RunState::settings`]).
    pub settings: String,
    pub champion: Vec<f32>,
    pub top_genome: Vec<f32>,
    pub parents: Vec<Vec<f32>>,
}

impl RunState {
    /// **Which arena a run is on**: the challenge's own name, ants, frames
    /// and banned outputs, then [`Settings::arena`]. Its format is part of
    /// every saved state: a change here refuses every state saved before it,
    /// so the hold-out's size is checked from the settings line instead
    /// ([`RunState::load`]).
    fn arena(ch: &dyn Challenge, set: &Settings) -> String {
        format!("challenge={} unit={:?} ants={} frames={} banned={:?} {}", ch.name(), ch.unit(), ch.ants(), ch.frames(), ch.banned(), set.arena)
            .trim_end()
            .to_string()
    }

    /// The settings that make a run this run: all but `gens`, `patience`,
    /// `every`, `threads`, `pictures` and `resume`, which a resume may
    /// change without drawing a different population.
    fn settings(set: &Settings) -> String {
        format!(
            "pop={} layouts={} pm={} sigma={} cross={} holdout={} seed={} prune={} wire={} grow={}",
            set.pop, set.layouts, set.pm, set.sigma, set.cross, set.holdout, set.seed, set.prune, set.wire, set.grow
        )
    }

    fn save(&self, path: &Path) -> std::io::Result<()> {
        let list = |v: &[usize]| v.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(" ");
        let mut s = format!(
            "run-state 2\nnext {}\nrng {:x}\nstart {:x}\narena {}\nparts {}\nbest_hold {:x} {}\ntop {:x} {}\ntop_under {}\nscored {} {}\nvetoed_pop {}\nvetoed_hold {}\nsecs {:x}\nlast_hold {}\nsettings {}\n",
            self.next,
            self.rng,
            self.start,
            self.arena,
            self.parts,
            self.best_hold.to_bits(),
            self.best_gen,
            self.top.to_bits(),
            self.top_gen,
            self.top_under.join(" "),
            self.scored_pop,
            self.scored_hold,
            list(&self.vetoed_pop),
            list(&self.vetoed_hold),
            self.secs.to_bits(),
            self.last_hold,
            self.settings,
        );
        let mut genome = |name: &str, g: &[f32]| {
            s.push_str(&format!("genome {name} {}\n", g.len()));
            for (i, w) in g.iter().enumerate() {
                if w.to_bits() != 0 {
                    s.push_str(&format!("{i} {:x}\n", w.to_bits()));
                }
            }
        };
        genome("champion", &self.champion);
        genome("top", &self.top_genome);
        for p in &self.parents {
            genome("parent", p);
        }
        // **The end mark** (review of PR 726, S2): without it any prefix
        // with one parent line in it loaded, so a copy cut short in a tar
        // or a sync resumed quietly with a parent zeroed or missing.
        s.push_str(&format!("end {}\n", self.parents.len()));
        let tmp = path.with_extension("tmp");
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(s.as_bytes())?;
        // On the disk before the rename makes it the state.
        f.sync_all()?;
        std::fs::rename(&tmp, path)
    }

    /// Read the state a run of `set` on `ch` from `start` saved. Panics on
    /// a state from another start, another arena, with other parts or of
    /// another genome length, which is another run, and on one with no end
    /// mark, which was cut short; a setting that differs only warns, since
    /// the run can go on, but not as the one that never stopped. A state
    /// from before the arena and the end mark were kept (`run-state 1`) is
    /// refused too: only PR 726's first head wrote one, never merged, and
    /// every such soil state is rev 2.6's linear pay with 24 parts, so it
    /// resumed with exit 0 into ragged rows and a best from the old pay
    /// (review of PR 726, N8).
    fn load(path: &Path, ch: &dyn Challenge, set: &Settings, start: &[f32]) -> RunState {
        let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("resume: {}: {e}", path.display()));
        let mut lines = text.lines();
        match lines.next() {
            Some("run-state 2") => {}
            Some("run-state 1") => panic!("resume: {} predates the arena and end checks and rev 2.8's pay: start the run afresh", path.display()),
            _ => panic!("resume: {} is not a run state", path.display()),
        }
        let mut field = |key: &str| -> String {
            let l = lines.next().unwrap_or_else(|| panic!("resume: no {key} line"));
            let rest = l.strip_prefix(key).unwrap_or_else(|| panic!("resume: {key} expected, found {l}"));
            rest.strip_prefix(' ').unwrap_or(rest).to_string()
        };
        let hex = |t: &str| u64::from_str_radix(t, 16).unwrap_or_else(|_| panic!("resume: {t} is not hex"));
        let pair = |t: String| -> (u64, usize) {
            let (a, b) = t.split_once(' ').expect("resume: bits and a generation");
            (hex(a), b.parse().unwrap())
        };
        let list = |t: String| -> Vec<usize> { t.split_whitespace().map(|n| n.parse().unwrap()).collect() };
        let next = field("next").parse().unwrap();
        let rng = hex(&field("rng"));
        let fp = hex(&field("start"));
        let (arena, parts) = (field("arena"), field("parts"));
        let (best_hold, best_gen) = pair(field("best_hold"));
        let (top, top_gen) = pair(field("top"));
        let top_under = field("top_under").split_whitespace().map(str::to_string).collect();
        let scored = list(field("scored"));
        let (vetoed_pop, vetoed_hold) = (list(field("vetoed_pop")), list(field("vetoed_hold")));
        let secs = f64::from_bits(hex(&field("secs")));
        let last_hold = field("last_hold");
        let settings = field("settings");
        assert_eq!(fp, fingerprint(start), "resume: the state in {} was saved from another start", path.display());
        assert_eq!(arena, RunState::arena(ch, set), "resume: the state in {} was saved on another arena", path.display());
        assert_eq!(parts, ch.parts().join(","), "resume: the state in {} was saved with other parts", path.display());
        // The hold-out's size alone sets [`holdout_layouts`], so another
        // `holdout=` is another set for the saved best and [`flat`]'s clock
        // to be compared against: S1's fault, refused rather than warned
        // (review of PR 726, N10). `seed=` does not move that set.
        let saved_holdout = settings.split_whitespace().find_map(|w| w.strip_prefix("holdout="));
        assert_eq!(saved_holdout, Some(set.holdout.to_string().as_str()), "resume: the state in {} was saved with another hold-out", path.display());
        if settings != RunState::settings(set) {
            println!("resume: WARNING the saved run had {settings}: this run goes on, but not as the run that never stopped");
        }
        let mut genomes: Vec<(String, Vec<f32>)> = vec![];
        let mut end = None;
        for l in lines {
            assert!(end.is_none(), "resume: {l} after the end mark in {}", path.display());
            if let Some(n) = l.strip_prefix("end ") {
                end = Some(n.parse::<usize>().unwrap_or_else(|_| panic!("resume: end mark {l}")));
            } else if let Some(head) = l.strip_prefix("genome ") {
                let (name, len) = head.split_once(' ').expect("resume: a genome's name and length");
                let len: usize = len.parse().unwrap();
                assert_eq!(len, start.len(), "resume: a {len}-slot genome in a run of {}-slot genomes", start.len());
                genomes.push((name.to_string(), vec![0.0; len]));
            } else {
                let (i, w) = l.split_once(' ').expect("resume: a slot and its bits");
                let g = &mut genomes.last_mut().expect("resume: a slot before any genome").1;
                g[i.parse::<usize>().unwrap()] = f32::from_bits(u32::from_str_radix(w, 16).unwrap());
            }
        }
        let mut take = |name: &str| {
            let k = genomes.iter().position(|(n, _)| n == name).unwrap_or_else(|| panic!("resume: no {name} genome"));
            genomes.remove(k).1
        };
        let (champion, top_genome) = (take("champion"), take("top"));
        let parents: Vec<Vec<f32>> = genomes
            .into_iter()
            .map(|(n, g)| {
                assert_eq!(n, "parent", "resume: an unknown genome {n}");
                g
            })
            .collect();
        assert!(!parents.is_empty(), "resume: no parents in {}", path.display());
        assert_eq!(end, Some(parents.len()), "resume: {} was cut short ({} parents read)", path.display(), parents.len());
        RunState {
            next,
            rng,
            start: fp,
            arena,
            parts,
            best_hold: f64::from_bits(best_hold),
            best_gen,
            top: f64::from_bits(top),
            top_gen,
            top_under,
            scored_pop: scored[0],
            scored_hold: scored[1],
            vetoed_pop,
            vetoed_hold,
            secs,
            last_hold,
            settings,
            champion,
            top_genome,
            parents,
        }
    }
}

/// FNV-1a over a genome's bits: which start a [`RunState`] belongs to.
pub fn fingerprint(g: &[f32]) -> u64 {
    g.iter().flat_map(|w| w.to_bits().to_le_bytes()).fold(0xCBF2_9CE4_8422_2325, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01B3))
}

/// **A run file opened for writing from generation `first`**: created with
/// its `head` line on a fresh run (`first` 0), and on a resume rewritten
/// with only the rows of generations before `first`. A run killed partway
/// through a generation wrote some of that generation's rows before its
/// state was saved; the resume runs that generation again, so its rows are
/// written once.
fn rows_before(path: &Path, head: &str, first: usize) -> std::io::Result<std::fs::File> {
    let mut kept = format!("{head}\n");
    if first > 0 {
        let old = std::fs::read_to_string(path)?;
        for l in old.lines().skip(1) {
            if l.split(',').next().and_then(|g| g.parse::<usize>().ok()).is_some_and(|g| g < first) {
                kept.push_str(l);
                kept.push('\n');
            }
        }
    }
    std::fs::write(path, kept)?;
    std::fs::OpenOptions::new().append(true).open(path)
}

/// A picture of the champion every this many generations, and after the
/// last: kept at the old hold-out cadence when the hold-out went to every
/// generation, so a run does not write forty pictures.
const PICTURE_EVERY: usize = 5;

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
/// magenta so a colony of one-cell animals is visible at all, and the
/// arena's [`Challenge::marks`] painted under the ants. An arena that looks
/// again after its bell (dig, after the settle it is scored on) gets a sixth
/// snapshot of what was scored.
pub fn picture(ch: &dyn Challenge, g: &[f32], layout: Layout, path: &Path) {
    let frames = ch.frames();
    let shots: Vec<u64> = (1..=5).map(|i| i * frames / 5).collect();
    let mut sheet: Vec<u8> = Vec::new();
    let mut size = (0u32, 0u32);
    let mut renderer = crate::render::Renderer::new();
    let particles = crate::sim::particle::ParticleSystem::new();
    let gs: Vec<&[f32]> = vec![g; if ch.unit() == Unit::Ant { ch.ants() } else { 1 }];
    ch.run(layout, &gs, &mut |f, w| {
        if !shots.contains(&f) && f <= frames {
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
        for ((x, y), [r, g, b]) in ch.marks(layout, w) {
            if x >= 0 && y >= 0 && (x as u32) < vw && (y as u32) < vh {
                let i = ((y as u32 * vw + x as u32) * 4) as usize;
                buf[i..i + 4].copy_from_slice(&[r, g, b, 255]);
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

/// **The numbers beside [`picture`]**: one genome on each layout, a line
/// every `every` frames -- the colony's heads, energy and food, and what the
/// minds chose if it has them. A picture says where the ants went; this says
/// whether they ate, and when they started to starve.
pub fn trace(ch: &dyn Challenge, g: &[f32], layouts: &[Layout], every: u64) {
    for &layout in layouts {
        println!("layout seed={} k={}", layout.seed, layout.k);
        trace_one(ch, g, layout, every);
    }
}

fn trace_one(ch: &dyn Challenge, g: &[f32], layout: Layout, every: u64) {
    let gs: Vec<&[f32]> = vec![g; if ch.unit() == Unit::Ant { ch.ants() } else { 1 }];
    let s = ch.run(layout, &gs, &mut |f, w| {
        if !f.is_multiple_of(every) {
            return;
        }
        let food = w.materials.id_of("provisions");
        let mut lying = 0usize;
        if let (Some(food), Some(b)) = (food, w.bounds()) {
            for y in 0..=b.max_y {
                for x in 0..=b.max_x {
                    lying += usize::from(w.get(x, y).material == food);
                }
            }
        }
        let ant = w.species.id_of("ant");
        let (mut adults, mut brood, mut energy, mut low, mut crop) = (0usize, 0usize, 0.0f64, f64::MAX, 0.0f64);
        for id in w.live_organism_ids() {
            let Some(st) = w.organism(id) else { continue };
            if Some(st.species) != ant {
                continue;
            }
            if st.brood.is_some() {
                brood += 1;
                continue;
            }
            adults += 1;
            energy += st.energy as f64;
            low = low.min(st.energy as f64);
            crop += st.crop.map_or(0.0, |c| c.worth() as f64);
        }
        let c = &w.creature_stats;
        let mean = if adults > 0 { energy / adults as f64 } else { 0.0 };
        let low = if adults > 0 { low } else { 0.0 };
        print!(
            "f={f} adults={adults} brood={brood} energy={mean:.0} low={low:.0} crop={crop:.0} lying={lying} pickups={} eats={} deliveries={} births={} eggs={} held_ticks={}",
            c.pickups, c.eats, c.deliveries, c.births, c.eggs_laid, c.food_brake_held
        );
        if let Some(m) = &w.minds {
            let k = &m.counts;
            print!(
                " | mind decisions={} chose {:?} picked_up={} ate={} put_down={} delivered={} fed_adult={} fed_larva_crop={} fed_larva_bank={} starved_soil={} stuck={} failed={}",
                k.decisions,
                k.chose,
                k.swallowed,
                k.ate,
                k.put_down,
                k.delivered,
                k.fed_adult,
                k.fed_larva_crop,
                k.fed_larva_bank,
                k.starved_soil,
                k.starved_soil_food,
                k.failed
            );
        }
        println!();
    });
    println!("score {:?}", mean_score(&s));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A run stops flat only well past its last real gain** ([`flat`]):
    /// never before [`STOP_MIN`], never while the flat stretch is shorter
    /// than the climb, never under 2% gains, and `patience=0` never. The
    /// staircase is today's brain on trail, seed 2 (`stage2/trail-lean`):
    /// flat 29 generations, then 4% up at generation 38.
    #[test]
    fn a_run_stops_flat_only_well_past_its_last_gain() {
        let stops = |curve: &[f64], patience: usize| {
            let (mut best, mut at) = (f64::NEG_INFINITY, 0);
            curve.iter().enumerate().find_map(|(g, &f)| {
                if new_best(f, best) {
                    (best, at) = (f, g);
                }
                flat(g, at, patience).then_some(g)
            })
        };
        let mut stair: Vec<f64> = (0..60).map(|g| if g < 9 { 0.1 * g as f64 } else { 0.836 }).collect();
        stair[38..].iter_mut().for_each(|f| *f = 0.873);
        assert_eq!(stops(&stair, 30), None, "flat from 38 to 59 is 21 generations: still running");
        let mut long = stair.clone();
        long.resize(100, 0.873);
        assert_eq!(stops(&long, 30), Some(77), "a best at 38 (the 39th run) runs to 77 (flat 39 more)");
        let early: Vec<f64> = (0..100).map(|g| if g < 3 { g as f64 } else { 2.0 }).collect();
        assert_eq!(stops(&early, 30), Some(39), "never before generation 40");
        let creep: Vec<f64> = (0..100).map(|g| 1.0 + 0.0002 * g as f64).collect();
        assert_eq!(stops(&creep, 30), Some(39), "steps under 2% are not gains");
        let slow: Vec<f64> = (0..100).map(|g| 1.0 + 0.001 * g as f64).collect();
        assert_eq!(stops(&slow, 30), None, "2% every 21 generations is still climbing");
        assert_eq!(stops(&long, 0), None, "patience 0 runs every generation");
        assert!(new_best(-0.01, -0.019) && !new_best(1.01, 1.0) && new_best(0.0, f64::NEG_INFINITY));
    }

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

    /// A mind arena scored by the genome alone, so a test can run whole
    /// generations in no time. On a training layout, the sum of its
    /// weights, so half of all steps gain, plus a share that depends on the
    /// layout, so each generation's draw counts; on a hold-out layout, how
    /// near that sum is to `aim`, so the hold-out climbs while training
    /// passes `aim` and falls after, and its best stays where it was found.
    /// Run it at `wire` 0: here a link's cost is a share of a score that is
    /// mostly the layout's.
    struct Bowl {
        aim: f64,
    }
    impl Challenge for Bowl {
        fn name(&self) -> &'static str {
            "bowl"
        }
        fn unit(&self) -> Unit {
            Unit::Colony
        }
        fn ants(&self) -> usize {
            1
        }
        fn parts(&self) -> &'static [&'static str] {
            &["seed"]
        }
        fn banned(&self) -> &'static [BrainOutput] {
            &[]
        }
        fn minds(&self) -> bool {
            true
        }
        fn verbs(&self) -> &'static [Verb] {
            &[Verb::Walk, Verb::Rest]
        }
        fn run(&self, l: Layout, g: &[&[f32]], _: &mut dyn FnMut(u64, &World)) -> Vec<Score> {
            let sum: f64 = g[0].iter().map(|&w| f64::from(w)).sum();
            // Hold-out layouts are numbered under 100,000 (`training_layouts`).
            let fit = if l.seed < 100_000 { -(sum - self.aim).abs() } else { (l.seed % 7) as f64 + sum };
            vec![Score { fit, parts: vec![l.seed as f64] }]
        }
        fn frames(&self) -> u64 {
            1
        }
    }

    /// **A resumed run is the run that never stopped** ([`Settings::resume`],
    /// owner 2026-10-19): six generations straight, and three then three
    /// more resumed from `state.txt`, write the same rows, the same
    /// champions and the same state to the bit, but for the clock. The
    /// first leg also leaves a row of generation 3 behind, as a run killed
    /// partway through it would, and the resume writes that generation once.
    #[test]
    fn a_resumed_run_is_the_run_that_never_stopped() {
        let dir = std::env::temp_dir().join(format!("train-resume-{}", std::process::id()));
        let (straight, split) = (dir.join("straight"), dir.join("split"));
        let start = mind::blank(&[Verb::Walk, Verb::Rest]);
        let bowl = Bowl { aim: start.iter().map(|&w| f64::from(w)).sum::<f64>() + 8.0 };
        let set = Settings { gens: 6, pop: 8, layouts: 2, holdout: 2, seed: 5, threads: 2, pictures: false, patience: 0, wire: 0.0, ..Settings::default() };
        train(&bowl, start.clone(), &set, &straight).unwrap();
        train(&bowl, start.clone(), &Settings { gens: 3, ..set.clone() }, &split).unwrap();
        for (f, row) in [("progress.csv", "3,9,9,9,9\n"), ("holdout.csv", "3,champion,9\n")] {
            let mut file = std::fs::OpenOptions::new().append(true).open(split.join(f)).unwrap();
            file.write_all(row.as_bytes()).unwrap();
        }
        train(&bowl, start.clone(), &Settings { resume: true, ..set.clone() }, &split).unwrap();
        let read = |d: &Path, f: &str| std::fs::read_to_string(d.join(f)).unwrap();
        // `secs`, progress's fifth column, is the clock's.
        let untimed = |t: String| {
            let rows = t.lines().map(|l| l.split(',').enumerate().filter(|&(i, _)| i != 4).map(|(_, c)| c).collect::<Vec<_>>().join(","));
            rows.collect::<Vec<_>>().join("\n")
        };
        assert_eq!(untimed(read(&straight, "progress.csv")), untimed(read(&split, "progress.csv")));
        assert_eq!(read(&split, "progress.csv").lines().count(), 7, "a header and six generations, the killed one once");
        for f in ["holdout.csv", "best.txt", "hold-best.txt", "champions/gen-005.txt"] {
            assert_eq!(read(&straight, f), read(&split, f), "{f}");
        }
        let state = |d: &Path| RunState { secs: 0.0, ..RunState::load(&d.join(STATE_FILE), &bowl, &set, &start) };
        let (a, b) = (state(&straight), state(&split));
        assert_eq!(a, b);
        // ...and not because nothing moved, and with the hold-out best in
        // the first leg, so the resume had to carry it.
        assert_eq!((a.next, a.parents.len()), (6, 2));
        assert!(a.top_gen < 3 && a.best_gen < 3, "hold-out best at {} and {}", a.top_gen, a.best_gen);
        assert!(a.champion != start, "the champion moved off the start");
        assert!(a.parents.iter().any(|p| p != &a.parents[0]), "the parents differ");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// **A resume is refused on another arena, another hold-out, with other
    /// parts, from a state cut short or from one saved before these checks**
    /// (review of PR 726, S1, S2, N8, N10): each of those resumed quietly,
    /// carrying one rung's hold-out best into another or breeding from a
    /// parent with its tail zeroed. The whole file is the control, and
    /// another `pop=` still only warns.
    #[test]
    fn a_resume_is_refused_on_another_arena_or_a_state_cut_short() {
        let dir = std::env::temp_dir().join(format!("train-refuse-{}", std::process::id()));
        let start = mind::blank(&[Verb::Walk, Verb::Rest]);
        let bowl = Bowl { aim: start.iter().map(|&w| f64::from(w)).sum::<f64>() + 8.0 };
        let set = Settings { gens: 2, pop: 8, layouts: 2, holdout: 2, seed: 5, threads: 2, pictures: false, patience: 0, wire: 0.0, arena: "step=4 shaft=4".into(), ..Settings::default() };
        train(&bowl, start.clone(), &set, &dir).unwrap();
        let text = std::fs::read_to_string(dir.join(STATE_FILE)).unwrap();
        let load = |text: &str, set: &Settings| {
            let path = dir.join("probe.txt");
            std::fs::write(&path, text).unwrap();
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| RunState::load(&path, &bowl, set, &start)))
        };
        // The refusal's own words, so a state refused for another reason
        // further down (a v1 file has no arena line) cannot stand in for it.
        let why = |r: std::thread::Result<RunState>| r.err().and_then(|e| e.downcast_ref::<String>().cloned()).unwrap_or_default();
        let whole = load(&text, &set).expect("the whole state loads");
        assert_eq!(whole.parents.len(), 2);
        assert!(load(&text, &Settings { arena: "step=3 shaft=4".into(), ..set.clone() }).is_err(), "another arena");
        assert!(why(load(&text, &Settings { holdout: 4, ..set.clone() })).contains("another hold-out"), "another hold-out");
        assert!(load(&text, &Settings { pop: 12, ..set.clone() }).is_ok(), "another population only warns");
        assert!(load(&text.replace("\nparts seed\n", "\nparts seed,loads\n"), &set).is_err(), "other parts");
        let lines: Vec<&str> = text.lines().collect();
        let second = lines.iter().rposition(|l| l.starts_with("genome parent")).unwrap();
        let cuts = [
            ("the end mark", lines[..lines.len() - 1].join("\n")),
            ("the last 3 bytes", text[..text.len() - 3].to_string()),
            ("after the first parent", lines[..second].join("\n")),
            ("midway through the second parent", lines[..(second + lines.len()) / 2].join("\n")),
            ("a wrong count", text.replace("\nend 2\n", "\nend 3\n")),
        ];
        for (cut, t) in cuts {
            assert!(load(&t, &set).is_err(), "a state cut at {cut} loaded");
        }
        // A state saved by PR 726's first head, before the arena, parts and
        // end mark were kept, is refused rather than loaded unchecked.
        let old: Vec<&str> = text.lines().filter(|l| !l.starts_with("arena ") && !l.starts_with("parts ") && !l.starts_with("end ")).collect();
        let old = old.join("\n").replace("run-state 2", "run-state 1");
        assert!(why(load(&old, &set)).contains("start the run afresh"), "a run-state 1 file");
        std::fs::remove_dir_all(&dir).ok();
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

    /// A colony arena whose score names the genome and the layout it ran:
    /// 100 x the genome's first weight, plus the layout's `k`.
    struct Echo;
    impl Challenge for Echo {
        fn name(&self) -> &'static str {
            "echo"
        }
        fn unit(&self) -> Unit {
            Unit::Colony
        }
        fn ants(&self) -> usize {
            1
        }
        fn parts(&self) -> &'static [&'static str] {
            &["k"]
        }
        fn banned(&self) -> &'static [BrainOutput] {
            &[]
        }
        fn mirrored(&self) -> bool {
            false
        }
        fn run(&self, l: Layout, g: &[&[f32]], _: &mut dyn FnMut(u64, &World)) -> Vec<Score> {
            vec![Score { fit: 100.0 * f64::from(g[0][0]) + l.k as f64, parts: vec![l.k as f64] }]
        }
        fn frames(&self) -> u64 {
            1
        }
    }

    /// **Pooled rows keep their own scores.** The hold-out scores its
    /// controls' rows in one pool (`score_each_rows`), so a job that read
    /// another row's genome or layout would move every HOLDOUT line. Rows of
    /// three lengths, more jobs than threads, and the fold matches one row
    /// at a time.
    #[test]
    fn pooled_rows_score_each_row_on_its_own_layouts() {
        let lay = training_layouts(5, 0, 8);
        let (a, b, c) = ([1.0f32], [2.0f32], [3.0f32]);
        let rows: Vec<(&[f32], &[Layout])> = vec![(&a, &lay[..5]), (&b, &lay[2..3]), (&c, &lay)];
        let got = score_each_rows(&Echo, &rows, 3);
        for (r, each) in rows.iter().zip(&got) {
            let want: Vec<f64> = r.1.iter().map(|l| 100.0 * f64::from(r.0[0]) + l.k as f64).collect();
            assert_eq!(each.iter().map(|s| s.fit).collect::<Vec<_>>(), want, "row of genome {}", r.0[0]);
        }
        let one_by_one: Vec<f64> = rows.iter().map(|r| score_clones(&Echo, r.0, r.1, 1).fit).collect();
        assert_eq!(score_rows(&Echo, &rows, 3).iter().map(|s| s.fit).collect::<Vec<_>>(), one_by_one);
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
        let a = mutate(&g, &slots, 0.5, 1.0, 0.0, 1.0, &mut Rng(3));
        let b = mutate(&g, &slots, 0.5, 1.0, 0.0, 1.0, &mut Rng(3));
        assert_eq!(a, b, "same seed, same child");
        for (i, &w) in a.iter().enumerate() {
            if w != 0.0 {
                assert!(slots.contains(&i), "slot {i} moved but is not trainable");
            }
        }
        assert!(a.iter().filter(|&&w| w != 0.0).count() > slots.len() / 4);
    }

    /// **A genome's blocks are read off its length**, the walk genes after
    /// the scorer where there is one, and any other length stops the run.
    #[test]
    fn genome_blocks_read_off_length() {
        let (b, s, w) = (brain::GENOME_LEN, targets::WEIGHTS, WALK);
        assert_eq!(blocks(b), Blocks { scorer: None, walk: None });
        assert_eq!(blocks(b + s), Blocks { scorer: Some(b), walk: None });
        assert_eq!(blocks(b + w), Blocks { scorer: None, walk: Some(b) });
        assert_eq!(blocks(b + s + w), Blocks { scorer: Some(b), walk: Some(b + s) });
        assert!(std::panic::catch_unwind(|| blocks(b + 1)).is_err(), "an unknown length must stop the run");
        assert_eq!(WALK_NAMES.len(), organism::WALK_SLOTS.len() + 1);
    }

    /// **Walk genes move on their own axis, and only where the genome
    /// carries them**: the reserve in `0..=1`, the rest in ±1, every one of
    /// them moved within a few hundred children, and no brain weight moved
    /// at a brain `pm` of 0.
    #[test]
    fn walk_genes_mutate_on_their_own_axis() {
        let ch = challenges::Forage { ants: 12, frames: 10, heap: 300, game_walk: true, far: false };
        let len = genome_len(&ch, true);
        let slots = trainable_for(&ch, len);
        let w = blocks(len).walk.expect("walk genes");
        assert!(slots.ends_with(&(w..w + WALK).collect::<Vec<_>>()));
        assert_eq!(trainable_for(&ch, SCORER), trainable(ch.banned()), "no walk genes, no walk slots");
        let mut g = vec![0.0; len];
        let mut moved = [false; WALK];
        let mut rng = Rng(5);
        for _ in 0..400 {
            g = mutate(&g, &slots, 0.0, 1.0, 0.0, 1.0, &mut rng);
            for (k, m) in moved.iter_mut().enumerate() {
                let v = g[w + k];
                let lo = if k == WALK - 1 { 0.0 } else { -1.0 };
                assert!((lo..=1.0).contains(&v), "{} at {v}", WALK_NAMES[k]);
                *m |= v != 0.0;
            }
        }
        assert!(moved.iter().all(|&m| m), "a walk gene never moved: {moved:?}");
        assert!(g[..SCORER].iter().all(|&x| x == 0.0), "a brain pm of 0 moved a brain weight");
    }

    /// **The connection cost takes a share of the fit and nothing when off**
    /// ([`wired`], [`connections`]): the same share on a 0-1 score and a head
    /// count, always downward, and a sparser brain ranks first at equal fit.
    /// Tight asserts on pure functions, not watched red.
    #[test]
    fn the_connection_cost_charges_a_share_and_nothing_off() {
        // Off: the arena's fit, exactly, whatever the brain's size.
        assert_eq!(wired(0.87, 1_000, 0.0), 0.87);
        assert_eq!(wired(-0.2, 1_000, 0.0), -0.2);
        // On: a share, the same on a 0-1 score and a head count, and always
        // down -- a signed score below 0 must not climb by growing.
        let (a, b) = (wired(0.9, 1_000, 5e-5), wired(18.0, 1_000, 5e-5));
        assert!((a / 0.9 - b / 18.0).abs() < 1e-12 && (0.94..0.96).contains(&(a / 0.9)), "{a} {b}");
        assert!(wired(-0.2, 1_000, 5e-5) < -0.2);
        // Equal fit, the sparser brain ranks first: the point of the cost.
        assert!(wired(0.9, 100, 5e-5) > wired(0.9, 1_000, 5e-5));
        // What it counts: a mind's live weights, today's non-zero ones.
        let mut m = mind::blank(&[Verb::Walk]);
        assert_eq!(connections(&m), 0);
        let last = m.len() - 1;
        m[last] = 0.5;
        assert_eq!(connections(&m), 1);
        let mut t = vec![0.0; brain::GENOME_LEN];
        t[0] = 1.0;
        t[3] = -2.0;
        assert_eq!(connections(&t), 2);
    }

    #[test]
    fn grow_leaves_empty_slots_empty_and_costs_nothing_at_one() {
        let slots = trainable(&[]);
        let g = vec![0.0; brain::GENOME_LEN];
        let made = |grow: f32| mutate(&g, &slots, 1.0, 1.0, 0.0, grow, &mut Rng(9)).iter().filter(|&&w| w != 0.0).count();
        // Every slot hit: all made at 1, about a tenth at 0.1, none at 0.
        let n = slots.len();
        assert!(made(1.0) > n * 9 / 10, "{} of {n}", made(1.0));
        let tenth = made(0.1);
        assert!((n / 20..n / 6).contains(&tenth), "{tenth} of {n}");
        assert_eq!(made(0.0), 0);
        // A live weight still steps whatever `grow` says.
        let mut l = g.clone();
        l[slots[0]] = 2.0;
        assert_ne!(mutate(&l, &slots, 1.0, 1.0, 0.0, 0.0, &mut Rng(9))[slots[0]], 2.0);
    }

    /// **Prune zeroes live weights, and at 0 is the operator before it**,
    /// draw for draw, so every run without it is the run it was.
    #[test]
    fn prune_zeroes_live_weights_and_costs_nothing_off() {
        let slots = trainable(&[]);
        let mut g = vec![0.0; brain::GENOME_LEN];
        for &i in slots.iter().step_by(2) {
            g[i] = 3.0;
        }
        let c = mutate(&g, &slots, 1.0, 1.0, 1.0, 1.0, &mut Rng(4));
        for &i in slots.iter().step_by(2) {
            assert_eq!(c[i], 0.0, "live slot {i} was not pruned");
        }
        let before = |g: &[f32], rng: &mut Rng| -> Vec<f32> {
            let mut c = g.to_vec();
            for &i in &slots {
                if rng.unit() < 0.3 {
                    c[i] = (c[i] + rng.step()).clamp(-40.0, 40.0);
                }
            }
            c
        };
        assert_eq!(mutate(&g, &slots, 0.3, 1.0, 0.0, 1.0, &mut Rng(4)), before(&g, &mut Rng(4)));
    }

    /// **The trim never switches a mind's verb off** (`mind::FIRST_LINK`):
    /// at prune 1 every live connection goes to 0 and every switch, rate and
    /// bound only moves. Put the fault back (prune any live slot) and every
    /// switch reads 0, which is off.
    #[test]
    fn the_trim_never_switches_a_minds_verb_off() {
        use crate::sim::creature::options::Verb;
        let verbs = [Verb::Walk, Verb::TurnAround, Verb::Rest];
        let mut g = mind::blank(&verbs);
        let slots = mind::trainable(&verbs, [false, true, false]);
        for &i in slots.iter().filter(|&&i| i >= mind::FIRST_LINK).step_by(3) {
            g[i] = 3.0;
        }
        let c = mutate(&g, &slots, 1.0, 0.0, 1.0, 1.0, &mut Rng(4));
        for &i in &slots {
            if i >= mind::FIRST_LINK {
                assert_eq!(c[i], 0.0, "live connection {i} was not trimmed");
            } else {
                assert_eq!(c[i], g[i], "setting {i} was trimmed");
            }
        }
        for v in verbs {
            assert_eq!(c[v as usize], 1.0, "{} switched off", v.name());
        }
    }

    /// **A start file keeps its walk genes, and a run that did not ask for
    /// them stops rather than drop them**; a scorer goes between the brain
    /// and the walk genes, and the two controls take out what they say.
    #[test]
    fn a_start_file_keeps_its_walk_genes() {
        let species = vec![0.0; brain::GENOME_LEN];
        let mut g = vec![0.0; SCORER + WALK];
        g[7] = 1.5;
        g[SCORER + 3] = -0.5;
        g[SCORER + WALK - 1] = 0.6;
        let p = std::env::temp_dir().join(format!("train-walk-genome-{}.txt", std::process::id()));
        save_genome(&p, &g).unwrap();
        let file = Start::File(p.clone());
        assert_eq!(start_genome(&file, &species, &[], 0, None, true), g);
        assert!(std::panic::catch_unwind(|| start_genome(&file, &species, &[], 0, None, false)).is_err(), "walk genes were dropped without a word");
        let fresh = start_genome(&Start::Species, &species, &[], 0, None, true);
        assert_eq!(fresh.len(), SCORER + WALK);
        assert!(fresh[SCORER..].iter().all(|&x| x == 0.0), "a fresh walk block is the shipped walk");
        let sc = vec![0.25; targets::WEIGHTS];
        let both = start_genome(&file, &species, &[], 0, Some(&sc), true);
        assert_eq!(blocks(both.len()), Blocks { scorer: Some(SCORER), walk: Some(SCORER + targets::WEIGHTS) });
        assert_eq!(both[SCORER..SCORER + targets::WEIGHTS], sc[..]);
        assert_eq!(both[SCORER + targets::WEIGHTS..], g[SCORER..]);
        assert_eq!(without_scorer(&both), g);
        assert!(walk_shipped(&both)[SCORER + targets::WEIGHTS..].iter().all(|&x| x == 0.0));
        assert_eq!(walk_shipped(&both)[..SCORER + targets::WEIGHTS], both[..SCORER + targets::WEIGHTS]);
        std::fs::remove_file(&p).ok();
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
