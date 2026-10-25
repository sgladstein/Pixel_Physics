//! **The lone scout seat's demo** (`demo=scout`): the player is one ordinary
//! worker in a hungry trained colony, and teaches her sisters the way to a
//! far heap one at a time, by touch (`sim::tandem`). Design:
//! `Reports/teaching-the-colony-2026-10-25/seat-scout.md`; the demo's plan
//! and its reviews: `Reports/teaching-the-colony-2026-10-25/scout-demo/`.
//!
//! ```text
//! cargo run --release --bin lab -- demo=scout arm=1      (2, 3: sealed arms)
//! cargo run --release --bin lab -- demo=scout arm=A      (A, B, C: named, knobs shown)
//! ```
//!
//! **The arms** (plan v2 §3.8-3.10): A, the scout (tap, follow by touch,
//! landmark stops, breaks, and a scripted cascade at chance `c` per trip
//! home); B, the owner's rejected design given equal terms (Y *calls*: every
//! willing sister within six cells falls in behind you and never stops or
//! strays); C, the escort (A with no cascade). `arm=1|2|3` is **sealed**: the
//! letters are shuffled by the box seed (`seal=` overrides) and written to
//! `target/scout-demo-seal.txt`, and `c` and the knobs are hidden.
//!
//! **Keys** (the seat keys of the shared layer): `Y` tap (B: call), `U` pin
//! the last pupil (the camera follows her; `U` again lets go), `P` sniff the
//! sister nearest you (what she knows, how hungry she is, how quick).
//!
//! **The warm-up.** Most of a landing colony starves in its first 2,000
//! frames (step-0 census, 44 of 52 on seed 1), so the demo runs the box at
//! the dial's top until frame [`HANDOVER`] and only then hands the player her
//! body: a modal-pace adult on the surface by the near trickle.

use std::collections::{BTreeMap, VecDeque};

use crate::sim::cell::OrganismId;
use crate::sim::steer;
use crate::sim::tandem::{self, Arm, Mark, Patch, Phase, TapResult, Tandems};
use crate::sim::world::World;

use super::super::Lab;

/// The seat's keys, as the help line prints them.
pub const SEAT_KEYS: &str = "Y TAP (B: CALL) -- U PIN PUPIL -- P SNIFF";

/// The frame the player gets her body (step-0 census: past the landing die-off).
pub const HANDOVER: u64 = 18_000;
/// The nest's column, the near trickle's and the far heap's (`demo_scout.ron`).
pub const NEST_X: i32 = 128;
pub const TRICKLE_X: i32 = 153;
pub const HEAP_X: i32 = 248;
/// The player's body is chosen near here: by the trickle, where foragers pass
/// and the way east is open (the mound boxed a scripted walker in).
pub const BODY_X: i32 = 160;

/// The six orders of the three arms, for the seal.
const ORDERS: [[Arm; 3]; 6] = [
    [Arm::Scout, Arm::Column, Arm::Escort],
    [Arm::Scout, Arm::Escort, Arm::Column],
    [Arm::Column, Arm::Scout, Arm::Escort],
    [Arm::Column, Arm::Escort, Arm::Scout],
    [Arm::Escort, Arm::Scout, Arm::Column],
    [Arm::Escort, Arm::Column, Arm::Scout],
];

/// The scout seat's running state.
#[derive(Clone, Debug)]
pub struct ScoutDemo {
    pub arm: Arm,
    /// Sealed: the HUD names the arm by number only and hides `c` and the knobs.
    pub sealed: bool,
    /// What the HUD calls the arm ("1", or "A").
    pub label: String,
    pub c: Option<f32>,
    /// The recall head's gain ([`start_tandems`]).
    pub gain: f32,
    pub started: bool,
    pub alive_at_start: usize,
    pub last_pupil: Option<OrganismId>,
    pub pin: bool,
    /// Each pupil's way home after arrival, sampled every 10 frames, for the
    /// fading route line (plan v2 §5: the payoff must be visible in seconds).
    pub routes: BTreeMap<OrganismId, VecDeque<(i32, i32)>>,
}

