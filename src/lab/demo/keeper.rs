//! **The keeper seat's demo** (`demo=keeper`): the player is a hand over the
//! formicarium, not an ant. The design is
//! `Reports/teaching-the-colony-2026-10-25/keeper.md`; the plan this file is
//! built to is the keeper thread's `plan-v2` (landing with
//! `Reports/keeper-seat-demo-2026-10-25.md`).
//!
//! **What the box holds** (`assets/lab_scenarios/demo_keeper.ron` plus this
//! file):
//!
//! - the colony at the west end, landed at the first tick, with a founding
//!   heap under it;
//! - a **far station** of provisions near the east end, kept topped up so it
//!   never runs dry ([`STATION_LEVEL`]), the place the player teaches;
//! - a **ration inside the nest** ([`KeeperDemo::ration`] cells per 1,000
//!   frames, onto dug home cells): enough that the colony is not starved out
//!   before the player has done anything, below what it eats, so foraging
//!   pressure stays. A pick-up at home never writes `last_food`
//!   (`mind::pick_up`), so the ration cannot overwrite a lesson.
//!
//! **The recall head** (`creature::recall`) is set on the world here, after
//! every rebuild, at the demo's gain and copy; [`KeeperDemo::listening`]
//! switches the gain to 0, the honest arm, without touching anything else.
//!
//! The counters are the plan's: station bites, station deliveries (a cell
//! taken at the station and put down at home, `trip_deliveries` -- never
//! `MindCounts::delivered`, how-we-test §4), the frame of the first one, and
//! how many ants know the station.
//!
//! **The player's verbs** are the seat keys (`Y U P J`, the launcher's), so
//! nothing in the lab's own bar or key map moves:
//!
//! - `Y` **CARRY**: press on an ant to pick it up, release to set it down
//!   (`creature::set_down`). Set down within [`LESSON_R`] of food that is not
//!   at home, it remembers that food as a lesson (`recall::plant`);
//! - `U` **TREAT**: click an ant to put a mouthful of provisions in its crop
//!   (`creature::treat`, booked as granted food). Away from home it also
//!   remembers the place it was treated;
//! - `P` **LISTENING**: the scripted gain on and off. Off is today's ant, and
//!   the HUD names which arm is live;
//! - `J` **OVERLAY**: off, who knows the station (a ring per taught or fed
//!   ant, by generation), or traffic (where heads have been lately).
//!
//! **What the measurement says this can and cannot show** (build step 4,
//! `examples/keeperdemo.rs`, 12 paired seeds, the 24,000 frames after the
//! demo opens, the pool of PR #906):
//!
//! - **Carrying is a real verb on today's ant, graded by how many you
//!   carry.** Listening *off*, against nobody carried: station visits +52
//!   (10 carried, 10 of 12 seeds up) and +105 (~17 carried, 10 of 12), food
//!   home +15 (9 of 12), and a bigger colony (median 120 ants, 146, 168).
//!   Three carried ants are inside the noise. A carried ant takes food at the
//!   station, walks it home, and walks back on its own trail.
//! - **The scripted listening adds to that**, unevenly: visits +36 (3
//!   carried, 9 of 12 up), +12 (10, 6 of 12), +89 (~17, 9 of 12); food home
//!   +4 to +14. No loss in digging or survival in any arm.
//! - **The meal copy moves nothing measurable**: copy 0 and copy 0.5 gave
//!   the same traffic, and the share that "knows" never rises past the ants
//!   taught by hand, because a fed place is weak and the next stray bite puts
//!   it to sleep. It stays on (neutral), counted on the HUD.
//!
//! An earlier pass read "carrying alone changes nothing": its scripted hand
//! asked for 10 ants and lifted about 6, because the nearest ants to the nest
//! were stacked on one another and [`creature::set_down`] refuses a stack.
//! The hand now picks the nearest ant it can lift (`creature::can_carry`).

use std::collections::BTreeMap;

use crate::sim::cell::{Cell, OrganismId};
use crate::sim::creature::{self, recall};
use crate::sim::material;
use crate::sim::organism::DeathCause;
use crate::sim::world::World;

use super::super::Lab;

/// The keeper seat's own keys, as the help line prints them.
pub const SEAT_KEYS: &str = "Y CARRY, U TREAT, P LISTENING, J OVERLAY";

