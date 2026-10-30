//! **The guest seat's demo** (`demo=guest`, 2026-10-25): the player is a
//! rove beetle that lives off an ant colony by smelling like it -- "the
//! beggar". The design, its adversarial review, the plan and the plan's
//! independent review are `Reports/guest-seat-demo-2026-10-25.md`; the
//! mechanism is `sim::creature::guest` (the smell ladder), and the body is
//! the `rove` species, steered by slice 0's hooks.
//!
//! **What the seat does and nothing else**: lands the rove beside the door
//! once the colony has grown (the clock runs at top speed until then, and
//! drops to 1x at the landing), sends R P Y B E J to the ladder's verbs,
//! respawns the rove three seconds after it dies, and draws what the ants
//! decided: a mark over every ant that judged you in the last second, the
//! rove outlined in a colour that says how far off it smells, and a ledger.
//! The ants' side is all in the engine; nothing here decides for them.
//!
//! **Stakes** (2026-10-30, on by default; `stakes=0` is the sandbox above).
//! The 10-30 review found no loop after the first 20 s: death respawned free
//! at full health, food bought nothing, the grade locked on the first gift
//! and begging paid on 76-92% of asks. With stakes the seat is a **visit**:
//! five minutes and three roves ([`VISIT_FRAMES`], [`LIVES`]); energy drains
//! ([`DRAIN_J_PER_S`] on top of the body's own burn); a nip costs joules
//! ([`NIP_COST_J`]) and a gift heals ([`heal`]); every beg makes the colony
//! warier (`guest::Wariness`, the engine's half) and a rub calms it; `L` lays
//! an egg in the nest for [`EGG_J`], which banks score; the grade reads the
//! last 30 s, so it moves both ways; and the visit ends on a card with the
//! score (alive seconds + eggs + energy left / 10, [`Score`]).

use crate::sim::cell::OrganismId;
use crate::sim::creature::guest::{self, Rung};
use crate::sim::organism::SpeciesId;
use crate::sim::scheduler::ActiveKind;
use crate::sim::steer::{self, SteerInput, SteerRole};
use crate::sim::world::World;

use super::super::Lab;

/// The guest seat's own keys, as the help line prints them.
pub const SEAT_KEYS: &str = "R RUB AN ANT, P SOAK (ON/OFF), Y BEG, L LAY AN EGG, B KILL A LARVA, E EAT A CORPSE, J JUMP, K NEW VISIT";

/// The frame the rove first lands at, unless `land=` says otherwise. The
/// colony lands at 600 (`demo_guest.ron`); by 30,000 steady income's
/// colony has brood and fed workers to beg from (`guestprobe` DONORS).
pub const LAND_DEFAULT: u64 = 30_000;
/// How far off the rove smells when it lands, unless `gap=` says otherwise,
/// in scent units (the ants' tolerance radius is about 1): far enough that
/// a raw guest is nipped within seconds of contact (ladder probe: at 1.0
/// three of five seeds killed it inside 6,000 frames; at 2.0 every seed
/// inside 1,800).
pub const GAP_DEFAULT: f32 = 1.2;
/// Columns from the nest's door to where the rove lands.
pub const DOOR_OFFSET: i32 = 24;
/// Frames from a death to the next rove (3 s at 1x).
pub const RESPAWN: u64 = 180;
/// Frames a mark stays over an ant (about a second at 1x).
pub const MARK_FRAMES: u64 = 66;
/// The speed preset the warm-up runs at (`time::PRESETS`: 256x).
const WARMUP_PRESET: usize = 5;

/// **A visit's length**, from the first landing: five minutes at 1x.
pub const VISIT_FRAMES: u64 = 5 * 60 * 60;
/// Roves per visit: the first landing and two more. No free respawn after.
pub const LIVES: u32 = 3;
/// Extra burn on the rove, J/s at 1x, on top of its body's own (~1.6 J/s,
/// review §2). With it a fed-nothing rove lasts ~75 s instead of ~2 min, so
/// food matters inside the 30-second loop.
pub const DRAIN_J_PER_S: f32 = 1.0;
/// What one nip costs in joules, on top of the bite's share of a cell.
pub const NIP_COST_J: f32 = 10.0;
/// One gift heals one nip's worth of a wound, or regrows a lost cell.
pub const HEAL_PER_GIFT: f32 = 0.34;
/// **An egg**: what it costs, the bank it must leave behind, and what it is
/// worth -- double when laid beside the host's brood (the risky place).
pub const EGG_J: f32 = 100.0;
pub const EGG_KEEP_J: f32 = 30.0;
pub const EGG_POINTS: u32 = 100;
pub const BROOD_EGG_POINTS: u32 = 200;
/// Columns either side of the door and rows above/below its surface that
/// count as in the nest for laying (the ladder's soak zone, `SOAK_REACH`,
/// `SOAK_DEPTH`).
const NEST_REACH: i32 = 16;
const NEST_DEPTH: i32 = 60;
/// Each ant beside you when you lay adds this to the colony's wariness.
pub const LAY_WARY: f32 = 0.15;
/// The grade's window: what you did in the last 30 s.
const GRADE_WINDOW: u64 = 30 * 60;
/// Colony wariness at which the ticker warns, and the grade reads PEST.
const WARY_WARN: f32 = 0.5;

/// One egg laid this visit.
#[derive(Clone, Copy, Debug)]
pub struct Egg {
    pub at: (i32, i32),
    pub frame: u64,
    /// Laid beside the host's brood: worth [`BROOD_EGG_POINTS`].
    pub brood: bool,
}

/// **A visit's score**: one point a second alive, the eggs, and the living
/// rove's energy over ten (lost if it is dead at the end).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Score {
    pub alive_s: u32,
    pub egg_points: u32,
    pub energy_points: u32,
}

impl Score {
    pub fn total(&self) -> u32 {
        self.alive_s + self.egg_points + self.energy_points
    }
}

/// Why the visit ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisitEnd {
    TimeUp,
    OutOfRoves,
}

/// The guest seat's running state.
#[derive(Clone, Debug, Default)]
pub struct GuestDemo {
    /// The frame the first rove lands.
    pub land: u64,
    /// Its landing smell, in scent units from the nest's odour.
    pub gap: f32,
    /// The rove now (alive or just dead).
    pub body: Option<OrganismId>,
    /// The frame the current rove landed, for its life's length.
    pub born: u64,
    /// The frame the last rove died, while waiting to respawn.
    pub died_at: Option<u64>,
    /// Roves landed so far.
    pub lives: u32,
    /// The longest life so far, in frames.
    pub best_life: u64,
    /// P: soaking in the nest's smell, on or off.
    pub soak: bool,
    /// The colony the guest visits, fixed at the first landing.
    pub host_colony: Option<u32>,
    /// The guest's own colony number, so a respawn rejoins it.
    pub guest_colony: Option<u32>,
    /// The host's adults and brood when the first rove landed.
    pub arrival: (usize, usize),
    /// The warm-up has started (the clock was set to top speed).
    pub warming: bool,
    /// Stakes on (the default; `stakes=0` is the sandbox).
    pub stakes: bool,
    /// The frame the visit's first rove landed.
    pub visit_start: Option<u64>,
    /// Frames alive in finished lives this visit.
    pub alive_frames: u64,
    /// Eggs laid this visit.
    pub eggs: Vec<Egg>,
    /// The visit ended: when, why, and the score it ended on.
    pub ended: Option<(u64, VisitEnd, Score)>,
    /// The ladder's fed and nipped counts as last seen, for the per-tick
    /// heal and nip cost.
    pub seen: (u64, u64),
    /// The rove's energy as last seen alive, to tell a starving death.
    pub last_energy: f32,
    /// The ticker has warned of wariness (cleared below half the line).
    pub warned: bool,
    /// The last frame a larva was killed or a corpse eaten, for the grade.
    pub last_kill: Option<u64>,
    pub last_eat: Option<u64>,
    /// The ladder's larvae-killed and corpses-eaten counts as last seen.
    pub seen_takes: (u64, u64),
}