impl ScoutDemo {
    pub fn from_args(args: &BTreeMap<String, String>, box_seed: u64) -> ScoutDemo {
        let raw = args.get("arm").map(|s| s.trim().to_ascii_uppercase()).unwrap_or_else(|| "A".into());
        let seal: u64 = args.get("seal").and_then(|s| s.parse().ok()).unwrap_or(box_seed);
        let order = ORDERS[(crate::sim::rng::stream(seal, 0x5c07, 0, 1).below(6)) as usize];
        let (arm, sealed) = match raw.as_str() {
            "1" => (order[0], true),
            "2" => (order[1], true),
            "3" => (order[2], true),
            "B" => (Arm::Column, false),
            "C" => (Arm::Escort, false),
            _ => (Arm::Scout, false),
        };
        if sealed {
            let text = format!(
                "scout demo seal {seal}: arm 1 = {}, arm 2 = {}, arm 3 = {} (A scout, B column, C escort)\n",
                order[0].letter(),
                order[1].letter(),
                order[2].letter()
            );
            let _ = std::fs::create_dir_all("target");
            let _ = std::fs::write("target/scout-demo-seal.txt", text);
        }
        ScoutDemo {
            arm,
            sealed,
            label: if sealed { raw.clone() } else { arm.letter().to_string() },
            c: args.get("c").and_then(|s| s.parse().ok()),
            gain: args.get("gain").and_then(|s| s.parse().ok()).unwrap_or(GAIN),
            started: false,
            alive_at_start: 0,
            last_pupil: None,
            pin: false,
            routes: BTreeMap::new(),
        }
    }
}

fn head(world: &World, id: OrganismId) -> Option<(i32, i32)> {
    world.organism(id).and_then(|s| s.chain.first().copied())
}

pub fn ants(world: &World) -> Vec<OrganismId> {
    let Some(ant) = world.species.id_of("ant") else { return Vec::new() };
    world.live_organism_ids().into_iter().filter(|&i| world.organism(i).is_some_and(|s| s.species == ant)).collect()
}

/// **The player's body**: an adult on the surface within 25 columns of
/// [`BODY_X`] whose decision interval is the colony's commonest, so she
/// walks at her pupils' pace (plan review 8), nearest the spot first.
pub fn pick_body(world: &World) -> Option<OrganismId> {
    let ant = world.species.id_of("ant")?;
    let def = world.species.creature_shared(ant)?;
    let ids = ants(world);
    let interval = |id| crate::sim::creature::organism_tick_interval(world, id, &def);
    let mut counts = BTreeMap::new();
    for &id in &ids {
        *counts.entry(interval(id)).or_insert(0) += 1;
    }
    let modal = counts.into_iter().max_by_key(|&(_, n)| n)?.0;
    ids.into_iter()
        .filter(|&id| interval(id) == modal)
        .filter_map(|id| {
            head(world, id)
                .filter(|h| world.room_surface_at(h.0).is_some_and(|top| h.1 <= top + 1))
                .map(|h| ((h.0 - BODY_X).abs(), id))
        })
        .filter(|&(d, _)| d <= 25)
        .min()
        .map(|(_, id)| id)
}

/// **Arm the tandem module** at the hand-over (also what a headless harness
/// calls, so the binary and `examples/scoutdemo.rs` start the same way).
///
/// `gain` is the recall head's listening gain (`creature::recall`): how
/// strongly a taught sister's walk leans toward the place she was shown.
/// Today's pool has no weight on the memory, so it is scripted, the same in
/// every arm, and named on the panel; `gain=0` is the honest arm (a lesson
/// that changes nothing after the first trip). [`GAIN`] by default: the
/// keeper seat's calibration put a place 60 columns out 37 columns nearer in
/// 1,500 frames at gain 4 and 91 at 8; the heap is 120 out.
pub fn start_tandems(world: &mut World, arm: Arm, c: Option<f32>, gain: f32) {
    let mut t = Tandems::new(arm, Some(Patch { x: HEAP_X, r: 10 }));
    if let (Some(c), Arm::Scout) = (c, arm) {
        t.knobs.cascade_c = c;
    }
    world.tandem = Some(Box::new(t));
    if world.recall.is_none() {
        world.recall = Some(crate::sim::creature::recall::Recall::new(gain, 0.0));
    }
}

/// The recall gain the seat runs at unless `gain=` says otherwise.
pub const GAIN: f32 = 6.0;

