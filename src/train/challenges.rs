//! The rig's first three arenas. Each one says what it pays for, what the
//! score could be faked with, and what stops that.

use super::{Challenge, Layout, Score, Unit};
use crate::lab::census::{self, Ids};
use crate::lab::scenario::Scenario;
use crate::sim::brain::BrainOutput as O;
use crate::sim::chunk::Rect;
use crate::sim::cell::OrganismId;
use crate::sim::organism::{self, SpeciesId};
use crate::sim::pheromone::{self, Channel};
use crate::sim::{creature, material, update, Cell, World};

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
/// ([`forage_steps`], `steps=off` takes it away): reaching the food,
/// picking some up, carrying it towards the nest, arriving with it, putting
/// it down there (owner, 2026-10-18). Trained on heads alone, at 80 columns
/// for 16 generations of 100 brains, no champion ever carried a cell home:
/// its ants picked up 11-35 cells a run and ate them, and turning round
/// with a load was worth nothing until it reached the nest, which a child
/// that merely could turn never did (`brain-training/stage2/forage-big`).
/// Heads stay in the score and are the rung's check (design rev3, *a second
/// number it cannot trade away*), so a colony that buys steps with its ants
/// shows it there, but the steps can lift a dead colony over a live one
/// that never left home.
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
    /// **How far out the food lands, in columns**: drawn per layout from
    /// `lo..=hi` ([`SCARCE_DIST`] unless `dist=` sets it). Nearer food is
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
}

/// The food spot's distance from the nest, in columns: the ladder's far
/// bed. At 40 columns and 12,000 frames a colony that over-breeds ties the
/// game ant on the worse side (19.2 against 19.3); at 80 the game ant beats
/// the cheap-brain champion on 4 of 4 pairs (17.7 against 8.3), so the rung
/// trains there (ladder review of the stage 2 notes, finding 5). Inside the
/// 256-wide bed either side.
pub const SCARCE_DIST: (i32, i32) = (80, 80);

impl Scarce {
    /// The nest's column, the food spot's and its distance, for one layout.
    fn geometry(&self, layout: Layout) -> (i32, i32, i32) {
        let side = if layout.k.is_multiple_of(2) { 1 } else { -1 };
        let (lo, hi) = self.dist;
        let dist = lo + ((layout.seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 32) % (hi - lo + 1) as u64) as i32;
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
/// food verbs.
const SCARCE_VERBS: &[creature::options::Verb] = {
    use creature::options::Verb as V;
    &[V::Walk, V::TurnAround, V::Rest, V::Eat, V::Drop, V::Feed]
};

impl Challenge for Scarce {
    fn name(&self) -> &'static str {
        if self.fed {
            "fetch"
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
        &["score", "adults", "kept", "starved", "births", "brood", "eaten", "delivered", "larvae_j", "held", "dist", "lived", "heads", "steps", "reach", "picked", "carried", "arrived", "home"]
    }
    fn banned(&self) -> &'static [O] {
        NOT_ANT_VERBS
    }
    fn minds(&self) -> bool {
        true
    }
    fn verbs(&self) -> &'static [creature::options::Verb] {
        SCARCE_VERBS
    }
    fn mirrored(&self) -> bool {
        true
    }
    fn run(&self, layout: Layout, genomes: &[&[f32]], look: &mut dyn FnMut(u64, &World)) -> Vec<Score> {
        let (nest_x, food_x, dist) = self.geometry(layout);
        let sc = self.bed(layout);
        let (mut w, _, _) = sc.build();
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
        let mut lived = (0.0, 0u32);
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
                    lived.0 += now.len() as f64 * kept;
                    lived.1 += 1;
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
        let lived = lived.0 / lived.1.max(1) as f64 / founders.len().max(1) as f64;
        let heads = scarce_floor(scores.iter().sum::<f64>() / scores.len() as f64, lived, founders.len());
        let n = founders.len().max(1) as f64;
        let (reach, picked, arrived) = (
            trip.reached.len() as f64 / n,
            trip.picked.len() as f64 / n,
            trip.arrived.len() as f64 / n,
        );
        let home = st.trip_deliveries as f64;
        let steps = forage_steps(reach, picked, trip.carried, arrived, home);
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
            ],
        }]
    }
}

