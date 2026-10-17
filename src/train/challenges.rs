//! The rig's first three arenas. Each one says what it pays for, what the
//! score could be faked with, and what stops that.

use super::{Challenge, Layout, Score, Unit};
use crate::lab::census::{self, Ids};
use crate::lab::scenario::Scenario;
use crate::sim::brain::BrainOutput as O;
use crate::sim::chunk::Rect;
use crate::sim::cell::OrganismId;
use crate::sim::organism::SpeciesId;
use crate::sim::pheromone::{self, Channel};
use crate::sim::{creature, material, update, Cell, World};

/// Verbs that belong to other species or to a game this rig is not playing
/// yet: hopping and flying (found by the probe as the cheap way to be
/// "home"), fighting, and births (the population is fixed in every arena).
pub const NOT_ANT_VERBS: &[O] = &[O::Impulse, O::Fly, O::Attack, O::Provision, O::Lay];

/// One frame of the world, the serial driver: one world per thread, so the
/// parallel driver's shared pool would only serialise the threads.
fn frame(w: &mut World, pheromones: bool) {
    update::step(w);
    w.step_active_sites();
    w.step_fields();
    if pheromones {
        w.step_pheromones();
    }
}

fn ants_of(w: &World, species: SpeciesId) -> Vec<OrganismId> {
    let mut v: Vec<OrganismId> = w
        .live_organism_ids()
        .into_iter()
        .filter(|&id| w.organism(id).map(|s| s.species) == Some(species))
        .collect();
    v.sort();
    v
}

const BED_W: i32 = 256;
const NEST_X: i32 = 128;

/// The lab bed every colony arena uses: 256 x 128, 40 rows of soil, no rain,
/// one colony at the centre, and whatever `extra` placements the arena adds.
fn bed(seed: u64, ants: usize, extra: &str) -> Scenario {
    let text = format!(
        r#"(
    name: "train", title: "TRAIN", question: "training arena",
    bed: (width: {BED_W}, height: 128, soil_depth: 40, ground_y: 64, compartments: 1, founders: 0,
          species: "grass", colonies: 0, colony_ants: {ants}, colony_species: "ant", predators: 0,
          lamp_spacing: 33, seed: {seed}, extra_walls: []),
    settings: [(subject: "the bed", field: "rain", value: 0)],
    placements: [
        Colony(species: "ant", x: {NEST_X}, count: {ants}),
        {extra}
    ],
    timeline: [],
    horizon: 100000,
)"#
    );
    ron::from_str(&text).expect("training bed parses")
}

/// Every ant in a fresh bed takes `genome`, births are put out of reach, and
/// the brain steers (`Chooser::Off`) unless `game_walk`. A `blind` layout
/// also cuts the body's homeward re-roll and forces the brain walk, so the
/// senses-cut control is blind outside the brain too ([`Layout::blind`]).
fn clone_colony(w: &mut World, genome: &[f32], game_walk: bool, blind: bool) -> SpeciesId {
    let sp = w.species.id_of("ant").expect("ant");
    let mut def = w.species.get(sp).creature.clone().expect("creature");
    def.reproduce_threshold = 1.0e30;
    if blind {
        def.home_bias = 0.0;
    }
    w.species.set_creature(sp, def);
    w.species.set_genome(sp, genome.to_vec());
    if !game_walk || blind {
        w.chooser = Some(creature::Chooser::Off);
    }
    for id in ants_of(w, sp) {
        w.set_organism_genome(id, genome.to_vec());
    }
    sp
}

/// **Bring food home, and stay alive doing it** (colony).
///
/// A heap of 60 provisions cells 35-100 columns from the nest, east on even
/// layouts and west on odd ones. Score: food at home at the end (cells within
/// 12 columns of the nest, plus crop food digested at the nest in cells --
/// which counts every meal eaten at the nest, a dead nestmate's body too;
/// the survival factor already charges for the death)
/// times the fraction of the colony still alive. The survival factor is there
/// because the probe's walking-only winner brought food home while the colony
/// shrank from 12 to 8; the hop ban because its unbanned winner flew it home.
pub struct Forage {
    pub ants: usize,
    pub frames: u64,
    pub heap: i32,
    pub game_walk: bool,
}

