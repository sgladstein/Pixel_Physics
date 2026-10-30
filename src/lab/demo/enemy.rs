//! **The enemy seat's demo** (`demo=enemy`): the player is a centipede, the
//! shipped nest raider, against a living trained colony. The design is
//! `Reports/teaching-the-colony-2026-10-25/seat-enemy.md`; the feasibility
//! review, the plan, its review and the measurements are in
//! `Reports/teaching-the-colony-2026-10-25/enemy-seat-demo.md`.
//!
//! **What it honestly is: a colony that fights back, not one that learns you.**
//! Alarm is a three-second flash (`pheromone.rs`), and a trained ant born in the
//! box carries its parent's mind unchanged, so nothing the colony "knows" can
//! change inside a session. The help line says so.
//!
//! The body is the engine's own: the five-cell chain, jaw 0.5 (soil 0.8 and a
//! packed lining 0.95 stop it; stored crumbs do not), the plate (four ant bites
//! a cell), the gut with its handling time, the trained raider mind under the
//! keys (`steer::take_forced`). What this file adds is the start (a visible
//! fast-forward while the colony digs in), the player's body rules (no budding:
//! [`steer::Steered::may_breed`]), and **legibility**: a HUD that says what
//! the engine really does -- one wound meter per body, and the cell that takes
//! the filling bite comes off, so the head kills and the neck takes everything
//! behind it -- a pulse where every bite lands, a ring where you kill, the
//! nest's doors marked, and a season card.
//!
//! Every count is read from the world's own records (`kills_log`,
//! `encounter_log`, the body's wound meter, `creature_stats.trip_deliveries`),
//! never kept by a rule of this file's own, so the HUD cannot disagree with
//! what happened. `examples/enemyhunt.rs` plays it headless.

use crate::sim::cell::OrganismId;
use crate::sim::creature::options::Verb;
use crate::sim::steer;
use crate::sim::world::{World, ENCOUNTER_BITE};

use super::super::Lab;

/// The seat's own keys, as the help line prints them.
pub const SEAT_KEYS: &str = "Y SEASON CARD -- R A NEW RAIDER -- BACK OUT TAIL FIRST: THE CELL THEY BITE IS THE CELL YOU LOSE";

/// The keys once the stakes are on (the default): the score, the lair and the brood.
pub const STAKES_KEYS: &str = "B ON A LARVA GRABS IT -- CARRY IT TO YOUR LAIR (x10) -- ANTS x1 -- 3 RAIDERS";

/// **The stakes** (seat review 2026-10-30, E2 then E1): before them a raider on
/// the surface was close to immortal (0 cells lost in 3 of 4 skulk lives that
/// survived the landing), its tank only grew (1,600 -> 5,547 J), death was a
/// free R, and the nest -- the seat's subject -- was entered by 0 of 11 lives,
/// because nothing in it was worth the door. Now: the tank drains, so a raider
/// must keep hunting; it is capped, so a feeding streak cannot store
/// immortality; a game is three raiders; ants score 1 and a larva carried out
/// of the nest to the lair scores [`BROOD_POINTS`]; the first seconds after a
/// landing cannot cost a cell. `stakes=0` puts the sandbox back.
pub const LIVES: usize = 3;
/// What one larva banked at the lair is worth, in ants.
pub const BROOD_POINTS: u32 = 10;
/// **The seat's hunger, in joules a frame on top of the body's own bill**
/// (about 0.1 J a frame for five cells at a walk). With it a full tank lasts
/// about two and a half minutes at 1x: an ant eaten pays roughly ten to fifteen
/// seconds of it, so a raider that stops hunting is dead inside three minutes.
pub const HUNGER: f32 = 0.09;
/// **The tank's cap, in full tanks.** Above it the meal is burnt off; it sits
/// above the regrowth bar (a full tank plus one cell's stamp, 1.19 tanks for the
/// centipede) so a fed raider can still regrow.
pub const CAP_TANKS: f32 = 1.5;
/// **Frames after a landing in which no bite can take a cell** (3 s at 1x): the
/// wound meter is held at zero. Seed 3 was dead 47 frames after landing on
/// both policies, before a player could find the arrows.
pub const GRACE: u64 = 180;
/// How near the lair (columns, and rows of the surface) a carried larva banks.
pub const LAIR_REACH: i32 = 6;
/// How far from the head the jaws reach a larva (cells, any direction).
pub const JAW_REACH: i32 = 2;
/// Rows under the surface that count as "in the nest" for a life's record.
pub const BELOW: i32 = 3;

/// The frame the raider is set down on: the colony lands at 6,000
/// (`demo_enemy.ron`) and has 20,000 frames to dig in first.
pub const WARM_UNTIL: u64 = 26_000;

/// How far from the nearest door the raider lands, in columns (`land=`
/// overrides it): **past the fruit heap, at the edge of the foragers' road,
/// not on it.** Measured 2026-10-25 on main at 648d705 with
/// `examples/enemyhunt.rs policy=skulk` (a player who waits for a forager
/// walking alone and backs off when bitten), 6 seeds a brain, 12,000
/// frames. At 30 -- between the door and the heap, the busiest road in the
/// box -- the trained colony killed the raider on 4 of 6 seeds, two of them
/// inside 430 frames, dead before a player has found the keys, and the
/// raider took 0-4 ants (8 in all). At 70 it still died on 4 of 6, but none
/// before 2,050 frames, with 2-6 kills each (18 in all); against the
/// hand-built colony it lived every run with 2-10 kills (38, against 9 at
/// 30). Walking straight at the door (`policy=door`) died at the mound on
/// 12 of 12 runs, both brains, with no kill: the door is defended.
const LAND_FROM_DOOR: i32 = 70;

/// How long a bite pulse and a kill ring stay on screen, in frames.
const PULSE_FRAMES: u64 = 40;
const RING_FRAMES: u64 = 600;

/// How long a "why did that key do nothing" line stays up, in frames.
const NOTE_FRAMES: u64 = 120;

/// What a mark on the world says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// The player bit (yellow).
    YouBit,
    /// An ant bit the player (red).
    BitYou,
    /// The player killed here (a ring).
    Kill,
    /// The player grabbed a larva here (a white ring).
    Grab,
    /// A larva was banked at the lair (a gold ring).
    Banked,
}

/// The enemy seat's running state.
#[derive(Clone, Debug, Default)]
pub struct EnemyDemo {
    /// The player's centipede, once it has landed.
    pub raider: Option<OrganismId>,
    /// The frame it landed, and the colony's state then.
    pub landed: Option<u64>,
    pub trips_at_landing: u64,
    pub ants_at_landing: usize,
    /// Kills by the player (from `kills_log`), bites it gave (from
    /// `encounter_log`), and bites it took. **Taken is read off the body's
    /// own wound meter, not the log**: an ant that *eats* at an animal cell
    /// (the hand-built ant's gnaw, `creature.rs`'s Eat path) wounds it
    /// without writing an `ENCOUNTER_BITE` row, so the log alone counted 1
    /// bite on a raider whose head came off (seed 5, hand-built colony,
    /// `examples/enemyhunt.rs`, 2026-10-25). A tick is one bite here unless
    /// the log names more.
    pub kills: u32,
    pub bites_given: u32,
    pub bites_taken: u32,
    /// Each cell the player lost: the frame, and its place in the chain when it
    /// came off (0 is the head).
    pub cells_lost: Vec<(u64, usize)>,
    /// How many `kills_log` rows have been read.
    kills_read: usize,
    /// The chain length and energy at the last tick, to see a cell come off.
    last_len: usize,
    last_energy: f32,
    last_gnawed: f32,
    start_energy: f32,
    /// Marks on the world: when, where, what.
    pub marks: Vec<(u64, (i32, i32), Mark)>,
    /// One line saying why a key did nothing, and when it was written.
    pub note: Option<(u64, String)>,
    last_act_read: u64,
    /// The season card is up.
    pub card: bool,
    /// How the raider ended, once it has.
    pub ended: Option<(u64, &'static str)>,
    /// The warm-up has asked for speed.
    warming: bool,
    /// Frames this raider spent with an ant within [`CONTACT`] cells of its
    /// head: the time the colony had to bite it.
    pub contact: u64,
    /// Every raider life so far, oldest first (kept across `R`).
    pub seasons: Vec<Season>,
    /// The colony's minds mutate at birth (`mutate=0` turns it off), and the
    /// colony the store was armed for once it landed.
    pub mutating: bool,
    /// Its level (`mutate=`, [`BIRTH`]).
    pub level: u8,
    armed: bool,
    store_colony: Option<u32>,
    /// Eggs that took a mutated mind since the colony landed, and since this
    /// raider landed.
    pub mutants: u32,
    pub mutants_at_landing: u32,
    /// Shift is held, and how much of the raider's wind is spent (0 fresh, 1
    /// out of breath). Spent to 1 the raider is winded and may not sprint
    /// again until it is back to [`WINDED_UNTIL`].
    pub sprint_held: bool,
    pub spent: f32,
    pub winded: bool,
    /// Frames this raider spent at a sprint.
    pub sprint_frames: u64,
    /// The stakes are on (read from `stakes=` before the first landing).
    pub stakes: bool,
    stakes_read: bool,
    /// The lives of this game so far, oldest first, and the best game score
    /// this session.
    pub game: Vec<Life>,
    pub best: u32,
    /// Ants alive when this game's first raider landed.
    pub game_ants_at_start: usize,
    /// The game is over: three raiders spent.
    pub over: bool,
    /// The last game over was a new best.
    pub new_best: bool,
    /// This life: larvae grabbed, larvae banked at the lair, and the deepest
    /// row under the surface the head reached.
    pub grabbed: u32,
    pub brood: u32,
    pub deepest: i32,
    /// This life's lair: the surface cell it landed on.
    pub lair: Option<(i32, i32)>,
    /// Joules the hunger took, and the cap burnt off, this life.
    pub hunger_j: f64,
    pub capped_j: f64,
}

/// **One raider life in a game**: what it scored and how it ended.
#[derive(Clone, Copy, Debug, Default)]
pub struct Life {
    pub frames: u64,
    pub kills: u32,
    pub grabbed: u32,
    pub brood: u32,
    pub deepest: i32,
    pub how: &'static str,
}

impl Life {
    pub fn points(&self) -> u32 {
        self.kills + BROOD_POINTS * self.brood
    }
}

impl EnemyDemo {
    /// Points this raider has made so far (ants plus banked brood).
    pub fn life_points(&self) -> u32 {
        self.kills + BROOD_POINTS * self.brood
    }

