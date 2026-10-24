//! **Colony training: selection inside one living colony, run by the game.**
//!
//! The teacher lane's in-run selection (owner's idea, 2026-10-21; design doc
//! <https://claude.ai/artifact/DT1eU7tWsJTyaUCLk67pSV>) as the lab's TRAINING
//! page drives it. One colony of mind ants lives on the 256 x 128 training
//! bed with food dropped on both sides; every ant earns a score for the work
//! it does for the colony (food carried home, nest dug, spoil carried out, a
//! meal at home after work, leaving home hungry); every 500 frames the worst
//! are culled and the best have children, each child a mutated copy of its
//! parent's mind. A **round** is 200,000 frames; the next round founds a
//! fresh bed on the round's 30 best living ants.
//!
//! **This is a port, and the harness is the reference.** The selection loop
//! is `examples/inrun.rs` on the teacher lane's compute branch
//! (`claude/project-thread-8y5ak0`), at the settings of its live control
//! (`scratch/inrun/jobs/ctl.txt`, confirmed current by the lane 2026-10-22),
//! and `scratch/inrun/rounds.sh` for the rounds. Only the control's code
//! paths came across: the rank bar (`turn=`), fresh-record children
//! (`fresh_kid=1`), the lifetime cap with retirement, the hall of the
//! starved, the tank price for food, pay for a hungry departure and for food
//! on its way home (`progress=1`), and size-neutral mutation. The harness's
//! experiments (the z bars, shrunk ranks, rim pay, the scratch engine
//! switches) did not. Every number here has the harness's key beside it in
//! [`Settings`], so a result read on this page can be taken to the lane in
//! its own vocabulary.
//!
//! **What differs from the harness, and why.** The game steps the bed with
//! its own tick (`lab::tick_world`, which is `sim::frame::step`), where the
//! harness steps a subset of the same phases; one set of physics for every
//! world is the owner's rule, so the game's is the one used here and a run
//! here is not bit-identical to the harness's. `examples/labtrain.rs` steps
//! this module with the harness's subset instead, which is how it is checked
//! against the harness (births, selection log). The random arm's next-round
//! shuffle is this module's own generator, not Python's.
//!
//! **Traps already paid for in the harness, kept here:** a cull is by id
//! (`creature::cull_organism`), never by position (ants overlap); every loop
//! over a map that touches credit or counters runs in sorted order (pellets
//! let go in `HashMap` order made 2 of 64 seeds diverge between runs).

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::sync::Arc;

use super::scenario::{self, Placement, Scenario};
use super::scene::LabBox;
use crate::sim::cell::OrganismId;
use crate::sim::creature::{self, mind::MindGenome};
use crate::sim::material;
use crate::sim::organism::{DeathCause, SpeciesId};
use crate::sim::World;
use crate::train::challenges::colony_bed;
use crate::train::{self, Rng};

/// **The old founder genome**: `h1`, the "go out when hungry" lesson's mind
/// (teacher lane, 2026-10-21), the in-run control's founder until the
/// standard line replaced it. Kept so its rows can be re-run; it was trained
/// with the home sense off, and has live links on the slots that sense writes.
pub const H1: &str = include_str!("../../assets/minds/h1.raw.txt");

/// **The new ant** (owner, 2026-10-24: the standard training path is default
/// on, in the lab too): the standard line's carried pool -- the 30 best ants
/// of the best-balanced colony at the last round of a colony-selection batch
/// on the standard settings. A pool rather than one brain (owner, 2026-10-22:
/// the colony's surviving mixture is the product). Each member carries its
/// own evolved dig head past [`creature::mind::GENOME_LEN`]. Format, source
/// and its common-garden numbers in the file's header.
///
/// **A ratchet** (owner, 2026-10-24: "every next test should be using the best
/// ant from the previous training"): each batch is founded from this pool, and
/// its best colony's carry replaces it. History: brood1ctl colony 7 round 31
/// (from hcs, rules weaned; garden nest 377, crater 3, starved 3, rooms 14) ->
/// stdcheck stdoff colony 27 round 15 (from that pool, rules off from round 0;
/// garden nest 530, crater 3, starved 6, rooms 44) -> farfood2 ramp2far colony 64 round 23 (food ramped 45 -> 80
/// columns, far sense 100:100) with its far-sense links dropped -> farfood3b logsense colony 12 round 23 (food ramped
/// 80 -> 100 columns, graded far sense `log:40`, now the default; garden nest 409, crater 0, starved 53; lab box with
/// fruit 150 columns out 95-110 alive, where this pool's first version starved to 0 beyond 100).
pub const STD_POOL: &str = include_str!("../../assets/minds/std_pool.txt");

/// [`STD_POOL`]'s members as genomes this build runs: each one whole, its own
/// dig head kept when heads evolve (`PP_DIGHEAD_EVOLVE`, on by default) and
/// cut off otherwise; a side head of another shape (`PP_DIGHEAD=path`)
/// replaces it.
pub fn std_pool() -> Vec<Vec<f32>> {
    let mut lines = STD_POOL.lines().filter(|l| !l.starts_with('#'));
    let head = lines.next().expect("std_pool: header");
    let mut h = head.split_whitespace();
    assert_eq!((h.next(), h.nth(1)), (Some("pool"), Some("genome")), "std_pool: {head:?}");
    let len: usize = h.next().and_then(|n| n.parse().ok()).expect("std_pool: genome length");
    let mut pool: Vec<Vec<f32>> = Vec::new();
    for l in lines {
        if l.starts_with("member ") {
            pool.push(pool.first().cloned().unwrap_or_else(|| vec![0.0; len]));
            continue;
        }
        let (i, w) = l.split_once(' ').unwrap_or_else(|| panic!("std_pool: {l:?} is not `slot value`"));
        let g = pool.last_mut().expect("std_pool: a slot before the first member");
        g[i.parse::<usize>().expect("std_pool slot")] = w.parse().expect("std_pool value");
    }
    pool.iter_mut().for_each(fit_head);
    pool
}

/// A saved genome made fit for this build's dig head: its own head kept when
/// heads evolve (`PP_DIGHEAD_EVOLVE`, on by default) and it carries one of
/// the side head's shape, the side head's added when it carries none or one
/// of another shape, and any head cut off when heads do not evolve.
pub fn fit_head(g: &mut Vec<f32>) {
    let body = creature::mind::GENOME_LEN;
    // SCRATCH (brood lane, 2026-10-24): a lay gate rides last ([`creature::mind::lay_bar_scale`]); off the end while
    // the head is fitted, and back on if this build reads gates (`PP_LAYGENE`, on by default).
    let hl = creature::mind::scratch_log::head_evolve().unwrap_or(0);
    let lay = (g.len() == body + hl + creature::mind::LAY_LEN).then(|| g.split_off(body + hl));
    match creature::mind::scratch_log::head_evolve() {
        Some(hl) if g.len() == body + hl => {}
        Some(_) => {
            g.truncate(body);
            g.extend(creature::mind::scratch_log::dighead().expect("a head evolves").flat());
        }
        None => g.truncate(body),
    }
    if let Some(lay) = lay.filter(|_| creature::mind::laygene_on()) {
        g.extend(lay);
    }
}

/// **Which ants have children.**
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The score picks (`sel=score`).
    Score,
    /// The same numbers of births and culls, chosen blind (`sel=random`):
    /// the control that says whether the score is doing anything.
    Random,
    /// Nobody is culled and nobody is born (`sel=off`): the colony as its
    /// founders leave it.
    Off,
}

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Mode::Score => "BY SCORE",
            Mode::Random => "AT RANDOM",
            Mode::Off => "OFF",
        }
    }
    pub fn next(self) -> Mode {
        match self {
            Mode::Score => Mode::Random,
            Mode::Random => Mode::Off,
            Mode::Off => Mode::Score,
        }
    }
}

