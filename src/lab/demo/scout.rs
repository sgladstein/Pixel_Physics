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
//!
//! **The lean season** (the stakes, 2026-10-30; `stakes=0` is the sandbox
//! before them; review `/mnt/project-files/seat-demos/review-2026-10-30/scout/review.md`
//! E1 + E2). At the hand-over the near trickle stops, so the far heap is the
//! only real food ([`LEAN_HEAP`] cells a drop, `heap=`), her pupils' memory
//! pulls at [`LEAN_GAIN`] rather than [`GAIN`], and a meter shows the
//! colony's stores draining: what its ants carry in their own banks, since
//! the food lying by the nest is eaten as it lands (measured 0-3 cells all
//! season on seeds 3 and 4).
//! The season lasts [`SEASON`] frames. **Score: her tree's harvest**, trips
//! home from the far heap by her pupils (one point) and by their pupils (two;
//! three from the third generation on) -- not every ant's, which is what the
//! old headline counted. **Lost** if the colony falls below [`FLOOR`] of its
//! size at the hand-over, or she starves.
//!
//! **Round 2** (2026-10-30, the goal: her teaching should decide the
//! colony's fate). The cascade now works in the season: a pupil home from
//! the heap taps a sister at chance [`LEAN_C`] and leads her out, and her
//! leader is unstuck (`tandem::Knobs::cascade_unstick`, which only this seat
//! sets) -- every round-1 cascade run stood at the mound until the pupil
//! starved. Her pupils' pupils now bring food home and score x2. What did
//! *not* follow, traced ant by ant with `examples/scoutdemo.rs probe=1`:
//! the colony at spring is mostly ants born in the season (of 99-138 alive
//! at the hand-over, 1-29 are alive at its end, in every arm), fed by what
//! comes home, and corpses bring home as much as the far heap does; and the
//! far heap cannot be hidden ([`LEAN_HEAP_X`]). So colony survival is still
//! the colony's own; her harvest is the score that tracks her skill.

use std::collections::{BTreeMap, VecDeque};

use crate::sim::cell::OrganismId;
use crate::sim::steer;
use crate::sim::tandem::{self, Arm, Mark, Patch, Phase, TapResult, Tandems};
use crate::sim::world::World;

use super::super::Lab;

/// The seat's keys, as the help line prints them.
pub const SEAT_KEYS: &str = "Y TAP (B: CALL) -- U PIN PUPIL -- P SNIFF -- R NEW SEASON";

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
    /// **The lean season** (on by default; `stakes=0` the sandbox before it).
    pub stakes: bool,
    /// Its length in frames from the hand-over (`season=`).
    pub season: u64,
    /// The colony may not fall below this share of its size at the hand-over (`floor=`).
    pub floor: f32,
    /// The frame the season began (the hand-over).
    pub season_start: u64,
    /// Her tree's harvest: trips home from the far heap by generation (her
    /// pupils, theirs, and every generation after), and the points they make.
    pub by_gen: [u32; 3],
    pub harvest: u32,
    /// Each tree ant's deliveries at the last look, and the fresh `+N` pips.
    pub seen: BTreeMap<OrganismId, u32>,
    pub pops: Vec<(OrganismId, u64, u8)>,
    /// **The colony's stores**: the full tanks its ants carry in their own
    /// banks (`tandem::energy_sense`), now and at most this season.
    pub larder: f32,
    pub larder_peak: f32,
    /// The far heap's cells a drop through the season (`heap=`).
    pub heap: u32,
    /// **Where the season's food falls** (`heapx=`): [`LEAN_HEAP_X`] with
    /// stakes on, the warm-up's [`HEAP_X`] in the sandbox.
    pub heap_x: i32,
    /// The fewest ants the colony has had this season.
    pub low: usize,
    pub outcome: Option<SeasonEnd>,
    /// The frame `R` was first pressed mid-season (a second press restarts).
    pub restart_asked: u64,
    /// **The sealed heap** (round 3; on with the stakes, `sealed=0` off): in
    /// the lean season only an ant who knows the place carries from the far
    /// heap ([`seal_tick`]).
    pub heap_sealed: bool,
    pub seal: Seal,
    /// The near trickle in the lean season: every this many frames, or 0 for
    /// none (`trickle=`).
    pub trickle: u64,
    /// The colony when a sealed season was lost at spring, for the card.
    pub spring: usize,
}

/// **The sealed heap's books and its marks** ([`seal_tick`], [`seal_watch`]).
#[derive(Clone, Debug, Default)]
pub struct Seal {
    /// Ants turned away and when their bounce ends, with its heading.
    pub bounced: BTreeMap<OrganismId, (u64, i8)>,
    /// Each ant's last-food memory at the last look, to see a pick-up.
    pub seen_food: BTreeMap<OrganismId, (i32, i32, u64)>,
    /// Fresh marks: `(ant, carried, frame)` -- a taught ant's pick-up
    /// (`true`) or an untaught ant turned away (`false`).
    pub marks: Vec<(OrganismId, bool, u64)>,
    /// Bounces (each a fresh turn-away, not a frame of one), pick-ups by ants
    /// who know the place, and pick-ups by ants who do not (leaks: the rule's
    /// own failure count, which should stay 0).
    pub turned_away: u32,
    pub carried: u32,
    pub leaks: u32,
    /// Distinct ants turned away at least once.
    pub turned_ids: std::collections::BTreeSet<OrganismId>,
    /// Each leak: the ant, the frame, the cell, and where her head was at
    /// the last pre-step look (`None`: not near enough to be checked).
    pub leak_log: Vec<(OrganismId, u64, (i32, i32), Option<(i32, i32)>)>,
    /// Heads of ants in the seal's columns at the last pre-step look.
    pub looked: BTreeMap<OrganismId, (i32, i32)>,
}

/// How the lean season ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeasonEnd {
    /// The colony fell below the floor, at this frame.
    Starved(u64),
    /// She died, at this frame.
    Died(u64),
    /// The season ran out with the colony this size.
    Endured(usize),
}

/// **The lean season's length**, frames from the hand-over: five minutes at 1x.
pub const SEASON: u64 = 18_000;
/// The colony may not fall below this share of its size at the hand-over.
pub const FLOOR: f32 = 0.5;