    /// The game's score: every finished life plus the one running.
    pub fn score(&self) -> u32 {
        self.game.iter().map(Life::points).sum::<u32>() + if self.ended.is_none() && self.raider.is_some() { self.life_points() } else { 0 }
    }

    /// Raiders left in this game, the one on the surface included.
    pub fn raiders_left(&self) -> usize {
        LIVES.saturating_sub(self.game.len())
    }

    /// In the landing's grace.
    pub fn in_grace(&self, frame: u64) -> bool {
        self.stakes && self.ended.is_none() && self.landed.is_some_and(|l| frame < l + GRACE)
    }
}

/// **One raider life**: how long, what it took, what it cost, and how hard
/// the colony bit back per minute it had the chance.
#[derive(Clone, Copy, Debug, Default)]
pub struct Season {
    pub frames: u64,
    pub kills: u32,
    pub bites_taken: u32,
    pub contact: u64,
    pub cells_lost: usize,
    pub mutants: u32,
}

impl Season {
    /// Bites on the raider per 3,600 frames (a minute at 1x) of contact.
    pub fn bite_rate(&self) -> f64 {
        if self.contact == 0 {
            0.0
        } else {
            self.bites_taken as f64 * 3600.0 / self.contact as f64
        }
    }
}

/// "Contact": an ant cell within this many cells of the raider's head.
const CONTACT: i32 = 3;

/// **The birth mutation** (owner's pick on the thread card, 2026-10-25:
/// "Turn on brain mutation in demo"): every egg of the trained colony takes a
/// mutated copy of its parent's mind, by the TRAINING page's own operator
/// (`lab::training::mutate_genome`). The level argument is the fathers
/// store's standing one; every egg gets the same.
fn birth_mutate(g: &[f32], level: u8, seed: u64) -> Vec<f32> {
    let (pm, sigma, events) = BIRTH[usize::from(level.clamp(1, BIRTH.len() as u8)) - 1];
    crate::lab::training::mutate_genome(g, pm, sigma, events, seed)
}

/// **The levels, `mutate=1..3`: a tenth of the training page's rate (the
/// default), a third of it, and the rate itself** (`ColonyTrainer`'s `kpm`
/// 0.03, `ksigma` 1.0, `kevents` 10). Measured 2026-10-25 with
/// `examples/enemyhunt.rs warm=1000000000 frames=176000` (the colony alone,
/// no raider), 6 seeds a level, ants at frame 175,000: off 16 / 183 / 274 /
/// 324 / 326 / 261; the training rate 182 / 2 / 0 / 0 / 4 / 72 (three
/// extinct by 150,000: in the lab nothing culls a bad mind before it lays,
/// so the training bed's rate is a meltdown here); a third 319 / 361 / 147 /
/// 293 / 128 / 109; a tenth 253 / 368 / 307 / 276 / 336 / 238, the off
/// arm's spread. So the default is a tenth: every egg still takes a changed
/// mind (about 3,000 of them in a run that long), and the colony lives.
const BIRTH: [(f32, f32, usize); 3] = [(0.003, 1.0, 1), (0.01, 1.0, 3), (0.03, 1.0, 10)];

/// **Which body is the player's**: the raider, once it has landed. Before
/// that (the warm-up) there is none, and the launcher binds nothing.
pub fn pick_body(lab: &Lab) -> Option<OrganismId> {
    let e = lab.demo.as_ref()?.enemy.as_ref()?;
    if e.ended.is_some() {
        return None;
    }
    e.raider.filter(|&id| lab.world.organism(id).is_some())
}

fn live_ants(world: &World) -> usize {
    let Some(ant) = world.species.id_of("ant") else { return 0 };
    world.live_organism_ids().into_iter().filter(|&id| world.organism(id).is_some_and(|s| s.species == ant)).count()
}

/// **Set the raider down**: on the surface `LAND_FROM_DOOR` columns from the
/// nearest door, thinking with the lab raider pool's first member under the
/// keys, marked as the player's body with no young.
fn land_raider(lab: &mut Lab) -> Option<OrganismId> {
    let minds = crate::lab::training::predator_minds("centipede")?;
    let doors: Vec<i32> = lab.world.nest_sites.iter().map(|s| s.x).collect();
    let (x0, x1) = lab.world.bounds().map(|b| (b.min_x, b.max_x)).unwrap_or((0, 511));
    let mid = (x0 + x1) / 2;
    let door = doors.iter().copied().min_by_key(|d| (d - mid).abs()).unwrap_or(mid);
    let far_arg = lab.demo.as_ref().and_then(|d| d.args.get("land")).and_then(|v| v.parse().ok());
    let far = far_arg.unwrap_or(LAND_FROM_DOOR);
    let mut x = if door - far > x0 + 8 { door - far } else { door + far };
    let stakes = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).is_some_and(|e| e.stakes);
    if stakes && far_arg.is_none() {
        x = quiet_column(&lab.world, door, (x0, x1)).unwrap_or(x);
    }
    let id = crate::train::predator::place_predator(&mut lab.world, "centipede", x, None, minds[0].clone(), 1.0)?;
    lab.world.steered = None;
    steer::mark_with(&mut lab.world, id, Some(steer::SteerInput::default()), steer::SteerRole::Worker);
    if let Some(s) = lab.world.steered.as_mut() {
        s.may_breed = false;
    }
    // A full crop does not stop the player's bite (owner's playtest, 2026-10-25): the swallow is
    // skipped and the cell still comes off. `fullbite=0` puts the sated rule back, for the A/B.
    let full = lab.demo.as_ref().and_then(|d| d.args.get("fullbite")).is_none_or(|v| v != "0");
    if let Some(s) = lab.world.steered.as_mut() {
        s.bites_when_full = full;
    }
    lab.world.encounter_log = Some(Default::default());
    let (trips, ants) = (lab.world.creature_stats.trip_deliveries, live_ants(&lab.world));
    let state = lab.world.organism(id)?;
    let (len, energy) = (state.chain.len(), state.energy);
    let start = lab.world.species.get(state.species).creature.as_ref().map_or(1.0, |d| d.start_energy);
    let frame = lab.world.frame;
    let lair = state.chain.first().map(|&(hx, hy)| (hx, lab.world.room_surface_at(hx).unwrap_or(hy)));
    let e = lab.demo.as_mut()?.enemy.as_mut()?;
    let kept = std::mem::take(e);
    let new_game = kept.game.is_empty();
    *e = EnemyDemo {
        stakes: kept.stakes,
        stakes_read: kept.stakes_read,
        game: kept.game,
        best: kept.best,
        game_ants_at_start: if new_game { ants } else { kept.game_ants_at_start },
        lair,
        seasons: kept.seasons,
        mutating: kept.mutating,
        level: kept.level,
        armed: kept.armed,
        store_colony: kept.store_colony,
        mutants: kept.mutants,
        mutants_at_landing: kept.mutants,
        raider: Some(id),
        landed: Some(frame),
        trips_at_landing: trips,
        ants_at_landing: ants,
        kills_read: lab.world.kills_log.len(),
        last_len: len,
        last_energy: energy,
        start_energy: start,
        last_act_read: frame,
        ..EnemyDemo::default()
    };
    Some(id)
}

/// **The landing column with the fewest ants near it**, among columns 50-110
/// from the door on either side (the shipped 70 sits in the middle): the
/// fixed spot put seed 3's raider on its foragers' road, dead in 47 frames.
/// Ties keep the nearest to [`LAND_FROM_DOOR`].
fn quiet_column(world: &World, door: i32, (x0, x1): (i32, i32)) -> Option<i32> {
    let ant = world.species.id_of("ant")?;
    let heads: Vec<(i32, i32)> = world.live_organism_ids().into_iter().filter_map(|id| world.organism(id).filter(|s| s.species == ant).and_then(|s| s.chain.first().copied())).collect();
    let mut best: Option<(usize, i32, i32)> = None;
    for side in [-1, 1] {
        for far in (50..=110).step_by(10) {
            let x = door + side * far;
            if x < x0 + 12 || x > x1 - 12 {
                continue;
            }
            let y = world.room_surface_at(x).unwrap_or(0);
            let near = heads.iter().filter(|h| (h.0 - x).abs() <= 16 && (h.1 - y).abs() <= 12).count();
            let key = (near, (far - LAND_FROM_DOOR).abs(), x);
            if best.is_none_or(|b| key < b) {
                best = Some(key);
            }
        }
    }
    best.map(|b| b.2)
}

/// **The body letters the seat routes itself**: `B`, so a bite at a larva
/// grabs it (the body's own bite does not take brood) and every other bite
/// goes on to the body unchanged.
pub fn takes(c: char) -> bool {
    c == 'B'
}

/// **Rows of ground over the head**: solid cells (no animal's) in the 30
/// rows straight above it. "In the nest" is a roof, not a row number: a
/// grown mound stands above the bed's surface datum, and the first heist
/// runs grabbed larvae 14 rows *above* it.
pub fn cover(world: &World, (x, y): (i32, i32)) -> i32 {
    (1..=30).filter(|&k| world.in_bounds(x, y - k) && { let c = world.get(x, y - k); !c.is_empty() && c.organism_id() == 0 }).count() as i32
}

/// The larva nearest the raider's head within [`JAW_REACH`]: its id and cell.
fn larva_in_reach(world: &World, head: (i32, i32)) -> Option<(OrganismId, (i32, i32))> {
    let mut best: Option<(i32, OrganismId, (i32, i32))> = None;
    for dy in -JAW_REACH..=JAW_REACH {
        for dx in -JAW_REACH..=JAW_REACH {
            let (x, y) = (head.0 + dx, head.1 + dy);
            if !world.in_bounds(x, y) {
                continue;
            }
            let o = world.get(x, y).organism_id();
            if o != 0 && world.organism(o).and_then(|s| s.brood).is_some_and(|b| b.stage == crate::sim::organism::BroodStage::Larva) {
                let d = dx.abs().max(dy.abs());
                if best.is_none_or(|b| d < b.0) {
                    best = Some((d, o, (x, y)));
                }
            }
        }
    }
    best.map(|b| (b.1, b.2))
}