/// Food within this many cells of where an ant is set down (or treated) is
/// what it remembers. The station heap spreads ~10 columns either side.
pub const LESSON_R: i32 = 12;
/// A press picks the ant whose head is nearest, within this many cells.
const PICK_R: i32 = 6;
/// Frames the box runs before the player gets it (`warmup=`): the landing's
/// die-off (about 30 of 52 founders by frame 3,000, on every landing tried,
/// `nest_goal`'s own included) happens before the demo opens, and the nest
/// has a first chamber for the ration.
pub const DEFAULT_WARMUP: u64 = 6000;
/// The traffic overlay's memory: its counts halve every this many frames.
const HEAT_HALF: u64 = 2000;

/// Provisions the station is topped back up to, in cells.
pub const STATION_LEVEL: usize = 80;
/// Columns either side of the station counted as "at the station", for
/// bites and for "knows".
pub const STATION_R: i32 = 20;
/// "Knows the station" is strength x freshness at or above this (plan §5).
pub const KNOWS_PHI: f32 = 0.25;
/// Frames between the counters' scans: one mind decision interval.
const SCAN_EVERY: u64 = 6;
/// Frames between station top-ups.
const TOP_UP_EVERY: u64 = 60;

/// **Gain 6, copy 0.5**, from build step 4. Gain: the dose's knee on the unit
/// box (furthest columns toward a spot 60 out in 1,500 frames: g4 37, g6
/// 124, g8 113), and on the demo box 4, 6 and 8 were alike within the seed
/// spread with 6 the best median. Copy: 0, 0.25, 0.5 and 0.75 gave the same
/// station traffic (10 ants carried, 12 seeds); 0.5 is kept, on by default
/// as neutral, so the HUD has copies to count.
pub const DEFAULT_GAIN: f32 = 6.0;
pub const DEFAULT_COPY: f32 = 0.5;

/// Which verb a press over the world is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Tool {
    /// The lab's own tool takes the pointer.
    #[default]
    None,
    Carry,
    Treat,
}

/// What is drawn over the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Overlay {
    #[default]
    Off,
    Knows,
    Traffic,
}

/// What the demo has counted since the box was built.
#[derive(Clone, Copy, Debug, Default)]
pub struct KeeperCounts {
    /// Station food cells gone between scans (bites, and any slump off the
    /// counted columns, which the wide count keeps small).
    pub station_bites: u64,
    /// Cells taken at the station and put down at home.
    pub station_deliveries: u64,
    pub first_station_delivery: Option<u64>,
    /// Provisions cells put into the nest as the ration.
    pub ration_given: u64,
    /// Station cells added by the top-up.
    pub station_given: u64,
    pub carried: u64,
    pub treated: u64,
    /// An ant's head coming into the station's columns
    /// from outside them: a visit. `taught_arrivals` are the visits by ants
    /// the player taught or that were fed the place (a provenance record,
    /// awake or dormant).
    pub arrivals: u64,
    pub taught_arrivals: u64,
}

/// The keeper seat's running state.
#[derive(Clone, Debug)]
pub struct KeeperDemo {
    pub gain: f32,
    pub copy: f32,
    /// False: gain 0, today's ants (the honest arm).
    pub listening: bool,
    /// The station's column.
    pub station_x: i32,
    /// Ration cells a 1,000 frames, onto home cells in the nest.
    pub ration: u32,
    pub counts: KeeperCounts,
    /// Ants seen taking food at the station, with their `trip_deliveries` at
    /// that moment: a rise after it is a station delivery.
    haul: BTreeMap<OrganismId, u32>,
    /// Station cells at the end of the last scan (after any top-up).
    station_cells: usize,
    last_scan: u64,
    /// Ration cells placed and not yet seen eaten.
    ration_cells: Vec<(i32, i32)>,
    /// Ants whose head was at the station at the last scan.
    at_station: std::collections::BTreeSet<OrganismId>,
    pub tool: Tool,
    pub overlay: Overlay,
    /// The ant in the hand (CARRY, between press and release).
    pub held: Option<OrganismId>,
    /// Frames to run before the player gets the box.
    pub warmup: u64,
    /// Ant-head counts per cell, halved every [`HEAT_HALF`] frames: the
    /// traffic overlay. Empty until the overlay is first shown.
    heat: Vec<u16>,
}

impl Default for KeeperDemo {
    fn default() -> Self {
        Self {
            gain: DEFAULT_GAIN,
            copy: DEFAULT_COPY,
            listening: true,
            station_x: 390,
            ration: 3,
            counts: KeeperCounts::default(),
            haul: BTreeMap::new(),
            station_cells: 0,
            last_scan: 0,
            ration_cells: Vec::new(),
            at_station: Default::default(),
            tool: Tool::None,
            overlay: Overlay::Off,
            held: None,
            warmup: DEFAULT_WARMUP,
            heat: Vec::new(),
        }
    }
}