impl Forage {
    fn dist(layout: Layout) -> i32 {
        35 + ((layout.seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 32) % 66) as i32
    }
}

fn cells_of(w: &World, x0: i32, x1: i32, mat: material::MaterialId) -> i64 {
    let mut n = 0;
    for x in x0..=x1 {
        for y in 0..128 {
            if w.get(x, y).material == mat {
                n += 1;
            }
        }
    }
    n
}

impl Challenge for Forage {
    fn name(&self) -> &'static str {
        "forage"
    }
    fn unit(&self) -> Unit {
        Unit::Colony
    }
    fn ants(&self) -> usize {
        self.ants
    }
    fn frames(&self) -> u64 {
        self.frames
    }
    fn parts(&self) -> &'static [&'static str] {
        &["home", "alive", "taken", "pickups", "dist"]
    }
    fn banned(&self) -> &'static [O] {
        NOT_ANT_VERBS
    }
    fn run(&self, layout: Layout, genomes: &[&[f32]], look: &mut dyn FnMut(u64, &World)) -> Vec<Score> {
        let side = if layout.k.is_multiple_of(2) { 1 } else { -1 };
        let dist = Self::dist(layout);
        let heap_x = NEST_X + side * dist;
        let extra = format!(r#"Heap(material: "provisions", x: {heap_x}, cells: {}),"#, self.heap);
        let (mut w, _, _) = bed(layout.seed, self.ants, &extra).build();
        let sp = clone_colony(&mut w, genomes[0], self.game_walk, layout.blind);
        let food = w.materials.id_of("provisions").expect("provisions");
        let before = cells_of(&w, heap_x - 20, heap_x + 20, food);
        let started = ants_of(&w, sp).len().max(1);
        for f in 0..self.frames {
            frame(&mut w, true);
            look(f + 1, &w);
        }
        let home = cells_of(&w, NEST_X - 12, NEST_X + 12, food) as f64 + w.creature_stats.digested_at_nest_face / 960.0;
        let alive = ants_of(&w, sp).len();
        let kept = (alive as f64 / started as f64).min(1.0);
        vec![Score {
            fit: home * kept,
            parts: vec![
                home,
                alive as f64,
                (before - cells_of(&w, heap_x - 20, heap_x + 20, food)) as f64,
                w.creature_stats.pickups as f64,
                dist as f64,
            ],
        }]
    }
}

/// **Take the trail, not the bare ground** (per ant).
///
/// A bare stone floor 256 wide, the ants standing three cells apart around
/// the centre. A channel-B trail runs from a few cells behind the group, so
/// every ant starts on it, out towards the world's edge on one side only
/// (east on even layouts, west on odd), rising to its peak. Where it starts
/// and ends, its peak and its slope vary with the layout's seed.
/// The other side is bare ground. That is the choice a side-view ant meets at
/// its door: left or right, and the trail says which. Nothing decays, nothing
/// is eaten, nobody starves, and every ant's `EmitA`/`EmitB` rows are banned,
/// so the only trail is the stamped one.
///
/// Score per ant: the mean over the run of how far along the trail it stood
/// (0 at its start or on the bare side, 1 at the far end). An ant that
/// always walks one way scores well on half the layouts and nothing on the
/// other half; only reading the trail scores on both.
///
/// **The first version started the trail twenty cells off centre**, so most
/// ants could not sense it from where they stood and the choice was a coin:
/// three runs from scratch reached 0.5 in training (one-way walkers on a
/// lucky pair of layouts) and 0.01-0.08 on hold-out.
pub struct Trail {
    pub ants: usize,
    pub frames: u64,
    /// Select whole arenas of clones (`Unit::Colony`) instead of single ants
    /// in mixed arenas. The hold-out always scores clones, so with this set
    /// training and hold-out measure the same thing; without it a champion
    /// trained among strangers is judged among copies of itself, which on a
    /// one-lane floor block each other where they all hesitate (design
    /// review of 2026-10-17, finding 1).
    pub clones: bool,
}

const TRAIL_BANNED: &[O] = &[O::Impulse, O::Fly, O::Attack, O::Provision, O::Lay, O::EmitA, O::EmitB];

