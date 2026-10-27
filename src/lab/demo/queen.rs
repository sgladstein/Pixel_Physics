//! **The queen seat's demo** (`demo=queen`): the player is the founding
//! queen. The design is `Reports/teaching-the-colony-2026-10-25/queen.md`
//! and the teaching plan's sections 3-6; the demo's own plan, its two
//! reviews and the measurements that changed it are in
//! `Reports/queen-seat-demo-2026-10-25.md`.
//!
//! What the player does, and what each verb leaves on screen:
//!
//! - **Found**: steer her (arrows), eat (E), dig (G), lay (L). Slice 0's
//!   verbs, on the hand-built path (`creature_tick`'s hand-off for the
//!   queen's body).
//! - **Fathers** (`sim::fathers`): she carries the 30 shipped minds as
//!   fathers, and every egg of hers draws one. The 30 are near-copies
//!   (measured, M0a), so they are one line on screen: **the stock**.
//! - **Gamble (P)**: her next egg is wild, a mutated copy of the father it
//!   drew; P again before she lays makes it feral (three and ten times
//!   training's mutation width, M0b). A wild egg founds a new named line in
//!   its own colour, on the egg, the larva and every ant it becomes.
//! - **Keep (Y)**: her next five eggs carry the line of the ant clicked with
//!   LOOK, or of the ledger row under the cursor (K).
//! - **The ledger** (bottom left): each line's adults and brood and its rate
//!   at one deed (J cycles digs, trips) per 1,000 frames alive.
//! - **The hold**: a bar drains between her eggs. Empty, the queen rule
//!   lapses and workers lay their own lines; her next egg takes it back
//!   (`steer::holding_queen`).

use std::sync::Arc;

use crate::sim::cell::OrganismId;
use crate::sim::creature::mind::{self, MindGenome};
use crate::sim::fathers::{Father, Fathers, Store};
use crate::sim::organism::BroodStage;
use crate::sim::steer;
use crate::sim::world::World;
use crate::train::challenges::colony_bed;

use super::super::Lab;

/// The queen seat's own keys, as the help line prints them.
pub const SEAT_KEYS: &str = "P WILD EGG (AGAIN: FERAL) -- Y NEXT 5 EGGS HER LINE (LOOK-CLICK OR K) -- J DEED -- K ROW -- U COLOURS";

/// **Her hold, in frames past her last egg.** Provisional until the scripted
/// founding (`examples/queendemo.rs`, M0c) measures her lay interval; the
/// plan sets it at that interval's p90 x 1.5.
pub const HOLD: u64 = 6_000;

/// Joules granted the frame she is marked: a founder lands with 200 J and the
/// lay bar is 1,100 J, so a queen of one reaches her first egg without a
/// blind forage first.
pub const RESERVE: f32 = 900.0;

/// **The zoom she opens at**: 4 screen pixels a cell puts her chamber and
/// the first galleries (about 128x80 cells) in the window, where an ant is
/// a dozen pixels and its line colour can be told from the soil.
pub const NEST_ZOOM: i32 = 4;

/// How many eggs one Y keeps.
pub const KEEP_EGGS: usize = 5;

/// An ant counts in her line's rate from this age on, so a line of newborns
/// is not ranked on a few frames of luck.
const MIN_AGE: u64 = 1_000;

/// The two wild widths, `(pm, sigma, events)`, from M0b: mild is three times
/// training's own mutation (2 jewels, 11 broken foragers of 20), feral ten
/// times (2 jewels, 17 broken -- visible diggers that never forage).
pub const WILD: [(f32, f32, usize); 2] = [(0.1, 1.0, 30), (0.3, 2.0, 100)];

/// A wild line's colours, cycled: bright hues the box's soil, grass and the
/// ant's own brown do not use.
const PALETTE: [[f32; 3]; 10] = [
    [255.0, 80.0, 80.0],
    [80.0, 200.0, 255.0],
    [255.0, 220.0, 60.0],
    [190.0, 110.0, 255.0],
    [90.0, 255.0, 140.0],
    [255.0, 140.0, 220.0],
    [255.0, 160.0, 40.0],
    [140.0, 255.0, 255.0],
    [220.0, 255.0, 90.0],
    [255.0, 255.0, 255.0],
];

/// Which deed the ledger ranks by (`J`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Deed {
    #[default]
    Digs,
    Trips,
}

impl Deed {
    fn name(self) -> &'static str {
        match self {
            Deed::Digs => "DIGS",
            Deed::Trips => "TRIPS",
        }
    }
}