/// **Every setting of a training run**, at the in-run control's values
/// (`ctl.txt`); the harness's key is in each field's doc.
#[derive(Clone, Debug)]
pub struct Settings {
    /// `sel=`
    pub mode: Mode,
    /// Frames a round runs (rounds.sh `FRAMES`).
    pub round_frames: u64,
    /// Founders of every round (rounds.sh: 30).
    pub founders: usize,
    /// `grants=`: every founder starts on this many full tanks.
    pub grants: f32,
    /// `kid_grants=`: every child starts on this many.
    pub kid_grants: f32,
    /// `turn=`: each adult is replaced about once per this many frames.
    pub turn: u64,
    /// `judge=`: frames between judgements.
    pub judge_every: u64,
    /// `min_age=`: frames before an ant is judged.
    pub min_age: u64,
    /// `cooldown=`: frames between one parent's children.
    pub cooldown: u64,
    /// `lo=` / `hi=`: the headcount band.
    pub lo: usize,
    pub hi: usize,
    /// `zhard=`: an adult this many standard errors below the median goes at once.
    pub zhard: f64,
    /// `kid_cap=` and `retire=1`: a parent's lifetime children, and whether it
    /// leaves when it has had them.
    pub kid_cap: u32,
    pub retire: bool,
    /// `hall=` / `dead_w=`: a starved adult stays a candidate parent this
    /// many frames, its score times `dead_w` (0 = off).
    pub hall: u64,
    pub dead_w: f64,
    /// `kpm=` / `ksigma=` / `kevents=`: size-neutral mutation (`mutation=neutral`).
    pub kpm: f32,
    pub ksigma: f32,
    pub kevents: usize,
    /// `w_deliver=`, `w_dig=`, `w_clear=`, `w_field=`, `w_depth=`,
    /// `w_home_eat=`, `w_hungry=`: what each job pays.
    pub w_deliver: f64,
    pub w_dig: f64,
    pub w_clear: f64,
    pub w_field: f64,
    pub w_depth: f64,
    pub w_home_eat: f64,
    pub w_hungry: f64,
    /// `hungry=` / `hungry_gap=`: below this many tanks a departure is hungry,
    /// paid at most once per this many frames.
    pub hungry: f64,
    pub hungry_gap: u64,
    /// `k=`: the dig and clear prices' reach above 1.
    pub kprice: f64,
    /// `tank=` / `ktank=`: food's price rises as the colony's mean tank falls
    /// below `tank` (`fprice=tank`).
    pub tank: f64,
    pub ktank: f64,
    /// `progress=1`: half a delivery's pay is paid on the way home.
    pub progress: bool,
    /// `rim=` / `rim_ramp=`: spoil let go within `rim` columns of the nest
    /// pays nothing, rising to full `rim_ramp` columns further out.
    pub rim: f64,
    pub rim_ramp: f64,
    /// `nest_r=`: only cells cut this close to the nest pay.
    pub nest_r: i32,
    /// `dist=`, `spread=`, `drip=`, `every=`, `cap=`: food dropped `dist`
    /// columns out on both sides, scattered over `spread` either way, at most
    /// `drip` cells every `every` frames, up to `cap` cells a side.
    pub dist: i32,
    pub spread: i32,
    pub drip: usize,
    pub every: u64,
    pub cap: usize,
    /// rounds.sh's round-0 mutants: `mutants=30:0.03:1.0:7`.
    pub start_pm: f32,
    pub start_sigma: f32,
    pub start_seed: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            mode: Mode::Score,
            round_frames: 200_000,
            founders: 30,
            grants: 3.0,
            // 3 since 2026-10-22 (teacher lane, 64 seeds x 6 rounds against the
            // control at 1: food delivered per seed 11,076 vs 6,923, nest score
            // -44 vs -53; newcomers on one tank starved in ~1.1k frames).
            kid_grants: 3.0,
            turn: 30_000,
            judge_every: 500,
            min_age: 8000,
            cooldown: 3000,
            lo: 24,
            hi: 36,
            zhard: 3.0,
            kid_cap: 4,
            retire: true,
            hall: 30_000,
            dead_w: 0.5,
            kpm: 0.03,
            ksigma: 1.0,
            kevents: 10,
            w_deliver: 7.0,
            w_dig: 1.0,
            w_clear: 1.0,
            w_field: 0.05,
            w_depth: 2.0,
            w_home_eat: 2.0,
            w_hungry: 10.0,
            hungry: 2.0,
            hungry_gap: 3000,
            kprice: 2.0,
            tank: 3.0,
            ktank: 4.0,
            progress: true,
            rim: 15.0,
            rim_ramp: 10.0,
            nest_r: 20,
            dist: 25,
            spread: 10,
            drip: 20,
            every: 500,
            cap: 60,
            start_pm: 0.03,
            start_sigma: 1.0,
            start_seed: 7,
        }
    }
}

/// A founder: its family tag and its mind's genome.
#[derive(Clone, Debug)]
pub struct Founder {
    pub tag: String,
    pub genome: Vec<f32>,
}

/// **Round 0's founders**: the standard pool ([`std_pool`]), member `k` as
/// founder `m{k}`, as colony selection carries a colony into its next round;
/// past the pool's size, mutants of its members (rounds.sh's
/// `mutants=30:0.03:1.0:7`). Until 2026-10-24 this was `h1` and 29 mutants.
pub fn first_founders(s: &Settings) -> Vec<Founder> {
    let pool = std_pool();
    let slots = colony_bed::slots();
    let mut rng = Rng(s.start_seed);
    (0..s.founders)
        .map(|k| {
            let base = &pool[k % pool.len()];
            let genome = if k < pool.len() { base.clone() } else { train::mutate(base, &slots, s.start_pm, s.start_sigma, 0.0, 1.0, &[], &mut rng) };
            Founder { tag: format!("m{k:02}"), genome }
        })
        .collect()
}

/// **The new ant, for any box**: the standard pool as minds for ants that are
/// not on the training bed (the lab box's ANT BRAIN switch), each cut to the
/// dig rung's verbs as the training bed's founders are. An ant gets the
/// member its id picks ([`new_ant_mind_for`]), so a box holds the mixture.
pub fn new_ant_minds() -> Vec<Arc<MindGenome>> {
    std_pool().iter().map(|g| colony_bed::mind(g)).collect()
}

/// Which of [`new_ant_minds`] ant `id` gets: fixed by the id, so a reload or
/// a sweep never reshuffles who thinks with what.
pub fn new_ant_mind_for(minds: &[Arc<MindGenome>], id: OrganismId) -> Arc<MindGenome> {
    minds[id as usize % minds.len()].clone()
}

/// Size-neutral mutation (owner, 2026-10-21: brain size should be set by the
/// link cost, not by the mutation operator): the harness's `neutral_mutate`.
/// Live links and settings take a weight step with probability `pm`; then
/// `events` structural changes, each a fair coin between adding a link and
/// removing one, so the expected change in size is zero at every size.
fn neutral_mutate(g: &[f32], slots: &[usize], pm: f32, sigma: f32, events: usize, rng: &mut Rng) -> Vec<f32> {
    let first = creature::mind::FIRST_LINK;
    let mut c = g.to_vec();
    for &i in slots {
        if (c[i] != 0.0 || i < first) && rng.unit() < pm {
            c[i] = (c[i] + sigma * rng.step()).clamp(-40.0, 40.0);
        }
    }
    for _ in 0..events {
        let add = rng.unit() < 0.5;
        let pool: Vec<usize> = slots.iter().copied().filter(|&i| i >= first && (c[i] == 0.0) == add).collect();
        if pool.is_empty() {
            continue;
        }
        let i = pool[rng.below(pool.len())];
        c[i] = if add { (sigma * rng.step()).clamp(-40.0, 40.0) } else { 0.0 };
    }
    c
}

/// One ant's ledger (the harness's `Who`).
#[derive(Clone, Debug)]
pub struct Who {
    pub born: u64,
    pub credit: f64,
    g2: f64,
    pub family: String,
    pub gen: u32,
    pub genome: Vec<f32>,
    pub last_kid: u64,
    pub kids: u32,
    tripped: bool,
    at_kid: (f64, f64),
}

impl Who {
    fn new(born: u64, family: String, gen: u32, genome: Vec<f32>) -> Who {
        Who { born, credit: 0.0, g2: 0.0, family, gen, genome, last_kid: 0, kids: 0, tripped: false, at_kid: (0.0, 0.0) }
    }

    /// Score: credit per thousand frames alive.
    pub fn score(&self, frame: u64) -> f64 {
        self.credit / ((frame - self.born) as f64 / 1000.0).max(0.1)
    }

    /// The founder line: the family tag up to its first `.` (a later round's
    /// founder is tagged `line.id`, as rounds.sh tags it).
    pub fn line(&self) -> &str {
        self.family.split('.').next().unwrap_or("")
    }

    /// Live links in its mind.
    pub fn links(&self) -> usize {
        self.genome.iter().filter(|v| **v != 0.0).count()
    }

    fn pay(&mut self, g: f64) {
        self.credit += g;
        self.g2 += g * g;
    }
}

/// What the births log records (the harness's `births.csv` `kind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A child of a living parent, earned on its score.
    Birth,
    /// A child of a living parent, to refill a colony below its band.
    Refill,
    /// A child of a starved ant from the hall.
    Hall,
    /// Removed by selection.
    Cull,
    /// Removed after its last child.
    Retire,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Birth => "birth",
            Kind::Refill => "refill",
            Kind::Hall => "hall",
            Kind::Cull => "cull",
            Kind::Retire => "retire",
        }
    }
}

/// One row of the births log.
#[derive(Clone, Debug)]
pub struct Event {
    pub frame: u64,
    pub id: OrganismId,
    /// The parent, for a birth (0 for a hall parent, which is dead).
    pub parent: OrganismId,
    pub family: String,
    pub gen: u32,
    pub kind: Kind,
    /// The child's live links, for a birth.
    pub links: Option<usize>,
    /// The removed ant's score at removal, for a cull or a retirement.
    pub score: Option<f64>,
}

impl Event {
    /// The harness's `births.csv` row.
    pub fn csv(&self) -> String {
        match self.links {
            Some(l) => format!("{},{},{},{},{},{},{l}", self.frame, self.id, self.parent, self.family, self.gen, self.kind.name()),
            None => format!("{},{},0,{},{},{}", self.frame, self.id, self.family, self.gen, self.kind.name()),
        }
    }
}