impl Challenge for Trail {
    fn name(&self) -> &'static str {
        "trail"
    }
    fn unit(&self) -> Unit {
        if self.clones {
            Unit::Colony
        } else {
            Unit::Ant
        }
    }
    fn ants(&self) -> usize {
        self.ants
    }
    fn frames(&self) -> u64 {
        self.frames
    }
    fn parts(&self) -> &'static [&'static str] {
        &["along", "reach", "end", "walked"]
    }
    fn banned(&self) -> &'static [O] {
        TRAIL_BANNED
    }
    fn run(&self, layout: Layout, genomes: &[&[f32]], look: &mut dyn FnMut(u64, &World)) -> Vec<Score> {
        if self.clones {
            let gs = vec![genomes[0]; self.ants];
            return vec![super::mean_score(&self.run_ants(layout, &gs, look))];
        }
        self.run_ants(layout, genomes, look)
    }
}

impl Trail {
    fn run_ants(&self, layout: Layout, genomes: &[&[f32]], look: &mut dyn FnMut(u64, &World)) -> Vec<Score> {
        let (ww, h) = (BED_W, 64);
        let mut w = World::new(Rect::new(0, 0, ww - 1, h - 1));
        w.seed = layout.seed;
        let floor = h - 8;
        let head_y = floor - 1;
        for x in 0..ww {
            for y in floor..h {
                w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        let sp = w.species.id_of("ant").expect("ant");
        let mut def = w.species.get(sp).creature.clone().expect("creature");
        def.reproduce_threshold = 1.0e30;
        def.start_energy = 1.0e9;
        if layout.blind {
            def.home_bias = 0.0;
        }
        w.species.set_creature(sp, def);
        w.chooser = Some(creature::Chooser::Off);

        let mid = ww / 2;
        let dir = if layout.k.is_multiple_of(2) { 1 } else { -1 };
        // **The geometry varies with the seed**, so a hold-out layout is a
        // trail training has not seen and not only new dice on the same two
        // arenas (coordinator review of PR 707): how far behind the group the
        // trail starts (always under it, so every ant can sense it), how far
        // short of the edge it ends, its strength and its slope.
        let mut geo = super::Rng(layout.seed ^ 0x7EA1_1A70);
        let behind = 1 + geo.below(8) as i32;
        let short = 3 + geo.below(40) as i32;
        // Half the plane's range at the peak, give or take 30%, so the
        // brain's `PheroBFront` (raw / `Scent::MAX`) reads about 0.05-0.65 on
        // the trail. The first version stamped 40 x `SCALE`, which reads
        // 0.03-0.16: a hand-wired follower whose `Move` was gated on it never
        // took a step.
        let peak = pheromone::Scent::MAX as f32 * (0.35 + 0.3 * geo.unit());
        let base = 0.1 + 0.3 * geo.unit();
        let (near, far) = (mid - dir * (3 * genomes.len() as i32 / 2 + behind), if dir > 0 { ww - 1 - short } else { short });
        let span = (far - near).abs() as f32;
        for i in 0..=(far - near).abs() {
            let x = near + dir * i;
            let t = i as f32 / span;
            let amount = (base + (1.0 - base) * t) * peak;
            for y in (head_y - 3)..=(head_y + 1) {
                w.deposit_pheromone(Channel::B, x, y, amount as pheromone::Scent);
            }
        }

        // Plant, then step once so the seeds become animals, then hand each
        // its genome in a fixed order.
        let n = genomes.len();
        for i in 0..n {
            // Three cells apart: closer, and two seeds share a body's cells
            // and only one becomes an animal.
            let x = mid - 3 * n as i32 / 2 + 3 * i as i32;
            w.plant_ant(x, head_y);
        }
        frame(&mut w, false);
        let ids = ants_of(&w, sp);
        assert_eq!(ids.len(), n, "every planted ant became an animal");
        let mut start = vec![0i32; n];
        for (k, &id) in ids.iter().enumerate() {
            w.set_organism_genome(id, genomes[k].to_vec());
            start[k] = w.organism(id).and_then(|s| s.chain.first().copied()).map_or(mid, |p| p.0);
        }
        let progress = |x: i32, k: usize| -> f64 { (((x - start[k]) * dir) as f64 / (far - start[k]).abs().max(1) as f64).clamp(0.0, 1.0) };
        let mut sum = vec![0.0f64; n];
        let mut reach = vec![0.0f64; n];
        let mut last = start.clone();
        let mut walked = vec![0i64; n];
        for f in 1..self.frames {
            frame(&mut w, false);
            for (k, &id) in ids.iter().enumerate() {
                let Some(x) = w.organism(id).and_then(|s| s.chain.first().copied()).map(|p| p.0) else { continue };
                let p = progress(x, k);
                sum[k] += p;
                reach[k] = reach[k].max(p);
                walked[k] += (x - last[k]).abs() as i64;
                last[k] = x;
            }
            look(f + 1, &w);
        }
        (0..n)
            .map(|k| {
                let along = sum[k] / (self.frames - 1) as f64;
                Score { fit: along, parts: vec![along, reach[k], progress(last[k], k), walked[k] as f64] }
            })
            .collect()
    }
}

/// **Dig a nest** (colony).
///
/// The lab bed with the colony fed for ever (every ant's energy topped back
/// to full every 50 frames, so nobody starves), so the arena asks only about
/// digging. Score: roofed room -- empty cells with ground over
/// them, standing or occupied -- gained since the first frame, times the
/// fraction of the colony alive. A pit open to the sky is not a nest and
/// does not count (`CLAUDE.md`'s excavation trap); the mound and the pit are
/// printed beside it so a colony that only quarries shows as one.
pub struct Dig {
    pub ants: usize,
    pub frames: u64,
    pub game_walk: bool,
}

impl Challenge for Dig {
    fn name(&self) -> &'static str {
        "dig"
    }
    fn unit(&self) -> Unit {
        Unit::Colony
    }
    fn ants(&self) -> usize {
        self.ants
    }
    fn frames(&self) -> u64 {
        self.frames
    }
    fn parts(&self) -> &'static [&'static str] {
        &["room", "pit", "mound", "alive"]
    }
    fn banned(&self) -> &'static [O] {
        NOT_ANT_VERBS
    }
    fn run(&self, layout: Layout, genomes: &[&[f32]], look: &mut dyn FnMut(u64, &World)) -> Vec<Score> {
        let sc = bed(layout.seed, self.ants, "");
        let (mut w, _, _) = sc.build();
        let sp = clone_colony(&mut w, genomes[0], self.game_walk, layout.blind);
        // **Fed by topping up, not by raising `start_energy`.** The ants are
        // already placed when the arena gets them, so a raised
        // `start_energy` leaves their energy where it was and moves the
        // `Energy` sense (`energy / start_energy`) to zero: every ant reads
        // as starving, and the first run of this arena lost all twelve.
        let full = w.species.get(sp).creature.as_ref().map_or(1.0, |d| d.start_energy);
        let ids = Ids::resolve(&w);
        let started = ants_of(&w, sp).len().max(1);
        let cols = [NEST_X];
        frame(&mut w, true);
        let base = census::census(&w, &sc.bed, 0.0, &cols, &ids);
        look(1, &w);
        for f in 1..self.frames {
            frame(&mut w, true);
            if f % 50 == 0 {
                for id in ants_of(&w, sp) {
                    w.set_organism_energy(id, full);
                }
            }
            look(f + 1, &w);
        }
        let end = census::census(&w, &sc.bed, 0.0, &cols, &ids);
        let room = end.roofed_room() as f64 - base.roofed_room() as f64;
        let alive = ants_of(&w, sp).len();
        let kept = (alive as f64 / started as f64).min(1.0);
        vec![Score {
            fit: room.max(0.0) * kept,
            parts: vec![room, end.pit as f64 - base.pit as f64, end.mound as f64, alive as f64],
        }]
    }
}