/// One of the seat keys (`Y U P J K R`) was pressed.
pub fn seat_key(lab: &mut Lab, key: char) {
    let Some(me) = steer::steered_id(&lab.world) else {
        if !lab.demo.as_ref().and_then(|d| d.scout.as_ref()).is_some_and(|s| s.started) {
            lab.ui.say("THE COLONY IS STILL SETTLING -- YOUR BODY COMES AT THE HAND-OVER");
        }
        return;
    };
    match key {
        'Y' => {
            let line = match tandem::tap(&mut lab.world, me) {
                TapResult::Accepted(id) => {
                    if let Some(s) = lab.demo.as_mut().and_then(|d| d.scout.as_mut()) {
                        s.last_pupil = Some(id);
                    }
                    "SHE TOUCHES YOUR ABDOMEN: LEAD HER -- WAIT WHEN SHE STOPS".to_string()
                }
                TapResult::Refused(_, r) => format!("REFUSED: {}", r.words().to_uppercase()),
                TapResult::NoOne => "NOBODY IN REACH TO TAP".to_string(),
            };
            lab.ui.say(line);
        }
        'U' => {
            if let Some(s) = lab.demo.as_mut().and_then(|d| d.scout.as_mut()) {
                s.pin = !s.pin && s.last_pupil.is_some();
                let line = if s.pin { "FOLLOWING YOUR LAST PUPIL -- U TO COME BACK" } else { "BACK ON YOU" };
                lab.ui.say(line);
            }
        }
        'P' => {
            let line = sniff(&lab.world, me);
            lab.ui.say(line);
        }
        _ => {}
    }
}

/// One line on the sister nearest `me`: what she knows, how hungry she is.
fn sniff(world: &World, me: OrganismId) -> String {
    let Some(at) = head(world, me) else { return String::new() };
    let near = ants(world)
        .into_iter()
        .filter(|&id| id != me)
        .filter_map(|id| head(world, id).map(|h| ((h.0 - at.0).abs().max((h.1 - at.1).abs()), id)))
        .min();
    let Some((d, id)) = near else { return "NOBODY NEAR".into() };
    let t = world.tandem.as_deref();
    let knows = t.and_then(|t| t.lessons.get(&id)).map_or("KNOWS NOTHING OUTSIDE".to_string(), |s| format!("KNOWS THE HEAP ({:.0}%)", s * 100.0));
    let led = t.and_then(|t| t.tree.get(&id)).map_or("", |&teacher| if teacher == me { " -- LED BY YOU" } else { " -- LED BY A SISTER" });
    let bank = tandem::energy_sense(world, id).map_or(0.0, |s| s / (1.0 - s).max(1e-3));
    let pace = world
        .organism(id)
        .and_then(|st| world.species.creature_shared(st.species))
        .map_or(5, |def| crate::sim::creature::organism_tick_interval(world, id, &def));
    format!("SISTER {d} CELLS OFF: {knows}{led} -- BANK {bank:.1} TANKS -- {}", if pace > 5 { "SLOW" } else if pace < 5 { "QUICK" } else { "STEADY" })
}