/// One judgement's line (the harness's `sel.csv`).
#[derive(Clone, Debug)]
pub struct SelRow {
    pub frame: u64,
    pub alive: usize,
    pub adults: usize,
    pub births: usize,
    pub removals: usize,
    pub bias: f64,
    pub median: f64,
    pub food_price: f64,
    pub dig_price: f64,
    pub clear_price: f64,
    pub ghosts: usize,
}

impl SelRow {
    pub fn csv(&self) -> String {
        format!(
            "{},{},{},{},{},{:.2},{:.3},{:.2},{:.2},{:.2},{}",
            self.frame, self.alive, self.adults, self.births, self.removals, self.bias, self.median, self.food_price, self.dig_price, self.clear_price, self.ghosts
        )
    }
}

/// One sample of the round's series, every 1,000 frames.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sample {
    pub frame: u64,
    pub alive: usize,
    pub food_home: u64,
    pub births: u64,
    pub removals: u64,
    pub starved: u64,
    /// The colony's mean bank, in full tanks.
    pub tank_mean: f64,
    /// Food cells stored in the nest (below the datum, within 12 columns).
    pub nest_food: u32,
}

/// A finished round, for the page's round-over-round strip.
#[derive(Clone, Debug)]
pub struct RoundSummary {
    pub round: usize,
    pub seed: u64,
    pub frames: u64,
    pub food_home: u64,
    pub births: u64,
    pub removals: u64,
    pub starved: u64,
    pub alive: usize,
    pub nest_score: Option<f64>,
    pub mode: Mode,
}

/// **One round: one bed, one colony, its ledger.** The harness's `main`.
pub struct Round {
    pub number: usize,
    pub seed: u64,
    pub sp: SpeciesId,
    /// The bed as built (and as `lab::tick_world` needs it).
    pub bed: LabBox,
    /// Ticks this round has run.
    pub fr: u64,
    pub book: HashMap<OrganismId, Who>,
    hall: Vec<(u64, f64, Who)>,
    surf: Vec<i32>,
    orig: Vec<bool>,
    held: HashMap<OrganismId, (OrganismId, f64)>,
    loose: HashMap<(i32, i32), (OrganismId, f64)>,
    loose_n: i64,
    culled_pending: HashSet<OrganismId>,
    quota: f64,
    retiring: Vec<OrganismId>,
    capped_only: bool,
    srng: Rng,
    kslots: Vec<usize>,
    bias: f64,
    delivered: VecDeque<(u64, f64)>,
    eaten: VecDeque<(u64, f64)>,
    cell_worth: f64,
    n_field: u64,
    dig_price: f64,
    base_open: Option<i64>,
    tank_p: f64,
    start_e: f64,
    leave: HashMap<OrganismId, (bool, u64)>,
    carry: HashMap<OrganismId, (f64, f64, f64, u32)>,
    home: (u32, (i32, i32)),
    pub food_home: u64,
    pub births: u64,
    pub removals: u64,
    pub starved: u64,
    pub birth_fail: u64,
    pub hungry_paid: u64,
    pub median: f64,
    /// The births log, oldest first.
    pub events: Vec<Event>,
    /// The judgements, oldest first.
    pub sel: Vec<SelRow>,
    /// Every 1,000 frames.
    pub samples: Vec<Sample>,
    /// The general nest score (`lab::nestscore`, HARVESTER), every 25,000
    /// frames. Printed only, never paid (owner, 2026-10-21).
    pub nest_score: Option<f64>,
}

fn ants(w: &World, sp: SpeciesId) -> Vec<OrganismId> {
    let mut v: Vec<OrganismId> = w.live_organism_ids().into_iter().filter(|&id| w.organism(id).map(|s| s.species) == Some(sp)).collect();
    v.sort();
    v
}

/// Ground as the dig arena counts it: powder or solid, not a body.
fn ground(w: &World, x: i32, y: i32) -> bool {
    let c = w.get(x, y);
    c.material != material::EMPTY && matches!(w.materials.kind(c.material), material::MaterialKind::Powder | material::MaterialKind::Solid) && c.organism_id() == 0
}

/// Open to an ant: empty, or an animal's body.
fn open(w: &World, x: i32, y: i32) -> bool {
    let m = w.get(x, y).material;
    m == material::EMPTY || w.materials.kind(m) == material::MaterialKind::Creature
}

/// A child at the colony's door: placed as a founder is, homed on the door,
/// given `genome` and `grants` full tanks. Tries the door's columns and the
/// rows above it until the body fits (the harness's `birth_in`).
fn birth_in(w: &mut World, sp: SpeciesId, colony: u32, anchor: (i32, i32), genome: Arc<MindGenome>, grants: f32) -> Option<OrganismId> {
    let full = grants * w.species.get(sp).creature.as_ref().map_or(0.0, |d| d.start_energy);
    for dy in 1..16 {
        for dx in [0, 2, -2, 4, -4, 6, -6] {
            let (x, y) = (anchor.0 + dx, anchor.1 - dy);
            if !w.in_bounds(x, y) || w.get(x, y).material != material::EMPTY {
                continue;
            }
            if let Some(site) = creature::plant_founder_in(w, x, y, "ant", Some(colony), Some(anchor)) {
                let crate::sim::scheduler::ActiveKind::Creature { organism } = site.kind else { continue };
                w.schedule_active_site(site);
                creature::mind::give(w, organism, genome);
                if let Some(s) = w.organism_mut(organism) {
                    s.energy = full;
                }
                return Some(organism);
            }
        }
    }
    None
}

/// A child at the door of the colony its first living ant belongs to.
fn birth(w: &mut World, sp: SpeciesId, genome: Arc<MindGenome>, grants: f32) -> Option<OrganismId> {
    let any = *ants(w, sp).first()?;
    let (colony, anchor) = w.organism(any).map(|s| (s.colony, s.forage_anchor))?;
    birth_in(w, sp, colony, anchor, genome, grants)
}

/// Provisions within `reach` columns of `x`.
fn food_near(w: &World, x: i32, reach: i32) -> usize {
    let Some(p) = w.materials.id_of("provisions") else { return 0 };
    let mut n = 0;
    for cx in (x - reach)..=(x + reach) {
        for y in 0..colony_bed::SIZE.1 {
            if w.get(cx, y).material == p {
                n += 1;
            }
        }
    }
    n
}

/// Top the food at `x` up towards `cap` cells, at most `drip` at a time.
fn top_up(w: &mut World, bed: &LabBox, x: i32, cap: usize, drip: usize, spread: i32) {
    let have = food_near(w, x, 8 + spread);
    if have >= cap {
        return;
    }
    let add = (cap - have).min(drip) as i32;
    let heap = if spread > 0 {
        Placement::Scatter { material: "provisions".into(), x0: x - spread, x1: x + spread, cells: add }
    } else {
        Placement::Heap { material: "provisions".into(), x, cells: add }
    };
    scenario::apply_placements(w, &[heap], bed);
}