/// **One brain, several jobs** (colony): every arena in the suite, scored
/// together, so training on a new task cannot quietly cost an old one.
///
/// The owner's question that made this (2026-10-17): if every task trains
/// its own ant, the result is a drawer of specialists rather than one ant
/// that can do everything. So each new task starts from the last champion,
/// and the suite keeps the earlier arenas in every generation.
///
/// Score: the geometric mean over arenas of `fit / reference`, where each
/// reference is the start genome's own hold-out score there
/// ([`Suite::calibrate`]). A product rather than a sum, so a brain that is
/// excellent at one job and useless at another scores near zero; and an
/// arena marked `keep` (an old skill) whose ratio falls under `floor`
/// multiplies the whole score by a tenth, so forgetting is never worth it.
/// Per-ant arenas are run with every ant a clone.
pub struct Suite {
    pub arenas: Vec<(Box<dyn Challenge>, bool)>,
    pub refs: Vec<f64>,
    pub floor: f64,
    names: &'static [&'static str],
    banned: &'static [O],
}

impl Suite {
    /// `arenas`: each challenge, and whether it is an old skill to keep.
    pub fn new(arenas: Vec<(Box<dyn Challenge>, bool)>, floor: f64) -> Self {
        let mut names: Vec<&'static str> = vec![];
        for (a, _) in &arenas {
            names.push(Box::leak(format!("{}_ratio", a.name()).into_boxed_str()));
        }
        for (a, _) in &arenas {
            names.push(Box::leak(format!("{}_fit", a.name()).into_boxed_str()));
        }
        // Banned everywhere if banned anywhere: one brain has one set of verbs.
        let mut banned: Vec<O> = vec![];
        for (a, _) in &arenas {
            for &b in a.banned() {
                if !banned.contains(&b) {
                    banned.push(b);
                }
            }
        }
        let refs = vec![1.0; arenas.len()];
        Suite { arenas, refs, floor, names: Box::leak(names.into_boxed_slice()), banned: Box::leak(banned.into_boxed_slice()) }
    }
}

