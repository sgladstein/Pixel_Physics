//! **The lone scout demo, played by a script** (`sim::tandem`; plan
//! `Reports/teaching-the-colony-2026-10-25/scout-demo/plan-v2.md` §4).
//!
//! Loads `demo_scout.ron` with the trained ant, warms the box to the hand-over
//! frame, picks the player's body (a modal-pace adult near the nest) and plays
//! her by a script: walk home, tap the nearest sister, lead her to the far
//! heap, **wait whenever her pip would turn amber** (a tell, a stop, or a
//! search), then walk home and do it again. Prints the tandem counters every
//! `every=` frames so a run can be judged by number as well as by picture.
//!
//! ```text
//! cargo run --release --example scoutdemo -- arm=A seed=1 frames=36000
//! cargo run --release --example scoutdemo -- arm=A seed=1 rude=1     # a player who never waits
//! cargo run --release --example scoutdemo -- arm=A seed=1 gain=0     # the memory pull off: the honest arm
//! cargo run --release --example scoutdemo -- arm=A seed=1 off=1      # module off, same player: frame cost
//! cargo run --release --example scoutdemo -- arm=A seed=1 frames=6000 gif=1 shot=40 zoom=6 out=a.gif
//! ```
//!
//! The run goes through the seat's own start (`lab::demo::scout::start_tandems`, `pick_body`, the worker's
//! role) and drives the body through the keys (`SteerInput`), so its pictures are the binary's.
//!
//! `rude=1` is the positive control for the pauses: a leader who walks on
//! through every stop must break runs that a waiting leader keeps.
//!
//! **The lean season** (on by default, as in the binary; `stakes=0` is the
//! sandbox before it; `season=` and `floor=` as the seat reads them): the
//! near trickle stops at the hand-over and the run lasts the season unless
//! `frames=` says otherwise. The `END` line carries the deciding numbers:
//! the colony alive when the season ended, the outcome, and her tree's
//! harvest by generation. `oldant=1` runs the hand-built ant instead of the
//! trained pool.

use pixel_physics::lab::demo::{scout, Demo, DemoSeat};
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::Lab;
use pixel_physics::sim::cell::OrganismId;
use pixel_physics::sim::steer::{self, SteerInput, SteerRole};
use pixel_physics::sim::tandem::{self, Arm, Phase, TapResult};
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

use scout::{BODY_X as TAP_X, NEST_X};

fn head(world: &World, id: OrganismId) -> Option<(i32, i32)> {
    world.organism(id).and_then(|s| s.chain.first().copied())
}

/// One ant through the season (`probe=1`).
#[derive(Debug, Clone, Default)]
struct AntRow {
    first: u64,
    min_x: i32,
    /// Ant-looks per 16-column bin (all ants pooled on the first row's entry: see `probe_report`).
    occ: Vec<u32>,
    max_east: i32,
    reached: Option<u64>,
    trips_seen: u32,
    heap_cells: u32,
    other_cells: u32,
    taught: Option<u8>,
    died: Option<u64>,
    bank0: f32,
}

type Casc = std::collections::BTreeMap<OrganismId, (OrganismId, u64, (i32, i32), (i32, i32), String)>;

