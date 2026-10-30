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
//! - **Winters** (the stakes, 2026-10-30; `stakes=0` is the sandbox before
//!   them): food comes in seasons. A summer of heaps, then a winter of none,
//!   [`WINTERS`] times, each winter longer ([`Seasons`]). The score is the
//!   workers alive at the end of each winter; she loses if she dies or a
//!   winter ends with no worker, and her rule never lapses. A wild
//!   egg costs her [`WILD_COST`] joules more than an ordinary one, so a
//!   gamble is fewer eggs. The review that asked for it:
//!   `/mnt/project-files/seat-demos/review-2026-10-30/queen/review.md` E1.

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
pub const SEAT_KEYS: &str = "P WILD EGG (AGAIN: FERAL) -- Y NEXT 5 EGGS HER LINE (LOOK-CLICK OR K) -- J DEED -- K ROW -- U COLOURS -- R NEW COLONY";

/// **Her hold, in frames past her last egg.** Provisional until the scripted
/// founding (`examples/queendemo.rs`, M0c) measures her lay interval; the
/// plan sets it at that interval's p90 x 1.5.
pub const HOLD: u64 = 6_000;

/// **The hold with the stakes on**: long enough never to lapse in a game.
pub const RULE_FOR_LIFE: u64 = u64::MAX / 4;

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
/// ant's own brown do not use. **No red, yellow or orange** (review
/// 2026-10-30, item 6): the provisions heap by the nest door is those three,
/// and a 2-4 cell ant in them read as food. No yellow-green either, which sat
/// on the stock ant's olive.
const PALETTE: [[f32; 3]; 8] = [
    [80.0, 200.0, 255.0],
    [190.0, 110.0, 255.0],
    [255.0, 120.0, 230.0],
    [140.0, 255.0, 255.0],
    [255.0, 255.0, 255.0],
    [120.0, 130.0, 255.0],
    [90.0, 255.0, 160.0],
    [255.0, 70.0, 170.0],
];

/// **The extra joules a wild egg costs her**, mild then feral, taken from her
/// bank the frame the egg is laid (the lay bar is 1,100 J). A gamble is
/// fewer ordinary eggs, and a feral one nearly a whole egg's worth.
pub const WILD_COST: [f32; 2] = [300.0, 700.0];

/// **The seasons**, in frames from the frame she is armed: the first summer
/// (founding), then winter and summer in turn, each winter [`Seasons::grow`]
/// longer than the last, [`WINTERS`] winters in all; the summer after the
/// last runs for ever. Set from the paired sweep in
/// `/mnt/project-files/seat-demos/build-2026-10-30/queen/README.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seasons {
    pub first: u64,
    pub summer: u64,
    pub winter: u64,
    pub grow: u64,
}

/// How many winters a game has.
pub const WINTERS: u32 = 3;

impl Default for Seasons {
    fn default() -> Seasons {
        Seasons { first: 14_000, summer: 8_000, winter: 6_000, grow: 2_000 }
    }
}

/// Where the clock is: the season's number (summer 1 is the founding), whether
/// it is winter, and its first and last frame (from arming; `u64::MAX` for
/// the endless last summer).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Season {
    pub n: u32,
    pub winter: bool,
    pub start: u64,
    pub end: u64,
}

impl Seasons {
    /// Read `first= summer= winter= grow=` from the demo's args.
    pub fn from_args(args: &std::collections::BTreeMap<String, String>) -> Seasons {
        let d = Seasons::default();
        let get = |k: &str, v: u64| args.get(k).and_then(|s| s.parse().ok()).unwrap_or(v);
        Seasons { first: get("first", d.first), summer: get("summer", d.summer), winter: get("winter", d.winter), grow: get("grow", d.grow) }
    }

    /// Every season in order, ending with the endless summer.
    pub fn list(&self) -> Vec<Season> {
        let mut out = Vec::new();
        let mut t = 0;
        for n in 1..=WINTERS {
            let s = if n == 1 { self.first } else { self.summer };
            out.push(Season { n, winter: false, start: t, end: t + s });
            t += s;
            let w = self.winter + self.grow * u64::from(n - 1);
            out.push(Season { n, winter: true, start: t, end: t + w });
            t += w;
        }
        out.push(Season { n: WINTERS + 1, winter: false, start: t, end: u64::MAX });
        out
    }

