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

/// Deaths of old age in one colony. **Left out of every survival factor**:
/// a founder that dies of age in the run would have died under any brain,
/// and on the dig arena every death in 20,000 frames was old age, the zero
/// genome's included (`ladder/probes/arena-zero-death-causes.log`).
/// Counted for the whole colony, so an ant that hatched in the run and died
/// of age in it would count too; a hatchling starts younger than every
/// founder, so it is the last to go.
fn old_age(w: &World, species: SpeciesId, colony: u32) -> usize {
    w.group_deaths_of(species, colony).map_or(0, |g| g.by_cause[crate::sim::organism::DeathCause::OldAge.index()] as usize)
}

const BED_W: i32 = 256;
const NEST_X: i32 = 128;
const GROUND_Y: i32 = 64;

/// The lab bed every colony arena uses: 256 x 128, 40 rows of soil, no rain,
/// one colony at the centre, and whatever `extra` placements the arena adds,
/// placed before the colony so a buried lens or a raised step is already
/// there when it digs its founding shaft.
fn bed(seed: u64, ants: usize, extra: &str) -> Scenario {
    bed_shaped(seed, ants, extra, 40, 33)
}

/// [`bed`] with its soil depth and lamp spacing given, for the dig arena's
/// varied beds ([`DigBed`]).
fn bed_shaped(seed: u64, ants: usize, extra: &str, soil_depth: i32, lamp_spacing: i32) -> Scenario {
    let text = format!(
        r#"(
    name: "train", title: "TRAIN", question: "training arena",
    bed: (width: {BED_W}, height: 128, soil_depth: {soil_depth}, ground_y: {GROUND_Y}, compartments: 1, founders: 0,
          species: "grass", colonies: 0, colony_ants: {ants}, colony_species: "ant", predators: 0,
          lamp_spacing: {lamp_spacing}, seed: {seed}, extra_walls: []),
    settings: [(subject: "the bed", field: "rain", value: 0)],
    placements: [
        {extra}
        Colony(species: "ant", x: {NEST_X}, count: {ants}),
    ],
    timeline: [],
    horizon: 100000,
)"#
    );
    ron::from_str(&text).expect("training bed parses")
}

/// Every ant in a fresh bed takes `genome`'s brain, and the world its target
/// scorer if it has one ([`super::SCORER`]); births are put out of reach, and
/// the brain steers (`Chooser::Off`) unless `game_walk`. A `blind` layout
/// also cuts the body's homeward re-roll and forces the brain walk, so the
/// senses-cut control is blind outside the brain too ([`Layout::blind`]); a
/// `legal` one draws the scorer's targets evenly ([`Layout::legal`]).
fn clone_colony(w: &mut World, genome: &[f32], game_walk: bool, layout: Layout) -> SpeciesId {
    let blind = layout.blind;
    let sp = w.species.id_of("ant").expect("ant");
    let mut def = w.species.get(sp).creature.clone().expect("creature");
    def.reproduce_threshold = 1.0e30;
    if blind {
        def.home_bias = 0.0;
    }
    w.species.set_creature(sp, def);
    let (brain, scorer) = genome.split_at(genome.len().min(super::SCORER));
    w.species.set_genome(sp, brain.to_vec());
    if !game_walk || blind {
        w.chooser = Some(creature::Chooser::Off);
    }
    w.home_blind = blind;
    if !scorer.is_empty() {
        let mut t = creature::targets::Targets::new(sp, scorer);
        t.even = layout.legal;
        w.targets = Some(t);
    }
    for id in ants_of(w, sp) {
        w.set_organism_genome(id, brain.to_vec());
    }
    sp
}