impl Challenge for Suite {
    fn name(&self) -> &'static str {
        "suite"
    }
    fn unit(&self) -> Unit {
        Unit::Colony
    }
    fn ants(&self) -> usize {
        self.arenas.iter().map(|(a, _)| a.ants()).max().unwrap_or(1)
    }
    fn frames(&self) -> u64 {
        // Pictures show the first arena.
        self.arenas.first().map_or(1, |(a, _)| a.frames())
    }
    fn parts(&self) -> &'static [&'static str] {
        self.names
    }
    fn banned(&self) -> &'static [O] {
        self.banned
    }
    fn run(&self, layout: Layout, genomes: &[&[f32]], look: &mut dyn FnMut(u64, &World)) -> Vec<Score> {
        let g = genomes[0];
        let mut ratios = vec![];
        let mut fits = vec![];
        let mut log_sum = 0.0;
        let mut forgot = false;
        for (i, (a, keep)) in self.arenas.iter().enumerate() {
            let gs: Vec<&[f32]> = vec![g; if a.unit() == Unit::Ant { a.ants() } else { 1 }];
            let mut quiet = |_: u64, _: &World| {};
            let watcher: &mut dyn FnMut(u64, &World) = if i == 0 { &mut *look } else { &mut quiet };
            let s = super::mean_score(&a.run(layout, &gs, watcher));
            let r = s.fit.max(0.0) / self.refs[i];
            if *keep && r < self.floor {
                forgot = true;
            }
            log_sum += r.max(1e-3).ln();
            ratios.push(r);
            fits.push(s.fit);
        }
        let mut fit = (log_sum / self.arenas.len() as f64).exp();
        if forgot {
            fit *= 0.1;
        }
        ratios.extend(fits);
        vec![Score { fit, parts: ratios }]
    }

    /// Set every arena's reference to `g`'s hold-out score there (floored
    /// at a small positive number, so an arena the start cannot do at all
    /// pays for any progress rather than dividing by zero).
    fn calibrate(&mut self, g: &[f32], layouts: &[Layout], threads: usize) {
        for (i, (a, _)) in self.arenas.iter().enumerate() {
            let s = super::score_clones(a.as_ref(), g, layouts, threads);
            self.refs[i] = s.fit.max(0.05);
            println!("suite reference {}: {:.3}", a.name(), self.refs[i]);
        }
    }
}