/// Whether the raider's jaws hold a corpse (a grabbed larva).
pub fn jaws_hold_brood(world: &World, id: OrganismId) -> bool {
    let corpse = world.materials.known(crate::sim::material::Known::Corpse);
    world.organism(id).and_then(|s| s.spoil).is_some_and(|sp| Some(sp.cell.material) == corpse)
}

/// **B: the heist's grab.** A larva within the jaws' reach is killed where it
/// lies (`brood::larva_eaten`, the guest seat's own kill: a corpse holding its
/// bank, and an alarm cried at it) and the corpse is lifted into the jaws, as
/// the storeroom's pick-up lifts a crumb (`Spoil`), so it rides with the body,
/// falls beside it if the raider dies, and goes down with Q. `Err` says why
/// nothing was grabbed; `Ok(false)` is "no larva here, bite as usual".
fn grab_larva(lab: &mut Lab) -> Result<bool, &'static str> {
    let Some(id) = pick_body(lab) else { return Ok(false) };
    let Some(head) = lab.world.organism(id).and_then(|s| s.chain.first().copied()) else { return Ok(false) };
    let Some((larva, at)) = larva_in_reach(&lab.world, head) else { return Ok(false) };
    if lab.world.organism(id).is_some_and(|s| s.spoil.is_some()) {
        return Err("JAWS FULL -- BANK IT AT THE LAIR, OR Q DROPS IT");
    }
    if !crate::sim::brood::larva_eaten(&mut lab.world, larva, at) {
        return Ok(false);
    }
    let corpse = lab.world.materials.known(crate::sim::material::Known::Corpse);
    let cell = lab.world.get(at.0, at.1);
    let load = if Some(cell.material) == corpse && cell.organism_id() == 0 {
        lab.world.set(at.0, at.1, crate::sim::cell::Cell::EMPTY);
        cell
    } else {
        // A larva too thin to leave a corpse (under a joule): the jaws still
        // carry its husk, worth nothing to eat.
        corpse.map_or(crate::sim::cell::Cell::EMPTY, |c| crate::sim::cell::Cell::new(c, 0))
    };
    if let Some(s) = lab.world.organism_mut(id) {
        s.spoil = Some(crate::sim::organism::Spoil { cell: load, store: false });
    }
    lab.world.log_player_action(format!("raider grabbed a larva at ({}, {})", at.0, at.1));
    if let Some(e) = lab.demo.as_mut().and_then(|d| d.enemy.as_mut()) {
        e.grabbed += 1;
        e.marks.push((lab.world.frame, at, Mark::Grab));
    }
    Ok(true)
}

/// **The heist's bank**: a raider whose jaws hold a larva and whose head is
/// within [`LAIR_REACH`] of its lair puts it down there, on the first open
/// cell over ground nearest the lair, and scores [`BROOD_POINTS`]. The corpse
/// stays: a pile of trophies the raider can eat later (E), and the colony can
/// carry back.
fn bank_at_lair(lab: &mut Lab, id: OrganismId) {
    let Some(lair) = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).and_then(|e| e.lair) else { return };
    let Some(head) = lab.world.organism(id).and_then(|s| s.chain.first().copied()) else { return };
    if (head.0 - lair.0).abs() > LAIR_REACH || (head.1 - lair.1).abs() > LAIR_REACH || !jaws_hold_brood(&lab.world, id) {
        return;
    }
    let w = &lab.world;
    let mut spot: Option<(i32, (i32, i32))> = None;
    for dx in -4..=4 {
        let x = lair.0 + dx;
        let Some(top) = w.room_surface_at(x) else { continue };
        // The first open cell going up from the ground in this column.
        let Some(y) = (top - 8..=top + 2).rev().find(|&y| w.in_bounds(x, y) && w.is_empty(x, y) && w.in_bounds(x, y + 1) && !w.is_empty(x, y + 1)) else { continue };
        let d = dx.abs();
        if spot.is_none_or(|s| d < s.0) {
            spot = Some((d, (x, y)));
        }
    }
    let Some((_, (x, y))) = spot else { return };
    let Some(load) = lab.world.organism_mut(id).and_then(|s| s.spoil.take()) else { return };
    lab.world.set(x, y, load.cell);
    lab.world.log_player_action(format!("raider banked a larva at its lair ({x}, {y})"));
    let frame = lab.world.frame;
    if let Some(e) = lab.demo.as_mut().and_then(|d| d.enemy.as_mut()) {
        e.brood += 1;
        e.marks.push((frame, (x, y), Mark::Banked));
        e.note = Some((frame, format!("LARVA BANKED: +{BROOD_POINTS}")));
    }
}

/// One of the seat keys was pressed.
pub fn seat_key(lab: &mut Lab, key: char) {
    match key {
        'Y' => {
            if let Some(e) = lab.demo.as_mut().and_then(|d| d.enemy.as_mut()) {
                e.card = !e.card;
            }
        }
        'B' => {
            let stakes = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).is_some_and(|e| e.stakes);
            let grabbed = if stakes { grab_larva(lab) } else { Ok(false) };
            match grabbed {
                Ok(true) => {}
                Ok(false) => {
                    if let Some(s) = lab.world.steered.as_mut() {
                        s.press(steer::SteerInput { bite: true, ..steer::SteerInput::default() });
                    }
                }
                Err(why) => {
                    let frame = lab.world.frame;
                    if let Some(e) = lab.demo.as_mut().and_then(|d| d.enemy.as_mut()) {
                        e.note = Some((frame, why.to_string()));
                    }
                }
            }
        }
        'R' => {
            let Some(e) = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()) else { return };
            if e.ended.is_none() {
                return;
            }
            let (stakes, over) = (e.stakes, e.over);
            if stakes && over {
                if let Some(e) = lab.demo.as_mut().and_then(|d| d.enemy.as_mut()) {
                    e.game.clear();
                    e.over = false;
                    e.new_best = false;
                }
            }
            if land_raider(lab).is_some() {
                let left = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).map_or(0, EnemyDemo::raiders_left);
                if !stakes {
                    lab.ui.say("A NEW RAIDER ON THE SURFACE -- THE COLONY IS AS YOU LEFT IT");
                } else if over {
                    lab.ui.say("A NEW GAME: THREE RAIDERS, SCORE 0 -- THE COLONY IS AS YOU LEFT IT");
                } else {
                    lab.ui.say(format!("RAIDER {} OF {LIVES} ON THE SURFACE -- THE COLONY IS AS YOU LEFT IT", LIVES + 1 - left));
                }
            }
        }
        _ => {}
    }
}

/// **Arm the birth mutation**: before the colony lands, every ant's mind is
/// drawn from minds the store built (so a child of any of them can be
/// rebuilt mutated); once the colony has landed, its store is opened with a
/// standing wild level. Trained colony only: the hand-built brain has its
/// own genome and its own birth mutation already.
fn arm_mutation(lab: &mut Lab) {
    let Some(e) = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()) else { return };
    if e.store_colony.is_some() || !lab.new_ant {
        return;
    }
    let level = match lab.demo.as_ref().and_then(|d| d.args.get("mutate")).map(String::as_str) {
        None => 1,
        Some("off") => 0,
        Some(v) => v.parse::<u8>().unwrap_or(1).min(BIRTH.len() as u8),
    };
    if level == 0 {
        return;
    }
    if !e.armed {
        use crate::train::challenges::colony_bed;
        let genomes = crate::lab::training::std_pool();
        let minds: Vec<_> = genomes.iter().map(|g| colony_bed::mind(g)).collect();
        let mut f = crate::sim::fathers::Fathers::new(colony_bed::mind, birth_mutate);
        for (m, g) in minds.iter().zip(genomes) {
            f.know(m, &std::sync::Arc::new(g));
        }
        lab.world.fathers = Some(Box::new(f));
        lab.set_new_ant_pool(minds);
        if let Some(e) = lab.demo.as_mut().and_then(|d| d.enemy.as_mut()) {
            e.armed = true;
            e.mutating = true;
            e.level = level;
        }
        return;
    }
    let Some(ant) = lab.world.species.id_of("ant") else { return };
    let colony = lab.world.live_organism_ids().into_iter().filter_map(|id| lab.world.organism(id)).find(|s| s.species == ant).map(|s| s.colony);
    let Some(colony) = colony else { return };
    if let Some(f) = lab.world.fathers.as_mut() {
        f.colonies.insert(colony, crate::sim::fathers::Store { wild_all: level, ..Default::default() });
    }
    if let Some(e) = lab.demo.as_mut().and_then(|d| d.enemy.as_mut()) {
        e.store_colony = Some(colony);
    }
}

/// Once a displayed frame: the warm-up's speed, then the landing.
pub fn frame(lab: &mut Lab) {
    arm_mutation(lab);
    let stakes = lab.demo.as_ref().and_then(|d| d.args.get("stakes")).is_none_or(|v| v != "0");
    if let Some(e) = lab.demo.as_mut().and_then(|d| d.enemy.as_mut()) {
        if !e.stakes_read {
            e.stakes = stakes;
            e.stakes_read = true;
        }
    }
    let Some(e) = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()) else { return };
    if e.raider.is_some() {
        return;
    }
    // `warm=` moves the landing (a test, or a player who wants straight in).
    let warm = lab.demo.as_ref().and_then(|d| d.args.get("warm")).and_then(|v| v.parse().ok()).unwrap_or(WARM_UNTIL);
    // And never before the colony has a door: `warm=0` would otherwise set
    // the raider down on an empty bed, 6,000 frames before the ants land.
    if lab.world.frame < warm || lab.world.nest_sites.is_empty() {
        if !e.warming {
            // The top of the dial that still draws: the colony digging in is the
            // demo's opening shot, and a blank window reads as a hang.
            lab.time.set_preset(5);
            if let Some(e) = lab.demo.as_mut().and_then(|d| d.enemy.as_mut()) {
                e.warming = true;
            }
        }
        return;
    }
    if land_raider(lab).is_some() {
        lab.time.set_preset(0);
        // **In close for the hunt**: the warm-up shows the whole box, where a
        // five-cell body is five pixels; three screen pixels a cell is where
        // a bite on its tail can be told from one on its head. The camera
        // then follows it (`Demo::follow`, Tab lets go).
        let bounds = lab.world.bounds();
        for _ in 0..2 {
            lab.renderer.zoom_within(1, (super::super::WIDTH, super::super::HEIGHT), bounds);
        }
        // The BATTLE panel opens on its own at the first predator kill and
        // lands on top of this HUD, saying less than it does: off for the
        // hunt (F9 still opens it).
        if lab.ui.battle.toggle() {
            lab.ui.battle.toggle();
        }
        let stakes = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).is_some_and(|e| e.stakes);
        if stakes {
            lab.ui.say("RAIDER 1 OF 3 -- EAT ANTS (x1), CARRY LARVAE TO YOUR LAIR (x10)");
        } else {
            lab.ui.say(format!("YOU ARE THE RAIDER -- {}", SEAT_KEYS));
        }
    }
}

