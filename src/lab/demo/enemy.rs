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
}

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
    let far = lab.demo.as_ref().and_then(|d| d.args.get("land")).and_then(|v| v.parse().ok()).unwrap_or(LAND_FROM_DOOR);
    let x = if door - far > x0 + 8 { door - far } else { door + far };
    let id = crate::train::predator::place_predator(&mut lab.world, "centipede", x, None, minds[0].clone(), 1.0)?;
    lab.world.steered = None;
    steer::mark_with(&mut lab.world, id, Some(steer::SteerInput::default()), steer::SteerRole::Worker);
    if let Some(s) = lab.world.steered.as_mut() {
        s.may_breed = false;
    }
    lab.world.encounter_log = Some(Default::default());
    let (trips, ants) = (lab.world.creature_stats.trip_deliveries, live_ants(&lab.world));
    let state = lab.world.organism(id)?;
    let (len, energy) = (state.chain.len(), state.energy);
    let start = lab.world.species.get(state.species).creature.as_ref().map_or(1.0, |d| d.start_energy);
    let frame = lab.world.frame;
    let e = lab.demo.as_mut()?.enemy.as_mut()?;
    *e = EnemyDemo {
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

/// One of the seat keys was pressed.
pub fn seat_key(lab: &mut Lab, key: char) {
    match key {
        'Y' => {
            if let Some(e) = lab.demo.as_mut().and_then(|d| d.enemy.as_mut()) {
                e.card = !e.card;
            }
        }
        'R' => {
            let ready = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).is_some_and(|e| e.ended.is_some());
            if ready && land_raider(lab).is_some() {
                lab.ui.say("A NEW RAIDER ON THE SURFACE -- THE COLONY IS AS YOU LEFT IT");
            }
        }
        _ => {}
    }
}

/// Once a displayed frame: the warm-up's speed, then the landing.
pub fn frame(lab: &mut Lab) {
    let Some(e) = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()) else { return };
    if e.raider.is_some() {
        return;
    }
    // `warm=` moves the landing (a test, or a player who wants straight in).
    let warm = lab.demo.as_ref().and_then(|d| d.args.get("warm")).and_then(|v| v.parse().ok()).unwrap_or(WARM_UNTIL);
    if lab.world.frame < warm {
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
        lab.ui.say(format!("YOU ARE THE RAIDER -- {}", SEAT_KEYS));
    }
}

/// Before each simulated tick.
pub fn before_tick(_lab: &mut Lab) {}