/// **The way home, step by step** (owner, 2026-10-18): reaching the food,
/// picking some up, carrying it towards the nest, arriving with it, putting
/// it down there. Each step outweighs every step below it together
/// (1 + 2 + 4 + 8 = 15 < 16), so stacking the easy ones never beats one
/// cell home.
pub const STEP_WEIGHTS: [f64; 5] = [1.0, 2.0, 4.0, 8.0, 16.0];

/// What one step point adds to a scarce colony's score, in heads: the whole
/// way short of the nest (15 points) is worth seven and a half ants kept, a
/// first cell home eight.
pub const STEP_SCALE: f64 = 0.5;

/// How often the trip is read, in frames: an ant walks well under a cell a
/// frame, so it cannot cross the food spot's reach between two reads.
const STEP_EVERY: u64 = 4;

/// **How far the colony got on the way home, in step points**
/// ([`STEP_WEIGHTS`]). `reach`, `picked` and `arrived` are the share of
/// the founders that ever reached the food, picked food up away from home
/// and stood at the nest still carrying it; `carried` is the furthest any
/// load got towards the nest, as a share of the way from the food spot;
/// `home` the cells from a trip put down at the nest, each doubling worth a
/// quarter of the first. Shares past 1 (a colony that bred) count as 1.
pub fn forage_steps(reach: f64, picked: f64, carried: f64, arrived: f64, home: f64) -> f64 {
    let w = STEP_WEIGHTS;
    let delivered = if home >= 1.0 { 1.0 + home.log2() / 4.0 } else { 0.0 };
    w[0] * reach.clamp(0.0, 1.0)
        + w[1] * picked.clamp(0.0, 1.0)
        + w[2] * carried.clamp(0.0, 1.0)
        + w[3] * arrived.clamp(0.0, 1.0)
        + w[4] * delivered
}

/// **What the scarce colony did on the way home**, read every
/// [`STEP_EVERY`] frames. A load is food in the crop from away from home
/// (`trip_cells`, which the game's pickup and a mind's swallow both mark),
/// so food taken at the nest carries nothing.
#[derive(Default)]
struct Trip {
    reached: Vec<OrganismId>,
    picked: Vec<OrganismId>,
    arrived: Vec<OrganismId>,
    carried: f64,
}

impl Trip {
    /// Within this many columns of the food spot is at the food.
    const REACH: i32 = 4;