impl GuestDemo {
    pub fn new(args: &std::collections::BTreeMap<String, String>) -> GuestDemo {
        GuestDemo {
            land: args.get("land").and_then(|v| v.parse().ok()).unwrap_or(LAND_DEFAULT),
            gap: args.get("gap").and_then(|v| v.parse().ok()).unwrap_or(GAP_DEFAULT),
            stakes: args.get("stakes").is_none_or(|v| v != "0"),
            ..GuestDemo::default()
        }
    }
}

fn state(lab: &Lab) -> Option<&GuestDemo> {
    lab.demo.as_ref().and_then(|d| d.guest.as_ref())
}

fn state_mut(lab: &mut Lab) -> Option<&mut GuestDemo> {
    lab.demo.as_mut().and_then(|d| d.guest.as_mut())
}

/// **Which body is the player's**: the living rove the ladder knows, if one
/// has landed. Never an ant (the launcher's default would hand you one).
pub fn pick_body(world: &World) -> Option<OrganismId> {
    world.visitors.as_ref()?.list.iter().map(|v| v.id).find(|&id| world.organism(id).is_some())
}

/// The colony with the most living ants, and the ant species.
fn host_colony(world: &World) -> Option<(u32, SpeciesId)> {
    let ant = world.species.id_of("ant")?;
    let mut counts: crate::sim::fxhash::FxHashMap<u32, usize> = Default::default();
    for id in world.live_organism_ids() {
        if let Some(s) = world.organism(id).filter(|s| s.species == ant) {
            *counts.entry(s.colony).or_default() += 1;
        }
    }
    let best = counts.into_iter().max_by_key(|&(c, n)| (n, std::cmp::Reverse(c)))?;
    Some((best.0, ant))
}

/// The host colony's adults and brood, for the ledger.
fn colony_size(world: &World, colony: u32) -> (usize, usize) {
    let ant = world.species.id_of("ant");
    let adults = world.live_organism_ids().into_iter().filter(|&id| world.organism(id).is_some_and(|s| Some(s.species) == ant && s.colony == colony)).count();
    let brood = world.live_brood_ids().into_iter().filter(|&id| world.colony_of(id) == colony).count();
    (adults, brood)
}

/// **Land a rove beside the host's door**, smelling `gap` from its odour,
/// registered with the ladder and marked as the player's body. Tries both
/// sides of the door, lifting until the body fits. Returns its id.
fn land(lab: &mut Lab) -> Option<OrganismId> {
    let (host, ant) = host_colony(&lab.world)?;
    let st = state(lab)?;
    let (gap, guest_colony, lives) = (st.gap, st.guest_colony, st.lives);
    let nest = lab.world.nest_sites.iter().filter(|n| n.seeded).min_by_key(|n| {
        // The site nearest the host's ants: the first one with a living
        // ant of that colony within 40 columns, else the first seeded.
        let near = lab.world.live_organism_ids().into_iter().filter(|&id| lab.world.organism(id).is_some_and(|s| s.colony == host && s.chain.first().is_some_and(|c| (c.0 - n.x).abs() < 40))).count();
        std::cmp::Reverse(near)
    })?;
    let (nx, surface, scent) = (nest.x, nest.surface, nest.scent);
    // Alternate sides life by life, so a respawn is not walked into the
    // same welcome.
    let side = if lives % 2 == 0 { 1 } else { -1 };
    let mut placed = None;
    'search: for off in [DOOR_OFFSET * side, -DOOR_OFFSET * side, (DOOR_OFFSET + 4) * side, -(DOOR_OFFSET + 4) * side] {
        for lift in 0..24 {
            if let Some(s) = crate::sim::creature::plant_creature_seed_in(&mut lab.world, nx + off, surface - 2 - lift, "rove", guest_colony) {
                if let ActiveKind::Creature { organism } = s.kind {
                    lab.world.schedule_active_site(s);
                    placed = Some(organism);
                    break 'search;
                }
            }
        }
    }
    let id = placed?;
    let (home, _) = guest::scent_away_from(scent, gap);
    guest::register(&mut lab.world, ant, id, home);
    // `wary=0` keeps the stakes but not the beg wariness: the positive
    // control that says wariness is what separates a spammer from a careful
    // beggar (README, the paired table).
    let wary_on = lab.demo.as_ref().is_none_or(|d| d.args.get("wary").is_none_or(|v| v != "0"));
    let stakes = state(lab).is_some_and(|s| s.stakes) && wary_on;
    if let Some(v) = lab.world.visitors.as_mut() {
        if stakes && v.wary.is_none() {
            v.wary = Some(guest::Wariness::default());
        }
    }
    // A respawn takes the dead body's place as the player's: slice 0 keeps a
    // dead body's id on `World::steered`, so it is cleared first.
    lab.world.steered = None;
    steer::mark_with(&mut lab.world, id, Some(SteerInput::default()), SteerRole::Worker);
    if let Some(s) = lab.world.steered.as_mut() {
        s.log = Some(Vec::new());
    }
    let frame = lab.world.frame;
    let colony = lab.world.colony_of(id);
    let arrival = colony_size(&lab.world, host);
    let soak = state(lab).is_some_and(|s| s.soak);
    if let Some(v) = lab.world.visitors.as_mut() {
        v.input.soak = soak;
    }
    let energy = lab.world.organism(id).map_or(0.0, |s| s.energy);
    let seen = lab.world.visitors.as_ref().map_or((0, 0), |v| (v.counts.fed, v.counts.nipped));
    let st = state_mut(lab)?;
    if st.lives == 0 {
        st.arrival = arrival;
        st.visit_start = Some(frame);
    }
    st.seen = seen;
    st.last_energy = energy;
    st.body = Some(id);
    st.born = frame;
    st.died_at = None;
    st.lives += 1;
    st.host_colony = Some(host);
    st.guest_colony = Some(colony);
    Some(id)
}