impl KeeperDemo {
    /// The demo's settings from the command line: `gain=`, `copy=`,
    /// `station=`, `ration=`, `listening=0`.
    pub fn from_args(args: &BTreeMap<String, String>) -> Self {
        let mut k = Self::default();
        let num = |key: &str| args.get(key).map(|v| v.parse::<f32>().unwrap_or_else(|_| panic!("demo=keeper: {key}={v:?} is not a number")));
        if let Some(g) = num("gain") {
            k.gain = g;
        }
        if let Some(c) = num("copy") {
            k.copy = c;
        }
        if let Some(x) = num("station") {
            k.station_x = x as i32;
        }
        if let Some(r) = num("ration") {
            k.ration = r as u32;
        }
        if let Some(l) = num("listening") {
            k.listening = l != 0.0;
        }
        if let Some(w) = num("warmup") {
            k.warmup = w as u64;
        }
        k
    }

    /// The gain the head runs at now.
    pub fn live_gain(&self) -> f32 {
        if self.listening {
            self.gain
        } else {
            0.0
        }
    }
}

/// The station as a place an ant can remember: its column at the surface.
pub fn station_place(world: &World, k: &KeeperDemo) -> (i32, i32) {
    (k.station_x, surface_under(world, k.station_x))
}

/// The ground (or the top of the heap) in column `x`, looked for from under
/// the lid: the first non-empty row from the top is the box's own ceiling.
fn surface_under(world: &World, x: i32) -> i32 {
    let top = world.enclosure().map_or(0, |e| e.ceiling_y + 1);
    creature::colony_surface(world, x, top).unwrap_or(super::super::HEIGHT as i32 / 2)
}

/// Provisions cells in the station's counted columns.
fn station_food(world: &World, k: &KeeperDemo) -> usize {
    let Some(food) = world.materials.id_of("provisions") else { return 0 };
    let (w, h) = (super::super::WIDTH as i32, super::super::HEIGHT as i32);
    let mut n = 0;
    for x in (k.station_x - STATION_R).max(0)..=(k.station_x + STATION_R).min(w - 1) {
        for y in 0..h {
            if world.get(x, y).material == food {
                n += 1;
            }
        }
    }
    n
}

fn keeper(lab: &Lab) -> Option<&KeeperDemo> {
    lab.demo.as_ref()?.keeper.as_ref()
}

fn keeper_mut(lab: &mut Lab) -> Option<&mut KeeperDemo> {
    lab.demo.as_mut()?.keeper.as_mut()
}

/// **Put the recall head on the world** if it is missing (a fresh box, or a
/// rebuild: `Lab::reset` builds a new `World`), and keep its gain at the
/// demo's live gain.
fn ensure_recall(lab: &mut Lab) {
    let Some((gain, copy)) = keeper(lab).map(|k| (k.live_gain(), k.copy)) else { return };
    match lab.world.recall.as_mut() {
        Some(r) => {
            r.gain = gain;
            r.copy = copy;
        }
        None => lab.world.recall = Some(recall::Recall::new(gain, copy)),
    }
}

/// **After the demo's scenario loads**: rebuild on `seed=` if one was asked
/// for, then run the warm-up with nobody watching.
pub fn on_start(lab: &mut Lab) {
    let seed = lab.demo.as_ref().and_then(|d| d.args.get("seed")).map(|v| v.parse::<u64>().unwrap_or_else(|_| panic!("demo=keeper: seed={v:?}")));
    if let (Some(seed), Some(mut sc)) = (seed, lab.scenario.clone()) {
        sc.bed.seed = seed;
        lab.load_scenario(sc);
    }
    let warmup = keeper(lab).map_or(0, |k| k.warmup);
    for _ in 0..warmup {
        lab.tick_for_harness();
    }
}