/// One line: the stock (index 0, every mind not a wild line's), or a wild
/// egg's new mind and every ant that inherits it.
#[derive(Clone, Debug)]
pub struct Line {
    /// `None` for the stock.
    pub mind: Option<Arc<MindGenome>>,
    pub name: String,
    pub colour: Option<[f32; 3]>,
    /// The wild level it was made at (0: the stock).
    pub wild: u8,
    pub born: u64,
    /// Living adults and brood at the last count, and whether it ever had any.
    pub adults: u32,
    pub brood: u32,
    pub seen: bool,
    pub gone: bool,
    /// Deeds and frames alive over its adults past `MIN_AGE`.
    pub digs: u64,
    pub trips: u64,
    pub frames: u64,
}

impl Line {
    fn rate(&self, deed: Deed) -> Option<f64> {
        let n = match deed {
            Deed::Digs => self.digs,
            Deed::Trips => self.trips,
        };
        (self.frames > 0).then(|| n as f64 * 1000.0 / self.frames as f64)
    }
}

/// What the demo has counted, for the HUD and the scripted player.
#[derive(Clone, Copy, Debug, Default)]
pub struct Tally {
    pub eggs: u32,
    pub wild_eggs: u32,
    pub kept_eggs: u32,
    pub worker_eggs: u32,
    pub lapses: u32,
    pub returns: u32,
    pub workers: u32,
    pub brood: u32,
    pub hungry_larvae: u32,
}

/// The queen seat's running state.
#[derive(Clone, Debug, Default)]
pub struct QueenDemo {
    pub ready: bool,
    pub colony: u32,
    pub lines: Vec<Line>,
    pub deed: Deed,
    pub cursor: usize,
    pub tint_off: bool,
    pub tally: Tally,
    /// `Fathers::log` entries already read.
    log_seen: usize,
    /// Whether she held at the last look, for the lapse/return notices.
    held: Option<bool>,
    /// The world frame the ledger was last counted on.
    counted: u64,
    /// `creature_stats.buds_held_for_nest` at the last tick, and the frame
    /// the "no room" notice last showed.
    held_for_nest: u64,
    no_room_said: u64,
}

/// **Which ant is the queen**: the oldest ant in the box. The demo's
/// scenario lands one founder alone, so this is her.
pub fn pick_body(world: &World) -> Option<OrganismId> {
    super::oldest_ant(world)
}

/// The wild mutation at a level (1 mild, 2 feral), for `Fathers::mutate`.
pub fn mutate_level(g: &[f32], level: u8, seed: u64) -> Vec<f32> {
    let (pm, sigma, events) = WILD[usize::from(level.clamp(1, 2)) - 1];
    crate::lab::training::mutate_genome(g, pm, sigma, events, seed)
}

fn state(lab: &Lab) -> Option<&QueenDemo> {
    lab.demo.as_ref().and_then(|d| d.queen.as_ref())
}

fn state_mut(lab: &mut Lab) -> Option<&mut QueenDemo> {
    lab.demo.as_mut().and_then(|d| d.queen.as_mut())
}