/// **Frames of sprint in a full wind** (4 s at 1x), **frames to get it all
/// back** (10 s), and **how far it must come back after running out** before
/// Shift works again. Hit and run, owner's playtest 2026-10-25: at its own
/// pace the raider could not get away from a crowd, and nothing made backing
/// off and coming back a choice. A wind meter rather than a charge on the
/// tank, because the tank holds 1,600 J and the raider's whole bill is about
/// 0.1 J a frame: a sprint already pays 2.5x that per frame
/// (`steer::SPRINT_PACE`), and that is not a price anyone would notice.
const SPRINT_FRAMES: f32 = 240.0;
const RECOVER_FRAMES: f32 = 600.0;
const WINDED_UNTIL: f32 = 0.4;

/// Shift pressed or let go.
pub fn sprint_key(lab: &mut Lab, pressed: bool) {
    if let Some(e) = lab.demo.as_mut().and_then(|d| d.enemy.as_mut()) {
        e.sprint_held = pressed;
    }
}

/// **The stakes' per-tick rules on the body**: the grace holds the wound
/// meter at zero, and the hunger and the cap move the tank, booked to the
/// ledger as the body's own bills are (`Metabolized`; the cap's burn-off
/// `Dissipated`), so the colony books still close.
fn stakes_on_body(lab: &mut Lab, hunger: bool) {
    let Some(e) = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()) else { return };
    if !e.stakes || e.ended.is_some() {
        return;
    }
    let Some(id) = e.raider else { return };
    let grace = e.in_grace(lab.world.frame);
    let Some(start) = lab.world.organism(id).and_then(|s| lab.world.species.get(s.species).creature.as_ref().map(|d| d.start_energy)) else { return };
    let colony = lab.world.colony_of(id);
    let (mut burnt, mut capped) = (0.0f32, 0.0f32);
    if let Some(s) = lab.world.organism_mut(id) {
        if grace {
            s.gnawed = 0.0;
        }
        if hunger {
            burnt = HUNGER.min(s.energy.max(0.0));
            s.energy -= burnt;
            let cap = CAP_TANKS * start;
            if s.energy > cap {
                capped = s.energy - cap;
                s.energy = cap;
            }
        }
    }
    if burnt > 0.0 {
        lab.world.book(colony, crate::sim::world::Account::Metabolized, f64::from(burnt));
    }
    if capped > 0.0 {
        lab.world.book(colony, crate::sim::world::Account::Dissipated, f64::from(capped));
    }
    if let Some(e) = lab.demo.as_mut().and_then(|d| d.enemy.as_mut()) {
        e.hunger_j += f64::from(burnt);
        e.capped_j += f64::from(capped);
    }
}

/// **Before each simulated tick**: spend or recover wind, and tell the body
/// whether it sprints this tick.
pub fn before_tick(lab: &mut Lab) {
    stakes_on_body(lab, true);
    let Some(e) = lab.demo.as_mut().and_then(|d| d.enemy.as_mut()) else { return };
    let run = e.raider.is_some() && e.ended.is_none() && e.sprint_held && !e.winded;
    if run {
        e.spent = (e.spent + 1.0 / SPRINT_FRAMES).min(1.0);
        e.winded = e.spent >= 1.0;
        e.sprint_frames += 1;
    } else {
        e.spent = (e.spent - 1.0 / RECOVER_FRAMES).max(0.0);
        e.winded &= e.spent > WINDED_UNTIL;
    }
    let sprint = run && !e.winded;
    if let Some(s) = lab.world.steered.as_mut() {
        s.sprint = sprint;
    }
}

/// **Eggs that took a mutated mind**, from the store's own log (drained, so it
/// does not grow for the life of the box), and every 600 frames the store's
/// genome table is cut to minds still in use: a mutated mind no ant carries
/// any more is held only by the table, and with every egg mutating the table
/// would otherwise keep every mind the colony ever had.
fn count_births(lab: &mut Lab) {
    let Some(colony) = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).and_then(|e| e.store_colony) else { return };
    let frame = lab.world.frame;
    let Some(f) = lab.world.fathers.as_mut() else { return };
    let n = f.log.drain(..).filter(|b| b.colony == colony && b.wild > 0).count() as u32;
    if frame.is_multiple_of(600) {
        f.genomes.retain(|(m, _)| std::sync::Arc::strong_count(m) > 1);
    }
    if let Some(e) = lab.demo.as_mut().and_then(|d| d.enemy.as_mut()) {
        e.mutants += n;
    }
}

/// **After each simulated tick**: read what happened to and by the raider from
/// the world's own logs.
pub fn after_tick(lab: &mut Lab) {
    count_births(lab);
    let Some(id) = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).and_then(|e| e.raider) else { return };
    let frame = lab.world.frame;
    let w = &mut lab.world;
    let ant = w.species.id_of("ant");
    let near = w.organism(id).and_then(|s| s.chain.first().copied()).is_some_and(|(hx, hy)| {
        (-CONTACT..=CONTACT).any(|dy| {
            (-CONTACT..=CONTACT).any(|dx| {
                let (x, y) = (hx + dx, hy + dy);
                w.in_bounds(x, y) && {
                    let o = w.get(x, y).organism_id();
                    o != 0 && o != id && w.organism(o).is_some_and(|s| Some(s.species) == ant)
                }
            })
        })
    });
    let head = w.organism(id).and_then(|s| s.chain.first().copied());
    let (len, energy, gnawed, alive) = w.organism(id).map_or((0, 0.0, 0.0, false), |s| (s.chain.len(), s.energy, s.gnawed, true));
    let mut bites = Vec::new();
    if let Some(log) = w.encounter_log.as_mut() {
        for ev in log.events.drain(..) {
            if ev.kind == ENCOUNTER_BITE && (ev.a == id || ev.b == id) {
                bites.push(ev);
            }
        }
    }
    let new_kills: Vec<_> = w.kills_log.iter().skip(lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).map_or(0, |e| e.kills_read)).copied().collect();
    let kills_len = w.kills_log.len();
    let last_act = w.steered.as_ref().filter(|s| s.id == id).and_then(|s| s.last_act);
    let spoil = w.organism(id).is_some_and(|s| s.spoil.is_some());
    let crop_full = w.organism(id).is_some_and(|s| {
        let def = w.species.get(s.species).creature.as_ref();
        match (s.crop, def) {
            (Some(c), Some(d)) => c.worth() + c.unit > crate::sim::creature::organism_crop_capacity(w, id, d),
            _ => false,
        }
    });
    let ant_at = |w: &World, other: OrganismId| w.organism(other).and_then(|s| s.chain.first().copied());
    let marks: Vec<(u64, (i32, i32), Mark)> = bites
        .iter()
        .filter_map(|ev| {
            if ev.a == id {
                head.map(|h| (frame, h, Mark::YouBit))
            } else {
                ant_at(w, ev.a).or(head).map(|p| (frame, p, Mark::BitYou))
            }
        })
        .collect();
    let Some(e) = lab.demo.as_mut().and_then(|d| d.enemy.as_mut()) else { return };
    let logged_taken = bites.iter().filter(|ev| ev.a != id).count() as u32;
    e.bites_given += bites.len() as u32 - logged_taken;
    let starved = e.last_energy < 0.02 * e.start_energy;
    let wounded = if alive { gnawed > e.last_gnawed + 1e-4 || len < e.last_len } else { e.ended.is_none() && !starved };
    e.bites_taken += logged_taken.max(wounded as u32);
    if wounded && logged_taken == 0 {
        if let Some(h) = head.or(e.marks.last().map(|m| m.1)) {
            e.marks.push((frame, h, Mark::BitYou));
        }
    }
    e.marks.extend(marks);
    for k in &new_kills {
        if k.detail.attacker == id {
            e.kills += 1;
            if let Some(h) = head {
                e.marks.push((frame, h, Mark::Kill));
            }
        }
    }
    e.kills_read = kills_len;
    if alive && len < e.last_len {
        // Cells came off: the one at `len` in the old chain and everything
        // behind it (a severed neck takes the rest), one row a cell -- one row
        // a bite read "1 CELLS TAKEN" on a raider down to 2 of 5.
        e.cells_lost.extend((len..e.last_len).map(|i| (frame, i)));
    }
    if !alive && e.ended.is_none() {
        if !starved {
            e.cells_lost.push((frame, 0));
        }
        e.ended = Some((frame, if starved { "YOU STARVED" } else { "THEY BIT YOUR HEAD OFF" }));
        e.card = true;
        if e.stakes {
            e.game.push(Life {
                frames: frame.saturating_sub(e.landed.unwrap_or(frame)),
                kills: e.kills,
                grabbed: e.grabbed,
                brood: e.brood,
                deepest: e.deepest,
                how: if starved { "STARVED" } else { "BITTEN" },
            });
            if e.game.len() >= LIVES {
                e.over = true;
                let score = e.score();
                e.new_best = score > e.best;
                e.best = e.best.max(score);
            }
        }
        let season = Season {
            frames: frame.saturating_sub(e.landed.unwrap_or(frame)),
            kills: e.kills,
            bites_taken: e.bites_taken,
            contact: e.contact,
            cells_lost: e.cells_lost.len(),
            mutants: e.mutants - e.mutants_at_landing,
        };
        e.seasons.push(season);
    }
    if alive && near {
        e.contact += 1;
    }
    if alive {
        e.last_len = len;
        e.last_energy = energy;
        e.last_gnawed = gnawed;
    }
    if let Some(a) = last_act.filter(|a| a.frame > e.last_act_read) {
        e.last_act_read = a.frame;
        let why = match a.asked {
            Some(Verb::Attack) if a.done == Verb::Attack && !a.ok => Some("THEIR PLATE HELD"),
            Some(Verb::Attack) if a.done != Verb::Attack => Some("NOTHING TO BITE AT YOUR HEAD"),
            Some(Verb::Dig) if a.done != Verb::Dig => Some(if spoil { "JAWS FULL -- PUT THE CRUMB DOWN (Q)" } else { "NOTHING YOUR JAW CAN CUT THERE" }),
            Some(Verb::PickUp) if a.done != Verb::PickUp && a.done != Verb::Eat => Some(if crop_full { "YOUR GUT IS FULL" } else { "NO FLESH IN REACH" }),
            Some(Verb::Eat) => Some("YOUR GUT SETS THE PACE"),
            Some(Verb::Drop) if a.done != Verb::Drop => Some("NOTHING TO PUT DOWN"),
            _ => None,
        };
        if let Some(why) = why {
            e.note = Some((frame, why.to_string()));
        }
    }
    e.marks.retain(|&(f, _, m)| frame.saturating_sub(f) < if matches!(m, Mark::Kill | Mark::Grab | Mark::Banked) { RING_FRAMES } else { PULSE_FRAMES });
    let stakes = e.stakes;
    if stakes && alive {
        if let Some(h) = head {
            // Under ground and within the nest's reach of a door: the box's
            // walls and lid put "ground" over a raider that climbed them.
            let by_door = lab.world.nest_sites.iter().any(|s| (s.x - h.0).abs() <= 40);
            let depth = if by_door { cover(&lab.world, h) } else { 0 };
            if let Some(e) = lab.demo.as_mut().and_then(|d| d.enemy.as_mut()) {
                e.deepest = e.deepest.max(depth);
            }
        }
        bank_at_lair(lab, id);
    }
    stakes_on_body(lab, false);
}