fn probe_tick(w: &World, rows: &mut std::collections::BTreeMap<OrganismId, AntRow>, gates: &mut Vec<String>, player: OrganismId, casc: &mut Casc, heap_x: i32) {
    let patch = tandem::Patch { x: heap_x, r: 10 };
    let frame = w.frame;
    let live = scout::ants(w);
    let t = w.tandem.as_deref();
    // **Every cascade run, start to end**: where the pair started, where each was last seen, its phase, and how
    // it ended (in the tree = taught; a mark says arrived or broke).
    if let Some(t) = t {
        for p in t.pairs.iter().filter(|p| p.cascade) {
            let (lh, fh) = (head(w, p.leader).unwrap_or((0, 0)), head(w, p.follower).unwrap_or((0, 0)));
            let lf = w.organism(p.leader).and_then(|s| s.last_food).map(|(x, y, _)| (x, y));
            let e = casc.entry(p.follower).or_insert((p.leader, frame, lh, fh, String::new()));
            e.2 = lh;
            e.3 = fh;
            e.4 = format!("{:?} learned {} met {} leader last_food {lf:?}", p.phase, p.learned, p.met);
        }
        let gone: Vec<OrganismId> = casc.keys().copied().filter(|f| !t.pairs.iter().any(|p| p.cascade && p.follower == *f)).collect();
        for f in gone {
            let (l, f0, lh, fh, last) = casc.remove(&f).expect("listed");
            let mark = t.marks.iter().rev().find(|m| m.0 == f).map(|m| format!("{:?}", m.1));
            gates.push(format!(
                "CASC f{f0}..f{frame} leader #{l} pupil #{f} last seen leader {lh:?} pupil {fh:?} [{last}] -> tree {} lesson {:?} mark {mark:?} pupil alive {}",
                t.tree.contains_key(&f),
                t.lessons.get(&f),
                head(w, f).is_some()
            ));
        }
    }
    for &id in &live {
        let Some(st) = w.organism(id) else { continue };
        let Some(h) = st.chain.first().copied() else { continue };
        let bank = tandem::energy_sense(w, id).map_or(0.0, |s| s / (1.0 - s).max(1e-3));
        let r = rows.entry(id).or_insert_with(|| AntRow { first: frame, max_east: h.0 - NEST_X, trips_seen: st.life.trip_deliveries, bank0: bank, ..AntRow::default() });
        r.max_east = r.max_east.max(h.0 - NEST_X);
        r.min_x = if r.min_x == 0 { h.0 } else { r.min_x.min(h.0) };
        if r.occ.is_empty() {
            r.occ = vec![0; 32];
        }
        let surface = w.room_surface_at(h.0).is_some_and(|top| h.1 <= top + 1);
        if surface {
            r.occ[(h.0.clamp(0, 511) / 16) as usize] += 1;
        }
        if patch.holds_column(h.0) && r.reached.is_none() {
            r.reached = Some(frame);
        }
        if let Some(t) = t {
            if t.tree.contains_key(&id) && r.taught.is_none() {
                let mut g = 1u8;
                let mut at = id;
                while let Some(&up) = t.tree.get(&at) {
                    if !t.tree.contains_key(&up) || g >= 8 {
                        break;
                    }
                    g += 1;
                    at = up;
                }
                r.taught = Some(g);
            }
        }
        let trips = st.life.trip_deliveries;
        if trips > r.trips_seen {
            let n = trips - r.trips_seen;
            let from = st.last_food.is_some_and(|(x, _, _)| patch.holds_column(x));
            if from {
                r.heap_cells += n;
            } else {
                r.other_cells += n;
            }
            r.trips_seen = trips;
            // **The cascade's gates, read at the trip-home instant** for a taught ant (not the player).
            if from && id != player && t.is_some_and(|t| t.tree.contains_key(&id)) {
                let t = t.expect("checked");
                let full = t.pairs.len() >= t.knobs.max_pairs;
                let leads = t.leads(id);
                let c3 = tandem::candidates(w, id, 3);
                let c8 = tandem::candidates(w, id, 8);
                let c20 = tandem::candidates(w, id, 20);
                let near = c3.first().or(c8.first()).copied();
                let sense = near.and_then(|n| tandem::energy_sense(w, n));
                let p = sense.and_then(|s| tandem::accept_chance(&t.knobs, s));
                let all = w.live_organism_ids().into_iter().filter(|&o| o != id && w.organism(o).is_some_and(|s| s.species == st.species) && head(w, o).is_some_and(|q| (q.0 - h.0).abs().max((q.1 - h.1).abs()) <= 3)).count();
                let surf = w.room_surface_at(h.0).map_or(0, |top| h.1 - top);
                gates.push(format!(
                    "f{frame} #{id} at ({},{}) depth {surf} full {full} leads {leads} ants<=3 {all} cand<=3 {} <=8 {} <=20 {} near sense {:?} accept {:?}",
                    h.0,
                    h.1,
                    c3.len(),
                    c8.len(),
                    c20.len(),
                    sense.map(|s| (s * 100.0).round() / 100.0),
                    p.map(|p| (p * 100.0).round() / 100.0)
                ));
            }
        }
    }
    for (id, r) in rows.iter_mut() {
        if r.died.is_none() && !live.contains(id) {
            r.died = Some(frame);
        }
    }
}