/// One of the seat keys was pressed. `B` and `E` are the seat's, not the
/// body's bite and eat (the queen's `seat_takes`): the rove's own mouth
/// would kill a worker, which is not a beggar's verb (review finding 4).
/// `L` is the seat's egg, not the body's lay (the rove never breeds), and
/// `K` starts a new visit once one has ended. Every press is logged
/// (`World::log_player_action`): each one changes the box or the rove.
pub fn seat_key(lab: &mut Lab, key: char) {
    let word = match key {
        'R' => "RUB",
        'P' => "SOAK",
        'Y' => "BEG",
        'L' => "LAY",
        'B' => "KILL A LARVA",
        'E' => "EAT A CORPSE",
        'J' => "JUMP",
        'K' => "NEW VISIT",
        _ => return,
    };
    lab.world.log_player_action(format!("GUEST: {word} ({key})"));
    if key == 'J' {
        if let Some(s) = lab.world.steered.as_mut() {
            s.press(SteerInput { jump: true, ..SteerInput::default() });
        }
        return;
    }
    if key == 'K' {
        new_visit(lab);
        return;
    }
    if key == 'L' {
        let line = lay(lab);
        lab.ui.say(line);
        return;
    }
    if key == 'P' {
        let on = state_mut(lab).map(|s| {
            s.soak = !s.soak;
            s.soak
        });
        if let (Some(on), Some(v)) = (on, lab.world.visitors.as_mut()) {
            v.input.soak = on;
        }
        lab.ui.say(if on == Some(true) { "SOAKING: STAY BY THE NEST OR THE BROOD TO TAKE ITS SMELL" } else { "SOAK OFF" });
        return;
    }
    let Some(v) = lab.world.visitors.as_mut() else { return };
    match key {
        'R' => v.input.rub = true,
        'Y' => v.input.beg = true,
        'B' => v.input.kill = true,
        'E' => v.input.eat = true,
        _ => {}
    }
}

/// Whether the guest seat takes this letter from the body's layer.
pub fn takes(key: char) -> bool {
    matches!(key, 'B' | 'E' | 'L')
}

/// The host nest's door nearest `(x, y)`: its column and surface row.
fn nearest_door(world: &World, (x, y): (i32, i32)) -> Option<(i32, i32)> {
    world.nest_sites.iter().filter(|n| n.seeded).min_by_key(|n| (n.x - x).pow(2) + (n.surface - y).pow(2)).map(|n| (n.x, n.surface))
}

/// Whether a cell is in the nest, for laying: within [`NEST_REACH`] columns
/// of a door, from the mound's top down into the galleries.
pub fn in_nest(world: &World, at: (i32, i32)) -> bool {
    nearest_door(world, at).is_some_and(|(nx, ny)| (at.0 - nx).abs() <= NEST_REACH && (-NEST_REACH..=NEST_DEPTH).contains(&(at.1 - ny)))
}

/// Host brood (any stage) within two cells of the rove's body.
fn brood_near(world: &World, body: OrganismId) -> bool {
    let Some(st) = world.organism(body) else { return false };
    st.chain.iter().any(|&(x, y)| {
        (-2..=2).any(|dy| {
            (-2..=2).any(|dx| {
                let o = world.get(x + dx, y + dy).organism_id();
                o != 0 && o != body && world.organism(o).is_some_and(|s| s.brood.is_some())
            })
        })
    })
}

/// **Lay an egg** (`L`, stakes only): in the nest, with [`EGG_J`] to spend
/// and [`EGG_KEEP_J`] left after. It banks [`EGG_POINTS`] (double beside the
/// host's brood) and stays drawn where it was laid; every ant beside you
/// sees it and the colony grows warier ([`LAY_WARY`] each). Returns the
/// ticker line.
pub fn lay(lab: &mut Lab) -> String {
    let Some(st) = state(lab) else { return String::new() };
    if !st.stakes {
        return "NO EGGS IN THE SANDBOX (STAKES=0)".into();
    }
    if st.ended.is_some() {
        return "THE VISIT IS OVER -- K FOR A NEW ONE".into();
    }
    let Some(body) = st.body.filter(|&id| lab.world.organism(id).is_some()) else { return "NO ROVE TO LAY".into() };
    let (energy, head) = lab.world.organism(body).map_or((0.0, (0, 0)), |s| (s.energy, s.chain.first().copied().unwrap_or((0, 0))));
    if energy < EGG_J + EGG_KEEP_J {
        return format!("AN EGG NEEDS {:.0} J -- YOU HAVE {energy:.0}: BEG FIRST", EGG_J + EGG_KEEP_J);
    }
    if !in_nest(&lab.world, head) {
        return "LAY IN THE NEST: GO TO THE DOOR OR DOWN THE SHAFT".into();
    }
    let brood = brood_near(&lab.world, body);
    let watchers = lab.world.visitors.as_ref().map_or(0, |v| guest::ants_beside(&lab.world, v.host, body).len());
    let frame = lab.world.frame;
    let colony = lab.world.colony_of(body);
    if let Some(s) = lab.world.organism_mut(body) {
        s.energy -= EGG_J;
    }
    lab.world.book(colony, crate::sim::world::Account::Metabolized, f64::from(EGG_J));
    if let Some(w) = lab.world.visitors.as_mut().and_then(|v| v.wary.as_mut()) {
        w.add(frame, None, LAY_WARY * watchers as f32, 0.0);
    }
    if let Some(s) = state_mut(lab) {
        s.eggs.push(Egg { at: head, frame, brood });
    }
    let pts = if brood { BROOD_EGG_POINTS } else { EGG_POINTS };
    match (brood, watchers) {
        (true, 0) => format!("EGG LAID IN THEIR BROOD, UNSEEN: +{pts}"),
        (true, n) => format!("EGG LAID IN THEIR BROOD: +{pts} -- {n} {} IT", saw(n)),
        (false, 0) => format!("EGG LAID, UNSEEN: +{pts}"),
        (false, n) => format!("EGG LAID: +{pts} -- {n} {} IT, THE COLONY IS WARIER", saw(n)),
    }
}

fn saw(n: usize) -> &'static str {
    if n == 1 {
        "ANT SAW"
    } else {
        "ANTS SAW"
    }
}

/// **The score now**: banked eggs, seconds alive (finished lives and this
/// one), and the living rove's energy over ten.
pub fn score_now(lab: &Lab) -> Score {
    let Some(st) = state(lab) else { return Score::default() };
    if let Some((_, _, sc)) = st.ended {
        return sc;
    }
    let w = &lab.world;
    let alive = st.body.and_then(|id| w.organism(id));
    let this_life = if alive.is_some() && st.died_at.is_none() { w.frame.saturating_sub(st.born) } else { 0 };
    Score {
        alive_s: ((st.alive_frames + this_life) / 60) as u32,
        egg_points: st.eggs.iter().map(|e| if e.brood { BROOD_EGG_POINTS } else { EGG_POINTS }).sum(),
        energy_points: alive.map_or(0, |s| (s.energy.max(0.0) / 10.0) as u32),
    }
}

/// End the visit now, freezing the score.
fn end_visit(lab: &mut Lab, why: VisitEnd) {
    let sc = score_now(lab);
    let frame = lab.world.frame;
    let eggs = state(lab).map_or(0, |s| s.eggs.len());
    if let Some(s) = state_mut(lab) {
        s.ended = Some((frame, why, sc));
    }
    let what = match why {
        VisitEnd::TimeUp => "TIME UP",
        VisitEnd::OutOfRoves => "NO ROVES LEFT",
    };
    lab.world.log_player_action(format!("GUEST VISIT OVER ({what}): SCORE {} -- {eggs} EGGS", sc.total()));
    lab.ui.say(format!("VISIT OVER, {what}: SCORE {} -- K FOR A NEW VISIT", sc.total()));
}