/// One of the seat keys (`Y U P J K R`) was pressed.
pub fn seat_key(lab: &mut Lab, key: char) {
    let Some(k) = keeper_mut(lab) else { return };
    let msg = match key {
        'Y' => {
            k.tool = if k.tool == Tool::Carry { Tool::None } else { Tool::Carry };
            k.held = None;
            if k.tool == Tool::Carry { "CARRY: PRESS ON AN ANT, RELEASE WHERE IT GOES -- NEAR FOOD IT REMEMBERS THE PLACE" } else { "CARRY OFF" }.to_string()
        }
        'U' => {
            k.tool = if k.tool == Tool::Treat { Tool::None } else { Tool::Treat };
            k.held = None;
            if k.tool == Tool::Treat { "TREAT: CLICK AN ANT TO FEED IT A MOUTHFUL -- AWAY FROM HOME IT REMEMBERS WHERE" } else { "TREAT OFF" }.to_string()
        }
        'P' => {
            k.listening = !k.listening;
            listening_line(k)
        }
        'J' => {
            k.overlay = match k.overlay {
                Overlay::Off => Overlay::Knows,
                Overlay::Knows => Overlay::Traffic,
                Overlay::Traffic => Overlay::Off,
            };
            match k.overlay {
                Overlay::Off => "OVERLAY OFF",
                Overlay::Knows => "OVERLAY: WHO KNOWS A TAUGHT PLACE -- WHITE BY YOUR HAND, AMBER FED ONCE, RED FED TWICE OR MORE, GREY ASLEEP",
                Overlay::Traffic => "OVERLAY: TRAFFIC -- WHERE HEADS HAVE BEEN IN THE LAST FEW THOUSAND FRAMES",
            }
            .to_string()
        }
        _ => return,
    };
    ensure_recall(lab);
    lab.ui.say(msg);
}

fn listening_line(k: &KeeperDemo) -> String {
    if k.listening {
        format!("LISTENING: SCRIPTED GAIN {} (TAUGHT ANTS LEAN TOWARD WHAT THEY REMEMBER)", k.gain)
    } else {
        "LISTENING: OFF (TODAY'S ANTS)".to_string()
    }
}

/// Once a displayed frame.
pub fn frame(_lab: &mut Lab) {}

/// Before each simulated tick.
pub fn before_tick(lab: &mut Lab) {
    ensure_recall(lab);
}

/// After each simulated tick: the counters, the station top-up, the ration.
pub fn after_tick(lab: &mut Lab) {
    let frame = lab.world.frame;
    let Some(mut k) = lab.demo.as_mut().and_then(|d| d.keeper.take()) else { return };
    // **A rebuilt box starts the counts again** (`Lab::reset` makes a new
    // `World` whose frame restarts): yesterday's ids mean other ants now.
    if frame < k.last_scan {
        k = KeeperDemo { gain: k.gain, copy: k.copy, listening: k.listening, station_x: k.station_x, ration: k.ration, tool: k.tool, overlay: k.overlay, warmup: k.warmup, ..KeeperDemo::default() };
    }
    if frame.is_multiple_of(SCAN_EVERY) {
        scan(&mut lab.world, &mut k, frame);
    }
    if frame.is_multiple_of(TOP_UP_EVERY) {
        top_up_station(lab, &mut k);
    }
    if frame.is_multiple_of(1000) && k.ration > 0 {
        give_ration(&mut lab.world, &mut k);
    }
    if frame.is_multiple_of(600) {
        recall::prune(&mut lab.world);
    }
    if !k.heat.is_empty() && frame.is_multiple_of(HEAT_HALF) {
        for h in &mut k.heat {
            *h /= 2;
        }
    }
    if let Some(d) = lab.demo.as_mut() {
        d.keeper = Some(k);
    }
}

/// The counters' scan: station bites since the last scan, ants seen taking
/// food at the station, and the station deliveries they then make.
fn scan(world: &mut World, k: &mut KeeperDemo, frame: u64) {
    let now = station_food(world, k);
    k.counts.station_bites += k.station_cells.saturating_sub(now) as u64;
    k.station_cells = now;
    let Some(ant) = world.species.id_of("ant") else { return };
    let since = k.last_scan;
    k.last_scan = frame;
    let (w, h) = (super::super::WIDTH as i32, super::super::HEIGHT as i32);
    if k.overlay == Overlay::Traffic && k.heat.is_empty() {
        k.heat = vec![0; (w * h) as usize];
    }
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        if st.species != ant {
            continue;
        }
        if let (false, Some(&(x, y))) = (k.heat.is_empty(), st.chain.first()) {
            if (0..w).contains(&x) && (0..h).contains(&y) {
                let c = &mut k.heat[(y * w + x) as usize];
                *c = c.saturating_add(1);
            }
        }
        let trips = st.life.trip_deliveries;
        let here = st.chain.first().is_some_and(|h| (h.0 - k.station_x).abs() <= STATION_R);
        if here && k.at_station.insert(id) {
            k.counts.arrivals += 1;
            if world.recall.as_ref().is_some_and(|r| r.provenance.contains_key(&id)) {
                k.counts.taught_arrivals += 1;
            }
        } else if !here {
            k.at_station.remove(&id);
        }
        let at_station = st.last_food.is_some_and(|(x, _, _)| (x - k.station_x).abs() <= STATION_R);
        let fresh = st.last_food.is_some_and(|(_, _, f)| f > since || since == 0);
        if let Some(&before) = k.haul.get(&id) {
            if trips > before {
                k.counts.station_deliveries += u64::from(trips - before);
                k.counts.first_station_delivery.get_or_insert(frame);
                k.haul.remove(&id);
            } else if !at_station {
                k.haul.remove(&id);
            }
        } else if at_station && fresh && st.crop.is_some() {
            k.haul.insert(id, trips);
        }
    }
    k.haul.retain(|&id, _| world.organism(id).is_some());
    k.at_station.retain(|&id| world.organism(id).is_some());
}