/// **Arm the demo once her body is bound**: the 30 fathers, her own mind one
/// the store knows, the hold, the reserve. Every ant of the box thinks with
/// the same 30 minds (the lab's pool is set to the store's), so every line
/// the ledger names is one the store can rebuild.
fn arm(lab: &mut Lab) {
    if state(lab).is_none_or(|q| q.ready) {
        return;
    }
    let Some(id) = steer::steered_id(&lab.world) else { return };
    let Some(colony) = lab.world.steered.as_ref().map(|s| s.colony) else { return };
    let genomes = crate::lab::training::std_pool();
    let minds: Vec<Arc<MindGenome>> = genomes.iter().map(|g| colony_bed::mind(g)).collect();
    let mut f = Fathers::new(colony_bed::mind, mutate_level);
    let mut store = Store { p: 1.0, queen_only: true, ..Store::default() };
    for (m, g) in minds.iter().zip(genomes) {
        let g = Arc::new(g);
        f.know(m, &g);
        store.fathers.push(Father { mind: m.clone(), genome: g, weight: 1.0 });
    }
    f.colonies.insert(colony, store);
    lab.world.fathers = Some(Box::new(f));
    lab.set_new_ant_pool(minds.clone());
    mind::give(&mut lab.world, id, minds[id as usize % minds.len()].clone());
    if let Some(s) = lab.world.steered.as_mut() {
        s.hold = Some(HOLD);
        s.last_lay = lab.world.frame;
    }
    steer::grant(&mut lab.world, RESERVE);
    // The stock in the ant's own brown, so a wild line's colour is the only
    // colour in the nest (the lab opens on BY COLONY, which golds them all).
    lab.renderer.creature_colour = crate::render::CreatureColour::Off;
    // **In close on the nest** (hand-off to-do 1, 2026-10-27): at the box's
    // own scale an ant is 2-3 pixels and a wild line's colour reads as a
    // speck; the camera already follows her (`Demo::follow`), so only the
    // scale is set here. `-` pulls back.
    let bounds = lab.world.bounds();
    for _ in 0..8 {
        if lab.renderer.zoom >= NEST_ZOOM {
            break;
        }
        lab.renderer.zoom_within(1, (super::super::WIDTH, super::super::HEIGHT), bounds);
    }
    let frame = lab.world.frame;
    let q = state_mut(lab).expect("read above");
    q.ready = true;
    q.colony = colony;
    q.lines = vec![Line {
        mind: None,
        name: "STOCK".to_string(),
        colour: None,
        wild: 0,
        born: frame,
        adults: 0,
        brood: 0,
        seen: true,
        gone: false,
        digs: 0,
        trips: 0,
        frames: 0,
    }];
    lab.ui.say(format!("QUEEN DEMO -- YOU ARE THE FOUNDING QUEEN. EAT (E), DIG A CHAMBER (G), LAY (L). {SEAT_KEYS}"));
}

/// The line `m` belongs to: a wild line's index, or 0 (the stock).
fn line_of(lines: &[Line], m: &Arc<MindGenome>) -> usize {
    lines.iter().position(|l| l.mind.as_ref().is_some_and(|x| Arc::ptr_eq(x, m))).unwrap_or(0)
}

/// One of the seat keys (`Y U P J K R`) was pressed.
pub fn seat_key(lab: &mut Lab, key: char) {
    if state(lab).is_none_or(|q| !q.ready) {
        lab.ui.say("THE QUEEN HAS NOT LANDED YET -- RUN THE CLOCK (SPACE)");
        return;
    }
    let colony = state(lab).map_or(0, |q| q.colony);
    match key {
        'P' => {
            let Some(store) = lab.world.fathers.as_mut().and_then(|f| f.colonies.get_mut(&colony)) else { return };
            store.wild = (store.wild + 1) % 3;
            let say = match store.wild {
                1 => "NEXT EGG: WILD -- A NEW LINE, ONE ANT UNLESS KEPT (Y). P AGAIN: FERAL",
                2 => "NEXT EGG: FERAL -- A WIDE MUTATION: OFTEN BROKEN, NOW AND THEN A JEWEL. P AGAIN TO CANCEL",
                _ => "NEXT EGG: ORDINARY",
            };
            lab.ui.say(say);
        }
        'Y' => keep(lab),
        'J' => {
            if let Some(q) = state_mut(lab) {
                q.deed = match q.deed {
                    Deed::Digs => Deed::Trips,
                    Deed::Trips => Deed::Digs,
                };
                let d = q.deed.name();
                lab.ui.say(format!("LEDGER RANKS BY {d} PER 1,000 FRAMES ALIVE"));
            }
        }
        'K' => {
            if let Some(q) = state_mut(lab) {
                let n = rows(q).len().max(1);
                q.cursor = (q.cursor + 1) % n;
            }
        }
        'U' => {
            if let Some(q) = state_mut(lab) {
                q.tint_off = !q.tint_off;
                let on = !q.tint_off;
                lab.renderer.line_tint_gen += 1;
                lab.ui.say(if on { "LINE COLOURS ON" } else { "LINE COLOURS OFF" });
            }
        }
        _ => {}
    }
}