/// **`K`: a new visit**, once one has ended: the last rove (if alive) is
/// left to starve, the ladder's counts are cleared, and a fresh rove lands
/// in three seconds.
fn new_visit(lab: &mut Lab) {
    let Some(st) = state(lab) else { return };
    if st.ended.is_none() {
        lab.ui.say("K STARTS A NEW VISIT ONCE THIS ONE IS OVER");
        return;
    }
    if let Some(s) = st.body.and_then(|id| lab.world.organism_mut(id)) {
        s.energy = -1.0e6;
    }
    let frame = lab.world.frame;
    if let Some(v) = lab.world.visitors.as_mut() {
        v.counts = guest::VisitCounts::default();
        v.wary = Some(guest::Wariness::default());
    }
    if let Some(s) = state_mut(lab) {
        *s = GuestDemo {
            land: frame + RESPAWN,
            gap: s.gap,
            stakes: s.stakes,
            soak: s.soak,
            guest_colony: s.guest_colony,
            warming: true,
            ..GuestDemo::default()
        };
    }
    lab.ui.say("A NEW VISIT: A FRESH ROVE LANDS IN 3 S");
}

/// **A gift heals**: one nip's worth off the wound, or, unwounded and short
/// of cells, one cell regrown at the tail (as `creature::regrow_tail` does,
/// paid for by the gift rather than the bank).
pub fn heal(world: &mut World, body: OrganismId) {
    let Some(st) = world.organism(body) else { return };
    if st.gnawed > 0.0 {
        if let Some(s) = world.organism_mut(body) {
            s.gnawed = (s.gnawed - HEAL_PER_GIFT).max(0.0);
        }
        return;
    }
    let full = world.species.creature_shared(st.species).map_or(0, |d| match d.body {
        crate::sim::organism::BodyPlan::Chain(n) => usize::from(n),
        _ => 0,
    });
    if st.chain.len() < 2 || st.chain.len() >= full {
        return;
    }
    let (tx, ty) = *st.chain.last().expect("two cells or more");
    let cell = world.get(tx, ty);
    let Some((x, y)) = crate::sim::structural::NEIGHBOURS_8.iter().map(|&(dx, dy)| (tx + dx, ty + dy)).find(|&(x, y)| world.in_bounds(x, y) && world.get(x, y).material == crate::sim::material::EMPTY) else {
        return;
    };
    world.set(x, y, cell);
    if let Some(s) = world.organism_mut(body) {
        s.chain.push((x, y));
    }
}

/// Once a displayed frame, after the body has its keys.
pub fn frame(_lab: &mut Lab) {}

/// **Before each simulated tick**: force the trained colony on, run the
/// warm-up at top speed, land the first rove at `land`, and a new one
/// [`RESPAWN`] frames after each death -- with stakes, only while the visit
/// has roves left and time to run.
pub fn before_tick(lab: &mut Lab) {
    let Some(st) = state(lab) else { return };
    let frame = lab.world.frame;
    if !st.warming {
        // The demo runs the trained colony (plan v2: the hand-built ant's
        // mouth and fight are not gated by the ladder).
        if !lab.new_ant {
            let line = lab.set_new_ant(true);
            lab.ui.say(line);
        }
        lab.time.set_preset(WARMUP_PRESET);
        // The welcome card and the battle panel would sit over the guest:
        // the help line (H) says the keys instead.
        lab.show_help = false;
        lab.ui.battle.on = false;
        lab.ui.say("THE COLONY IS GROWING -- YOU LAND WHEN IT HAS BROOD TO STEAL FROM");
        let args = lab.demo.as_ref().map(|d| d.args.clone()).unwrap_or_default();
        if let Some(s) = state_mut(lab) {
            *s = GuestDemo::new(&args);
            s.warming = true;
        }
        return;
    }
    if st.ended.is_some() {
        return;
    }
    if st.stakes && st.visit_start.is_some_and(|t| frame >= t + VISIT_FRAMES) {
        end_visit(lab, VisitEnd::TimeUp);
        return;
    }
    let due = match (st.lives, st.died_at) {
        (0, _) => frame >= st.land,
        (_, Some(d)) => frame >= d + RESPAWN && !(st.stakes && st.lives >= LIVES),
        _ => false,
    };
    if !due {
        return;
    }
    let first = st.lives == 0;
    if land(lab).is_some() {
        if first {
            lab.time.set_preset(0);
        }
        let (gap, stakes, lives) = state(lab).map_or((0.0, false, 0), |s| (s.gap, s.stakes, s.lives));
        lab.ui.say(if !stakes {
            format!("YOU LAND SMELLING {gap:.1} OFF: RUB ANTS (R), SOAK (P), BEG (Y)")
        } else if first {
            "5 MIN, 3 ROVES: RUB (R) TILL GREEN, BEG (Y), LAY EGGS IN THE NEST (L)".to_string()
        } else {
            format!("ROVE {lives} OF {LIVES} LANDS SMELLING {gap:.1} OFF: RUB BEFORE YOU BEG")
        });
    }
}

/// **After each simulated tick**: with stakes, the drain, the nip cost and
/// the gift's heal; then notice a death and start the respawn clock (or end
/// the visit on the last rove).
pub fn after_tick(lab: &mut Lab) {
    let Some(st) = state(lab) else { return };
    let Some(body) = st.body else { return };
    if st.stakes && st.ended.is_none() && st.died_at.is_none() && lab.world.organism(body).is_some() {
        stakes_tick(lab, body);
    }
    let Some(st) = state(lab) else { return };
    if st.died_at.is_some() || lab.world.organism(body).is_some() {
        return;
    }
    let frame = lab.world.frame;
    let life = frame.saturating_sub(st.born);
    let (stakes, lives, starved, ended) = (st.stakes, st.lives, st.last_energy <= 5.0, st.ended.is_some());
    if let Some(s) = state_mut(lab) {
        s.died_at = Some(frame);
        s.best_life = s.best_life.max(life);
        if !ended {
            s.alive_frames += life;
        }
    }
    if !stakes {
        lab.ui.say(format!("THE COLONY KILLED YOU AFTER {} -- ANOTHER ROVE LANDS IN 3 S", clock(life)));
        return;
    }
    if ended {
        return;
    }
    let how = if starved { "STARVED" } else { "KILLED" };
    lab.world.log_player_action(format!("GUEST ROVE {lives} {how} AFTER {}", clock(life)));
    if lives >= LIVES {
        end_visit(lab, VisitEnd::OutOfRoves);
    } else {
        lab.ui.say(format!("ROVE {how} AFTER {} -- {} LEFT, NEXT LANDS IN 3 S", clock(life), LIVES - lives));
    }
}

/// **The wariness the next beg meets**: the colony's, plus the warier of
/// the ants beside the rove (their own memory of being asked). The HUD's
/// WARY bar, the ticker's warning and the grade's PEST all read this, not
/// the colony term alone: on the first 12-seed table a spammer was refused
/// on 58% of asks while the colony term sat near 0 -- it was the same few
/// ants being asked again -- so a colony-only bar said CALM beside a row
/// of refusals.
pub fn wary_near(world: &World, body: OrganismId) -> f32 {
    let Some(v) = world.visitors.as_ref() else { return 0.0 };
    let Some(w) = v.wary.as_ref() else { return 0.0 };
    let now = world.frame;
    let ant = guest::ants_beside(world, v.host, body).iter().map(|e| w.ant(e.0, now)).fold(0.0, f32::max);
    w.colony(now) + ant
}