/// **The pointer over the world.** With the stakes on the seat takes every
/// press and does nothing with it: the lab's toolbar stays on screen, but
/// CULL, PLANT, ADD SOIL and the rest cannot reach the box, so the colony
/// cannot be culled with the mouse -- a bigger weapon than the centipede
/// (seat review 2026-10-30, finding 8). `stakes=0` hands the pointer back.
pub fn mouse(lab: &mut Lab, _cell: (i32, i32), act: super::MouseAct) -> bool {
    let stakes = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).is_some_and(|e| e.stakes);
    if stakes && matches!(act, super::MouseAct::Press { .. }) {
        let frame = lab.world.frame;
        if let Some(e) = lab.demo.as_mut().and_then(|d| d.enemy.as_mut()) {
            e.note = Some((frame, "THE LAB'S TOOLS ARE OFF IN A RAID (stakes=0 FOR THE SANDBOX)".to_string()));
        }
    }
    stakes
}

/// Whether [`draw_world`] draws anything this frame.
pub fn marks_world(lab: &Lab) -> bool {
    lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).is_some_and(|e| e.raider.is_some())
}

/// **The marks over the world**: every door, a ring on the player, a pulse
/// where a bite landed (yellow yours, red theirs), a ring where you killed.
/// Full replaces on fixed colours (`ui::draw_world_marks`' rule).
pub fn draw_world(lab: &Lab, frame_buf: &mut [u8]) {
    let Some(e) = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()) else { return };
    if e.raider.is_none() {
        return;
    }
    let hc = crate::lab::ui::hud_canvas(&lab.renderer);
    let at = |x: i32, y: i32| {
        let (x0, y0, x1, y1, _) = lab.renderer.world_rect_to_logical(x, y, x, y);
        ((x0 + x1) / 2, (y0 + y1) / 2)
    };
    let top = crate::lab::ui::bar_top();
    let dot = |frame_buf: &mut [u8], cx: i32, cy: i32, r: i32, colour: [u8; 4]| {
        for dy in -r..=r {
            for dx in -r..=r {
                let d2 = dx * dx + dy * dy;
                if d2 <= r * r && d2 >= (r - 1) * (r - 1) && cy + dy < top {
                    hc.put(frame_buf, cx + dx, cy + dy, colour);
                }
            }
        }
    };
    // Bigger doors with the stakes on: the door is now where the score is,
    // and a 5-pixel chevron was easy to miss (seat review, finding 4).
    let door_size = if e.stakes { 5 } else { 3 };
    for site in &lab.world.nest_sites {
        let top_y = lab.world.room_surface_at(site.x).unwrap_or(site.y);
        let (cx, cy) = at(site.x, top_y - 3);
        for k in 0..door_size {
            if cy - k < top {
                hc.put(frame_buf, cx - k, cy - k, DOOR);
                hc.put(frame_buf, cx + k, cy - k, DOOR);
            }
        }
    }
    let now = lab.world.frame;
    for &(f, (x, y), m) in &e.marks {
        let (cx, cy) = at(x, y);
        let age = now.saturating_sub(f);
        match m {
            Mark::YouBit => dot(frame_buf, cx, cy, 2 + (age / 10) as i32, YOU_BIT),
            Mark::BitYou => dot(frame_buf, cx, cy, 2 + (age / 10) as i32, BIT_YOU),
            Mark::Kill => dot(frame_buf, cx, cy, 4, KILL),
            Mark::Grab => {
                dot(frame_buf, cx, cy, 5, LARVA);
                dot(frame_buf, cx, cy, 7, LARVA);
            }
            Mark::Banked => {
                dot(frame_buf, cx, cy, 5, LAIR);
                dot(frame_buf, cx, cy, 8, LAIR);
            }
        }
    }
    if e.stakes {
        // **The brood, in white**: a larva is one cell, the colour of the
        // soil's pale grains, and the heist is for them -- every living larva
        // is a full replace on a fixed white, ringed when it is in the jaws'
        // reach.
        let head = e.raider.and_then(|id| lab.world.organism(id)).and_then(|s| s.chain.first().copied());
        for id in lab.world.live_brood_ids() {
            let Some(s) = lab.world.organism(id) else { continue };
            if !s.brood.is_some_and(|b| b.stage == crate::sim::organism::BroodStage::Larva) {
                continue;
            }
            let Some(&(x, y)) = s.chain.first().or_else(|| s.cells.keys().next()) else { continue };
            let (x0, y0, x1, y1, _) = lab.renderer.world_rect_to_logical(x, y, x, y);
            for py in y0..=y1.max(y0) {
                for px in x0..=x1.max(x0) {
                    if py < top {
                        hc.put(frame_buf, px, py, LARVA);
                    }
                }
            }
            if head.is_some_and(|h| (h.0 - x).abs().max((h.1 - y).abs()) <= JAW_REACH) {
                let (cx, cy) = at(x, y);
                dot(frame_buf, cx, cy, 3, LARVA);
            }
        }
        // **The lair**: a gold flag on the surface where this raider landed,
        // with the reach it banks in drawn as a bracket on the ground.
        if let Some((lx, ly)) = e.lair {
            let (cx, cy) = at(lx, ly - 1);
            let (rx, _) = at(lx + LAIR_REACH, ly);
            let span = (rx - cx).max(4);
            for k in 0..14 {
                if cy - k < top {
                    hc.put(frame_buf, cx, cy - k, LAIR);
                }
            }
            for k in 0..6 {
                for j in 0..(6 - k) {
                    if cy - 13 + k < top {
                        hc.put(frame_buf, cx + 1 + j, cy - 13 + k, LAIR);
                    }
                }
            }
            for dx in -span..=span {
                if cy + 1 < top {
                    hc.put(frame_buf, cx + dx, cy + 1, LAIR);
                }
            }
            for k in 0..4 {
                if cy + 1 - k < top {
                    hc.put(frame_buf, cx - span, cy + 1 - k, LAIR);
                    hc.put(frame_buf, cx + span, cy + 1 - k, LAIR);
                }
            }
        }
    }
    // **The player's body in its own colour.** The shipped centipede is
    // amber, the ants' colour within a shade, and at hunt zoom the first
    // frames of the demo could not be read: which of three amber dots is
    // you? The world's own pixels under it are replaced, cell for cell, so
    // a severed tail goes back to amber the frame it comes off.
    if let Some(s) = e.raider.and_then(|id| lab.world.organism(id)) {
        for (i, &(x, y)) in s.chain.iter().enumerate() {
            let (x0, y0, x1, y1, _) = lab.renderer.world_rect_to_logical(x, y, x, y);
            let colour = if i == 0 { HEAD } else { BODY };
            for py in y0..=y1.max(y0) {
                for px in x0..=x1.max(x0) {
                    if py < top {
                        hc.put(frame_buf, px, py, colour);
                    }
                }
            }
        }
    }
    if let Some(head) = e.raider.and_then(|id| lab.world.organism(id)).and_then(|s| s.chain.first().copied()) {
        let (cx, cy) = at(head.0, head.1);
        dot(frame_buf, cx, cy, 6, YOU);
        // The grace: a second, pulsing cyan ring while no bite can take a cell.
        if e.in_grace(lab.world.frame) && (lab.world.frame / 8).is_multiple_of(2) {
            dot(frame_buf, cx, cy, 9, DOOR);
        }
        // A larva in the jaws: a white block on the head, big enough to read.
        if e.raider.is_some_and(|id| jaws_hold_brood(&lab.world, id)) {
            // Above the ring, not under it, outlined dark so it reads on sky
            // and on soil alike.
            for dy in -3i32..=3 {
                for dx in -3i32..=3 {
                    let edge = dx.abs() == 3 || dy.abs() == 3;
                    if cy - 12 + dy < top {
                        hc.put(frame_buf, cx + dx, cy - 12 + dy, if edge { [20, 20, 20, 255] } else { LARVA });
                    }
                }
            }
        }
    }
}

const DOOR: [u8; 4] = [120, 220, 255, 255];
const HEAD: [u8; 4] = [255, 110, 170, 255];
const BODY: [u8; 4] = [200, 40, 110, 255];
const YOU: [u8; 4] = [255, 255, 255, 255];
const YOU_BIT: [u8; 4] = [255, 220, 60, 255];
const BIT_YOU: [u8; 4] = [255, 60, 60, 255];
const KILL: [u8; 4] = [255, 120, 40, 255];
const LARVA: [u8; 4] = [255, 255, 235, 255];
const LAIR: [u8; 4] = [255, 205, 40, 255];