/// Once a displayed frame, after the body has its keys.
pub fn frame(lab: &mut Lab) {
    let Some(s) = lab.demo.as_ref().and_then(|d| d.scout.as_ref()) else { return };
    if !s.started {
        if lab.world.frame < HANDOVER {
            // The top of the dial; the wall clock caps what it delivers.
            if lab.time.multiple() < 256 {
                lab.time.set_preset(5);
            }
            if lab.world.frame.is_multiple_of(600) {
                lab.ui.say(format!("THE COLONY IS SETTLING ({}%) -- YOU HATCH AT THE HAND-OVER", lab.world.frame * 100 / HANDOVER));
            }
            return;
        }
        let (arm, c, gain) = (s.arm, s.c, s.gain);
        start_tandems(&mut lab.world, arm, c, gain);
        lab.time.set_preset(0);
        let alive = ants(&lab.world).len();
        if let Some(s) = lab.demo.as_mut().and_then(|d| d.scout.as_mut()) {
            s.started = true;
            s.alive_at_start = alive;
        }
        lab.demo_bind_body();
        lab.ui.say(format!("YOU HATCH: ONE WORKER AMONG {alive}. FOOD LIES FAR TO THE EAST. {SEAT_KEYS}"));
        return;
    }
    let Some(me) = steer::steered_id(&lab.world) else { return };
    // **The leader keeps her pupil's pace** (plan v2 §3.3): while a pupil of
    // hers is following three or more cells back, the player's step is held.
    let lagging = lab.world.tandem.as_deref().is_some_and(|t| {
        t.pairs.iter().any(|p| {
            p.leader == me
                && matches!(p.phase, Phase::Following)
                && tandem::contact_gap(&lab.world, p.follower, me).is_some_and(|g| g >= 3)
        })
    });
    if lagging {
        if let Some(st) = lab.world.steered.as_mut() {
            if let Some(input) = st.input.as_mut() {
                input.dir = None;
            }
        }
    }
    let pin = lab.demo.as_ref().and_then(|d| d.scout.as_ref()).and_then(|s| s.pin.then_some(s.last_pupil).flatten());
    if let Some(at) = pin.and_then(|id| lab.world.organism(id)).and_then(super::super::roster::anchor_of) {
        if let Some(d) = lab.demo.as_mut() {
            d.follow = false;
        }
        let bounds = lab.world.bounds();
        lab.renderer.follow(at, (super::super::WIDTH, super::super::HEIGHT), bounds);
    }
}

/// Before each simulated tick (the tandem tick itself runs in `tick_world`).
pub fn before_tick(_lab: &mut Lab) {}

/// After each simulated tick: sample the routes home of fresh pupils.
pub fn after_tick(lab: &mut Lab) {
    if !lab.world.frame.is_multiple_of(10) {
        return;
    }
    let Some(t) = lab.world.tandem.as_deref() else { return };
    let frame = lab.world.frame;
    let fresh: Vec<OrganismId> = t
        .marks
        .iter()
        .filter(|&&(_, m, f)| m == Mark::Arrived && frame.saturating_sub(f) <= 3_000)
        .map(|&(id, _, _)| id)
        .collect();
    let at: Vec<(OrganismId, (i32, i32))> = fresh.iter().filter_map(|&id| head(&lab.world, id).map(|h| (id, h))).collect();
    let Some(s) = lab.demo.as_mut().and_then(|d| d.scout.as_mut()) else { return };
    for (id, h) in at {
        let r = s.routes.entry(id).or_default();
        r.push_back(h);
        while r.len() > 120 {
            r.pop_front();
        }
    }
    s.routes.retain(|id, _| fresh.contains(id));
}

/// The pointer over the world; the scout seat does not take it.
pub fn mouse(_lab: &mut Lab, _cell: (i32, i32), _act: super::MouseAct) -> bool {
    false
}

const WHITE: [u8; 4] = [255, 255, 255, 255];
const ORANGE: [u8; 4] = [255, 150, 30, 255];
const GREEN: [u8; 4] = [80, 230, 90, 255];
const AMBER: [u8; 4] = [255, 200, 40, 255];
const GREY: [u8; 4] = [150, 150, 150, 255];
const RED: [u8; 4] = [235, 60, 50, 255];
const TREE: [u8; 4] = [120, 200, 255, 255];