impl Round {
    /// **Build a round's bed and found its colony** on `founders`, in the
    /// harness's founding order (round-robin over `mix`, each entry its
    /// count of ants). Returns the round and its world.
    pub fn found(number: usize, seed: u64, mix: &[(Founder, usize)], s: &Settings) -> (Round, World, Scenario) {
        let n: usize = mix.iter().map(|m| m.1).sum();
        let sc = colony_bed::scenario(seed, n);
        let (mut w, _) = sc.bed.build_counted();
        let mut bed = sc.bed.clone();
        sc.apply(&mut w, &mut bed);
        scenario::place_colony(&mut w, "ant", colony_bed::NEST, n as i32, Some(true));
        w.births_paused = true;
        // Births only from the score; mutation is the loop's, not the body's.
        w.mutation = Some(false);
        let sp = w.species.id_of("ant").expect("ant");
        let mut def = w.species.get(sp).creature.clone().expect("creature");
        def.reproduce_threshold = 1.0e30;
        // No old age (owner, 2026-10-21): selection's removals are the only
        // deaths but starvation.
        def.life_half_life = 0;
        w.species.set_creature(sp, def);
        // Interleaved round-robin, so no genome owns one end of the heap.
        let mut order: Vec<usize> = Vec::with_capacity(n);
        let mut left: Vec<usize> = mix.iter().map(|m| m.1).collect();
        while order.len() < n {
            for (i, k) in left.iter_mut().enumerate() {
                if *k > 0 {
                    *k -= 1;
                    order.push(i);
                }
            }
        }
        let slot = |k: usize| order[k.min(n.saturating_sub(1))];
        let minds: Vec<Arc<MindGenome>> = mix.iter().map(|m| colony_bed::mind(&m.0.genome)).collect();
        for (k, id) in ants(&w, sp).into_iter().enumerate() {
            creature::mind::give(&mut w, id, minds[slot(k)].clone());
        }
        w.cut_log = Some(Vec::new());
        w.food_log = Some(Vec::new());
        w.digest_log = Some(Vec::new());
        w.death_log = Some(Vec::new());
        let founders = ants(&w, sp);
        let full = s.grants * w.species.get(sp).creature.as_ref().map_or(0.0, |d| d.start_energy);
        for &id in &founders {
            if let Some(st) = w.organism_mut(id) {
                st.energy = full;
            }
        }
        let start_e = w.species.get(sp).creature.as_ref().map_or(1.0, |d| d.start_energy) as f64;
        let mut book = HashMap::new();
        for (k, id) in founders.iter().enumerate() {
            let f = &mix[slot(k)].0;
            book.insert(*id, Who::new(0, f.tag.clone(), 0, f.genome.clone()));
        }
        let home = founders.first().and_then(|&i| w.organism(i)).map(|st| (st.colony, st.forage_anchor)).unwrap_or((0, (colony_bed::NEST, 0)));
        let (bw, bh) = colony_bed::SIZE;
        let surf: Vec<i32> = (0..bw).map(|x| super::census::surface_of(&w, &bed, x)).collect();
        let mut orig = vec![false; (bw * bh) as usize];
        for x in 0..bw {
            for y in surf[x as usize].max(0)..bh {
                orig[(y * bw + x) as usize] = ground(&w, x, y);
            }
        }
        let round = Round {
            number,
            seed,
            sp,
            bed,
            fr: 0,
            book,
            hall: Vec::new(),
            surf,
            orig,
            held: HashMap::new(),
            loose: HashMap::new(),
            loose_n: 0,
            culled_pending: HashSet::new(),
            quota: 0.0,
            retiring: Vec::new(),
            capped_only: false,
            srng: Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xA5A5),
            kslots: colony_bed::slots(),
            bias: 0.0,
            delivered: VecDeque::new(),
            eaten: VecDeque::new(),
            cell_worth: 0.0,
            n_field: 0,
            dig_price: 1.0 + s.kprice,
            base_open: None,
            tank_p: 1.0,
            start_e,
            leave: HashMap::new(),
            carry: HashMap::new(),
            home,
            food_home: 0,
            births: 0,
            removals: 0,
            starved: 0,
            birth_fail: 0,
            hungry_paid: 0,
            median: f64::NAN,
            events: Vec::new(),
            sel: Vec::new(),
            samples: Vec::new(),
            nest_score: None,
        };
        (round, w, sc)
    }

    /// How many ants are alive.
    pub fn alive(&self, w: &World) -> usize {
        ants(w, self.sp).len()
    }

    fn in_nest(&self, s: &Settings, x: i32) -> bool {
        s.nest_r <= 0 || (x - colony_bed::NEST).abs() <= s.nest_r
    }

    fn surf_at(&self, x: i32) -> i32 {
        self.surf[x.clamp(0, colony_bed::SIZE.0 - 1) as usize]
    }

    /// **Everything the harness does after a step**: the food drip, the
    /// ledger from this tick's logs, the prices, and every `judge_every`
    /// frames the judgement. Call once after every tick of the bed.
    pub fn after_step(&mut self, w: &mut World, s: &Settings) {
        self.fr += 1;
        let fr = self.fr;
        if fr % s.every == 1 || s.every == 1 {
            for x in [colony_bed::NEST + s.dist, colony_bed::NEST - s.dist] {
                top_up(w, &self.bed, x, s.cap, s.drip, s.spread);
            }
        }
        let frame = w.frame;
        let (bw, bh) = colony_bed::SIZE;
        // ---- cuts
        let cuts = w.cut_log.as_mut().map(std::mem::take).unwrap_or_default();
        for &(x, y, _, who) in &cuts {
            let i = (y * bw + x) as usize;
            if (0..bw).contains(&x) && (0..bh).contains(&y) && self.orig[i] {
                self.orig[i] = false;
                let worth = if self.in_nest(s, x) { 1.0 } else { 0.0 };
                self.held.insert(who, (who, worth));
                if worth == 0.0 {
                    continue;
                }
                let g = worth * 0.5 * s.w_dig * self.dig_price;
                if let Some(b) = self.book.get_mut(&who) {
                    b.tripped = true;
                    b.pay(g);
                }
            } else {
                let (mut tag, mut nest) = (0, 0.0);
                'find: for r in 0..=2 {
                    for dy in -r..=r {
                        for dx in -r..=r {
                            if let Some((t, n)) = self.loose.remove(&(x + dx, y + dy)) {
                                (tag, nest) = (t, n);
                                break 'find;
                            }
                        }
                    }
                }
                self.held.insert(who, (tag, nest));
                if tag != 0 {
                    self.loose_n -= 1;
                }
            }
        }
        // Put back for whoever else reads the log this tick (the lab's dig
        // map drains it); the next tick's `before_step` empties it.
        if let Some(log) = w.cut_log.as_mut() {
            *log = cuts;
        }
        // ---- food
        let food = w.food_log.as_mut().map(std::mem::take).unwrap_or_default();
        for r in food {
            let (x, y) = r.at;
            match r.kind {
                creature::FOOD_SWALLOW if r.flag => {
                    if let Some(b) = self.book.get_mut(&r.who) {
                        if b.tripped && s.w_home_eat > 0.0 {
                            b.tripped = false;
                            b.pay(s.w_home_eat);
                        }
                    }
                }
                creature::FOOD_SWALLOW => {
                    self.n_field += 1;
                    self.cell_worth += (r.worth as f64 - self.cell_worth) / self.n_field as f64;
                    if let Some(b) = self.book.get_mut(&r.who) {
                        b.credit -= s.w_field;
                        b.g2 += s.w_field * s.w_field;
                    }
                }
                creature::FOOD_DROP if r.flag => {
                    self.food_home += 1;
                    self.delivered.push_back((frame, if self.n_field > 0 { self.cell_worth } else { 1.0 }));
                    let deep = 1.0 + s.w_depth * ((y - self.surf_at(x)).clamp(0, 10) as f64 / 10.0);
                    let g = deep * s.w_deliver * self.tank_p * if s.progress { 0.5 } else { 1.0 };
                    if let Some(b) = self.book.get_mut(&r.who) {
                        if let Some(c) = self.carry.get_mut(&r.who) {
                            c.3 += 1;
                        }
                        b.pay(g);
                    }
                }
                _ => {}
            }
        }
        // ---- digestion
        let digested = w.digest_log.as_mut().map(std::mem::take).unwrap_or_default();
        for (_, worth, _) in digested {
            self.eaten.push_back((frame, worth as f64));
        }
        // ---- deaths
        let deaths = w.death_log.as_mut().map(std::mem::take).unwrap_or_default();
        for r in deaths {
            if r.species != self.sp {
                continue;
            }
            self.held.remove(&r.id);
            if r.cause == DeathCause::Starved {
                self.starved += 1;
            }
            if !self.culled_pending.remove(&r.id) {
                if let Some(b) = self.book.remove(&r.id) {
                    let age = frame - b.born;
                    if s.hall > 0 && age >= s.min_age && r.cause == DeathCause::Starved {
                        self.hall.push((frame, b.credit / (age as f64 / 1000.0), b));
                    }
                }
            }
        }
        // ---- pellets let go, where the head is now. Sorted: `held` is a
        // map, and the order pellets are handled in moves `loose_n` and the
        // order credits are summed (2 of 64 seeds diverged unsorted).
        let mut gone: Vec<OrganismId> = self.held.keys().copied().filter(|&id| w.organism(id).is_none_or(|st| st.spoil.is_none())).collect();
        gone.sort_unstable();
        for id in gone {
            let (tag, worth) = self.held.remove(&id).expect("listed above");
            let Some(&(x, y)) = w.organism(id).and_then(|st| st.chain.first()) else { continue };
            if y < self.surf_at(x) {
                let alive = self.book.len().max(1) as f64;
                let far = if s.rim > 0.0 { ((((x - colony_bed::NEST).abs() as f64) - s.rim) / s.rim_ramp.max(1.0)).clamp(0.0, 1.0) } else { 1.0 };
                let far = far * worth;
                let pc = far * s.w_clear * (1.0 + s.kprice * (self.loose_n as f64 / (2.0 * alive)).clamp(0.0, 1.0));
                if let Some(b) = self.book.get_mut(&id) {
                    b.tripped = true;
                    b.pay(pc);
                }
                if tag != 0 {
                    let g = far * 0.5 * s.w_dig * self.dig_price;
                    if let Some(b) = self.book.get_mut(&tag) {
                        b.pay(g);
                    }
                }
            } else {
                self.loose.insert((x, y), (tag, worth));
                self.loose_n += 1;
            }
        }
        // ---- food paid on its way home (`progress=1`)
        if s.progress && fr.is_multiple_of(100) {
            let fpr_now = self.tank_p;
            let mut ids: Vec<OrganismId> = self.book.keys().copied().collect();
            ids.sort_unstable();
            for id in ids {
                let st = w.organism(id).map(|st| (st.chain.first().copied(), st.trip_cells, st.crop.map_or(0, |c| c.cells)));
                let Some((Some((x, _)), trip, cells)) = st else { continue };
                if trip > 0 && cells > 0 {
                    let d = (x - colony_bed::NEST).abs() as f64;
                    let c = self.carry.entry(id).or_insert((d.max(1.0), d, 0.0, 0));
                    if d < c.1 {
                        let g = 0.5 * s.w_deliver * fpr_now * cells as f64 * (c.1 - d) / c.0;
                        c.1 = d;
                        c.2 += g;
                        self.book.get_mut(&id).expect("listed").pay(g);
                    }
                } else if let Some(c) = self.carry.remove(&id) {
                    if c.3 == 0 && c.2 > 0.0 {
                        self.book.get_mut(&id).expect("listed").credit -= c.2;
                    }
                }
            }
        }
        if fr.is_multiple_of(100) {
            // The tank price: food pays least while everyone is fat.
            let es: Vec<f64> = ants(w, self.sp).iter().filter_map(|&i| w.organism(i)).map(|st| st.energy as f64).collect();
            let mean = es.iter().sum::<f64>() / es.len().max(1) as f64;
            self.tank_p = 1.0 + s.ktank * (1.0 - mean / (s.tank * self.start_e)).clamp(0.0, 1.0);
            if s.w_hungry > 0.0 {
                let mut ids: Vec<OrganismId> = self.book.keys().copied().collect();
                ids.sort_unstable();
                for id in ids {
                    let Some(st) = w.organism(id) else { continue };
                    let Some(&(x, _)) = st.chain.first() else { continue };
                    let dx = (x - colony_bed::NEST).abs();
                    let e = st.energy as f64;
                    let lv = self.leave.entry(id).or_insert((true, 0));
                    if dx <= 12 {
                        lv.0 = true;
                    } else if dx >= 15 && lv.0 {
                        lv.0 = false;
                        if e < s.hungry * self.start_e && (lv.1 == 0 || frame - lv.1 >= s.hungry_gap) {
                            lv.1 = frame;
                            self.hungry_paid += 1;
                            self.book.get_mut(&id).expect("listed").pay(s.w_hungry);
                        }
                    }
                }
            }
            for (id, b) in self.book.iter_mut() {
                if let Some(&(x, _)) = w.organism(*id).and_then(|st| st.chain.first()) {
                    if (x - colony_bed::NEST).abs() >= 15 {
                        b.tripped = true;
                    }
                }
            }
        }
        while self.delivered.front().is_some_and(|d| d.0 + 5000 < frame) {
            self.delivered.pop_front();
        }
        while self.eaten.front().is_some_and(|d| d.0 + 5000 < frame) {
            self.eaten.pop_front();
        }
        if fr.is_multiple_of(1000) || fr == 1 {
            // The dig price: open nest cells per ant against 25.
            let mut open_n = 0i64;
            for x in (colony_bed::NEST - 100)..=(colony_bed::NEST + 100) {
                for y in self.surf[x as usize].max(40)..bh {
                    if open(w, x, y) {
                        open_n += 1;
                    }
                }
            }
            let b0 = *self.base_open.get_or_insert(open_n);
            let per = (open_n - b0) as f64 / self.book.len().max(1) as f64;
            self.dig_price = 1.0 + s.kprice * (1.0 - per / 25.0).max(0.0);
        }
        if s.mode != Mode::Off && fr.is_multiple_of(s.judge_every) {
            self.judge(w, s);
        }
        if fr.is_multiple_of(25_000) {
            let door = w.nest_sites.first().map_or(colony_bed::NEST, |n| n.x);
            let (ns, _) = super::nestscore::of_world(w, &self.bed, door, &super::nestscore::HARVESTER);
            self.nest_score = Some(ns.score);
        }
        if fr.is_multiple_of(1000) {
            self.samples.push(self.sample(w));
        }
    }

    fn sample(&self, w: &World) -> Sample {
        let living = ants(w, self.sp);
        let tanks: Vec<f64> = living.iter().filter_map(|&i| w.organism(i)).map(|st| st.energy as f64 / self.start_e).collect();
        let (bw, bh) = colony_bed::SIZE;
        let corpse = w.materials.id_of("corpse");
        let mut nest_food = 0u32;
        for x in (colony_bed::NEST - 12).max(0)..=(colony_bed::NEST + 12).min(bw - 1) {
            for y in self.surf[x as usize].max(0)..bh {
                let c = w.get(x, y);
                if c.organism_id() == 0 && Some(c.material) != corpse && w.materials.get(c.material).food_energy > 0.0 {
                    nest_food += 1;
                }
            }
        }
        Sample {
            frame: self.fr,
            alive: living.len(),
            food_home: self.food_home,
            births: self.births,
            removals: self.removals,
            starved: self.starved,
            tank_mean: tanks.iter().sum::<f64>() / tanks.len().max(1) as f64,
            nest_food,
        }
    }

    fn cull(&mut self, w: &mut World, id: OrganismId, kind: Kind, frame: u64) -> bool {
        if !creature::cull_organism(w, id) {
            return false;
        }
        self.culled_pending.insert(id);
        self.removals += 1;
        let b = self.book.remove(&id).expect("culled from the book");
        self.events.push(Event { frame, id, parent: 0, family: b.family.clone(), gen: b.gen, kind, links: None, score: Some(b.score(frame)) });
        true
    }

    /// **The judgement** (the harness's `turn=` arm): each adult replaced
    /// about once per `turn` frames, the bar a rank rather than a fixed z;
    /// clear failures (z below `-zhard`) go on top of the quota; nobody below
    /// the median breeds; the best get up to three children.
    fn judge(&mut self, w: &mut World, s: &Settings) {
        let frame = w.frame;
        let ids: Vec<OrganismId> = {
            let mut v: Vec<OrganismId> = self.book.keys().copied().collect();
            v.sort_unstable();
            v
        };
        let n = ids.len();
        // Each ant's rate per 1,000 frames alive, against the adults' median,
        // in units of its own standard error: a sum of credits is Poisson-like,
        // so its error is sqrt(sum of squares), floored at what the colony's
        // average ant would have shown over the same age.
        let (sg2, st): (f64, f64) = ids.iter().map(|i| (self.book[i].g2, (frame - self.book[i].born) as f64 / 1000.0)).fold((0.0, 0.0), |a, c| (a.0 + c.0, a.1 + c.1));
        let rate_g2 = sg2 / st.max(1e-9);
        let adults: Vec<OrganismId> = ids.iter().copied().filter(|i| frame - self.book[i].born >= s.min_age).collect();
        let mut births_n = 0usize;
        let mut removals_n = 0usize;
        let mut med = f64::NAN;
        if !adults.is_empty() {
            if n < s.lo {
                self.bias = (self.bias - 0.05).max(-1.5);
            } else if n > s.hi {
                self.bias = (self.bias + 0.05).min(1.5);
            }
            let score = |b: &Who| b.credit / ((frame - b.born) as f64 / 1000.0).max(0.1);
            let mut sc: Vec<f64> = adults.iter().map(|i| score(&self.book[i])).collect();
            sc.sort_by(|a, b| a.partial_cmp(b).unwrap());
            med = sc[sc.len() / 2];
            let z = |b: &Who| {
                let t = ((frame - b.born) as f64 / 1000.0).max(0.1);
                (score(b) - med) / (b.g2.max(rate_g2 * t).sqrt() / t).max(1e-6)
            };
            // The family cap: no founder line past a quarter of the band's top.
            let mut fam: BTreeMap<String, usize> = BTreeMap::new();
            for i in &ids {
                *fam.entry(self.book[i].family.clone()).or_default() += 1;
            }
            let cap_f = (s.hi / 4).max(2);
            let mut zs: Vec<(OrganismId, f64)> = adults.iter().map(|i| (*i, z(&self.book[i]))).collect();
            zs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap().then(a.0.cmp(&b.0)));
            // Owed replacements carry over until a child is born, capped at 5.
            self.quota = (self.quota + adults.len() as f64 * s.judge_every as f64 / s.turn.max(1) as f64).min(5.0);
            let q_now = self.quota.floor() as usize;
            let mut cut: Vec<OrganismId> = zs.iter().filter(|p| p.1 < -s.zhard).map(|p| p.0).collect();
            let hard_n = cut.len();
            let extra_cut = n.saturating_sub(s.hi);
            for p in &zs {
                if cut.len() >= hard_n + extra_cut || p.1 >= 0.0 {
                    break;
                }
                if !cut.contains(&p.0) {
                    cut.push(p.0);
                }
            }
            // The quota's removals wait for its births.
            let mut quota_cut: Vec<OrganismId> = zs.iter().filter(|p| p.1 < 0.0 && !cut.contains(&p.0)).take(q_now).map(|p| p.0).collect();
            let short = if s.hall > 0 { 0 } else { s.lo.saturating_sub(n) };
            let want = (hard_n + quota_cut.len() + short) as u32;
            // Each child earned on the record since the parent's last one.
            let zb_of = |b: &Who| -> Option<f64> {
                if b.kids >= s.kid_cap || frame - b.last_kid < s.cooldown {
                    return None;
                }
                if b.kids == 0 {
                    return Some(z(b));
                }
                let since = frame - b.last_kid;
                if since < s.min_age {
                    return None;
                }
                let t = since as f64 / 1000.0;
                let (c, g2) = (b.credit - b.at_kid.0, b.g2 - b.at_kid.1);
                Some((c / t - med) / (g2.max(rate_g2 * t).sqrt() / t).max(1e-6))
            };
            let mut okz: Vec<(OrganismId, f64)> = zs.iter().filter_map(|p| zb_of(&self.book[&p.0]).map(|v| (p.0, v))).filter(|p| p.1 > 0.0 || (n < s.lo && p.1 >= 0.0)).collect();
            // The family cap steers births to other lines; it never stops them.
            let open_ok: Vec<(OrganismId, f64)> = okz.iter().copied().filter(|p| fam.get(&self.book[&p.0].family).copied().unwrap_or(0) < cap_f).collect();
            self.capped_only = open_ok.is_empty();
            if okz.is_empty() && n < s.lo {
                if let Some(best) = zs.last() {
                    okz.push(*best);
                }
            }
            if !self.capped_only {
                okz = open_ok;
            }
            okz.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
            let ok: Vec<OrganismId> = okz.into_iter().map(|p| p.0).collect();
            let mut kids = vec![0u32; ok.len()];
            let mut given = 0;
            'passes: for span in [ok.len(), ok.len().div_ceil(2), ok.len().div_ceil(4)] {
                for k in kids.iter_mut().take(span) {
                    if given >= want {
                        break 'passes;
                    }
                    *k += 1;
                    given += 1;
                }
            }
            let mut parents: Vec<(OrganismId, u32)> = ok.into_iter().zip(kids).filter(|p| p.1 > 0).collect();
            let room = (s.hi + 4).saturating_sub(n);
            let floor_room = n.saturating_sub(s.lo.saturating_sub(8));
            cut.truncate(floor_room);
            if s.mode == Mode::Random {
                // Same counts, chosen blind.
                let kids: u32 = parents.iter().map(|p| p.1).sum();
                let pool: Vec<OrganismId> = adults.iter().copied().filter(|i| frame - self.book[i].last_kid >= s.cooldown || self.book[i].last_kid == 0).collect();
                parents = (0..kids).filter(|_| !pool.is_empty()).map(|_| (pool[self.srng.below(pool.len())], 1)).collect();
                let mut ad = adults.clone();
                let mut c2 = Vec::new();
                for _ in 0..cut.len().min(ad.len()) {
                    c2.push(ad.swap_remove(self.srng.below(ad.len())));
                }
                cut = c2;
                let mut qc = Vec::new();
                for _ in 0..quota_cut.len().min(ad.len()) {
                    qc.push(ad.swap_remove(self.srng.below(ad.len())));
                }
                quota_cut = qc;
            }
            for &id in &cut {
                if self.cull(w, id, Kind::Cull, frame) {
                    removals_n += 1;
                }
            }
            for (pid, k) in parents {
                for _ in 0..k {
                    if births_n >= room {
                        break;
                    }
                    let Some(p) = self.book.get(&pid) else { break };
                    let f = p.family.clone();
                    if fam.get(&f).copied().unwrap_or(0) >= cap_f && !self.capped_only {
                        break;
                    }
                    let child = neutral_mutate(&p.genome, &self.kslots, s.kpm, s.ksigma, s.kevents, &mut self.srng);
                    let gen = p.gen + 1;
                    let Some(cid) = birth(w, self.sp, colony_bed::mind(&child), s.kid_grants) else {
                        self.birth_fail += 1;
                        continue;
                    };
                    births_n += 1;
                    self.births += 1;
                    *fam.entry(f.clone()).or_default() += 1;
                    let pb = self.book.get_mut(&pid).expect("parent");
                    pb.last_kid = frame;
                    pb.kids += 1;
                    pb.at_kid = (pb.credit, pb.g2);
                    if s.retire && pb.kids >= s.kid_cap {
                        self.retiring.push(pid);
                    }
                    let links = child.iter().filter(|v| **v != 0.0).count();
                    self.events.push(Event { frame, id: cid, parent: pid, family: f.clone(), gen, kind: Kind::Birth, links: Some(links), score: None });
                    self.book.insert(cid, Who::new(frame, f, gen, child));
                }
            }
            for id in std::mem::take(&mut self.retiring) {
                if self.book.contains_key(&id) && self.cull(w, id, Kind::Retire, frame) {
                    removals_n += 1;
                }
            }
            let refill = if s.hall > 0 { 0 } else { s.lo.saturating_sub(n) };
            let paid = births_n.saturating_sub(refill + hard_n).min(q_now);
            self.quota -= paid as f64;
            for id in quota_cut.into_iter().take(births_n.saturating_sub(refill + hard_n)) {
                if self.book.contains_key(&id) && self.cull(w, id, Kind::Cull, frame) {
                    removals_n += 1;
                }
            }
        }
        if s.hall > 0 {
            self.hall.retain(|h| frame - h.0 <= s.hall);
            let n_now = self.book.len();
            if n_now < s.lo {
                // Plain score per 1k frames for both; a dead ant's is cut by
                // `dead_w` (a loss grows by 1/dead_w).
                let pen = |v: f64| if v > 0.0 { v * s.dead_w } else { v / s.dead_w.max(1e-6) };
                let mut ids: Vec<OrganismId> = self.book.keys().copied().collect();
                ids.sort_unstable();
                let mut cand: Vec<(f64, Option<OrganismId>, usize)> = ids
                    .iter()
                    .filter(|i| frame - self.book[i].born >= s.min_age && self.book[i].kids < s.kid_cap)
                    .map(|i| (self.book[i].credit / ((frame - self.book[i].born) as f64 / 1000.0), Some(*i), 0))
                    .collect();
                cand.extend(self.hall.iter().enumerate().filter(|h| h.1 .2.kids < s.kid_cap).map(|(k, h)| (pen(h.1), None, k)));
                if s.mode == Mode::Random {
                    for k in (1..cand.len()).rev() {
                        cand.swap(k, self.srng.below(k + 1));
                    }
                } else {
                    cand.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
                }
                let need = s.lo - n_now;
                let mut kids = vec![0u32; cand.len()];
                let mut given = 0;
                'hp: for span in [cand.len(), cand.len().div_ceil(2), cand.len().div_ceil(4)] {
                    for k in kids.iter_mut().take(span) {
                        if given >= need {
                            break 'hp;
                        }
                        *k += 1;
                        given += 1;
                    }
                }
                let (home_colony, home_anchor) = self.home;
                for (c, k) in cand.iter().zip(kids) {
                    for _ in 0..k {
                        let (genome, family, gen, pid) = match c.1 {
                            Some(i) => (self.book[&i].genome.clone(), self.book[&i].family.clone(), self.book[&i].gen, i),
                            None => {
                                let h = &self.hall[c.2];
                                (h.2.genome.clone(), h.2.family.clone(), h.2.gen, 0)
                            }
                        };
                        let child = neutral_mutate(&genome, &self.kslots, s.kpm, s.ksigma, s.kevents, &mut self.srng);
                        let Some(cid) = birth_in(w, self.sp, home_colony, home_anchor, colony_bed::mind(&child), s.kid_grants) else {
                            self.birth_fail += 1;
                            continue;
                        };
                        births_n += 1;
                        self.births += 1;
                        match c.1 {
                            Some(i) => {
                                let pb = self.book.get_mut(&i).expect("parent");
                                pb.kids += 1;
                                pb.last_kid = frame;
                                pb.at_kid = (pb.credit, pb.g2);
                            }
                            None => self.hall[c.2].2.kids += 1,
                        }
                        let kind = if c.1.is_some() { Kind::Refill } else { Kind::Hall };
                        let links = child.iter().filter(|v| **v != 0.0).count();
                        self.events.push(Event { frame, id: cid, parent: pid, family: family.clone(), gen: gen + 1, kind, links: Some(links), score: None });
                        self.book.insert(cid, Who::new(frame, family, gen + 1, child));
                    }
                }
            }
        }
        if !med.is_nan() {
            self.median = med;
        }
        let cp = 1.0 + s.kprice * (self.loose_n as f64 / (2.0 * self.book.len().max(1) as f64)).clamp(0.0, 1.0);
        let ghosts = ants(w, self.sp).iter().filter(|i| !self.book.contains_key(i)).count();
        self.sel.push(SelRow {
            frame,
            alive: self.book.len(),
            adults: adults.len(),
            births: births_n,
            removals: removals_n,
            bias: self.bias,
            median: med,
            food_price: self.tank_p,
            dig_price: self.dig_price,
            clear_price: cp,
            ghosts,
        });
    }

    /// **The round's best living ants**, best first, by score at its last
    /// frame rounded to three places as rounds.sh reads `index.csv` (ties by
    /// id); at random in the random arm.
    pub fn ranked(&self, frame: u64, mode: Mode, rng: &mut Rng) -> Vec<(OrganismId, &Who)> {
        let mut r: Vec<(OrganismId, &Who)> = self.book.iter().map(|(i, b)| (*i, b)).collect();
        r.sort_by_key(|p| p.0);
        if mode == Mode::Random {
            for k in (1..r.len()).rev() {
                r.swap(k, rng.below(k + 1));
            }
        } else {
            let three = |b: &Who| format!("{:.3}", b.score(frame)).parse::<f64>().unwrap_or(0.0);
            r.sort_by(|a, b| three(b.1).partial_cmp(&three(a.1)).unwrap().then(a.0.cmp(&b.0)));
        }
        r
    }

    /// **The next round's founders** (rounds.sh): the top `n` living ants,
    /// each tagged `family.id`, repeated to make `n` when fewer are alive.
    /// Empty when the colony is extinct.
    pub fn next_founders(&self, frame: u64, s: &Settings) -> Vec<(Founder, usize)> {
        let mut rng = Rng(self.seed.wrapping_mul(7).wrapping_add(self.number as u64 + 1));
        let top: Vec<(OrganismId, &Who)> = self.ranked(frame, s.mode, &mut rng).into_iter().take(s.founders).collect();
        let n = top.len();
        top.iter()
            .enumerate()
            .map(|(i, (id, b))| (Founder { tag: format!("{}.{id}", b.family), genome: b.genome.clone() }, s.founders / n + usize::from(i < s.founders % n)))
            .collect()
    }

    pub fn summary(&self, w: &World, mode: Mode) -> RoundSummary {
        RoundSummary {
            round: self.number,
            seed: self.seed,
            frames: self.fr,
            food_home: self.food_home,
            births: self.births,
            removals: self.removals,
            starved: self.starved,
            alive: self.alive(w),
            nest_score: self.nest_score,
            mode,
        }
    }
}