/// **Bring food home, and stay alive doing it** (colony).
///
/// A heap of provisions ([`FORAGE_DIST`] columns from the nest, east on even
/// layouts and west on odd ones, on one seed per pair, so a pair differs only
/// by the side and the score is the worse side ([`super::fold`]). Score: food
/// brought home, in provisions cells -- cells lying within 12 columns of the
/// nest at the end, crop food digested at the nest, and food that went into
/// larvae -- less every corpse meal the colony ate, times the fraction of the
/// **founders** still alive, old age excepted ([`old_age`]).
///
/// **The heap is 300 cells, not 60**: at 60 the best trained colony emptied
/// all six of its eastern heaps, after which the score measured losses on
/// the way rather than foraging, and the best arms could not be told apart
/// (`ladder/rungs.md`). The survival factor is
/// there because the probe's walking-only winner brought food home while the
/// colony shrank from 12 to 8; the hop ban because its unbanned winner flew
/// it home.
///
/// **Founders, not heads.** The founding colony carries brood, and brood
/// that hatches during the run lifted a plain head count past the starting
/// 12: the game walk ended at 16.75 alive, so four deaths could have hidden
/// under four hatchings (`design/review-rev2.md` N10). Counting the ants
/// that were there at the first frame charges every death.
///
/// **Food into larvae counts as food home.** The game walk took as much from
/// the heap as the first trained colony (30.1 cells each) and scored far
/// less `home`, because it fed brood and the score read only adults' meals
/// and the floor. A larva is fed from a carrier's crop (`brood_crop_fed_j`)
/// or eats food lying beside it (`brood_ate_j`); neither passes through an
/// adult's digestion, so neither is already in `digested_at_nest_face`. A
/// nurse's or a `Share` from its own bank (`brood_nursed_j`,
/// `brood_shared_j`) is left out: that food was counted when the nurse
/// digested it. Both larval counters are joules **kept**, so they are put
/// back into face value by the colony's own keep for the run (adult joules
/// kept over adult face digested), then into cells at provisions' 960 J.
///
/// **Corpse meals are taken off**, wherever they were eaten: a colony that
/// eats its dead, or its starved larvae, has not foraged. The adult face
/// count carries corpse meals at the nest. The corpse joules are put back
/// into face value by the same keep, and that keep reads the colony's whole
/// intake, which holds meals that never reach `digested_face` (nectar, a
/// meal toward a birth), so it runs high and the corpse taken off runs a
/// little low: this errs **for** the colony. It is bounded by the clamp at
/// 1 and near nothing with births off (coordinator review of PR 708).
pub struct Forage {
    pub ants: usize,
    pub frames: u64,
    pub heap: i32,
    pub game_walk: bool,
}

/// The heap's distance from the nest, in columns. **Set by the heap's own
/// width, not chosen**: provisions rest at 26 degrees, so a heap of 300
/// slumps to about 25 columns either side of its peak. Nearer than 45 its
/// toe reaches the 12 columns that count as home, and the colony scores
/// food it never carried; farther than 95 it runs into the box wall.
/// `the_heap_lies_clear_of_home_and_the_wall` measures both ends. The
/// ladder lane asked for 30-120 (`ladder/rungs.md`), which fits a heap of
/// 60 and not one of 300.
const FORAGE_DIST: (i32, i32) = (45, 95);

/// Columns either side of the heap's peak that the `taken` count reads: a
/// heap of 300 at rest spans about 25.
const HEAP_REACH: i32 = 40;