/// Top the station back up to [`STATION_LEVEL`].
fn top_up_station(lab: &mut Lab, k: &mut KeeperDemo) {
    let have = station_food(&lab.world, k);
    if have >= STATION_LEVEL {
        return;
    }
    let want = (STATION_LEVEL - have) as i32;
    let placed = super::super::scenario::apply_placements(
        &mut lab.world,
        &[super::super::scenario::Placement::Heap { material: "provisions".into(), x: k.station_x, cells: want }],
        &lab.spec,
    );
    k.counts.station_given += placed.cells as u64;
    k.station_cells = station_food(&lab.world, k);
}

/// **The ration**: up to `ration` provisions cells onto empty home cells
/// with ground under them, deepest first (inside the chamber, not on the
/// mound), never more than four rations standing. No home dug yet: nothing.
fn give_ration(world: &mut World, k: &mut KeeperDemo) {
    let Some(food) = world.materials.id_of("provisions") else { return };
    k.ration_cells.retain(|&(x, y)| world.get(x, y).material == food);
    let room = (4 * k.ration as usize).saturating_sub(k.ration_cells.len()).min(k.ration as usize);
    if room == 0 {
        return;
    }
    let Some(site) = world.nest_sites.first().copied() else { return };
    let Some(ant) = super::oldest_ant(world) else { return };
    let (w, h) = (super::super::WIDTH as i32, super::super::HEIGHT as i32);
    let mut spots = Vec::new();
    for x in (site.x - 60).max(1)..=(site.x + 60).min(w - 2) {
        for y in (site.surface - 4).max(1)..=(site.surface + 80).min(h - 2) {
            if world.get(x, y).material != material::EMPTY || world.get(x, y + 1).material == material::EMPTY || world.get(x, y + 1).material == food {
                continue;
            }
            if creature::is_home_for(world, ant, (x, y)) {
                spots.push((x, y));
            }
        }
    }
    // Deepest first, then nearest the site's column: inside the chamber.
    spots.sort_by_key(|&(x, y)| (-y, (x - site.x).abs()));
    for (x, y) in spots.into_iter().take(room) {
        world.set(x, y, Cell::new(food, 0));
        k.ration_cells.push((x, y));
        k.counts.ration_given += 1;
    }
}

/// **The read-out a harness and the HUD share**: ants alive, ants that know
/// the station (strength x freshness >= [`KNOWS_PHI`]), and starved deaths.
pub fn census(lab: &Lab) -> (usize, usize, u64) {
    let Some(k) = keeper(lab) else { return (0, 0, 0) };
    let place = station_place(&lab.world, k);
    let Some(ant) = lab.world.species.id_of("ant") else { return (0, 0, 0) };
    let mut alive = 0;
    let mut knows = 0;
    for id in lab.world.live_organism_ids() {
        if lab.world.organism(id).is_none_or(|s| s.species != ant) {
            continue;
        }
        alive += 1;
        if recall::knows(&lab.world, id, place, STATION_R).is_some_and(|v| v >= KNOWS_PHI) {
            knows += 1;
        }
    }
    (alive, knows, lab.world.deaths_by_cause[DeathCause::Starved as usize])
}

/// The demo's counts, for a harness.
pub fn counts(lab: &Lab) -> Option<KeeperCounts> {
    keeper(lab).map(|k| k.counts)
}

/// **Teach**: carry `ant` to the station and set it down there, and plant
/// the place in its memory. Counted as a carry.
pub fn carry_to_station(lab: &mut Lab, ant: OrganismId) -> Result<(i32, i32), &'static str> {
    let Some(k) = keeper(lab) else { return Err("no keeper demo") };
    let place = station_place(&lab.world, k);
    ensure_recall(lab);
    let at = creature::set_down(&mut lab.world, ant, (place.0 - 6, place.1 - 2))?;
    recall::plant(&mut lab.world, ant, place);
    if let Some(k) = keeper_mut(lab) {
        k.counts.carried += 1;
    }
    Ok(at)
}