/// `Y`: queue her next `KEEP_EGGS` eggs on a line -- the LOOK-clicked ant's
/// mind if one is clicked, else the cursor row's.
fn keep(lab: &mut Lab) {
    let colony = state(lab).map_or(0, |q| q.colony);
    let clicked = lab.ui.inspected_organism().and_then(|id| mind::genome_of(&lab.world, id));
    let row_mind = state(lab).and_then(|q| rows(q).get(q.cursor).and_then(|&i| q.lines[i].mind.clone()));
    let Some(m) = clicked.or(row_mind) else {
        lab.ui.say("KEEP WHICH LINE? CLICK AN ANT WITH LOOK, OR MOVE THE LEDGER CURSOR (K) TO A WILD LINE");
        return;
    };
    let Some(f) = lab.world.fathers.as_mut() else { return };
    let Some(g) = f.genome_of(&m) else {
        lab.ui.say("THAT ANT'S MIND IS NOT ONE THE QUEEN CAN CARRY");
        return;
    };
    let Some(store) = f.colonies.get_mut(&colony) else { return };
    store.queue.clear();
    for _ in 0..KEEP_EGGS {
        store.queue.push_back(Father { mind: m.clone(), genome: g.clone(), weight: 1.0 });
    }
    let name = state(lab).map(|q| q.lines[line_of(&q.lines, &m)].name.clone()).unwrap_or_default();
    lab.ui.say(format!("HER NEXT {KEEP_EGGS} EGGS CARRY THE {name} LINE"));
}

/// The ledger's rows: the stock, then each wild line not yet gone, in birth
/// order (a line's end is a notice, not a row).
fn rows(q: &QueenDemo) -> Vec<usize> {
    (0..q.lines.len()).filter(|&i| i == 0 || !q.lines[i].gone).collect()
}

/// Once a displayed frame, after the body has its keys.
pub fn frame(lab: &mut Lab) {
    arm(lab);
}

/// Before each simulated tick.
pub fn before_tick(lab: &mut Lab) {
    arm(lab);
}

/// After each simulated tick: read the store's log (new lines, notices),
/// watch the hold, and every 50 frames count the ledger and refill the tint.
pub fn after_tick(lab: &mut Lab) {
    if state(lab).is_none_or(|q| !q.ready) {
        return;
    }
    let frame = lab.world.frame;
    let seed = lab.world.seed;
    let mut notes: Vec<String> = Vec::new();
    // New eggs from the store.
    let fresh: Vec<_> = {
        let seen = state(lab).map_or(0, |q| q.log_seen);
        lab.world.fathers.as_ref().map(|f| f.log[seen.min(f.log.len())..].to_vec()).unwrap_or_default()
    };
    {
        let q = state_mut(lab).expect("ready");
        q.log_seen += fresh.len();
        for e in &fresh {
            q.tally.eggs += 1;
            if e.queued {
                q.tally.kept_eggs += 1;
            }
            if e.wild > 0 {
                q.tally.wild_eggs += 1;
                let n = q.lines.len();
                let name = crate::lab::names::line_name(seed ^ 0x9E3E_57A7, n as u32);
                let colour = PALETTE[(n - 1) % PALETTE.len()];
                q.lines.push(Line {
                    mind: Some(e.mind.clone()),
                    name: name.clone(),
                    colour: Some(colour),
                    wild: e.wild,
                    born: frame,
                    adults: 0,
                    brood: 1,
                    seen: false,
                    gone: false,
                    digs: 0,
                    trips: 0,
                    frames: 0,
                });
                let kind = if e.wild >= 2 { "FERAL" } else { "WILD" };
                notes.push(format!("A {kind} EGG: THE {name} LINE BEGINS"));
            }
        }
    }
    // The hold.
    let colony = state(lab).map_or(0, |q| q.colony);
    let holds = steer::holding_queen(&lab.world, colony).is_some();
    {
        let q = state_mut(lab).expect("ready");
        if let Some(was) = q.held {
            if was && !holds {
                q.tally.lapses += 1;
                notes.push("HER HOLD LAPSED -- WORKERS MAY LAY THEIR OWN LINES. LAY (L) TO TAKE IT BACK".to_string());
            } else if !was && holds {
                q.tally.returns += 1;
                notes.push("SHE LAID: HER RULE IS BACK, ONLY SHE LAYS".to_string());
            }
        }
        q.held = Some(holds);
    }
    // **L pressed where no egg fits** (the shaft, a full chamber): the lay
    // is held back with nothing on screen, so say why, at most every 600
    // frames. Measured: a scripted queen holding L in her shaft for 10,000
    // frames at 4,000-8,000 J laid nothing (`pile_site` found no cell).
    let held_now = lab.world.creature_stats.buds_held_for_nest;
    let pressed = lab.world.steered.as_ref().is_some_and(|s| s.used.lay);
    {
        let q = state_mut(lab).expect("ready");
        if pressed && held_now > q.held_for_nest && frame >= q.no_room_said + 600 {
            q.no_room_said = frame;
            notes.push("NO ROOM FOR AN EGG HERE -- DIG HER A CHAMBER (G, SIDEWAYS) AND LAY THERE".to_string());
        }
        q.held_for_nest = held_now;
    }
    if frame.saturating_sub(state(lab).map_or(0, |q| q.counted)) >= 50 {
        count(lab, &mut notes);
    }
    for n in notes {
        lab.ui.say(n);
    }
}