impl Forage {
    fn dist(layout: Layout) -> i32 {
        let (lo, hi) = FORAGE_DIST;
        lo + ((layout.seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 32) % (hi - lo + 1) as u64) as i32
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
        &["home", "alive", "taken", "pickups", "dist", "founders_kept", "floor", "adult", "larvae", "corpse"]
    }
    fn banned(&self) -> &'static [O] {
        NOT_ANT_VERBS
    }
    fn mirrored(&self) -> bool {
        true
    }
    fn run(&self, layout: Layout, genomes: &[&[f32]], look: &mut dyn FnMut(u64, &World)) -> Vec<Score> {
        let side = if layout.k.is_multiple_of(2) { 1 } else { -1 };
        let dist = Self::dist(layout);
        let heap_x = NEST_X + side * dist;
        let extra = format!(r#"Heap(material: "provisions", x: {heap_x}, cells: {}),"#, self.heap);
        let (mut w, _, _) = bed(layout.seed, self.ants, &extra).build();
        let sp = clone_colony(&mut w, genomes[0], self.game_walk, layout);
        let food = w.materials.id_of("provisions").expect("provisions");
        let before = cells_of(&w, heap_x - HEAP_REACH, heap_x + HEAP_REACH, food);
        let founders = ants_of(&w, sp);
        // The colony's number now, while its founders live: a dead id's
        // colony reads 0, whose books are empty, and the first run of this
        // score read no meals at all on the layouts where founder 0 died.
        let colony = founders.first().map_or(0, |&id| w.colony_of(id));
        for f in 0..self.frames {
            frame(&mut w, true);
            look(f + 1, &w);
        }
        let st = &w.creature_stats;
        let books = w.colony_books(colony);
        let corpse_j: f64 = w.materials.id_of("corpse").map_or(0.0, |c| books.diet().iter().filter(|(m, _)| *m == c).map(|(_, j)| j).sum());
        let larvae_j = st.brood_crop_fed_j + st.brood_ate_j;
        // Joules kept per joule of face value, from the adults' own meals.
        let keep = if st.digested_face > 0.0 { ((books.intake() - larvae_j) / st.digested_face).clamp(0.05, 1.0) } else { 1.0 };
        let floor = cells_of(&w, NEST_X - 12, NEST_X + 12, food) as f64;
        let adult = st.digested_at_nest_face / 960.0;
        let larvae = larvae_j / keep / 960.0;
        let corpse = corpse_j / keep / 960.0;
        let home = (floor + adult + larvae - corpse).max(0.0);
        let now = ants_of(&w, sp);
        let alive = now.len();
        let lived = founders.iter().filter(|id| now.binary_search(id).is_ok()).count() + old_age(&w, sp, colony);
        let kept = (lived as f64 / founders.len().max(1) as f64).min(1.0);
        vec![Score {
            fit: home * kept,
            parts: vec![
                home,
                alive as f64,
                (before - cells_of(&w, heap_x - HEAP_REACH, heap_x + HEAP_REACH, food)) as f64,
                w.creature_stats.pickups as f64,
                dist as f64,
                kept,
                floor,
                adult,
                larvae,
                corpse,
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
    fn mirrored(&self) -> bool {
        true
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
        w.home_blind = layout.blind;

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

/// Frames the dig bed runs on after the bell, every ant on the zero
/// genome, before room is counted ([`Dig`]).
pub const DIG_SETTLE: u64 = 2000;

/// **Dig a nest** (colony).
///
/// A lab bed drawn from the layout ([`DigBed`]) with the colony fed for ever
/// (every ant's energy topped back to full every 50 frames, so nobody
/// starves), so the arena asks only about digging. Score: **joined room**
/// -- cells below the ground as it stood at the first frame, roofed by that
/// ground, empty or with an ant in them, and joined to the open air by a
/// path an ant could walk ([`joined_room`]) -- gained since the first frame
/// and counted after [`DIG_SETTLE`] frames with the work stopped, times the
/// fraction of the ants placed that are alive at the bell, old age excepted
/// ([`old_age`]). A pit open to the sky is not a nest and does not count
/// (`CLAUDE.md`'s excavation trap); the mound and the pit are printed beside
/// it so a colony that only quarries shows as one. The bed's one-sided
/// features are mirrored across a pair, and the pair scores its worse side.
///
/// **Why joined, and why roofed by the old ground.** The census's roofed
/// room asks only for ground overhead, of any kind, so a colony could score
/// "room" by heaping spoil over a pit or sealing a pocket it can no longer
/// reach -- the shapes the design review named as the way to fake this
/// arena once policy rules come out (`design/general-brain-design-rev3.md`,
/// *How we will know*). The census's own roofed room is printed beside it
/// as `roofed`, so the gap between the two is the faked part; `spoil_below`
/// is the spoil left in the ground ([`spoil_below`]).
pub struct Dig {
    pub ants: usize,
    pub frames: u64,
    pub game_walk: bool,
    /// **The target-choice arena** (stage 1 of the brain plan): genomes carry
    /// an evolved target scorer, which picks the cell dug and where each
    /// pellet goes down in place of the hand rules (`creature::targets`).
    /// Its counters are printed beside the score: decisions that were a
    /// choice at all, the margin between the two best, pellets left
    /// inside.
    pub targets: bool,
    /// The hand filters the scorer replaces as well, the rung of the
    /// one-rule-at-a-time ladder ([`creature::targets::Free`]).
    pub free: creature::targets::Free,
}

const DIG_PARTS: &[&str] = &["room", "pit", "mound", "alive", "roofed", "spoil_below", "placed", "holding"];
const DIG_TARGET_PARTS: &[&str] = &[
    "room", "pit", "mound", "alive", "roofed", "spoil_below", "placed", "holding", "dig_choices", "dig_margin", "dig_none", "drop_choices", "drop_margin", "drop_none", "dropped_inside", "dropped_outside",
];

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
        if self.targets {
            DIG_TARGET_PARTS
        } else {
            DIG_PARTS
        }
    }
    fn scorer(&self) -> bool {
        self.targets
    }
    fn banned(&self) -> &'static [O] {
        NOT_ANT_VERBS
    }
    fn mirrored(&self) -> bool {
        true
    }
    fn run(&self, layout: Layout, genomes: &[&[f32]], look: &mut dyn FnMut(u64, &World)) -> Vec<Score> {
        let shape = DigBed::of(layout);
        let sc = bed_shaped(layout.seed, self.ants, &shape.placements(), shape.depth, shape.lamp);
        let (mut w, _, _) = sc.build();
        let sp = clone_colony(&mut w, genomes[0], self.game_walk, layout);
        if let Some(t) = w.targets.as_mut() {
            t.free = self.free;
        }
        // **Fed by topping up, not by raising `start_energy`.** The ants are
        // already placed when the arena gets them, so a raised
        // `start_energy` leaves their energy where it was and moves the
        // `Energy` sense (`energy / start_energy`) to zero: every ant reads
        // as starving, and the first run of this arena lost all twelve.
        let full = w.species.get(sp).creature.as_ref().map_or(1.0, |d| d.start_energy);
        let ids = Ids::resolve(&w);
        let placed = ants_of(&w, sp);
        let colony = placed.first().map_or(0, |&id| w.colony_of(id));
        let cols = [NEST_X];
        let feed = |w: &mut World| {
            for id in ants_of(w, sp) {
                w.set_organism_energy(id, full);
            }
        };
        frame(&mut w, true);
        let base = census::census(&w, &sc.bed, 0.0, &cols, &ids);
        let base_joined = joined_room(&w, &sc.bed);
        look(1, &w);
        for f in 1..self.frames {
            frame(&mut w, true);
            if f % 50 == 0 {
                feed(&mut w);
            }
            look(f + 1, &w);
        }
        let alive = ants_of(&w, sp).len();
        let lived = alive + old_age(&w, sp, colony);
        // Read at the bell, before the settle: a carrier still laden then
        // could not dig, and a scorer that keeps saying "not yet" leaves its
        // carriers laden (ladder lane's stage 1 review, finding 7).
        let holding = ants_of(&w, sp).into_iter().filter(|&id| w.organism(id).is_some_and(|s| s.spoil.is_some())).count();
        let t = w.targets.as_ref().map(|t| t.stats.clone()).unwrap_or_default();
        // **Settle before the count**: every ant takes the zero genome,
        // which digs nothing, and the bed runs on, so what is counted is
        // what stands once the work stops rather than what is standing at
        // the bell. On the game ant it changed nothing on 4 of 4 beds
        // (`ladder/dropin-nest-room.md`).
        let zero = vec![0.0; genomes[0].len().min(super::SCORER)];
        w.chooser = Some(creature::Chooser::Off);
        for id in ants_of(&w, sp) {
            w.set_organism_genome(id, zero.clone());
        }
        for f in 0..DIG_SETTLE {
            frame(&mut w, true);
            if f % 50 == 0 {
                feed(&mut w);
            }
        }
        let end = census::census(&w, &sc.bed, 0.0, &cols, &ids);
        let room = joined_room(&w, &sc.bed) as f64 - base_joined as f64;
        let roofed = end.roofed_room() as f64 - base.roofed_room() as f64;
        let kept = (lived as f64 / placed.len().max(1) as f64).min(1.0);
        let mut parts = vec![room, end.pit as f64 - base.pit as f64, end.mound as f64, alive as f64, roofed, spoil_below(&w, &sc.bed) as f64, placed.len() as f64, holding as f64];
        if self.targets {
            let (d, p) = (creature::targets::VERB_DIG, creature::targets::VERB_DROP);
            let margin = |v: usize| if t.choices[v] > 0 { t.margin_sum[v] / t.choices[v] as f64 } else { 0.0 };
            parts.extend([
                t.choices[d] as f64,
                margin(d),
                t.none[d] as f64,
                t.choices[p] as f64,
                margin(p),
                t.none[p] as f64,
                t.dropped_inside as f64,
                t.dropped_outside as f64,
            ]);
        }
        vec![Score { fit: room.max(0.0) * kept, parts }]
    }
}

/// **Spoil left in the ground**: loose `spoil` cells at or below the frozen
/// ground datum -- a pellet put down inside the workings rather than carried
/// out. Dig's second number: room is net void, so backfill already costs
/// room, but a colony that shuffles spoil about below ground shows here and
/// not in the room count.
fn spoil_below(w: &World, spec: &crate::lab::scene::LabBox) -> usize {
    let Some(spoil) = w.materials.id_of("spoil") else { return 0 };
    let (xs, ys) = census::extents(w, spec);
    let mut n = 0;
    for x in xs {
        let surf = census::surface_of(w, spec, x);
        n += ys.clone().filter(|&y| y >= surf && w.get(x, y).material == spoil).count();
    }
    n
}

/// **One dig bed, drawn from the layout** (ladder lane, `ladder/dig-layouts.md`).
///
/// Every dig hold-out was the same flat bed with new dice, so a scorer could
/// learn the bed rather than read it: "the floor is at row 104". Now the
/// soil depth (24-52 rows), the lamp spacing (25, 33 or 41), a buried stone
/// lens on 3 layouts in 4, and a raised step in the surface vary, every
/// draw a hash of the seed and the feature. The lens and the step lie on
/// one side of the nest, and an odd `k` mirrors them across it, so a pair
/// differs only by the side and is scored by its worse side.
///
/// **Every bed has at least one one-sided feature**: the step is forced on
/// a bed that drew no lens. The probe's one bed with neither gave two
/// identical runs to the cell, a pair that tests nothing about a one-way
/// habit.
///
/// On the probe's four beds the game ant gained 66-79 cells of room (east
/// and west within 4 cells of each other) and the zero genome 0.
#[derive(Clone, Debug, PartialEq)]
pub struct DigBed {
    pub depth: i32,
    pub lamp: i32,
    /// `(x, y, w, h)` of the buried stone.
    pub lens: Option<(i32, i32, i32, i32)>,
    /// `(x, up)`: the ground is `up` rows higher from column `x` out to the
    /// wall on the bed's side.
    pub step: Option<(i32, i32)>,
    /// `1` east, `-1` west.
    pub side: i32,
}

/// The box's inside edges on [`BED_W`], for the step's fill.
const BED_INSIDE: (i32, i32) = (6, BED_W - 6);

impl DigBed {
    pub fn of(layout: Layout) -> DigBed {
        let seed = layout.seed;
        let draw = |salt: u64, lo: i32, hi: i32| {
            let mut z = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ salt.wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            lo + ((z ^ (z >> 31)) % (hi - lo + 1) as u64) as i32
        };
        let side = if layout.k.is_multiple_of(2) { 1 } else { -1 };
        let depth = draw(1, 24, 52);
        let lens = (draw(2, 0, 3) > 0).then(|| {
            let w = draw(3, 8, 30);
            let off = draw(4, 4, 40);
            let h = draw(6, 2, 5);
            let y = (GROUND_Y + draw(5, 8, (depth - 6).max(9))).min(GROUND_Y + depth - h - 1);
            (if side > 0 { NEST_X + off } else { NEST_X - off - w }, y, w, h)
        });
        let step = (lens.is_none() || draw(7, 0, 2) > 0).then(|| (NEST_X + side * draw(8, 6, 30), draw(9, 2, 6)));
        DigBed { depth, lamp: [25, 33, 41][draw(10, 0, 2) as usize], lens, step, side }
    }

    /// The bed's extra placements, in the scenario's own `Fill`s.
    fn placements(&self) -> String {
        let mut extra = String::new();
        if let Some((x, y, w, h)) = self.lens {
            extra.push_str(&format!(r#"Fill(material: "stone", x: {x}, y: {y}, w: {w}, h: {h}),"#));
        }
        if let Some((at, up)) = self.step {
            let (x, w) = if self.side > 0 { (at, BED_INSIDE.1 - at) } else { (BED_INSIDE.0, at - BED_INSIDE.0) };
            extra.push_str(&format!(r#"Fill(material: "soil", x: {x}, y: {}, w: {w}, h: {up}),"#, GROUND_Y - up));
        }
        extra
    }
}

/// **Room an ant can walk into**: cells at or below the frozen ground datum
/// ([`census::surface_of`]), roofed by the original bed -- ground at or
/// below the datum in the cell's own column, above it, and not a spoil
/// pellet -- that are empty or hold an animal, and are joined to the sky by
/// a path of such cells. Eight neighbours, because an ant steps in eight
/// directions (`src/sim/`'s rule: a traversal uses the writer's
/// neighbourhood).
///
/// **Two lids the first version paid for**, both shapes that stand (ladder
/// lane, `ladder/dropin-nest-room.md` and `review-ladder-docs.md` s1; a
/// one-row lid over a ten-wide pit falls in by itself). It seeded the path
/// from every open cell above the datum, so a nest capped at its mouth by a
/// lid one row up read as joined through the air pocket under the lid (40
/// cells); now the seed is the sky, the rows above [`census::MOUND_REACH`]
/// that no roof can reach. And it took the census's roof, any ground within
/// `MOUND_REACH` rows, so a packed lid two rows thick over a pit, left with
/// one gap, counted the pit below it (72); now the roof must be the old
/// ground. **Where, not what**: digging under a heap packs its bottom row
/// into packed soil, the material a wall lined in place is made of, so
/// packed soil above the datum is a lid and below it a roof. On the rig's own arena it costs an honest nest nothing: 946 cells
/// either way over the game ant's 12 hold-out layouts
/// (`ladder/review-ladder-docs.md` s2).
pub fn joined_room(w: &World, spec: &crate::lab::scene::LabBox) -> usize {
    let (xs, ys) = census::extents(w, spec);
    let (x0, y0) = (*xs.start(), *ys.start());
    let (nw, nh) = ((xs.end() - x0 + 1) as usize, (ys.end() - y0 + 1) as usize);
    let at = |x: i32, y: i32| (y - y0) as usize * nw + (x - x0) as usize;
    let open = |x: i32, y: i32| {
        let m = w.get(x, y).material;
        m == material::EMPTY || w.materials.kind(m) == material::MaterialKind::Creature
    };
    let ground = |x: i32, y: i32| {
        let c = w.get(x, y);
        c.material != material::EMPTY && matches!(w.materials.kind(c.material), material::MaterialKind::Powder | material::MaterialKind::Solid) && c.organism_id() == 0
    };
    let mut seen = vec![false; nw * nh];
    let mut stack = Vec::new();
    let surface: Vec<i32> = xs.clone().map(|x| census::surface_of(w, spec, x)).collect();
    for x in xs.clone() {
        for y in ys.clone() {
            if y < surface[(x - x0) as usize] - census::MOUND_REACH && open(x, y) {
                seen[at(x, y)] = true;
                stack.push((x, y));
            }
        }
    }
    while let Some((x, y)) = stack.pop() {
        for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
            let (nx, ny) = (x + dx, y + dy);
            if !xs.contains(&nx) || !ys.contains(&ny) || seen[at(nx, ny)] || !open(nx, ny) {
                continue;
            }
            seen[at(nx, ny)] = true;
            stack.push((nx, ny));
        }
    }
    let spoil = w.materials.id_of("spoil");
    let mut n = 0;
    for x in xs.clone() {
        let surf = surface[(x - x0) as usize];
        let mut roofed = false;
        for y in ys.clone() {
            if y >= surf && ground(x, y) && Some(w.get(x, y).material) != spoil {
                roofed = true;
            } else if y >= surf && roofed && seen[at(x, y)] {
                n += 1;
            }
        }
    }
    n
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
    /// Worse side of a pair if any arena has sides: one brain that does one
    /// arena's job on one side only has not learned the job.
    fn mirrored(&self) -> bool {
        self.arenas.iter().any(|(a, _)| a.mirrored())
    }
    fn scorer(&self) -> bool {
        self.arenas.iter().any(|(a, _)| a.scorer())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::scene::LabBox;

    fn bare_bed() -> (World, LabBox) {
        let spec = LabBox { founders: 0, colonies: 0, width: 256, height: 320, ..LabBox::default() };
        let mut world = spec.build();
        world.freeze_room_datum();
        (world, spec)
    }

    /// **A sealed pocket is roofed room and not a nest; open a way to it and
    /// it is.** The census counts the 3x3 pocket either way, which is the
    /// gap `joined_room` closes: before the shaft, census 9 and joined 0;
    /// after it, joined equals the census's roofed room. The shaft meets
    /// the pocket at a corner only, so the join is also the eight-neighbour
    /// step an ant takes.
    #[test]
    fn room_counts_only_once_it_is_joined_to_the_air() {
        let (mut w, spec) = bare_bed();
        let ids = Ids::resolve(&w);
        let (cx, cy) = (spec.width / 2, spec.ground_y + 10);
        for dy in 0..3 {
            for dx in 0..3 {
                w.set(cx + dx, cy + dy, Cell::EMPTY);
            }
        }
        let census = |w: &World| census::census(w, &spec, 0.0, &[cx], &ids).roofed_room();
        assert_eq!(census(&w), 9);
        assert_eq!(joined_room(&w, &spec), 0, "a pocket nothing can walk into is not room");
        // A shaft from the open air down to the cell diagonal from the
        // pocket's top-left corner.
        let surface = census::surface_of(&w, &spec, cx - 1);
        for y in surface..cy {
            w.set(cx - 1, y, Cell::EMPTY);
        }
        assert_eq!(joined_room(&w, &spec), census(&w), "joined through a corner, every roofed cell is room");
        assert_eq!(joined_room(&w, &spec), 9, "the shaft itself is open to the sky, so a pit, not room");
    }

    /// **A lid is not a roof, and a sealed nest is not room.** The shapes
    /// that stand: a one-row lid over a ten-wide pit falls in by itself
    /// within 50 frames, lined or not, so the cases here are the ones the
    /// review found still standing after 3,000 (ladder lane,
    /// `ladder/review-ladder-docs.md` s1). Each has its first-version count,
    /// and each can fail for one of [`joined_room`]'s two rules alone:
    /// * a packed lid two rows thick, one row up, with a one-cell gap: the
    ///   pit is joined to the air, so only the roof rule closes it (72 -> 0);
    /// * a shaft-and-gallery nest capped at the mouth by a two-row packed
    ///   lid one row up: the roof rule alone still counts the gallery, joined
    ///   through the air pocket under the lid, so only the sky seed closes it
    ///   (40, roof rule alone 24, -> 0).
    ///
    /// The census's own roof test counts both, the positive control that
    /// the shapes are there to refuse. Beside them, an honest shaft and
    /// gallery keeps its 24 cells, and a 10x8 chamber under three rows of
    /// the old ground keeps its 72 (the 8 under its way in are not roofed).
    #[test]
    fn a_lid_is_not_a_roof() {
        let clear = |w: &mut World, x: i32, y: i32, wd: i32, h: i32| {
            for xx in x..x + wd {
                for yy in y..y + h {
                    w.set(xx, yy, Cell::EMPTY);
                }
            }
        };
        let lay = |w: &mut World, x: i32, y: i32, wd: i32, h: i32| {
            let m = w.materials.id_of("packedsoil").expect("packedsoil");
            for xx in x..x + wd {
                for yy in y..y + h {
                    w.set(xx, yy, Cell::new(m, 0));
                }
            }
        };
        type Shape = fn(&mut World, i32, i32, &dyn Fn(&mut World, i32, i32, i32, i32), &dyn Fn(&mut World, i32, i32, i32, i32));
        let refused: [(&str, Shape); 2] = [
            ("two-row packed lid one row up, one gap", |w, x, s, clear, lay| {
                clear(w, x, s, 10, 8);
                lay(w, x - 2, s - 3, 14, 2);
                lay(w, x - 2, s - 1, 1, 1);
                lay(w, x + 11, s - 1, 1, 1);
                clear(w, x + 5, s - 3, 1, 2);
            }),
            ("nest capped at the mouth", |w, x, s, clear, lay| {
                clear(w, x, s, 2, 8);
                clear(w, x + 2, s + 6, 12, 2);
                lay(w, x - 2, s - 3, 6, 2);
                lay(w, x - 2, s - 1, 1, 1);
                lay(w, x + 3, s - 1, 1, 1);
            }),
        ];
        for (name, shape) in refused {
            let (mut w, spec) = bare_bed();
            let ids = Ids::resolve(&w);
            let x = spec.width / 2 + 22;
            let s = census::surface_of(&w, &spec, x);
            let census = |w: &World| census::census(w, &spec, 0.0, &[x], &ids).roofed_room();
            let before = census(&w);
            shape(&mut w, x, s, &clear, &lay);
            assert!(census(&w) > before, "{name}: the census does not count it either, so this case tests nothing");
            assert_eq!(joined_room(&w, &spec), 0, "{name}: counted as room");
        }
        let (mut w, spec) = bare_bed();
        let x = spec.width / 2 + 22;
        let s = census::surface_of(&w, &spec, x);
        clear(&mut w, x, s, 2, 8);
        clear(&mut w, x + 2, s + 6, 12, 2);
        assert_eq!(joined_room(&w, &spec), 24, "a shaft down to a 12x2 gallery is 24 cells of nest");
        let (mut w, spec) = bare_bed();
        clear(&mut w, x, s + 3, 10, 8);
        clear(&mut w, x + 5, s, 1, 3);
        assert_eq!(joined_room(&w, &spec), 72, "a 10x8 chamber under three rows of old ground, less the 8 under its way in");
    }

    /// **The dig beds vary, mirror and are never symmetric.** Over 200
    /// seeds: every bed has a lens or a step; the two sides of a pair have
    /// the same depth and lamps and their one-sided features mirrored
    /// across the nest; and the draws cover their ranges, so the generator
    /// is not stuck on one bed.
    #[test]
    fn dig_beds_mirror_and_always_have_a_side() {
        let (mut lens, mut step, mut depths) = (0, 0, std::collections::BTreeSet::new());
        for seed in 0..200 {
            let east = DigBed::of(Layout { seed, k: 0, blind: false, legal: false });
            let west = DigBed::of(Layout { seed, k: 1, blind: false, legal: false });
            assert!(east.lens.is_some() || east.step.is_some(), "seed {seed}: a bed with nothing one-sided");
            assert_eq!((east.depth, east.lamp), (west.depth, west.lamp));
            assert_eq!(east.lens.map(|(x, y, w, h)| (2 * NEST_X - x - w, y, w, h)), west.lens, "seed {seed}: lens not mirrored");
            assert_eq!(east.step.map(|(x, up)| (2 * NEST_X - x, up)), west.step, "seed {seed}: step not mirrored");
            lens += east.lens.is_some() as usize;
            step += east.step.is_some() as usize;
            depths.insert(east.depth);
        }
        assert!((120..=180).contains(&lens), "lens on {lens} of 200 beds, drawn on 3 in 4");
        assert!(step > 120, "step on {step} of 200 beds");
        assert!(depths.len() > 20, "{} distinct soil depths of 29", depths.len());
    }

    /// **The forage heap lies clear of home and of the wall**, at both ends
    /// of [`FORAGE_DIST`] and on both sides: once the 300 cells have slumped,
    /// none lie within the 12 columns that count as home, every one lies
    /// within [`HEAP_REACH`] of the peak, and every one is inside the box.
    /// No ants, so nothing carries any of it. The positive control: at the
    /// 30 columns first asked for, the same heap does reach home.
    #[test]
    fn the_heap_lies_clear_of_home_and_the_wall() {
        let slumped = |dist: i32, side: i32| {
            let heap_x = NEST_X + side * dist;
            let extra = format!(r#"Heap(material: "provisions", x: {heap_x}, cells: 300),"#);
            let (mut w, _, _) = bed(3, 0, &extra).build();
            for _ in 0..2000 {
                frame(&mut w, false);
            }
            (w, heap_x)
        };
        for dist in [FORAGE_DIST.0, FORAGE_DIST.1] {
            for side in [1, -1] {
                let (w, heap_x) = slumped(dist, side);
                let food = w.materials.id_of("provisions").expect("provisions");
                let inside = cells_of(&w, BED_INSIDE.0, BED_INSIDE.1, food);
                assert_eq!(inside, 300, "dist {dist} side {side}: {inside} of 300 cells inside the box");
                assert_eq!(cells_of(&w, NEST_X - 12, NEST_X + 12, food), 0, "dist {dist} side {side}: heap reaches home");
                assert_eq!(cells_of(&w, heap_x - HEAP_REACH, heap_x + HEAP_REACH, food), 300, "dist {dist} side {side}: heap wider than HEAP_REACH");
            }
        }
        let (w, _) = slumped(30, 1);
        let food = w.materials.id_of("provisions").expect("provisions");
        assert!(cells_of(&w, NEST_X - 12, NEST_X + 12, food) > 0, "a heap 30 columns out should reach home; if not, this test cannot see it");
    }

    /// **A blind colony gets no pull to the door from the spoil haul.**
    /// `home_bias` 0 cuts every homeward pull that reads it, and the haul
    /// (`SPOIL_OUT`'s `haul`, on by default) did not read it, so pellets in
    /// a "blind" dig run still walked to the door (coordinator review of
    /// PR 707). Both arms here have `home_bias` 0 and the shipped genome;
    /// the control arm leaves `World::home_blind` off, and must see the
    /// haul fire, or the test proves nothing.
    #[test]
    fn a_blind_dig_has_no_spoil_haul() {
        let fired = |home_blind: bool| {
            let (mut w, _, _) = bed(7, 12, "").build();
            let sp = w.species.id_of("ant").expect("ant");
            let g = w.species.get(sp).genome.clone();
            clone_colony(&mut w, &g, false, Layout { seed: 7, k: 0, blind: true, legal: false });
            w.home_blind = home_blind;
            for _ in 0..4000 {
                frame(&mut w, true);
            }
            w.creature_stats.homeward_aim.iter().sum::<u64>()
        };
        assert!(fired(false) > 0, "the control's haul never fired, so this cannot see the leak");
        assert_eq!(fired(true), 0, "a homeward pick fired in a blind world");
    }
}