/// The ant whose head is nearest `cell`, within [`PICK_R`]: one the hand can
/// lift if there is one (`creature::can_carry`), else the nearest, so a
/// refusal names why.
fn ant_at(world: &World, cell: (i32, i32)) -> Option<OrganismId> {
    ant_near(world, cell, true).or_else(|| ant_near(world, cell, false))
}

fn ant_near(world: &World, cell: (i32, i32), liftable: bool) -> Option<OrganismId> {
    let ant = world.species.id_of("ant")?;
    let mut best: Option<(i32, OrganismId)> = None;
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        if st.species != ant || (liftable && !creature::can_carry(world, id)) {
            continue;
        }
        let Some(&(x, y)) = st.chain.first() else { continue };
        let d = (x - cell.0).abs().max((y - cell.1).abs());
        if d <= PICK_R && best.is_none_or(|b| (d, id) < b) {
            best = Some((d, id));
        }
    }
    best.map(|(_, id)| id)
}

/// The edible cell nearest `at` within [`LESSON_R`], if any.
fn food_near(world: &World, at: (i32, i32)) -> Option<(i32, i32)> {
    let mut best: Option<(i32, (i32, i32))> = None;
    for dy in -LESSON_R..=LESSON_R {
        for dx in -LESSON_R..=LESSON_R {
            let c = (at.0 + dx, at.1 + dy);
            if !world.in_bounds(c.0, c.1) {
                continue;
            }
            let cell = world.get(c.0, c.1);
            if cell.material == material::EMPTY || cell.organism_id() != 0 || creature::food_value(world, cell) <= 0.0 {
                continue;
            }
            let d = dx.abs().max(dy.abs());
            if best.is_none_or(|b| d < b.0) {
                best = Some((d, c));
            }
        }
    }
    best.map(|(_, c)| c)
}

/// **Teach `ant` the food near `at`**, unless that food is at home (a lesson
/// at home teaches "here is the nest", and a mind never writes one there
/// itself). Returns where it was taught.
fn teach_near(lab: &mut Lab, ant: OrganismId, at: (i32, i32)) -> Option<(i32, i32)> {
    let food = food_near(&lab.world, at)?;
    if creature::is_home_for(&lab.world, ant, food) {
        return None;
    }
    ensure_recall(lab);
    recall::plant(&mut lab.world, ant, food).then_some(food)
}

/// The pointer over the world; `true` if the keeper seat took it.
pub fn mouse(lab: &mut Lab, cell: (i32, i32), act: super::MouseAct) -> bool {
    let Some(tool) = keeper(lab).map(|k| k.tool) else { return false };
    match (tool, act) {
        (Tool::None, _) => false,
        (Tool::Carry, super::MouseAct::Press { .. }) => {
            let ant = ant_at(&lab.world, cell);
            if let Some(k) = keeper_mut(lab) {
                k.held = ant;
            }
            if ant.is_none() {
                lab.ui.say("CARRY: NO ANT UNDER THE HAND".to_string());
            }
            true
        }
        (Tool::Carry, super::MouseAct::Drag) => true,
        (Tool::Carry, super::MouseAct::Release) => {
            let Some(ant) = keeper_mut(lab).and_then(|k| k.held.take()) else { return true };
            let msg = match creature::set_down(&mut lab.world, ant, cell) {
                Ok(at) => {
                    if let Some(k) = keeper_mut(lab) {
                        k.counts.carried += 1;
                    }
                    match teach_near(lab, ant, at) {
                        Some(f) => format!("SET DOWN AT {}, {} -- IT REMEMBERS THE FOOD AT {}, {}", at.0, at.1, f.0, f.1),
                        None => format!("SET DOWN AT {}, {} -- NO FOOD AWAY FROM HOME WITHIN {LESSON_R} CELLS, SO NOTHING TO REMEMBER", at.0, at.1),
                    }
                }
                Err(why) => format!("CANNOT SET IT DOWN THERE: {}", why.to_uppercase()),
            };
            lab.ui.say(msg);
            true
        }
        (Tool::Treat, super::MouseAct::Press { .. }) => {
            let msg = match ant_at(&lab.world, cell) {
                None => "TREAT: NO ANT UNDER THE HAND".to_string(),
                Some(ant) => {
                    let Some(food) = lab.world.materials.id_of("provisions") else { return true };
                    let worth = creature::food_value(&lab.world, Cell::new(food, 0));
                    if creature::treat(&mut lab.world, ant, food, worth) {
                        if let Some(k) = keeper_mut(lab) {
                            k.counts.treated += 1;
                        }
                        let at = lab.world.organism(ant).and_then(|s| s.chain.first().copied()).unwrap_or(cell);
                        let taught = (!creature::is_home_for(&lab.world, ant, at)).then(|| {
                            ensure_recall(lab);
                            recall::plant(&mut lab.world, ant, at)
                        });
                        if taught == Some(true) {
                            "TREATED -- IT REMEMBERS WHERE".to_string()
                        } else {
                            "TREATED AT HOME -- A MEAL, BUT NOTHING TO REMEMBER".to_string()
                        }
                    } else {
                        "TREAT REFUSED: ITS CROP IS FULL OR HOLDS ANOTHER FOOD".to_string()
                    }
                }
            };
            lab.ui.say(msg);
            true
        }
        (Tool::Treat, _) => true,
    }
}