/// The HUD's lines, as plain text: what the panel draws and what a headless
/// harness prints, from one function.
pub fn hud_lines(lab: &Lab) -> Vec<String> {
    let Some(e) = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()) else { return Vec::new() };
    let w = &lab.world;
    let brain = if lab.new_ant { "TRAINED" } else { "HAND-BUILT" };
    if e.landed.is_none() {
        return vec![format!("THE COLONY IS DIGGING IN: {} / {WARM_UNTIL}  ({brain} ANTS)", w.frame.min(WARM_UNTIL))];
    };
    if e.stakes {
        return stakes_hud(lab, e);
    }
    let mut out = Vec::new();
    let ants = live_ants(w);
    let trips = w.creature_stats.trip_deliveries.saturating_sub(e.trips_at_landing);
    if let Some(s) = e.raider.and_then(|id| w.organism(id)) {
        let quarters = (s.gnawed.clamp(0.0, 1.0) * 4.0).floor() as usize;
        let def = w.species.get(s.species).creature.as_ref();
        let plan = def.map_or(s.chain.len(), |d| match d.body {
            crate::sim::organism::BodyPlan::Chain(n) => usize::from(n),
            _ => s.chain.len(),
        });
        let tank_over = def.is_some_and(|d| s.energy >= d.start_energy + d.body_energy);
        out.push(format!(
            "RAIDER {}/{plan} CELLS  WOUND {}{}  {}",
            s.chain.len(),
            "*".repeat(quarters),
            "-".repeat(4 - quarters.min(4)),
            if s.chain.len() >= plan {
                "A FULL WOUND TAKES THE BITTEN CELL; THE HEAD KILLS"
            } else if s.gnawed > 0.0 {
                "HEAL FIRST, THEN YOUR TAIL REGROWS"
            } else if tank_over {
                "REGROWING YOUR TAIL"
            } else {
                "EAT TO OVERFILL YOUR TANK AND YOUR TAIL REGROWS"
            }
        ));
        let (cells, cap) = match (s.crop, def) {
            (Some(c), Some(d)) => (c.cells as i32, (crate::sim::creature::organism_crop_capacity(w, e.raider.unwrap_or(0), d) / c.unit.max(1e-6)).floor() as i32),
            (None, Some(_)) => (0, 0),
            _ => (0, 0),
        };
        let tank = (s.energy / e.start_energy.max(1.0)).clamp(0.0, 1.0);
        let bars = (tank * 10.0).round() as usize;
        let legs = ((1.0 - e.spent) * 10.0).round() as usize;
        out.push(format!(
            "WIND {}{}  {}",
            "*".repeat(legs.min(10)),
            "-".repeat(10 - legs.min(10)),
            if e.winded {
                "WINDED -- WAIT"
            } else if e.sprint_held && e.spent > 0.0 {
                "SPRINTING"
            } else {
                "HOLD SHIFT TO SPRINT"
            }
        ));
        out.push(format!(
            "GUT {}   TANK {}{}{}",
            if cells == 0 { "EMPTY".to_string() } else if cap > 0 { format!("{cells}/{cap} DIGESTING") } else { format!("{cells} DIGESTING") },
            "*".repeat(bars),
            "-".repeat(10 - bars.min(10)),
            if s.spoil.is_some() { "   JAWS: A CRUMB (Q PUTS IT DOWN)" } else { "" }
        ));
    } else if let Some((_, how)) = e.ended {
        out.push(format!("{how} -- R FOR A NEW RAIDER, Y FOR THE CARD"));
    }
    out.push(format!(
        "YOU: {} KILLS, {} BITES   THEY: {} BITES ON YOU, {} CELLS TAKEN",
        e.kills,
        e.bites_given,
        e.bites_taken,
        e.cells_lost.len()
    ));
    let price = if trips >= 200 { format!("   {:.1} EATEN PER 1,000 LOADS", 1000.0 * e.kills as f64 / trips as f64) } else { String::new() };
    out.push(format!("COLONY {ants} ANTS ({brain}), {trips} LOADS HOME SINCE YOU CAME{price}"));
    if e.mutating {
        out.push(format!(
            "SEASON {}   {} YOUNG BORN WITH MUTATED MINDS ({}), {} SINCE YOU CAME",
            e.seasons.len() + 1,
            e.mutants,
            ["GENTLE", "STRONGER", "TRAINING RATE"][usize::from(e.level.clamp(1, 3)) - 1],
            e.mutants - e.mutants_at_landing
        ));
    }
    if let Some((f, why)) = &e.note {
        if w.frame.saturating_sub(*f) < NOTE_FRAMES {
            out.push(why.clone());
        }
    }
    if lab.time.multiple() > super::STEER_CAP {
        out.push(format!("INSTINCT HUNTS (DIAL ABOVE {}X)", super::STEER_CAP));
    }
    out
}

/// **The HUD with the stakes on**: the score first, then the body, the tank
/// against its cap (so an overfull tank reads full and a hungry one reads
/// low), and what is in the jaws. Every line under 72 characters, written
/// for the raider rather than the researcher (seat review, finding 7).
fn stakes_hud(lab: &Lab, e: &EnemyDemo) -> Vec<String> {
    let w = &lab.world;
    let mut out = Vec::new();
    let left = e.raiders_left();
    let lives = format!("{left} OF {LIVES} LEFT");
    out.push(format!(
        "SCORE {}   BEST {}   RAIDERS {lives}{}",
        e.score(),
        e.best,
        if e.in_grace(w.frame) { "   LANDING: SAFE FOR A MOMENT" } else { "" }
    ));
    if let Some(s) = e.raider.filter(|_| e.ended.is_none()).and_then(|id| w.organism(id)) {
        let def = w.species.get(s.species).creature.as_ref();
        let plan = def.map_or(s.chain.len(), |d| match d.body {
            crate::sim::organism::BodyPlan::Chain(n) => usize::from(n),
            _ => s.chain.len(),
        });
        let quarters = (s.gnawed.clamp(0.0, 1.0) * 4.0).floor() as usize;
        let regrow_at = def.map_or(f32::MAX, |d| d.start_energy + d.body_energy);
        out.push(format!(
            "BODY {}/{plan}  WOUND {}{}  {}",
            s.chain.len(),
            "*".repeat(quarters),
            "-".repeat(4 - quarters.min(4)),
            if s.chain.len() >= plan {
                "THE CELL THEY BITE COMES OFF; THE HEAD KILLS"
            } else if s.gnawed > 0.0 {
                "HEAL FIRST, THEN THE TAIL REGROWS"
            } else if s.energy >= regrow_at {
                "REGROWING YOUR TAIL"
            } else {
                "FILL THE TANK PAST THE + TO REGROW"
            }
        ));
        // The tank against the cap, 15 slots; a + at the regrowth line (the HUD font has no bar glyph).
        let cap = CAP_TANKS * e.start_energy.max(1.0);
        let slots = 15usize;
        let filled = ((s.energy / cap).clamp(0.0, 1.0) * slots as f32).round() as usize;
        let mark = ((regrow_at / cap).clamp(0.0, 1.0) * slots as f32).round() as usize;
        let bar: String = (0..slots).map(|i| if i == mark { '+' } else if i < filled { '*' } else { '-' }).collect();
        let tank = s.energy / e.start_energy.max(1.0);
        out.push(format!(
            "TANK {bar} {:>4} J  {}",
            s.energy.max(0.0).round() as i64,
            if tank < 0.15 {
                "STARVING -- EAT NOW"
            } else if tank < 0.4 {
                "HUNGRY"
            } else if s.energy >= cap - 1.0 {
                "FULL"
            } else {
                ""
            }
        ));
        let legs = ((1.0 - e.spent) * 10.0).round() as usize;
        let (cells, gut_cap) = match (s.crop, def) {
            (Some(c), Some(d)) => (c.cells as i32, (crate::sim::creature::organism_crop_capacity(w, e.raider.unwrap_or(0), d) / c.unit.max(1e-6)).floor() as i32),
            _ => (0, 0),
        };
        out.push(format!(
            "WIND {}{}  {}   GUT {}",
            "*".repeat(legs.min(10)),
            "-".repeat(10 - legs.min(10)),
            if e.winded {
                "WINDED"
            } else if e.sprint_held && e.spent > 0.0 {
                "SPRINT"
            } else {
                "SHIFT"
            },
            if cells == 0 { "EMPTY".to_string() } else if gut_cap > 0 { format!("{cells}/{gut_cap}") } else { format!("{cells}") }
        ));
        let larvae = w.live_brood_ids().into_iter().filter(|&b| w.organism(b).and_then(|s| s.brood).is_some_and(|b| b.stage == crate::sim::organism::BroodStage::Larva)).count();
        let jaws = if e.raider.is_some_and(|id| jaws_hold_brood(w, id)) {
            "JAWS: A LARVA -- TO THE GOLD FLAG!".to_string()
        } else if s.spoil.is_some() {
            "JAWS: A CRUMB (Q)".to_string()
        } else if larvae == 0 {
            "NO LARVAE IN THE NEST YET".to_string()
        } else {
            format!("{larvae} LARVAE IN THE NEST (WHITE)")
        };
        out.push(format!("ANTS {} (x1)  BROOD {} (x{BROOD_POINTS})  {jaws}", e.kills, e.brood));
    } else if let Some((_, how)) = e.ended {
        out.push(if e.over { format!("{how} -- GAME OVER -- R FOR A NEW GAME") } else { format!("{how} -- R FOR YOUR NEXT RAIDER") });
    }
    if let Some((f, why)) = &e.note {
        if w.frame.saturating_sub(*f) < NOTE_FRAMES {
            out.push(why.clone());
        }
    }
    if lab.time.multiple() > super::STEER_CAP {
        out.push(format!("INSTINCT HUNTS (DIAL ABOVE {}X)", super::STEER_CAP));
    }
    out
}