    fn look(&mut self, w: &World, adults: &[OrganismId], nest_x: i32, food_x: i32, dist: i32) {
        let mark = |v: &mut Vec<OrganismId>, id: OrganismId| {
            if let Err(i) = v.binary_search(&id) {
                v.insert(i, id);
            }
        };
        for &id in adults {
            let Some(st) = w.organism(id) else { continue };
            let Some(&(hx, _)) = st.chain.first() else { continue };
            if (hx - food_x).abs() <= Self::REACH {
                mark(&mut self.reached, id);
            }
            if st.trip_cells == 0 || st.crop.is_none_or(|c| c.cells == 0) {
                continue;
            }
            mark(&mut self.picked, id);
            let way = 1.0 - (hx - nest_x).abs() as f64 / dist.max(1) as f64;
            self.carried = self.carried.max(way.clamp(0.0, 1.0));
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
    let authored = def.start_energy;
    def.start_energy = 1.0e9;
    let k = authored / def.start_energy;
    def.synapse_fraction *= k;
    def.sight_fraction *= k;
    def.curvature_fraction *= k;
    def.force_fraction *= k;
    def.armour_fraction *= k;
}

/// **A colony gone before the last quarter still ranks by how long it
/// lasted**: `score` (the last quarter's reads) when it is above 0,
/// otherwise `lived` (the same adults-times-founders-kept read every 64th
/// of the run, over the founders) scaled under the least score a live
/// colony can read, a single founder at one of the four reads, so a dead
/// colony never outranks a live one and no live score moves.
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
pub struct Suite {
    pub arenas: Vec<(Box<dyn Challenge>, bool)>,
    pub refs: Vec<f64>,
    /// Each arena's spread at calibration: the standard deviation of its
    /// folded units (a mirrored pair's worse side, or one layout).
    pub sds: Vec<f64>,
    /// The share of its reference under which a `keep` arena counts as
    /// forgotten.
    pub forget: f64,
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
            names: Box::leak(names.into_boxed_slice()),
            banned: Box::leak(banned.into_boxed_slice()),
            verbs: Box::leak(verbs.into_boxed_slice()),
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
        for (i, (a, _)) in self.arenas.iter().enumerate() {
            let gs: Vec<&[f32]> = vec![g; if a.unit() == Unit::Ant { a.ants() } else { 1 }];
            let mut quiet = |_: u64, _: &World| {};
            let watcher: &mut dyn FnMut(u64, &World) = if i == 0 { &mut *look } else { &mut quiet };
            let s = super::mean_score(&a.run(layout, &gs, watcher));
            ratios.push(self.ratio(i, s.fit));
            fits.push(s.fit);
        }
        let fit = (ratios.iter().map(|r| r.max(1e-3).ln()).sum::<f64>() / ratios.len() as f64).exp();
        ratios.extend(fits);
        vec![Score { fit, parts: ratios }]
    }

    /// Each arena folded on its own ([`super::worse_side`] on its raw fits),
    /// then the geometric mean of the capped ratios, with the forget floor
    /// read on each `keep` arena's folded score: under `forget` of its
    /// reference.
    /// `parts` are each arena's folded ratio, then its folded fit.
    fn combine(&self, v: &[Score], layouts: &[Layout]) -> Score {
        let n = self.arenas.len();
        let mut ratios = vec![];
        let mut fits = vec![];
        let mut forgot = false;
        for (i, (a, keep)) in self.arenas.iter().enumerate() {
            let own: Vec<Score> = v.iter().map(|s| Score { fit: s.parts[n + i], parts: vec![] }).collect();
            let f = super::worse_side(a.mirrored(), &own, layouts).fit;
            if *keep && f < self.forget * self.refs[i] {
                forgot = true;
            }
            ratios.push(self.ratio(i, f));
            fits.push(f);
        }
        let mut fit = (ratios.iter().map(|r| r.max(1e-3).ln()).sum::<f64>() / n as f64).exp();
        if forgot {
            fit *= 0.1;
        }
        ratios.extend(fits);
        Score { fit, parts: ratios }
    }

    /// Set every arena's reference to `g`'s folded score there (floored at
    /// a small positive number, so an arena the start cannot do at all pays
    /// for any progress rather than dividing by zero), and its spread over
    /// the folded units, printed beside it.
    fn calibrate(&mut self, g: &[f32], layouts: &[Layout], threads: usize) {
        for (i, (a, _)) in self.arenas.iter().enumerate() {
            let each = super::score_each(a.as_ref(), g, layouts, threads);
            let units: Vec<f64> = super::sides(a.mirrored(), layouts)
                .into_iter()
                .map(|(x, y)| y.map_or(each[x].fit, |y| each[x].fit.min(each[y].fit)))
                .collect();
            let m = units.len().max(1) as f64;
            let mean = units.iter().sum::<f64>() / m;
            self.refs[i] = mean.max(REF_FLOOR);
            self.sds[i] = (units.iter().map(|u| (u - mean).powi(2)).sum::<f64>() / (m - 1.0).max(1.0)).sqrt();
            println!("suite reference {}: {:.3}, spread {:.3} over {} units", a.name(), self.refs[i], self.sds[i], units.len());
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
        for v in [Verb::Eat, Verb::Drop, Verb::Feed] {
            assert!(cut.w[v as usize] < 0.0 && whole.w[v as usize] > 0.0, "{} on the trail arena", v.name());
        }
    }

    /// **The scarce arena's food arrives on the timeline**: five cells of
    /// provisions at the spot on the first frame and five more by frame
    /// 1,001, ticked by `frame_timed` -- and none at all under plain
    /// `frame`, the positive control that the cells come from the drip and
    /// not the bed (review of PR 712, N5: no test ran a bed with a
    /// timeline).
    #[test]
    fn the_scarce_drip_lands_on_the_timeline() {
        let ch = Scarce { ants: 2, frames: 0, drip: 5, every: 1000, game_walk: true, brake: None, dist: SCARCE_DIST, steps: true, fed: false };
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
        assert_eq!(forage_steps(0.0, 0.0, 0.0, 0.0, 0.0), 0.0);
        let below = |k: usize| {
            forage_steps(
                f64::from(u8::from(k > 0)),
                f64::from(u8::from(k > 1)),
                f64::from(u8::from(k > 2)),
                f64::from(u8::from(k > 3)),
                0.0,
            )
        };
        let one = |k: usize| {
            let mut s = [0.0; 5];
            s[k] = 1.0;
            forage_steps(s[0], s[1], s[2], s[3], s[4])
        };
        for k in 1..5 {
            assert!(one(k) > below(k), "step {k}: {} against {} below it", one(k), below(k));
        }
        assert!(forage_steps(1.0, 1.0, 1.0, 1.0, 8.0) > forage_steps(1.0, 1.0, 1.0, 1.0, 1.0));
        assert!(
            forage_steps(0.0, 0.0, 0.4, 0.0, 0.0) > forage_steps(0.0, 0.0, 0.2, 0.0, 0.0),
            "carrying further pays"
        );
        assert_eq!(
            forage_steps(3.0, 3.0, 3.0, 3.0, 0.0),
            15.0,
            "a share past 1 counts as 1"
        );
        assert_eq!(
            forage_steps(0.0, 0.0, 0.0, 0.0, 0.5),
            0.0,
            "no cell home is no cell home"
        );
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
        let forager = creature::mind::preset("forager", ch.verbs()).expect("forager");
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
        let per_tick = (def.synapse_fraction * def.start_energy * genome.live as f32) as f64;
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
    }

    /// **A suite folds each arena on its own before the product**: one
    /// arena done only east and another only west is worse side 1 on both,
    /// not the `sqrt(4 x 1) = 2` the first suite read on each side; a
    /// runaway ratio counts up to `RATIO_CAP`, except on the one arena a
    /// rung is learning; and a kept arena under `forget` of its reference
    /// costs nine tenths.
    #[test]
    fn a_suite_folds_each_arena_before_the_product() {
        let lay = [Layout { seed: 1, k: 0, blind: false, legal: false }, Layout { seed: 1, k: 1, blind: false, legal: false }];
        let g = vec![0.0f32; 1];
        let score = |s: &Suite| crate::train::score_clones(s, &g, &lay, 1).fit;
        let split = Suite::new(vec![(Box::new(Sided { east: 4.0, west: 1.0 }), false), (Box::new(Sided { east: 1.0, west: 4.0 }), false)], 0.8);
        assert!((score(&split) - 1.0).abs() < 1e-9, "one-sided arenas scored {}", score(&split));
        let runaway = Suite::new(vec![(Box::new(Sided { east: 10.0, west: 10.0 }), false), (Box::new(Sided { east: 1.0, west: 1.0 }), false)], 0.8);
        assert!((score(&runaway) - RATIO_CAP.sqrt()).abs() < 1e-9, "a ratio of 10 was not capped: {}", score(&runaway));
        let rung = Suite::new(vec![(Box::new(Sided { east: 10.0, west: 10.0 }), false), (Box::new(Sided { east: 1.0, west: 1.0 }), true)], 0.8);
        assert!((score(&rung) - 10f64.sqrt()).abs() < 1e-9, "the one arena being learned is not capped: {}", score(&rung));
        let mut kept = Suite::new(vec![(Box::new(Sided { east: 1.0, west: 1.0 }), true)], 0.8);
        assert!((score(&kept) - 1.0).abs() < 1e-9);
        kept.refs[0] = 1.5;
        assert!((score(&kept) - 0.1 / 1.5).abs() < 1e-9, "a kept arena at two thirds of its reference was not forgotten: {}", score(&kept));
        kept.refs[0] = 1.2;
        assert!((score(&kept) - 1.0 / 1.2).abs() < 1e-9, "at 83% of its reference it is kept, not forgotten: {}", score(&kept));
    }
}