/// The stakes' per-tick bills and rewards on a living rove: the drain, the
/// nip cost, a heal per gift, the wariness warning, and when food was last
/// taken without asking (the grade's thief and predator rungs).
fn stakes_tick(lab: &mut Lab, body: OrganismId) {
    let Some(c) = lab.world.visitors.as_ref().map(|v| v.counts) else { return };
    let (seen_fed, seen_nip) = state(lab).map_or((c.fed, c.nipped), |s| s.seen);
    let colony = lab.world.colony_of(body);
    let cost = DRAIN_J_PER_S / 60.0 + NIP_COST_J * c.nipped.saturating_sub(seen_nip) as f32;
    if let Some(s) = lab.world.organism_mut(body) {
        s.energy -= cost;
    }
    lab.world.book(colony, crate::sim::world::Account::Metabolized, f64::from(cost));
    for _ in 0..c.fed.saturating_sub(seen_fed) {
        heal(&mut lab.world, body);
    }
    let frame = lab.world.frame;
    let wary = wary_near(&lab.world, body);
    let energy = lab.world.organism(body).map_or(0.0, |s| s.energy);
    let mut warn = None;
    if let Some(s) = state_mut(lab) {
        if c.larvae_killed > s.seen_takes.0 {
            s.last_kill = Some(frame);
        }
        if c.corpses_eaten > s.seen_takes.1 {
            s.last_eat = Some(frame);
        }
        s.seen_takes = (c.larvae_killed, c.corpses_eaten);
        s.seen = (c.fed, c.nipped);
        s.last_energy = energy;
        if wary >= WARY_WARN && !s.warned {
            s.warned = true;
            warn = Some("THE COLONY IS WARY OF YOUR BEGGING -- RUB ANTS OR MOVE ON");
        } else if wary < 0.5 * WARY_WARN {
            s.warned = false;
        }
    }
    if let Some(line) = warn {
        lab.ui.say(line);
    }
}

/// `m:ss` of game time at 60 frames a second.
fn clock(frames: u64) -> String {
    let s = frames / 60;
    format!("{}:{:02}", s / 60, s % 60)
}

/// The pointer: the guest seat leaves it to the lab.
pub fn mouse(_lab: &mut Lab, _cell: (i32, i32), _act: super::MouseAct) -> bool {
    false
}

/// **The smell ramp**, fixed, full replace (never a blend into the body's own
/// colour): green family, amber doubtful, red stranger, at the ladder's own
/// centres (antennate 0.5, nip line 0.8, nip 1.3).
pub fn gap_colour(g: f32) -> [u8; 4] {
    if !g.is_finite() {
        return [160, 160, 160, 255];
    }
    let stops: [(f32, [f32; 3]); 4] = [(0.0, [60.0, 220.0, 90.0]), (0.5, [170.0, 220.0, 60.0]), (0.9, [255.0, 170.0, 40.0]), (1.4, [240.0, 50.0, 40.0])];
    let g = g.clamp(0.0, 1.4);
    let mut c = stops[stops.len() - 1].1;
    for w in stops.windows(2) {
        let ((a, ca), (b, cb)) = (w[0], w[1]);
        if g <= b {
            let t = (g - a) / (b - a);
            c = [0, 1, 2].map(|k| ca[k] + t * (cb[k] - ca[k]));
            break;
        }
    }
    [c[0] as u8, c[1] as u8, c[2] as u8, 255]
}

/// A mark's glyph and colour.
fn mark_glyph(r: Rung) -> Option<(&'static str, [u8; 4])> {
    match r {
        Rung::Antennate => Some(("?", [255, 230, 60, 255])),
        Rung::Display => Some(("!", [255, 140, 30, 255])),
        Rung::Nip => Some(("X", [255, 40, 40, 255])),
        Rung::Fed => Some(("+", [80, 240, 100, 255])),
        Rung::Empty => Some(("-", [170, 170, 170, 255])),
        Rung::Rubbed => Some(("*", [120, 200, 255, 255])),
        Rung::Ignore => None,
    }
}

/// Marks over the world: none on the box's buffer (they are on the 2x
/// interface layer, [`draw_hud`]), so the dirty-rect skip is untouched.
pub fn draw_world(_lab: &Lab, _frame_buf: &mut [u8]) {}

/// Whether [`draw_world`] drew anything this frame.
pub fn marks_world(_lab: &Lab) -> bool {
    false
}