/// **The card with the stakes on**: after a death, that raider's line and
/// the game so far; after the third, the game-over card with every life and
/// the score against the best. The verdict on the colony is its head count,
/// never a ratio of kills to loads (the old card said "THEY PAID DEARLY" over
/// a colony that grew from 179 to 191).
fn stakes_card(lab: &Lab, e: &EnemyDemo) -> Vec<String> {
    let w = &lab.world;
    let ants = live_ants(w);
    let mut out = Vec::new();
    let row = |i: usize, l: &Life| {
        let secs = l.frames / 60;
        format!("{:>6}  {:>2}:{:02}  {:>4}  {:>5}  {:>5}  {:>6}  {}", i + 1, secs / 60, secs % 60, l.kills, l.brood, l.deepest.max(0), l.points(), l.how)
    };
    let header = "RAIDER  LIVED  ANTS  BROOD  DEPTH  POINTS  HOW".to_string();
    if e.over {
        out.push(format!("-- GAME OVER: SCORE {}{} (Y CLOSES) --", e.score(), if e.new_best { ", A NEW BEST" } else { "" }));
        out.push(header);
        out.extend(e.game.iter().enumerate().map(|(i, l)| row(i, l)));
        out.push(format!("BEST THIS SESSION {}", e.best));
        let was = e.game_ants_at_start;
        out.push(if ants == 0 {
            "THE COLONY IS GONE.".to_string()
        } else if ants > was {
            format!("THE COLONY GREW FROM {was} TO {ants} ANTS WHILE YOU RAIDED.")
        } else if ants < was {
            format!("THE COLONY SHRANK FROM {was} TO {ants} ANTS. THEY FELT YOU.")
        } else {
            format!("THE COLONY HELD AT {ants} ANTS.")
        });
        out.push("R STARTS A NEW GAME AGAINST THE SAME COLONY.".to_string());
        return out;
    }
    let n = e.game.len() + usize::from(e.ended.is_none());
    match e.ended {
        Some((_, how)) => out.push(format!("-- RAIDER {n} OF {LIVES}: {how} (Y CLOSES) --")),
        None => out.push(format!("-- RAIDER {n} OF {LIVES}, STILL HUNTING (Y CLOSES) --")),
    }
    out.push(header);
    out.extend(e.game.iter().enumerate().map(|(i, l)| row(i, l)));
    if e.ended.is_none() {
        let now = Life {
            frames: w.frame.saturating_sub(e.landed.unwrap_or(w.frame)),
            kills: e.kills,
            grabbed: e.grabbed,
            brood: e.brood,
            deepest: e.deepest,
            how: "HUNTING",
        };
        out.push(row(e.game.len(), &now));
    }
    out.push(format!("SCORE SO FAR {}   BEST {}   RAIDERS LEFT {}", e.score(), e.best, e.raiders_left()));
    out.push(format!("COLONY {} ANTS (WAS {} WHEN THE GAME BEGAN)", ants, e.game_ants_at_start));
    if e.ended.is_some() {
        out.push("R SETS YOUR NEXT RAIDER DOWN.".to_string());
    }
    out
}

/// **The season card**: what you did and what it cost them, from the counts.
pub fn card_lines(lab: &Lab) -> Vec<String> {
    let Some(e) = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()) else { return Vec::new() };
    let Some(landed) = e.landed else { return Vec::new() };
    if e.stakes {
        return stakes_card(lab, e);
    }
    let w = &lab.world;
    let until = e.ended.map_or(w.frame, |(f, _)| f);
    let secs = until.saturating_sub(landed) / 60;
    let trips = w.creature_stats.trip_deliveries.saturating_sub(e.trips_at_landing);
    let ants = live_ants(w);
    let lost: Vec<String> = e.cells_lost.iter().map(|&(_, i)| if i == 0 { "HEAD".to_string() } else { format!("CELL {}", i + 1) }).collect();
    let verdict = if ants == 0 {
        "THE COLONY IS GONE.".to_string()
    } else if e.kills == 0 {
        "THEY NEVER PAID YOU.".to_string()
    } else if trips >= 200 {
        let per = trips as f64 / e.kills as f64;
        if ants >= e.ants_at_landing {
            format!("THE COLONY GREW ({} TO {ants}): YOU ATE 1 FOR EVERY {per:.0} LOADS.", e.ants_at_landing)
        } else if per > 100.0 {
            format!("THEY KEPT FORAGING: YOU ATE 1 FOR EVERY {per:.0} LOADS THEY BROUGHT HOME.")
        } else {
            format!("THEY PAID DEARLY: 1 ANT FOR EVERY {per:.0} LOADS.")
        }
    } else {
        format!("{} KILLS, AND THE COLONY HAS BROUGHT HOME ONLY {trips} LOADS SINCE YOU CAME.", e.kills)
    };
    [
        "-- THE SEASON CARD (Y CLOSES) --".to_string(),
        format!("HUNTED {}:{:02}   {}", secs / 60, secs % 60, e.ended.map_or("STILL HUNTING", |(_, h)| h)),
        format!("KILLS {}   BITES GIVEN {}   BITES TAKEN {}", e.kills, e.bites_given, e.bites_taken),
        format!("CELLS LOST {}{}", e.cells_lost.len(), if lost.is_empty() { String::new() } else { format!(" ({})", lost.join(", ")) }),
        format!("ANTS {} WHEN YOU CAME, {} NOW   LOADS HOME WHILE YOU HUNTED {}", e.ants_at_landing, ants, trips),
        verdict,
    ]
    .into_iter()
    .chain(season_lines(e))
    .chain(if e.mutating {
        vec!["ITS YOUNG ARE BORN WITH MUTATED COPIES OF THEIR PARENTS' MINDS:".to_string(), "THE ANTS YOU EAT LEAVE NO YOUNG. WATCH THE BITE RATE, SEASON BY SEASON.".to_string()]
    } else if lab.new_ant {
        vec!["THIS COLONY FIGHTS BACK BUT DOES NOT LEARN YOU:".to_string(), "ITS YOUNG ARE BORN WITH THEIR PARENTS' MINDS.".to_string()]
    } else {
        Vec::new()
    })
    .collect()
}

/// **The seasons so far**, newest four: how long each raider lived, what it
/// took, and how hard the colony bit back per minute it was in reach. The
/// rate, not the count: a raider that hides is bitten less without the
/// colony having changed at all.
fn season_lines(e: &EnemyDemo) -> Vec<String> {
    if e.seasons.is_empty() {
        return Vec::new();
    }
    let first = e.seasons.len().saturating_sub(4);
    std::iter::once("SEASON  LIVED  KILLS  BITTEN  PER MINUTE IN REACH".to_string())
        .chain(e.seasons.iter().enumerate().skip(first).map(|(i, s)| {
            let secs = s.frames / 60;
            format!("{:>6}  {:>2}:{:02}  {:>5}  {:>6}  {:>18.1}", i + 1, secs / 60, secs % 60, s.kills, s.bites_taken, s.bite_rate())
        }))
        .collect()
}