fn probe_report(w: &World, rows: &std::collections::BTreeMap<OrganismId, AntRow>, gates: &[String]) {
    println!("PROBE ants traced {}", rows.len());
    let mut by = std::collections::BTreeMap::new();
    for r in rows.values() {
        let class = match (r.taught, r.reached.is_some()) {
            (Some(g), _) => format!("taught g{g}"),
            (None, true) => "untaught, reached heap".to_string(),
            (None, false) => "untaught, never reached".to_string(),
        };
        let e = by.entry(class).or_insert((0u32, 0u32, 0u32, 0u32, 0f32));
        e.0 += 1;
        e.1 += u32::from(r.died.is_some());
        e.2 += r.heap_cells;
        e.3 += r.other_cells;
        e.4 += r.bank0;
    }
    for (k, (n, dead, heap, other, bank)) in &by {
        println!("PROBE {k}: n {n} died {dead} heap cells home {heap} other cells home {other} mean bank at first look {:.2}", bank / *n as f32);
    }
    let mut hist = [0u32; 10];
    for r in rows.values() {
        hist[((r.max_east.max(0) / 40) as usize).min(9)] += 1;
    }
    println!("PROBE furthest east of the nest, 40-col bins (0-39 .. 360+): {hist:?}");
    let first = rows.values().map(|r| r.first).min().unwrap_or(0);
    let (old, born): (Vec<&AntRow>, Vec<&AntRow>) = rows.values().partition(|r| r.first == first);
    let heap_all: u32 = rows.values().map(|r| r.heap_cells).sum();
    let other_all: u32 = rows.values().map(|r| r.other_cells).sum();
    println!(
        "PROBE cohort: at hand-over {} alive at end {} | born in season {} alive at end {} | cells home: heap {heap_all} other {other_all}",
        old.len(),
        old.iter().filter(|r| r.died.is_none()).count(),
        born.len(),
        born.iter().filter(|r| r.died.is_none()).count()
    );
    let mut occ = [0u32; 32];
    let mut west = [0u32; 9];
    for r in rows.values() {
        for (i, n) in r.occ.iter().enumerate() {
            occ[i] += n;
        }
        west[((r.min_x.max(0)) / 16).min(8) as usize] += 1;
    }
    println!("PROBE surface ant-looks per 16-col bin (x 0..511): {occ:?}");
    println!("PROBE furthest west, 16-col bins of x (0-15 .. 128+): {west:?}");
    let far: Vec<_> = rows.iter().filter(|(_, r)| r.max_east >= 300).take(8).map(|(id, r)| (*id, r.max_east, r.first, r.died, w.organism(*id).map(|s| s.chain.clone()))).collect();
    println!("PROBE far rangers (id, max east, first seen, died, chain now): {far:?}");
    let mut finders: Vec<_> = rows.iter().filter(|(_, r)| r.taught.is_none() && r.heap_cells > 0).map(|(id, r)| (r.heap_cells, *id, r.reached, r.first)).collect();
    finders.sort_by(|a, b| b.cmp(a));
    println!("PROBE untaught heap haulers (cells, id, first reached, first seen): {:?}", &finders[..finders.len().min(20)]);
    println!("PROBE cascade gate rows {}", gates.len());
    for g in gates {
        println!("GATE {g}");
    }
    let _ = w;
}