/// **The guest's panel and marks**, on the interface canvas: a mark over
/// every ant that judged you in the last second, the rove ringed in its
/// smell colour, and the ledger.
pub fn draw_hud(lab: &Lab, hc: crate::render::Hud, frame_buf: &mut [u8]) {
    let Some(st) = state(lab) else { return };
    let w = &lab.world;
    let frame = w.frame;
    let Some(v) = w.visitors.as_ref() else {
        if st.lives == 0 {
            let left = st.land.saturating_sub(frame);
            hc.text(frame_buf, 8, 40, &format!("GUEST: THE COLONY GROWS -- YOU LAND IN {}", clock(left)), [230, 200, 140, 255]);
        }
        return;
    };
    // The eggs, where they were laid: cream, gold-rimmed in the brood.
    // Drawn first, so the marks and the rove's ring stay on top of them.
    let z = lab.renderer.zoom.max(1);
    for e in &st.eggs {
        if let Some((sx, sy)) = lab.renderer.world_to_screen(e.at.0, e.at.1) {
            let (ew, eh) = (z + 3, z + 5);
            let rim = if e.brood { [255, 200, 40, 255] } else { [40, 30, 20, 255] };
            hc.fill_rect(frame_buf, sx - 1, sy, ew + 2, eh, rim);
            hc.fill_rect(frame_buf, sx, sy - 1, ew, eh + 2, rim);
            hc.fill_rect(frame_buf, sx, sy + 1, ew, eh - 2, [245, 235, 205, 255]);
            hc.fill_rect(frame_buf, sx + 1, sy, ew - 2, eh, [245, 235, 205, 255]);
        }
    }
    // Marks over the ants: one glyph each, over the head, for ~1 s.
    for (&ant, m) in &v.marks {
        if frame.saturating_sub(m.at) > MARK_FRAMES {
            continue;
        }
        let Some((glyph, colour)) = mark_glyph(m.rung) else { continue };
        let Some(&(x, y)) = w.organism(ant).and_then(|s| s.chain.first()) else { continue };
        if let Some((sx, sy)) = lab.renderer.world_to_screen(x, y) {
            hc.fill_rect(frame_buf, sx - 3, sy - 12, 7, 9, [0, 0, 0, 200]);
            hc.text(frame_buf, sx - 2, sy - 11, glyph, colour);
        }
    }
    let body = st.body.and_then(|id| w.organism(id).map(|s| (id, s)));
    let g = st.body.and_then(|id| v.list.iter().find(|x| x.id == id)).map_or(f32::NAN, |x| x.gap);
    let ring = gap_colour(g);
    if let Some((_, s)) = body {
        // The ring: a box round the rove's cells in its smell colour.
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for &(x, y) in &s.chain {
            if let Some((sx, sy)) = lab.renderer.world_to_screen(x, y) {
                (x0, y0, x1, y1) = (x0.min(sx), y0.min(sy), x1.max(sx), y1.max(sy));
            }
        }
        if x0 <= x1 {
            let z = lab.renderer.zoom.max(1);
            let (x0, y0, x1, y1) = (x0 - 2, y0 - 2, x1 + z + 1, y1 + z + 1);
            hc.fill_rect(frame_buf, x0, y0, x1 - x0 + 1, 1, ring);
            hc.fill_rect(frame_buf, x0, y1, x1 - x0 + 1, 1, ring);
            hc.fill_rect(frame_buf, x0, y0, 1, y1 - y0 + 1, ring);
            hc.fill_rect(frame_buf, x1, y0, 1, y1 - y0 + 1, ring);
        }
    }
    // With an egg's worth banked outside the nest, a "LAY HERE" tag over the
    // nearest door says where L works (the ticker's refusal alone was a
    // sentence to remember while steering).
    if let Some((_, s)) = body.filter(|_| st.stakes && st.ended.is_none()) {
        let head = s.chain.first().copied().unwrap_or((0, 0));
        if s.energy >= EGG_J + EGG_KEEP_J && !in_nest(w, head) {
            if let Some((sx, sy)) = nearest_door(w, head).and_then(|(nx, ny)| lab.renderer.world_to_screen(nx, ny - NEST_REACH / 2)) {
                hc.fill_rect(frame_buf, sx - 26, sy - 12, 53, 9, [0, 0, 0, 200]);
                hc.text(frame_buf, sx - 25, sy - 11, "LAY HERE", [255, 240, 150, 255]);
            }
        }
    }
    if st.stakes {
        return draw_visit_panel(lab, st, v, hc, frame_buf, (g, ring), body.map(|(_, s)| s));
    }
    // The ledger, top left under the clock. Rows kept under 34 glyphs so
    // the panel stays 210 logical pixels wide.
    let c = v.counts;
    let (x, mut y) = (8, 40);
    let row = |frame_buf: &mut [u8], y: &mut i32, text: String, colour: [u8; 4]| {
        hc.text(frame_buf, x, *y, &text, colour);
        *y += 9;
    };
    hc.blend_rect(frame_buf, x - 4, y - 4, 214, 9 * 9 + 6, [0, 0, 0, 255], 0.6);
    let (energy, cells, cap) = body.map_or((0.0, 0, 1.0), |(_, s)| (s.energy, s.chain.len(), w.species.creature_shared(s.species).map_or(200.0, |d| d.start_energy)));
    let life = match st.died_at {
        Some(_) => "DEAD -- BACK IN 3 S".to_string(),
        None => format!("ALIVE {}", clock(frame.saturating_sub(st.born))),
    };
    row(frame_buf, &mut y, format!("GUEST: ROVE {}  {life}", st.lives), [240, 200, 140, 255]);
    // The smell bar, on the same ramp as the ring: full is family.
    let bar = 64;
    let word = if !g.is_finite() {
        "?"
    } else if g < 0.5 {
        "FAMILY"
    } else if g < 0.8 {
        "DOUBTFUL"
    } else {
        "STRANGER"
    };
    hc.text(frame_buf, x, y, "SMELL", [220, 220, 220, 255]);
    hc.fill_rect(frame_buf, x + 40, y + 1, bar, 5, [40, 40, 40, 255]);
    let fill = if g.is_finite() { ((1.0 - (g / 1.6).clamp(0.0, 1.0)) * bar as f32) as i32 } else { 0 };
    hc.fill_rect(frame_buf, x + 40, y + 1, fill.max(1), 5, ring);
    hc.text(frame_buf, x + 44 + bar, y, &format!("{g:.2} {word}"), ring);
    y += 9;
    hc.text(frame_buf, x, y, "ENERGY", [220, 220, 220, 255]);
    hc.fill_rect(frame_buf, x + 40, y + 1, bar, 5, [40, 40, 40, 255]);
    hc.fill_rect(frame_buf, x + 40, y + 1, ((energy / (2.0 * cap)).clamp(0.0, 1.0) * bar as f32) as i32, 5, [230, 200, 80, 255]);
    hc.text(frame_buf, x + 44 + bar, y, &format!("{energy:.0}  CELLS {cells}"), [220, 220, 220, 255]);
    y += 9;
    row(frame_buf, &mut y, format!("FED {} ({:.0} J) NO {} EMPTY {}", c.fed, c.fed_j, c.refused, c.empty), [120, 240, 130, 255]);
    row(frame_buf, &mut y, format!("? {}  ! {}  NIPS {}  LOST {}", c.antennated, c.displayed, c.nipped, c.cells_lost), [255, 200, 90, 255]);
    row(frame_buf, &mut y, format!("RUBS {} SOAK {} LARVAE {} ATE {:.0}", c.rubs, if st.soak { "ON" } else { "OFF" }, c.larvae_killed, c.eaten_j), [160, 210, 255, 255]);
    if let Some(host) = st.host_colony {
        let (a, b) = colony_size(w, host);
        row(frame_buf, &mut y, format!("COLONY {a} ANTS {b} BROOD (WAS {})", st.arrival.0), [210, 210, 210, 255]);
    }
    let now = st.died_at.map_or(frame.saturating_sub(st.born), |_| 0);
    row(frame_buf, &mut y, format!("BEST {}  GRADE {}", clock(st.best_life.max(now)), grade(&c)), [255, 255, 255, 255]);
}

/// The smell word the bar prints.
fn smell_word(g: f32) -> &'static str {
    if !g.is_finite() {
        "?"
    } else if g < 0.5 {
        "FAMILY"
    } else if g < 0.8 {
        "DOUBTFUL"
    } else {
        "STRANGER"
    }
}

/// A labelled bar: `fill` in `[0, 1]`, then `text` after it.
fn bar_row(hc: crate::render::Hud, frame_buf: &mut [u8], (x, y): (i32, i32), label: &str, fill: f32, colour: [u8; 4], text: &str) {
    let bar = 64;
    hc.text(frame_buf, x, y, label, [220, 220, 220, 255]);
    hc.fill_rect(frame_buf, x + 40, y + 1, bar, 5, [40, 40, 40, 255]);
    hc.fill_rect(frame_buf, x + 40, y + 1, ((fill.clamp(0.0, 1.0) * bar as f32) as i32).max(1), 5, colour);
    hc.text(frame_buf, x + 44 + bar, y, text, colour);
}