/// **With the heap sealed the season is judged at spring** (round 3): lost
/// if fewer than this share of the hand-over's colony came through. The
/// colony at spring is the season's births (rounds 1-2's trace: 1-21 of the
/// hand-over's ants live to its end, in every arm), so the old rule's check
/// at every half-second read the old cohort dying of age -- its trough --
/// which no teaching moves; every sealed season fell under half by frame
/// 4,000-8,000, patient player or none (12 of 12 each). Measured at the end,
/// sealed, 12 paired seeds: see the README's Round 3 table for the share
/// each player brought through and why 0.2.
pub const SPRING_FLOOR: f32 = 0.2;
/// **The far heap in the lean season**, cells a drop (every 6,000 frames):
/// the sandbox's own 120. Measured (gain 10, arm A, 8 paired seeds, colony
/// alive at the season's end, patient player against teaching off): at 120
/// the untaught colony on seed 4 left 90-165 cells in the heap while it
/// starved from 105 to 18 and a taught one stripped it and kept 69, but on
/// seed 3 untaught ants stripped it too -- patient beat off on 4 of 8. A
/// richer heap did not help: at 160 still 4 of 8; at 240 the box is no
/// longer lean (colonies boomed to 150-200 and off beat patient on 5 of 8).
pub const LEAN_HEAP: u32 = 120;

/// **Where the lean season's food falls** (`heapx=`): the warm-up's own
/// heap. Round 2 (2026-10-30) moved it to hide it from untaught foragers,
/// and it cannot be hidden: the probe (`examples/scoutdemo.rs probe=1`,
/// seeds 3-4, teaching off) found 37-104 untaught ants reaching it at every
/// place tried -- x 60, 248, 330, 400, 440 -- because a third of the colony
/// walks to one wall or the other whatever the food does. Farther only
/// lengthened her leads (at 400 her tree brought 9 cells home in a season).
/// Moving the warm-up's heap away instead (west, or none at all) starved the
/// colony before the hand-over on seed 4.
pub const LEAN_HEAP_X: i32 = HEAP_X;

/// **The cascade chance in the lean season** (`c=` overrides): a pupil home
/// from the heap taps a sister on every trip she can. With the leaders
/// unstuck (`tandem::Knobs::cascade_unstick`) a cascade run arrives, so
/// her tree grows by itself: seed 4, 7 cascade runs, 6 arrived, and her
/// pupils' pupils brought food home 8 times (0 in every round-1 run).
pub const LEAN_C: f32 = 1.0;

/// A pupil's own run to the heap with a sister in tow is given up after this
/// many frames (`tandem::Knobs::cascade_timeout`): two walks out at a
/// follower's pace.
pub const CASCADE_TIMEOUT: u64 = 4_000;

/// **Her pupils' memory pull in the lean season** (the recall head's gain,
/// scripted and named on the panel, `gain=`): 10 against the sandbox's 6.
/// On seed 1 it took her tree's harvest from 6 trips to 17-19 in the
/// season, so the score she plays for moves while she watches.
pub const LEAN_GAIN: f32 = 10.0;

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
            gain: args.get("gain").and_then(|s| s.parse().ok()).unwrap_or(if args.get("stakes").is_none_or(|v| v != "0") { LEAN_GAIN } else { GAIN }),
            started: false,
            alive_at_start: 0,
            last_pupil: None,
            pin: false,
            routes: BTreeMap::new(),
            stakes: args.get("stakes").is_none_or(|v| v != "0"),
            season: args.get("season").and_then(|s| s.parse().ok()).unwrap_or(SEASON),
            floor: args.get("floor").and_then(|s| s.parse().ok()).unwrap_or(if args.get("sealed").is_none_or(|v| v != "0") { SPRING_FLOOR } else { FLOOR }),
            season_start: 0,
            by_gen: [0; 3],
            harvest: 0,
            seen: BTreeMap::new(),
            pops: Vec::new(),
            larder: 0.0,
            larder_peak: 0.0,
            heap: args.get("heap").and_then(|s| s.parse().ok()).unwrap_or(LEAN_HEAP),
            heap_x: args.get("heapx").and_then(|s| s.parse().ok()).unwrap_or(if args.get("stakes").is_none_or(|v| v != "0") { LEAN_HEAP_X } else { HEAP_X }),
            low: 0,
            outcome: None,
            restart_asked: 0,
            heap_sealed: args.get("sealed").is_none_or(|v| v != "0"),
            seal: Seal::default(),
            trickle: args.get("trickle").and_then(|s| s.parse().ok()).unwrap_or(0),
            spring: 0,
        }
    }
}

/// **Start the lean season** at the hand-over: the near trickle stops (its
/// timeline event gets an `until`), so the far heap is the only food that
/// still comes. The binary and `examples/scoutdemo.rs` both call this, after
/// the seat's state is set.
pub fn start_season(lab: &mut Lab) {
    let frame = lab.world.frame;
    let alive = ants(&lab.world).len();
    let Some(s) = lab.demo.as_mut().and_then(|d| d.scout.as_mut()) else { return };
    s.season_start = frame;
    s.low = alive;
    if !s.stakes {
        return;
    }
    let (heap, heap_x, c_arg, trickle) = (s.heap, s.heap_x, s.c, s.trickle);
    if let Some(t) = lab.world.tandem.as_mut() {
        t.patch = Some(Patch { x: heap_x, r: 10 });
        t.knobs.cascade_unstick = true;
        t.knobs.cascade_timeout = CASCADE_TIMEOUT;
        if t.arm == tandem::Arm::Scout && c_arg.is_none() {
            t.knobs.cascade_c = LEAN_C;
        }
    }
    if let Some(sc) = lab.scenario.as_mut() {
        for e in &mut sc.timeline {
            if e.every == 0 {
                continue;
            }
            match &mut e.what {
                crate::lab::scenario::Placement::Heap { x, .. } if *x == TRICKLE_X => {
                    if trickle == 0 {
                        e.until = frame;
                    } else {
                        e.every = trickle;
                    }
                }
                crate::lab::scenario::Placement::Heap { x, cells, .. } if *x != TRICKLE_X => {
                    *cells = heap as i32;
                    *x = heap_x;
                }
                _ => {}
            }
        }
    }
    let floor = (alive as f32 * s.floor).ceil() as usize;
    let sealed = s.heap_sealed;
    lab.world.log_player_action("scout demo: the lean season begins, the near trickle stops");
    lab.ui.say(if sealed {
        format!("SEALED HEAP: ONLY ANTS LED THERE CAN CARRY. {floor}+ ALIVE BY SPRING")
    } else {
        format!("LEAN SEASON: THE NEAR FOOD HAS STOPPED. KEEP {floor}+ ALIVE TILL SPRING")
    });
}

/// **The colony's stores, in full tanks**: what its ants carry in their own
/// banks. The food by the nest is eaten as it lands, so this is the store
/// that drains once the near trickle stops.
fn larder(world: &World) -> f32 {
    ants(world).into_iter().filter_map(|id| tandem::energy_sense(world, id)).map(|s| s / (1.0 - s).max(1e-3)).sum()
}