/// Fixed colours (a full replace, never a blend: CLAUDE.md's debug-readout
/// rule): by generation, white for the player's own hand.
const GEN_COLOURS: [[u8; 4]; 3] = [[255, 255, 255, 255], [255, 176, 32, 255], [230, 40, 40, 255]];
const ASLEEP: [u8; 4] = [110, 110, 110, 255];
/// The traffic ramp, dark to bright; a cell's count picks the step.
const HEAT_RAMP: [[u8; 4]; 5] = [[40, 40, 140, 255], [40, 120, 200, 255], [40, 200, 160, 255], [230, 230, 60, 255], [255, 255, 255, 255]];

fn put_world(lab: &Lab, frame_buf: &mut [u8], (x, y): (i32, i32), colour: [u8; 4]) {
    let hc = super::super::ui::hud_canvas(&lab.renderer);
    if let Some((sx, sy)) = lab.renderer.world_to_logical(x, y) {
        hc.put(frame_buf, sx, sy, colour);
    }
}

/// A square ring of half-width `r` around `at`.
fn ring(lab: &Lab, frame_buf: &mut [u8], at: (i32, i32), r: i32, colour: [u8; 4]) {
    for d in -r..=r {
        put_world(lab, frame_buf, (at.0 + d, at.1 - r), colour);
        put_world(lab, frame_buf, (at.0 + d, at.1 + r), colour);
        put_world(lab, frame_buf, (at.0 - r, at.1 + d), colour);
        put_world(lab, frame_buf, (at.0 + r, at.1 + d), colour);
    }
}

/// Marks over the world: the overlay, the ant in the hand, the station.
pub fn draw_world(lab: &Lab, frame_buf: &mut [u8]) {
    let Some(k) = keeper(lab) else { return };
    match k.overlay {
        Overlay::Off => {}
        Overlay::Knows => {
            if let Some(r) = lab.world.recall.as_ref() {
                for (&id, p) in &r.provenance {
                    let Some(&head) = lab.world.organism(id).and_then(|s| s.chain.first()) else { continue };
                    let colour = if recall::lesson(&lab.world, id).is_some() { GEN_COLOURS[(p.generation as usize).min(2)] } else { ASLEEP };
                    ring(lab, frame_buf, head, 2, colour);
                }
            }
        }
        Overlay::Traffic => {
            let w = super::super::WIDTH as i32;
            for (i, &c) in k.heat.iter().enumerate() {
                if c == 0 {
                    continue;
                }
                let step = ((c as f32).log2() as usize / 2).min(HEAT_RAMP.len() - 1);
                put_world(lab, frame_buf, (i as i32 % w, i as i32 / w), HEAT_RAMP[step]);
            }
        }
    }
    if k.overlay != Overlay::Off {
        let st = station_place(&lab.world, k);
        ring(lab, frame_buf, st, STATION_R, [80, 220, 80, 255]);
    }
    if let Some(&head) = k.held.and_then(|id| lab.world.organism(id)).and_then(|s| s.chain.first()) {
        ring(lab, frame_buf, head, 3, [80, 220, 255, 255]);
    }
}

/// Whether [`draw_world`] drew anything this frame.
pub fn marks_world(lab: &Lab) -> bool {
    keeper(lab).is_some_and(|k| k.overlay != Overlay::Off || k.held.is_some())
}

