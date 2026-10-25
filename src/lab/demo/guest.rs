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

use crate::sim::cell::OrganismId;
use crate::sim::creature::guest::{self, Rung};
use crate::sim::organism::SpeciesId;
use crate::sim::scheduler::ActiveKind;
use crate::sim::steer::{self, SteerInput, SteerRole};
use crate::sim::world::World;

use super::super::Lab;

/// The guest seat's own keys, as the help line prints them.
pub const SEAT_KEYS: &str = "R RUB AN ANT, P SOAK (ON/OFF), Y BEG, B KILL A LARVA, E EAT A CORPSE, J JUMP";

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
}

impl GuestDemo {
    pub fn new(args: &std::collections::BTreeMap<String, String>) -> GuestDemo {
        GuestDemo {
            land: args.get("land").and_then(|v| v.parse().ok()).unwrap_or(LAND_DEFAULT),
            gap: args.get("gap").and_then(|v| v.parse().ok()).unwrap_or(GAP_DEFAULT),
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
    let st = state_mut(lab)?;
    if st.lives == 0 {
        st.arrival = arrival;
    }
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
pub fn seat_key(lab: &mut Lab, key: char) {
    if key == 'J' {
        if let Some(s) = lab.world.steered.as_mut() {
            s.press(SteerInput { jump: true, ..SteerInput::default() });
        }
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
    matches!(key, 'B' | 'E')
}

/// Once a displayed frame, after the body has its keys.
pub fn frame(_lab: &mut Lab) {}

/// **Before each simulated tick**: force the trained colony on, run the
/// warm-up at top speed, land the first rove at `land`, and a new one
/// [`RESPAWN`] frames after each death.
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
    let due = match (st.lives, st.died_at) {
        (0, _) => frame >= st.land,
        (_, Some(d)) => frame >= d + RESPAWN,
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
        let gap = state(lab).map_or(0.0, |s| s.gap);
        lab.ui.say(format!("YOU LAND SMELLING {gap:.1} OFF: RUB ANTS (R), SOAK (P), BEG (Y)"));
    }
}

/// **After each simulated tick**: notice a death and start the respawn clock.
pub fn after_tick(lab: &mut Lab) {
    let Some(st) = state(lab) else { return };
    let Some(body) = st.body else { return };
    if st.died_at.is_some() || lab.world.organism(body).is_some() {
        return;
    }
    let frame = lab.world.frame;
    let life = frame.saturating_sub(st.born);
    if let Some(s) = state_mut(lab) {
        s.died_at = Some(frame);
        s.best_life = s.best_life.max(life);
    }
    lab.ui.say(format!("THE COLONY KILLED YOU AFTER {} -- ANOTHER ROVE LANDS IN 3 S", clock(life)));
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

/// **The grade of the guest's career**, from where its food came from:
/// a cuckoo lives on gifts, a thief on the dead, a predator on the brood.
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

    #[test]
    fn the_smell_ramp_runs_green_to_red_and_never_blends() {
        let (fam, mid, far) = (gap_colour(0.0), gap_colour(0.9), gap_colour(2.0));
        assert!(fam[1] > fam[0] && far[0] > far[1] && mid[0] > mid[2], "{fam:?} {mid:?} {far:?}");
        assert_eq!(gap_colour(1.4), gap_colour(9.0), "the ramp is not fixed past its end");
    }
}