/// **A training run**: its settings, the round on the bed, and every
/// finished round's summary.
pub struct Training {
    pub settings: Settings,
    /// rounds.sh's `SEED`: round `r` is founded on `seed * 1000 + r`, round 0 on `seed`.
    pub seed: u64,
    pub round: Round,
    pub past: Vec<RoundSummary>,
    /// Set when a round ended with no ant alive.
    pub extinct: bool,
}

impl Training {
    /// Round 0 on `seed`: the bed and its 30 founders.
    pub fn start(settings: Settings, seed: u64) -> (Training, World, Scenario) {
        let mix: Vec<(Founder, usize)> = first_founders(&settings).into_iter().map(|f| (f, 1)).collect();
        let (round, world, sc) = Round::found(0, seed, &mix, &settings);
        (Training { settings, seed, round, past: Vec::new(), extinct: false }, world, sc)
    }

    /// Call before every tick: the logs the ledger reads are on and empty.
    pub fn before_step(&self, w: &mut World) {
        w.cut_log.get_or_insert_with(Vec::new).clear();
        w.food_log.get_or_insert_with(Vec::new);
        w.digest_log.get_or_insert_with(Vec::new);
        w.death_log.get_or_insert_with(Vec::new);
    }

    /// Call after every tick.
    pub fn after_step(&mut self, w: &mut World) {
        self.round.after_step(w, &self.settings);
    }