/// Count the ledger, the tally and the tint map from the living box.
fn count(lab: &mut Lab, notes: &mut Vec<String>) {
    let world = &lab.world;
    let frame = world.frame;
    let Some(ant) = world.species.id_of("ant") else { return };
    let queen = steer::steered_id(world);
    let q = lab.demo.as_mut().and_then(|d| d.queen.as_mut()).expect("ready");
    q.counted = frame;
    for l in &mut q.lines {
        (l.adults, l.brood, l.digs, l.trips, l.frames) = (0, 0, 0, 0, 0);
    }
    let (mut workers, mut brood, mut hungry, mut worker_eggs) = (0, 0, 0, 0);
    let tint = &mut lab.renderer.line_tint;
    tint.clear();
    // Brood is listed apart from the living adults (`live_brood_ids`).
    for id in world.live_organism_ids().into_iter().chain(world.live_brood_ids()) {
        let Some(s) = world.organism(id) else { continue };
        if s.species != ant || s.colony != q.colony {
            continue;
        }
        let Some(m) = mind::genome_of(world, id) else { continue };
        let i = line_of(&q.lines, &m);
        let l = &mut q.lines[i];
        if let Some(b) = s.brood {
            l.brood += 1;
            brood += 1;
            if b.stage == BroodStage::Larva && b.target > 0.0 && s.energy < 0.5 * b.target {
                hungry += 1;
            }
        } else {
            l.adults += 1;
            if Some(id) != queen {
                workers += 1;
                worker_eggs += s.life.offspring;
                let age = frame.saturating_sub(s.born_frame);
                if age >= MIN_AGE {
                    l.digs += u64::from(s.life.digs);
                    l.trips += u64::from(s.life.trip_deliveries);
                    l.frames += age;
                }
            }
        }
        if !q.tint_off {
            if let Some(c) = l.colour {
                tint.insert(id, c);
            }
        }
    }
    for l in q.lines.iter_mut().skip(1) {
        let alive = l.adults + l.brood > 0;
        if l.adults > 0 && !l.seen {
            l.seen = true;
            // Only she lays while she holds, so a wild line stays this one ant
            // unless she is told to carry it (hand-off to-do 2): say so here,
            // where the player first sees the ant. The ticker clips at about 72
            // characters, so it is short.
            notes.push(format!("FIRST {} ANT HATCHED -- KEEP THE LINE: LOOK-CLICK IT, THEN Y", l.name));
        }
        if !alive && !l.gone {
            l.gone = true;
            notes.push(format!("THE {} LINE IS GONE", l.name));
        }
        // A line counted out between an egg's death and a sister's hatch
        // comes back: `gone` is a notice, not a verdict.
        if alive {
            l.gone = false;
        }
    }
    q.tally.workers = workers;
    q.tally.brood = brood;
    q.tally.hungry_larvae = hungry;
    q.tally.worker_eggs = worker_eggs;
}

/// The pointer over the world; `true` if the queen seat took it.
pub fn mouse(_lab: &mut Lab, _cell: (i32, i32), _act: super::MouseAct) -> bool {
    false
}

/// Marks over the world.
pub fn draw_world(_lab: &Lab, _frame_buf: &mut [u8]) {}

/// Whether [`draw_world`] drew anything this frame.
pub fn marks_world(_lab: &Lab) -> bool {
    false
}