/// A tap's outcome as the `tap outcomes` line counts it.
fn tap_key(r: scout::SeatTap) -> String {
    match r {
        scout::SeatTap::Tap(TapResult::Refused(_, why)) => format!("Refused {why:?}"),
        scout::SeatTap::Tap(TapResult::Accepted(_)) => "Accepted".to_string(),
        scout::SeatTap::Tap(TapResult::NoOne) => "NoOne".to_string(),
        scout::SeatTap::TurnedAway(_) => "Refused TurnedAway".to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Plan {
    Home,
    Tapping,
    Leading,
}

fn main() {
    let arm = match arg::<String>("arm").unwrap_or_else(|| "A".into()).as_str() {
        "B" => Arm::Column,
        "C" => Arm::Escort,
        _ => Arm::Scout,
    };
    let seed: u64 = arg("seed").unwrap_or(1);
    let start: u64 = arg("start").unwrap_or(18_000);
    let stakes = arg::<u8>("stakes").unwrap_or(1) == 1;
    let season: u64 = arg("season").unwrap_or(scout::SEASON);
    let frames: u64 = arg("frames").unwrap_or(if stakes { season + 60 } else { 36_000 });
    let oldant = arg::<u8>("oldant").unwrap_or(0) == 1;
    let every: u64 = arg("every").unwrap_or(3_000);
    let rude2 = arg::<u8>("rude").unwrap_or(0) == 2;
    let rude = arg::<u8>("rude").unwrap_or(0) >= 1;
    let camp = arg::<u8>("camp").unwrap_or(0) == 1;
    let campx: i32 = arg("campx").unwrap_or(16);
    let campin: i32 = arg("campin").unwrap_or(3);
    let c: Option<f32> = arg("c");
    // `letgo=N`: the patient player uses the seat's `K` -- a pupil she has waited on N frames with no closing is let
    // go (the seat's lag-hold gives up at `scout::HOLD_TIMEOUT` and says so). Off by default: the builder's play.
    let letgo: Option<u64> = arg("letgo");
    let mut wait_since: Option<(u64, i32)> = None;
    let mut let_gos = 0u32;
    let trace = arg::<u8>("trace").unwrap_or(0) == 1;
    println!(
        "scoutdemo arm={} seed={seed} start={start} frames={frames} every={every} rude={rude} c={c:?} gain={:?} stakes={stakes} season={season} floor={:?} oldant={oldant} off={:?} sealed={:?}",
        arm.letter(),
        arg::<f32>("gain"),
        arg::<f32>("floor"),
        arg::<u8>("off"),
        arg::<u8>("sealed")
    );

    let mut sc = Scenario::load("demo_scout").expect("demo_scout.ron ships");
    sc.bed.seed = seed;
    let mut lab = Lab::new(sc.bed.clone());
    lab.load_scenario(sc);
    let (full_w0, full_h0) = (pixel_physics::lab::WIDTH, pixel_physics::lab::HEIGHT);
    if !oldant {
        lab.set_new_ant(true);
    }
    while lab.world.frame < start {
        lab.tick_for_harness();
    }
    let alive0 = scout::ants(&lab.world).len();
    // **The seat's own start**, so this run and `--bin lab -- demo=scout` begin the same way: the seat's
    // tandem set-up, the seat's choice of body, the worker's role (her colony keeps its own breeding rule),
    // and the seat's overlays and panel in every captured frame.
    let gain: f32 = arg("gain").unwrap_or(if stakes { scout::LEAN_GAIN } else { scout::GAIN });
    scout::start_tandems(&mut lab.world, arm, c, gain);
    let mut demo = Demo::new(DemoSeat::Scout);
    let mut args = std::collections::BTreeMap::new();
    args.insert("arm".to_string(), arm.letter().to_string());
    if let Some(c) = c {
        args.insert("c".to_string(), c.to_string());
    }
    args.insert("gain".to_string(), gain.to_string());
    args.insert("stakes".to_string(), if stakes { "1" } else { "0" }.to_string());
    args.insert("season".to_string(), season.to_string());
    if let Some(f) = arg::<f32>("floor") {
        args.insert("floor".to_string(), f.to_string());
    }
    if let Some(h) = arg::<u32>("heap") {
        args.insert("heap".to_string(), h.to_string());
    }
    if let Some(h) = arg::<i32>("heapx") {
        args.insert("heapx".to_string(), h.to_string());
    }
    // `sealed=0`: the far heap open to every forager, as in rounds 1-2 (the seal is on with the stakes).
    if let Some(v) = arg::<u64>("trickle") {
        args.insert("trickle".to_string(), v.to_string());
    }
    if let Some(v) = arg::<u8>("sealed") {
        args.insert("sealed".to_string(), v.to_string());
    }
    let mut seat = scout::ScoutDemo::from_args(&args, seed);
    // The season's heap (it moves at the hand-over with stakes on): where the script leads and the probe looks.
    let heap_x = seat.heap_x;
    seat.started = true;
    seat.alive_at_start = alive0;
    demo.scout = Some(seat);
    demo.args = args;
    lab.demo = Some(demo);
    // The seat's own season start: the near trickle stops here, as in the binary.
    scout::start_season(&mut lab);
    // `cardshot=x.png`: the seat card over the box at the hand-over, as the window opens it.
    if let Some(path) = arg::<String>("cardshot") {
        lab.show_help = false;
        if let Some(d) = lab.demo.as_mut() {
            d.window = true;
        }
        lab.open_seat_card(false);
        let mut full = vec![0u8; (full_w0 * full_h0 * 4) as usize];
        lab.draw(&mut full, 60.0);
        match image::RgbaImage::from_raw(full_w0, full_h0, full).map(|img| img.save(&path)) {
            Some(Ok(())) => println!("cardshot -> {path}"),
            other => eprintln!("scoutdemo: cardshot failed: {other:?}"),
        }
        lab.close_seat_card();
        if let Some(d) = lab.demo.as_mut() {
            d.window = false;
        }
    }
    lab.world.death_log = Some(Vec::new());
    let Some(player) = scout::pick_body(&lab.world) else {
        println!("no player body: {alive0} ants alive at {start}");
        return;
    };
    // Driven through the keys exactly as the window drives her (`SteerInput`), not `steer::force`.
    steer::mark_with(&mut lab.world, player, Some(SteerInput::default()), SteerRole::Worker);
    println!("hand-over at {}: {alive0} ants alive, player #{player}", lab.world.frame);

    // **Capture** (`gif=1 out=x.gif shot=N zoom=Z`): the seat's own frame, the camera on the player, one shot
    // every `shot` frames, through `lab.draw` exactly as `labgif` does.
    let gif = arg::<u8>("gif").unwrap_or(0) == 1;
    let out: String = arg("out").unwrap_or_else(|| format!("target/scoutdemo-{}-s{seed}.gif", arm.letter()));
    let shot: u64 = arg("shot").unwrap_or(60);
    let zoom: i32 = arg("zoom").unwrap_or(2);
    let (full_w, full_h) = (pixel_physics::lab::WIDTH, pixel_physics::lab::HEIGHT);
    let endshot: Option<String> = arg("endshot");
    if gif || endshot.is_some() {
        // The welcome card a fresh lab opens on would sit over every shot.
        lab.show_help = false;
        for _ in 1..zoom {
            lab.renderer.adjust_zoom(1);
        }
    }
    let bounds = pixel_physics::sim::chunk::Rect::new(0, 0, lab.spec.width - 1, lab.spec.height - 1);
    let (span_x, span_y) = lab.renderer.visible_span((full_w, full_h));
    let mut shots: Vec<image::RgbaImage> = Vec::new();

    let mut plan = Plan::Home;
    // `off=1`: the same player and script, the module disarmed for every tick (re-armed empty between ticks so
    // the script's reads still work). The frame-cost control: the tick time is what the module adds.
    let off = arg::<u8>("off").unwrap_or(0) == 1;
    let map_at: Option<u64> = arg("map");
    let mut tick_ns: u128 = 0;
    let mut ticks: u64 = 0;
    let mut refusals: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    let mut tap_tries = 0u32;
    let mut leads_done = 0u32;
    let mut gaps = [0u64; 8];
    let mut best_dist = i32::MAX;
    let mut last_goal = 0;
    let mut moved_at = 0u64;
    // `probe=1`: every ant traced through the season (how far east she went, whether she reached the far heap,
    // what she brought home from it, whether she was taught, when she died), and every trip home from the heap by
    // a taught ant with the cascade's gates read at that instant. Individuals, not a population statistic.
    let probe = arg::<u8>("probe").unwrap_or(0) == 1;
    let mut ants_seen: std::collections::BTreeMap<OrganismId, AntRow> = std::collections::BTreeMap::new();
    let mut gate_rows: Vec<String> = Vec::new();
    let mut casc: Casc = Casc::new();
    let end = start + frames;
    let mut player_dead = false;
    while lab.world.frame < end {
        let w = &mut lab.world;
        let Some(at) = head(w, player) else {
            // **The colony's season goes on without her** (round 3): the deciding number is the colony alive at the
            // season's end, and a teaching-off player who never walks to the heap starves first on most seeds (7 of
            // 12 sealed), which used to end the run early and read the colony at her death.
            if !player_dead {
                let cause = w.death_log.as_ref().and_then(|l| l.iter().rev().find(|r| r.id == player)).map(|r| r.cause);
                println!("frame {}: the player died ({cause:?})", w.frame);
                player_dead = true;
            }
            if off {
                lab.world.tandem = None;
            }
            lab.tick_for_harness();
            if off {
                lab.world.tandem = Some(Box::new(tandem::Tandems::new(arm, None)));
            }
            continue;
        };
        let pairs: Vec<_> = w.tandem.as_ref().map_or(&[][..], |t| &t.pairs[..]).iter().filter(|p| p.leader == player).cloned().collect();
        for p in &pairs {
            if let Some(g) = tandem::contact_gap(w, p.follower, player) {
                gaps[(g as usize).min(7)] += 1;
            }
        }
        // ADVERSARIAL SCRATCH: `camp=1` -- she never leads. She walks to the heap's west foot once and stands there,
        // pressing Y every 30 frames on whoever comes by (the sisters the seal turns away walk west, past her).
        let camp_dir = camp.then(|| {
            // With a pupil in tow she steps the few cells into the heap's columns; else back to her post.
            let post = if pairs.is_empty() { heap_x - campx } else { heap_x - campin };
            if (at.0 - post).abs() > 1 {
                Some(((post - at.0).signum() as i8, 0))
            } else if !pairs.is_empty() {
                None
            } else {
                if w.frame.is_multiple_of(30) {
                    // Through the seat's own `Y` (`scout::tap_for`), so the harness is refused what the window refuses.
                    let r = scout::tap_for(w, lab.demo.as_mut().and_then(|d| d.scout.as_mut()), player);
                    *refusals.entry(tap_key(r)).or_insert(0u32) += 1;
                    lab.ui.say(scout::tap_line(r));
                }
                None
            }
        });
        let dir: Option<(i8, i8)> = if let Some(d) = camp_dir { d } else { match plan {
            Plan::Home => {
                if (at.0 - TAP_X).abs() <= 3 {
                    plan = Plan::Tapping;
                    tap_tries = 0;
                    None
                } else {
                    Some(((TAP_X - at.0).signum() as i8, 0))
                }
            }
            Plan::Tapping if tandem::candidates(w, player, 3).is_empty() => {
                // Nobody in reach: walk toward the nest, where sisters are, a step at a time.
                if w.frame.is_multiple_of(30) {
                    tap_tries += 1;
                }
                if tap_tries > 40 {
                    plan = Plan::Leading;
                }
                (at.0 > NEST_X + 6).then_some((-1, 0))
            }
            Plan::Tapping => {
                if w.frame.is_multiple_of(30) {
                    tap_tries += 1;
                    // Through the seat's own `Y` (`scout::tap_for`), so the harness is refused what the window refuses.
                    let r = scout::tap_for(w, lab.demo.as_mut().and_then(|d| d.scout.as_mut()), player);
                    *refusals.entry(tap_key(r)).or_insert(0u32) += 1;
                    lab.ui.say(scout::tap_line(r));
                    match r {
                        scout::SeatTap::Tap(TapResult::Accepted(_)) => plan = Plan::Leading,
                        _ if tap_tries > 20 => plan = Plan::Leading,
                        _ => {}
                    }
                }
                None
            }
            Plan::Leading => {
                if pairs.is_empty() {
                    leads_done += 1;
                    plan = Plan::Home;
                    None
                } else {
                    // A patient leader stands while any pupil lags, stops or searches; a rude one never does.
                    // ADVERSARIAL SCRATCH: `rude=2` is a human holding the arrow down in the binary: she never waits
                    // on her own, but the seat's frame hook (`scout::frame`) holds her step while a Following pupil
                    // is 3+ cells back -- which this harness, calling only `tick_for_harness`, never applies.
                    let held_by_seat = rude2
                        && pairs.iter().any(|p| matches!(p.phase, Phase::Following) && tandem::contact_gap(w, p.follower, player).is_some_and(|g| g >= 3));
                    let wait = held_by_seat || (!rude
                        && pairs.iter().any(|p| {
                            let g = tandem::contact_gap(w, p.follower, player).unwrap_or(99);
                            !matches!(p.phase, Phase::Following) || g >= 3
                        }));
                    let g_min = pairs.iter().filter_map(|p| tandem::contact_gap(w, p.follower, player)).min().unwrap_or(99);
                    if wait {
                        let (since, best) = wait_since.get_or_insert((w.frame, g_min));
                        if g_min < *best {
                            (*since, *best) = (w.frame, g_min);
                        }
                        if letgo.is_some_and(|n| w.frame.saturating_sub(*since) > n) {
                            scout::let_go_nearest(w, player);
                            let_gos += 1;
                            wait_since = None;
                        }
                    } else {
                        wait_since = None;
                    }
                    if wait {
                        None
                    } else if (heap_x - at.0).abs() <= 8 {
                        // She stops at the near edge of the heap, not on it: the heap's next drop lands on
                        // whoever stands in it, and a body under it has no option but to turn round.
                        None
                    } else {
                        Some(((heap_x - at.0).signum() as i8, 0))
                    }
                }
            }
        } };
        // The headless player eats at the heap (the E key: pick up, or eat from the crop), as a person would.
        let hungry = w.organism(player).is_some_and(|s| s.energy < 1_500.0);
        let food_near = w.materials.id_of("provisions").is_some_and(|f| (-3..=3).any(|dx| (-3..=3).any(|dy| w.get(at.0 + dx, at.1 + dy).material == f)));
        let eat = hungry && food_near;
        // `map=FRAME`: the cells round the player and her options at that frame (`a` an ant's cell, `@` her
        // head). How the buried-by-the-heap and walking-backwards stalls were found.
        if map_at == Some(w.frame) {
            for y in at.1 - 8..=at.1 + 3 {
                let row: String = (at.0 - 14..=at.0 + 14)
                    .map(|x| {
                        let c = w.get(x, y);
                        if (x, y) == at {
                            '@'
                        } else if c.organism_id() != 0 {
                            'a'
                        } else if c.is_empty() {
                            '.'
                        } else {
                            w.materials.get(c.material).name.chars().next().unwrap_or('?')
                        }
                    })
                    .collect();
                println!("    {y}: {row}");
            }
            let st = w.organism(player).unwrap();
            println!("    chain {:?}", st.chain);
            let ant = w.species.id_of("ant").unwrap();
            let def = w.species.creature_shared(ant).unwrap();
            let mut opts = Vec::new();
            pixel_physics::sim::creature::options::build_with(w, player, &def, true, &mut opts);
            println!("    opts {:?}", opts.iter().map(|o| (o.verb, o.target)).collect::<Vec<_>>());
        }
        if trace && w.frame.is_multiple_of(150) {
            let e = w.organism(player).map_or(0.0, |s| s.energy);
            let fs: Vec<String> = pairs.iter().map(|p| format!("{:?}@{:?} {:?} l{}/{}", p.follower, head(w, p.follower), p.phase, p.learned, p.met)).collect();
            println!("  t{} player {:?} e{:.0} {:?} dir {:?} | {}", w.frame, at, e, plan, dir, fs.join(" ; "));
        }
        // A scripted player who is stuck (a tunnel wall, a face) tries upward headings in turn, as a person
        // pressing Up would; a real player reads the screen instead.
        // Progress is getting nearer the goal column, not merely moving: a chain that turns round in place moves
        // its head every decision and gets nowhere.
        let goal = if camp { if pairs.is_empty() { heap_x - campx } else { heap_x - campin } } else { match plan {
            Plan::Home => TAP_X,
            Plan::Tapping => NEST_X,
            Plan::Leading => heap_x - 8 * (heap_x - TAP_X).signum(),
        } };
        let dist = (goal - at.0).abs();
        if dir.is_none() || dist < best_dist || goal != last_goal {
            best_dist = dist;
            last_goal = goal;
            moved_at = w.frame;
        }
        let dir = match dir {
            Some(d) if w.frame.saturating_sub(moved_at) > 40 => {
                let alts = [(d.0, -1), (0, -1), (-d.0, -1), (-d.0, 0), (d.0, 1)];
                Some(alts[((w.frame - moved_at) / 40 - 1) as usize % alts.len()])
            }
            other => other,
        };
        // Boxed in for a while (provisions and spoil pile over the nest mound), she digs her way out, as a player
        // pressing G would.
        let boxed = dir.is_some() && w.frame.saturating_sub(moved_at) > 200;
        if let Some(st) = w.steered.as_mut() {
            st.input = Some(SteerInput { dir, eat, dig: boxed && !eat, ..SteerInput::default() });
        }
        // ADVERSARIAL SCRATCH: `cheat=F` -- F frames into the season the player arms the lab's own COLONY tool
        // (toolbar C), sets the stock to 52 and clicks the ground east of the nest, as the window would.
        if arg::<u64>("cheat").is_some_and(|f| lab.world.frame == start + f) {
            use pixel_physics::lab::ui::{Action, Tool};
            lab.show_help = false;
            lab.act(Action::Tool(Tool::Colony));
            for _ in 0..8 {
                lab.act(Action::Stock(1));
            }
            while lab.ui.stock() > 52 {
                lab.act(Action::Stock(-1));
            }
            let wx = NEST_X + 40;
            let wy = lab.world.room_surface_at(wx).unwrap_or(150) - 2;
            lab.renderer.set_camera(wx - span_x / 2, wy - span_y / 2, (full_w, full_h), Some(bounds));
            let (x0, y0, x1, y1, _) = lab.renderer.world_rect_to_logical(wx, wy, wx, wy);
            let (lx, ly) = ((x0 + x1) / 2, (y0 + y1) / 2);
            let before = scout::ants(&lab.world).len();
            lab.press(lx, ly);
            lab.release(lx, ly);
            println!("CHEAT frame {}: COLONY tool, stock {}, click at logical ({lx},{ly}) world ({wx},{wy}): ants {before} -> {}", lab.world.frame, lab.ui.stock(), scout::ants(&lab.world).len());
        }
        if off {
            lab.world.tandem = None;
        }
        let t0 = std::time::Instant::now();
        lab.tick_for_harness();
        tick_ns += t0.elapsed().as_nanos();
        ticks += 1;
        if off {
            lab.world.tandem = Some(Box::new(tandem::Tandems::new(arm, None)));
        }
        if probe && lab.world.frame.is_multiple_of(30) {
            probe_tick(&lab.world, &mut ants_seen, &mut gate_rows, player, &mut casc, heap_x);
        }
        // `pngat=F1,F2 pngout=PREFIX`: single PNGs at those world frames, framed as the GIF's shots are.
        let png_now = arg::<String>("pngat").is_some_and(|l| l.split(',').any(|f| f.parse::<u64>().ok() == Some(lab.world.frame)));
        if png_now || (gif && (lab.world.frame - start).is_multiple_of(shot) && lab.world.frame >= arg::<u64>("gifstart").unwrap_or(0)) {
            // `cam=cascade`: on a pupil leading a sister of her own while one is under way (round 2's picture).
            let casc_leader = (arg::<String>("cam").as_deref() == Some("cascade"))
                .then(|| lab.world.tandem.as_deref().and_then(|t| t.pairs.iter().find(|p| p.cascade).map(|p| p.leader)))
                .flatten();
            // `cam=heap`: on the sealed heap, where the turn-aways and the carry marks are (round 3's picture).
            let heap_cam = (arg::<String>("cam").as_deref() == Some("heap")).then(|| (heap_x, lab.world.room_surface_at(heap_x).unwrap_or(150) - 6));
            if let Some((hx, hy)) = heap_cam.or_else(|| head(&lab.world, casc_leader.unwrap_or(player))) {
                // Centred: the seat panel is in the sky at the top now (2026-10-30), not over the lower half.
                lab.renderer.set_camera(hx - span_x / 2, hy - span_y / 2, (full_w, full_h), Some(bounds));
            }
            let mut full = vec![0u8; (full_w * full_h * 4) as usize];
            lab.draw(&mut full, 60.0);
            if let Some(img) = image::RgbaImage::from_raw(full_w, full_h, full) {
                if png_now {
                    let path = format!("{}-f{}.png", arg::<String>("pngout").unwrap_or_else(|| "target/scoutdemo".into()), lab.world.frame);
                    match img.save(&path) {
                        Ok(()) => println!("png -> {path}"),
                        Err(e) => eprintln!("scoutdemo: png failed: {e}"),
                    }
                } else {
                    shots.push(img);
                }
            }
        }
        if (lab.world.frame - start).is_multiple_of(every) {
            // Food left in the far heap and by the nest: is the heap the bottleneck, or the walk to it?
            let food = lab.world.materials.id_of("provisions");
            let cells = |x0: i32, x1: i32| (x0..=x1).map(|x| (0..lab.spec.height).filter(|&y| Some(lab.world.get(x, y).material) == food).count()).sum::<usize>();
            println!("  food: far heap {} near nest {}", cells(heap_x - 15, heap_x + 15), cells(NEST_X - 40, NEST_X + 40));
            let t = lab.world.tandem.as_ref().unwrap();
            let knows = t.lessons.len();
            println!(
                "f{} alive {} plan {:?} | {:?} | know {} income/min {} | gaps0-7 {:?} | recall tilted {} flipped {}",
                lab.world.frame,
                scout::ants(&lab.world).len(),
                plan,
                t.counts,
                knows,
                t.income_over(lab.world.frame, 3_600),
                gaps,
                lab.world.recall.as_ref().map_or(0, |r| r.counts.tilted),
                lab.world.recall.as_ref().map_or(0, |r| r.counts.picks_flipped)
            );
        }
    }
    if gif {
        let delay_ms = arg::<u64>("delay").unwrap_or(80).max(1);
        let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(delay_ms));
        let n = shots.len();
        let frames: Vec<image::Frame> = shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, delay)).collect();
        match std::fs::File::create(&out) {
            Ok(file) => {
                let mut enc = image::codecs::gif::GifEncoder::new(file);
                let _ = enc.set_repeat(image::codecs::gif::Repeat::Infinite);
                if let Err(e) = enc.encode_frames(frames) {
                    eprintln!("scoutdemo: gif encode failed: {e}");
                }
                println!("gif: {n} shots, one every {shot} frames, {delay_ms} ms each, zoom {zoom} -> {out}");
            }
            Err(e) => eprintln!("scoutdemo: cannot write {out}: {e}"),
        }
    }
    // `endshot=x.png`: the seat's frame at the end of the run (the end card over the box), camera between the nest
    // and the heap -- taken whether or not the player lived, since the card is the colony's.
    if let Some(path) = endshot {
        let mid = (NEST_X + heap_x) / 2;
        let y = lab.world.room_surface_at(mid).unwrap_or(150);
        lab.renderer.set_camera(mid - span_x / 2, y - span_y / 2, (full_w, full_h), Some(bounds));
        let mut full = vec![0u8; (full_w * full_h * 4) as usize];
        lab.draw(&mut full, 60.0);
        match image::RgbaImage::from_raw(full_w, full_h, full).map(|img| img.save(&path)) {
            Some(Ok(())) => println!("endshot -> {path}"),
            other => eprintln!("scoutdemo: endshot failed: {other:?}"),
        }
    }
    println!("tap outcomes: {refusals:?}");
    println!("tick cost: {:.3} ms mean over {ticks} ticks (off={off})", tick_ns as f64 / ticks.max(1) as f64 / 1e6);
    if let Some(sd) = lab.demo.as_ref().and_then(|d| d.scout.as_ref()) {
        let out = match sd.outcome {
            Some(scout::SeasonEnd::Endured(_)) => "endured".to_string(),
            Some(scout::SeasonEnd::Starved(f)) => format!("starved@{}", f - sd.season_start),
            Some(scout::SeasonEnd::Died(f)) => format!("died@{}", f - sd.season_start),
            None => "none".to_string(),
        };
        println!(
            "SEASON seed={seed} rude={rude} camp={camp} off={off} sealed={} colony={} alive_end={} alive0={alive0} low={} outcome={out} harvest={} gen={:?} stores={:.0} seal: carried {} turned_away {} ants_turned {} leaks {}",
            sd.heap_sealed,
            // Held or fell under the floor at some 30-frame check, whatever ended the seat's season first (her own
            // death ends the card, not the colony's season).
            // Sealed, the seat judges at spring: the colony at the season's end against the floor.
            if (if sd.heap_sealed { scout::colony_alive(&lab) } else { sd.low }) < (sd.alive_at_start as f32 * sd.floor).ceil() as usize { "fell" } else { "held" },
            scout::colony_alive(&lab),
            sd.low,
            sd.harvest,
            sd.by_gen,
            sd.larder,
            sd.seal.carried,
            sd.seal.turned_away,
            sd.seal.turned_ids.len(),
            sd.seal.leaks
        );
    }
    if let Some(sd) = lab.demo.as_ref().and_then(|d| d.scout.as_ref()) {
        // **The lessons, graded** (review 2026-10-31, B1): how strong the season's lessons were, in tenths.
        let t = lab.world.tandem.as_ref().unwrap();
        let mut tenths = [0u32; 11];
        for &l in t.lessons.values() {
            tenths[((l * 10.0).floor() as usize).min(10)] += 1;
        }
        let colonies: std::collections::BTreeSet<u32> = scout::ants(&lab.world).iter().filter_map(|&i| lab.world.organism(i)).map(|s| s.colony).collect();
        println!(
            "LESSONS tenths {tenths:?} | taps refused turned-away {} | let-gos {let_gos} | colonies in box {colonies:?}, hers {:?}, all ants {}",
            sd.seal.tap_refused,
            sd.colony,
            scout::ants(&lab.world).len()
        );
    }
    if probe {
        probe_report(&lab.world, &ants_seen, &gate_rows);
    }
    let t = lab.world.tandem.as_ref().unwrap();
    println!("END arm={} seed={seed} rude={rude}: leads {leads_done} alive {}->{} {:?}", arm.letter(), alive0, scout::ants(&lab.world).len(), t.counts);
}