    /// Whether the round on the bed has run its frames.
    pub fn round_done(&self) -> bool {
        self.round.fr >= self.settings.round_frames
    }

    /// **End the round on the bed and found the next** on its best living
    /// ants. `None`, and `extinct` set, when nobody is alive to found it.
    pub fn next_round(&mut self, w: &World) -> Option<(World, Scenario)> {
        self.past.push(self.round.summary(w, self.settings.mode));
        let mix = self.round.next_founders(w.frame, &self.settings);
        if mix.is_empty() {
            self.extinct = true;
            return None;
        }
        let number = self.round.number + 1;
        let seed = self.seed * 1000 + number as u64;
        let (round, world, sc) = Round::found(number, seed, &mix, &self.settings);
        self.round = round;
        Some((world, sc))
    }
}

/// **When a setting takes effect.**
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum When {
    /// From the next tick.
    Now,
    /// At the next START.
    NextStart,
}

/// **One setting the TRAINING page can step**: its label, the harness key it
/// is, the ladder of values a click walks, and what it means.
pub struct Knob {
    pub label: &'static str,
    pub key: &'static str,
    pub note: &'static str,
    pub when: When,
    steps: &'static [f64],
    get: fn(&Settings) -> f64,
    set: fn(&mut Settings, f64),
    show: fn(f64) -> String,
}