/// The keeper's panel, under the frame counter.
pub fn draw_hud(lab: &Lab, hc: crate::render::Hud, frame_buf: &mut [u8]) {
    let Some(k) = keeper(lab) else { return };
    let (alive, knows, starved) = census(lab);
    let c = k.counts;
    let r = lab.world.recall.as_ref().map(|r| r.counts).unwrap_or_default();
    let tool = match k.tool {
        Tool::None => "NONE",
        Tool::Carry => "CARRY",
        Tool::Treat => "TREAT",
    };
    let first = c.first_station_delivery.map_or("NOT YET".to_string(), |f| format!("FRAME {f}"));
    // Short lines: the panel is 512 logical pixels wide, ~80 glyphs.
    let lines = [
        (format!("KEEPER  Y CARRY  U TREAT  P LISTEN  J OVERLAY  -- HAND: {tool}"), [230, 230, 230, 255]),
        (
            if k.listening { format!("LISTENING: SCRIPTED GAIN {}", k.gain) } else { "LISTENING: OFF (TODAY'S ANTS)".to_string() },
            if k.listening { [120, 230, 120, 255] } else { [230, 170, 90, 255] },
        ),
        (format!("STATION: {} VISITS ({} TAUGHT OR FED) -- {} CELLS HOME -- FIRST {first}", c.arrivals, c.taught_arrivals, c.station_deliveries), [200, 200, 200, 255]),
        (format!("KNOW IT: {knows} OF {alive} -- CARRIED {}, TREATED {} -- PASSED ON {}", c.carried, c.treated, r.copies), [200, 200, 200, 255]),
        (format!("RATION {} CELLS -- STARVED {starved}", c.ration_given), [150, 150, 150, 255]),
    ];
    for (i, (line, colour)) in lines.iter().enumerate() {
        hc.text(frame_buf, 4, 16 + 10 * i as i32, line, *colour);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo(args: &[(&str, &str)]) -> Lab {
        let sc = crate::lab::scenario::Scenario::load("demo_keeper").expect("demo_keeper ships");
        let mut lab = Lab::new(sc.bed.clone());
        lab.set_new_ant(true);
        let kv = args.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        lab.start_demo_with(super::super::DemoSeat::Keeper, kv);
        lab
    }

    /// **The hand end to end, and a rebuild after it.** CARRY through
    /// [`mouse`] (press on an ant, release beside the station) sets the ant
    /// down there, teaches it the station's food, and counts the carry; the
    /// recall head is on the world at the demo's gain. Then `Lab::reset`
    /// builds a new `World`: the head must come back on the first tick and
    /// the counts start again, or every carry after a rebuild teaches
    /// nothing. Positive controls: the lesson is checked to be *absent*
    /// before the carry, and the head absent straight after the reset.
    #[test]
    fn the_hand_carries_and_teaches_and_survives_a_rebuild() {
        let mut lab = demo(&[("warmup", "300")]);
        assert!(lab.world.minds.is_some(), "the demo must run the trained ant");
        assert_eq!(lab.world.recall.as_ref().map(|r| r.gain), Some(DEFAULT_GAIN));
        let (ant, head) = lab
            .world
            .live_organism_ids()
            .into_iter()
            .filter(|&id| creature::can_carry(&lab.world, id))
            .find_map(|id| Some((id, *lab.world.organism(id)?.chain.first()?)))
            .expect("an ant to lift");
        seat_key(&mut lab, 'Y');
        assert!(mouse(&mut lab, head, super::super::MouseAct::Press { erase: false }));
        // The hand takes the nearest liftable head, which in a pile may be a
        // neighbour's: teach whichever it took.
        let ant = keeper(&lab).and_then(|k| k.held).unwrap_or_else(|| panic!("the press beside ant {ant:?} picked nothing"));
        assert!(recall::lesson(&lab.world, ant).is_none(), "taught before the hand touched it");
        let place = station_place(&lab.world, keeper(&lab).expect("keeper"));
        assert!(mouse(&mut lab, (place.0 - 6, place.1 - 2), super::super::MouseAct::Release));
        let at = *lab.world.organism(ant).and_then(|s| s.chain.first()).expect("still alive");
        assert!((at.0 - place.0).abs() <= LESSON_R, "set down at {at:?}, the station is at {place:?}");
        assert!(recall::lesson(&lab.world, ant).is_some(), "set down beside food and taught nothing");
        assert_eq!(counts(&lab).map(|c| c.carried), Some(1));
        lab.reset();
        assert!(lab.world.recall.is_none(), "a rebuilt world kept the old head: this test is not testing the re-apply");
        lab.tick_for_harness();
        assert!(lab.world.recall.is_some(), "the recall head did not come back after a rebuild");
        assert_eq!(counts(&lab).map(|c| c.carried), Some(0), "the counts outlived the box they counted");
    }
}
