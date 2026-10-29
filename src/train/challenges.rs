//! The rig's first three arenas. Each one says what it pays for, what the
//! score could be faked with, and what stops that.

use super::{Challenge, Layout, Score, Unit};
use crate::lab::census::{self, Ids};
use crate::lab::nestscore;
use crate::lab::scenario::Scenario;
use crate::sim::brain::BrainOutput as O;
use crate::sim::chunk::Rect;
use crate::sim::cell::OrganismId;
use crate::sim::organism::{self, SpeciesId};
use crate::sim::pheromone::{self, Channel};
use crate::sim::{creature, material, update, Cell, World};

pub mod maze;
pub mod openmaze;
pub use maze::Maze;
pub use openmaze::OpenMaze;

/// Verbs that belong to other species or to a game this rig is not playing
/// yet: hopping and flying (found by the probe as the cheap way to be
/// "home"), fighting, and laying. **Banning `Lay` does not stop births**: the
/// egg bar is the brood block's own `lay_at` (`creature::birth_bar`), and a
/// banned output reads 0, which is above `LAY_HOLD_BELOW`, so an ant that can
/// afford an egg lays one. Births happen where there is food (forage), never
/// in dig, which has none; forage's `founders_kept` counts founders for that
/// reason (ladder lane, 2026-10-17: 9 eggs, 6 births in 3k frames).
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