fn thousands(v: f64) -> String {
    if v == 0.0 {
        "OFF".to_string()
    } else {
        format!("{}K", (v / 1000.0).round() as u64)
    }
}

fn plain(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{v:.0}")
    } else {
        format!("{v}")
    }
}

/// **Every setting on the page**, in the order it lists them. The control's
/// value is always one of the steps, so a click round the ladder comes back to it.
pub const KNOBS: &[Knob] = &[
    Knob {
        label: "WHO HAS CHILDREN",
        key: "sel",
        note: "BY SCORE: THE BEST ANTS HAVE CHILDREN AND THE WORST ARE CULLED. AT RANDOM: THE SAME NUMBER OF BIRTHS AND CULLS, PICKED BLIND -- THE CONTROL THAT SAYS WHETHER THE SCORE IS DOING ANYTHING. OFF: NOBODY IS CULLED OR BORN. HARNESS KEY SEL.",
        when: When::Now,
        steps: &[0.0, 1.0, 2.0],
        get: |s| match s.mode {
            Mode::Score => 0.0,
            Mode::Random => 1.0,
            Mode::Off => 2.0,
        },
        set: |s, v| s.mode = [Mode::Score, Mode::Random, Mode::Off][v as usize],
        show: |v| [Mode::Score, Mode::Random, Mode::Off][v as usize].label().to_string(),
    },
    Knob {
        label: "ROUND LENGTH",
        key: "FRAMES",
        note: "FRAMES BEFORE THE ROUND ENDS AND A FRESH BED IS FOUNDED ON ITS 30 BEST LIVING ANTS. THE TEACHER LANE RUNS 200K.",
        when: When::Now,
        steps: &[50_000.0, 100_000.0, 200_000.0, 400_000.0],
        get: |s| s.round_frames as f64,
        set: |s, v| s.round_frames = v as u64,
        show: thousands,
    },
    Knob {
        label: "TURNOVER",
        key: "turn",
        note: "EACH ADULT IS REPLACED ABOUT ONCE PER THIS MANY FRAMES: THE PACE OF SELECTION. FASTER IS MORE GENERATIONS AND NOISIER PICKS (AN ANT'S SCORE SAYS LITTLE ABOUT ITS LINE BEFORE ~8K FRAMES).",
        when: When::Now,
        steps: &[15_000.0, 30_000.0, 60_000.0, 120_000.0],
        get: |s| s.turn as f64,
        set: |s, v| s.turn = v as u64,
        show: thousands,
    },
    Knob {
        label: "CHILD STARTS ON",
        key: "kid_grants",
        note: "FULL TANKS OF FOOD A NEWBORN STARTS WITH. ON ONE TANK NEWCOMERS STARVED IN ABOUT 1,100 FRAMES; 3 DELIVERED 60% MORE FOOD OVER 64 SEEDS (TEACHER LANE, 2026-10-22).",
        when: When::Now,
        steps: &[1.0, 2.0, 3.0, 5.0],
        get: |s| s.kid_grants as f64,
        set: |s, v| s.kid_grants = v as f32,
        show: |v| format!("{} TANKS", plain(v)),
    },
    Knob {
        label: "MUTATION",
        key: "kevents/kpm",
        note: "HOW FAR A CHILD'S MIND STRAYS FROM ITS PARENT'S: LINKS ADDED OR REMOVED (10 AT NORMAL, A FAIR COIN EACH, SO BRAINS NEITHER GROW NOR SHRINK ON AVERAGE) AND THE SHARE OF LIVE LINKS NUDGED (3% AT NORMAL).",
        when: When::Now,
        steps: &[0.5, 1.0, 2.0],
        get: |s| s.kevents as f64 / 10.0,
        set: |s, v| {
            s.kevents = (10.0 * v) as usize;
            s.kpm = 0.03 * v as f32;
        },
        show: |v| ["SMALL", "NORMAL", "LARGE"][if v < 0.75 { 0 } else if v < 1.5 { 1 } else { 2 }].to_string(),
    },
    Knob {
        label: "PAY: FOOD HOME",
        key: "w_deliver",
        note: "WHAT A FOOD CELL CARRIED HOME PAYS, HALF ON THE WAY AND HALF ON ARRIVAL, MORE THE DEEPER IT IS PUT DOWN AND THE EMPTIER THE COLONY'S TANKS.",
        when: When::Now,
        steps: &[0.0, 3.5, 7.0, 14.0],
        get: |s| s.w_deliver,
        set: |s, v| s.w_deliver = v,
        show: plain,
    },
    Knob {
        label: "PAY: DIGGING",
        key: "w_dig",
        note: "WHAT CUTTING A CELL OF THE NEST PAYS (HALF ON THE CUT, HALF WHEN THE SPOIL LEAVES THE NEST). ONLY CELLS WITHIN 20 COLUMNS OF THE NEST PAY.",
        when: When::Now,
        steps: &[0.0, 0.5, 1.0, 2.0],
        get: |s| s.w_dig,
        set: |s, v| s.w_dig = v,
        show: plain,
    },
    Knob {
        label: "PAY: SPOIL OUT",
        key: "w_clear",
        note: "WHAT CARRYING A PELLET OF SPOIL UP AND OUT PAYS. NOTHING WITHIN 15 COLUMNS OF THE NEST (IT ROLLS BACK IN), FULL PAY 10 COLUMNS FURTHER OUT.",
        when: When::Now,
        steps: &[0.0, 0.5, 1.0, 2.0],
        get: |s| s.w_clear,
        set: |s, v| s.w_clear = v,
        show: plain,
    },
    Knob {
        label: "PAY: HUNGRY TRIP",
        key: "w_hungry",
        note: "WHAT LEAVING HOME WITH LESS THAN TWO TANKS IN THE BANK PAYS, AT MOST ONCE PER 3,000 FRAMES: THE HABIT A COLONY THAT STARVES TOGETHER LACKS.",
        when: When::Now,
        steps: &[0.0, 5.0, 10.0, 20.0],
        get: |s| s.w_hungry,
        set: |s, v| s.w_hungry = v,
        show: plain,
    },
    Knob {
        label: "PAY: MEAL AT HOME",
        key: "w_home_eat",
        note: "WHAT A MEAL AT HOME PAYS, ONCE PER STRETCH OF WORK (A TRIP OUT, A CUT, A PELLET CARRIED) -- SO WORKING AND COMING BACK TO EAT PAYS AND SITTING AT HOME EATING THE STORE DOES NOT.",
        when: When::Now,
        steps: &[0.0, 1.0, 2.0, 4.0],
        get: |s| s.w_home_eat,
        set: |s, v| s.w_home_eat = v,
        show: plain,
    },
    Knob {
        label: "FOOD PER DROP",
        key: "drip",
        note: "CELLS OF FOOD DROPPED ON EACH SIDE EVERY 500 FRAMES, UP TO 60 A SIDE.",
        when: When::Now,
        steps: &[5.0, 10.0, 20.0, 40.0],
        get: |s| s.drip as f64,
        set: |s, v| s.drip = v as usize,
        show: plain,
    },
    Knob {
        label: "FOOD DISTANCE",
        key: "dist",
        note: "HOW FAR FROM THE NEST THE FOOD IS DROPPED, IN COLUMNS, BOTH SIDES. FOOD ALREADY DOWN STAYS WHERE IT IS.",
        when: When::Now,
        steps: &[15.0, 25.0, 40.0, 60.0],
        get: |s| s.dist as f64,
        set: |s, v| s.dist = v as i32,
        show: plain,
    },
    Knob {
        label: "HALL OF THE STARVED",
        key: "hall",
        note: "A STARVED ADULT STAYS A CANDIDATE PARENT THIS LONG, AT HALF ITS SCORE, SO A COLONY THAT STARVES TOGETHER IS REFILLED FROM ITS BEST ANTS AND NOT ONLY FROM WHOEVER IS LEFT.",
        when: When::Now,
        steps: &[0.0, 15_000.0, 30_000.0, 60_000.0],
        get: |s| s.hall as f64,
        set: |s, v| s.hall = v as u64,
        show: thousands,
    },
];