/// **Marks over the world**: a full replace on fixed colours (CLAUDE.md: a
/// debug readout must not be a blend of the thing it shows). The player
/// white with a ring; ants that know the heap orange by how well; a pip over
/// each follower (green in contact, amber stopping, grey lost) with a tick
/// per landmark learned; the ground bracket of a search; the teaching tree;
/// each fresh pupil's way home; a red cross over a refusal.
pub fn draw_world(lab: &Lab, frame_buf: &mut [u8]) {
    let Some(t) = lab.world.tandem.as_deref() else { return };
    let hc = super::super::ui::hud_canvas(&lab.renderer);
    let r = &lab.renderer;
    let top = super::super::ui::bar_top();
    let at = |x: i32, y: i32| {
        let (x0, y0, x1, y1, _) = r.world_rect_to_logical(x, y, x, y);
        ((x0 + x1) / 2, (y0 + y1) / 2)
    };
    let dot = |buf: &mut [u8], (x, y): (i32, i32), c: [u8; 4]| {
        if y < top {
            hc.put(buf, x, y, c);
        }
    };
    // A body cell filled whole: one pixel per cell vanishes at play zoom.
    let cell = |buf: &mut [u8], x: i32, y: i32, c: [u8; 4]| {
        let (x0, y0, x1, y1, _) = r.world_rect_to_logical(x, y, x, y);
        let y1 = y1.min(top - 1);
        if y1 >= y0 {
            hc.fill_rect(buf, x0, y0, x1 - x0 + 1, y1 - y0 + 1, c);
        }
    };
    let world = &lab.world;
    // Knowledge tint over every body that holds a lesson.
    // Strength times freshness when the recall head holds her lesson, so the tint fades as the memory does.
    for (&id, &s) in &t.lessons {
        let s = crate::sim::creature::recall::lesson(world, id)
            .and_then(|p| crate::sim::creature::recall::knows(world, id, p.place, 0))
            .unwrap_or(s);
        if let Some(st) = world.organism(id) {
            let k = 0.45 + 0.55 * s.clamp(0.0, 1.0);
            let c = [(ORANGE[0] as f32 * k) as u8, (ORANGE[1] as f32 * k) as u8, (ORANGE[2] as f32 * k) as u8, 255];
            for &(x, y) in &st.chain {
                cell(frame_buf, x, y, c);
            }
        }
    }
    // The teaching tree: a dotted line from each living pupil to her living teacher.
    for (&pupil, &teacher) in &t.tree {
        let (Some(a), Some(b)) = (head(world, pupil), head(world, teacher)) else { continue };
        let (pa, pb) = (at(a.0, a.1), at(b.0, b.1));
        let n = (pa.0 - pb.0).abs().max((pa.1 - pb.1).abs()).max(1);
        for i in (0..=n).step_by(3) {
            dot(frame_buf, (pa.0 + (pb.0 - pa.0) * i / n, pa.1 + (pb.1 - pa.1) * i / n), TREE);
        }
    }
    // Each fresh pupil's way home, oldest dimmest.
    if let Some(s) = lab.demo.as_ref().and_then(|d| d.scout.as_ref()) {
        for route in s.routes.values() {
            let n = route.len().max(1);
            for (i, &(x, y)) in route.iter().enumerate() {
                let k = 0.3 + 0.7 * i as f32 / n as f32;
                dot(frame_buf, at(x, y - 2), [(GREEN[0] as f32 * k) as u8, (GREEN[1] as f32 * k) as u8, (GREEN[2] as f32 * k) as u8, 255]);
            }
        }
    }
    // Pips, landmark ticks and search brackets.
    for p in &t.pairs {
        let Some(f) = head(world, p.follower) else { continue };
        let (px, py) = at(f.0, f.1 - 3);
        let c = match p.phase {
            Phase::Following => GREEN,
            Phase::Tell { .. } | Phase::Pausing { .. } => AMBER,
            Phase::Lost { .. } => GREY,
        };
        hc.fill_rect(frame_buf, px - 1, py - 1, 3, 3, c);
        // Her body in the pip's colour too: at play zoom an ant is a few pixels, and the first GIFs lost the
        // pupil among her sisters.
        if let Some(st) = world.organism(p.follower) {
            for &(x, y) in &st.chain {
                cell(frame_buf, x, y, c);
            }
        }
        for i in 0..i32::from(p.learned.min(8)) {
            dot(frame_buf, (px - 4 + i, py - 3), WHITE);
        }
        if let Phase::Lost { centre, leg, .. } = p.phase {
            let len = t.knobs.sweep_legs[usize::from(leg).min(t.knobs.sweep_legs.len() - 1)];
            let (l, rr) = (at(centre.0 - len, centre.1 + 1), at(centre.0 + len, centre.1 + 1));
            for x in l.0..=rr.0 {
                dot(frame_buf, (x, l.1), GREY);
            }
            for dy in 0..3 {
                dot(frame_buf, (l.0, l.1 - dy), GREY);
                dot(frame_buf, (rr.0, rr.1 - dy), GREY);
            }
        }
    }
    // Refusals: a red cross over her for two seconds.
    for &(id, m, f) in &t.marks {
        if world.frame.saturating_sub(f) > 120 {
            continue;
        }
        let Some(h) = head(world, id) else { continue };
        let (x, y) = at(h.0, h.1 - 4);
        let c = match m {
            Mark::Refused(_) => RED,
            Mark::Broke => GREY,
            Mark::Arrived | Mark::Recontact | Mark::Landmark => continue,
        };
        for d in -2..=2 {
            dot(frame_buf, (x + d, y + d), c);
            dot(frame_buf, (x + d, y - d), c);
        }
    }
    // The player: white body, and a ring round her.
    if let Some(me) = steer::steered_id(world).and_then(|id| world.organism(id)) {
        for &(x, y) in &me.chain {
            cell(frame_buf, x, y, WHITE);
        }
        if let Some(&(x, y)) = me.chain.first() {
            let (cx, cy) = at(x, y);
            let (x0, _, x1, _, _) = r.world_rect_to_logical(x, y, x, y);
            let rad = (2 * (x1 - x0 + 1) + 3) as f32;
            for a in 0..24 {
                let th = a as f32 * std::f32::consts::TAU / 24.0;
                dot(frame_buf, (cx + (th.cos() * rad).round() as i32, cy + (th.sin() * rad).round() as i32), WHITE);
            }
        }
    }
}