/// **After each simulated tick**: read what happened to and by the raider from
/// the world's own logs.
pub fn after_tick(lab: &mut Lab) {
    let Some(id) = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).and_then(|e| e.raider) else { return };
    let frame = lab.world.frame;
    let w = &mut lab.world;
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
        // A cell came off: the one at `len` in the old chain (everything behind
        // a severed neck goes with it, so the bitten one is the first missing).
        e.cells_lost.push((frame, len));
    }
    if !alive && e.ended.is_none() {
        if !starved {
            e.cells_lost.push((frame, 0));
        }
        e.ended = Some((frame, if starved { "YOU STARVED" } else { "THEY BIT YOUR HEAD OFF" }));
        e.card = true;
    }
    if alive {
        e.last_len = len;
        e.last_energy = energy;
        e.last_gnawed = gnawed;
    }
    if let Some(a) = last_act.filter(|a| a.frame > e.last_act_read) {
        e.last_act_read = a.frame;
        let why = match a.asked {
            Some(Verb::Attack) if a.done == Verb::Attack && !a.ok => Some(if crop_full { "YOUR GUT IS FULL -- SPIT A CELL (Q) OR WAIT" } else { "THEIR PLATE HELD" }),
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
    e.marks.retain(|&(f, _, m)| frame.saturating_sub(f) < if m == Mark::Kill { RING_FRAMES } else { PULSE_FRAMES });
}

/// The pointer over the world: the lab's own tools keep it.
pub fn mouse(_lab: &mut Lab, _cell: (i32, i32), _act: super::MouseAct) -> bool {
    false
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
    for site in &lab.world.nest_sites {
        let top_y = lab.world.room_surface_at(site.x).unwrap_or(site.y);
        let (cx, cy) = at(site.x, top_y - 3);
        for k in 0..3 {
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
    }
}

const DOOR: [u8; 4] = [120, 220, 255, 255];
const HEAD: [u8; 4] = [255, 110, 170, 255];
const BODY: [u8; 4] = [200, 40, 110, 255];
const YOU: [u8; 4] = [255, 255, 255, 255];
const YOU_BIT: [u8; 4] = [255, 220, 60, 255];
const BIT_YOU: [u8; 4] = [255, 60, 60, 255];
const KILL: [u8; 4] = [255, 120, 40, 255];

/// The HUD's lines, as plain text: what the panel draws and what a headless
/// harness prints, from one function.
pub fn hud_lines(lab: &Lab) -> Vec<String> {
    let Some(e) = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()) else { return Vec::new() };
    let w = &lab.world;
    let brain = if lab.new_ant { "TRAINED" } else { "HAND-BUILT" };
    if e.landed.is_none() {
        return vec![format!("THE COLONY IS DIGGING IN: {} / {WARM_UNTIL}  ({brain} ANTS)", w.frame.min(WARM_UNTIL))];
    };
    let mut out = Vec::new();
    let ants = live_ants(w);
    let trips = w.creature_stats.trip_deliveries.saturating_sub(e.trips_at_landing);
    if let Some(s) = e.raider.and_then(|id| w.organism(id)) {
        let quarters = (s.gnawed.clamp(0.0, 1.0) * 4.0).floor() as usize;
        out.push(format!(
            "RAIDER {} CELLS  WOUND {}{}  A FULL WOUND TAKES THE BITTEN CELL; THE HEAD KILLS",
            s.chain.len(),
            "*".repeat(quarters),
            "-".repeat(4 - quarters.min(4))
        ));
        let def = w.species.get(s.species).creature.as_ref();
        let (cells, cap) = match (s.crop, def) {
            (Some(c), Some(d)) => (c.cells as i32, (crate::sim::creature::organism_crop_capacity(w, e.raider.unwrap_or(0), d) / c.unit.max(1e-6)).floor() as i32),
            (None, Some(_)) => (0, 0),
            _ => (0, 0),
        };
        let tank = (s.energy / e.start_energy.max(1.0)).clamp(0.0, 1.0);
        let bars = (tank * 10.0).round() as usize;
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

/// **The season card**: what you did and what it cost them, from the counts.
pub fn card_lines(lab: &Lab) -> Vec<String> {
    let Some(e) = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()) else { return Vec::new() };
    let Some(landed) = e.landed else { return Vec::new() };
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
        if per > 100.0 {
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
    .chain(lab.new_ant.then(|| ["THIS COLONY FIGHTS BACK BUT DOES NOT LEARN YOU:".to_string(), "ITS YOUNG ARE BORN WITH THEIR PARENTS' MINDS.".to_string()]).into_iter().flatten())
    .collect()
}

/// The seat's panel: the HUD under the clock, and the card when it is up.
pub fn draw_hud(lab: &Lab, hc: crate::render::Hud, frame_buf: &mut [u8]) {
    let Some(e) = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()) else { return };
    let white = [235u8, 235, 235, 255];
    let warn = [255u8, 200, 90, 255];
    for (i, line) in hud_lines(lab).iter().enumerate() {
        let colour = if i >= 4 { warn } else { white };
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
    fn quick_lab(warm: &str) -> Lab {
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
        lab.demo = Some(demo);
        lab.load_scenario(sc);
        lab
    }

    /// **Off is off, and the warm-up is the plain box**: until the raider
    /// lands the seat only turns the speed dial, so the box under it is the
    /// box with no demo, cell for cell, over 3,000 frames.
    #[test]
    fn the_box_is_untouched_until_the_raider_lands() {
        let mut with = quick_lab("1000000000");
        let mut without = quick_lab("0");
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
        let mut lab = quick_lab("600");
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
}