/// Her generation in the tree: 1 for her own pupil, 2 for a pupil's pupil.
fn generation(t: &Tandems, id: OrganismId) -> u8 {
    let mut g = 1u8;
    let mut at = id;
    while let Some(&teacher) = t.tree.get(&at) {
        if !t.tree.contains_key(&teacher) || g >= 8 {
            break;
        }
        g += 1;
        at = teacher;
    }
    g
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
    if key == 'R' {
        restart(lab);
        return;
    }
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

/// **`R`: a new season** -- at once when the season is over (or she is
/// dead), else on a second press within 3 seconds. The same args, so the
/// arm, `stakes=0` and the knobs carry over; the box warms up again.
fn restart(lab: &mut Lab) {
    let frame = lab.world.frame;
    let over = lab.demo.as_ref().and_then(|d| d.scout.as_ref()).is_some_and(|s| s.outcome.is_some());
    let asked = lab.demo.as_ref().and_then(|d| d.scout.as_ref()).map_or(0, |s| s.restart_asked);
    if !over && (asked == 0 || frame > asked + 180) {
        if let Some(s) = lab.demo.as_mut().and_then(|d| d.scout.as_mut()) {
            s.restart_asked = frame.max(1);
        }
        lab.ui.say("R AGAIN TO START A NEW SEASON");
        return;
    }
    let args = lab.demo.as_ref().map(|d| d.args.clone()).unwrap_or_default();
    let msg = lab.start_demo_with(super::DemoSeat::Scout, args);
    lab.world.log_player_action("scout demo: R, a new season");
    lab.ui.say(msg);
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
        start_season(lab);
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

/// Before each simulated tick (the tandem tick itself runs in `tick_world`):
/// the sealed heap turns away the ants who do not know it.
pub fn before_tick(lab: &mut Lab) {
    seal_tick(lab);
}

/// **Columns either side of the heap's drop that the seal covers**: the
/// patch's own 10 and the pile's spread past it (a 120-cell drop is a
/// pyramid 21 wide that slumps a few columns further).
pub const SEAL_R: i32 = 16;
/// An ant who does not know the heap is turned away when her head comes
/// within this many cells of a sealed food cell: a pick-up reaches 1
/// (`options::build`, the eight neighbours), so 2 is a step of margin.
pub const SEAL_NEAR: i32 = 2;
/// How long a turned-away ant is walked off the heap before her mind has
/// her back (about 40 steps at the modal pace).
pub const BOUNCE_FRAMES: u64 = 240;

/// Is the seal in force now: the lean season, stakes on, `sealed` not 0.
pub fn seal_live(s: &ScoutDemo, frame: u64) -> bool {
    s.started && s.stakes && s.heap_sealed && frame >= s.season_start && frame < s.season_start + s.season
}

/// **Does `id` know the far heap?** She was led there (the tandem module's
/// lesson, by the player or by a pupil: a broken run that learned a landmark
/// counts, at its strength), or the recall head's taught-place memory points
/// into the heap, or she is the player.
pub fn knows_heap(world: &World, id: OrganismId, heap_x: i32) -> bool {
    steer::is_steered(world, id)
        || world.tandem.as_deref().is_some_and(|t| t.lessons.contains_key(&id))
        || world.recall.as_ref().and_then(|r| r.provenance.get(&id)).is_some_and(|p| (p.place.0 - heap_x).abs() <= SEAL_R)
}

/// A provisions cell of the sealed heap within `r` of `at`.
fn sealed_food_near(world: &World, at: (i32, i32), heap_x: i32, r: i32) -> bool {
    let Some(food) = world.materials.id_of("provisions") else { return false };
    (-r..=r).any(|dx| {
        let x = at.0 + dx;
        (x - heap_x).abs() <= SEAL_R && (-r..=r).any(|dy| world.in_bounds(x, at.1 + dy) && world.get(x, at.1 + dy).material == food)
    })
}

/// **The sealed heap** (round 3, 2026-10-30). Rounds 1 and 2 found the
/// colony's fate was not hers because untaught foragers reached the far heap
/// wherever it lay (37-104 a season at every place tried). In the lean
/// season, an ant who does not know the place ([`knows_heap`]) and comes
/// within [`SEAL_NEAR`] of the heap's food is walked off it for
/// [`BOUNCE_FRAMES`] (`steer::force`, a heading only: a forced heading never
/// picks up, so she gets nothing, and she is not left standing at the heap).
/// Followers and leaders in a run are the tandem module's and are left
/// alone. Nothing else in the box changes: the rule is a seat field, read
/// here, and off it is one `Option` read and a bool.
pub fn seal_tick(lab: &mut Lab) {
    let frame = lab.world.frame;
    let Some(s) = lab.demo.as_ref().and_then(|d| d.scout.as_ref()) else { return };
    let live = seal_live(s, frame);
    if !live && s.seal.bounced.is_empty() {
        return;
    }
    let heap_x = s.heap_x;
    let w = &lab.world;
    let in_run = |w: &World, id: OrganismId| w.tandem.as_deref().is_some_and(|t| t.pairs.iter().any(|p| p.follower == id || p.leader == id));
    let mut turn: Vec<(OrganismId, i8)> = Vec::new();
    let mut looked = BTreeMap::new();
    if live {
        for id in ants(w) {
            let Some(h) = head(w, id) else { continue };
            if (h.0 - heap_x).abs() > SEAL_R + SEAL_NEAR + 4 {
                continue;
            }
            looked.insert(id, h);
            if (h.0 - heap_x).abs() > SEAL_R + SEAL_NEAR || knows_heap(w, id, heap_x) || in_run(w, id) {
                continue;
            }
            if sealed_food_near(w, h, heap_x, SEAL_NEAR) {
                // The shortest way off the heap; the middle goes west, home.
                turn.push((id, if h.0 > heap_x { 1 } else { -1 }));
            }
        }
    }
    let stale: Vec<OrganismId> = s
        .seal
        .bounced
        .iter()
        .filter(|&(&id, &(until, _))| !live || frame >= until || head(w, id).is_none() || in_run(w, id))
        .map(|(&id, _)| id)
        .filter(|id| !turn.iter().any(|(t, _)| t == id))
        .collect();
    let release: Vec<OrganismId> = stale.iter().copied().filter(|&id| head(w, id).is_some() && !in_run(w, id)).collect();
    let fresh: Vec<(OrganismId, i8)> = turn.iter().copied().filter(|(id, _)| !s.seal.bounced.contains_key(id)).collect();
    for &id in &release {
        steer::unforce(&mut lab.world, id);
    }
    for &(id, dx) in &turn {
        steer::force(&mut lab.world, id, steer::ForcedPick { dir: Some((dx, 0)), verb: None, once: false });
    }
    if !fresh.is_empty() {
        lab.world.log_player_action(format!("scout demo: the sealed heap turned away {} ant(s)", fresh.len()));
    }
    let Some(s) = lab.demo.as_mut().and_then(|d| d.scout.as_mut()) else { return };
    s.seal.looked = looked;
    for id in stale {
        s.seal.bounced.remove(&id);
    }
    for (id, dx) in turn {
        let e = s.seal.bounced.entry(id).or_insert((0, dx));
        e.0 = frame + BOUNCE_FRAMES;
    }
    for (id, _) in fresh {
        s.seal.turned_away += 1;
        s.seal.turned_ids.insert(id);
        s.seal.marks.push((id, false, frame));
    }
}

/// **Who carried from the sealed heap**, every tick after the step: an ant
/// whose last-food memory moved to a heap cell beside her head with food in
/// her crop has just picked it up. One who knows the place gets the carry
/// mark; one who does not is a leak, the rule's own failure count.
fn seal_watch(lab: &mut Lab) {
    let frame = lab.world.frame;
    let Some(s) = lab.demo.as_ref().and_then(|d| d.scout.as_ref()) else { return };
    if !seal_live(s, frame) {
        return;
    }
    let heap_x = s.heap_x;
    let w = &lab.world;
    let mut seen: Vec<(OrganismId, (i32, i32, u64), Option<bool>)> = Vec::new();
    for id in ants(w) {
        let Some(st) = w.organism(id) else { continue };
        let (Some(lf), Some(&h)) = (st.last_food, st.chain.first()) else { continue };
        let picked = (lf.0 - heap_x).abs() <= SEAL_R
            && lf.2 + 2 >= frame
            && st.crop.is_some_and(|c| c.cells > 0)
            && (h.0 - lf.0).abs().max((h.1 - lf.1).abs()) <= 3
            && !steer::is_steered(w, id);
        seen.push((id, lf, picked.then(|| knows_heap(w, id, heap_x))));
    }
    let mut caught: Vec<(OrganismId, i8)> = Vec::new();
    let Some(s) = lab.demo.as_mut().and_then(|d| d.scout.as_mut()) else { return };
    for (id, lf, picked) in seen {
        if s.seal.seen_food.insert(id, lf) == Some(lf) {
            continue;
        }
        match picked {
            Some(true) => {
                s.seal.carried += 1;
                s.seal.marks.push((id, true, frame));
            }
            Some(false) => {
                s.seal.leaks += 1;
                let before = s.seal.looked.get(&id).copied();
                s.seal.leak_log.push((id, frame, (lf.0, lf.1), before));
                // **A scramble's crumb stays one crumb**: she is turned away at once. Traced (the seal's unit
                // test): a sister checked a cell off the heap's food at the pre-step look took a cell that a
                // taught ant dropped beside her within the same step -- food that moved after the look.
                caught.push((id, if lf.0 > heap_x { 1 } else { -1 }));
                if s.seal.bounced.insert(id, (frame + BOUNCE_FRAMES, caught.last().unwrap().1)).is_none() {
                    s.seal.turned_away += 1;
                    s.seal.turned_ids.insert(id);
                    s.seal.marks.push((id, false, frame));
                }
            }
            None => {}
        }
    }
    for (id, dx) in caught {
        steer::force(&mut lab.world, id, steer::ForcedPick { dir: Some((dx, 0)), verb: None, once: false });
    }
    let Some(s) = lab.demo.as_mut().and_then(|d| d.scout.as_mut()) else { return };
    s.seal.marks.retain(|&(_, _, f)| frame.saturating_sub(f) <= 90);
    if frame.is_multiple_of(600) {
        let live: std::collections::BTreeSet<OrganismId> = ants(&lab.world).into_iter().collect();
        if let Some(s) = lab.demo.as_mut().and_then(|d| d.scout.as_mut()) {
            s.seal.seen_food.retain(|id, _| live.contains(id));
        }
    }
}

/// After each simulated tick: sample the routes home of fresh pupils, and
/// keep the season's books.
pub fn after_tick(lab: &mut Lab) {
    seal_watch(lab);
    if lab.world.frame.is_multiple_of(30) {
        season_tick(lab);
    }
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

/// **The season's books**, every 30 frames: her tree's harvest by
/// generation (a trip home whose food lay in the far heap, as `tandem`'s own
/// `tree_trips` counts it), the larder, and the end.
fn season_tick(lab: &mut Lab) {
    let frame = lab.world.frame;
    if !lab.demo.as_ref().and_then(|d| d.scout.as_ref()).is_some_and(|s| s.started) {
        return;
    }
    let mut fresh: Vec<(OrganismId, u32, u8)> = Vec::new();
    if let Some(t) = lab.world.tandem.as_deref() {
        let patch = t.patch;
        for &id in t.tree.keys() {
            let Some(st) = lab.world.organism(id) else { continue };
            let from_patch = st.last_food.is_some_and(|(x, _, _)| patch.is_some_and(|p| p.holds_column(x)));
            fresh.push((id, st.life.trip_deliveries, if from_patch { generation(t, id) } else { 0 }));
        }
    }
    let alive = ants(&lab.world).len();
    let me = steer::steered_id(&lab.world);
    let food = frame.is_multiple_of(90).then(|| larder(&lab.world));
    let mut notes = Vec::new();
    let Some(s) = lab.demo.as_mut().and_then(|d| d.scout.as_mut()) else { return };
    for (id, trips, g) in fresh {
        let before = s.seen.insert(id, trips);
        let Some(before) = before else { continue };
        if trips <= before || g == 0 {
            continue;
        }
        let gi = usize::from(g.min(3)) - 1;
        s.by_gen[gi] += 1;
        s.harvest += u32::from(g.min(3));
        s.pops.push((id, frame, g.min(3)));
    }
    s.pops.retain(|&(_, f, _)| frame.saturating_sub(f) <= 150);
    if let Some(f) = food {
        s.larder = f;
        s.larder_peak = s.larder_peak.max(f);
    }
    s.low = s.low.min(alive);
    if !s.stakes || s.outcome.is_some() {
        return;
    }
    let floor = (s.alive_at_start as f32 * s.floor).ceil() as usize;
    let end = s.season_start + s.season;
    if me.is_none() {
        s.outcome = Some(SeasonEnd::Died(frame));
        notes.push("YOU STARVED -- THE SEASON GOES ON WITHOUT YOU".to_string());
    } else if alive == 0 || (!s.heap_sealed && alive < floor) || (s.heap_sealed && frame >= end && alive < floor) {
        s.outcome = Some(SeasonEnd::Starved(frame));
        s.spring = alive;
        notes.push(if s.heap_sealed {
            format!("SPRING: ONLY {alive} CAME THROUGH, {floor} NEEDED -- THE SEASON IS LOST")
        } else {
            format!("THE COLONY FELL TO {alive}, UNDER {floor} -- THE SEASON IS LOST")
        });
    } else if frame >= end {
        s.outcome = Some(SeasonEnd::Endured(alive));
        notes.push(format!("SPRING: {alive} OF {} CAME THROUGH -- YOUR TREE BROUGHT {} HOME", s.alive_at_start, s.harvest));
    }
    if let Some(o) = s.outcome {
        lab.world.log_player_action(format!("scout demo: the lean season ended, {o:?}"));
    }
    for n in notes {
        lab.ui.say(n);
    }
}

/// The pointer over the world; the scout seat does not take it.
pub fn mouse(_lab: &mut Lab, _cell: (i32, i32), _act: super::MouseAct) -> bool {
    false
}

const WHITE: [u8; 4] = [255, 255, 255, 255];
/// **"Knows the heap"**: violet, not the orange it was (review 2026-10-30,
/// item 3) -- the surface is strewn with yellow-orange provisions, and at
/// zoom 4 an orange ant was a crumb of food.
const KNOWS: [u8; 4] = [200, 120, 255, 255];
const WINTER: [u8; 4] = [160, 210, 255, 255];
const GREEN: [u8; 4] = [80, 230, 90, 255];
const AMBER: [u8; 4] = [255, 200, 40, 255];
const GREY: [u8; 4] = [150, 150, 150, 255];
const RED: [u8; 4] = [235, 60, 50, 255];
const TREE: [u8; 4] = [120, 200, 255, 255];
/// The sealed heap's rim and lock: the "knows the heap" violet, a shade
/// paler so a taught ant on the heap still reads against it.
const LOCK: [u8; 4] = [225, 175, 255, 255];

/// **Marks over the world**: a full replace on fixed colours (CLAUDE.md: a
/// debug readout must not be a blend of the thing it shows). The player
/// white with a ring; ants that know the heap violet by how well; a pip over
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
            let c = [(KNOWS[0] as f32 * k) as u8, (KNOWS[1] as f32 * k) as u8, (KNOWS[2] as f32 * k) as u8, 255];
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
    // **Her tree's harvest, where it happens**: a `+1` (a grand-pupil's
    // `+2`) rising over each tree ant the moment she brings far-heap food home.
    if let Some(s) = lab.demo.as_ref().and_then(|d| d.scout.as_ref()) {
        for &(id, f, g) in &s.pops {
            let Some(h) = head(world, id) else { continue };
            let (x, y) = at(h.0, h.1);
            let rise = (world.frame.saturating_sub(f) / 10) as i32;
            let y = y - 10 - rise;
            if y > 0 && y + crate::hud::GLYPH_HEIGHT < top {
                let text = format!("+{g}");
                hc.fill_rect(frame_buf, x - 2, y - 1, crate::hud::text_width(&text) + 3, crate::hud::GLYPH_HEIGHT + 2, [10, 12, 16, 255]);
                hc.text(frame_buf, x, y, &text, GREEN);
            }
        }
    }
    // **The sealed heap**: its top cells rimmed and a lock over it, in the violet of "knows the heap" (violet
    // opens violet); a green tick over a taught ant as she takes food from it, a red arrow over an untaught one
    // as she is turned away, pointing the way she is sent.
    if let Some(s) = lab.demo.as_ref().and_then(|d| d.scout.as_ref()).filter(|s| seal_live(s, world.frame)) {
        if let Some(food) = world.materials.id_of("provisions") {
            let h = lab.spec.height;
            let mut peak: Option<(i32, i32)> = None;
            for x in s.heap_x - SEAL_R..=s.heap_x + SEAL_R {
                if let Some(y) = (0..h).find(|&y| world.in_bounds(x, y) && world.get(x, y).material == food) {
                    cell(frame_buf, x, y, LOCK);
                    if peak.is_none_or(|p| y < p.1) {
                        peak = Some((x, y));
                    }
                }
            }
            let top_y = peak.map(|p| p.1).or_else(|| world.room_surface_at(s.heap_x)).unwrap_or(0);
            let (lx, ly) = at(s.heap_x, top_y);
            let (bx, by) = (lx - 4, ly - 14);
            if by - 6 > 0 && by + 6 < top {
                hc.fill_rect(frame_buf, bx - 1, by - 7, 11, 14, [10, 12, 16, 255]);
                hc.fill_rect(frame_buf, bx, by, 9, 6, LOCK);
                hc.fill_rect(frame_buf, bx + 4, by + 2, 1, 2, [10, 12, 16, 255]);
                for dy in 1..=4 {
                    dot(frame_buf, (bx + 2, by - dy), LOCK);
                    dot(frame_buf, (bx + 6, by - dy), LOCK);
                }
                for dx in 3..=5 {
                    dot(frame_buf, (bx + dx, by - 5), LOCK);
                }
                let label = "SEALED";
                hc.fill_rect(frame_buf, bx + 11, by - 1, crate::hud::text_width(label) + 3, crate::hud::GLYPH_HEIGHT + 2, [10, 12, 16, 255]);
                hc.text(frame_buf, bx + 13, by, label, LOCK);
            }
        }
        for &(id, carried, f) in &s.seal.marks {
            let Some(hd) = head(world, id) else { continue };
            let (x, y) = at(hd.0, hd.1 - 4);
            if carried {
                for (dx, dy) in [(-2, 0), (-1, 1), (0, 2), (1, 1), (2, 0), (3, -1), (4, -2)] {
                    dot(frame_buf, (x + dx, y + dy), GREEN);
                    dot(frame_buf, (x + dx, y + dy - 1), GREEN);
                }
            } else if world.frame.saturating_sub(f) <= 60 {
                let d = s.seal.bounced.get(&id).map_or(-1, |&(_, d)| i32::from(d));
                for i in -3..=3 {
                    dot(frame_buf, (x + i, y), RED);
                }
                for k in 1..=2 {
                    dot(frame_buf, (x + d * (3 - k), y - k), RED);
                    dot(frame_buf, (x + d * (3 - k), y + k), RED);
                }
            }
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

/// **The scout's panel**, in the sky at the top with a dark backing
/// (review 2026-10-30: at the bottom its sixth line hid under the tab strip,
/// and with no backing it was unreadable over the mound): the season and the
/// colony against its floor, the larder, her tree's harvest, her bank and her
/// pupil's, the run counters; in a sealed arm, nothing that names the arm.
pub fn draw_hud(lab: &Lab, hc: crate::render::Hud, frame_buf: &mut [u8]) {
    let lines = panel(lab);
    if lines.is_empty() {
        return;
    }
    let row = crate::hud::GLYPH_HEIGHT + 3;
    let (x, top) = (4, 14);
    let w = lines.iter().map(|(t, _)| crate::hud::text_width(t)).max().unwrap_or(0) + 8;
    hc.blend_rect(frame_buf, x, top, w, row * lines.len() as i32 + 4, [14, 16, 20, 255], 0.72);
    for (i, (t, c)) in lines.iter().enumerate() {
        hc.text(frame_buf, x + 4, top + 3 + row * i as i32, t, *c);
    }
    if let Some((title, card, good)) = end_card(lab) {
        let tw = 2 * crate::hud::text_width(&title);
        let w = card.iter().map(|t| crate::hud::text_width(t)).max().unwrap_or(0).max(tw) + 20;
        let h = 2 * crate::hud::GLYPH_HEIGHT + 16 + row * card.len() as i32;
        let x = (super::super::WIDTH as i32 - w) / 2;
        let y = top + row * lines.len() as i32 + 16;
        hc.blend_rect(frame_buf, x, y, w, h, [10, 12, 16, 255], 0.88);
        let edge = if good { GREEN } else { RED };
        hc.fill_rect(frame_buf, x, y, w, 2, edge);
        hc.fill_rect(frame_buf, x, y + h - 2, w, 2, edge);
        big_text(hc, frame_buf, x + (w - tw) / 2, y + 6, &title, edge);
        for (i, t) in card.iter().enumerate() {
            hc.text(frame_buf, x + 10, y + 2 * crate::hud::GLYPH_HEIGHT + 12 + row * i as i32, t, if i + 1 == card.len() { WHITE } else { GREY });
        }
    }
}

/// A text bar of `n` cells, `left` of it full.
fn bar(left: f32, n: usize) -> String {
    let k = (left.clamp(0.0, 1.0) * n as f32).round() as usize;
    format!("[{}{}]", "=".repeat(k), ".".repeat(n - k))
}

/// The panel's lines, as `(text, colour)`; also what the harness prints.
pub fn panel(lab: &Lab) -> Vec<(String, [u8; 4])> {
    let Some(s) = lab.demo.as_ref().and_then(|d| d.scout.as_ref()) else { return Vec::new() };
    let mut out = Vec::new();
    if !s.started {
        out.push((format!("SCOUT DEMO, ARM {} -- THE COLONY IS SETTLING", s.label), WHITE));
        return out;
    }
    let Some(t) = lab.world.tandem.as_deref() else { return out };
    let world = &lab.world;
    let alive = ants(world).len();
    if s.stakes {
        let end = s.season_start + s.season;
        let left = end.saturating_sub(world.frame);
        let floor = (s.alive_at_start as f32 * s.floor).ceil() as usize;
        let clock = if left > 0 { format!("LEAN SEASON {} SPRING IN {left}", bar(left as f32 / s.season.max(1) as f32, 10)) } else { "SPRING: THE LEAN SEASON IS OVER".to_string() };
        out.push((clock, WINTER));
        let danger = alive < floor + floor / 5 + 1;
        let rule = if s.heap_sealed { format!("SPRING NEEDS {floor}") } else { format!("LOST BELOW {floor}") };
        out.push((format!("COLONY {alive} OF {} -- {rule}", s.alive_at_start), if danger { RED } else { WHITE }));
        out.push((format!("STORES {} {:.0} TANKS IN THEIR BELLIES", bar(s.larder / s.larder_peak.max(1.0), 10), s.larder), WINTER));
        out.push((format!("YOUR TREE'S HARVEST {} -- PUPILS {}, THEIRS {} (X2), ON {} (X3)", s.harvest, s.by_gen[0], s.by_gen[1], s.by_gen[2]), GREEN));
        if s.heap_sealed {
            out.push((format!("SEALED HEAP: ONLY THE TAUGHT CARRY -- CARRIED {}, TURNED AWAY {}", s.seal.carried, s.seal.turned_away), KNOWS));
        }
    } else {
        let income = t.income_over(world.frame, 3_600);
        let bar: String = std::iter::repeat_n('|', income.min(40)).collect();
        out.push((format!("FOOD HOME FROM THE FAR HEAP, LAST MINUTE: {income} {bar}"), KNOWS));
    }
    let tanks = |id: OrganismId| tandem::energy_sense(world, id).map_or(0.0, |s| s / (1.0 - s).max(1e-3));
    let me = steer::steered_id(world);
    let mine = me.map_or("YOU HAVE DIED".to_string(), |id| format!("YOUR BANK {:.1} TANKS", tanks(id)));
    let low = me.is_some_and(|id| tanks(id) < 1.0);
    let pupil = t.pairs.iter().find(|p| Some(p.leader) == me).map_or(String::new(), |p| format!(" -- PUPIL {:.1} TANKS", tanks(p.follower)));
    out.push((format!("ARM {} -- {mine}{pupil}", s.label), if low { RED } else { WHITE }));
    let c = &t.counts;
    out.push((format!("RUNS {} STARTED, {} ARRIVED, {} LOST ({} FOUND IT ANYWAY)", c.started, c.done, c.broken, c.broken_still_arrived), GREEN));
    out.push((format!("KNOW THE HEAP {} -- STARVED WHILE LED {} -- TAPS {} REFUSED {}", t.lessons.len(), c.starved_while_led, c.taps, c.refusals), TREE));
    if !s.stakes {
        out.push((format!("COLONY {alive} (WAS {} AT HATCH)", s.alive_at_start), GREY));
    }
    if !s.sealed {
        let gain = world.recall.as_ref().map_or(0.0, |r| r.gain);
        out.push((
            format!("KNOBS: C {:.2}/TRIP, CASCADE RUNS {}, PAUSE 1/{:.0} STEPS, MEMORY PULL {gain:.0} (SCRIPTED)", t.knobs.cascade_c, c.cascade_runs, 1.0 / t.knobs.pause_chance),
            GREY,
        ));
    }
    out
}

/// The end card: a title, its lines, and whether it is good news.
pub fn end_card(lab: &Lab) -> Option<(String, Vec<String>, bool)> {
    let s = lab.demo.as_ref().and_then(|d| d.scout.as_ref())?;
    let o = s.outcome?;
    let floor = (s.alive_at_start as f32 * s.floor).ceil() as usize;
    let (title, good) = match o {
        SeasonEnd::Endured(_) => ("SPRING CAME".to_string(), true),
        SeasonEnd::Starved(_) => ("THE COLONY STARVED".to_string(), false),
        SeasonEnd::Died(_) => ("YOU STARVED".to_string(), false),
    };
    let when = |f: u64| f.saturating_sub(s.season_start);
    let first = match o {
        SeasonEnd::Endured(n) => format!("{n} OF {} CAME THROUGH THE LEAN SEASON (FLOOR {floor})", s.alive_at_start),
        SeasonEnd::Starved(_) if s.heap_sealed => format!("{} OF {} CAME THROUGH -- SPRING NEEDED {floor}", s.spring, s.alive_at_start),
        SeasonEnd::Starved(f) => format!("UNDER {floor} OF {} AFTER {} OF {} FRAMES", s.alive_at_start, when(f), s.season),
        SeasonEnd::Died(f) => format!("YOUR BANK RAN OUT AFTER {} OF {} FRAMES", when(f), s.season),
    };
    let lines = vec![
        first,
        format!("YOUR TREE'S HARVEST: {} POINTS", s.harvest),
        format!("PUPILS' TRIPS {} -- THEIR PUPILS' {} -- LATER {}", s.by_gen[0], s.by_gen[1], s.by_gen[2]),
        format!("FEWEST ALIVE {}", s.low),
        "R: A NEW SEASON".to_string(),
    ];
    Some((title, lines, good))
}

/// Text at twice the size: drawn small into a scratch buffer, then each lit
/// pixel as a 2x2 block.
fn big_text(hc: crate::render::Hud, frame_buf: &mut [u8], x: i32, y: i32, text: &str, c: [u8; 4]) {
    let w = crate::hud::text_width(text).max(1) as u32;
    let h = crate::hud::GLYPH_HEIGHT as u32;
    let mut tmp = vec![0u8; (w * h * 4) as usize];
    crate::hud::draw_text(&mut tmp, w, h, 0, 0, text, [255, 255, 255, 255]);
    for py in 0..h {
        for px in 0..w {
            if tmp[((py * w + px) * 4 + 3) as usize] > 0 {
                hc.fill_rect(frame_buf, x + 2 * px as i32, y + 2 * py as i32, 2, 2, c);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::demo::{Demo, DemoSeat};
    use crate::lab::scenario::Placement;

    /// The trickle's and the far heap's repeating events, as `(cells, until)`.
    fn heaps(lab: &Lab) -> ((i32, u64), (i32, u64)) {
        let sc = lab.scenario.as_ref().expect("the scenario stays loaded");
        let find = |want: i32| {
            sc.timeline
                .iter()
                .find_map(|e| match e.what {
                    Placement::Heap { x, cells, .. } if x == want && e.every > 0 => Some((cells, e.until)),
                    _ => None,
                })
                .expect("the heap event")
        };
        (find(TRICKLE_X), find(HEAP_X))
    }

    /// **The lean season starts at the hand-over**: the near trickle gets an
    /// `until` (so no more drops) and the far heap its lean-season size;
    /// `stakes=0` leaves both as the sandbox had them (the positive control:
    /// the same call, the other answer).
    #[test]
    fn the_lean_season_stops_the_trickle_and_stakes_0_keeps_it() {
        for stakes in [true, false] {
            let sc = crate::lab::scenario::Scenario::load("demo_scout").expect("demo_scout ships");
            let mut lab = Lab::new(sc.bed.clone());
            lab.load_scenario(sc);
            lab.tick_for_harness();
            let mut args = BTreeMap::new();
            args.insert("stakes".to_string(), if stakes { "1" } else { "0" }.to_string());
            let mut demo = Demo::new(DemoSeat::Scout);
            demo.scout = Some(ScoutDemo::from_args(&args, 1));
            lab.demo = Some(demo);
            let before = heaps(&lab);
            start_season(&mut lab);
            let (trickle, far) = heaps(&lab);
            if stakes {
                assert_eq!(trickle.1, lab.world.frame, "the near trickle did not stop");
                assert_eq!(far.0, LEAN_HEAP as i32, "the far heap is not the lean season's");
            } else {
                assert_eq!((trickle, far), before, "stakes=0 changed the box");
            }
        }
    }

    /// **The season arms the cascade**: leaders unstuck, a timeout, the
    /// lean-season chance; `stakes=0` leaves the sandbox's knobs (the control).
    #[test]
    fn the_season_unsticks_the_cascade_and_stakes_0_does_not() {
        for stakes in [true, false] {
            let sc = crate::lab::scenario::Scenario::load("demo_scout").expect("demo_scout ships");
            let mut lab = Lab::new(sc.bed.clone());
            lab.load_scenario(sc);
            lab.tick_for_harness();
            let mut args = BTreeMap::new();
            args.insert("stakes".to_string(), if stakes { "1" } else { "0" }.to_string());
            let mut demo = Demo::new(DemoSeat::Scout);
            demo.scout = Some(ScoutDemo::from_args(&args, 1));
            lab.demo = Some(demo);
            start_tandems(&mut lab.world, Arm::Scout, None, GAIN);
            start_season(&mut lab);
            let k = &lab.world.tandem.as_deref().expect("armed").knobs;
            let got = (k.cascade_unstick, k.cascade_timeout, k.cascade_c);
            if stakes {
                assert_eq!(got, (true, CASCADE_TIMEOUT, LEAN_C));
            } else {
                assert_eq!(got, (false, 0, tandem::Knobs::default().cascade_c));
            }
        }
    }

    /// **A boxed cascade leader tries another heading, and a run that never
    /// arrives is given up** (round 2: every round-1 cascade run stood at the
    /// mound until the pupil starved). The same pair with the knobs off is the
    /// control: same heading after 90 frames of no progress, and still paired.
    #[test]
    fn a_stuck_cascade_leader_turns_and_her_run_times_out() {
        for on in [true, false] {
            let sc = crate::lab::scenario::Scenario::load("demo_scout").expect("demo_scout ships");
            let mut lab = Lab::new(sc.bed.clone());
            lab.load_scenario(sc);
            lab.set_new_ant(true);
            while lab.world.frame < 600 {
                lab.tick_for_harness();
            }
            let mut t = Tandems::new(Arm::Scout, Some(Patch { x: HEAP_X, r: 10 }));
            (t.knobs.weak, t.knobs.accept_hi, t.knobs.accept_lo, t.knobs.accept_full) = (0.0, 1.0, 1.0, 1.0);
            t.knobs.cascade_unstick = on;
            t.knobs.cascade_timeout = if on { 100 } else { 0 };
            lab.world.tandem = Some(Box::new(t));
            let w = &mut lab.world;
            let mut ids = ants(w);
            ids.sort_unstable();
            let leader = ids
                .into_iter()
                .find(|&id| crate::sim::creature::mind::has(w, id) && head(w, id).is_some_and(|h| h.0 < HEAP_X - 20) && !tandem::candidates(w, id, 3).is_empty())
                .expect("an ant west of the heap with a sister in reach");
            assert!(matches!(tandem::tap(w, leader), TapResult::Accepted(_)));
            w.tandem.as_mut().unwrap().pairs[0].cascade = true;
            let y = head(w, leader).unwrap().1;
            let frame = w.frame;
            w.organism_mut(leader).unwrap().last_food = Some((HEAP_X, y, frame));
            let dir = |w: &World| w.mind_force.get(&leader).and_then(|f| f.pick.dir);
            tandem::steer_cascade_leaders(w);
            let first = dir(w);
            assert_eq!(first, Some((1, 0)), "she heads for the heap");
            w.frame += 90;
            tandem::steer_cascade_leaders(w);
            assert_eq!(dir(w), if on { Some((0, -1)) } else { first }, "unstick {on}");
            w.frame += 200;
            tandem::tick(w);
            let paired = w.tandem.as_deref().unwrap().pairs.iter().any(|p| p.leader == leader);
            assert_eq!(paired, !on, "timeout {on}");
        }
    }

    /// **The sealed heap turns away an ant who does not know it** (round 3):
    /// an untaught ant with heap food all round her head is walked off and
    /// never takes a cell of it; the same ant taught, or the same box with
    /// `sealed=0` (the positive control: the food is reachable and she does
    /// take it), is left to her own mind.
    #[test]
    fn the_sealed_heap_turns_away_the_untaught_and_not_the_taught() {
        let mut took = BTreeMap::new();
        for case in ["untaught", "taught", "open"] {
            let sc = crate::lab::scenario::Scenario::load("demo_scout").expect("demo_scout ships");
            let mut lab = Lab::new(sc.bed.clone());
            lab.load_scenario(sc);
            lab.set_new_ant(true);
            while lab.world.frame < 600 {
                lab.tick_for_harness();
            }
            let w = &lab.world;
            let mut ids = ants(w);
            ids.sort_unstable();
            let id = ids
                .into_iter()
                .find(|&id| {
                    crate::sim::creature::mind::has(w, id)
                        && w.organism(id).is_some_and(|s| s.crop.is_none_or(|c| c.cells == 0))
                        && head(w, id).is_some_and(|h| w.room_surface_at(h.0).is_some_and(|top| h.1 <= top + 1))
                })
                .expect("a minded ant on the surface with an empty crop");
            let h = head(&lab.world, id).unwrap();
            // The heap's food on every free cell round her head: a pick-up is one decision away.
            let food = lab.world.materials.id_of("provisions").expect("provisions");
            for dx in -2..=2 {
                for dy in -2..=2 {
                    if lab.world.is_empty(h.0 + dx, h.1 + dy) {
                        lab.world.set(h.0 + dx, h.1 + dy, crate::sim::cell::Cell::new(food, 0));
                    }
                }
            }
            let mut args = BTreeMap::new();
            if case == "open" {
                args.insert("sealed".to_string(), "0".to_string());
            }
            let mut seat = ScoutDemo::from_args(&args, 1);
            (seat.started, seat.season_start, seat.heap_x) = (true, lab.world.frame, h.0);
            let mut demo = Demo::new(DemoSeat::Scout);
            demo.scout = Some(seat);
            lab.demo = Some(demo);
            start_tandems(&mut lab.world, Arm::Scout, None, GAIN);
            if case == "taught" {
                lab.world.tandem.as_mut().unwrap().lessons.insert(id, 1.0);
            }
            let start = lab.world.frame;
            lab.tick_for_harness();
            let forced = lab.world.mind_force.contains_key(&id);
            let mut taken = false;
            let mut anyone = false;
            for _ in 0..600 {
                lab.tick_for_harness();
                let took = |w: &World, a: OrganismId| w.organism(a).and_then(|s| s.last_food).is_some_and(|(x, _, f)| f >= start && (x - h.0).abs() <= SEAL_R);
                taken |= took(&lab.world, id);
                anyone |= ants(&lab.world).into_iter().any(|a| took(&lab.world, a));
            }
            let seal = &lab.demo.as_ref().unwrap().scout.as_ref().unwrap().seal;
            assert_eq!(forced, case == "untaught", "{case}: turned away {forced}");
            // A leak is a sister who took food that moved after the pre-step look (a taught ant's drop); she is
            // turned away at once, so it is one cell, and never the ant the food was laid round.
            assert!(seal.leak_log.iter().all(|l| l.0 != id || case != "untaught") && seal.leak_log.iter().all(|l| seal.turned_ids.contains(&l.0)), "{case}: leaks {:?}", seal.leak_log);
            if case == "untaught" {
                assert_eq!(seal.leaks, 0, "an ant who does not know the heap carried from it: {:?}", seal.leak_log);
                assert!(seal.turned_away >= 1 && seal.turned_ids.contains(&id) && !taken, "{case}: {seal:?}, took {taken}");
            }
            if case == "taught" {
                assert!(seal.carried >= 1, "a taught ant took from the sealed heap and no carry was booked");
            }
            if case == "open" {
                assert_eq!(seal.turned_away, 0, "sealed=0 turned an ant away");
            }
            took.insert(case, (taken, anyone));
        }
        // The positive control: without the seal the food is taken (by her or a sister), and taught she takes it.
        assert!(took["open"].1 && took["taught"].0, "food all round her and nobody took it: the test proves nothing ({took:?})");
    }

    /// **The tree's generations**: her pupil is 1, that pupil's pupil 2.
    #[test]
    fn a_pupils_pupil_is_the_second_generation() {
        let mut t = Tandems::new(Arm::Scout, Some(Patch { x: HEAP_X, r: 10 }));
        // She is 1 (never a pupil); 2 is hers, 3 is 2's, 4 is 3's.
        t.tree.insert(2, 1);
        t.tree.insert(3, 2);
        t.tree.insert(4, 3);
        assert_eq!(generation(&t, 2), 1);
        assert_eq!(generation(&t, 3), 2);
        assert_eq!(generation(&t, 4), 3);
    }
}