/// **The visit panel** (stakes on), top left under the clock where the
/// sandbox's ledger sat, seven rows so it stops above the mound: roves and
/// time left, smell, the colony's wariness, energy against an egg's price,
/// gifts and nips, rubs and eggs, and the grade and score. When the visit
/// has ended it is the end card instead.
fn draw_visit_panel(lab: &Lab, st: &GuestDemo, v: &guest::Visitors, hc: crate::render::Hud, frame_buf: &mut [u8], (g, ring): (f32, [u8; 4]), body: Option<&crate::sim::organism::OrganismState>) {
    let w = &lab.world;
    let frame = w.frame;
    let c = v.counts;
    let (x, mut y) = (8, 40);
    let sc = score_now(lab);
    if let Some((_, why, sc)) = st.ended {
        let rows = 8;
        hc.blend_rect(frame_buf, x - 4, y - 4, 214, 9 * rows + 6, [0, 0, 0, 255], 0.75);
        hc.fill_rect(frame_buf, x - 4, y - 4, 214, 1, [255, 220, 120, 255]);
        let mut row = |text: String, colour: [u8; 4]| {
            hc.text(frame_buf, x, y, &text, colour);
            y += 9;
        };
        let what = if why == VisitEnd::TimeUp { "TIME UP" } else { "NO ROVES LEFT" };
        let eggs = st.eggs.len();
        let brood = st.eggs.iter().filter(|e| e.brood).count();
        row(format!("VISIT OVER: {what}"), [255, 220, 120, 255]);
        row(format!("SCORE {}", sc.total()), [255, 255, 255, 255]);
        row(format!("ALIVE {:>14}  {:>4}", clock(u64::from(sc.alive_s) * 60), sc.alive_s), [220, 220, 220, 255]);
        row(format!("EGGS {eggs} ({brood} IN BROOD) {:>8}", sc.egg_points), [245, 235, 205, 255]);
        row(format!("ENERGY LEFT / 10 {:>11}", sc.energy_points), [230, 200, 80, 255]);
        row(format!("FED {} ({:.0} J) NO {} NIPS {}", c.fed, c.fed_j, c.refused, c.nipped), [120, 240, 130, 255]);
        row(format!("ROVES LOST {}  GRADE {}", st.lives - u32::from(st.died_at.is_none()), grade_now(lab)), [255, 200, 90, 255]);
        row("K: A NEW VISIT".to_string(), [160, 210, 255, 255]);
        return;
    }
    hc.blend_rect(frame_buf, x - 4, y - 4, 214, 9 * 7 + 6, [0, 0, 0, 255], 0.6);
    let left = st.visit_start.map_or(VISIT_FRAMES, |t| (t + VISIT_FRAMES).saturating_sub(frame));
    let life = match st.died_at {
        Some(_) => "DEAD".to_string(),
        None => format!("ALIVE {}", clock(frame.saturating_sub(st.born))),
    };
    hc.text(frame_buf, x, y, &format!("ROVE {}/{LIVES} {life}  LEFT {}", st.lives, clock(left)), [240, 200, 140, 255]);
    y += 9;
    let smell = if g.is_finite() { 1.0 - (g / 1.6).clamp(0.0, 1.0) } else { 0.0 };
    bar_row(hc, frame_buf, (x, y), "SMELL", smell, ring, &format!("{g:.2} {}", smell_word(g)));
    y += 9;
    let wary = st.body.map_or(0.0, |id| wary_near(w, id));
    let wcol = gap_colour(wary.min(1.0) * 1.4);
    let wword = if wary >= WARY_WARN { "ONTO YOU" } else if wary >= 0.5 * WARY_WARN { "WATCHING" } else { "CALM" };
    bar_row(hc, frame_buf, (x, y), "WARY", wary, wcol, &format!("{wary:.2} {wword}"));
    y += 9;
    let (energy, cells) = body.map_or((0.0, 0), |s| (s.energy, s.chain.len()));
    let need = EGG_J + EGG_KEEP_J;
    let ready = energy >= need;
    bar_row(hc, frame_buf, (x, y), "ENERGY", energy / need, if ready { [255, 240, 150, 255] } else { [230, 200, 80, 255] }, &format!("{energy:.0} C{cells}{}", if ready { " L READY" } else { "" }));
    y += 9;
    let mut row = |text: String, colour: [u8; 4]| {
        hc.text(frame_buf, x, y, &text, colour);
        y += 9;
    };
    row(format!("FED {} ({:.0} J) NO {} NIPS {}", c.fed, c.fed_j, c.refused, c.nipped), [120, 240, 130, 255]);
    row(format!("RUBS {} SOAK {} EGGS {}", c.rubs, if st.soak { "ON" } else { "OFF" }, st.eggs.len()), [160, 210, 255, 255]);
    row(format!("GRADE {}  SCORE {}", grade_now(lab), sc.total()), [255, 255, 255, 255]);
}

/// **The grade now** (stakes), from the last 30 s: it moves both ways.
pub fn grade_now(lab: &Lab) -> &'static str {
    let Some(st) = state(lab) else { return "?" };
    let w = &lab.world;
    let Some(v) = w.visitors.as_ref() else { return "?" };
    let frame = w.frame;
    let recent = |at: u64| frame.saturating_sub(at) <= GRADE_WINDOW;
    let gifts = v.events.iter().filter(|e| e.rung == Rung::Fed && e.ant != 0 && recent(e.frame)).count();
    let nips = v.events.iter().filter(|e| e.rung == Rung::Nip && recent(e.frame)).count();
    let g = st.body.and_then(|id| v.list.iter().find(|x| x.id == id)).map_or(f32::NAN, |x| x.gap);
    let wary = st.body.map_or(0.0, |id| wary_near(w, id));
    grade_of(g, wary, gifts, nips, st.last_kill.is_some_and(recent), st.last_eat.is_some_and(recent))
}

/// **The visit's grade** from what the last 30 s held: a predator or thief
/// if you took food unasked, a stranger while you smell wrong, a pest while
/// the colony is onto you, a cuckoo while it feeds you, and a guest between.
pub fn grade_of(gap: f32, wary: f32, gifts: usize, nips: usize, killed: bool, ate: bool) -> &'static str {
    if killed {
        "PREDATOR"
    } else if ate {
        "THIEF"
    } else if !gap.is_finite() || gap >= 0.8 {
        "STRANGER"
    } else if wary >= WARY_WARN || nips >= 2 {
        "PEST"
    } else if gifts >= 2 {
        "CUCKOO"
    } else {
        "GUEST"
    }
}

/// **The grade of the guest's career** (sandbox, `stakes=0`), from where its
/// food came from: a cuckoo lives on gifts, a thief on the dead, a predator
/// on the brood. It never goes down (review §2), which is why stakes use
/// [`grade_now`].
pub fn grade(c: &guest::VisitCounts) -> &'static str {
    if c.larvae_killed > 0 && c.eaten_j > c.fed_j {
        "PREDATOR"
    } else if c.eaten_j > c.fed_j {
        "THIEF"
    } else if c.fed > 0 {
        "CUCKOO"
    } else {
        "STRANGER"
    }
}

#[cfg(test)]
mod tests {
    use super::super::{DemoKey, DemoSeat};
    use super::*;