    /// The season at `t` frames from arming.
    pub fn at(&self, t: u64) -> Season {
        *self.list().iter().find(|s| t < s.end).expect("the last summer is endless")
    }
}

/// How the game ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// She died, in this season.
    Lost(Season),
    /// This winter ended with no worker alive.
    StarvedOut(Season),
    /// Every winter came through.
    Endured,
}

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
    /// **The stakes** (on by default; `stakes=0` is the sandbox before them).
    pub stakes: bool,
    pub seasons: Seasons,
    /// The frame she was armed: the season clock's zero.
    pub start: u64,
    /// The season at the last count, for the turn notices.
    pub season: Option<Season>,
    /// Workers alive at the end of each winter come through: the score.
    pub winters: Vec<u32>,
    pub outcome: Option<Outcome>,
    /// The frame the game ended on.
    pub ended: u64,
    /// Food cells (provisions) in the box at the last count: the colony's
    /// stores, heap and larder both, which no longer refill in winter.
    pub food: u32,
    /// The most food the box held this game, for the meter's scale.
    pub food_peak: u32,
    food_at: u64,
    /// Her bank at the last count, in joules.
    pub bank: f32,
    /// Joules her wild eggs have cost her, over the game.
    pub wild_joules: f32,
    /// The frame `R` was first pressed mid-game (a second press restarts).
    restart_asked: u64,
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
    let args = lab.demo.as_ref().map(|d| d.args.clone()).unwrap_or_default();
    let stakes = args.get("stakes").is_none_or(|v| v != "0");
    let seasons = Seasons::from_args(&args);
    if stakes {
        seasonal_food(lab, &seasons, frame);
        // **Her rule holds while she lives** (stakes on): the hold never
        // lapses, so only she lays. Measured (8 paired seeds, the README):
        // with the lapse, a let-go queen's workers bred on what they foraged
        // and out-grew the scripted player on 5 of 8 seeds even through the
        // winters -- the player was optional, which is the review's headline
        // complaint. Without it, her eggs are the colony's only growth.
        if let Some(s) = lab.world.steered.as_mut() {
            s.hold = Some(RULE_FOR_LIFE);
        }
    }
    let q = state_mut(lab).expect("read above");
    q.ready = true;
    q.colony = colony;
    q.stakes = stakes;
    q.seasons = seasons;
    q.start = frame;
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
    if stakes {
        lab.ui.say("WINTER IS COMING: NO FOOD FALLS THEN. RAISE WORKERS, NOT MOUTHS");
    }
}

/// **Food in seasons**: the scenario's repeating heap becomes one repeating
/// heap per summer, with nothing in the winters between (the timeline's own
/// `at`/`every`/`until`, as `feast_and_famine.ron` schedules boom and bust).
/// The one-shot events stay as they are.
fn seasonal_food(lab: &mut Lab, seasons: &Seasons, start: u64) {
    use crate::lab::scenario::{Event, Placement};
    let Some(sc) = lab.scenario.as_mut() else { return };
    let (heaps, mut keep): (Vec<Event>, Vec<Event>) = std::mem::take(&mut sc.timeline).into_iter().partition(|e| e.every > 0 && matches!(e.what, Placement::Heap { .. }));
    for e in heaps {
        for season in seasons.list().into_iter().filter(|s| !s.winter) {
            // A later summer's first heap falls on its first frame; the
            // founding summer's already fell with her, so it goes on as it was.
            let at = if season.n == 1 { e.at + e.every * ((start + 1).saturating_sub(e.at)).div_ceil(e.every) } else { start + season.start };
            let until = if season.end == u64::MAX { 0 } else { start + season.end - 1 };
            keep.push(Event { at, every: e.every, until, what: e.what.clone() });
        }
    }
    sc.timeline = keep;
}

/// The line `m` belongs to: a wild line's index, or 0 (the stock).
fn line_of(lines: &[Line], m: &Arc<MindGenome>) -> usize {
    lines.iter().position(|l| l.mind.as_ref().is_some_and(|x| Arc::ptr_eq(x, m))).unwrap_or(0)
}