impl Knob {
    pub fn value(&self, s: &Settings) -> String {
        (self.show)((self.get)(s))
    }

    /// Step to the next value on the ladder, round to the first.
    pub fn step(&self, s: &mut Settings) {
        let now = (self.get)(s);
        let i = self.steps.iter().position(|v| (v - now).abs() < 1e-9).map_or(0, |i| (i + 1) % self.steps.len());
        (self.set)(s, self.steps[i]);
    }

    /// Whether it is at the control's value.
    pub fn is_default(&self, s: &Settings) -> bool {
        ((self.get)(s) - (self.get)(&Settings::default())).abs() < 1e-9
    }
}

/// One ant on the page's list of the best.
#[derive(Clone, Debug)]
pub struct AntRow {
    pub id: OrganismId,
    pub born: u64,
    pub family: String,
    pub gen: u32,
    pub score: f64,
    pub kids: u32,
    pub links: usize,
    /// Its bank, in full tanks.
    pub tanks: f64,
}

/// **What the TRAINING page shows**, read off a run at draw time.
#[derive(Clone, Debug)]
pub struct View {
    pub settings: Settings,
    pub seed: u64,
    pub round: usize,
    pub fr: u64,
    pub alive: usize,
    pub adults: usize,
    pub births: u64,
    pub removals: u64,
    pub starved: u64,
    pub food_home: u64,
    pub median: f64,
    pub nest_score: Option<f64>,
    pub samples: Vec<Sample>,
    pub past: Vec<RoundSummary>,
    /// Every living ant, best score first.
    pub ants: Vec<AntRow>,
    /// The births log, newest first.
    pub recent: Vec<Event>,
    /// Living ants per founder line, most first.
    pub lines: Vec<(String, usize)>,
    pub extinct: bool,
}

impl Training {
    pub fn view(&self, w: &World) -> View {
        let r = &self.round;
        let frame = w.frame;
        let start_e = r.start_e;
        let mut ants: Vec<AntRow> = r
            .book
            .iter()
            .map(|(id, b)| AntRow {
                id: *id,
                born: b.born,
                family: b.family.clone(),
                gen: b.gen,
                score: b.score(frame),
                kids: b.kids,
                links: b.links(),
                tanks: w.organism(*id).map_or(0.0, |st| st.energy as f64 / start_e),
            })
            .collect();
        ants.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal).then(a.id.cmp(&b.id)));
        let mut lines: BTreeMap<String, usize> = BTreeMap::new();
        for b in r.book.values() {
            *lines.entry(b.line().to_string()).or_default() += 1;
        }
        let mut lines: Vec<(String, usize)> = lines.into_iter().collect();
        lines.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        View {
            settings: self.settings.clone(),
            seed: self.seed,
            round: r.number,
            fr: r.fr,
            alive: r.book.len(),
            adults: r.book.values().filter(|b| frame - b.born >= self.settings.min_age).count(),
            births: r.births,
            removals: r.removals,
            starved: r.starved,
            food_home: r.food_home,
            median: r.median,
            nest_score: r.nest_score,
            samples: r.samples.clone(),
            past: self.past.clone(),
            ants,
            recent: r.events.iter().rev().take(40).cloned().collect(),
            lines,
            extinct: self.extinct,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The founder pool loads as minds**: thirty founders, each the pool
    /// member of its index, all different, every one a mind with live links
    /// and (heads evolving by default) its own dig head; past the pool's size
    /// the founders are mutants.
    #[test]
    fn the_first_founders_are_the_standard_pool() {
        let s = Settings::default();
        let f = first_founders(&s);
        let pool = std_pool();
        assert_eq!((f.len(), pool.len()), (30, 30));
        assert_eq!(f[0].tag, "m00");
        assert!(f.iter().zip(&pool).all(|(f, g)| &f.genome == g));
        let distinct: HashSet<Vec<u32>> = pool.iter().map(|g| g.iter().map(|w| w.to_bits()).collect()).collect();
        assert_eq!(distinct.len(), 30, "the pool's members differ");
        for g in &pool {
            assert!(train::is_mind(g.len()));
            let m = colony_bed::mind(g);
            assert!(m.live > 100, "a member has {} live links", m.live);
            assert_eq!(m.head.is_some(), creature::mind::scratch_log::head_evolve().is_some());
        }
        let more = first_founders(&Settings { founders: 40, ..Settings::default() });
        assert!(more[30..].iter().zip(&pool).filter(|(f, g)| &f.genome != *g).count() >= 8, "founders past the pool are mutants");
    }

    /// **A round founds a colony of minds and judges it**: every founder in
    /// the book and alive with a mind, and after the first judgements the
    /// selection log has a row each and no ghosts (live ants the book does
    /// not know).
    #[test]
    fn a_round_founds_minds_and_keeps_its_book() {
        let s = Settings { min_age: 600, judge_every: 300, ..Settings::default() };
        let (mut t, mut w, _) = Training::start(s, 1);
        let founders = ants(&w, t.round.sp);
        // **24, not 30**: the heap at the door fits 24 founders and the
        // last six of the thirty are never placed -- the harness's own
        // round founds 24 too (`inrun` on seed 1 prints "24 founders").
        assert_eq!(founders.len(), 24, "the heap at the door");
        assert!(founders.iter().all(|&id| t.round.book.contains_key(&id) && creature::mind::has(&w, id)));
        for _ in 0..1500 {
            t.before_step(&mut w);
            crate::sim::update::step(&mut w);
            w.step_active_sites();
            w.step_fields();
            w.step_pheromones();
            t.after_step(&mut w);
        }
        assert_eq!(t.round.sel.len(), 5, "judged every 300 frames");
        assert!(t.round.sel.iter().all(|r| r.ghosts == 0), "{:?}", t.round.sel);
        assert!(t.round.book.values().any(|b| b.credit != 0.0), "no ant earned anything in 1,500 frames");
    }
}