/// [`frame`], then whatever the bed's timeline has due, as the lab's
/// `tick_timeline` lands it after the step. A bed with no timeline (trail,
/// dig and forage's heap) returns before reading it, so this is [`frame`]
/// exactly; one with a drip of food ([`Scarce`], and the lab's
/// `steady_income`) needs it, since a timeline nothing ticks is a supply
/// that never arrives. Trail and dig call [`frame`], so a timeline given to
/// their beds would be dropped.
fn frame_timed(w: &mut World, sc: &Scenario, pheromones: bool) {
    frame(w, pheromones);
    if !sc.timeline.is_empty() {
        crate::lab::scenario::tick_timeline(sc, w, &sc.bed);
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

/// The plain lab bed, for tests: 256 x 128, 40 rows of soil, no rain, one
/// colony at the centre, and whatever `extra` placements are added, placed
/// before the colony so a buried lens or a raised step is already there
/// when it digs its founding shaft. The arenas build theirs by
/// [`bed_shaped`] or [`bed_at`].
#[cfg(test)]
fn bed(seed: u64, ants: usize, extra: &str) -> Scenario {
    bed_shaped(seed, ants, extra, 40, 33)
}

/// The lab bed the colony arenas use: 256 x 128, no rain, one colony at
/// the centre, with its soil depth and lamp spacing given (the dig arena
/// varies them, [`DigBed`]) and whatever `extra` placements the arena adds,
/// placed before the colony.
fn bed_shaped(seed: u64, ants: usize, extra: &str, soil_depth: i32, lamp_spacing: i32) -> Scenario {
    bed_at(seed, ants, extra, soil_depth, lamp_spacing, BED_W, NEST_X)
}

/// [`bed_shaped`] at any width, the colony at `nest_x`: the far forage bed
/// ([`Forage::far`]) is wider than [`BED_W`], with the nest
/// [`FAR_BEHIND`] columns from the wall behind it.
fn bed_at(seed: u64, ants: usize, extra: &str, soil_depth: i32, lamp_spacing: i32, width: i32, nest_x: i32) -> Scenario {
    bed_timed(seed, ants, extra, soil_depth, lamp_spacing, width, nest_x, "")
}

/// [`bed_at`] with a scenario `timeline`, the lab's own entries: the scarce
/// arena's drip of food ([`Scarce`]) is one, ticked by [`frame_timed`].
#[allow(clippy::too_many_arguments)]
fn bed_timed(seed: u64, ants: usize, extra: &str, soil_depth: i32, lamp_spacing: i32, width: i32, nest_x: i32, timeline: &str) -> Scenario {
    let text = format!(
        r#"(
    name: "train", title: "TRAIN", question: "training arena",
    bed: (width: {width}, height: 128, soil_depth: {soil_depth}, ground_y: {GROUND_Y}, compartments: 1, founders: 0,
          species: "grass", colonies: 0, colony_ants: {ants}, colony_species: "ant", predators: 0,
          lamp_spacing: {lamp_spacing}, seed: {seed}, extra_walls: []),
    settings: [(subject: "the bed", field: "rain", value: 0)],
    placements: [
        {extra}
        Colony(species: "ant", x: {nest_x}, count: {ants}),
    ],
    timeline: [{timeline}],
    horizon: 100000,
)"#
    );
    ron::from_str(&text).expect("training bed parses")
}

/// Every ant in a fresh bed takes `genome`'s brain, and the world its target
/// scorer if it has one ([`super::SCORER`]); births follow food (see
/// [`NOT_ANT_VERBS`]), and the brain steers (`Chooser::Off`) unless `game_walk`. A `blind` layout
/// also cuts the body's homeward re-roll and forces the brain walk, so the
/// senses-cut control is blind outside the brain too ([`Layout::blind`]); a
/// `legal` one draws the scorer's targets evenly ([`Layout::legal`]); walk
/// genes, if the genome carries them, go in as [`give_walk`] puts them.
fn clone_colony(w: &mut World, genome: &[f32], game_walk: bool, layout: Layout) -> SpeciesId {
    let blind = layout.blind;
    let sp = w.species.id_of("ant").expect("ant");
    let mut def = w.species.get(sp).creature.clone().expect("creature");
    def.reproduce_threshold = 1.0e30;
    if blind {
        def.home_bias = 0.0;
    }
    w.species.set_creature(sp, def);
    let at = super::blocks(genome.len());
    let brain = &genome[..super::SCORER];
    w.species.set_genome(sp, brain.to_vec());
    if !game_walk || blind {
        w.chooser = Some(creature::Chooser::Off);
    }
    w.home_blind = blind;
    if let Some(s) = at.scorer {
        let mut t = creature::targets::Targets::new(sp, &genome[s..s + creature::targets::WEIGHTS]);
        t.even = layout.legal;
        w.targets = Some(t);
    }
    let ids = ants_of(w, sp);
    for &id in &ids {
        w.set_organism_genome(id, brain.to_vec());
    }
    if let Some(k) = at.walk {
        give_walk(w, sp, &ids, &genome[k..k + super::WALK]);
    }
    sp
}

/// **A mind as an arena runs it**: the verbs and scents the arena does not
/// offer switched off (`mind::apply_bans`). One brain climbs the ladder
/// and every rung keeps the earlier arenas (design rev3, *No forgetting*),
/// so a forager is tested on the trail rung with the trail's verbs alone:
/// it neither feeds a nestmate there nor lays a trail of its own over the
/// stamped one.
fn arena_mind(genome: &[f32], verbs: &[creature::options::Verb], emits: [bool; creature::mind::EMITS]) -> std::sync::Arc<creature::mind::MindGenome> {
    let mut g = genome.to_vec();
    creature::mind::apply_bans(&mut g, verbs, emits);
    std::sync::Arc::new(creature::mind::MindGenome::new(g))
}

/// **Every ant in a fresh bed takes `genome` as its mind** (`creature::mind`,
/// stage 2), as [`arena_mind`] cuts it to the arena's verbs and scents,
/// brood included, so a founding larva hatches with it and every egg laid
/// inherits it. The larva still feeds itself from a touching adult
/// (`brood`'s `NURSE`, the game's rule): moving that into the adult's turn
/// is the brood rung's own step (design rev3, *Risks*), and with it off a
/// forage colony had to learn to feed its larvae before it could breed.
/// The mind's `Feed` verb is there beside it. A `blind` layout cuts the
/// home vector here; the rig cuts the scent (`train::blinded`).
fn clone_minds(w: &mut World, genome: &[f32], layout: Layout, verbs: &[creature::options::Verb], emits: [bool; creature::mind::EMITS]) -> SpeciesId {
    let sp = w.species.id_of("ant").expect("ant");
    let mut def = w.species.get(sp).creature.clone().expect("creature");
    def.reproduce_threshold = 1.0e30;
    w.species.set_creature(sp, def);
    w.home_blind = layout.blind;
    let g = arena_mind(genome, verbs, emits);
    for id in ants_of(w, sp) {
        creature::mind::give(w, id, g.clone());
    }
    sp
}

/// **Walk genes into a colony** ([`super::WALK`]): each allele into every
/// ant's own traits and the species', where the walk reads them and births
/// and eggs copy them from, and the reserve into the world's food trail.
/// A colony with no brood when it is cloned (every bed founds none) has
/// nothing else to reach.
fn give_walk(w: &mut World, sp: SpeciesId, ids: &[OrganismId], genes: &[f32]) {
    let slots = organism::WALK_SLOTS;
    let mut def = w.species.get(sp).creature.clone().expect("creature");
    for (k, &slot) in slots.iter().enumerate() {
        def.traits[slot] = genes[k];
    }
    w.species.set_creature(sp, def);
    for &id in ids {
        let st = w.organism_mut(id).expect("live ant");
        for (k, &slot) in slots.iter().enumerate() {
            st.traits[slot] = genes[k];
        }
    }
    let reserve = genes[slots.len()];
    w.food_trail = Some(creature::FoodTrail { reserve, ..creature::food_trail_of(w) });
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
    /// **Far food** (`forage-far`): the heap [`FAR_DIST`] columns out, on a
    /// bed widened to hold it. The near distances only ever show a scout's
    /// reserve (`FoodTrail::reserve`) as a gain, so selection there settles
    /// on a leash: a fixed reserve of 0.6 lifted the forage champion's worse
    /// side 43.5 -> 66.1 with the food at 95 and cut it 14.8 -> 2.8 at 170,
    /// and at 200 lost 95 of its 96 founders (8 layouts, 12k frames,
    /// `ladder/reserve-leash-check.md`). Scored as the near arena is; a
    /// suite of both (`arenas=forage,forage-far`) prices a leash by the
    /// product.
    pub far: bool,
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

/// The far heap's distance ([`Forage::far`]): from just past the near
/// arena's [`FORAGE_DIST`] to where the leash check found the 0.6 reserve
/// wiping the champion's colony out.
const FAR_DIST: (i32, i32) = (120, 200);

/// The far bed's ends, the ladder lane's far-food bed: the nest
/// `FAR_BEHIND` columns from the wall behind it and `FAR_BEYOND` columns of
/// ground past the heap, so the bed is `FAR_BEHIND + dist + FAR_BEYOND`
/// wide. **At 70 columns that is the near bed exactly** (256 wide, the nest
/// at 128), and replayed there it scored the rig's own hold-out to the digit
/// (`ladder/probes/leash/rigcheck-champ-2.log`). A wider bed costs no frame
/// time of its own: a 6k-frame layout took 1.4-1.8 s at 256, 326 and 386
/// wide, one thread.
const FAR_BEHIND: i32 = 128;
const FAR_BEYOND: i32 = 58;
const _: () = assert!(FAR_BEHIND == NEST_X && FAR_BEHIND + 70 + FAR_BEYOND == BED_W);

impl Forage {
    fn dist(&self, layout: Layout) -> i32 {
        let (lo, hi) = if self.far { FAR_DIST } else { FORAGE_DIST };
        lo + ((layout.seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 32) % (hi - lo + 1) as u64) as i32
    }

    /// The bed's width, the nest's column and the heap's, for one layout:
    /// the food east on even layouts and west on odd ones. Near, the nest
    /// is the centre of [`BED_W`]; far, it keeps [`FAR_BEHIND`] to the wall
    /// behind it on either side, so the two sides of a pair mirror.
    fn geometry(&self, layout: Layout) -> (i32, i32, i32) {
        let side = if layout.k.is_multiple_of(2) { 1 } else { -1 };
        let dist = self.dist(layout);
        if self.far {
            far_bed(dist, side)
        } else {
            (BED_W, NEST_X, NEST_X + side * dist)
        }
    }
}

/// The far bed for a heap `dist` columns out on `side` (1 east, -1 west):
/// its width, the nest's column and the heap's ([`FAR_BEHIND`]).
fn far_bed(dist: i32, side: i32) -> (i32, i32, i32) {
    let width = FAR_BEHIND + dist + FAR_BEYOND;
    let nest_x = if side > 0 { FAR_BEHIND } else { width - FAR_BEHIND };
    (width, nest_x, nest_x + side * dist)
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

/// **Every training world holds the soil's placement rule where the ladder
/// was measured**: pellets put down stay where they are put
/// (`World::spoil_roll` 0), whatever `PIXEL_PHYSICS_SPOIL_ROLL` or its
/// default says. The roll-on-placement rule (owner-approved for the games,
/// teacher lane PR 753) changes where a put-down pellet comes to rest, so
/// what S3 credits and what falls back into the hole; turned on mid-step it
/// would move every baseline the dig ladder is judged against (s161, s171,
/// the pace bar). It comes into the arenas at a rung boundary, baselines
/// re-measured -- brain lane and teacher lane, 2026-10-21.
pub(crate) fn hold_training_world(w: &mut World) {
    w.spoil_roll = Some(0);
}

impl Challenge for Forage {
    fn name(&self) -> &'static str {
        if self.far {
            "forage-far"
        } else {
            "forage"
        }
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
        // **No bigger than 300**: [`FORAGE_DIST`] is set by a 300-cell
        // heap's slump, and a bigger one reaches home at the near end (food
        // scored that was never carried) and the wall at the far end.
        assert!(self.heap <= 300, "a heap of {} slumps past FORAGE_DIST's bounds (the_heap_lies_clear_of_home_and_the_wall)", self.heap);
        let side = if layout.k.is_multiple_of(2) { 1 } else { -1 };
        let dist = self.dist(layout);
        let (width, nest_x, heap_x) = self.geometry(layout);
        let extra = format!(r#"Heap(material: "provisions", x: {heap_x}, cells: {}),"#, self.heap);
        let sc = bed_at(layout.seed, self.ants, &extra, 40, 33, width, nest_x);
        let (mut w, _, _) = sc.build();
        hold_training_world(&mut w);
        let sp = clone_colony(&mut w, genomes[0], self.game_walk, layout);
        let food = w.materials.id_of("provisions").expect("provisions");
        // **`taken` stops short of home**: at 45 columns a window of
        // `HEAP_REACH` either side reached home columns +5 to +12, and food
        // carried home into that strip read as never taken (review of PR
        // 708, nit 7). A printed part only, never `fit`.
        let (lo, hi) = (heap_x - HEAP_REACH, heap_x + HEAP_REACH);
        let (lo, hi) = if side > 0 { (lo.max(nest_x + 13), hi) } else { (lo, hi.min(nest_x - 13)) };
        let before = cells_of(&w, lo, hi, food);
        let founders = ants_of(&w, sp);
        // The colony's number now, while its founders live: a dead id's
        // colony reads 0, whose books are empty, and the first run of this
        // score read no meals at all on the layouts where founder 0 died.
        let colony = founders.first().map_or(0, |&id| w.colony_of(id));
        for f in 0..self.frames {
            frame_timed(&mut w, &sc, true);
            look(f + 1, &w);
        }
        let st = &w.creature_stats;
        let books = w.colony_books(colony);
        let corpse_j: f64 = w.materials.id_of("corpse").map_or(0.0, |c| books.diet().iter().filter(|(m, _)| *m == c).map(|(_, j)| j).sum());
        let larvae_j = st.brood_crop_fed_j + st.brood_ate_j;
        // Joules kept per joule of face value, from the adults' own meals.
        let keep = if st.digested_face > 0.0 { ((books.intake() - larvae_j) / st.digested_face).clamp(0.05, 1.0) } else { 1.0 };
        let floor = cells_of(&w, nest_x - 12, nest_x + 12, food) as f64;
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
                (before - cells_of(&w, lo, hi, food)) as f64,
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

/// **Live on a trickle of food** (colony): stage 2's forage rung.
///
/// The forage bed with no heap: [`Scarce::drip`] cells of provisions land at
/// one spot every [`Scarce::every`] frames, 80 columns from the nest
/// ([`SCARCE_DIST`]), east on even layouts and west on odd ones, so a pair
/// differs only by the side and the score is the worse side. Births are on,
/// and food caps them ([`Scarce::brake`]).
///
/// **Score: adults alive times the share of founders kept** (a founder's
/// old age counts as kept), read four times over the run's last quarter and
/// averaged, the ladder lane's recommendation
/// (`ladder/scarce-forage-arena.md`). **Before food capped births** it ranked
/// the game ant above the
/// cheap-brain forage champion and that above the champion, on every paired
/// layout at both distances, at 24,000 frames: the lab's order. **Food
/// brought home never ranks them right** -- all three bring nearly all of it
/// -- and neither does energy with births off: the champion lays half as
/// many again and its colony starves on the same food, which only a head
/// count read late enough sees. A colony that never feeds its larvae keeps
/// at most its founders, so refusing brood is no cheat; corpse meals earn
/// nothing, since the score counts heads. A colony gone before the last
/// quarter reads [`scarce_floor`]: how long it lasted, under any live one.
///
/// **Plus a step on the way home for each part of the trip it has learned**
/// ([`forage_steps`], `steps=off` takes it away): heading out towards the
/// food, reaching it,
/// picking some up, carrying it towards the nest, arriving with it, putting
/// it down there (owner, 2026-10-18). Trained on heads alone, at 80 columns
/// for 16 generations of 100 brains, no champion ever carried a cell home:
/// its ants picked up 11-35 cells a run and ate them, and turning round
/// with a load was worth nothing until it reached the nest, which a child
/// that merely could turn never did (`brain-training/stage2/forage-big`).
/// Heads stay in the score and are the rung's check (design rev3, *a second
/// number it cannot trade away*), so a colony that buys steps with its ants
/// shows it there. **But heads are a readout beside the steps, not a gate**:
/// a colony that dies with every founder reaching, picking up, carrying and
/// arriving, and one cell home, scores 15.75, above the live hand-built
/// forager (12.6) and the game ant's heads alone (14.5) (review of PR 717,
/// N1). Gating the steps on survival (times `kept`) is the owner's call.
///
/// **The horizon is a setting**: 12,000 frames trains in half the time and
/// still separates the champion (3.2) from the game ant (21.8); only by
/// 24,000 does a colony that over-breeds lose to the game ant on heads, so
/// a hold-out check runs there (`frames=24000`).
///
/// **With food capping births** (the ladder lane, section 6, 8 layouts at
/// 80 columns, read at the last frame) the champion stays last on every
/// pair, but the cheap-brain champion draws level with the game ant (12,000
/// frames: 16.4 against 14.9, game ahead on 2 of 4 pairs; 24,000: 21.9
/// against 16.4, 4 of 4): the old order between those two was the cheap
/// arm's over-breeding, which a mind cannot choose, since it lays only
/// through `settle`.
///
/// Runs minds (`creature::mind`) as well as today's brain: walking, turning
/// round, resting, eating, putting food down and feeding, beside the
/// larvae's own feeding from a touching adult ([`clone_minds`]).
pub struct Scarce {
    pub ants: usize,
    pub frames: u64,
    pub drip: u32,
    pub every: u64,
    /// Today's brain under the shipped walk (`walk=game`); minds always
    /// walk by their own choice.
    pub game_walk: bool,
    /// **Food caps births** (`creature::FoodBrake`): the colony lays less as
    /// its income closes on what it burns, brood included, and not at all
    /// once it burns what it earns. [`creature::FoodBrake::FORAGE`] unless
    /// `brake=off`.
    pub brake: Option<creature::FoodBrake>,
    /// **How far out the food lands, in columns**: `lo`, `hi` and two
    /// between, one per mirrored pair in turn ([`SCARCE_DIST`] unless
    /// `dist=` sets it), so a generation of 8 layouts, or any multiple of
    /// 8, meets the whole range; fewer meet it over successive generations
    /// (review of PR 717, N10). Drawn
    /// per layout from the seed, a generation could draw only far food: a
    /// fetch run at 10-80 columns found an ant that reached and picked up
    /// in generations 0-1 (fetch 1.53 on its layouts, 1.27 on the
    /// hold-outs) and lost it by generation 2, when every brain read about
    /// 1.0 and the trail alone chose (`stage2/fetch2/mind-s3`, the run
    /// before this). Nearer food is
    /// the forage rung's stepping stone: trained as planned, at 80 columns
    /// for 40 generations, the mind's colonies walked one way past the food
    /// to the far wall and never carried any home (scarce alone 1.0-1.9,
    /// against 0.90 for a colony that never moves and 4.57 for the hand-built
    /// forager; `brain-training/stage2/forage-lean/census`).
    pub dist: (i32, i32),
    /// **Score the way home step by step** ([`forage_steps`]) on top of the
    /// heads; on unless `steps=off`.
    pub steps: bool,
    /// **The fetch arena** (`challenge=fetch`, one ant unless `ants=`): no
    /// one starves ([`never_starve`]) and no one is born, so the steps are
    /// the whole score and an ant has the run to find the food. Asked for by
    /// the owner (2026-10-18) so a brain learns the trip without paying
    /// eleven nestmates per score; the colony arena stays its check. A lone
    /// ant as the colony runs it starved by frame ~2,900 and never reached
    /// food 80 columns out, the hand-built forager and the game ant alike.
    pub fed: bool,
    /// **The kept colony with Dig switched on** (`challenge=scarce-dig`;
    /// the dig rung's from step 4, R12 in `dig-rung-design.md`): the same
    /// colony on the same trickle, offered the cut ([`SOIL_VERBS`]) and
    /// scored on heads alone. Digging when it should forage costs founders
    /// here, which is where the forage-or-dig choice gets trained; the soil
    /// arena, where nobody starves, cannot teach it. Its forget floor is
    /// two standard errors rather than a share ([`Challenge::floor_in_se`]).
    pub dig: bool,
}

/// The food spot's distance from the nest, in columns: the ladder's far
/// bed. At 40 columns and 12,000 frames a colony that over-breeds ties the
/// game ant on the worse side (19.2 against 19.3); at 80 the game ant beats
/// the cheap-brain champion on 4 of 4 pairs (17.7 against 8.3), so the rung
/// trains there (ladder review of the stage 2 notes, finding 5). Inside the
/// 256-wide bed either side.
pub const SCARCE_DIST: (i32, i32) = (80, 80);

impl Scarce {
    /// **The nearest the food may land, in columns.** Below it the far half
    /// of the way no longer keeps the nest's doorstep out of [`Trip::HEAP`]:
    /// at 8 columns a cell shuffled across the nest's edge reads as taken at
    /// the heap, and 50 laps book 50 cells home (review of PR 717, N9).
    /// `a_cell_shuffled_across_the_nest_edge_pays_nothing` holds from here.
    pub const DIST_MIN: i32 = 10;

    /// The nest's column, the food spot's and its distance, for one layout.
    fn geometry(&self, layout: Layout) -> (i32, i32, i32) {
        let side = if layout.k.is_multiple_of(2) { 1 } else { -1 };
        let (lo, hi) = self.dist;
        let dist = lo + (hi - lo) * (layout.k / 2 % 4) as i32 / 3;
        (NEST_X, NEST_X + side * dist, dist)
    }

    /// The bed for one layout, the drip on its timeline.
    fn bed(&self, layout: Layout) -> Scenario {
        let (nest_x, food_x, _) = self.geometry(layout);
        let timeline = format!(r#"(at: 1, every: {}, until: 0, what: Heap(material: "provisions", x: {food_x}, cells: {})),"#, self.every, self.drip);
        bed_timed(layout.seed, self.ants, "", 40, 33, BED_W, nest_x, &timeline)
    }
}

/// The verbs a forager uses: the trail rung's walk, turn and rest, and the
/// food verbs. **Pick up** since fix plan step 1, which gave it the
/// swallow Eat had: before that it was listed by the body, banned here
/// and never acted.
const SCARCE_VERBS: &[creature::options::Verb] = {
    use creature::options::Verb as V;
    &[V::Walk, V::TurnAround, V::Rest, V::PickUp, V::Eat, V::Drop, V::Feed]
};

impl Challenge for Scarce {
    fn name(&self) -> &'static str {
        if self.fed {
            "fetch"
        } else if self.dig {
            "scarce-dig"
        } else {
            "scarce"
        }
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
        &["score", "adults", "kept", "starved", "births", "brood", "pickups", "delivered", "larvae_j", "held_ticks", "dist", "lived", "heads", "steps", "reach", "picked", "carried", "arrived", "home", "out", "ate", "starved_soil", "stuck"]
    }
    fn banned(&self) -> &'static [O] {
        NOT_ANT_VERBS
    }
    fn minds(&self) -> bool {
        true
    }
    fn verbs(&self) -> &'static [creature::options::Verb] {
        if self.dig {
            SOIL_VERBS
        } else {
            SCARCE_VERBS
        }
    }
    /// The colony with Dig only ([`Scarce::dig`]): its heads swing far
    /// more than fetch's or trail's, so a share of its reference cuts on
    /// noise.
    fn floor_in_se(&self) -> bool {
        self.dig
    }
    fn mirrored(&self) -> bool {
        true
    }
    fn run(&self, layout: Layout, genomes: &[&[f32]], look: &mut dyn FnMut(u64, &World)) -> Vec<Score> {
        let (nest_x, food_x, dist) = self.geometry(layout);
        let sc = self.bed(layout);
        let (mut w, _, _) = sc.build();
        hold_training_world(&mut w);
        w.food_brake = self.brake;
        // Every death, by ant, so `kept` can count founders' old age only.
        w.death_log = Some(Vec::new());
        let sp = if super::is_mind(genomes[0].len()) {
            clone_minds(&mut w, genomes[0], layout, self.verbs(), self.mind_emits())
        } else {
            clone_colony(&mut w, genomes[0], self.game_walk, layout)
        };
        if self.fed {
            let mut def = w.species.get(sp).creature.clone().expect("creature");
            never_starve(&mut def);
            w.species.set_creature(sp, def);
            for id in ants_of(&w, sp) {
                if let Some(s) = w.organism_mut(id) {
                    s.energy = 1.0e9;
                }
            }
            w.births_paused = true;
        }
        let adults_of = |w: &World| -> Vec<OrganismId> { ants_of(w, sp).into_iter().filter(|&id| w.organism(id).is_some_and(|s| s.brood.is_none())).collect() };
        let founders = adults_of(&w);
        let colony = founders.first().map_or(0, |&id| w.colony_of(id));
        // Adults now, and the share of founders kept so far.
        let census = |w: &World| -> (Vec<OrganismId>, f64) {
            let now = adults_of(w);
            // **Founders' old age only.** The colony's old-age count
            // (`old_age`) took hatchlings' too, and the hazard is a Rayleigh
            // draw, so a hatchling can go before a founder: a colony that
            // bred and lost young ants to age counted them as founders kept
            // (ladder review of the stage 2 notes, finding 5). Counted this
            // way the ladder's ranking still holds on every pair: 25.1 /
            // 10.3 / 0.0 at 40 columns.
            let aged = w.death_log.as_ref().map_or(0, |log| {
                log.iter()
                    .filter(|r| {
                        matches!(r.cause, organism::DeathCause::OldAge) && founders.binary_search(&r.id).is_ok()
                    })
                    .count()
            });
            let lived = founders.iter().filter(|id| now.binary_search(id).is_ok()).count() + aged;
            let kept = (lived as f64 / founders.len().max(1) as f64).min(1.0);
            (now, kept)
        };
        // **Scored over the run's last quarter, not its last frame**: with
        // food capping births a colony's head count rises and falls as it
        // lays to its income, and the ladder's 24,000-frame order rested on
        // one frame (game ant ahead of the cheap arm on 4 of 4 pairs at
        // 24,000, on 1-2 of 4 at every snapshot from 10,000 to 22,000;
        // `ladder/scarce-forage-arena.md` section 6). Four reads, the last
        // at the end.
        let snaps: Vec<u64> = (13..=16).map(|k| self.frames * k / 16).filter(|&f| f > 0).collect();
        let mut scores = Vec::new();
        // The same read over the whole run, for the floor ([`scarce_floor`]).
        let lived_every = (self.frames / 64).max(1);
        let mut lived_sum = (0.0, 0u32);
        let mut trip = Trip::default();
        for f in 0..self.frames {
            frame_timed(&mut w, &sc, true);
            look(f + 1, &w);
            if (f + 1).is_multiple_of(STEP_EVERY) {
                trip.look(&w, &adults_of(&w), nest_x, food_x, dist);
            }
            let at_snap = snaps.contains(&(f + 1));
            if at_snap || (f + 1).is_multiple_of(lived_every) {
                let (now, kept) = census(&w);
                if at_snap {
                    scores.push(now.len() as f64 * kept);
                }
                if (f + 1).is_multiple_of(lived_every) {
                    lived_sum.0 += now.len() as f64 * kept;
                    lived_sum.1 += 1;
                }
            }
        }
        let (now, kept) = census(&w);
        if scores.is_empty() {
            scores.push(now.len() as f64 * kept);
        }
        let deaths = |c: organism::DeathCause| w.group_deaths_of(sp, colony).map_or(0, |g| g.by_cause[c.index()]);
        let starved = deaths(organism::DeathCause::Starved) + deaths(organism::DeathCause::StarvedInFlight);
        let st = &w.creature_stats;
        let brood = ants_of(&w, sp).len() - now.len();
        let larvae_j = st.brood_crop_fed_j + st.brood_ate_j + st.brood_shared_j + st.brood_nursed_j;
        let lived = lived_sum.0 / lived_sum.1.max(1) as f64 / founders.len().max(1) as f64;
        let heads = scarce_floor(scores.iter().sum::<f64>() / scores.len() as f64, lived, founders.len());
        let n = founders.len().max(1) as f64;
        let home = trip.home as f64;
        let (reach, picked, arrived) = (
            trip.reached.len() as f64 / n,
            trip.picked.len() as f64 / n,
            trip.arrived.len() as f64 / n,
        );
        let steps = forage_steps(trip.out, reach, picked, trip.carried, arrived, home);
        // **Fix plan step 1's reads.** `pickups` is every cell taken into a
        // crop (it was labelled `eaten` until then, and counted pick-ups);
        // `home` is food home net of churn, the cells from the heap put down
        // at the nest, where `delivered` counts every put-down there (one
        // colony counted 413 against 40.5 home). `ate` is food eaten from
        // the crop by choice (`MindCounts::ate`), which should fall towards
        // zero as `home` rises; the crop's own digestion is not in it.
        // `starved_soil` counts ants that starved holding soil, and `stuck`
        // those of them with food in reach (`MindCounts::starved_soil`).
        let (ate, stuck) = w.minds.as_ref().map_or((0, (0, 0)), |m| (m.counts.ate, (m.counts.starved_soil, m.counts.starved_soil_food)));
        let score = heads + if self.steps { STEP_SCALE * steps } else { 0.0 };
        vec![Score {
            fit: score,
            parts: vec![
                score,
                now.len() as f64,
                kept,
                starved as f64,
                st.births as f64,
                brood as f64,
                st.pickups as f64,
                st.deliveries as f64,
                larvae_j,
                st.food_brake_held as f64,
                dist as f64,
                lived,
                heads,
                steps,
                reach,
                picked,
                trip.carried,
                arrived,
                home,
                trip.out,
                ate as f64,
                stuck.0 as f64,
                stuck.1 as f64,
            ],
        }]
    }
}

/// **The way home, step by step** (owner, 2026-10-18): heading out towards
/// the food, reaching it, picking some up, carrying it towards the nest,
/// arriving with it, putting it down there. Each step outweighs every step
/// below it together (0.5 + 1 + 2 + 4 + 8 = 15.5 < 16), so stacking the
/// easy ones never beats one cell home. **At full shares**: in a colony,
/// reaching, picking up and arriving are shares of the founders, so twelve
/// ants that reach and pick up with a load carried half way (5) outrank one
/// that carries all the way and arrives (4.92; review of PR 717, N2). With
/// one ant (`fetch`) the order holds.
///
/// **Heading out is a step because reaching the food was all or nothing.**
/// Scored from reaching up, one ant at 80 columns, judged on the worse of
/// two mirrored layouts, read 1.00 for every brain of four 100-brain runs:
/// no start ant reached the food on both sides of the nest, so the fetch
/// score was flat and the runs were selected on the trail alone (gens 0-11,
/// `brain-training/stage2/fetch`, 2026-10-18). Graded like `carried`, the
/// furthest an ant got out pays a searcher that goes both ways and not one
/// that walks off in one direction, which the worse side reads as 0.
pub const STEP_WEIGHTS: [f64; 6] = [0.5, 1.0, 2.0, 4.0, 8.0, 16.0];

/// What one step point adds to a scarce colony's score, in heads: the whole
/// way short of the nest (15.5 points) is worth under eight ants kept, a
/// first cell home eight.
pub const STEP_SCALE: f64 = 0.5;

/// How often the trip is read, in frames: an ant walks well under a cell a
/// frame, so it cannot cross the food spot's reach between two reads.
const STEP_EVERY: u64 = 4;

/// **How far the colony got on the way home, in step points**
/// ([`STEP_WEIGHTS`]). `out` is the furthest any ant got from the nest
/// towards the food, as a share of the way there; `reach`, `picked` and
/// `arrived` are the share of the founders that ever reached the heap,
/// picked food up there and stood at the nest still carrying it (or put it
/// down there); `carried` is the most way home any load from the heap
/// closed, as a share of the way; `home` the cells from the heap put down
/// at the nest, each doubling worth a quarter of the first (all read by
/// `Trip`). Shares past 1 (a colony that bred) count as 1.
pub fn forage_steps(out: f64, reach: f64, picked: f64, carried: f64, arrived: f64, home: f64) -> f64 {
    let w = STEP_WEIGHTS;
    let delivered = if home >= 1.0 { 1.0 + home.log2() / 4.0 } else { 0.0 };
    w[0] * out.clamp(0.0, 1.0)
        + w[1] * reach.clamp(0.0, 1.0)
        + w[2] * picked.clamp(0.0, 1.0)
        + w[3] * carried.clamp(0.0, 1.0)
        + w[4] * arrived.clamp(0.0, 1.0)
        + w[5] * delivered
}

/// **What the scarce colony did on the way home**, read every
/// [`STEP_EVERY`] frames. **A load counts only from food taken at the
/// heap** ([`Trip::HEAP`]). The trip mark (`trip_cells`, which the game's
/// pickup and a mind's swallow both set) is any food taken away from home,
/// and the mind's swallow has no trip minimum, so one cell swallowed a
/// column outside the nest and put down inside booked a trip delivery on
/// every lap -- 50 laps, 50 deliveries (review of PR 717 at `edb9137e`,
/// B1). So each ant's cells taken at the heap are kept apart (`held`), and
/// a delivery pays only out of them: a cell shuffled across the nest's edge
/// pays nothing, and a cell carried home pays once.
#[derive(Default)]
struct Trip {
    out: f64,
    reached: Vec<OrganismId>,
    picked: Vec<OrganismId>,
    arrived: Vec<OrganismId>,
    carried: f64,
    loads: std::collections::BTreeMap<OrganismId, Load>,
    /// Cells taken at the heap and put down at the nest.
    home: u64,
}

/// One ant's crop as the trip saw it at the last read.
#[derive(Default)]
struct Load {
    /// Its trip cells.
    cells: u16,
    /// Its trip deliveries so far (`LifeCounters::trip_deliveries`).
    put: u32,
    /// How many of its trip cells it took at the heap.
    held: u16,
    /// How far from the nest it was when it took them.
    taken: i32,
}

impl Trip {
    /// **At the food is within this many columns of the food spot, on the
    /// far half of the way.** The heap is a pyramid that spreads past four
    /// columns of where it drips, so the old ±4 read an ant picking up at
    /// its foot as picking up without reaching (review of PR 717, N3); the
    /// far half keeps the nest's doorstep out at 10 columns.
    const HEAP: i32 = 10;

    fn look(&mut self, w: &World, adults: &[OrganismId], nest_x: i32, food_x: i32, dist: i32) {
        let mark = |v: &mut Vec<OrganismId>, id: OrganismId| {
            if let Err(i) = v.binary_search(&id) {
                v.insert(i, id);
            }
        };
        let way = dist.max(1) as f64;
        for &id in adults {
            let Some(st) = w.organism(id) else { continue };
            let Some(&(hx, _)) = st.chain.first() else { continue };
            let from_nest = (hx - nest_x).abs();
            let out = 1.0 - (hx - food_x).abs() as f64 / way;
            self.out = self.out.max(out.clamp(0.0, 1.0));
            let at_heap = (hx - food_x).abs() <= Self::HEAP && 2 * from_nest >= dist;
            if at_heap {
                mark(&mut self.reached, id);
            }
            let cells = if st.crop.is_none_or(|c| c.cells == 0) {
                0
            } else {
                st.trip_cells
            };
            let put = st.life.trip_deliveries;
            let l = self.loads.entry(id).or_insert(Load { put, ..Load::default() });
            // Deliveries since the last read pay out of the heap's cells.
            let paid = put.saturating_sub(l.put).min(u32::from(l.held)) as u16;
            if paid > 0 {
                self.home += u64::from(paid);
                mark(&mut self.arrived, id);
            }
            l.held = (l.held - paid).min(cells);
            if cells > l.cells && at_heap {
                if l.held == 0 {
                    l.taken = from_nest;
                }
                l.held += cells - l.cells;
                mark(&mut self.picked, id);
            }
            (l.cells, l.put) = (cells, put);
            if l.held == 0 {
                continue;
            }
            // **Distance closed**, not nearness: a load taken at the heap's
            // near foot starts part of the way home (N3). One taken past the
            // food spot reads full about `HEAP` columns short of the nest,
            // where arriving takes over (N12).
            self.carried = self.carried.max((f64::from(l.taken - from_nest) / way).clamp(0.0, 1.0));
            if creature::at_nest(w, id) {
                mark(&mut self.arrived, id);
            }
        }
    }
}

/// **Nobody starves: the budget rises and the bills do not.** Every
/// standing tax is a fraction of `start_energy` (§13j), so raising the
/// budget alone raised them with it: a mind with ~700 live weights spent its
/// whole 1e9 inside the 6,000 frames, and a fully live one lost all six ants
/// by frame ~1,300 (review of PR 713, F1). Each fraction is rescaled so it
/// bills what it bills at the species' authored budget, for both brains
/// alike.
fn never_starve(def: &mut organism::CreatureDef) {
    budget_to(def, 1.0e9);
}

/// **A colony gone before the last quarter still ranks by how long it
/// lasted**: `score` (the last quarter's reads) when it is above 0,
/// otherwise `lived` (the same adults-times-founders-kept read every 64th
/// of the run, over the founders) scaled under the least score a live
/// colony can read, a single founder at one of the four reads, so a dead
/// colony never outranks a live one on heads and no live score moves. On
/// heads only: the scarce arena's own steps ([`forage_steps`]) can lift a
/// dead colony over a live one, and so, in a suite, can another arena's
/// ratio (review of PR 715, N2).
///
/// **Why**: today's brain started the forage rung from its trail champion,
/// which never picks food up; its colonies starved by frame 6,000 of
/// 12,000 on every layout, every child of 48 for 20 generations read
/// exactly 0 at both step sizes (`brain-training/stage2/forage-lean`,
/// today-plan-s1, today-old-s1), and selection had nothing to choose by. The
/// mind's trail champion read 0 too but picks food up, so its first
/// children already lived into the last quarter. A child that eats a little
/// or walks a little less dies later and now ranks higher. Resting is not
/// newly paid: a colony that does nothing already outlives the last quarter
/// at 12,000 frames and scores on the reads (`ladder/scarce-forage-arena.md`).
pub fn scarce_floor(score: f64, lived: f64, founders: usize) -> f64 {
    if score > 0.0 {
        return score;
    }
    // One ant at one read, with one founder of `founders` kept.
    let least = 1.0 / (4.0 * founders.max(1) as f64);
    lived.clamp(0.0, 1.0) * least / 2.0
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
/// is eaten, nobody starves (the budget is raised to 1e9 and every tax is
/// booked at the authored one's size, which an f32 at 1e9 never takes off
/// the body, so a brain's size is free here unless `Settings::wire` charges
/// it: review of PR 713 at `d8a55651`, N1), and every ant's `EmitA`/`EmitB`
/// rows are banned, so the only trail is the stamped one.
///
/// Score per ant: the mean over the run of how far along the trail it stood
/// (0 at its start, 1 at the far end), **signed**: as far out on the bare
/// side counts as far below 0. An ant that always walks one way scores well
/// on half the layouts and below nothing on the other half; only reading
/// the trail scores on both. Unsigned, a walker that never read the trail
/// scored 0.17-0.26 as six (the walls turn it round, and in a group some
/// go each way); signed, every blind walker reads 0.000 +- 0.004 and the
/// hand-built follower is unchanged layout by layout (ladder lane,
/// `trail-rung-controls.md`).
///
/// **`right`**, a part: whether the ant ended at least a tenth of the way
/// along. A clone arena's mean is the share that went the right way: a
/// follower sends nearly all of them, and a walker that keeps whatever
/// heading it has about half, on both sides, which the worse-side fold
/// alone does not catch (ladder lane, `trail-rung-controls.md`).
///
/// **The first rung of stage 2's ladder**: it runs minds
/// (`creature::mind`) as well as today's brain, walking, turning round and
/// resting, and laying nothing.
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
        &["along", "reach", "end", "walked", "right"]
    }
    fn banned(&self) -> &'static [O] {
        TRAIL_BANNED
    }
    fn minds(&self) -> bool {
        true
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
        hold_training_world(&mut w);
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
        // Nobody starves ([`never_starve`]).
        never_starve(&mut def);
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
        // No walk genes: this arena always walks by the brain, which reads
        // none of them (`super::WALK`), so a suite's walk block rides
        // through here unread.
        for (k, &id) in ids.iter().enumerate() {
            if super::is_mind(genomes[k].len()) {
                creature::mind::give(&mut w, id, arena_mind(genomes[k], self.verbs(), self.mind_emits()));
            } else {
                w.set_organism_genome(id, genomes[k][..super::SCORER].to_vec());
            }
            start[k] = w.organism(id).and_then(|s| s.chain.first().copied()).map_or(mid, |p| p.0);
        }
        let progress = |x: i32, k: usize| -> f64 { (((x - start[k]) * dir) as f64 / (far - start[k]).abs().max(1) as f64).clamp(-1.0, 1.0) };
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
                let end = progress(last[k], k);
                Score { fit: along, parts: vec![along, reach[k], end, walked[k] as f64, f64::from(u8::from(end >= 0.1))] }
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

const DIG_PARTS: &[&str] = &["room", "pit", "mound", "alive", "roofed", "spoil_below", "placed", "holding", "under_fill", "depth"];
const DIG_TARGET_PARTS: &[&str] = &[
    "room", "pit", "mound", "alive", "roofed", "spoil_below", "placed", "holding", "under_fill", "depth", "dig_choices", "dig_margin", "dig_none", "drop_choices", "drop_margin", "drop_none", "dropped_inside", "dropped_outside",
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
    /// Room cyan, room under packed fill violet, pit red, refused amber
    /// ([`dug_cells`]): what the score counted, what is open to the sky, and
    /// what the census would count and the score does not. Cyan includes
    /// the founding cut the bed starts with (about 10 cells), which the
    /// score subtracts.
    fn marks(&self, layout: Layout, w: &World) -> Vec<((i32, i32), [u8; 3])> {
        let shape = DigBed::of(layout);
        let spec = bed_shaped(layout.seed, self.ants, &shape.placements(), shape.depth, shape.lamp).bed;
        let colour = |kind| match kind {
            Dug::Room => [0, 190, 255],
            Dug::Filled => [170, 90, 255],
            Dug::Pit => [255, 50, 30],
            Dug::Refused => [255, 190, 0],
        };
        dug_cells(w, &spec).into_iter().map(|(at, kind)| (at, colour(kind))).collect()
    }
    fn run(&self, layout: Layout, genomes: &[&[f32]], look: &mut dyn FnMut(u64, &World)) -> Vec<Score> {
        let shape = DigBed::of(layout);
        let sc = bed_shaped(layout.seed, self.ants, &shape.placements(), shape.depth, shape.lamp);
        let (mut w, _, _) = sc.build();
        hold_training_world(&mut w);
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
        // What is scored, for a picture's sixth snapshot (`train::picture`).
        look(self.frames + DIG_SETTLE, &w);
        let end = census::census(&w, &sc.bed, 0.0, &cols, &ids);
        let room = joined_room(&w, &sc.bed) as f64 - base_joined as f64;
        let roofed = end.roofed_room() as f64 - base.roofed_room() as f64;
        let kept = (lived as f64 / placed.len().max(1) as f64).min(1.0);
        // **Two checks on what the room is**, beside the count: room whose
        // only roof is packed fill ([`Dug::Filled`], which has a floor of
        // about 9 on the hand rules), and how deep the room lies below the
        // old ground on average, so a scorer that backfills and digs under
        // its fill, or cuts shallow, shows in the numbers and not only in a
        // picture (coordinator review of PR 709). `depth` averages **every**
        // counted cell, the founding cut's included, unlike `room`, which is
        // the difference from the start: a colony that dug nothing reads
        // about 6.5, the founding cut's own depth, and one with no room at
        // all reads 0, the same as room at the datum (review of PR 710).
        let cells = dug_cells(&w, &sc.bed);
        let under_fill = cells.iter().filter(|c| c.1 == Dug::Filled).count() as f64;
        let depths: Vec<f64> = cells.iter().filter(|c| matches!(c.1, Dug::Room | Dug::Filled)).map(|&((x, y), _)| (y - census::surface_of(&w, &sc.bed, x)) as f64).collect();
        let depth = if depths.is_empty() { 0.0 } else { depths.iter().sum::<f64>() / depths.len() as f64 };
        let mut parts = vec![room, end.pit as f64 - base.pit as f64, end.mound as f64, alive as f64, roofed, spoil_below(&w, &sc.bed) as f64, placed.len() as f64, holding as f64, under_fill, depth];
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

/// **The dig rung's soil trip, for minds** (`challenge=soil`): the first
/// four steps of the ladder lane's dig rung (`brain-training/ladder/
/// dig-rung-design.md` rev 2), built with the changes of its adversarial
/// review (`review-dig-rung-design.md`). Today's [`Dig`] stays as it was.
///
/// One ant (unless `ants=`) on a bed founded with a shaft of `shaft` rows,
/// so the ant starts at a hole it can deepen. Steps 1-3 use a flat bed,
/// [`DigBed`]'s soil depth and lamps for the layout with no lens and no
/// step; **from step 4 the whole [`DigBed`]**, buried stone and raised step
/// included, in mirrored pairs scored by the worse side (design section
/// 3: "mirrored pairs come back with the generator beds from step 4").
/// Its mind runs the forager's verbs and `Dig` ([`SOIL_VERBS`]). Each
/// pellet is scored by how far along the soil trip it got ([`soil_trip`]):
/// cut out of the original ground, put down, put down away from the hole,
/// carried up from below the ground. `step` (1-4) is the stage being
/// learned: it pays for every pellet, the stages under it for at most
/// [`SOIL_CAP`] an ant, and the stages above it nothing yet. Step 4 is
/// trained at shafts of 4, 8 and 12 rows in turn (the design's "up and
/// out", 2 -> 12 rows), each from the last one's hold-best.
///
/// **Settings are the world's, set by the arena** (design section 3): the
/// shaft's depth and the loose door between `build_counted` and `apply`,
/// because the founding shaft is cut inside `apply` and a depth set after
/// it is never read (review R4; the ladder lane's door probe H0 hit the
/// same). The door column keeps pellets out of the shaft's own column and
/// does not clear it: with a resting ant the door stayed open on 41 of 41
/// checks either way, and what shuts it is loose soil beside the mouth
/// (H0, 2026-10-18), which is the ant's to learn not to leave.
///
/// **Energy is a budget, not a feed** (review R13): `budget` joules, every
/// standing bill kept what it is at the species' own budget (as
/// [`never_starve`] keeps it), so the energy sense falls as an unfed
/// founder's does and a mind that grows links or digs to no purpose runs
/// dry before the bell. `None` feeds it for ever, as `fetch` does.
///
/// **Scored after a settle**, as [`Dig`] is: at the bell every ant takes the
/// `rest` mind and the bed runs [`DIG_SETTLE`] frames, so a pellet left
/// where it will slide back into the hole costs its refill.
pub struct Soil {
    pub ants: usize,
    pub frames: u64,
    pub step: u8,
    pub shaft: i32,
    pub budget: Option<f32>,
    /// **Step 4b: only the shaft's foot pays** (`foot=1`, ladder lane,
    /// `dig-rung-design.md` rev 2.9; owner, 2026-10-19: "Take that and
    /// continue training"): from step 4 a first cut earns S1 only at least
    /// `shaft` rows down, so a cut into the shaft's walls pays nothing at
    /// any stage, as a top-row cut has since rev 2.4. Seed 111's gen-226
    /// hold-best carried 2.83 of its 3.25 foot cuts a world all the way
    /// out but made 4.0 wall cuts besides, 34% of its score; carrying was
    /// solved and where it cut was not. Off is rev 2.8, which step 4's
    /// guards and numbers are written against. At `shaft` 0 the foot is
    /// the first row under the top, so on is rev 2.8 there too.
    pub foot: bool,
    /// **The share of bed pairs with the step forced near the shaft**
    /// (`stepnear=`, [`DigBed::drawn`]), 0 for the beds every earlier run
    /// and score met. It moves the hold-out with the training beds, so a
    /// run's own hold-out numbers are not comparable across it; judge on
    /// `sealed=1` at the default.
    pub step_near: f64,
    /// **Print the general nest score at the bell** (part `nest`,
    /// [`nestscore`]; owner, 2026-10-21: a printed number, never paid).
    /// `examples/train.rs` sets it for `score=` and `noise=1` only: a suite
    /// runs this arena on ~2,800 worlds a generation and keeps only its fit,
    /// so reading the nest on every one would slow every run for a number
    /// nobody reads (brain lane, 2026-10-21). Off, the part is 0. A read of
    /// the grid: it touches neither the world nor a random stream, so on or
    /// off the run is the same bit for bit.
    pub nest: bool,
}

/// The soil trip's verbs: the forager's ([`SCARCE_VERBS`]) and the cut.
/// Putting down is one verb for food and soil: which one goes down is what
/// the jaws hold (`mind::put_down`), the owner's choice of one put-down
/// over two (dig design, section 0).
const SOIL_VERBS: &[creature::options::Verb] = {
    use creature::options::Verb as V;
    &[V::Walk, V::TurnAround, V::Rest, V::PickUp, V::Eat, V::Drop, V::Feed, V::Dig]
};

/// What a pellet earns at each stage of the soil trip: cut, put down, put
/// down away from the hole, and -- the most it can earn there -- carried
/// up and out ([`SOIL_UP_ROWS`]). Each is worth more than the ones below
/// it together, as the forage steps are ([`STEP_WEIGHTS`]).
pub const SOIL_POINTS: [f64; 4] = [1.0, 2.0, 4.0, 8.0];

/// **Rows of climb that earn S4 in full** (design section 4): a pellet
/// carried up `r` rows earns `SOIL_POINTS[3] * min(1, r / SOIL_UP_ROWS)`,
/// paid per row (review R8), so a pellet carried up one row is worth
/// something and the eighth row as much as the first.
pub const SOIL_UP_ROWS: i32 = 8;

/// **Pellets an ant is paid for at a stage under the one being learned**
/// (from step 4, its deepest: [`soil_trip`]).
/// A farmer that cuts and drops at the face for the whole run earns the
/// cap and no more, so the next stage is always worth reaching (design,
/// section 4).
pub const SOIL_CAP: usize = 10;

/// **Away from the hole**: at least this many cells, Chebyshev, from every
/// hole cell (design, section 4's S3) -- a pellet 3 cells off is away, one 2
/// cells off is not.
pub const SOIL_AWAY: i32 = 3;

/// What each hole cell that is ground again after the settle costs, from
/// step 3: a pellet put back, or one left at the edge that slid in.
pub const SOIL_REFILL: f64 = 4.0;

/// **The soil trip's energy budget, in joules** (`budget=`, or `fed` for
/// none): about 1.3x what the forage starts burn over a 6,000-frame run
/// with `Dig` switched on (dig design, section 3). Measured on 4 layouts at
/// a budget too big to run out: the 450-link small-brain start
/// (`stage2/colony/starts/c33-trim-k450`) 800 J, the 1,189-link 10-30
/// fetch winner 1,091 J, a blank mind 334 J, a resting one 131 J. Against
/// the species' own 200 J, a fed ant's 1e9 and the colony's real hunger,
/// this is the middle way the review asked for (R13): enough for a start to
/// work the whole run, not enough for a lineage that doubles its links.
pub const SOIL_BUDGET: f32 = 1200.0;

/// **What the soil trip came to**, for [`Soil`]'s score and the numbers
/// beside it. `stages` counts pellets that reached each stage, every ant,
/// uncapped; `score` is capped and gated as [`soil_trip`] says.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SoilTrip {
    pub score: f64,
    pub cuts: usize,
    pub stages: [usize; 3],
    /// Of `stages[2]`, pellets carried up at least a row from below the
    /// original ground (S4), and the rows they climbed, each pellet's
    /// capped at [`SOIL_UP_ROWS`]. Counted at every step, paid from step 4.
    pub up: usize,
    pub up_rows: usize,
    /// **First cuts of original ground from below its surface** (depth
    /// above 0): the pellets that could earn S4, so `up / deep` is step 4's
    /// share (at least 0.75, pooled; ladder lane, `dig-rung-design.md` step
    /// 4).
    pub deep: usize,
    /// **First cuts of original ground in its top row or above** (depth 0
    /// or less): rim quarrying, counted at every step and paid nothing from
    /// step 4 (`dig-rung-design.md` rev 2.4).
    pub rim: usize,
    /// **Loads up and out, depth-weighted** (`dig-rung-design.md` rev 2.6,
    /// the step 4 bar): each S4 pellet counts `min(1, rows paid /
    /// full_depth)`, so a pellet carried up from a pit 2 rows deep at a
    /// 4-row shaft is half a load. `load_base` is the same weight over the
    /// first cuts below the surface (`min(1, depth / full_depth)`), and
    /// `loads / load_base`, pooled over worlds, is the bar's 0.75 share.
    /// Prints only, at every step. **It does not gate** (rev 2.8): it rose
    /// with total rows climbed whatever the depth, so 6 pellets from 1.67
    /// rows (2.5 loads) beat 4 from 2.25 (2.25; review of PR 726, D1).
    pub loads: f64,
    pub load_base: f64,
    /// **Full loads** (`dig-rung-design.md` rev 2.8, the step 4 bar): S4
    /// pellets cut at `full_depth` or deeper and carried at least that many
    /// rows up, counted before [`SOIL_UP_ROWS`] caps the rows paid, so a
    /// 12-row shaft can have them. `full_cuts` are first cuts of original
    /// ground at `full_depth` or deeper, and `full_loads / full_cuts`,
    /// pooled, is the bar's 0.75 share. Prints only, at every step.
    pub full_loads: usize,
    pub full_cuts: usize,
    /// Pellets whose cut earned S1 and that were never put down: still in
    /// the jaws at the bell, or lost with their ant.
    pub held: usize,
    pub dumped: usize,
    /// Hole cells ground again after the settle, where a pellet was put
    /// down and where none was (a fall).
    pub refill_pellet: usize,
    pub refill_fall: usize,
    /// Founding-cut cells ground again after the settle, a pellet's or a
    /// fall's: the nest filling, which step 4 gates (mean <= 0.5 a run;
    /// ladder lane, `dig-rung-design.md` 4). 62 of round 5's 70 falls landed
    /// there or in the cell cut under its floor.
    pub chamber_fill: usize,
    /// Of `refill_fall`, those within 2 cells of a pellet that left where it
    /// was put down without being cut back up: a put-down that rolled in.
    /// The rest were set off by a cut (round 5: every one of 83 grains,
    /// `stage2/dig/README.md`, "What drops soil into the founding chamber").
    /// Read from the end state, not traced frame by frame, so a pellet that
    /// rolled somewhere else beside a fall is counted as its source.
    pub fall_putdown: usize,
}

/// **Each pellet's trip, from the two logs** (`World::cut_log` and
/// `Minds::dump_log`, `(x, y, frame, ant)`), paired ant by ant in frame
/// order: a cut fills the jaws and the next put-down empties them.
///
/// * **S1 cut**: the cell was ground at frame 0 (`was_ground`) and nobody
///   had cut it earlier in the run. **Every later stage needs S1** (review
///   R7): an ant that cuts its own tailings back up and puts them down
///   again ("the tailings shuffler") earns nothing at any stage. **From
///   step 4, also below the original surface** (`depth` 1 or more, as S4
///   reads it; ladder lane, `dig-rung-design.md` rev 2.4): step 4's first
///   four runs cut about 4 cells in 5 from the top row, 12-13.5 cuts a
///   world against 2-2.8 deep, and stopped just past the cap, since a rim
///   pellet carried away filled S1-S3 as well as a deep one
///   (`stage2/dig/step4-sh4-README.md`). It is the trap `dead-ends.md`
///   records for the game ant's dig roll: paying for breaking original
///   ground pays for scraping a flat surface. So a rim cut earns nothing at
///   any stage, and a deep pellet put down away pays 7 and its S4. On bare
///   ground (step 7) a new hole's first cut pays nothing and the one under
///   it pays in full.
/// * **From step 4, S1-S3 pay by depth** (`dig-rung-design.md` line 149,
///   the response fixed in advance for review N2): each pellet's S1, S2
///   and S3 points are scaled by `min(1, depth / full_depth)`, and the
///   arena passes its shaft's rows, so at a 4-row shaft a cut one row
///   under the top pays a quarter and one at the shaft's foot in full.
///   The trap came back one row down, as N2 warned: on the gated scoring
///   seed 83's hold-best brought up 4.5 pellets a world against its
///   start's 1.67 (12 sealed worlds), each climbing 1.89 rows against the
///   start's 2.90, from a pit 1-2 rows deep widened beside the shaft
///   (`stage2/dig/step4-gate-README.md`). Under the cap an ant's deepest
///   pellets are the ones paid. A rim cut has depth 0 or less and pays
///   nothing, as above; `full_depth` 1 is rev 2.4's rule, every cut below
///   the top row in full. **The weight is squared** (rev 2.8): linear,
///   below the cap a pellet from `d` rows paid 2.75 `d` at a 4-row shaft
///   (1.75 from S1-S3, 1 from S4), so two pellets from 2 rows paid what
///   one from 4 did, and the depth-pay batch's diggers got more loads by
///   digging shallower (review of PR 726, D1: s101 2.2 -> 1.7 rows a
///   pellet, s102 2.3 -> 1.8). Squared, one pellet pays 1.44 / 3.75 / 6.94
///   / 11 from 1 / 2 / 3 / 4 rows, so eight from 2 rows pay 30 against
///   four from 4 rows' 44.
/// * **S2 let go**: the pellet was put down (on ground: the put-down
///   checks that, `mind::put_down_pellet`).
/// * **S4 up and out** (from step 4): a pellet that reached S3 earns
///   `SOIL_POINTS[3]` per [`SOIL_UP_ROWS`] rows it climbed from its cut to
///   its put-down, up to the full points. **Only climbing out of the
///   ground pays**: the rows counted are at most the cut's `depth`, the
///   rows under the lowest original surface within 3 columns of it (review
///   R8), so a cut into a raised step's face is not a pit and a shallow
///   pellet carried up a heap earns no more than its depth. Needs S3, as
///   each forage step needs the one before: a pellet carried up from the
///   deep and put down beside the hole is the shaft dumper's trip.
/// * **S3 away**: at least [`SOIL_AWAY`] cells, Chebyshev, from every hole
///   cell. **The hole** ([`soil_hole`]) is every cell of the original
///   ground cut this run and every open cell at or below the original
///   surface at frame 0 (`founding`): the founding cut is part of it, so
///   dumping down the shaft is not away (review R9). Two departures from
///   the design's text, both the same on the flat bed (review of PR 720,
///   N9): a re-cut pellet's site is not a hole cell (it was not ground at
///   frame 0), and the founding cut is every open cell under the surface,
///   not a flood from the door -- on a generator bed with voids (step 4 on)
///   those are not the same. A cell emptied by a fall rather than a cut
///   and then refilled with a pellet would earn S1 once; on tamped soil
///   that does not happen.
///
/// Stage `step` pays for every pellet; each stage under it for at most
/// [`SOIL_CAP`] an ant; stages above it nothing. From step 3, every hole
/// cell that is ground again (`ground_now`, read after the settle) costs
/// [`SOIL_REFILL`]. The score is not floored: a run that fills its own
/// hole reads below one that never dug.
#[allow(clippy::too_many_arguments)]
pub fn soil_trip(
    step: u8,
    full_depth: i32,
    was_ground: impl Fn(i32, i32) -> bool,
    ground_now: impl Fn(i32, i32) -> bool,
    depth: impl Fn(i32, i32) -> i32,
    founding: &[(i32, i32)],
    cuts: &[(i32, i32, u64, OrganismId)],
    dumps: &[(i32, i32, u64, OrganismId)],
) -> SoilTrip {
    soil_trip_at(step, full_depth, 1, was_ground, ground_now, depth, founding, cuts, dumps)
}

/// [`soil_trip`], with **the fewest rows down a first cut earns S1 from
/// step 4** (`s1_rows`): 1 is rev 2.4's rule, every cut under the top row;
/// the shaft's rows are step 4b's ([`Soil::foot`]), the foot only. Under
/// step 4 every first cut earns S1 whatever this says.
#[allow(clippy::too_many_arguments)]
pub fn soil_trip_at(
    step: u8,
    full_depth: i32,
    s1_rows: i32,
    was_ground: impl Fn(i32, i32) -> bool,
    ground_now: impl Fn(i32, i32) -> bool,
    depth: impl Fn(i32, i32) -> i32,
    founding: &[(i32, i32)],
    cuts: &[(i32, i32, u64, OrganismId)],
    dumps: &[(i32, i32, u64, OrganismId)],
) -> SoilTrip {
    use std::collections::{BTreeMap, HashSet};
    let mut cuts = cuts.to_vec();
    cuts.sort_by_key(|c| c.2);
    // S1, in the order the cuts happened: a cell counts once.
    let mut cut_before = HashSet::new();
    let hole = soil_hole(&was_ground, founding, &cuts);
    // One ant's acts by frame: a cut, with whether it earned S1 and how
    // deep it was, or a put-down (`None`), and where.
    type Acts = Vec<(u64, Option<(bool, i32)>, (i32, i32))>;
    let mut per_ant: BTreeMap<OrganismId, Acts> = BTreeMap::new();
    let step = step.clamp(1, 4) as usize;
    // How much of the shaft's depth `d` rows are, for the loads; a
    // pellet's S1-S3 are worth that squared from step 4, and in full under
    // it.
    let weight = |d: i32| (f64::from(d) / f64::from(full_depth.max(1))).clamp(0.0, 1.0);
    let pay = |d: i32| if step < 4 { 1.0 } else { weight(d).powi(2) };
    let mut load_base = 0.0;
    let (mut deep_cuts, mut rim_cuts, mut full_cuts) = (0, 0, 0);
    for &(x, y, f, id) in &cuts {
        let first = was_ground(x, y) && cut_before.insert((x, y));
        let d = depth(x, y);
        deep_cuts += usize::from(first && d > 0);
        if first && d > 0 {
            load_base += weight(d);
        }
        rim_cuts += usize::from(first && d <= 0);
        full_cuts += usize::from(first && d >= full_depth.max(1));
        let s1 = first && (step < 4 || d >= s1_rows.max(1));
        per_ant.entry(id).or_default().push((f, Some((s1, d)), (x, y)));
    }
    for &(x, y, f, id) in dumps {
        per_ant.entry(id).or_default().push((f, None, (x, y)));
    }
    let away = |at: (i32, i32)| soil_away(&hole, at);
    let mut trip = SoilTrip { cuts: cuts.len(), dumped: dumps.len(), deep: deep_cuts, rim: rim_cuts, load_base, full_cuts, ..SoilTrip::default() };
    for events in per_ant.values_mut() {
        // A cut and a put-down never share an ant's frame: one act a
        // decision.
        events.sort_by_key(|e| (e.0, e.1.is_none()));
        // Each stage's pellets, by what each is worth ([`soil_trip`]'s
        // depth pay).
        let mut n: [Vec<f64>; 3] = Default::default();
        // Each S4 pellet's rows, in the order they were put down.
        let mut ups: Vec<i32> = Vec::new();
        // What the jaws hold: whether its cut earned S1, the cut's depth
        // and row.
        let mut jaws: Option<(bool, i32, i32)> = None;
        for &(_, cut, at) in events.iter() {
            match cut {
                Some((s1, deep)) => {
                    if jaws.is_some_and(|j| j.0) {
                        trip.held += 1;
                    }
                    jaws = Some((s1, deep, at.1));
                    if s1 {
                        n[0].push(pay(deep));
                    }
                }
                None => {
                    if let Some((true, deep, cut_y)) = jaws {
                        n[1].push(pay(deep));
                        if away(at) {
                            n[2].push(pay(deep));
                            let lift = (cut_y - at.1).min(deep);
                            let rows = lift.min(SOIL_UP_ROWS);
                            if rows > 0 {
                                ups.push(rows);
                                trip.full_loads += usize::from(lift >= full_depth.max(1));
                            }
                        }
                    }
                    jaws = None;
                }
            }
        }
        trip.held += usize::from(jaws.is_some_and(|j| j.0));
        let paid = |k: usize, n: usize| match (k + 1).cmp(&step) {
            std::cmp::Ordering::Less => n.min(SOIL_CAP),
            std::cmp::Ordering::Equal => n,
            std::cmp::Ordering::Greater => 0,
        };
        // Under the cap, an ant's deepest pellets are the ones paid: one
        // more pellet never lowers its pay, and a shallow one cut first
        // does not crowd out a deep one cut later.
        for (k, pellets) in n.iter_mut().enumerate() {
            trip.stages[k] += pellets.len();
            pellets.sort_by(|a, b| b.total_cmp(a));
            trip.score += SOIL_POINTS[k] * pellets.iter().take(paid(k, pellets.len())).sum::<f64>();
        }
        trip.up += ups.len();
        trip.up_rows += ups.iter().sum::<i32>() as usize;
        trip.loads += ups.iter().map(|&r| weight(r)).sum::<f64>();
        let climbed = ups.iter().take(paid(3, ups.len())).sum::<i32>();
        trip.score += SOIL_POINTS[3] * f64::from(climbed) / f64::from(SOIL_UP_ROWS);
    }
    let dumped: HashSet<(i32, i32)> = dumps.iter().map(|d| (d.0, d.1)).collect();
    let chamber: HashSet<(i32, i32)> = founding.iter().copied().collect();
    let rolled: Vec<(i32, i32)> = dumps
        .iter()
        .filter(|d| !ground_now(d.0, d.1) && !cuts.iter().any(|c| (c.0, c.1) == (d.0, d.1) && c.2 > d.2))
        .map(|d| (d.0, d.1))
        .collect();
    for &(x, y) in &hole {
        if ground_now(x, y) {
            if dumped.contains(&(x, y)) {
                trip.refill_pellet += 1;
            } else {
                trip.refill_fall += 1;
                trip.fall_putdown += usize::from(rolled.iter().any(|r| (r.0 - x).abs().max((r.1 - y).abs()) <= 2));
            }
            trip.chamber_fill += usize::from(chamber.contains(&(x, y)));
        }
    }
    if step >= 3 {
        trip.score -= SOIL_REFILL * (trip.refill_pellet + trip.refill_fall) as f64;
    }
    trip
}

/// **The hole, as the soil trip scores it**: the founding cut, and every
/// cut cell that was ground at frame 0. One definition, for the score and
/// the picture's colours alike (review of PR 720, N3).
pub fn soil_hole(
    was_ground: impl Fn(i32, i32) -> bool,
    founding: &[(i32, i32)],
    cuts: &[(i32, i32, u64, OrganismId)],
) -> std::collections::HashSet<(i32, i32)> {
    let mut hole: std::collections::HashSet<(i32, i32)> = founding.iter().copied().collect();
    hole.extend(cuts.iter().filter(|c| was_ground(c.0, c.1)).map(|c| (c.0, c.1)));
    hole
}

/// Whether a pellet at `at` is [`SOIL_AWAY`] or more cells, Chebyshev,
/// from every cell of `hole`.
pub fn soil_away(hole: &std::collections::HashSet<(i32, i32)>, (x, y): (i32, i32)) -> bool {
    let r = SOIL_AWAY - 1;
    !(-r..=r).any(|dy| (-r..=r).any(|dx| hole.contains(&(x + dx, y + dy))))
}

/// **Ground, as the soil trip reads it**: a `Powder` or `Solid` cell that is
/// no animal's body, as [`dug_cells`] reads it.
fn is_ground(w: &World, x: i32, y: i32) -> bool {
    let c = w.get(x, y);
    c.material != material::EMPTY && matches!(w.materials.kind(c.material), material::MaterialKind::Powder | material::MaterialKind::Solid) && c.organism_id() == 0
}

/// **Rescale a species to a finite budget**, every standing bill kept at
/// what it is at the authored budget ([`never_starve`] is this at 1e9).
/// **The energy sense keeps the authored tank** (`CreatureDef::sensed_tank`):
/// the budget is the bills' unit, not what one tank feels like, so an ant
/// here feels its bank against the same tank as in the colony and the game.
/// Before 2026-10-22 it read a flat 0.5 in fetch and trail; copy v2 on
/// reliab's soil worlds, priced before the change: 10.71 -> 10.12 full
/// loads (-0.58 +- 0.32), fetch 19.83 -> 19.76, so brains trained before it
/// want a re-check.
fn budget_to(def: &mut organism::CreatureDef, joules: f32) {
    assert!(
        joules.is_finite() && joules > 0.0,
        "a budget of {joules} J: every bill priced as a fraction of it would be infinite, or free"
    );
    let authored = def.start_energy;
    def.sensed_tank.get_or_insert(authored);
    def.start_energy = joules;
    let k = authored / def.start_energy;
    def.synapse_fraction *= k;
    def.sight_fraction *= k;
    def.curvature_fraction *= k;
    def.force_fraction *= k;
    def.armour_fraction *= k;
}

impl Soil {
    /// The bed for one layout: [`DigBed`]'s depth and lamps, flat, to step
    /// 3 -- a mirrored pair would be the same flat bed twice, so each layout
    /// takes its own dice instead (`k`'s parity into the seed). **From step
    /// 4 the whole bed**, its stone and its step on the side `k` picks, so
    /// a pair differs by the side alone, as [`Dig`]'s do.
    fn bed(&self, layout: Layout) -> Scenario {
        let shape = DigBed::drawn(layout, self.step_near);
        if self.generated() {
            bed_shaped(layout.seed, self.ants, &shape.placements(), shape.depth, shape.lamp)
        } else {
            bed_shaped(layout.seed * 2 + (layout.k & 1), self.ants, "", shape.depth, shape.lamp)
        }
    }

    /// Whether the bed is [`DigBed`]'s whole draw, mirrored: from step 4.
    fn generated(&self) -> bool {
        self.step >= 4
    }

    /// **The world at frame 0**: the bed built, the shaft cut to `shaft`
    /// rows, the door column set, `genome` in every ant on its budget.
    /// Returns the world, its scenario, the ants' species and the budget.
    fn world(&self, layout: Layout, genome: &[f32]) -> (World, Scenario, SpeciesId, f32) {
        let sc = self.bed(layout);
        let (mut w, _) = sc.bed.build_counted();
        hold_training_world(&mut w);
        // **The build split** (review R4): the founding shaft is cut inside
        // `apply`, so what it reads is set before it.
        w.nest_shaft = Some(self.shaft);
        w.door_loose = Some(true);
        let mut spec = sc.bed.clone();
        sc.apply(&mut w, &mut spec);
        w.door_column = Some(creature::DoorColumn { drop: true, clear: false });
        w.births_paused = true;
        let sp = clone_minds(&mut w, genome, layout, self.verbs(), self.mind_emits());
        let mut def = w.species.get(sp).creature.clone().expect("creature");
        match self.budget {
            Some(j) => budget_to(&mut def, j),
            None => never_starve(&mut def),
        }
        let full = def.start_energy;
        w.species.set_creature(sp, def);
        for id in ants_of(&w, sp) {
            w.set_organism_energy(id, full);
        }
        w.cut_log = Some(Vec::new());
        w.minds.as_mut().expect("minds").dump_log = Some(Vec::new());
        (w, sc, sp, full)
    }
}

/// **Rows under the original ground at `(x, y)`**, for S4 (design section
/// 4, review R8): under the lowest frozen datum ([`World::room_surface_at`],
/// one per column from `x0`, in `datum`) within 3 columns either side. So a
/// cut into a raised step's face, which stands over the ground beside it,
/// is not a pit, and the founding shaft, cut after the datum froze, is no
/// surface. 0 or less above the ground, and 0 with no datum near.
fn depth_below_ground(datum: &[Option<i32>], x0: i32, x: i32, y: i32) -> i32 {
    (x - 3..=x + 3)
        .filter_map(|c| usize::try_from(c - x0).ok().and_then(|i| datum.get(i).copied().flatten()))
        .max()
        .map_or(0, |low| y - low)
}

/// **The hole at frame 0**: every open cell (empty, or an animal) at or
/// below the original surface, which on [`Soil`]'s flat bed is the founding
/// cut and nothing else.
fn founding_cut(w: &World, spec: &crate::lab::scene::LabBox) -> Vec<(i32, i32)> {
    let (xs, ys) = census::extents(w, spec);
    let mut out = Vec::new();
    for y in ys {
        for x in xs.clone() {
            let m = w.get(x, y).material;
            let open = m == material::EMPTY || w.materials.kind(m) == material::MaterialKind::Creature;
            if open && y >= census::surface_of(w, spec, x) {
                out.push((x, y));
            }
        }
    }
    out
}

impl Challenge for Soil {
    fn name(&self) -> &'static str {
        "soil"
    }
    /// **No food and no larvae in the soil box**, so Pick up, Eat and Feed
    /// never have a target: measured on s161 gen 336's 38,028 decisions over
    /// the 24 sealed worlds (investigation 2026-10-20), and guarded by
    /// `soil_never_offers_food_verbs`.
    fn acting_verbs(&self) -> Vec<creature::options::Verb> {
        use creature::options::Verb as V;
        SOIL_VERBS.iter().copied().filter(|v| !matches!(v, V::PickUp | V::Eat | V::Feed)).collect()
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
        &["score", "cuts", "s1", "s2", "s3", "held", "dumped", "refill_pellet", "refill_fall", "away_share", "alive", "energy_left", "dig_j", "holding", "spent", "chamber_fill", "fall_putdown", "s4", "up_rows", "deep_cuts", "rim_cuts", "loads", "load_base", "load_share", "full_loads", "full_cuts", "full_share", "nest"]
    }
    fn banned(&self) -> &'static [O] {
        NOT_ANT_VERBS
    }
    fn minds(&self) -> bool {
        true
    }
    fn verbs(&self) -> &'static [creature::options::Verb] {
        SOIL_VERBS
    }
    fn mirrored(&self) -> bool {
        self.generated()
    }
    /// Cut cells cyan, pellets put down near the hole amber and away from
    /// it green, read from the run's own logs: where the trip went, not
    /// what the renderer draws. Near and away are the score's own
    /// ([`soil_hole`], [`soil_away`]), the founding cut included, read off
    /// the layout's world at frame 0, which is the same world for any
    /// genome.
    fn marks(&self, layout: Layout, w: &World) -> Vec<((i32, i32), [u8; 3])> {
        let cuts = w.cut_log.clone().unwrap_or_default();
        let mut out: Vec<((i32, i32), [u8; 3])> = cuts.iter().map(|c| ((c.0, c.1), [0, 190, 255])).collect();
        let (w0, sc, _, _) = self.world(layout, &creature::mind::blank(SOIL_VERBS));
        let hole = soil_hole(|x, y| is_ground(&w0, x, y), &founding_cut(&w0, &sc.bed), &cuts);
        for d in w.minds.as_ref().and_then(|m| m.dump_log.as_ref()).into_iter().flatten() {
            out.push(((d.0, d.1), if soil_away(&hole, (d.0, d.1)) { [40, 220, 60] } else { [255, 190, 0] }));
        }
        out
    }
    fn run(&self, layout: Layout, genomes: &[&[f32]], look: &mut dyn FnMut(u64, &World)) -> Vec<Score> {
        let (mut w, sc, sp, full) = self.world(layout, genomes[0]);
        let placed = ants_of(&w, sp);
        // The ground and the hole as they stand before the first frame.
        let (xs, ys) = census::extents(&w, &sc.bed);
        let (x0, y0, nw) = (*xs.start(), *ys.start(), (xs.end() - xs.start() + 1) as usize);
        let at = |x: i32, y: i32| (xs.contains(&x) && ys.contains(&y)).then(|| (y - y0) as usize * nw + (x - x0) as usize);
        let mut ground0 = vec![false; nw * (ys.end() - y0 + 1) as usize];
        for y in ys.clone() {
            for x in xs.clone() {
                ground0[at(x, y).expect("inside")] = is_ground(&w, x, y);
            }
        }
        let founding = founding_cut(&w, &sc.bed);
        let datum: Vec<Option<i32>> = xs.clone().map(|x| w.room_surface_at(x)).collect();
        let deep = |x: i32, y: i32| depth_below_ground(&datum, x0, x, y);
        // SCRATCH (teacher-training measurement 2): the privileged facts a teacher reads, once a world: extents, the
        // depth datum per column, the founding hole, the ground at frame 0.
        if std::env::var("PP_MIND_LOG").is_ok() || creature::mind::scratch_log::copy_on() {
            let dat: Vec<String> = datum.iter().map(|d| d.map_or("x".into(), |v| v.to_string())).collect();
            let fc: Vec<String> = founding.iter().map(|(x, y)| format!("{x}:{y}")).collect();
            let g0: String = ground0.iter().map(|&b| if b { '1' } else { '0' }).collect();
            let row = format!("G,{},{x0},{y0},{nw},{},{},{},{},{g0}", w.seed, ground0.len() / nw, self.shaft, dat.join(" "), fc.join(" "));
            creature::mind::scratch_log::cline(&row);
            creature::mind::scratch_log::line(row);
        }
        // SCRATCH (teacher-training ceiling test): hand the teacher its facts for the run's frames only.
        if creature::mind::scratch_log::teach_on() || creature::mind::scratch_log::label_on() {
            let top = founding.iter().map(|c| c.1).min().unwrap_or(0);
            let mouth: Vec<i32> = founding.iter().filter(|c| c.1 == top).map(|c| c.0).collect();
            let ctx = creature::mind::scratch_log::TeachCtx {
                x0,
                y0,
                nw,
                nh: ground0.len() / nw,
                ground0: ground0.clone(),
                datum: datum.clone(),
                founding: founding.clone(),
                shaft: self.shaft,
                mouth_x: mouth.iter().sum::<i32>() as f32 / mouth.len().max(1) as f32,
                clear: SOIL_AWAY - 1 + creature::mind::scratch_log::arms().3,
            };
            creature::mind::scratch_log::TEACH.with(|t| *t.borrow_mut() = Some(ctx));
        }
        let dig_j0 = w.creature_stats.dig_energy;
        for f in 0..self.frames {
            frame(&mut w, true);
            look(f + 1, &w);
        }
        // The settle that follows is scored: the teacher must not act in it.
        creature::mind::scratch_log::TEACH.with(|t| *t.borrow_mut() = None);
        creature::mind::scratch_log::SETTLE.with(|s| s.set(true));
        let alive = ants_of(&w, sp);
        let holding = alive.iter().filter(|&&id| w.organism(id).is_some_and(|s| s.spoil.is_some())).count();
        let energy_left = alive.iter().filter_map(|&id| w.organism(id).map(|s| s.energy as f64)).sum::<f64>() / placed.len().max(1) as f64;
        // **Ants that ran their budget nearly dry** (dead, or under a tenth
        // left): where a champion does, the budget is the limit rather
        // than the skill (ladder lane, 2026-10-18).
        let spent = placed.iter().filter(|&&id| w.organism(id).is_none_or(|s| s.energy < 0.1 * full)).count();
        let dig_j = w.creature_stats.dig_energy - dig_j0;
        // **The settle**: every ant rests, topped up so none dies in it, and
        // the bed runs on, so what is scored is what stands once the work
        // stops.
        let rest = arena_mind(&creature::mind::preset("rest", self.verbs()).expect("rest"), self.verbs(), self.mind_emits());
        for &id in &alive {
            creature::mind::give(&mut w, id, rest.clone());
            w.set_organism_energy(id, full);
        }
        for _ in 0..DIG_SETTLE {
            frame(&mut w, true);
        }
        look(self.frames + DIG_SETTLE, &w);
        let cuts = w.cut_log.clone().unwrap_or_default();
        let dumps = w.minds.as_ref().and_then(|m| m.dump_log.clone()).unwrap_or_default();
        let s1_rows = if self.foot { self.shaft.max(1) } else { 1 };
        let trip = soil_trip_at(self.step, self.shaft, s1_rows, |x, y| at(x, y).is_some_and(|i| ground0[i]), |x, y| is_ground(&w, x, y), deep, &founding, &cuts, &dumps);
        creature::mind::scratch_log::SETTLE.with(|s| s.set(false));
        // SCRATCH (investigation): every cut and put-down, for tracing pellets offline -- into the
        // decision log, and the copy log (world-tagged) when that is on.
        if std::env::var("PP_MIND_LOG").is_ok() || creature::mind::scratch_log::copy_on() {
            let hole = soil_hole(|x, y| at(x, y).is_some_and(|i| ground0[i]), &founding, &cuts);
            let mut rows = Vec::new();
            for &(x, y, f, id) in &cuts {
                rows.push(format!("C,{},{f},{id},{x},{y},{},{}", w.seed, deep(x, y), u8::from(at(x, y).is_some_and(|i| ground0[i]))));
            }
            for &(x, y, f, id) in &dumps {
                rows.push(format!("D,{},{f},{id},{x},{y},{}", w.seed, u8::from(soil_away(&hole, (x, y)))));
            }
            rows.push(format!("E,{},{},{}", w.seed, trip.full_loads, trip.full_cuts));
            for r in rows {
                creature::mind::scratch_log::cline(&r);
                creature::mind::scratch_log::line(r);
            }
        }
        let away_share = if trip.stages[1] > 0 { trip.stages[2] as f64 / trip.stages[1] as f64 } else { 0.0 };
        // The nest as it stands after the settle, its door the founding
        // shaft's column. At steps 4-5 every ant's nest is a shaft and the
        // cuts round it, which read near zero whatever the ant did (s171 on
        // the 96 test worlds, 8-row shaft: -22.4 to 4.8, median -5.2, once
        // the open pit at the rim is charged; -13.5 to 6.3, median -2.5
        // before the per-column sky, which read the box's walls as ground at
        // the lid and every pit as 0): a baseline for the room steps, not a
        // signal yet ("Dig and nest scoring", 2026-10-22).
        let nest = match (self.nest, w.nest_sites.first()) {
            (true, Some(site)) => nestscore::of_world(&w, &sc.bed, site.x, &nestscore::HARVESTER).0.score,
            _ => 0.0,
        };
        vec![Score {
            fit: trip.score,
            parts: vec![
                trip.score,
                trip.cuts as f64,
                trip.stages[0] as f64,
                trip.stages[1] as f64,
                trip.stages[2] as f64,
                trip.held as f64,
                trip.dumped as f64,
                trip.refill_pellet as f64,
                trip.refill_fall as f64,
                away_share,
                alive.len() as f64,
                energy_left,
                dig_j,
                holding as f64,
                spent as f64,
                trip.chamber_fill as f64,
                trip.fall_putdown as f64,
                trip.up as f64,
                trip.up_rows as f64,
                trip.deep as f64,
                trip.rim as f64,
                trip.loads,
                trip.load_base,
                if trip.load_base > 0.0 { trip.loads / trip.load_base } else { 0.0 },
                trip.full_loads as f64,
                trip.full_cuts as f64,
                // This world's share, 0 with no full cut. Averaged over the
                // worlds it is **not** the bar's pooled share, mean(full_loads)
                // / mean(full_cuts): s81's hold-best reads 0.50 here and 1.00
                // pooled, and `load_share` is the same (review of PR 726, N9).
                if trip.full_cuts > 0 { trip.full_loads as f64 / trip.full_cuts as f64 } else { 0.0 },
                // Last, so every part before it keeps its column: the brain
                // lane's `judge.py` and saved test-score files read
                // `full_loads` by position.
                nest,
            ],
        }]
    }
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
/// **Not an exact mirror** (review of PR 722, N4): the shaft is the two
/// columns `NEST_X` and `NEST_X + 1`, an east feature starts at `NEST_X + d`
/// and its west twin ends at `NEST_X - d - 1`, so the east one sits two
/// columns nearer the shaft. The worse-side fold leans slightly east for
/// it. Dig's colony arena has drawn its beds this way since it took them;
/// left as it is, since moving the west side would change every bed it
/// has scored.
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
    /// One feature's draw for the bed on `seed`: `lo..=hi`, a hash of the
    /// seed and the feature's `salt`, so a pair's two sides draw alike.
    fn roll(seed: u64, salt: u64, lo: i32, hi: i32) -> i32 {
        let mut z = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ salt.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        lo + ((z ^ (z >> 31)) % (hi - lo + 1) as u64) as i32
    }

    /// **[`DigBed::of`], with a step forced near the shaft on a share
    /// `near` of the pairs** ([`Soil::step_near`]): there the step is drawn
    /// 6-10 columns out instead of 6-30, mirrored as ever, its height as
    /// drawn. Why: on 192 hold-out beds at the 8-row shaft (2026-10-21) the
    /// step within 10 columns of the shaft, 40 beds of 192, cost both
    /// trained brains 1.4 and 1.7 full loads (2 s.e.), and the 4-row brain
    /// they came from only 0.3: the walk out to a fixed put-down that a
    /// flat bed pays. A step anywhere else, the stone, the soil depth and
    /// the lamps cost nothing measurable, and a plain bed (no lens, no step,
    /// one depth) is no less noisy -- the per-world spread is the run's own.
    /// 0 is [`DigBed::of`] exactly, bed for bed.
    pub fn drawn(layout: Layout, near: f64) -> DigBed {
        let mut bed = Self::of(layout);
        let draw = |salt: u64, lo: i32, hi: i32| Self::roll(layout.seed, salt, lo, hi);
        if near > 0.0 && f64::from(draw(11, 0, 9999)) < near * 10_000.0 {
            bed.step = Some((NEST_X + bed.side * draw(12, 6, 10), draw(9, 2, 6)));
        }
        bed
    }

    pub fn of(layout: Layout) -> DigBed {
        let draw = |salt: u64, lo: i32, hi: i32| Self::roll(layout.seed, salt, lo, hi);
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
/// packed soil above the datum is a lid and below it a roof. It cost an
/// honest nest nothing on the flat beds before [`DigBed`]: 946 cells either
/// way over the game ant's 12 hold-out layouts (`ladder/review-ladder-docs.md`
/// s2). On the varied beds, `dug_cells`' amber (refused) count beside the
/// room says the same thing per run: 0 on the game ant's first hold-out bed.
pub fn joined_room(w: &World, spec: &crate::lab::scene::LabBox) -> usize {
    dug_cells(w, spec).iter().filter(|c| matches!(c.1, Dug::Room | Dug::Filled)).count()
}

/// What [`joined_room`] made of an open cell at or below the datum.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dug {
    /// Counted: roofed by the old ground and joined to the sky.
    Room,
    /// Counted too, but the only roof over it is packed soil: no undisturbed
    /// soil or stone above it below the datum. Packed soil below the datum
    /// is a wall lined in place, so it roofs; it is also what a pellet laid
    /// in a gallery below the ground line packs into, so a colony that
    /// backfills and digs under its own fill counts it as room. Painted and
    /// counted apart (`under_fill`) so that can be seen (coordinator review
    /// of PR 709: free=holds lets the scorer lay pellets below the line).
    ///
    /// **Not a backfill detector on its own: it has a floor.** The hand
    /// rules read 5-12 a bed (mean 8.7 on the 12 hold-outs) with
    /// `spoil_below` at 0 or 1: cells in the two columns beside the founding
    /// shaft, roofed by soil packed during the run, most likely the ants'
    /// lining of the widened shaft (`line_burrow` packs every neighbour of
    /// a cut; inferred from where it sits, not traced, coordinator review of
    /// PR 710). The material cannot tell lining from a packed pellet, so
    /// read `under_fill` against the hand rules' row, never against 0;
    /// `dropped_inside` counts the scorer's pellets below the datum.
    Filled,
    /// Open to the sky in its own column (no ground within
    /// [`census::MOUND_REACH`] rows above it): the census's `pit` and
    /// `pit_bodies`, and never room.
    Pit,
    /// Neither: under a roof that is not the old ground (a spoil lid, a
    /// mound), or roofed but cut off from the sky. The census calls these
    /// roofed; the score does not.
    Refused,
}

/// Every open cell at or below the frozen datum, with what the score made
/// of it, so a picture can show what was counted (`Challenge::marks`).
pub fn dug_cells(w: &World, spec: &crate::lab::scene::LabBox) -> Vec<((i32, i32), Dug)> {
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
    let packed = w.materials.id_of("packedsoil");
    let mut cells = Vec::new();
    for x in xs.clone() {
        let surf = surface[(x - x0) as usize];
        let mut roofed = false;
        let mut natural = false;
        let mut covered = false;
        for y in ys.clone() {
            if ground(x, y) {
                // The census's cover: any ground near the surface, mound and
                // lids included ([`census::census`]'s `covered`).
                covered |= y >= surf - census::MOUND_REACH;
                let m = Some(w.get(x, y).material);
                roofed |= y >= surf && m != spoil;
                natural |= y >= surf && m != spoil && m != packed;
            } else if y >= surf && open(x, y) {
                let kind = if roofed && seen[at(x, y)] {
                    if natural {
                        Dug::Room
                    } else {
                        Dug::Filled
                    }
                } else if covered {
                    Dug::Refused
                } else {
                    Dug::Pit
                };
                cells.push(((x, y), kind));
            }
        }
    }
    cells
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
/// reference is the start genome's own score there on the calibration
/// layouts ([`Suite::calibrate`]). A product rather than a sum, so a brain
/// that is excellent at one job and useless at another scores near zero;
/// and an arena marked `keep` (an old skill) that falls below `forget` of
/// its reference (0.8, design rev3's "below 80% of where it was")
/// multiplies the whole score by a tenth, so forgetting is never worth it. Per-ant arenas are run with
/// every ant a clone.
///
/// **Each arena is folded on its own, then multiplied** ([`Challenge::
/// combine`]): the worse side of each mirrored pair, arena by arena. The
/// first suite folded the product, so a brain that foraged only east and
/// dug only on the west bed scored `sqrt(hi x lo)` on both sides and passed
/// as having learned both (ladder lane, `next-rung-forage-dig.md`). The
/// forget floor is read on the folded score for the same reason: per
/// layout, one bad bed of a noisy arena tripped it.
///
/// **The floor is a share of the reference, not standard errors.** Read
/// as two standard errors of the start's calibration spread, it sat at 95%
/// of the trail rung's reference (spread 0.035 on 0.898): the forage
/// rung's first pilot cut every forager whose trail fell to 0.76-0.85, so
/// selection read the cut, not the foraging (15 gens, scarce 0.04 -> 1.71
/// with trail 0.92 -> 0.76, scored 0.16 against the start's 0.28). On a
/// noisy arena the same rule was too loose -- forage at `layouts=2` could
/// lose 57% (review of PR 712, N2). A share is the design's rule and
/// neither. It is still unpaired, the start's calibration mean against
/// this generation's layouts, so keep a noisy arena only once it is
/// paired. **A ratio counts up to
/// [`RATIO_CAP`]**, so one arena's runaway cannot pay for another's loss --
/// **except the one arena a rung is learning** (every other arena kept):
/// a kept arena's loss is the forget floor's to catch, and capped, the
/// rung's arena stopped paying at three times a start that could barely do
/// it. The forage rung's pilot from the trail champion hit the cap on
/// scarce (reference 0.21) by its second generation, at fits of 0.6-1.4
/// against the game ant's 19.5, and selection had nothing left to climb.
///
/// **Gated** ([`Suite::gated`], from the dig rung's step 4): a kept arena
/// is a pass/fail gate. Above its floor it counts 1 and its ratio never
/// enters the product; under it, the same tenth. So the fit is the
/// learned arenas' alone, and a lone learned arena's fit is exactly its
/// ratio. Step 4's first four runs (`stage2/dig/step4-sh4-README.md`)
/// selected on the kept colony's luck: its ratio ran 0.44-1.61, about 0.32
/// in log fitness against about 0.015 for one more pellet carried up, and
/// s81's last champion gave up soil 62.7 -> 41.3 for it. Clamping at
/// `min(ratio, 1)` keeps the downside noise (log ratios to about -0.82,
/// still about twenty pellets), so the ladder lane chose the gate
/// (`dig-rung-design.md` rev 2.4). Each kept arena adds an `_under` part,
/// 1 where it is under its floor, so a run can print how often each gate
/// vetoes: a gate that vetoes often is a noise source of its own.
pub struct Suite {
    pub arenas: Vec<(Box<dyn Challenge>, bool)>,
    pub refs: Vec<f64>,
    /// Each arena's spread at calibration: the standard deviation of its
    /// folded units (a mirrored pair's worse side, or one layout).
    pub sds: Vec<f64>,
    /// The share of its reference under which a `keep` arena counts as
    /// forgotten.
    pub forget: f64,
    /// Kept arenas are pass/fail gates, not ratios ([`Suite::gated`]).
    pub gates: bool,
    names: &'static [&'static str],
    banned: &'static [O],
    /// Every verb any arena offers: a mind's genome carries them all, and
    /// each arena cuts it to its own ([`arena_mind`]).
    verbs: &'static [creature::options::Verb],
}

/// The most one arena's ratio to its reference counts for in a suite, when
/// it is kept or is one of several being learned ([`Suite`]).
/// **Relative to the start**: an arena the start cannot do has the floor
/// [`REF_FLOOR`] for its reference, so it stops paying at a fit of 0.15 --
/// by design (a weak start must not chase one arena), but it means a suite
/// cannot *learn* an arena from a start near 0 (review of PR 712, N3).
pub const RATIO_CAP: f64 = 3.0;

/// The least an arena's reference can be, so a start that cannot do it at
/// all pays for any progress rather than dividing by zero.
const REF_FLOOR: f64 = 0.05;

impl Suite {
    /// `arenas`: each challenge, and whether it is an old skill to keep.
    /// **No target scorer**: the genome's scorer would go into every arena's
    /// world, and `free=` reaches only dig's (coordinator review of PR 709,
    /// N8; refused here, not only on the command line, since PR 710's).
    /// Arena `i`'s fit against its reference, capped at [`RATIO_CAP`] unless
    /// it is the one arena this suite is learning.
    fn ratio(&self, i: usize, fit: f64) -> f64 {
        let r = fit.max(0.0) / self.refs[i];
        let learning = self.arenas.iter().filter(|(_, keep)| !keep).count();
        if self.arenas[i].1 || learning > 1 {
            r.min(RATIO_CAP)
        } else {
            r
        }
    }

    /// **Where kept arena `i` counts as forgotten**, its folded score read
    /// over `units` folded units: `forget` of its reference, or for an
    /// arena that asks for it ([`Challenge::floor_in_se`]) the reference
    /// less two standard errors of a mean over `units`, from the start's
    /// calibration spread (R12). The share is the rule everywhere else for
    /// the reason above: as standard errors it sat at 95% of trail's
    /// reference and cut every forager whose trail dipped. The colony with
    /// Dig is the other case, a noisy arena, where a share cuts on noise.
    fn floor(&self, i: usize, units: usize) -> f64 {
        if self.arenas[i].0.floor_in_se() {
            self.refs[i] - 2.0 * self.sds[i] / (units.max(1) as f64).sqrt()
        } else {
            self.forget * self.refs[i]
        }
    }

    pub fn new(arenas: Vec<(Box<dyn Challenge>, bool)>, forget: f64) -> Self {
        assert!(forget > 0.0 && forget <= 1.0, "forget={forget}: a share of the reference, 0 to 1 (0.8 is the design's)");
        assert!(arenas.iter().all(|(a, _)| !a.scorer()), "a suite would put the target scorer in every arena's world");
        let mut names: Vec<&'static str> = vec![];
        for (a, _) in &arenas {
            names.push(Box::leak(format!("{}_ratio", a.name()).into_boxed_str()));
        }
        for (a, _) in &arenas {
            names.push(Box::leak(format!("{}_fit", a.name()).into_boxed_str()));
        }
        for (a, _) in arenas.iter().filter(|(_, keep)| *keep) {
            names.push(Box::leak(format!("{}_under", a.name()).into_boxed_str()));
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
        let verbs: Vec<creature::options::Verb> = creature::options::Verb::ALL
            .into_iter()
            .filter(|v| arenas.iter().any(|(a, _)| a.verbs().contains(v)))
            .collect();
        let refs = vec![1.0; arenas.len()];
        let sds = vec![0.0; arenas.len()];
        Suite {
            arenas,
            refs,
            sds,
            forget,
            gates: false,
            names: Box::leak(names.into_boxed_slice()),
            banned: Box::leak(banned.into_boxed_slice()),
            verbs: Box::leak(verbs.into_boxed_slice()),
        }
    }

    /// **Whether a suite is [`gated`](Suite::gated)**: from the dig rung's
    /// step 4, when soil is in it and is learned rather than kept
    /// (`dig-rung-design.md` rev 2.4). `examples/train.rs` asks this; the
    /// review of PR 723 (N4) found the switch had no test of its own.
    pub fn gated_at(step: u8, arenas: &str, keep: &str) -> bool {
        step >= 4 && arenas.split(',').any(|n| n == "soil") && !keep.split(',').any(|k| k == "soil")
    }

    /// The same suite with its kept arenas as pass/fail gates (see
    /// [`Suite`]).
    pub fn gated(mut self) -> Self {
        self.gates = true;
        self
    }

    /// The fit from each arena's ratio: the geometric mean over every
    /// arena, or with [`Suite::gates`] over the learned arenas alone (a
    /// kept arena counts 1), and a tenth where a kept arena is under its
    /// floor.
    fn product(&self, ratios: &[f64], forgot: bool) -> f64 {
        let counted: Vec<f64> =
            ratios.iter().zip(&self.arenas).filter(|(_, (_, keep))| !(self.gates && *keep)).map(|(r, _)| *r).collect();
        let fit = if counted.is_empty() {
            1.0
        } else {
            (counted.iter().map(|r| r.max(1e-3).ln()).sum::<f64>() / counted.len() as f64).exp()
        };
        if forgot {
            fit * 0.1
        } else {
            fit
        }
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
    /// Minds when every arena runs them. Unlike today's brain, whose bans
    /// are the union, a mind's verbs and scents are what *any* arena offers,
    /// and each arena runs it cut to its own ([`arena_mind`]): the forage
    /// rung's scent trail is no ban on the trail rung's stamped one.
    fn minds(&self) -> bool {
        self.arenas.iter().all(|(a, _)| a.minds())
    }
    fn verbs(&self) -> &'static [creature::options::Verb] {
        self.verbs
    }
    fn unfloored(&self, s: &Score) -> f64 {
        self.product(&s.parts[..self.arenas.len()], false)
    }
    fn untested_verbs(&self) -> Vec<creature::options::Verb> {
        let learning: Vec<creature::options::Verb> = self.arenas.iter().filter(|(_, keep)| !keep).flat_map(|(a, _)| a.acting_verbs()).collect();
        self.verbs.iter().copied().filter(|v| !learning.contains(v)).collect()
    }
    fn mind_emits(&self) -> [bool; creature::mind::EMITS] {
        let mut on = [false; creature::mind::EMITS];
        for (a, _) in &self.arenas {
            for (o, e) in on.iter_mut().zip(a.mind_emits()) {
                *o |= e;
            }
        }
        on
    }
    /// Worse side of a pair if any arena has sides: one brain that does one
    /// arena's job on one side only has not learned the job.
    fn mirrored(&self) -> bool {
        self.arenas.iter().any(|(a, _)| a.mirrored())
    }
    /// One layout: every arena run on it, and its fit the product of this
    /// layout's capped ratios, for a per-layout readout only. What training
    /// and the hold-out read is [`Suite::combine`]'s, from the raw fits in
    /// the second half of `parts`.
    fn run(&self, layout: Layout, genomes: &[&[f32]], look: &mut dyn FnMut(u64, &World)) -> Vec<Score> {
        let g = genomes[0];
        let mut ratios = vec![];
        let mut fits = vec![];
        let mut under = vec![];
        for (i, (a, keep)) in self.arenas.iter().enumerate() {
            let gs: Vec<&[f32]> = vec![g; if a.unit() == Unit::Ant { a.ants() } else { 1 }];
            let mut quiet = |_: u64, _: &World| {};
            let watcher: &mut dyn FnMut(u64, &World) = if i == 0 { &mut *look } else { &mut quiet };
            let s = super::mean_score(&a.run(layout, &gs, watcher));
            ratios.push(self.ratio(i, s.fit));
            fits.push(s.fit);
            if *keep {
                under.push(f64::from(u8::from(s.fit < self.floor(i, 1))));
            }
        }
        let fit = self.product(&ratios, false);
        ratios.extend(fits);
        ratios.extend(under);
        vec![Score { fit, parts: ratios }]
    }

    /// Each arena folded on its own ([`super::worse_side`] on its raw fits),
    /// then the geometric mean of the capped ratios ([`Suite::product`]: of
    /// the learned arenas only when gated), with the forget floor read on
    /// each `keep` arena's folded score: under `forget` of its reference.
    /// `parts` are each arena's folded ratio, then its folded fit, then for
    /// each kept arena 1 where it is under its floor.
    fn combine(&self, v: &[Score], layouts: &[Layout]) -> Score {
        let n = self.arenas.len();
        let mut ratios = vec![];
        let mut fits = vec![];
        let mut under = vec![];
        for (i, (a, keep)) in self.arenas.iter().enumerate() {
            let own: Vec<Score> = v.iter().map(|s| Score { fit: s.parts[n + i], parts: vec![] }).collect();
            let f = super::worse_side(a.mirrored(), &own, layouts).fit;
            if *keep {
                under.push(f64::from(u8::from(f < self.floor(i, super::sides(a.mirrored(), layouts).len()))));
            }
            ratios.push(self.ratio(i, f));
            fits.push(f);
        }
        let fit = self.product(&ratios, under.iter().any(|&u| u > 0.0));
        ratios.extend(fits);
        ratios.extend(under);
        Score { fit, parts: ratios }
    }

    /// Set every arena's reference to the start's folded score there
    /// (floored at a small positive number, so an arena the start cannot do
    /// at all pays for any progress rather than dividing by zero), and its
    /// spread over the folded units, printed beside it. **A kept arena's
    /// reference is the start as saved** (`saved`), before the rung's entry
    /// (`mind::enter_rung`), so harm the entry does is never taken as the
    /// baseline (R12); where the two differ, the entered start is scored on
    /// each kept arena too and printed against its floor, a pre-flight.
    fn calibrate(&mut self, g: &[f32], saved: &[f32], layouts: &[Layout], threads: usize) {
        for i in 0..self.arenas.len() {
            let (a, keep) = (&self.arenas[i].0, self.arenas[i].1);
            let folded = |g: &[f32]| -> Vec<f64> {
                let each = super::score_each(a.as_ref(), g, layouts, threads);
                super::sides(a.mirrored(), layouts)
                    .into_iter()
                    .map(|(x, y)| y.map_or(each[x].fit, |y| each[x].fit.min(each[y].fit)))
                    .collect()
            };
            let units = folded(if keep { saved } else { g });
            let m = units.len().max(1) as f64;
            let mean = units.iter().sum::<f64>() / m;
            let name = a.name();
            self.refs[i] = mean.max(REF_FLOOR);
            self.sds[i] = (units.iter().map(|u| (u - mean).powi(2)).sum::<f64>() / (m - 1.0).max(1.0)).sqrt();
            let whose = if keep && saved != g { " (the start as saved, or keepref=)" } else { "" };
            println!("suite reference {name}: {:.3}{whose}, spread {:.3} over {} units", self.refs[i], self.sds[i], units.len());
            if keep && saved != g {
                let now = folded(g);
                let f = now.iter().sum::<f64>() / now.len().max(1) as f64;
                let floor = self.floor(i, now.len());
                let under = if f < floor { " -- UNDER THE FLOOR: the entry itself costs this arena" } else { "" };
                println!("suite pre-flight {name}: the entered start scores {f:.3} against {:.3}, floor {floor:.3}{under}", self.refs[i]);
            }
            let a = &self.arenas[i].0;
            if mean < REF_FLOOR && self.ratio(i, f64::MAX) <= RATIO_CAP {
                println!(
                    "suite reference {}: the start cannot do it, so its reference is the floor {REF_FLOOR} and the arena stops paying at fit {:.2} (RATIO_CAP x the floor)",
                    a.name(),
                    RATIO_CAP * REF_FLOOR
                );
            }
        }
    }
}

/// **The bed and the mind of in-run colony selection** (`lab::training`):
/// the teacher lane's harness (`examples/inrun.rs` on its compute branch,
/// 2026-10-21) selects ants inside one living colony on this same 256 x 128
/// training bed, each founder given a mind cut to the dig rung's verbs. The
/// game's training page runs that loop, so it needs the four things the
/// harness reached through its scratch copy of this file: the bed, the
/// mind, the slots a child mutates and the suite a founder enters by.
pub mod colony_bed {
    use super::*;

    /// The nest's column on the bed.
    pub const NEST: i32 = NEST_X;
    /// The bed's width and height.
    pub const SIZE: (i32, i32) = (BED_W, 128);

    /// The scarce-dig arena's emits: what its minds may signal. Under
    /// `PIXEL_PHYSICS_MIND_ATTACK=on` the alarm scent too ([`verbs`]).
    pub fn emits() -> [bool; creature::mind::EMITS] {
        let mut e = Scarce { ants: 1, frames: 1, drip: 1, every: 1, game_walk: false, brake: None, dist: (40, 40), steps: false, fed: false, dig: true }.mind_emits();
        // SCRATCH (lone predator thread): `PP_MIND_ALARM=off` keeps the alarm emit shut with the bite on.
        e[2] |= creature::options::attack_on() && std::env::var("PP_MIND_ALARM").map_or(true, |v| v != "off");
        // SCRATCH (evolvable-pheromones lane, 2026-10-25): `PP_MIND_ALARM=train` opens the alarm emit row to mutation
        // without the bite. **Why**: on the bed the alarm row is outside `slots()`, so a pool's alarm emission is a
        // frozen leftover (std_pool lays ~690k units a 200k round, mostly empty ants away from home, and deafening it
        // moves nothing: decode, 2026-10-25). The emergence test needs that free channel free to evolve.
        e[2] |= std::env::var("PP_MIND_ALARM").is_ok_and(|v| v == "train");
        e
    }

    /// The colony's verbs: the dig rung's ([`SOIL_VERBS`]), and under
    /// `PIXEL_PHYSICS_MIND_ATTACK=on` the bite (lone predator thread, stage
    /// two, 2026-10-24): **a colony with a predator on its bed may fight
    /// back**, and may call for help with the alarm scent ([`emits`]). Off,
    /// exactly the dig rung's. A founder banned from biting by an earlier rung
    /// takes the verb by [`creature::mind::enter_bite`], which the harness
    /// calls on the genome it breeds from; one that skips it keeps the bite
    /// switched off.
    pub fn verbs() -> &'static [creature::options::Verb] {
        if creature::options::attack_on() {
            BITE_VERBS
        } else {
            SOIL_VERBS
        }
    }

    const BITE_VERBS: &[creature::options::Verb] = {
        use creature::options::Verb as V;
        &[V::Walk, V::TurnAround, V::Rest, V::PickUp, V::Eat, V::Drop, V::Feed, V::Dig, V::Attack]
    };

    /// A founder's or a child's mind: `genome` cut to the colony's verbs
    /// ([`verbs`]) and emits, as the harness runs it.
    pub fn mind(genome: &[f32]) -> std::sync::Arc<creature::mind::MindGenome> {
        arena_mind(genome, verbs(), emits())
    }

    /// The slots a child's mutation may move: the scarce-dig trainer's, and
    /// the bite's and the alarm's under the switch.
    pub fn slots() -> Vec<usize> {
        let mut v = creature::mind::trainable(verbs(), emits());
        // SCRATCH (dig-scoring thread): an evolvable dig head's slots, past GENOME_LEN (`PP_DIGHEAD_EVOLVE`)
        if let Some(hl) = creature::mind::scratch_log::head_evolve() {
            v.extend(creature::mind::GENOME_LEN..creature::mind::GENOME_LEN + hl);
        }
        v
    }

    /// The suite a founder genome enters by ([`super::super::start_mind`]):
    /// the harness's own, soil + scarce + trail, so a file read here enters
    /// the rung exactly as `founders=`/`mutants=` read it there.
    pub fn suite() -> Suite {
        Suite::new(
            vec![
                (Box::new(Soil { ants: 1, frames: 12000, step: 4, shaft: 8, budget: Some(2400.0), foot: true, step_near: 0.0, nest: false }), false),
                (
                    Box::new(Scarce { ants: 1, frames: 6000, drip: 5, every: 1000, game_walk: false, brake: Some(creature::FoodBrake::FORAGE), dist: (10, 30), steps: true, fed: true, dig: false }),
                    true,
                ),
                (Box::new(Trail { ants: 6, frames: 6000, clones: false }), true),
            ],
            0.8,
        )
    }

    /// **The bed with no colony on it yet**: the training bed's scenario
    /// (`bed_timed`, 40 rows of soil, lamps 33 apart, no rain) with its
    /// `Colony` placement left out, because the colony is founded heaped at
    /// the door (`scenario::place_colony` with `pile`), which the harness
    /// asked for through the process-wide `PIXEL_PHYSICS_NEST_DOOR_FOUNDERS`
    /// and a game that also runs an ordinary box cannot.
    pub fn scenario(seed: u64, ants: usize) -> Scenario {
        let mut sc = bed_timed(seed, ants, "", 40, 33, BED_W, NEST_X, "");
        sc.placements.retain(|p| !matches!(p, crate::lab::scenario::Placement::Colony { .. }));
        sc
    }
}

/// SCRATCH (teacher lane, in-run selection pilot, 2026-10-21): the rest of
/// `examples/inrun.rs`'s bed -- one colony on the training bed with Dig on
/// and surface food dripped at `dist` columns, every ant given its own mind,
/// the body's egg-laying off. Only the harness calls it. Landed on `main`
/// 2026-10-22 (owner: no work parked on branches) with the scratch switches
/// it reads, every one off unless its variable or argument is set; off, an
/// hA1 batch came back byte-identical to the compute branch.
pub mod inrun {
    use super::*;

    pub const NEST: i32 = colony_bed::NEST;

    /// SCRATCH (teacher lane, 2026-10-24): `PP_BED_W=W` widens the in-run bed to `W` columns, eastward only -- the
    /// door stays at [`NEST`], 128 from the west wall, so every reader of the door's column is unchanged. Default
    /// [`BED_W`] (256), the bed exactly. **Why**: the lab box's food sits 100-240 columns from a colony, and the food
    /// ramp (colsel `distramp=`) stopped at 110 because a 256-wide bed has no room past it: laygene2's ramp reached
    /// 110 at round 15 and held, its pool lived 110-130 ants with fruit 150 columns out and 9-37 at 200, where the
    /// hand-built ant lived 114-132. Training never asked for a walk the box needs. A heap that would fall past the
    /// west wall is dropped beside the east one (`examples/inrun.rs`), so far food is one-sided.
    /// SCRATCH (nest lane, 2026-10-29): `PP_BED_SOIL=D` gives the in-run bed `D` rows of soil over the stone floor
    /// instead of 40, up to 60 (the bed is 128 rows with the ground at [`GROUND_Y`], so 60 leaves 4 rows of stone).
    /// Default 40, the bed exactly. **Why** (owner, 2026-10-29: "our dirt isn't deep enough, at some point we're just
    /// hitting the rock at the bottom"): the trained ant's cone and the hand-written digger's tunnels both reach the
    /// stone at row 104 by the end of the first 200k-frame round, so any nest shape below that is decided by the floor.
    pub fn bed_soil() -> i32 {
        static D: std::sync::OnceLock<i32> = std::sync::OnceLock::new();
        *D.get_or_init(|| std::env::var("PP_BED_SOIL").ok().and_then(|v| v.trim().parse().ok()).map_or(40, |d: i32| d.clamp(40, 60)))
    }

    pub fn bed_w() -> i32 {
        static W: std::sync::OnceLock<i32> = std::sync::OnceLock::new();
        *W.get_or_init(|| std::env::var("PP_BED_W").ok().and_then(|v| v.trim().parse().ok()).filter(|&w: &i32| w >= BED_W).unwrap_or(BED_W))
    }

    // The mind, its emits and a child's slots are the game's training page's own ([`colony_bed`]), which main took
    // from this module when the in-run loop moved into the lab (`lab::training`).
    pub use super::colony_bed::{emits, mind, slots, verbs};

    /// The bed and its colony. `genome_of(k)` is the k-th founder's mind
    /// (founders in id order).
    pub fn colony(seed: u64, ants: usize, dist: i32, drip: u32, every: u64, genome_of: &dyn Fn(usize) -> std::sync::Arc<creature::mind::MindGenome>) -> (World, Scenario, SpeciesId) {
        // Food is topped up by the harness ([`top_up`]), capped, on both sides:
        // an uncapped drip buried the nest under provisions by frame 20,000
        // (two heaps of 50 cells every 500 frames, 88 ants eating ~35 cells a
        // thousand frames), and one-sided food starved the founders standing
        // on the other side (60 founders, food east: 27 of 60 by frame 1,500).
        let _ = (dist, drip, every);
        colony_in(seed, ants, &Garden::default(), genome_of)
    }

    /// SCRATCH (plant-food lane, 2026-10-23): **a planted bed** -- the colony eats what living plants make (seed,
    /// fruit, litter, leaf) instead of the harness's pile. Off by default: `plants` empty is the bare bed exactly.
    ///
    /// `plants`: `(species, column)` founders, placed as the lab's `Plant` placement places them. `grow`: frames the
    /// bed runs with no colony on it, so the colony lands on a grown stand, as the played bed's does at 6,000 (a colony
    /// on seedlings is a colony on nothing). `rain`: the mister's setting (`lab::rain::Rain` index); the bare bed runs
    /// it at 0, and a garden with no water is a garden that dies.
    #[derive(Clone, Default)]
    pub struct Garden {
        pub plants: Vec<(String, i32)>,
        pub grow: u64,
        pub rain: Option<f32>,
    }

    impl Garden {
        /// `MIX[:SPACING[:BAND]]`, e.g. `grass,herb,shrub,scrambler:12:40`: the species in turn every `SPACING`
        /// columns (default 12) across the bed, none within `BAND` columns of the nest (default 40, the colony's
        /// footing and its first chambers: a grown bed has no bare ground left to found on, `played_bed.ron`).
        pub fn parse(spec: &str, grow: u64, rain: Option<f32>) -> Garden {
            let mut it = spec.split(':');
            let mix: Vec<&str> = it.next().unwrap_or("").split(',').filter(|s| !s.is_empty()).collect();
            let spacing: i32 = it.next().map_or(12, |v| v.parse().expect("plants=MIX:SPACING"));
            let band: i32 = it.next().map_or(40, |v| v.parse().expect("plants=MIX:SPACING:BAND"));
            let mut plants = Vec::new();
            if !mix.is_empty() {
                let mut k = 0;
                let mut x = spacing / 2 + 2;
                while x < bed_w() - 2 {
                    if (x - NEST).abs() > band {
                        plants.push((mix[k % mix.len()].to_string(), x));
                        k += 1;
                    }
                    x += spacing;
                }
            }
            Garden { plants, grow, rain }
        }
    }

    /// [`colony`] on a [`Garden`]: the plants placed and grown for `grow` frames first, then the colony founded on
    /// the grown bed. The datum is the bare bed's (frozen at build, before anything grew).
    pub fn colony_in(seed: u64, ants: usize, garden: &Garden, genome_of: &dyn Fn(usize) -> std::sync::Arc<creature::mind::MindGenome>) -> (World, Scenario, SpeciesId) {
        let mut sc = bed_timed(seed, ants, "", bed_soil(), 33, bed_w(), NEST_X, "");
        let w = if garden.plants.is_empty() {
            sc.build().0
        } else {
            if let Some(r) = garden.rain {
                sc.settings.retain(|s| s.field != "rain");
                sc.settings.push(crate::lab::scenario::Setting { subject: "the bed".into(), field: "rain".into(), value: r });
            }
            let colony: Vec<_> = sc.placements.iter().filter(|p| matches!(p, crate::lab::scenario::Placement::Colony { .. })).cloned().collect();
            sc.placements.retain(|p| !matches!(p, crate::lab::scenario::Placement::Colony { .. }));
            for (sp, x) in &garden.plants {
                sc.placements.push(crate::lab::scenario::Placement::Plant { species: sp.clone(), x: *x });
            }
            let (mut w, _, _) = sc.build();
            for _ in 0..garden.grow {
                frame_timed(&mut w, &sc, true);
            }
            crate::lab::scenario::apply_placements(&mut w, &colony, &sc.bed);
            w
        };
        let mut w = w;
        w.births_paused = true;
        let sp = w.species.id_of("ant").expect("ant");
        let mut def = w.species.get(sp).creature.clone().expect("creature");
        def.reproduce_threshold = 1.0e30;
        // No old age (owner, 2026-10-21): selection's removals are the only deaths but starvation.
        def.life_half_life = 0;
        // SCRATCH (foraging-loop thread): `PP_SYNAPSE_K=f` scales the brain's per-link bill, to price a copy's grown
        // links (c1 1,347 against h1's 881) at the founder's bill and see whether the bill is what starves it.
        if let Some(k) = std::env::var("PP_SYNAPSE_K").ok().and_then(|v| v.parse::<f32>().ok()) {
            def.synapse_fraction *= k;
        }
        w.species.set_creature(sp, def);
        // SCRATCH (foraging-loop thread's switch, 2026-10-22): PP_NOMIND=1 leaves every founder on the game's own ant
        // (chooser + act), the old ant, in this same bed. Unset: no change.
        let nomind = std::env::var("PP_NOMIND").is_ok_and(|v| v == "1");
        for (k, id) in ants_of(&w, sp).into_iter().enumerate() {
            if !nomind {
                creature::mind::give(&mut w, id, genome_of(k));
            }
        }
        // `PP_SHADOW=1` (mind.rs): minds watch the old ant, so the world builds the old ant's ways as it does with none.
        // `PP_OLDLAB=1` ticks the old ant in clones of this world, so the same.
        if creature::mind::scratch_log::shadow_on() || creature::mind::scratch_log::oldlab_on() {
            if let Some(m) = w.minds.as_mut() {
                m.build_ways = true;
            }
        }
        (w, sc, sp)
    }

    /// Provisions within `reach` columns of `x`, at or above the datum's
    /// lowest row there plus `below` (the heap and what rolled off it).
    pub fn food_near(w: &World, x: i32, reach: i32) -> usize {
        food_near_of(w, x, reach, "provisions")
    }

    /// [`food_near`] for any food material (`inrun food=`).
    pub fn food_near_of(w: &World, x: i32, reach: i32, material: &str) -> usize {
        let Some(p) = w.materials.id_of(material) else { return 0 };
        let mut n = 0;
        for cx in (x - reach)..=(x + reach) {
            for y in 0..128 {
                if w.get(cx, y).material == p {
                    n += 1;
                }
            }
        }
        n
    }

    /// Top the heap at `x` up towards `cap` cells, at most `drip` at a time.
    pub fn top_up(w: &mut World, sc: &Scenario, x: i32, cap: usize, drip: usize, spread: i32) -> usize {
        top_up_with(w, sc, x, cap, drip, spread, "provisions")
    }

    /// [`top_up`] with another food material (`inrun food=`): the lab's
    /// boxes feed on fruit as much as provisions, and a colony trained only
    /// on provisions reached ~60 ants on fruit against ~200 on provisions
    /// (`examples/labsurvive.rs`, food 25 columns out, 2026-10-24).
    pub fn top_up_with(w: &mut World, sc: &Scenario, x: i32, cap: usize, drip: usize, spread: i32, material: &str) -> usize {
        let have = food_near_of(w, x, 8 + spread, material);
        if have >= cap {
            return 0;
        }
        let add = (cap - have).min(drip) as i32;
        // spread>0 scatters the food over x±spread instead of one heap (a 90-ant
        // colony jammed in two clumps a few columns short of single heaps).
        let heap = if spread > 0 {
            crate::lab::scenario::Placement::Scatter { material: material.into(), x0: x - spread, x1: x + spread, cells: add }
        } else {
            crate::lab::scenario::Placement::Heap { material: material.into(), x, cells: add }
        };
        crate::lab::scenario::apply_placements(w, &[heap], &sc.bed);
        add as usize
    }

    pub fn step(w: &mut World, sc: &Scenario) {
        frame_timed(w, sc, true);
    }

    pub fn ants(w: &World, sp: SpeciesId) -> Vec<OrganismId> {
        ants_of(w, sp)
    }

    /// A child of the colony at its door: placed as a founder is (`plant_founder_in`, homed on the door), given
    /// `genome` and `grants` full grants. Tries the door's columns and the rows above it until the body fits.
    pub fn birth(w: &mut World, sp: SpeciesId, genome: std::sync::Arc<creature::mind::MindGenome>, grants: f32) -> Option<OrganismId> {
        let any = *ants_of(w, sp).first()?;
        let (colony, anchor) = w.organism(any).map(|s| (s.colony, s.forage_anchor))?;
        birth_in(w, sp, colony, anchor, genome, grants)
    }

    /// [`birth`] into a named colony and anchor, for a colony with no ant left alive to read them from.
    pub fn birth_in(w: &mut World, sp: SpeciesId, colony: u32, anchor: (i32, i32), genome: std::sync::Arc<creature::mind::MindGenome>, grants: f32) -> Option<OrganismId> {
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

    /// Remove an ant on the engine's own death path (a corpse, the slot freed), by id: `slay` by position hit the wrong
    /// ant wherever ants overlapped.
    pub fn cull(w: &mut World, id: OrganismId) -> bool {
        creature::cull_organism(w, id)
    }

    /// Set body trait `k` to `v` on every living ant (brood lane's `hatch_grant=`: the founders' birth-grant allele).
    pub fn set_trait(w: &mut World, sp: SpeciesId, k: usize, v: f32) {
        for id in ants_of(w, sp) {
            if let Some(s) = w.organism_mut(id) {
                s.traits[k] = v;
            }
        }
    }

    /// Every founder on `grants` full grants (the founding deals staggered reserves).
    pub fn flat_reserves(w: &mut World, sp: SpeciesId, grants: f32) {
        let full = grants * w.species.get(sp).creature.as_ref().map_or(0.0, |d| d.start_energy);
        for id in ants_of(w, sp) {
            if let Some(s) = w.organism_mut(id) {
                s.energy = full;
            }
        }
    }

    /// The frozen ground datum of column `x` (the dig arena's).
    pub fn surface(w: &World, sc: &Scenario, x: i32) -> i32 {
        census::surface_of(w, &sc.bed, x)
    }

    /// Open to an ant: empty, or an animal's body (the dig arena's `open`).
    pub fn open(w: &World, x: i32, y: i32) -> bool {
        let m = w.get(x, y).material;
        m == material::EMPTY || w.materials.kind(m) == material::MaterialKind::Creature
    }

    /// One cell for a picture: `.` open, `a` an animal, `f` provisions, `#` anything else.
    pub fn cell_char(w: &World, x: i32, y: i32) -> char {
        let m = w.get(x, y).material;
        if m == material::EMPTY {
            '.'
        } else if w.materials.kind(m) == material::MaterialKind::Creature {
            'a'
        } else if Some(m) == w.materials.id_of("provisions") {
            'f'
        } else {
            '#'
        }
    }

    /// Ground as the dig arena counts it: powder or solid, not a body.
    pub fn ground(w: &World, x: i32, y: i32) -> bool {
        let c = w.get(x, y);
        c.material != material::EMPTY && matches!(w.materials.kind(c.material), material::MaterialKind::Powder | material::MaterialKind::Solid) && c.organism_id() == 0
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

    /// **A suite runs a mind with every verb any arena offers, and each
    /// arena cuts it to its own** ([`arena_mind`]): the forage rung's suite
    /// offers eating, putting down, feeding and scent, and its trail arena
    /// runs that forager with walk, turn and rest alone and laying nothing.
    #[test]
    fn a_suite_mind_carries_every_verb_and_each_arena_cuts_it() {
        use creature::options::Verb;
        let scarce = Scarce {
            ants: 12,
            frames: 10,
            drip: 5,
            every: 1000,
            game_walk: false,
            brake: Some(creature::FoodBrake::FORAGE),
            dist: SCARCE_DIST,
            steps: true,
            fed: false,
            dig: false,
        };
        let trail = Trail { ants: 2, frames: 10, clones: true };
        let (trail_verbs, trail_emits) = (trail.verbs(), trail.mind_emits());
        let s = Suite::new(vec![(Box::new(scarce), false), (Box::new(trail), true)], 0.8);
        assert!(s.minds());
        assert!(s.verbs().len() == SCARCE_VERBS.len() && SCARCE_VERBS.iter().all(|v| s.verbs().contains(v)), "{:?}", s.verbs());
        assert_eq!(s.mind_emits(), [true, true, false]);
        let mut g = creature::mind::blank(s.verbs());
        creature::mind::enter_rung(&mut g, s.verbs(), s.mind_emits());
        let whole = creature::mind::MindGenome::new(g.clone());
        let cut = arena_mind(&g, trail_verbs, trail_emits);
        assert!(whole.live > 0 && cut.live == 0, "the scent layer is {} live weights whole, {} cut", whole.live, cut.live);
        // The verb switches lead the genome.
        for v in [Verb::PickUp, Verb::Eat, Verb::Drop, Verb::Feed] {
            assert!(cut.w[v as usize] < 0.0 && whole.w[v as usize] > 0.0, "{} on the trail arena", v.name());
        }
    }

    /// **A mind file names its layout, and one saved before Pick up had its
    /// act converts once** (fix plan step 1; `save_genome`,
    /// `mind::split_pick_up`): an unmarked file loads as the converter
    /// leaves it, and saved again it carries today's layout and loads
    /// unchanged, so a champion is never converted twice. The converter
    /// includes the last-food memory's entry (`mind::enter_last_food`):
    /// an unmarked file is older than layout 5 too.
    #[test]
    fn a_mind_file_names_its_layout_and_an_old_one_converts_once() {
        let ch = Scarce { ants: 2, frames: 0, drip: 5, every: 1000, game_walk: false, brake: None, dist: SCARCE_DIST, steps: true, fed: false, dig: false };
        let n = creature::mind::GENOME_LEN;
        let old: Vec<f32> = (0..n).map(|i| ((i * 7919 % 1000) as f32 / 500.0 - 1.0) * 0.5).collect();
        let dir = std::env::temp_dir().join(format!("train-layout-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (unmarked, marked) = (dir.join("old.txt"), dir.join("new.txt"));
        let body: String = old.iter().enumerate().map(|(i, w)| format!("{i} {w}\n")).collect();
        std::fs::write(&unmarked, format!("genome {n}\n{body}")).unwrap();
        assert_eq!(super::super::genome_file_layout(&unmarked), None);
        let mut want = old.clone();
        creature::mind::split_pick_up(&mut want);
        creature::mind::enter_last_food(&mut want, None);
        creature::mind::apply_bans(&mut want, ch.verbs(), ch.mind_emits());
        let got = super::super::start_mind_saved(&ch, unmarked.to_str().unwrap());
        assert_eq!(got, want, "an unmarked file converts");
        assert_ne!(got, { let mut o = old.clone(); creature::mind::apply_bans(&mut o, ch.verbs(), ch.mind_emits()); o }, "the conversion did something");
        super::super::save_genome(&marked, &got).unwrap();
        assert_eq!(super::super::genome_file_layout(&marked), Some(creature::mind::LAYOUT));
        assert_eq!(super::super::start_mind_saved(&ch, marked.to_str().unwrap()), got, "a marked file converted twice");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// **The scarce arena's food arrives on the timeline**: five cells of
    /// provisions at the spot on the first frame and five more by frame
    /// 1,001, ticked by `frame_timed` -- and none at all under plain
    /// `frame`, the positive control that the cells come from the drip and
    /// not the bed (review of PR 712, N5: no test ran a bed with a
    /// timeline).
    #[test]
    fn the_scarce_drip_lands_on_the_timeline() {
        let ch = Scarce { ants: 2, frames: 0, drip: 5, every: 1000, game_walk: true, brake: None, dist: SCARCE_DIST, steps: true, fed: false, dig: false };
        let layout = Layout { seed: 1, k: 0, blind: false, legal: false };
        let (_, food_x, _) = ch.geometry(layout);
        let food_near = |w: &World| -> usize {
            let food = w.materials.id_of("provisions").expect("provisions");
            let mut n = 0;
            for x in food_x - 12..=food_x + 12 {
                for y in 0..128 {
                    n += usize::from(w.get(x, y).material == food);
                }
            }
            n
        };
        let sc = ch.bed(layout);
        let (mut timed, _, _) = sc.build();
        let (mut plain, _, _) = sc.build();
        assert_eq!(food_near(&timed), 0, "the bed starts with food at the spot");
        let mut at = Vec::new();
        for f in 1..=1002u64 {
            frame_timed(&mut timed, &sc, true);
            frame(&mut plain, true);
            if f == 2 || f == 1002 {
                at.push(food_near(&timed));
            }
        }
        assert_eq!(at, vec![5, 10], "the drip at frames 1 and 1,001");
        assert_eq!(food_near(&plain), 0, "food landed without the timeline tick");
    }

    /// **A dead colony ranks by how long it lasted, always under a live
    /// one, and a live score is untouched** ([`scarce_floor`]). Put the
    /// fault back (the floor returning `score`) and the first assert reads
    /// 0 against 0. Tight asserts on a pure function, not watched red.
    #[test]
    fn a_dead_colony_ranks_by_how_long_it_lasted_and_never_above_a_live_one() {
        let early = scarce_floor(0.0, 0.2, 12);
        let late = scarce_floor(0.0, 0.6, 12);
        assert!(0.0 < early && early < late, "dying later ranks higher: {early} {late}");
        // The least a live colony reads: one founder at one of four reads.
        let least_live = 1.0 / 12.0 / 4.0;
        assert!(scarce_floor(0.0, 1.0, 12) < least_live);
        assert!(scarce_floor(0.0, 9.0, 12) < least_live, "a colony grown past its founders still clamps");
        assert_eq!(scarce_floor(least_live, 0.0, 12), least_live);
        assert_eq!(scarce_floor(14.5, 0.7, 12), 14.5);
        assert_eq!(scarce_floor(0.0, 0.0, 12), 0.0);
        assert_eq!(scarce_floor(0.0, 0.5, 0), 0.5 / 4.0 / 2.0, "no founders: no division by zero");
    }

    /// **Each step on the way home outweighs every step below it together**
    /// ([`forage_steps`]): the whole way short of the nest scores under one
    /// cell home, each step pays on its own, more cells home keep paying,
    /// and a colony that never left home scores nothing.
    #[test]
    fn each_step_home_outweighs_every_step_below_it() {
        assert_eq!(forage_steps(0.0, 0.0, 0.0, 0.0, 0.0, 0.0), 0.0);
        let below = |k: usize| {
            let s: Vec<f64> = (0..5).map(|j| f64::from(u8::from(j < k))).collect();
            forage_steps(s[0], s[1], s[2], s[3], s[4], 0.0)
        };
        let one = |k: usize| {
            let mut s = [0.0; 6];
            s[k] = 1.0;
            forage_steps(s[0], s[1], s[2], s[3], s[4], s[5])
        };
        for k in 1..6 {
            assert!(one(k) > below(k), "step {k}: {} against {} below it", one(k), below(k));
        }
        assert!(forage_steps(1.0, 1.0, 1.0, 1.0, 1.0, 8.0) > forage_steps(1.0, 1.0, 1.0, 1.0, 1.0, 1.0));
        assert!(
            forage_steps(0.0, 0.0, 0.0, 0.4, 0.0, 0.0) > forage_steps(0.0, 0.0, 0.0, 0.2, 0.0, 0.0),
            "carrying further pays"
        );
        assert!(
            forage_steps(0.4, 0.0, 0.0, 0.0, 0.0, 0.0) > forage_steps(0.2, 0.0, 0.0, 0.0, 0.0, 0.0),
            "heading further out pays"
        );
        assert_eq!(
            forage_steps(3.0, 3.0, 3.0, 3.0, 3.0, 0.0),
            15.5,
            "a share past 1 counts as 1"
        );
        assert_eq!(
            forage_steps(0.0, 0.0, 0.0, 0.0, 0.0, 0.5),
            0.0,
            "no cell home is no cell home"
        );
    }

    /// **A world of minds builds no way out, and plays exactly as if it
    /// had** (`creature::mind::every_ant_has_one`), run with the mound's and
    /// the open air's ways skipped and again built (`Minds::build_ways`):
    /// every ant ends on the same cells and every count is the same. Two
    /// beds, so both the drop sites and the hatchlings are read: a scarce
    /// colony of hand-built foragers that breeds, and one fed forager at 12
    /// columns that puts food down at the nest. The skip fired on every
    /// refresh, hatchlings included: only the built runs have a way out.
    /// And a world with one ant that has no mind builds them again.
    #[test]
    fn a_world_of_minds_skips_the_ways_out_and_plays_the_same() {
        let colony = Scarce {
            ants: 12,
            frames: 6000,
            drip: 5,
            every: 1000,
            game_walk: false,
            brake: None,
            dist: (30, 30),
            steps: true,
            fed: false,
            dig: false,
        };
        let fetch = Scarce {
            ants: 1,
            dist: (12, 12),
            fed: true,
            dig: false,
            ..colony
        };
        let forager = creature::mind::preset("forager", colony.verbs()).expect("forager");
        let play = |ch: &Scarce, k: u64, build: bool| {
            let layout = Layout {
                seed: 50_000 + k / 2,
                k,
                blind: false,
                legal: false,
            };
            let sc = ch.bed(layout);
            let (mut w, _, _) = sc.build();
            w.food_brake = ch.brake;
            let sp = clone_minds(&mut w, &forager, layout, ch.verbs(), ch.mind_emits());
            if ch.fed {
                let mut def = w.species.get(sp).creature.clone().expect("creature");
                never_starve(&mut def);
                w.species.set_creature(sp, def);
                for id in ants_of(&w, sp) {
                    w.organism_mut(id).expect("ant").energy = 1.0e9;
                }
                w.births_paused = true;
            }
            w.minds.as_mut().expect("minds").build_ways = build;
            let mut ways = 0;
            for _ in 0..ch.frames {
                frame_timed(&mut w, &sc, true);
                ways = ways.max(w.mound_ways.len() + w.mound_in_ways.len() + w.out_ways.len());
            }
            let ants: Vec<_> = ants_of(&w, sp)
                .iter()
                .map(|&id| w.organism(id).map(|s| s.chain.clone()))
                .collect();
            let st = w.creature_stats;
            (format!("{ants:?} {st:?}"), ways, (st.deliveries, st.births))
        };
        let (skipped, none, (_, born)) = play(&colony, 0, false);
        let (built, some, _) = play(&colony, 0, true);
        assert!(born > 0, "the colony bred");
        assert_eq!((none, some > 0), (0, true), "the colony's skip fired on every refresh");
        assert_eq!(skipped, built, "the colony");
        let k = (0..12)
            .find(|&k| play(&fetch, k, false).2.0 > 0)
            .expect("the forager puts food down at the nest");
        let (skipped, none, _) = play(&fetch, k, false);
        let (built, some, _) = play(&fetch, k, true);
        assert_eq!((none, some > 0), (0, true), "the fetch skip fired on layout {k}");
        assert_eq!(skipped, built, "fetch, layout {k}");
        // One ant that walks by the game's rules brings them back (review of
        // PR 717, N14): a skip blind to mindless ants passes all of the above.
        let layout = Layout {
            seed: 50_000,
            k: 0,
            blind: false,
            legal: false,
        };
        let sc = colony.bed(layout);
        let (mut w, _, _) = sc.build();
        let sp = clone_minds(&mut w, &forager, layout, colony.verbs(), colony.mind_emits());
        assert!(creature::mind::every_ant_has_one(&w), "every ant has a mind");
        let first = ants_of(&w, sp)[0];
        w.minds.as_mut().expect("minds").forget(first);
        assert!(!creature::mind::every_ant_has_one(&w), "one ant without one");
        let mut ways = 0;
        for _ in 0..2 * creature::REST_REFRESH {
            frame_timed(&mut w, &sc, true);
            ways = ways.max(w.mound_ways.len() + w.mound_in_ways.len() + w.out_ways.len());
        }
        assert!(ways > 0, "a mixed world builds the ways out");
    }

    /// **A cell shuffled across the nest's edge pays nothing, and one
    /// carried home from the heap pays once** (`Trip`, review of PR 717,
    /// B1). The mind's swallow marks any food taken off the nest as from a
    /// trip, so before the heap's own count these 50 laps booked 50 cells
    /// home, worth more than a real delivery. The ant is moved and its crop
    /// written as the swallow and the put-down write them, with the food from
    /// [`Scarce::DIST_MIN`] out to the bed's edge.
    #[test]
    fn a_cell_shuffled_across_the_nest_edge_pays_nothing() {
        for d in [Scarce::DIST_MIN, 40, 110] {
            let (laps, home, want) = shuffle(d);
            assert_eq!(laps, (0, 0, 0, 0.0), "50 laps across the edge at {d}");
            assert_eq!(home, want, "one cell home from the heap, then 50 more laps, at {d}");
        }
        // The floor is load-bearing: below it the doorstep is the heap.
        assert!(shuffle(Scarce::DIST_MIN - 2).0.0 > 0, "laps pay below the floor");
    }

    /// A trip as (home, picked, arrived, carried).
    type TripRead = (u64, usize, usize, f64);

    /// The 50 laps alone, then one cell from the heap and 50 more laps, and
    /// what the second should read.
    fn shuffle(d: i32) -> (TripRead, TripRead, TripRead) {
        let ch = Scarce {
            ants: 1,
            frames: 3000,
            drip: 5,
            every: 1000,
            game_walk: false,
            brake: None,
            dist: (d, d),
            steps: true,
            fed: true,
            dig: false,
        };
        let layout = Layout {
            seed: 50_000,
            k: 0,
            blind: false,
            legal: false,
        };
        let (nest_x, food_x, dist) = ch.geometry(layout);
        let (mut w, _, _) = ch.bed(layout).build();
        let rest = creature::mind::preset("rest", ch.verbs()).expect("rest");
        let sp = clone_minds(&mut w, &rest, layout, ch.verbs(), ch.mind_emits());
        let id = ants_of(&w, sp)[0];
        // The whole body on one cell, so the nest's reach reads that cell.
        let set = |w: &mut World, (x, y): (i32, i32), cells: u16, put: u32| {
            let s = w.organism_mut(id).expect("ant");
            s.chain.iter_mut().for_each(|c| *c = (x, y));
            s.trip_cells = cells;
            s.crop = (cells > 0).then_some(organism::Crop {
                material: material::EMPTY,
                shade: 0,
                unit: 1.0,
                cells,
                digesting: 0.0,
                passenger: None,
            });
            s.life.trip_deliveries = put;
        };
        // The nest's edge: the last cell at the nest towards the food on the
        // first row the nest reaches, and the next one out.
        let side = (food_x - nest_x).signum();
        let at = |w: &mut World, p: (i32, i32)| {
            set(w, p, 0, 0);
            creature::at_nest(w, id)
        };
        let y = (0..128).find(|&y| at(&mut w, (nest_x, y))).expect("a nest at nest_x");
        let inside = (0..dist)
            .map(|d| (nest_x + side * d, y))
            .take_while(|&p| at(&mut w, p))
            .last()
            .expect("nest");
        let outside = (inside.0 + side, y);
        let mut trip = Trip::default();
        let mut put = 0;
        let look = |w: &mut World, p: (i32, i32), cells: u16, put: u32, trip: &mut Trip| {
            set(w, p, cells, put);
            trip.look(w, &[id], nest_x, food_x, dist);
        };
        let laps = |w: &mut World, trip: &mut Trip, put: &mut u32| {
            for _ in 0..50 {
                look(w, outside, 1, *put, trip);
                *put += 1;
                look(w, inside, 0, *put, trip);
            }
        };
        laps(&mut w, &mut trip, &mut put);
        assert_eq!(put, 50, "the world booked every lap");
        let first = (trip.home, trip.picked.len(), trip.arrived.len(), trip.carried);
        let mut x = food_x;
        while x != inside.0 {
            look(&mut w, (x, y), 1, put, &mut trip);
            x -= side;
        }
        put += 1;
        look(&mut w, inside, 0, put, &mut trip);
        laps(&mut w, &mut trip, &mut put);
        // Last read loaded one column out, before the put-down at the nest.
        let walked = f64::from(dist - (outside.0 - nest_x).abs()) / f64::from(dist);
        let second = (trip.home, trip.picked.len(), trip.arrived.len(), trip.carried);
        (first, second, (1, 1, 1, walked))
    }

    /// **The fetch arena reads the trip** (`Scarce::fed`): one ant that
    /// cannot starve, the steps its whole score. An ant that never moves
    /// scores the head alone, 1, on every layout; the hand-built forager
    /// reaches the food and picks some up on at least one -- the positive
    /// control that the trip is read at all. `steps=off` is the head alone.
    #[test]
    fn the_fetch_arena_scores_the_trip_and_a_still_ant_scores_none() {
        let ch = Scarce {
            ants: 1,
            frames: 3000,
            drip: 5,
            every: 1000,
            game_walk: false,
            brake: None,
            dist: SCARCE_DIST,
            steps: true,
            fed: true,
            dig: false,
        };
        let layouts: Vec<Layout> = (0..4u64)
            .map(|k| Layout {
                seed: 50_000 + k / 2,
                k,
                blind: false,
                legal: false,
            })
            .collect();
        let part = |s: &Score, name: &str| s.parts[ch.parts().iter().position(|p| *p == name).expect(name)];
        let rest = creature::mind::preset("rest", ch.verbs()).expect("rest");
        for &l in &layouts {
            let s = &ch.run(l, &[&rest], &mut |_, _| {})[0];
            assert_eq!(
                (s.fit, part(s, "steps"), part(s, "heads")),
                (1.0, 0.0, 1.0),
                "a still ant on {l:?}"
            );
        }
        // The forager below its egg bar: a fed ant feels rich, and a rich
        // forager goes home to feed instead of out to forage.
        let mut forager = creature::mind::preset("forager", ch.verbs()).expect("forager");
        creature::mind::below_the_egg_bar(&mut forager);
        let runs: Vec<Score> = layouts
            .iter()
            .map(|&l| ch.run(l, &[&forager], &mut |_, _| {})[0].clone())
            .collect();
        let best = runs.iter().max_by(|a, b| a.fit.total_cmp(&b.fit)).expect("runs");
        assert!(
            part(best, "reach") == 1.0 && part(best, "picked") == 1.0 && best.fit > 1.0,
            "the forager's best trip: {:?}",
            best.parts
        );
        let off = Scarce { steps: false, ..ch };
        let s = &off.run(layouts[0], &[&forager], &mut |_, _| {})[0];
        assert_eq!(s.fit, part(s, "heads"), "steps=off scores the head alone");
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
        // **Spoil below the datum is not old ground** (the `!= spoil`
        // clause, untested until the PR 708 review): the same gallery with
        // its first six columns roofed by backfilled spoil instead keeps
        // only the other six. The census still counts all 24.
        let spoil = w.materials.id_of("spoil").expect("spoil");
        for xx in x + 2..x + 8 {
            for yy in s..s + 6 {
                w.set(xx, yy, Cell::new(spoil, 0));
            }
        }
        let ids = Ids::resolve(&w);
        assert_eq!(census::census(&w, &spec, 0.0, &[x], &ids).roofed_room(), 24, "the census does not roof it either, so this case tests nothing");
        assert_eq!(joined_room(&w, &spec), 12, "a gallery under backfilled spoil counted as room");
        let (mut w, spec) = bare_bed();
        clear(&mut w, x, s + 3, 10, 8);
        clear(&mut w, x + 5, s, 1, 3);
        assert_eq!(joined_room(&w, &spec), 72, "a 10x8 chamber under three rows of old ground, less the 8 under its way in");
    }

    /// **The picture's colours are the score's and the census's own
    /// split** (`dug_cells`, painted by `Challenge::marks`). On every shape
    /// of the lid test: room is `joined_room`, pit is the census's pit, and
    /// room plus refused is the census's roofed room. So a cell painted as
    /// pit is one the census calls open to the sky, and one painted refused
    /// is one the census would count and the score does not. The lid with a
    /// gap is pinned as numbers too (its gap column is pit, the rest
    /// refused), so the split cannot pass by being empty.
    #[test]
    fn dug_cells_split_like_the_census() {
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
        let count = |cells: &[((i32, i32), Dug)], k: Dug| cells.iter().filter(|c| c.1 == k).count();
        type Shape = fn(&mut World, i32, i32, &dyn Fn(&mut World, i32, i32, i32, i32), &dyn Fn(&mut World, i32, i32, i32, i32));
        let shapes: [(&str, Shape); 4] = [
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
            ("shaft and gallery", |w, x, s, clear, _| {
                clear(w, x, s, 2, 8);
                clear(w, x + 2, s + 6, 12, 2);
            }),
            ("chamber under old ground", |w, x, s, clear, _| {
                clear(w, x, s + 3, 10, 8);
                clear(w, x + 5, s, 1, 3);
            }),
        ];
        for (i, (name, shape)) in shapes.into_iter().enumerate() {
            let (mut w, spec) = bare_bed();
            let ids = Ids::resolve(&w);
            let x = spec.width / 2 + 22;
            let s = census::surface_of(&w, &spec, x);
            let before = census::census(&w, &spec, 0.0, &[x], &ids);
            assert!(dug_cells(&w, &spec).is_empty(), "the bare bed has open cells below the datum");
            shape(&mut w, x, s, &clear, &lay);
            let c = census::census(&w, &spec, 0.0, &[x], &ids);
            let cells = dug_cells(&w, &spec);
            let (room, pit, refused) = (count(&cells, Dug::Room) + count(&cells, Dug::Filled), count(&cells, Dug::Pit), count(&cells, Dug::Refused));
            assert_eq!(room, joined_room(&w, &spec), "{name}");
            assert_eq!(pit, c.pit + c.pit_bodies - before.pit - before.pit_bodies, "{name}: pit is not the census's");
            assert_eq!(room + refused, c.roofed_room() - before.roofed_room(), "{name}: room and refused are not the census's roofed room");
            if i == 0 {
                assert_eq!((room, pit, refused), (0, 8, 72), "{name}: the gap column is open, the rest is under the lid");
            }
        }
        // **Room under fill** is room, painted apart: a 10x2 gallery three
        // rows down, its first five columns roofed only by packed soil laid
        // below the datum (a backfill), the other five by the old ground.
        let (mut w, spec) = bare_bed();
        let x = spec.width / 2 + 22;
        let s = census::surface_of(&w, &spec, x);
        clear(&mut w, x, s, 1, 5);
        clear(&mut w, x + 1, s + 3, 10, 2);
        lay(&mut w, x + 1, s, 5, 3);
        let cells = dug_cells(&w, &spec);
        assert_eq!((count(&cells, Dug::Room), count(&cells, Dug::Filled)), (10, 10), "backfilled columns are room under fill, the rest room");
        assert_eq!(joined_room(&w, &spec), 20, "room under fill still counts");
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

    /// **The far heap lies clear of home and of the wall too**, at both ends
    /// of [`FAR_DIST`] and on both sides, on the far bed ([`far_bed`]), and
    /// the two sides mirror. The positive control: on a bed with only 10
    /// columns past the peak, the same heap does run into the wall.
    #[test]
    fn the_trail_rungs_controls_rank_on_the_minds_own_walk() {
        // **Stage 2's first rung, its controls on the mind's act path**
        // (`creature::mind`; ladder lane, `trail-rung-controls.md`). On the
        // full hold-out (8 layouts, 6,000 frames, six clones) they read:
        // rest 0.000, blank -0.025, persist -0.016, follower 0.860 with
        // every ant the right way, and the follower blind -0.015. Here on
        // two layouts and half the frames, with room for that.
        let t = Trail { ants: 6, frames: 3000, clones: true };
        let lay = super::super::holdout_layouts(2);
        let score = |g: &[f32]| -> (Score, crate::sim::creature::mind::MindCounts) {
            let mut counts = None;
            let s: Vec<Score> = lay
                .iter()
                .map(|&l| {
                    let mut look = |_: u64, w: &World| counts = w.minds.as_ref().map(|m| m.counts);
                    t.run(l, &[g], &mut look).remove(0)
                })
                .collect();
            (super::super::worse_side(true, &s, &lay), counts.expect("the ants had minds"))
        };
        let preset = |name: &str| super::super::start_mind(&t, name);
        let (rest, rc) = score(&preset("rest"));
        let (blank, bc) = score(&preset("blank"));
        let (persist, _) = score(&preset("persist"));
        let follower_g = preset("follower");
        let (follower, fc) = score(&follower_g);
        let (blind, _) = score(&super::super::blinded(&follower_g));
        assert_eq!(rest.fit, 0.0, "an ant that never moves goes nowhere: {rest:?}");
        assert!(rc.decisions > 0 && rc.chose[crate::sim::creature::options::Verb::Walk as usize] == 0, "it decided, and never walked: {rc:?}");
        assert!(blank.fit.abs() < 0.15, "the random null stays near 0 on the signed score: {blank:?}");
        assert!(follower.fit > 0.5 && follower.parts[4] >= 0.8, "the hand-built follower goes the right way: {follower:?}");
        assert!(persist.fit < follower.fit - 0.3, "a heading-keeper must not score like a follower: {persist:?}");
        assert!(blind.fit < 0.15, "the follower blind to scent is no follower: {blind:?}");
        for c in [rc, bc, fc] {
            assert_eq!(c.failed, 0, "every pick came from the body's own checks, so none can fail: {c:?}");
        }
    }

    /// A blank mind with every zero trainable weight at 1e-4: it behaves as
    /// blank and every connection it can hold is live, the dearest brain
    /// the arena can be handed.
    fn fully_live_mind(ch: &dyn Challenge) -> Vec<f32> {
        let mut g = super::super::start_mind(ch, "blank");
        for i in crate::sim::creature::mind::trainable(ch.verbs(), ch.mind_emits()) {
            if g[i] == 0.0 {
                g[i] = 1.0e-4;
            }
        }
        g
    }

    #[test]
    fn a_fully_live_mind_lives_out_the_trail_arena() {
        // **Nobody starves, however big the brain** (review of PR 713, F1).
        // With the taxes billed against the raised 1e9 budget, this mind's
        // six ants were all dead by frame ~1,300, and a 700-weight champion
        // ended with 3-5 of six alive at 0.3-1.5% energy. The dearest mind
        // keeping all six to the bell covers every cheaper one; the bill
        // still lands, so the tax is priced and not switched off.
        let t = Trail { ants: 6, frames: 6000, clones: true };
        let g = fully_live_mind(&t);
        let live = crate::sim::creature::mind::MindGenome::new(g.clone()).live;
        assert!(live > 2000, "the mind is fully live: {live}");
        let (mut alive, mut low, mut taxed) = (0, f32::MAX, 0.0);
        let mut look = |_: u64, w: &World| {
            let sp = w.species.id_of("ant").expect("ant");
            let ids = ants_of(w, sp);
            alive = ids.len();
            low = ids.iter().filter_map(|&id| w.organism(id)).map(|s| s.energy).fold(f32::MAX, f32::min);
            taxed = w.energy_ledger.synapse_tax;
        };
        t.run(super::super::holdout_layouts(1)[0], &[&g], &mut look);
        assert_eq!(alive, 6, "every clone lives to the bell");
        assert!(low > 0.99e9, "and none spent more than a sliver of its budget: lowest {low}");
        assert!(taxed > 0.0, "the brain's bill is still booked");
    }

    #[test]
    fn a_minds_fall_ticks_pay_the_brain_tax() {
        // **Every tick bills the brain, falling or not, as the game's ant's
        // does** (review of PR 713, F2: billed only on decisions, a trail
        // champion fell on 36% of its ticks and thought for free there). An
        // ant on a ledge has the ledge taken away and falls a long way: the
        // brain's bill is exactly one tax per tick, falls and decisions
        // alike.
        let mut w = World::new(Rect::new(0, 0, 63, 159));
        for x in 0..64 {
            for y in (20..24).chain(150..160) {
                w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        w.plant_ant(32, 19);
        frame(&mut w, false);
        let sp = w.species.id_of("ant").expect("ant");
        let ids = ants_of(&w, sp);
        assert_eq!(ids.len(), 1, "the ant hatched");
        let t = Trail { ants: 1, frames: 1, clones: true };
        let genome = std::sync::Arc::new(crate::sim::creature::mind::MindGenome::new(fully_live_mind(&t)));
        let def = w.species.get(sp).creature.clone().expect("creature");
        let per_tick = (def.synapse_fraction * crate::sim::creature::link_price() * def.start_energy * genome.live as f32) as f64;
        crate::sim::creature::mind::give(&mut w, ids[0], genome);
        for x in 0..64 {
            for y in 20..24 {
                w.set(x, y, Cell::EMPTY);
            }
        }
        let before = w.energy_ledger.synapse_tax;
        for _ in 0..1200 {
            frame(&mut w, false);
        }
        let c = w.minds.as_ref().expect("minds").counts;
        assert!(c.falls >= 10 && c.decisions > 0, "it fell, and then it decided: {c:?}");
        let paid = w.energy_ledger.synapse_tax - before;
        let ticks = (c.falls + c.decisions) as f64;
        assert!((paid - ticks * per_tick).abs() <= 1e-9 * paid, "paid {paid} for {ticks} ticks at {per_tick}: {c:?}");
    }

    #[test]
    fn the_far_heap_lies_clear_of_home_and_the_wall() {
        let slumped = |width: i32, nest_x: i32, heap_x: i32| {
            let extra = format!(r#"Heap(material: "provisions", x: {heap_x}, cells: 300),"#);
            let (mut w, _, _) = bed_at(3, 0, &extra, 40, 33, width, nest_x).build();
            for _ in 0..2000 {
                frame(&mut w, false);
            }
            w
        };
        for dist in [FAR_DIST.0, FAR_DIST.1] {
            let (we, ne, he) = far_bed(dist, 1);
            let (ww, nw, hw) = far_bed(dist, -1);
            assert_eq!((we, ne, he), (ww, we - nw, we - hw), "dist {dist}: the two sides do not mirror");
            for (nest_x, heap_x) in [(ne, he), (nw, hw)] {
                let w = slumped(we, nest_x, heap_x);
                let food = w.materials.id_of("provisions").expect("provisions");
                let inside = cells_of(&w, 6, we - 6, food);
                assert_eq!(inside, 300, "dist {dist} heap {heap_x}: {inside} of 300 cells inside the box");
                assert_eq!(cells_of(&w, nest_x - 12, nest_x + 12, food), 0, "dist {dist} heap {heap_x}: heap reaches home");
                assert_eq!(cells_of(&w, heap_x - HEAP_REACH, heap_x + HEAP_REACH, food), 300, "dist {dist} heap {heap_x}: heap wider than HEAP_REACH");
            }
        }
        let (width, nest_x, heap_x) = (FAR_BEHIND + FAR_DIST.1 + 10, FAR_BEHIND, FAR_BEHIND + FAR_DIST.1);
        let w = slumped(width, nest_x, heap_x);
        let food = w.materials.id_of("provisions").expect("provisions");
        assert!(cells_of(&w, 6, width - 6, food) < 300, "a heap 10 columns from the wall should run into it; if not, this test cannot see it");
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

    /// **Walk genes reach every ant, the species and the world**, and a
    /// genome without them leaves all three as the bed made them. The
    /// brain-only arm is the control: without it a bed that already set the
    /// reserve would pass.
    #[test]
    fn walk_genes_reach_every_ant_and_the_world() {
        use crate::train::{SCORER, WALK};
        let lay = Layout { seed: 7, k: 0, blind: false, legal: false };
        let scout = organism::WALK_SLOTS.iter().position(|&s| s == organism::TRAIT_SCOUT).expect("scout is a walk gene");
        let (mut w, _, _) = bed(7, 12, "").build();
        let sp = w.species.id_of("ant").expect("ant");
        let before = creature::food_trail_of(&w);
        let mut g = w.species.get(sp).genome.clone();
        clone_colony(&mut w, &g, true, lay);
        assert_eq!(creature::food_trail_of(&w), before, "a brain-only genome set the reserve");
        assert!(ants_of(&w, sp).iter().all(|&id| w.organism(id).expect("live").traits[organism::TRAIT_SCOUT] == 0.0));
        g.extend_from_slice(&[0.0; WALK]);
        g[SCORER + scout] = -1.0;
        g[SCORER + WALK - 1] = 0.6;
        let (mut w, _, _) = bed(7, 12, "").build();
        clone_colony(&mut w, &g, true, lay);
        let ids = ants_of(&w, sp);
        assert_eq!(ids.len(), 12);
        for id in ids {
            let st = w.organism(id).expect("live");
            assert_eq!(st.traits[organism::TRAIT_SCOUT], -1.0, "ant {id} kept the shipped scout pull");
            assert_eq!(st.genome[..], g[..SCORER], "ant {id} has the wrong brain");
        }
        assert_eq!(w.species.get(sp).creature.as_ref().expect("creature").traits[organism::TRAIT_SCOUT], -1.0);
        assert_eq!(creature::food_trail_of(&w), creature::FoodTrail { reserve: 0.6, ..before });
    }

    /// An arena that scores `east` on even layouts and `west` on odd ones.
    struct Sided {
        east: f64,
        west: f64,
        se: bool,
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
            &[]
        }
        fn banned(&self) -> &'static [O] {
            &[]
        }
        fn frames(&self) -> u64 {
            1
        }
        fn mirrored(&self) -> bool {
            true
        }
        fn run(&self, l: Layout, _: &[&[f32]], _: &mut dyn FnMut(u64, &World)) -> Vec<Score> {
            vec![Score { fit: if l.k.is_multiple_of(2) { self.east } else { self.west }, parts: vec![] }]
        }
        fn floor_in_se(&self) -> bool {
            self.se
        }
    }

    /// **A suite folds each arena on its own before the product**: one
    /// arena done only east and another only west is worse side 1 on both,
    /// not the `sqrt(4 x 1) = 2` the first suite read on each side; a
    /// runaway ratio counts up to `RATIO_CAP`, except on the one arena a
    /// rung is learning; a kept arena under `forget` of its reference
    /// costs nine tenths; and one that asks for it (the colony with Dig,
    /// R12) is floored two standard errors under its reference instead.
    #[test]
    fn a_suite_folds_each_arena_before_the_product() {
        let lay = [Layout { seed: 1, k: 0, blind: false, legal: false }, Layout { seed: 1, k: 1, blind: false, legal: false }];
        let g = vec![0.0f32; 1];
        let score = |s: &Suite| crate::train::score_clones(s, &g, &lay, 1).fit;
        let split = Suite::new(vec![(Box::new(Sided { east: 4.0, west: 1.0, se: false }), false), (Box::new(Sided { east: 1.0, west: 4.0, se: false }), false)], 0.8);
        assert!((score(&split) - 1.0).abs() < 1e-9, "one-sided arenas scored {}", score(&split));
        let runaway = Suite::new(vec![(Box::new(Sided { east: 10.0, west: 10.0, se: false }), false), (Box::new(Sided { east: 1.0, west: 1.0, se: false }), false)], 0.8);
        assert!((score(&runaway) - RATIO_CAP.sqrt()).abs() < 1e-9, "a ratio of 10 was not capped: {}", score(&runaway));
        let rung = Suite::new(vec![(Box::new(Sided { east: 10.0, west: 10.0, se: false }), false), (Box::new(Sided { east: 1.0, west: 1.0, se: false }), true)], 0.8);
        assert!((score(&rung) - 10f64.sqrt()).abs() < 1e-9, "the one arena being learned is not capped: {}", score(&rung));
        let mut kept = Suite::new(vec![(Box::new(Sided { east: 1.0, west: 1.0, se: false }), true)], 0.8);
        assert!((score(&kept) - 1.0).abs() < 1e-9);
        kept.refs[0] = 1.5;
        assert!((score(&kept) - 0.1 / 1.5).abs() < 1e-9, "a kept arena at two thirds of its reference was not forgotten: {}", score(&kept));
        kept.refs[0] = 1.2;
        assert!((score(&kept) - 1.0 / 1.2).abs() < 1e-9, "at 83% of its reference it is kept, not forgotten: {}", score(&kept));
        // One folded unit: the floor is the reference less twice the spread.
        // At spread 0.6 it is 0.3, so 1.0 is kept where the share (1.2)
        // would forget it; at spread 0.2 it is 1.1 and 1.0 is forgotten.
        let mut noisy = Suite::new(vec![(Box::new(Sided { east: 1.0, west: 1.0, se: true }), true)], 0.8);
        noisy.refs[0] = 1.5;
        noisy.sds[0] = 0.6;
        assert!((score(&noisy) - 1.0 / 1.5).abs() < 1e-9, "within two standard errors, kept: {}", score(&noisy));
        noisy.sds[0] = 0.2;
        assert!((score(&noisy) - 0.1 / 1.5).abs() < 1e-9, "past two standard errors, forgotten: {}", score(&noisy));
    }

    /// An arena that scores a genome's weight `a` on layouts before
    /// `split` and weight `b` from it on, so a test can dip one brain under
    /// a floor on racing's first round only.
    struct Weights {
        a: usize,
        b: usize,
        split: usize,
    }
    impl Challenge for Weights {
        fn name(&self) -> &'static str {
            "weights"
        }
        fn unit(&self) -> Unit {
            Unit::Colony
        }
        fn ants(&self) -> usize {
            1
        }
        fn parts(&self) -> &'static [&'static str] {
            &[]
        }
        fn banned(&self) -> &'static [O] {
            &[]
        }
        fn frames(&self) -> u64 {
            1
        }
        fn run(&self, l: Layout, g: &[&[f32]], _: &mut dyn FnMut(u64, &World)) -> Vec<Score> {
            vec![Score { fit: f64::from(g[0][if (l.k as usize) < self.split { self.a } else { self.b }]), parts: vec![] }]
        }
    }

    /// **Racing's first round does not cut on a kept arena's floor**
    /// ([`Challenge::unfloored`]): the best digger keeps fetch at 0.5 of its
    /// reference on the 4 first-round worlds and 1.2 on the other 6 -- 0.92
    /// over all 10, above the 0.8 floor. Ranked by the floored score it was
    /// cut (0.1 of its product); unfloored it survives and ends unpenalised,
    /// while a digger under the floor on every world survives the cut and
    /// is still penalised on all ten.
    #[test]
    fn racing_does_not_cut_on_a_floor_read_over_a_few_worlds() {
        let mut s = Suite::new(vec![(Box::new(Weights { a: 0, b: 0, split: 4 }), false), (Box::new(Weights { a: 1, b: 2, split: 4 }), true)], 0.8);
        s.refs = vec![1.0, 1.0];
        let lay: Vec<Layout> = (0..10).map(|k| Layout { seed: 1, k, blind: false, legal: false }).collect();
        let mut genomes: Vec<Vec<f32>> = (1..=8).map(|d| vec![d as f32 * 0.5, 1.0, 1.0]).collect();
        genomes.push(vec![10.0, 0.5, 1.2]); // 8: dips on the first round only
        genomes.push(vec![10.0, 0.5, 0.5]); // 9: forgot fetch everywhere
        genomes.push(vec![1.0, 1.0, 1.0]); // the control
        let (means, survived) = crate::train::race(&s, &genomes, &lay, 4, 0.0, 2, &mut crate::train::Rng(1));
        assert!(survived[8] && survived[9], "the diggers were cut on the floor: {survived:?}");
        assert!((means[8].fit - (10.0f64 * 0.92).sqrt()).abs() < 1e-6, "the dip brain over all ten: {}", means[8].fit);
        assert!((means[9].fit - 0.1 * (10.0f64 * 0.5).sqrt()).abs() < 1e-9, "the forgetful one is still penalised: {}", means[9].fit);
    }

    /// **The step-4 switch** ([`Suite::gated_at`]): on from step 4 with soil
    /// learned, off before it, off without soil, and off with soil only
    /// kept.
    #[test]
    fn a_suite_is_gated_from_step_four_when_it_learns_soil() {
        let all = "soil,fetch,trail,scarce-dig";
        let kept = "fetch,trail,scarce-dig";
        assert!(Suite::gated_at(4, all, kept));
        assert!(Suite::gated_at(5, "soil,trail", "trail"));
        assert!(!Suite::gated_at(3, all, kept));
        assert!(!Suite::gated_at(4, "fetch,trail", "trail"));
        assert!(!Suite::gated_at(4, "soilish,trail", "trail"));
        assert!(!Suite::gated_at(4, all, "soil,fetch"));
    }

    /// **A gated suite scores its learned arena alone** ([`Suite::gated`],
    /// `dig-rung-design.md` rev 2.4): above every floor the fit is exactly
    /// the learned arena's ratio, and a luckier kept arena moves nothing;
    /// under a floor it is a tenth of that, and the kept arena's `_under`
    /// part reads 1. The control: ungated, the same kept ratio enters the
    /// product.
    #[test]
    fn a_gated_suite_scores_the_learned_arena_alone() {
        let lay = [Layout { seed: 1, k: 0, blind: false, legal: false }, Layout { seed: 1, k: 1, blind: false, legal: false }];
        let g = vec![0.0f32; 1];
        let score = |s: &Suite| crate::train::score_clones(s, &g, &lay, 1);
        let suite = |kept: f64, gated: bool| {
            let mut s = Suite::new(
                vec![(Box::new(Sided { east: 6.0, west: 6.0, se: false }), false), (Box::new(Sided { east: kept, west: kept, se: false }), true)],
                0.8,
            );
            s.refs = vec![2.0, 1.0];
            if gated {
                s.gated()
            } else {
                s
            }
        };
        assert_eq!(suite(1.0, false).parts().len(), 5, "two ratios, two fits, one gate");
        let ungated = score(&suite(2.0, false));
        assert!((ungated.fit - 6f64.sqrt()).abs() < 1e-9, "ungated, the kept ratio enters: {}", ungated.fit);
        for kept in [1.0, 2.0, 2.9] {
            let s = score(&suite(kept, true));
            assert!((s.fit - 3.0).abs() < 1e-9, "kept at {kept}: gated, the fit is the learned ratio alone, read {}", s.fit);
            assert_eq!(s.parts[4], 0.0);
        }
        let under = score(&suite(0.5, true));
        assert!((under.fit - 0.3).abs() < 1e-9, "under its floor: a tenth, read {}", under.fit);
        assert_eq!(under.parts[4], 1.0, "the gate's veto is not reported");
    }

    /// The soil trip's ground for the guards below: every cell from row 10
    /// down was ground at frame 0.
    fn below_10(_x: i32, y: i32) -> bool {
        y >= 10
    }

    /// **A pellet earns nothing at any stage unless its cut earned S1**
    /// (review of the dig design, R7): an ant that cuts back up only
    /// pellets put down earlier -- open air at frame 0 -- and puts them down
    /// again far from any hole scores 0 at every step. The positive
    /// control: the same trips cut out of the original ground score every
    /// stage.
    #[test]
    fn the_tailings_shuffler_scores_nothing() {
        let cuts: Vec<_> = (0..20).map(|i| (50 + i, 5, 10 * i as u64, 7)).collect();
        let dumps: Vec<_> = (0..20).map(|i| (100 + i, 5, 10 * i as u64 + 5, 7)).collect();
        for step in 1..=3 {
            let t = soil_trip(step, 1, below_10, |_, _| false, |_, _| 0, &[], &cuts, &dumps);
            assert_eq!((t.score, t.stages), (0.0, [0, 0, 0]), "step {step}: {t:?}");
        }
        let honest: Vec<_> = cuts.iter().map(|&(x, _, f, id)| (x, 12, f, id)).collect();
        let t = soil_trip(3, 1, below_10, |_, _| false, |_, _| 0, &[], &honest, &dumps);
        assert_eq!(t.stages, [20, 20, 20]);
        assert_eq!(t.score, 4.0 * 20.0 + 2.0 * SOIL_CAP as f64 + SOIL_CAP as f64);
        // A cell counts once: the same cell cut twice is one S1.
        let twice = [(60, 12, 0, 7), (60, 12, 20, 7)];
        let t = soil_trip(1, 1, below_10, |_, _| false, |_, _| 0, &[], &twice, &[(100, 5, 10, 7)]);
        assert_eq!((t.cuts, t.stages[0]), (2, 1));
    }

    /// **The farmer hits the cap** (dig design, section 4): an ant that cuts
    /// and puts each pellet down beside the face for the whole run is paid
    /// for [`SOIL_CAP`] pellets at each stage under the one being learned,
    /// so one pellet carried away is worth more than ten more at the face.
    #[test]
    fn the_cut_and_drop_farmer_hits_the_cap() {
        let cuts: Vec<_> = (0..40).map(|i| (60, 10 + i, 10 * i as u64, 7)).collect();
        let dumps: Vec<_> = (0..40).map(|i| (61, 10 + i, 10 * i as u64 + 5, 7)).collect();
        let at = |step| soil_trip(step, 1, below_10, |_, _| false, |_, _| 0, &[], &cuts, &dumps);
        assert_eq!(at(1).stages, [40, 40, 0]);
        assert_eq!(at(1).score, 40.0);
        assert_eq!(at(2).score, SOIL_CAP as f64 + 2.0 * 40.0);
        let farmer = at(3).score;
        assert_eq!(farmer, SOIL_CAP as f64 + 2.0 * SOIL_CAP as f64);
        // One more trip, put down twenty columns off.
        let mut more = cuts.clone();
        more.push((60, 50, 1000, 7));
        let mut away = dumps.clone();
        away.push((80, 5, 1005, 7));
        assert_eq!(soil_trip(3, 1, below_10, |_, _| false, |_, _| 0, &[], &more, &away).score, farmer + SOIL_POINTS[2]);
    }

    /// **Down the shaft is not away, and costs its refill** (review R9): the
    /// founding cut is part of the hole. Pellets cut at the shaft's foot and
    /// put down at its top earn nothing for S3, and each top cell ground
    /// again after the settle costs [`SOIL_REFILL`]. The positive control:
    /// with the founding cut left out of the hole the same put-downs read
    /// as away (4 rows above the nearest cut).
    #[test]
    fn the_shaft_dumper_is_not_away() {
        let founding: Vec<_> = (10..14).flat_map(|y| [(60, y), (61, y)]).collect();
        let cuts: Vec<_> = (0..8).map(|i| (60 + i % 2, 14 + i / 2, 10 * i as u64, 7)).collect();
        let dumps: Vec<_> = (0..8).map(|i| (60 + i % 2, 10, 10 * i as u64 + 5, 7)).collect();
        let top_filled = |x: i32, y: i32| (x == 60 || x == 61) && y == 10;
        let t = soil_trip(3, 1, below_10, top_filled, |_, _| 0, &founding, &cuts, &dumps);
        assert_eq!(t.stages, [8, 8, 0]);
        assert_eq!((t.refill_pellet, t.refill_fall), (2, 0));
        assert_eq!(t.score, 8.0 + 2.0 * 8.0 - 2.0 * SOIL_REFILL);
        assert_eq!(soil_trip(3, 1, below_10, top_filled, |_, _| 0, &[], &cuts, &dumps).stages, [8, 8, 8]);
        // Refill is step 3's: at step 2 putting the pellet down anywhere is the lesson.
        assert_eq!(soil_trip(2, 1, below_10, top_filled, |_, _| 0, &founding, &cuts, &dumps).score, 8.0 + 2.0 * 8.0);
    }

    /// **The nest filling, and what set each fall off** (step 4's prints,
    /// [`SoilTrip::chamber_fill`] and [`SoilTrip::fall_putdown`]): a
    /// founding-cut cell ground again counts in the fill and a cut cell
    /// outside it does not; a fall is a put-down's when a pellet left its
    /// site within 2 cells of it, and not when the ant cut that pellet back
    /// up.
    #[test]
    fn the_chamber_fill_and_each_falls_source_are_counted() {
        let founding: Vec<_> = (10..14).flat_map(|y| [(60, y), (61, y)]).collect();
        let cuts = [(60, 14, 0, 7)];
        let filled = |x: i32, y: i32| (x, y) == (60, 11) || (x, y) == (60, 14);
        let t = soil_trip(3, 1, below_10, filled, |_, _| 0, &founding, &cuts, &[]);
        assert_eq!((t.refill_fall, t.chamber_fill, t.fall_putdown), (2, 1, 0));
        // A pellet put down at the mouth and gone by the end rolled in.
        let rolled = [(61, 9, 5, 7)];
        let t = soil_trip(3, 1, below_10, filled, |_, _| 0, &founding, &cuts, &rolled);
        assert_eq!((t.refill_fall, t.chamber_fill, t.fall_putdown), (2, 1, 1));
        // ...unless the ant cut it back up.
        let recut = [(60, 14, 0, 7), (61, 9, 9, 7)];
        assert_eq!(soil_trip(3, 1, below_10, filled, |_, _| 0, &founding, &recut, &rolled).fall_putdown, 0);
    }

    /// **A pellet is paired with its own ant's next put-down**, and one cut
    /// over full jaws -- a pellet lost -- is held, not let go.
    #[test]
    fn soil_trips_pair_by_ant() {
        // Ant 7 cuts twice before putting down; ant 8 cuts and puts down
        // in between. Ant 7's first pellet was lost (held), its second put
        // down; ant 8's put-down is its own.
        let cuts = [(60, 12, 0, 7), (70, 12, 5, 8), (61, 12, 20, 7)];
        let dumps = [(90, 5, 10, 8), (100, 5, 30, 7)];
        let t = soil_trip(3, 1, below_10, |_, _| false, |_, _| 0, &[], &cuts, &dumps);
        assert_eq!((t.stages, t.held), ([3, 2, 2], 1));
    }

    /// **Up and out pays per row climbed, out of the ground only** (step 4's
    /// S4; design section 4, review R8). The ground's surface is row 10: a
    /// pellet cut 8 rows down and put down away, 9 rows up, earns S4 in
    /// full (its depth caps the climb); one cut 3 rows down and carried 9
    /// rows up onto a heap earns 3 rows; one cut at the surface earns none,
    /// and from step 4 nothing at any stage (rev 2.4); a deep one put down
    /// beside its own cut is not away and earns none. Counted at step 3,
    /// paid from step 4.
    #[test]
    fn up_and_out_pays_per_row_climbed_out_of_the_ground() {
        let depth = |_x: i32, y: i32| y - 10;
        let cuts = [(60, 18, 0, 7), (70, 13, 0, 8), (80, 10, 0, 9), (90, 18, 0, 10)];
        let dumps = [(120, 9, 5, 7), (130, 4, 5, 8), (140, 9, 5, 9), (91, 17, 5, 10)];
        let t = soil_trip(4, 1, below_10, |_, _| false, depth, &[], &cuts, &dumps);
        assert_eq!((t.stages, t.up, t.up_rows), ([3, 3, 2], 2, 8 + 3));
        // Three of the four cuts were below the surface: the share's base.
        assert_eq!((t.deep, t.rim), (3, 1));
        let below = 3.0 * SOIL_POINTS[0] + 3.0 * SOIL_POINTS[1] + 2.0 * SOIL_POINTS[2];
        assert_eq!(t.score, below + SOIL_POINTS[3] * 11.0 / f64::from(SOIL_UP_ROWS));
        // At step 3 the surface pellet still pays its three stages.
        let t3 = soil_trip(3, 1, below_10, |_, _| false, depth, &[], &cuts, &dumps);
        let below3 = 4.0 * SOIL_POINTS[0] + 4.0 * SOIL_POINTS[1] + 3.0 * SOIL_POINTS[2];
        assert_eq!((t3.stages, t3.up, t3.up_rows, t3.score), ([4, 4, 3], 2, 11, below3));
        // A full climb outweighs every stage under it together.
        assert!(SOIL_POINTS[3] > SOIL_POINTS[..3].iter().sum::<f64>());
        // Under step 4 the stages below are capped: 12 deep pellets, each
        // carried 8 rows up and away, pay S1-S3 for 10 and S4 for all 12.
        let cuts: Vec<_> = (0..12).map(|i| (60 + i, 18, 10 * i as u64, 7)).collect();
        let dumps: Vec<_> = (0..12).map(|i| (100 + 4 * i, 9, 10 * i as u64 + 5, 7)).collect();
        let t = soil_trip(4, 1, below_10, |_, _| false, depth, &[], &cuts, &dumps);
        assert_eq!((t.stages, t.up, t.deep), ([12, 12, 12], 12, 12));
        let capped = SOIL_CAP as f64 * SOIL_POINTS[..3].iter().sum::<f64>();
        assert_eq!(t.score, capped + 12.0 * SOIL_POINTS[3]);
    }

    /// **From step 4 the rim pays nothing** (`dig-rung-design.md` rev 2.4,
    /// the step-4 readout): 12 pellets cut from the top row of the ground
    /// and carried well away score 0 at step 4, every one counted as a rim
    /// cut. The control: the same trips pay S1-S3 at step 3, so the cap,
    /// not the trips, is what step 4 takes away; and the same trips cut a
    /// row lower pay at step 4. A cut in a raised step's face (depth below
    /// 0) is rim too.
    #[test]
    fn from_step_four_rim_pellets_score_nothing() {
        type Moves = Vec<(i32, i32, u64, OrganismId)>;
        let depth = |_x: i32, y: i32| y - 10;
        let trips = |row: i32| -> (Moves, Moves) {
            let cuts = (0..12).map(|i| (60 + i, row, 10 * i as u64, 7)).collect();
            let dumps = (0..12).map(|i| (100 + 4 * i, 9, 10 * i as u64 + 5, 7)).collect();
            (cuts, dumps)
        };
        let (cuts, dumps) = trips(10);
        let t = soil_trip(4, 1, below_10, |_, _| false, depth, &[], &cuts, &dumps);
        assert_eq!((t.score, t.stages, t.up, t.deep, t.rim), (0.0, [0, 0, 0], 0, 0, 12), "{t:?}");
        let t3 = soil_trip(3, 1, below_10, |_, _| false, depth, &[], &cuts, &dumps);
        assert_eq!((t3.stages, t3.rim), ([12, 12, 12], 12));
        assert_eq!(t3.score, 12.0 * SOIL_POINTS[2] + SOIL_CAP as f64 * (SOIL_POINTS[0] + SOIL_POINTS[1]));
        let (lower, dumps) = trips(11);
        let t = soil_trip(4, 1, below_10, |_, _| false, depth, &[], &lower, &dumps);
        assert_eq!((t.stages, t.deep, t.rim), ([12, 12, 12], 12, 0));
        let face = |_x: i32, y: i32| y - 12;
        let t = soil_trip(4, 1, below_10, |_, _| false, face, &[], &lower, &dumps);
        assert_eq!((t.score, t.rim), (0.0, 12), "a cut above the lowest surface near it is not a pit");
    }

    /// **From step 4, S1-S3 pay by depth squared, down to the shaft's foot**
    /// (`dig-rung-design.md` rev 2.8, its worked numbers). At a 4-row
    /// shaft, 12 pellets cut 2 rows down and carried well away pay a
    /// quarter of their S1-S3 for the 10 under the cap and S4 for 2 rows
    /// each, 41.5; the same trips 4 rows down pay 118, and a 2-row scrape
    /// 16.375. Eight from 2 rows pay 30 against four from 4 rows' 44, where
    /// linear pay tied them (review of PR 726, D1). The control: at
    /// `full_depth` 1 (rev 2.4's rule) the 2-row trips fill the cap, 94.
    /// And under the cap the deepest pellets are paid, whatever order they
    /// were cut in: 10 shallow pellets then 10 deep ones pay the deep ones'
    /// S1-S3 in full. Only pellets from the shaft's depth carried its
    /// height are full loads.
    #[test]
    fn from_step_four_s1_to_s3_pay_by_depth() {
        type Moves = Vec<(i32, i32, u64, OrganismId)>;
        let depth = |_x: i32, y: i32| y - 10;
        let trips = |rows: &[i32]| -> (Moves, Moves) {
            let cuts = rows.iter().enumerate().map(|(i, &row)| (60 + i as i32, row, 10 * i as u64, 7)).collect();
            let dumps = (0..rows.len()).map(|i| (100 + 4 * i as i32, 9, 10 * i as u64 + 5, 7)).collect();
            (cuts, dumps)
        };
        let below = SOIL_POINTS[..3].iter().sum::<f64>();
        let score = |full: i32, rows: &[i32]| {
            let (cuts, dumps) = trips(rows);
            soil_trip(4, full, below_10, |_, _| false, depth, &[], &cuts, &dumps)
        };
        let t = score(4, &[12; 12]);
        assert_eq!((t.stages, t.up, t.up_rows, t.deep), ([12, 12, 12], 12, 24, 12));
        assert_eq!(t.score, SOIL_CAP as f64 * 0.25 * below + 12.0 * SOIL_POINTS[3] * 2.0 / f64::from(SOIL_UP_ROWS));
        assert_eq!(t.score, 41.5);
        // Half a load each, carried up 2 rows at a 4-row shaft, and none of
        // them full.
        assert_eq!((t.loads, t.load_base, t.full_loads, t.full_cuts), (6.0, 6.0, 0, 0));
        let t = score(4, &[14; 12]);
        assert_eq!((t.score, t.loads, t.load_base, t.full_loads, t.full_cuts), (118.0, 12.0, 12.0, 12, 12));
        assert_eq!(score(1, &[12; 12]).score, 94.0);
        // Shallow no longer ties deep: under linear pay both were 44.
        let (shallow, deep) = (score(4, &[12; 8]), score(4, &[14; 4]));
        assert_eq!((shallow.score, shallow.full_loads), (30.0, 0));
        assert_eq!((deep.score, deep.full_loads), (44.0, 4));
        // The 2-row scrape (review N2): twelve rim cuts and twelve a row
        // under them pays 16.375 where it filled the cap (82), and comes to
        // 3 loads, none full.
        let t = score(4, &[[10; 12], [11; 12]].concat());
        assert_eq!((t.score, t.rim, t.loads, t.load_base, t.full_loads), (16.375, 12, 3.0, 3.0, 0));
        // A full load is counted from the climb, not the rows paid: at a
        // 12-row shaft, cut 13 rows down and carried 13 up, the S4 rows stop
        // at 8 and it is still full.
        let t = score(12, &[23; 2]);
        assert_eq!((t.up_rows, t.full_loads, t.full_cuts), (16, 2, 2));
        // Cut at the shaft's foot but put down after 3 of its 4 rows: a full
        // cut, not a full load (ladder lane, rev 2.8).
        let (cuts, _) = trips(&[14]);
        let t = soil_trip(4, 4, below_10, |_, _| false, depth, &[], &cuts, &[(100, 11, 5, 7)]);
        assert_eq!((t.up, t.up_rows, t.full_loads, t.full_cuts), (1, 3, 0, 1));
        // Ten shallow then ten deep: S4 pays every one (1 row and 4 rows),
        // S1-S3 the ten deep ones in full.
        let mixed: Vec<i32> = [[11; 10], [14; 10]].concat();
        let t = score(4, &mixed);
        assert_eq!(t.score, SOIL_CAP as f64 * below + SOIL_POINTS[3] * (10.0 + 40.0) / f64::from(SOIL_UP_ROWS));
        // Deep pellets put down beside their cut are not loads, though they
        // count in the base.
        let (cuts, _) = trips(&[14; 4]);
        let near: Moves = cuts.iter().map(|&(x, y, f, id)| (x, y - 1, f + 5, id)).collect();
        let t = soil_trip(4, 4, below_10, |_, _| false, depth, &[], &cuts, &near);
        assert_eq!((t.up, t.loads, t.load_base), (0, 0.0, 4.0));
        // Under step 4 depth does not scale: step 3 pays the 2-row trips in full.
        let (cuts, dumps) = trips(&[12; 12]);
        let t3 = soil_trip(3, 4, below_10, |_, _| false, depth, &[], &cuts, &dumps);
        assert_eq!(t3.score, 12.0 * SOIL_POINTS[2] + SOIL_CAP as f64 * (SOIL_POINTS[0] + SOIL_POINTS[1]));
    }

    /// **Step 4b pays only the shaft's foot** ([`Soil::foot`], ladder lane,
    /// `dig-rung-design.md` rev 2.9). At a 4-row shaft, 12 pellets cut 2
    /// rows down (the shaft's wall) and carried well away score 41.5 under
    /// rev 2.8 and nothing at step 4b, at any stage, though they are still
    /// read as deep cuts; the same trips from the foot pay 118 either way.
    /// Six from the wall and six from the foot: 4b pays the six foot
    /// pellets alone, 42 for S1-S3 and 24 for S4, where rev 2.8 also paid
    /// four wall pellets' S1-S3 under the cap and all six walls' S4, 85.
    /// The control: `s1_rows` 1 is [`soil_trip`] exactly, and under step 4
    /// the rows asked for change nothing.
    #[test]
    fn step_four_b_pays_only_the_shaft_foot() {
        type Moves = Vec<(i32, i32, u64, OrganismId)>;
        let depth = |_x: i32, y: i32| y - 10;
        let trips = |rows: &[i32]| -> (Moves, Moves) {
            let cuts = rows.iter().enumerate().map(|(i, &row)| (60 + i as i32, row, 10 * i as u64, 7)).collect();
            let dumps = (0..rows.len()).map(|i| (100 + 4 * i as i32, 9, 10 * i as u64 + 5, 7)).collect();
            (cuts, dumps)
        };
        let score = |step: u8, s1_rows: i32, rows: &[i32]| {
            let (cuts, dumps) = trips(rows);
            soil_trip_at(step, 4, s1_rows, below_10, |_, _| false, depth, &[], &cuts, &dumps)
        };
        let wall = score(4, 4, &[12; 12]);
        assert_eq!((wall.score, wall.stages, wall.up, wall.full_loads), (0.0, [0, 0, 0], 0, 0), "{wall:?}");
        assert_eq!((wall.deep, wall.rim, wall.cuts), (12, 0, 12), "a wall cut is still a deep cut in the readout");
        assert_eq!(score(4, 1, &[12; 12]).score, 41.5, "rev 2.8 pays the wall");
        assert_eq!(score(4, 4, &[14; 12]).score, 118.0);
        assert_eq!(score(4, 1, &[14; 12]).score, 118.0);
        let mixed: Vec<i32> = [[12; 6], [14; 6]].concat();
        let t = score(4, 4, &mixed);
        assert_eq!((t.score, t.stages, t.up, t.full_loads), (66.0, [6, 6, 6], 6, 6));
        assert_eq!(score(4, 1, &mixed).score, 85.0);
        for rows in [&[12; 12][..], &[14; 12], &mixed, &[10; 12]] {
            let (cuts, dumps) = trips(rows);
            assert_eq!(score(4, 1, rows), soil_trip(4, 4, below_10, |_, _| false, depth, &[], &cuts, &dumps));
            assert_eq!(score(3, 4, rows), score(3, 1, rows), "step 3 pays every first cut");
        }
    }

    /// **From step 4 the soil bed is the generator's, mirrored** (design
    /// section 3), and depth is read from the ground the step raised: on a
    /// layout whose [`DigBed`] drew a raised step, step 4's bed has it on
    /// `k`'s side and step 3's is flat there; the shaft is the one asked
    /// for on both sides; a cell under the flat ground beside the shaft is
    /// as deep as its rows, a cell in the step's face (above the ground 3
    /// columns off) is not below the ground, and a cell well inside the
    /// step is measured from the step's top.
    #[test]
    fn step_four_beds_are_the_generators_and_mirror() {
        let g = creature::mind::blank(SOIL_VERBS);
        let lay = |seed: u64, k: u64| Layout { seed, k, blind: false, legal: false };
        let seed = (0..64).find(|&s| DigBed::of(lay(s, 0)).step.is_some_and(|(x, _)| (x - NEST_X).abs() >= 12)).expect("a bed with a step");
        for k in [0, 1] {
            let world = |step: u8| {
                let soil = Soil { ants: 1, frames: 1, step, shaft: 8, budget: None, foot: false, step_near: 0.0, nest: false };
                assert_eq!(soil.mirrored(), step >= 4);
                soil.world(lay(seed, k), &g)
            };
            let bed = DigBed::of(lay(seed, k));
            let (at, up) = bed.step.expect("the step");
            let (w, sc, _, _) = world(4);
            let inside = at + bed.side * 6;
            assert_eq!(census::surface_of(&w, &sc.bed, inside), GROUND_Y - up, "k={k}: the raised step");
            let rows = founding_cut(&w, &sc.bed).iter().map(|&(x, y)| y - census::surface_of(&w, &sc.bed, x) + 1).max();
            assert_eq!(rows, Some(8 + 2), "k={k}: an 8-row shaft and its chamber");
            let (xs, _) = census::extents(&w, &sc.bed);
            let datum: Vec<Option<i32>> = xs.clone().map(|x| w.room_surface_at(x)).collect();
            let deep = |x, y| depth_below_ground(&datum, *xs.start(), x, y);
            assert_eq!(deep(NEST_X - bed.side * 5, GROUND_Y + 5), 5, "k={k}: beside the shaft");
            assert!(deep(at + bed.side, GROUND_Y - 1) <= 0, "k={k}: the step's face is no pit");
            assert_eq!(deep(inside, GROUND_Y - up + 2), 2, "k={k}: under the step's top");
            let (w3, sc3, _, _) = world(3);
            assert_eq!(census::surface_of(&w3, &sc3.bed, inside), GROUND_Y, "k={k}: step 3's bed is flat");
        }
    }

    /// **An arena's budget does not move what a full tank feels like**
    /// ([`budget_to`], `CreatureDef::sensed_tank`): a soil ant on 2,400 J
    /// and a fed one on 1e9 feel their bank against the species' authored
    /// tank, as the colony and the game do. Put the fault back (the sense
    /// over `start_energy`) and both read exactly 0.5.
    #[test]
    fn an_arena_budget_leaves_the_energy_sense_on_the_authored_tank() {
        let lay = Layout { seed: 3, k: 0, blind: false, legal: false };
        for budget in [Some(2400.0), None] {
            let soil = Soil { ants: 1, frames: 1, step: 4, shaft: 8, budget, foot: true, step_near: 0.0, nest: false };
            let (w, _, sp, full) = soil.world(lay, &creature::mind::blank(SOIL_VERBS));
            let def = w.species.get(sp).creature.clone().expect("creature");
            let tank = def.sensed_tank.expect("budget_to keeps the authored tank");
            assert!(tank > 0.0 && tank < full, "{budget:?}: authored {tank} under the budget {full}");
            let id = ants_of(&w, sp)[0];
            let (ctx, _) = creature::mind::context(&w, id, &def, &creature::mind::MindState::default(), &[]);
            let e = full / tank;
            let want = e / (1.0 + e);
            assert!((ctx[creature::mind::c::ENERGY] - want).abs() < 1e-6, "{budget:?}: reads {} for a bank of {full} on a tank of {tank}", ctx[creature::mind::c::ENERGY]);
            assert!(ctx[creature::mind::c::ENERGY] > 0.9, "{budget:?}: a full ant feels full, not 0.5");
        }
    }

    /// **The soil arena's world never rolls a pellet** ([`hold_training_world`]):
    /// its world reads `spoil_roll` 0 whatever the process says; the control,
    /// the same bed built plainly, leaves it to the process (`None`).
    #[test]
    fn training_worlds_hold_pellets_where_they_are_put() {
        let soil = Soil { ants: 1, frames: 1, step: 4, shaft: 8, budget: None, foot: true, step_near: 0.0, nest: false };
        let lay = Layout { seed: 3, k: 0, blind: false, legal: false };
        let (w, _, _, _) = soil.world(lay, &creature::mind::blank(SOIL_VERBS));
        assert_eq!(w.spoil_roll, Some(0));
        assert_eq!(creature::spoil_roll_of(&w), 0);
        assert_eq!(soil.bed(lay).bed.build_counted().0.spoil_roll, None, "the control: a plain build leaves it to the process");
    }

    /// **`stepnear=` forces the step near the shaft on its share of pairs,
    /// and nothing else** ([`DigBed::drawn`]): at 0 every bed is
    /// [`DigBed::of`]'s; at 1 every bed's step is 6-10 columns out on its
    /// own side, mirrored across a pair, and nothing but the step moved; at
    /// 0.3 about that share of 400 pairs is forced. The beds' own share
    /// within 10 columns is the control: about one bed in five.
    #[test]
    fn step_near_forces_the_step_by_the_shaft_and_nothing_else() {
        let lay = |seed: u64, k: u64| Layout { seed, k, blind: false, legal: false };
        let near = |b: &DigBed| b.step.is_some_and(|(x, _)| (6..=10).contains(&((x - NEST_X) * b.side)));
        let (mut own, mut forced) = (0, 0);
        for seed in 0..400 {
            for k in [0, 1] {
                let l = lay(seed, k);
                let bed = DigBed::of(l);
                assert_eq!(DigBed::drawn(l, 0.0), bed, "seed {seed}: 0 is the bed as it was");
                let all = DigBed::drawn(l, 1.0);
                assert!(near(&all), "seed {seed} k {k}: {:?}", all.step);
                assert_eq!(DigBed { step: bed.step, ..all.clone() }, bed, "seed {seed}: only the step moves");
                if k == 0 {
                    own += usize::from(near(&bed));
                    forced += usize::from(DigBed::drawn(l, 0.3) != bed && near(&DigBed::drawn(l, 0.3)));
                    let west = DigBed::drawn(lay(seed, 1), 1.0);
                    assert_eq!(all.step.map(|s| s.0 - NEST_X), west.step.map(|s| NEST_X - s.0), "seed {seed}: mirrored");
                }
            }
        }
        assert!((60..=110).contains(&own), "the beds' own near steps: {own} of 400");
        assert!((90..=150).contains(&forced), "0.3 forced {forced} of 400");
    }

    /// **No stone in the founding cut** (ladder lane, `dig-rung-design.md`
    /// step 4's guard): on 64 generated beds, both sides, step 4's deepest
    /// shaft is cut to its full 12 rows below the ground at the shaft and
    /// its 2-row chamber is as wide as on the flat bed, so a short shaft
    /// never reads as the ant's failure to climb. Measured at the shaft's
    /// own column: the chamber can run under a raised step's foot (seed 20
    /// has its step 6 columns out), where the ground is 6 rows higher and
    /// the same cut reads 20 rows deep. The positive control: a 20-row
    /// shaft meets a stone lens on 6 of the same beds (seeds 4 and 43 both
    /// sides among them) and its chamber narrows there, so the count can
    /// see one. Measured margin: 14 rows clean on all 128, the first bed
    /// bites at 16 (seed 23), and from 24 rows a bed can found no ant.
    #[test]
    fn a_twelve_row_shaft_is_cut_in_full_on_every_generated_bed() {
        let g = creature::mind::blank(SOIL_VERBS);
        let lay = |seed: u64, k: u64| Layout { seed, k, blind: false, legal: false };
        // (deepest row below the ground at the shaft, cells in the chamber's two rows)
        let cut = |soil: &Soil, l: Layout| -> (i32, usize) {
            let (w, sc, _, _) = soil.world(l, &g);
            let cells = founding_cut(&w, &sc.bed);
            let bottom = cells.iter().map(|c| c.1).max().unwrap_or(0);
            let top = w.room_surface_at(NEST_X).expect("a datum at the shaft");
            (bottom - top + 1, cells.iter().filter(|c| c.1 >= bottom - 1).count())
        };
        let short = |shaft: i32| -> Vec<(u64, u64, (i32, usize))> {
            let soil = Soil { ants: 1, frames: 1, step: 4, shaft, budget: None, foot: false, step_near: 0.0, nest: false };
            let flat = cut(&Soil { step: 3, ..soil }, lay(0, 0));
            assert_eq!(flat.0, shaft + 2, "the flat bed's shaft");
            let mut out = vec![];
            for seed in 0..64 {
                for k in [0, 1] {
                    let got = cut(&soil, lay(seed, k));
                    if got != flat {
                        out.push((seed, k, got));
                    }
                }
            }
            out
        };
        assert_eq!(short(12), vec![], "a 12-row shaft cut short, or its chamber narrowed");
        assert!(!short(20).is_empty(), "the control: a 20-row shaft must meet stone somewhere");
    }

    /// **The soil arena cuts the shaft it asks for** (review R4): the
    /// founding shaft is cut inside `apply`, so `Soil::world` sets its
    /// depth between the build and the colony. The positive control: the
    /// same depth set after `apply` is never read, and the world cuts its
    /// default shaft instead.
    #[test]
    fn the_soil_arena_cuts_the_shaft_it_asks_for() {
        let lay = Layout { seed: 3, k: 0, blind: false, legal: false };
        let g = creature::mind::blank(SOIL_VERBS);
        let deepest = |w: &World, sc: &Scenario| founding_cut(w, &sc.bed).iter().map(|&(x, y)| y - census::surface_of(w, &sc.bed, x) + 1).max();
        let soil = |shaft| Soil { ants: 1, frames: 1, step: 1, shaft, budget: None, foot: false, step_near: 0.0, nest: false };
        let (w, sc, _, _) = soil(0).world(lay, &g);
        assert_eq!(deepest(&w, &sc), None, "no shaft asked for, and a hole at frame 0");
        let (w, sc, _, _) = soil(2).world(lay, &g);
        let two = deepest(&w, &sc).expect("a two-row shaft");
        // Two shaft rows over the founding's two-row chamber.
        assert_eq!(two, 4, "a 2-row shaft and its chamber");
        // The wrong order: build, colony, then the depth.
        let sc = soil(2).bed(lay);
        let (mut late, _) = sc.bed.build_counted();
        let mut spec = sc.bed.clone();
        sc.apply(&mut late, &mut spec);
        late.nest_shaft = Some(2);
        let default = deepest(&late, &sc).expect("the default shaft");
        assert!(two < default, "a 2-row shaft read {two} rows deep, the default {default}");
    }

    /// **The soil picture's colours are the score's** ([`Soil::marks`]): a
    /// pellet put down inside the founding chamber is near the hole (amber),
    /// never away (green), and one put down far along the bare surface is
    /// away (review of PR 721, N2: painting without the founding cut left
    /// every test green).
    #[test]
    fn the_soil_picture_paints_near_and_away_as_the_score_does() {
        let lay = Layout { seed: 3, k: 0, blind: false, legal: false };
        let soil = Soil { ants: 1, frames: 1, step: 3, shaft: 2, budget: None, foot: false, step_near: 0.0, nest: false };
        let (mut w, sc, _, _) = soil.world(lay, &creature::mind::blank(SOIL_VERBS));
        let chamber = *founding_cut(&w, &sc.bed).last().expect("a founding cut");
        let x = chamber.0 + 20;
        let far = (x, census::surface_of(&w, &sc.bed, x) - 1);
        let id = w.live_organism_ids().into_iter().next().expect("an ant");
        w.minds.as_mut().expect("minds").dump_log = Some(vec![(chamber.0, chamber.1, 1, id), (far.0, far.1, 2, id)]);
        let marks = soil.marks(lay, &w);
        let colour = |at: (i32, i32)| marks.iter().find(|(p, _)| *p == at).map(|(_, c)| *c);
        assert_eq!(colour(chamber), Some([255, 190, 0]), "a pellet in the founding chamber is near the hole");
        assert_eq!(colour(far), Some([40, 220, 60]), "a pellet 20 columns along the surface is away");
    }

    /// **The soil ledger closes** (dig design, section 9's guards): every
    /// cut the world counts is in the cut log and the mind's own count,
    /// every pellet put down in the put-down log and the count, on a blank
    /// mind, which digs and drops at random.
    #[test]
    fn the_soil_ledger_matches_the_world_counts() {
        let lay = Layout { seed: 3, k: 0, blind: false, legal: false };
        let soil = Soil { ants: 1, frames: 3000, step: 1, shaft: 2, budget: Some(SOIL_BUDGET), foot: false, step_near: 0.0, nest: false };
        let (mut w, _, sp, _) = soil.world(lay, &creature::mind::blank(SOIL_VERBS));
        let (digs, dumped, dig_j) = (w.creature_stats.digs, w.creature_stats.spoil_dumped, w.creature_stats.dig_energy);
        for _ in 0..soil.frames {
            frame(&mut w, true);
        }
        let m = w.minds.as_ref().expect("minds");
        let (cuts, puts) = (w.cut_log.as_ref().expect("cut log").len() as u64, m.dump_log.as_ref().expect("dump log").len() as u64);
        assert!(cuts > 5 && puts > 5, "a blank mind cut {cuts} and put down {puts}: too few to test the ledger");
        assert_eq!((w.creature_stats.digs - digs, m.counts.dug), (cuts, cuts));
        assert_eq!((w.creature_stats.spoil_dumped - dumped, m.counts.dumped), (puts, puts));
        // **Every cut is billed, as the game bills one** (review of PR 720,
        // N1: switching the mind's cut bill off left every test green).
        let def = w.species.get(sp).creature.clone().expect("creature");
        let ant = ants_of(&w, sp)[0];
        let body = w.organism(ant).expect("alive").chain.len() as f32;
        let per_cut = (def.move_cost_per_cell * body * def.dig_cost_in_moves) as f64;
        let billed = w.creature_stats.dig_energy - dig_j;
        assert!(per_cut > 0.0 && (billed - cuts as f64 * per_cut).abs() < 1e-6 * billed.max(1.0), "{cuts} cuts billed {billed} J, {per_cut} J each");
        // **The budget keeps every standing bill at its authored size**: a
        // fraction times the budget is what it is at the species' own.
        let (bare, _) = soil.bed(lay).bed.build_counted();
        let authored = bare.species.get(bare.species.id_of("ant").expect("ant")).creature.clone().expect("creature");
        let bills = |d: &organism::CreatureDef| {
            [d.synapse_fraction, d.sight_fraction, d.curvature_fraction, d.force_fraction, d.armour_fraction].map(|f| f * d.start_energy)
        };
        assert_ne!(def.start_energy, authored.start_energy, "the budget is not the authored energy, so this tests nothing");
        for (k, (got, want)) in bills(&def).iter().zip(bills(&authored)).enumerate() {
            assert!((got - want).abs() <= 1e-5 * want.abs(), "bill {k}: {got} J at the budget, {want} J authored");
        }
    }

    /// **Exactly [`SOIL_AWAY`] cells off is away, one fewer is not**
    /// (review of PR 720, N2: no test sat on the boundary).
    #[test]
    fn soil_away_starts_three_cells_from_the_hole() {
        let ground = |_: i32, y: i32| y >= 5;
        // Ant 1 cuts (0,5) and puts it 3 off; ant 2 cuts (0,6) and puts it 2 off.
        let cuts = [(0, 5, 1, 1), (0, 6, 3, 2)];
        let dumps = [(3, 2, 2, 1), (2, 3, 4, 2)];
        let t = soil_trip(3, 1, ground, |_, _| false, |_, _| 0, &[], &cuts, &dumps);
        assert_eq!(t.stages, [2, 2, 1]);
    }

    /// **The soil box never offers the food verbs** (`Soil::acting_verbs`,
    /// the protection in `Settings::protect`): a mind with every verb on,
    /// one ant on a step-4 bed for 3,000 frames. The counter is checked
    /// against verbs that do act (walking, cutting, putting down), so a
    /// blind counter fails here too; if food or larvae ever reach the soil
    /// box this fails, and the protection must be revisited, not silenced.
    #[test]
    fn soil_never_offers_food_verbs() {
        use creature::options::Verb as V;
        let soil = Soil { ants: 1, frames: 3000, step: 4, shaft: 4, budget: Some(1200.0), foot: true, step_near: 0.0, nest: false };
        let g = super::super::start_mind(&soil, "training/starts/s134-4b-g34.txt");
        for k in 0..2 {
            let (mut w, _, _, _) = soil.world(Layout { seed: 70_000, k, blind: false, legal: false }, &g);
            for _ in 0..soil.frames {
                frame(&mut w, true);
            }
            let c = w.minds.as_ref().expect("minds").counts;
            for v in [V::Walk, V::Dig, V::Drop] {
                assert!(c.offered[v as usize] > 0, "k={k}: {v:?} never offered -- the counter is blind");
            }
            for v in [V::PickUp, V::Eat, V::Feed] {
                assert_eq!(c.offered[v as usize], 0, "k={k}: {v:?} had a target in the soil box");
            }
            let acting = soil.acting_verbs();
            assert!(SOIL_VERBS.iter().all(|v| acting.contains(v) == (c.offered[*v as usize] > 0)), "k={k}: acting_verbs {acting:?} disagrees with what acted");
        }
    }

    /// **A digging suite cannot test the food verbs, a foraging one can**
    /// (`Suite::untested_verbs`): soil learned with fetch and trail kept
    /// leaves Pick up, Eat and Feed untested; with fetch learned, nothing.
    #[test]
    fn a_digging_suite_leaves_the_food_verbs_untested() {
        use creature::options::Verb as V;
        let soil = || Soil { ants: 1, frames: 10, step: 4, shaft: 4, budget: Some(1200.0), foot: true, step_near: 0.0, nest: false };
        let fetch = || Scarce {
            ants: 1,
            frames: 10,
            drip: 5,
            every: 1000,
            game_walk: false,
            brake: Some(creature::FoodBrake::FORAGE),
            dist: (10, 30),
            steps: true,
            fed: true,
            dig: false,
        };
        let digging = Suite::new(vec![(Box::new(soil()), false), (Box::new(fetch()), true), (Box::new(Trail { ants: 2, frames: 10, clones: true }), true)], 0.8);
        let mut u = digging.untested_verbs();
        u.sort_by_key(|v| *v as usize);
        assert_eq!(u, vec![V::PickUp, V::Eat, V::Feed]);
        let foraging = Suite::new(vec![(Box::new(fetch()), false), (Box::new(soil()), true)], 0.8);
        assert!(foraging.untested_verbs().iter().all(|v| *v == V::Dig), "{:?}", foraging.untested_verbs());
    }
}