/// One of the seat keys (`Y U P J K R`) was pressed.
pub fn seat_key(lab: &mut Lab, key: char) {
    if key == 'R' {
        restart(lab);
        return;
    }
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

/// **`R`: play again** -- at once when the game is over, else on a second
/// press within 3 seconds. The same args, so `stakes=0` and the season
/// lengths carry over.
fn restart(lab: &mut Lab) {
    let frame = lab.world.frame;
    let over = state(lab).is_some_and(|q| q.outcome.is_some());
    let asked = state(lab).map_or(0, |q| q.restart_asked);
    if !over && (asked == 0 || frame > asked + 180) {
        if let Some(q) = state_mut(lab) {
            q.restart_asked = frame.max(1);
        }
        lab.ui.say("R AGAIN TO START A NEW COLONY");
        return;
    }
    let args = lab.demo.as_ref().map(|d| d.args.clone()).unwrap_or_default();
    let msg = lab.start_demo_with(super::DemoSeat::Queen, args);
    lab.world.log_player_action("queen demo: R, a new colony");
    lab.ui.say(msg);
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

/// How many ledger rows the panel shows, the stock's included.
const ROWS: usize = 7;

/// The ledger's rows: the stock, then each wild line in birth order -- **a
/// line that ended keeps a grey row** (review 2026-10-30: a gamble that
/// resolved between two looks left nothing behind). Past [`ROWS`], the oldest
/// ended lines give way first, then the oldest living.
fn rows(q: &QueenDemo) -> Vec<usize> {
    let mut wild: Vec<usize> = (1..q.lines.len()).collect();
    while wild.len() > ROWS - 1 {
        let drop = wild.iter().position(|&i| q.lines[i].gone).unwrap_or(0);
        wild.remove(drop);
    }
    std::iter::once(0).chain(wild).collect()
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
    let queen = steer::steered_id(&lab.world);
    let mut wild_cost = 0.0;
    {
        let q = state_mut(lab).expect("ready");
        q.log_seen += fresh.len();
        for e in &fresh {
            if q.stakes && e.wild > 0 && Some(e.layer) == queen {
                wild_cost += WILD_COST[usize::from(e.wild.clamp(1, 2)) - 1];
            }
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
    // **A wild egg's price**, out of her bank, booked as a negative grant so
    // the colony's ledger still closes (`steer::grant` takes only gifts).
    if let (Some(id), true) = (queen, wild_cost > 0.0) {
        let colony = state(lab).map_or(0, |q| q.colony);
        if let Some(st) = lab.world.organism_mut(id) {
            let take = wild_cost.min((st.energy - 1.0).max(0.0));
            st.energy -= take;
            lab.world.book(colony, crate::sim::world::Account::Granted, -f64::from(take));
            if let Some(q) = state_mut(lab) {
                q.wild_joules += take;
            }
            notes.push(format!("THE GAMBLE COST HER {take:.0} J OF HER OWN"));
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
        if state(lab).is_some_and(|q| q.stakes) {
            seasons_tick(lab, &mut notes);
        }
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

/// Food cells in the box: the heap by the door and whatever the workers
/// carried down, both of which only fall in summer now.
fn food_cells(world: &World) -> u32 {
    let Some(food) = world.materials.id_of("provisions") else { return 0 };
    let Some(b) = world.bounds() else { return 0 };
    let mut n = 0;
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            n += u32::from(world.get(x, y).material == food);
        }
    }
    n
}

/// **The season clock, the score and the end**, after each count: the
/// turn of a season is a notice; the end of a winter writes its survivors
/// into the score; her death ends the game.
fn seasons_tick(lab: &mut Lab, notes: &mut Vec<String>) {
    let frame = lab.world.frame;
    // The whole box is 164k cells: a look every 200 frames is plenty for a meter.
    let food = state(lab).is_some_and(|q| q.food_peak == 0 || frame >= q.food_at + 200).then(|| food_cells(&lab.world));
    let bank = steer::steered_id(&lab.world).and_then(|id| lab.world.organism(id)).map(|s| s.energy);
    // Dead: `World::steered` still names her, but she no longer lives.
    let dead = lab.world.steered.is_some() && bank.is_none();
    let q = state_mut(lab).expect("ready");
    if let Some(f) = food {
        q.food_at = frame;
        q.food = f;
        q.food_peak = q.food_peak.max(f);
    }
    q.bank = bank.unwrap_or(0.0);
    if q.outcome.is_some() {
        return;
    }
    let now = q.seasons.at(frame.saturating_sub(q.start));
    let was = q.season.replace(now);
    if dead {
        q.outcome = Some(Outcome::Lost(now));
        q.ended = frame;
        let when = if now.winter { format!("WINTER {}", now.n) } else { format!("SUMMER {}", now.n) };
        notes.push(format!("THE QUEEN IS DEAD, IN {when} -- R TO FOUND A NEW COLONY"));
        lab.world.log_player_action(format!("queen demo: she died in {when}"));
        return;
    }
    let Some(was) = was else { return };
    if was == now {
        return;
    }
    if now.winter {
        let len = now.end - now.start;
        notes.push(format!("WINTER {}: NO FOOD FOR {len} FRAMES -- EVERY EGG IS A MOUTH", now.n));
    } else {
        let w = q.tally.workers;
        q.winters.push(w);
        if w == 0 {
            // **Starved out**: no worker came through, so nobody fetches food
            // for her or her brood. The game ends here, with this winter's 0.
            q.outcome = Some(Outcome::StarvedOut(was));
            q.ended = frame;
            notes.push(format!("NO WORKER CAME THROUGH WINTER {} -- R TO FOUND A NEW COLONY", was.n));
            lab.world.log_player_action(format!("queen demo: the colony starved out in winter {}", was.n));
            return;
        }
        notes.push(format!("WINTER {} SURVIVED WITH {w} WORKERS -- FOOD FALLS AGAIN", was.n));
        if was.n >= WINTERS {
            q.outcome = Some(Outcome::Endured);
            q.ended = frame;
            notes.push(format!("THE COLONY ENDURED {WINTERS} WINTERS -- R TO PLAY AGAIN"));
        }
    }
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

const INK: [u8; 4] = [230, 232, 236, 255];
const DIM: [u8; 4] = [150, 156, 168, 255];
const GONE: [u8; 4] = [100, 104, 112, 255];
const SUMMER: [u8; 4] = [140, 225, 120, 255];
const WINTER: [u8; 4] = [160, 210, 255, 255];
const LAPSED: [u8; 4] = [255, 120, 90, 255];

/// `12345` as `12,345`.
fn thousands(n: u64) -> String {
    let d = n.to_string();
    let mut out = String::new();
    for (i, c) in d.chars().enumerate() {
        if i > 0 && (d.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A text bar of `n` cells, `left` of it full.
fn bar(left: f32, n: usize) -> String {
    let k = ((left.clamp(0.0, 1.0)) * n as f32).round() as usize;
    format!("[{}{}]", "=".repeat(k), ".".repeat(n - k))
}

/// **The status block** (top left): the season clock, the score, the stores,
/// the hold and her bank. Every line short enough to leave the top right to
/// the ledger.
pub fn status(lab: &Lab) -> Vec<(String, [u8; 4])> {
    let Some(q) = state(lab).filter(|q| q.ready) else { return Vec::new() };
    let mut out = Vec::new();
    let frame = lab.world.frame;
    if q.stakes {
        let t = frame.saturating_sub(q.start);
        let s = q.seasons.at(t);
        let line = match (s.winter, s.end) {
            _ if matches!(q.outcome, Some(Outcome::Lost(_) | Outcome::StarvedOut(_))) => ("THE GAME IS OVER -- R FOR A NEW COLONY".to_string(), LAPSED),
            (_, u64::MAX) => ("SPRING: THE WINTERS ARE OVER".to_string(), SUMMER),
            (true, end) => (format!("WINTER {} {} SPRING IN {}", s.n, bar((end - t) as f32 / (end - s.start) as f32, 10), thousands(end - t)), WINTER),
            (false, end) => (format!("SUMMER {} {} WINTER IN {}", s.n, bar((end - t) as f32 / (end - s.start) as f32, 10), thousands(end - t)), SUMMER),
        };
        out.push(line);
        let marks: Vec<String> = (1..=WINTERS)
            .map(|n| match q.winters.get(n as usize - 1) {
                Some(w) => format!("{w}"),
                None if s.winter && s.n == n && q.outcome.is_none() => "..".to_string(),
                None if matches!(q.outcome, Some(Outcome::Lost(l)) if l.n == n) => "X".to_string(),
                None => "--".to_string(),
            })
            .collect();
        out.push((format!("SCORE, WORKERS AT EACH WINTER'S END: {}", marks.join(" / ")), INK));
        let peak = q.food_peak.max(1) as f32;
        out.push((format!("FOOD {} {} CELLS", bar(q.food as f32 / peak, 10), q.food), if s.winter { WINTER } else { DIM }));
    }
    let left = steer::hold_left(&lab.world).unwrap_or(0.0);
    let store = lab.world.fathers.as_ref().and_then(|f| f.colonies.get(&q.colony));
    let (wild, queued) = store.map_or((0, 0), |s| (s.wild, s.queue.len()));
    let price = |l: usize| if q.stakes { format!(" -{:.0}J", WILD_COST[l]) } else { String::new() };
    let next = match (wild, queued) {
        (2, _) => format!("FERAL{}", price(1)),
        (1, _) => format!("WILD{}", price(0)),
        (_, 0) => "STOCK".to_string(),
        (_, n) => format!("KEPT x{n}"),
    };
    if q.stakes {
        out.push((format!("ONLY SHE LAYS -- NEXT EGG: {next}"), INK));
    } else {
        out.push((format!("HOLD {}  NEXT EGG: {next}", bar(left, 10)), if left > 0.0 { INK } else { LAPSED }));
    }
    let t = q.tally;
    let bank = if q.stakes && q.outcome.is_none() { format!("QUEEN {} J  ", thousands(q.bank as u64)) } else { String::new() };
    out.push((format!("{bank}WORKERS {}  BROOD {} ({} HUNGRY)", t.workers, t.brood, t.hungry_larvae), DIM));
    out
}

/// **The ledger block** (top right): each line's adults, brood and rate; a
/// line that ended stays, grey.
pub fn ledger(lab: &Lab) -> Vec<(String, [u8; 4])> {
    let Some(q) = state(lab).filter(|q| q.ready) else { return Vec::new() };
    let t = q.tally;
    let mut out = vec![(format!("EGGS {} ({} WILD)  LINE  AD BR {}/1K", t.eggs, t.wild_eggs, q.deed.name()), DIM)];
    out[0].0 = format!("{:<13}  AD  BR {}/1K", format!("EGGS {} W{}", t.eggs, t.wild_eggs), q.deed.name());
    for (r, &i) in rows(q).iter().enumerate() {
        let l = &q.lines[i];
        let mark = if r == q.cursor { ">" } else { " " };
        let name: String = l.name.chars().take(12).collect();
        let tag = if l.wild >= 2 { " F" } else if l.wild == 1 { " W" } else { "" };
        if l.gone {
            out.push((format!("{mark}{name:<12}  -- GONE{tag}"), GONE));
            continue;
        }
        let rate = l.rate(q.deed).map_or("  --".to_string(), |v| format!("{v:5.2}"));
        let colour = l.colour.map_or(INK, |c| [c[0] as u8, c[1] as u8, c[2] as u8, 255]);
        out.push((format!("{mark}{name:<12} {:>3} {:>3} {rate}{tag}", l.adults, l.brood), colour));
    }
    out
}

/// The panel's lines, top to bottom, as `(text, colour)`: the status block,
/// then the ledger. What the scripted player prints.
pub fn panel(lab: &Lab) -> Vec<(String, [u8; 4])> {
    let mut out = status(lab);
    out.extend(ledger(lab));
    out
}

/// The end card's lines: a title, then the score.
pub fn end_card(lab: &Lab) -> Option<(String, Vec<String>)> {
    let q = state(lab)?;
    let o = q.outcome?;
    let title = match o {
        Outcome::Lost(s) if s.winter => format!("THE QUEEN STARVED IN WINTER {}", s.n),
        Outcome::Lost(s) => format!("THE QUEEN DIED IN SUMMER {}", s.n),
        Outcome::StarvedOut(s) => format!("STARVED OUT IN WINTER {}", s.n),
        Outcome::Endured => format!("THE COLONY ENDURED {WINTERS} WINTERS"),
    };
    let marks: Vec<String> = q.winters.iter().enumerate().map(|(i, w)| format!("WINTER {}: {w}", i + 1)).collect();
    let mut lines = vec![
        format!("WINTERS SURVIVED: {} OF {WINTERS}", q.winters.iter().filter(|&&w| w > 0).count()),
        if marks.is_empty() { "NO WINTER SURVIVED".to_string() } else { format!("WORKERS AT EACH END -- {}", marks.join("  ")) },
        format!("EGGS {} -- {} WILD, COSTING HER {} J", q.tally.eggs, q.tally.wild_eggs, thousands(q.wild_joules as u64)),
        format!("LINES FOUNDED {}  -- STILL LIVING {}", q.lines.len() - 1, q.lines.iter().skip(1).filter(|l| !l.gone).count()),
    ];
    lines.push(if matches!(o, Outcome::Endured) { "R: PLAY AGAIN -- OR PLAY ON, THE FOOD STAYS".to_string() } else { "R: FOUND A NEW COLONY".to_string() });
    Some((title, lines))
}

/// Text at twice the size, for the end card's title: drawn small into a
/// scratch buffer, then each lit pixel as a 2x2 block.
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

/// **The queen's panels, in the sky** (review 2026-10-30: at the bottom left
/// the ledger sat on the nest's galleries; the top ~45% of her view is open
/// air). The status block top left, the ledger top right, and the end card
/// in the middle when the game is over.
pub fn draw_hud(lab: &Lab, hc: crate::render::Hud, frame_buf: &mut [u8]) {
    let row = crate::hud::GLYPH_HEIGHT + 3;
    crate::render::paint_owner::set("QUEEN");
    let top = 14;
    let block = |frame_buf: &mut [u8], x: i32, lines: &[(String, [u8; 4])]| {
        let w = lines.iter().map(|(t, _)| crate::hud::text_width(t)).max().unwrap_or(0) + 8;
        let x = if x < 0 { super::super::WIDTH as i32 - w - 4 } else { x };
        let h = row * lines.len() as i32 + 4;
        hc.blend_rect(frame_buf, x, top, w, h, [14, 16, 20, 255], 0.72);
        for (i, (t, c)) in lines.iter().enumerate() {
            hc.text(frame_buf, x + 4, top + 3 + row * i as i32, t, *c);
        }
    };
    let st = status(lab);
    if st.is_empty() {
        return;
    }
    block(frame_buf, 4, &st);
    block(frame_buf, -1, &ledger(lab));
    let show_card = state(lab).is_some_and(|q| match q.outcome {
        Some(Outcome::Lost(_) | Outcome::StarvedOut(_)) => true,
        Some(Outcome::Endured) => lab.world.frame < q.ended + 3_000,
        None => false,
    });
    if let (true, Some((title, lines))) = (show_card, end_card(lab)) {
        let lost = matches!(state(lab).and_then(|q| q.outcome), Some(Outcome::Lost(_) | Outcome::StarvedOut(_)));
        let tw = 2 * crate::hud::text_width(&title);
        let w = lines.iter().map(|t| crate::hud::text_width(t)).max().unwrap_or(0).max(tw) + 20;
        let h = 2 * crate::hud::GLYPH_HEIGHT + 16 + row * lines.len() as i32;
        let x = (super::super::WIDTH as i32 - w) / 2;
        // Under both blocks, so a long ledger is not covered.
        let y = top + row * st.len().max(1 + rows(state(lab).expect("ready")).len()) as i32 + 10;
        hc.blend_rect(frame_buf, x, y, w, h, [10, 12, 16, 255], 0.88);
        let edge = if lost { LAPSED } else { SUMMER };
        hc.fill_rect(frame_buf, x, y, w, 2, edge);
        hc.fill_rect(frame_buf, x, y + h - 2, w, 2, edge);
        big_text(hc, frame_buf, x + (w - tw) / 2, y + 6, &title, edge);
        for (i, t) in lines.iter().enumerate() {
            let c = if i + 1 == lines.len() { INK } else { DIM };
            hc.text(frame_buf, x + 10, y + 2 * crate::hud::GLYPH_HEIGHT + 12 + row * i as i32, t, c);
        }
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
        // The stakes are on by default: her rule is for life (`stakes=0`: `HOLD`).
        assert_eq!(lab.world.steered.as_ref().and_then(|s| s.hold), Some(RULE_FOR_LIFE));
        let wild = |lab: &Lab| lab.world.fathers.as_ref().and_then(|f| f.colonies.get(&q_colony(lab))).map(|s| s.wild);
        for want in [1, 2, 0] {
            assert!(lab.demo_key(DemoKey::Letter('P'), true));
            assert_eq!(wild(&lab), Some(want));
        }
    }

    /// **The stakes arm with her**: the repeating heap becomes one per
    /// summer, so no heap is due on any winter frame and some are due in
    /// every summer (counted, not assumed: the positive control is the
    /// summers' count); her rule is set for life; `stakes=0` keeps the old
    /// endless heap and the old hold. The season clock's boundaries are the
    /// ones the HUD and the score read.
    #[test]
    fn the_winters_stop_the_food_and_stakes_0_keeps_the_sandbox() {
        let s = Seasons::default();
        let list = s.list();
        assert_eq!(list.len() as u32, 2 * WINTERS + 1);
        assert_eq!(s.at(0), list[0]);
        assert_eq!(s.at(s.first), list[1], "the first winter starts where the founding summer ends");
        assert!(list[1].winter && list[3].winter && !list[2].winter);
        assert_eq!(list[3].end - list[3].start, s.winter + s.grow, "the second winter is longer");
        assert_eq!(list.last().map(|l| l.end), Some(u64::MAX));
        for stakes in [true, false] {
            let sc = crate::lab::scenario::Scenario::load("demo_queen").expect("demo_queen ships");
            let mut lab = Lab::new(sc.bed.clone());
            let mut args = std::collections::BTreeMap::new();
            if !stakes {
                args.insert("stakes".to_string(), "0".to_string());
            }
            lab.start_demo_with(DemoSeat::Queen, args);
            for _ in 0..1_300 {
                lab.demo_frame();
                lab.tick_for_harness();
            }
            let q = lab.demo.as_ref().and_then(|d| d.queen.as_ref()).expect("queen state").clone();
            assert!(q.ready);
            assert_eq!(q.stakes, stakes);
            let sc = lab.scenario.as_ref().expect("the scenario stays loaded");
            let heaps_in = |season: &Season| {
                let end = season.end.min(season.start + 40_000);
                (q.start + season.start..q.start + end).filter(|&f| sc.due(f).iter().any(|p| matches!(p, crate::lab::scenario::Placement::Heap { .. }))).count()
            };
            let winter_heaps: usize = q.seasons.list().iter().filter(|x| x.winter).map(heaps_in).sum();
            let summers: Vec<usize> = q.seasons.list().iter().filter(|x| !x.winter).map(heaps_in).collect();
            assert!(summers.iter().all(|&n| n > 0), "a summer with no food: {summers:?}");
            let hold = lab.world.steered.as_ref().and_then(|s| s.hold);
            if stakes {
                assert_eq!(winter_heaps, 0, "food fell in a winter");
                assert_eq!(hold, Some(RULE_FOR_LIFE));
                assert!(status(&lab).iter().any(|(t, _)| t.starts_with("SUMMER 1")), "no season clock on the panel");
            } else {
                assert!(winter_heaps > 0, "stakes=0 lost the endless heap");
                assert_eq!(hold, Some(HOLD));
            }
        }
    }

    /// **A line that ended keeps a grey row**, and past [`ROWS`] the oldest
    /// ended line gives way first.
    #[test]
    fn an_ended_line_keeps_its_row() {
        let line = |gone: bool| Line {
            mind: None,
            name: "L".into(),
            colour: None,
            wild: 1,
            born: 0,
            adults: 0,
            brood: 0,
            seen: true,
            gone,
            digs: 0,
            trips: 0,
            frames: 0,
        };
        let mut q = QueenDemo { lines: vec![line(false), line(true), line(false)], ..QueenDemo::default() };
        assert_eq!(rows(&q), vec![0, 1, 2], "the ended line lost its row");
        q.lines.extend((0..ROWS).map(|_| line(false)));
        let r = rows(&q);
        assert_eq!(r.len(), ROWS);
        assert!(!r.contains(&1), "an ended line outlived a living one: {r:?}");
    }

    fn q_colony(lab: &Lab) -> u32 {
        lab.demo.as_ref().and_then(|d| d.queen.as_ref()).map_or(0, |q| q.colony)
    }
}