/// The seat's panel: the HUD under the clock, and the card when it is up.
pub fn draw_hud(lab: &Lab, hc: crate::render::Hud, frame_buf: &mut [u8]) {
    let Some(e) = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()) else { return };
    let white = [235u8, 235, 235, 255];
    let warn = [255u8, 200, 90, 255];
    let note = e.note.as_ref().map(|(_, n)| n.as_str());
    for (i, line) in hud_lines(lab).iter().enumerate() {
        let colour = if Some(line.as_str()) == note || line.starts_with("INSTINCT") { warn } else { white };
        hc.text(frame_buf, 4, 16 + 10 * i as i32, line, colour);
    }
    if e.card {
        let lines = card_lines(lab);
        let y0 = 90;
        let wpx = lines.iter().map(|l| crate::hud::text_width(l)).max().unwrap_or(0) + 12;
        hc.fill_rect(frame_buf, 8, y0 - 6, wpx, 10 * lines.len() as i32 + 10, [10, 10, 14, 230]);
        for (i, line) in lines.iter().enumerate() {
            hc.text(frame_buf, 14, y0 + 10 * i as i32, line, if i == 0 { warn } else { white });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::demo::{Demo, DemoKey, DemoSeat};
    use crate::lab::scenario::{Placement, Scenario};

    fn grid_hash(w: &World) -> u64 {
        fn fnv1a(h: u64, v: u64) -> u64 {
            (h ^ v).wrapping_mul(0x0000_0100_0000_01b3)
        }
        let b = w.bounds().expect("the lab bed sets bounds");
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                let c = w.get(x, y);
                h = fnv1a(h, c.material.0 as u64);
                h = fnv1a(h, c.aux() as u64);
                h = fnv1a(h, c.organism_id() as u64);
            }
        }
        fnv1a(h, w.live_organism_count() as u64)
    }

    /// The enemy box with its colony landing at 200 rather than 6,000, so a
    /// test reaches the hunt in hundreds of frames.
    fn quick_lab(warm: &str, mutate: bool) -> Lab {
        let mut sc = Scenario::load("demo_enemy").expect("demo_enemy ships");
        for e in &mut sc.timeline {
            if matches!(e.what, Placement::Colony { .. } | Placement::Heap { .. }) {
                e.at = 200;
            }
        }
        let mut lab = Lab::new(sc.bed.clone());
        lab.set_new_ant(true);
        let mut demo = Demo::new(DemoSeat::Enemy);
        demo.args.insert("warm".to_string(), warm.to_string());
        if !mutate {
            demo.args.insert("mutate".to_string(), "0".to_string());
        }
        lab.demo = Some(demo);
        lab.load_scenario(sc);
        lab
    }

    /// **Off is off, and with `mutate=0` the warm-up is the plain box**:
    /// until the raider lands the seat only turns the speed dial, so the box
    /// under it is the box with no demo, cell for cell, over 3,000 frames.
    /// (With the birth mutation on, the demo's default since the owner's pick,
    /// the colony is meant to differ from its first egg.)
    #[test]
    fn the_box_is_untouched_until_the_raider_lands() {
        let mut with = quick_lab("1000000000", false);
        let mut without = quick_lab("0", false);
        without.demo = None;
        for _ in 0..3000 {
            with.demo_frame();
            with.tick_for_harness();
            without.tick_for_harness();
        }
        assert!(with.demo.as_ref().and_then(|d| d.enemy.as_ref()).is_some_and(|e| e.raider.is_none()), "landed during a warm-up that never ends");
        assert_eq!(grid_hash(&with.world), grid_hash(&without.world), "the warm-up changed the box");
    }

    /// **The demo end to end**: after the warm-up a centipede lands, is the
    /// player's body with no young, the camera closes in, and a bite or a
    /// wound reaches the HUD's counts. The positive control for "the keys
    /// reach it" is the head moving the way the arrow is held.
    #[test]
    fn a_raider_lands_after_the_warm_up_and_obeys_the_arrows() {
        let mut lab = quick_lab("600", true);
        let zoom0 = lab.renderer.zoom;
        for _ in 0..700 {
            lab.demo_frame();
            lab.tick_for_harness();
        }
        let id = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).and_then(|e| e.raider).expect("no raider landed after the warm-up");
        let centipede = lab.world.species.id_of("centipede").expect("centipede ships");
        assert_eq!(lab.world.organism(id).map(|s| s.species), Some(centipede));
        assert_eq!(steer::steered_id(&lab.world), Some(id), "the raider is not the player's body");
        let s = lab.world.steered.as_ref().expect("marked");
        assert!(!s.may_breed, "the player's raider may bud");
        assert_eq!(s.role, steer::SteerRole::Worker);
        assert!(lab.renderer.zoom > zoom0, "the camera did not close in on the landing");
        assert!(!hud_lines(&lab).is_empty());
        let x0 = lab.world.organism(id).and_then(|s| s.chain.first().copied()).expect("alive").0;
        let door = lab.world.nest_sites.iter().map(|s| s.x).min_by_key(|d| (d - x0).abs()).expect("a door");
        // Away from the door, so the walk is not into the mound's jaws.
        let key = if x0 < door { DemoKey::Left } else { DemoKey::Right };
        assert!(lab.demo_key(key, true));
        for _ in 0..200 {
            lab.demo_frame();
            lab.tick_for_harness();
        }
        let x1 = lab.world.organism(id).and_then(|s| s.chain.first().copied()).map(|h| h.0);
        if let Some(x1) = x1 {
            assert!((x1 - x0).signum() == (x0 - door).signum() && x1 != x0, "held {key:?} for 200 frames and the head went {x0} -> {x1}");
        }
    }

    fn raider_of(lab: &Lab) -> OrganismId {
        lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).and_then(|e| e.raider).expect("a raider landed")
    }

    fn step(lab: &mut Lab, n: usize) {
        for _ in 0..n {
            lab.demo_frame();
            lab.tick_for_harness();
        }
    }

    fn seat(lab: &Lab) -> &EnemyDemo {
        lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).expect("enemy seat")
    }

    /// **The stakes end to end** (seat review 2026-10-30, E2 and E1): on by
    /// default; the landing's grace holds the wound meter at zero; the tank
    /// is capped and drains with no food; a larva in the jaws carried to the
    /// lair is put down there and scores ten; the mouse is the seat's; three
    /// deaths end the game with the score as the best, and R starts a new
    /// game with three raiders. `stakes=0` is the sandbox: no cap, no lives.
    #[test]
    fn the_stakes_drain_cap_bank_and_end_the_game() {
        let mut lab = quick_lab("600", false);
        step(&mut lab, 620);
        let id = raider_of(&lab);
        assert!(seat(&lab).stakes, "the stakes are not on by default");
        assert!(seat(&lab).in_grace(lab.world.frame), "no grace at the landing");
        lab.world.organism_mut(id).expect("alive").gnawed = 0.9;
        step(&mut lab, 1);
        assert_eq!(lab.world.organism(id).map(|s| s.gnawed), Some(0.0), "a wound stood during the grace");
        // The cap.
        lab.world.organism_mut(id).expect("alive").energy = 9_000.0;
        step(&mut lab, 1);
        let cap = CAP_TANKS * 1600.0;
        assert!(lab.world.organism(id).expect("alive").energy <= cap + 1e-3, "the tank went over its cap");
        assert!(seat(&lab).capped_j > 6_000.0);
        // The hunger.
        lab.world.organism_mut(id).expect("alive").energy = 1_000.0;
        let j0 = seat(&lab).hunger_j;
        step(&mut lab, 100);
        assert!(seat(&lab).hunger_j - j0 >= 100.0 * f64::from(HUNGER) - 1e-3, "the tank did not drain");
        // The bank: a larva's corpse in the jaws, the head at the lair.
        let corpse = lab.world.materials.known(crate::sim::material::Known::Corpse).expect("corpse ships");
        let lair = seat(&lab).lair.expect("a lair");
        let head = lab.world.organism(id).and_then(|s| s.chain.first().copied()).expect("alive");
        assert!((head.0 - lair.0).abs() <= LAIR_REACH, "test setup: the raider walked off its lair ({head:?} vs {lair:?})");
        lab.world.organism_mut(id).expect("alive").spoil = Some(crate::sim::organism::Spoil { cell: crate::sim::cell::Cell::new(corpse, 0).with_aux(50), store: false });
        let score0 = seat(&lab).score();
        step(&mut lab, 1);
        assert_eq!(seat(&lab).brood, 1, "the larva was not banked at the lair");
        assert_eq!(seat(&lab).score(), score0 + BROOD_POINTS);
        assert!(lab.world.organism(id).is_some_and(|s| s.spoil.is_none()), "the jaws still hold the larva");
        // The pointer is the seat's: the lab's CULL cannot reach the box.
        assert!(lab.demo_mouse((10, 10), super::super::MouseAct::Press { erase: false }), "the lab's tools still act in a raid");
        // Three deaths and the game is over.
        for life in 0..LIVES {
            let id = raider_of(&lab);
            lab.world.organism_mut(id).expect("alive").energy = -1.0e6;
            for _ in 0..40 {
                step(&mut lab, 1);
                if lab.world.organism(id).is_none() {
                    break;
                }
            }
            assert!(seat(&lab).ended.is_some(), "raider {life} did not die");
            assert_eq!(seat(&lab).game.len(), life + 1);
            if life + 1 < LIVES {
                assert!(!seat(&lab).over, "the game ended with raiders left");
                lab.demo_key(DemoKey::Letter('R'), true);
                assert!(seat(&lab).ended.is_none(), "R did not land the next raider");
            }
        }
        let e = seat(&lab);
        assert!(e.over && e.best == e.score() && e.best >= BROOD_POINTS, "game over: over {} best {} score {}", e.over, e.best, e.score());
        assert!(card_lines(&lab)[0].contains("GAME OVER"), "{:?}", card_lines(&lab));
        lab.demo_key(DemoKey::Letter('R'), true);
        assert!(seat(&lab).game.is_empty() && !seat(&lab).over && seat(&lab).raider.is_some(), "R after the game did not start a new one");
        // The sandbox.
        let mut sand = quick_lab("600", false);
        sand.demo.as_mut().expect("demo").args.insert("stakes".to_string(), "0".to_string());
        step(&mut sand, 620);
        let id = raider_of(&sand);
        assert!(!seat(&sand).stakes);
        sand.world.organism_mut(id).expect("alive").energy = 9_000.0;
        step(&mut sand, 1);
        assert!(sand.world.organism(id).expect("alive").energy > cap, "stakes=0 still caps the tank");
        assert!(!sand.demo_mouse((10, 10), super::super::MouseAct::Press { erase: false }), "stakes=0 kept the pointer");
    }

    /// **A raider bitten down to its head regrows** (`creature::regrow_tail`,
    /// whose `len < 2` guard stopped it; seat review 2026-10-30): cut to one
    /// cell, fed past the regrowth bar and left alone, it grows back, and the
    /// first new cell is a segment, not a second head. Watched red with the
    /// guard put back.
    #[test]
    fn a_raider_bitten_to_its_head_regrows() {
        let mut lab = quick_lab("600", false);
        lab.demo.as_mut().expect("demo").args.insert("stakes".to_string(), "0".to_string());
        step(&mut lab, 620);
        let id = raider_of(&lab);
        let tail: Vec<(i32, i32)> = lab.world.organism(id).expect("alive").chain[1..].to_vec();
        for (x, y) in tail {
            lab.world.set(x, y, crate::sim::cell::Cell::EMPTY);
        }
        step(&mut lab, 12);
        assert_eq!(lab.world.organism(id).map(|s| s.chain.len()), Some(1), "test setup: not cut to the head");
        let grown0 = lab.world.creature_stats.regrown_cells;
        for _ in 0..2_000 {
            if let Some(s) = lab.world.organism_mut(id) {
                s.energy = 3_000.0;
                s.gnawed = 0.0;
            }
            step(&mut lab, 1);
        }
        let s = lab.world.organism(id).expect("the raider died while regrowing");
        assert!(s.chain.len() >= 2 && lab.world.creature_stats.regrown_cells > grown0, "a head alone never regrew: {} cells", s.chain.len());
        let (x, y) = s.chain[1];
        assert_eq!(crate::sim::organism::cell_type(lab.world.get(x, y).aux()), Some(crate::sim::organism::CellType::Segment), "the regrown cell is not a segment");
    }

    /// **The birth mutation reaches the colony's eggs**: with it on, eggs are
    /// laid with minds the founders never had (each a mutated copy, counted
    /// by the seat), and with `mutate=0` none is, and no store exists. The
    /// positive control for "a mind differs" is the store's own flat genome
    /// for each newborn, compared against all thirty founders.
    #[test]
    fn eggs_take_mutated_minds_only_with_mutation_on() {
        let run = |mutate: bool| -> (u32, usize, bool) {
            let mut lab = quick_lab("1000000000", mutate);
            let founders = crate::lab::training::std_pool();
            for _ in 0..4000 {
                lab.demo_frame();
                lab.tick_for_harness();
            }
            // Every living ant's (and egg's) flat genome, as the store rebuilt it.
            let novel = lab.world.fathers.as_ref().map_or(0, |f| {
                lab.world
                    .live_organism_ids()
                    .into_iter()
                    .filter_map(|id| crate::sim::creature::mind::genome_of(&lab.world, id))
                    .filter_map(|m| f.genome_of(&m))
                    .filter(|g| !founders.iter().any(|h| h[..] == g[..]))
                    .count()
            });
            let e = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).expect("enemy seat");
            (e.mutants, novel, lab.world.fathers.is_some())
        };
        let (on, novel, store) = run(true);
        assert!(store, "mutation on and no fathers store");
        assert!(on > 0, "no egg took a mutated mind in 4,000 frames with mutation on");
        assert!(novel > 0, "the eggs counted as mutants carry founders' genomes");
        let (off, _, store) = run(false);
        assert_eq!(off, 0);
        assert!(!store, "mutate=0 and a fathers store exists");
    }
}