    /// **The seat end to end, without a window**: `demo=guest` runs the
    /// trained colony, lands a registered rove (never an ant) as the player's
    /// body at `land=`, sends R and Y to the ladder and B away from the body's
    /// bite, and lands a new rove of the same guest colony three seconds after
    /// the last one dies. The positive control for "the key reached the
    /// ladder" is the latch, which only `seat_key` sets and `guest::tick`
    /// clears.
    #[test]
    fn the_guest_lands_takes_its_keys_and_comes_back_after_dying() {
        let mut lab = Lab::new(crate::lab::scene::LabBox::default());
        let mut args = std::collections::BTreeMap::new();
        args.insert("land".to_string(), "1500".to_string());
        lab.start_demo_with(DemoSeat::Guest, args);
        lab.demo_frame();
        lab.tick_for_harness();
        assert!(lab.new_ant, "the guest demo did not turn the trained colony on");
        assert!(steer::steered_id(&lab.world).is_none(), "a body was bound before the rove landed");
        while lab.world.frame < 1_501 {
            lab.demo_frame();
            lab.tick_for_harness();
        }
        lab.demo_frame();
        let rove = steer::steered_id(&lab.world).expect("no rove landed at land=");
        let rove_species = lab.world.species.id_of("rove").expect("rove ships");
        assert_eq!(lab.world.organism(rove).map(|s| s.species), Some(rove_species), "the player's body is not the rove");
        assert!(guest::is_visitor(&lab.world, rove), "the rove was not registered with the ladder");
        assert_eq!(lab.time.multiple(), 1, "the clock did not drop to 1x at the landing");
        let colony = lab.world.colony_of(rove);
        assert!(lab.demo_key(DemoKey::Letter('R'), true) && lab.demo_key(DemoKey::Letter('Y'), true));
        assert!(lab.demo_key(DemoKey::Letter('B'), true), "B went to the body's bite, not the seat");
        let input = lab.world.visitors.as_ref().expect("on").input;
        assert!(input.rub && input.beg && input.kill, "the seat's keys did not reach the ladder: {input:?}");
        assert!(!lab.world.steered.as_ref().expect("bound").latched.bite, "B also reached the rove's own bite");
        lab.tick_for_harness();
        let input = lab.world.visitors.as_ref().expect("on").input;
        assert!(!input.rub && !input.beg && !input.kill, "a press outlived its tick");
        // Death and the respawn.
        // Starved outright: it dies at its next decision.
        lab.world.organism_mut(rove).expect("alive").energy = -1.0e6;
        for _ in 0..30 {
            lab.demo_frame();
            lab.tick_for_harness();
            if lab.world.organism(rove).is_none() {
                break;
            }
        }
        assert!(lab.world.organism(rove).is_none(), "test setup: the starved rove did not die");
        let died = lab.world.frame;
        while lab.world.frame < died + RESPAWN + 2 {
            lab.demo_frame();
            lab.tick_for_harness();
        }
        lab.demo_frame();
        let next = steer::steered_id(&lab.world).expect("no rove came back after the respawn time");
        assert_ne!(next, rove);
        assert_eq!(lab.world.colony_of(next), colony, "the respawn joined another colony");
        assert_eq!(state(&lab).map(|s| s.lives), Some(2));
    }

    /// **A visit with stakes is three roves, then a card** (2026-10-30): each
    /// death before the last brings a rove back, the third ends the visit
    /// (no fourth lands), the score freezes, the drain bills the living
    /// rove faster than its body alone, and an egg's points count. The
    /// positive control for "no fourth" is the first two respawns, and
    /// `stakes=0` must still respawn past three (the sandbox).
    #[test]
    fn a_visit_is_three_roves_then_a_card_and_the_sandbox_is_not() {
        for stakes in ["1", "0"] {
            let mut lab = Lab::new(crate::lab::scene::LabBox::default());
            let mut args = std::collections::BTreeMap::new();
            args.insert("land".to_string(), "1500".to_string());
            args.insert("stakes".to_string(), stakes.to_string());
            lab.start_demo_with(DemoSeat::Guest, args);
            while steer::steered_id(&lab.world).is_none() {
                lab.demo_frame();
                lab.tick_for_harness();
                assert!(lab.world.frame < 2_200, "no rove landed");
            }
            let on = stakes == "1";
            assert_eq!(state(&lab).map(|s| s.stakes), Some(on));
            assert_eq!(lab.world.visitors.as_ref().map(|v| v.wary.is_some()), Some(on), "wariness is not tied to the stakes");
            if on {
                // The drain: over 120 ticks the rove loses at least the
                // seat's share on top of whatever its body burns.
                let rove = steer::steered_id(&lab.world).expect("landed");
                let e0 = lab.world.organism(rove).expect("alive").energy;
                for _ in 0..120 {
                    lab.demo_frame();
                    lab.tick_for_harness();
                }
                let e1 = lab.world.organism(rove).map_or(e0, |s| s.energy);
                assert!(e0 - e1 >= 2.0 * DRAIN_J_PER_S - 0.1, "the drain did not bill the rove: {e0} -> {e1}");
                if let Some(s) = state_mut(&mut lab) {
                    s.eggs.push(Egg { at: (0, 0), frame: 0, brood: false });
                }
                assert!(score_now(&lab).egg_points == EGG_POINTS && score_now(&lab).alive_s >= 2, "{:?}", score_now(&lab));
            }
            for death in 1..=4u32 {
                let Some(rove) = steer::steered_id(&lab.world).filter(|&id| lab.world.organism(id).is_some()) else {
                    assert!(on && death == 4, "stakes={stakes}: no rove for death {death}");
                    break;
                };
                lab.world.organism_mut(rove).expect("alive").energy = -1.0e6;
                let until = lab.world.frame + RESPAWN + 60;
                while lab.world.frame < until {
                    lab.demo_frame();
                    lab.tick_for_harness();
                }
                lab.demo_frame();
                let lives = state(&lab).map_or(0, |s| s.lives);
                let ended = state(&lab).and_then(|s| s.ended);
                if on && death >= LIVES {
                    assert_eq!(lives, LIVES, "a fourth rove landed with stakes on");
                    assert_eq!(ended.map(|e| e.1), Some(VisitEnd::OutOfRoves), "the third death did not end the visit");
                    let frozen = score_now(&lab);
                    for _ in 0..120 {
                        lab.tick_for_harness();
                    }
                    assert_eq!(score_now(&lab), frozen, "the score moved after the visit ended");
                } else {
                    assert_eq!(lives, death + 1, "stakes={stakes}: death {death} brought no rove back");
                    assert!(ended.is_none(), "stakes={stakes}: the visit ended early");
                }
            }
        }
    }

    /// **The grade moves both ways** (review §2: it locked on the first gift).
    #[test]
    fn the_grade_moves_both_ways() {
        assert_eq!(grade_of(1.2, 0.0, 0, 0, false, false), "STRANGER");
        assert_eq!(grade_of(0.3, 0.0, 3, 0, false, false), "CUCKOO");
        assert_eq!(grade_of(0.3, 0.7, 3, 0, false, false), "PEST", "a fed guest the colony is onto is a pest");
        assert_eq!(grade_of(0.3, 0.1, 0, 0, false, false), "GUEST", "thirty quiet seconds later the cuckoo is a guest again");
        assert_eq!(grade_of(0.3, 0.1, 3, 0, true, false), "PREDATOR");
    }

    #[test]
    fn the_smell_ramp_runs_green_to_red_and_never_blends() {
        let (fam, mid, far) = (gap_colour(0.0), gap_colour(0.9), gap_colour(2.0));
        assert!(fam[1] > fam[0] && far[0] > far[1] && mid[0] > mid[2], "{fam:?} {mid:?} {far:?}");
        assert_eq!(gap_colour(1.4), gap_colour(9.0), "the ramp is not fixed past its end");
    }
}