/// The panel's lines, top to bottom, as `(text, colour)`; also what the
/// scripted player prints.
pub fn panel(lab: &Lab) -> Vec<(String, [u8; 4])> {
    const INK: [u8; 4] = [230, 232, 236, 255];
    const DIM: [u8; 4] = [150, 156, 168, 255];
    let Some(q) = state(lab).filter(|q| q.ready) else { return Vec::new() };
    let mut out = Vec::new();
    let left = steer::hold_left(&lab.world).unwrap_or(0.0);
    let bar_n = (left * 10.0).round() as usize;
    let wild = lab.world.fathers.as_ref().and_then(|f| f.colonies.get(&q.colony)).map_or(0, |s| s.wild);
    let queued = lab.world.fathers.as_ref().and_then(|f| f.colonies.get(&q.colony)).map_or(0, |s| s.queue.len());
    let next = match (wild, queued) {
        (2, _) => "FERAL".to_string(),
        (1, _) => "WILD".to_string(),
        (_, 0) => "STOCK".to_string(),
        (_, n) => format!("KEPT x{n}"),
    };
    out.push((format!("HOLD [{}{}]  NEXT EGG: {next}", "=".repeat(bar_n), ".".repeat(10 - bar_n)), if left > 0.0 { INK } else { [255, 120, 90, 255] }));
    let t = q.tally;
    out.push((format!("WORKERS {}  BROOD {} ({} HUNGRY)  EGGS {} ({} WILD)", t.workers, t.brood, t.hungry_larvae, t.eggs, t.wild_eggs), DIM));
    out.push((format!("LINE           ADULT BROOD {}/1K", q.deed.name()), DIM));
    for (r, &i) in rows(q).iter().enumerate().take(7) {
        let l = &q.lines[i];
        let rate = l.rate(q.deed).map_or("  --".to_string(), |v| format!("{v:5.2}"));
        let mark = if r == q.cursor { ">" } else { " " };
        let name: String = l.name.chars().take(13).collect();
        let colour = l.colour.map_or(INK, |c| [c[0] as u8, c[1] as u8, c[2] as u8, 255]);
        let tag = if l.wild >= 2 { " F" } else if l.wild == 1 { " W" } else { "" };
        out.push((format!("{mark}{name:<13} {:>5} {:>5} {rate}{tag}", l.adults, l.brood), colour));
    }
    out
}

/// The queen's panel, bottom left above the lab's bar.
pub fn draw_hud(lab: &Lab, hc: crate::render::Hud, frame_buf: &mut [u8]) {
    let lines = panel(lab);
    if lines.is_empty() {
        return;
    }
    let row = crate::hud::GLYPH_HEIGHT + 3;
    let w = lines.iter().map(|(t, _)| crate::hud::text_width(t)).max().unwrap_or(0) + 8;
    let h = row * lines.len() as i32 + 6;
    let x = 4;
    // Above the bar and the notice strip that sits on it.
    let y = crate::lab::ui::bar_top() - h - 18;
    crate::render::paint_owner::set("QUEEN");
    hc.blend_rect(frame_buf, x, y, w, h, [14, 16, 20, 255], 0.72);
    for (i, (t, c)) in lines.iter().enumerate() {
        hc.text(frame_buf, x + 4, y + 4 + row * i as i32, t, *c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::demo::{DemoKey, DemoSeat};

    /// **The demo arms once she lands, and its keys reach the store**: the
    /// 30 fathers with p = 1 for her colony, her own mind one the store
    /// knows (so Y on her line can be carried), the hold set, P cycling the
    /// wild level ordinary -> mild -> feral -> ordinary.
    #[test]
    fn the_queen_demo_arms_on_landing_and_p_sets_the_wild_level() {
        let sc = crate::lab::scenario::Scenario::load("demo_queen").expect("demo_queen ships");
        let mut lab = Lab::new(sc.bed.clone());
        lab.start_demo(DemoSeat::Queen);
        assert!(lab.world.fathers.is_none(), "armed before she landed");
        for _ in 0..1_300 {
            lab.demo_frame();
            lab.tick_for_harness();
        }
        let id = steer::steered_id(&lab.world).expect("no queen bound after the landing");
        let q = lab.demo.as_ref().and_then(|d| d.queen.as_ref()).expect("queen state");
        assert!(q.ready);
        assert_eq!(lab.renderer.zoom, NEST_ZOOM, "the camera did not close in on her nest");
        let f = lab.world.fathers.as_ref().expect("no father store");
        let store = f.colonies.get(&q.colony).expect("no store for her colony");
        assert_eq!(store.fathers.len(), 30);
        assert_eq!(store.p, 1.0);
        let hers = mind::genome_of(&lab.world, id).expect("she has no mind");
        assert!(f.genome_of(&hers).is_some(), "her own mind is not one the store can carry");
        assert_eq!(lab.world.steered.as_ref().and_then(|s| s.hold), Some(HOLD));
        let wild = |lab: &Lab| lab.world.fathers.as_ref().and_then(|f| f.colonies.get(&q_colony(lab))).map(|s| s.wild);
        for want in [1, 2, 0] {
            assert!(lab.demo_key(DemoKey::Letter('P'), true));
            assert_eq!(wild(&lab), Some(want));
        }
    }

    fn q_colony(lab: &Lab) -> u32 {
        lab.demo.as_ref().and_then(|d| d.queen.as_ref()).map_or(0, |q| q.colony)
    }
}