/// Whether [`draw_world`] drew anything this frame.
pub fn marks_world(lab: &Lab) -> bool {
    lab.world.tandem.is_some()
}

/// **The scout's panel**: her bank and her pupil's, the heap's income, and
/// the run counters; in a sealed arm, nothing that names the arm.
pub fn draw_hud(lab: &Lab, hc: crate::render::Hud, frame_buf: &mut [u8]) {
    let Some(s) = lab.demo.as_ref().and_then(|d| d.scout.as_ref()) else { return };
    let line_h = crate::hud::GLYPH_HEIGHT + 2;
    let x = 4;
    let mut y = super::super::ui::bar_top() - 6 * line_h - 4;
    let mut say = |text: String, c: [u8; 4]| {
        hc.text(frame_buf, x, y, &text, c);
        y += line_h;
    };
    if !s.started {
        say(format!("SCOUT DEMO, ARM {} -- THE COLONY IS SETTLING", s.label), WHITE);
        return;
    }
    let Some(t) = lab.world.tandem.as_deref() else { return };
    let world = &lab.world;
    let tanks = |id: OrganismId| tandem::energy_sense(world, id).map_or(0.0, |s| s / (1.0 - s).max(1e-3));
    let me = steer::steered_id(world);
    let mine = me.map_or("YOU HAVE DIED".to_string(), |id| format!("YOUR BANK {:.1} TANKS", tanks(id)));
    let pupil = t.pairs.iter().find(|p| Some(p.leader) == me).map_or(String::new(), |p| format!(" -- PUPIL {:.1} TANKS", tanks(p.follower)));
    say(format!("SCOUT DEMO, ARM {} -- {mine}{pupil}", s.label), WHITE);
    let income = t.income_over(world.frame, 3_600);
    let bar: String = std::iter::repeat_n('|', income.min(40)).collect();
    say(format!("FOOD HOME FROM THE FAR HEAP, LAST MINUTE: {income} {bar}"), ORANGE);
    let c = &t.counts;
    say(format!("RUNS {} STARTED, {} ARRIVED, {} LOST ({} FOUND IT ANYWAY)", c.started, c.done, c.broken, c.broken_still_arrived), GREEN);
    say(format!("KNOW THE HEAP {} -- TRIPS HOME BY THEM {} -- STARVED WHILE LED {}", t.lessons.len(), c.tree_trips, c.starved_while_led), TREE);
    let alive = ants(world).len();
    say(format!("COLONY {alive} (WAS {} AT HATCH) -- TAPS {} REFUSED {}", s.alive_at_start, c.taps, c.refusals), GREY);
    if !s.sealed {
        let gain = world.recall.as_ref().map_or(0.0, |r| r.gain);
        say(
            format!("KNOBS: C {:.2}/TRIP, CASCADE RUNS {}, PAUSE 1/{:.0} STEPS, MEMORY PULL {gain:.0} (SCRIPTED)", t.knobs.cascade_c, c.cascade_runs, 1.0 / t.knobs.pause_chance),
            GREY,
        );
    }
}
