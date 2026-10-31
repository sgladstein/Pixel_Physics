//! **The enemy seat played by a script**: the `demo=enemy` box (a colony
//! digging in under grow lights, a steered centipede set down on the surface
//! at frame 26,000) with the keyboard replaced by a small autopilot that
//! presses the same keys a player would, through `Lab::demo_key`. Built
//! 2026-10-25 for the enemy seat demo (`Reports/teaching-the-colony-2026-10-25/enemy-seat-demo.md`),
//! to answer the plan's measurement A0.2 -- **how often does the colony bite
//! back once the raider is inside the nest**, trained brain against the
//! hand-built one -- and to render the demo's GIF without a window.
//!
//! ```text
//! cargo run --release --example enemyhunt -- seed=1 after=12000
//! cargo run --release --example enemyhunt -- seed=1 oldant            # the hand-built colony
//! cargo run --release --example enemyhunt -- seed=1 policy=skulk gif=hunt.gif every=3 card=card.png
//! ```
//!
//! `policy=skulk` is the player who waits out on the surface for a forager
//! walking alone, away from the mound, and backs off when bitten; the
//! default (`door`) walks straight at the nearest door. `land=` is the
//! demo's own (how far from the door the raider is set down). `drawcost`
//! prints what the seat's view costs `Lab::draw`, in microseconds.
//! `respawn` presses R after each death and plays on, one `SEASON` line a
//! raider life; `mutate=0` turns the demo's birth mutation off (the control).
//!
//! The autopilot is deliberately dumb and the same for both brains: walk to
//! the nearest door, hold down and dig until the head is three rows under
//! the surface, then steer at the nearest ant within 24 cells and press B
//! whenever one is within two; press E after a kill; back out (hold up) once
//! the body is badly gnawed or has lost two cells, and rest on the surface
//! for 1,500 frames. A press of G after 120 frames without the head moving.
//! Every row says which state the script was in, so a number can be read
//! against what the player was doing.
//!
//! **What it counts** comes from the demo's own counters (`EnemyDemo`), which
//! read the world's encounter and kill logs -- the same numbers the HUD and
//! the season card print -- plus the frames the head spent under the surface,
//! so `taken_per_min_in_nest` is bites on the raider per 3,600 frames inside.
use pixel_physics::lab::demo::{Demo, DemoKey, DemoSeat};
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

fn flag(key: &str) -> bool {
    std::env::args().skip(1).any(|a| a == key)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Approach,
    Hunt,
    Retreat,
    Rest,
    /// `policy=skulk`: wait out on the surface for a forager walking alone.
    Stalk,
    Flee,
    Heal,
    /// `policy=heist`: down the galleries to the nearest larva, by a path.
    Dive,
    /// `policy=heist`: back up and out to the lair, larva in the jaws or not.
    Escape,
    /// `policy=camp`: sits where it landed and never bites (the hunger's
    /// positive control: with the stakes on it must starve).
    Camp,
    /// SCRATCH (adversarial review 2026-10-31): back at the lair after a bank,
    /// dig the trophy corpse back into the jaws (G) so it banks again.
    Rebank,
}

/// **A crawl path** from `from` to the first cell `goal` accepts, through
/// open cells and cells ants stand in (they move), each beside solid ground
/// -- a centipede walks on surfaces, it does not fly -- breadth first, so the
/// path is shortest in steps. `None` if no such cell is reachable.
fn crawl_path(world: &World, me: u32, from: (i32, i32), goal: &dyn Fn(i32, i32) -> bool) -> Option<Vec<(i32, i32)>> {
    let b = world.bounds()?;
    let (w, h) = (b.max_x - b.min_x + 1, b.max_y - b.min_y + 1);
    let idx = |x: i32, y: i32| ((y - b.min_y) * w + (x - b.min_x)) as usize;
    let solid = |x: i32, y: i32| world.in_bounds(x, y) && { let c = world.get(x, y); !c.is_empty() && c.organism_id() == 0 };
    let open = |x: i32, y: i32| {
        if !world.in_bounds(x, y) {
            return false;
        }
        let c = world.get(x, y);
        let o = c.organism_id();
        let ok = c.is_empty() || o == me || (o != 0 && world.organism(o).is_some_and(|s| s.brood.is_none()));
        ok && (-1..=1).any(|dy| (-1..=1).any(|dx| (dx, dy) != (0, 0) && solid(x + dx, y + dy)))
    };
    let mut prev = vec![u32::MAX; (w * h) as usize];
    let mut q = std::collections::VecDeque::new();
    prev[idx(from.0, from.1)] = idx(from.0, from.1) as u32;
    q.push_back(from);
    while let Some((x, y)) = q.pop_front() {
        if (x, y) != from && goal(x, y) {
            let mut path = vec![(x, y)];
            let mut i = idx(x, y);
            while i != idx(from.0, from.1) {
                i = prev[i] as usize;
                path.push(((i as i32 % w) + b.min_x, (i as i32 / w) + b.min_y));
            }
            path.reverse();
            return Some(path);
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if world.in_bounds(nx, ny) && prev[idx(nx, ny)] == u32::MAX && open(nx, ny) {
                prev[idx(nx, ny)] = idx(x, y) as u32;
                q.push_back((nx, ny));
            }
        }
    }
    None
}

/// Every living larva's cell.
fn larvae(world: &World) -> Vec<(i32, i32)> {
    world
        .live_brood_ids()
        .into_iter()
        .filter_map(|id| world.organism(id))
        .filter(|s| s.brood.is_some_and(|b| b.stage == pixel_physics::sim::organism::BroodStage::Larva))
        .filter_map(|s| s.chain.first().copied().or_else(|| s.cells.keys().next().copied()))
        .collect()
}

/// The keys the script holds this frame, as the layer's levels.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Hold {
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    /// Shift: sprint while fleeing or backing out (`sprint=0` never does).
    sprint: bool,
}

fn ants(world: &World) -> Vec<(i32, i32)> {
    let Some(ant) = world.species.id_of("ant") else { return Vec::new() };
    world
        .live_organism_ids()
        .into_iter()
        .filter_map(|id| world.organism(id).filter(|s| s.species == ant).and_then(|s| s.chain.first().copied()))
        .collect()
}

/// The ground row at a door nearest `x`, and that door's column.
fn nearest_door(world: &World, x: i32) -> Option<(i32, i32)> {
    world.nest_sites.iter().map(|s| (s.x, s.surface)).min_by_key(|&(dx, _)| (dx - x).abs())
}

fn main() {
    let seed: u64 = arg("seed").unwrap_or(1);
    let after: u64 = arg("after").unwrap_or(12_000);
    let oldant = flag("oldant");
    let policy = arg::<String>("policy").unwrap_or_else(|| "door".to_string());
    // `heist` feeds on the surface like `skulk`, and dives for brood when fed and whole.
    let rebank = policy == "rebank";
    let instinct = policy == "instinct";
    // `coarse` (fix round 2, 2026-10-31): the heist as a person plays it, holding
    // only the general direction -- toward the door's chevron, then down, then at
    // the nearest white larva, then up and to the gold flag -- and B when the
    // ring says a larva is in reach. Meant with `lag=15`: a fairer model of a
    // person than `heist`'s next-cell path follower under the same lag.
    let coarse = policy == "coarse";
    let heist = policy == "heist" || rebank || coarse;
    let camp = policy == "camp" || policy == "rebanktest";
    // `massacre` (fix round 2026-10-31): camps the nearest door and bites every
    // ant within reach -- the queue at the door -- backing off to heal as
    // `skulk` does and never diving: the review's "half the score is ants
    // killed at the door", played on purpose.
    let massacre = policy == "massacre";
    let skulk = policy == "skulk" || heist || massacre;
    let sprints = arg::<u8>("sprint") != Some(0);
    let gif: Option<String> = arg("gif");
    let every: u64 = arg("every").unwrap_or(6);
    let row_every: u64 = arg("rows").unwrap_or(600);
    // `dive=` the tank share a heist waits for (default 0.8 of a full tank).
    let dive_at: f32 = arg("dive").unwrap_or(0.8);
    println!(
        "enemyhunt seed={seed} after={after} brain={} policy={policy} stakes={} gif={gif:?} every={every} dive={dive_at}",
        if oldant { "hand-built" } else { "trained" },
        arg::<String>("stakes").as_deref() != Some("0")
    );
    let mut rebank_digs = 0u32;
    let (mut funnel_frames, mut held_flips, mut held_seen) = (0u64, 0u64, 0u64);
    // Round 3's levers, over every life: wound meter the door cover and the carrying plate took off (cells), frames the funnel led out.
    let (mut door_abs, mut door_seen, mut carry_abs, mut carry_seen, mut out_frames, mut out_seen) = (0f32, 0f32, 0f32, 0f32, 0u64, 0u64);
    let mut instinct_set = false;
    let start_state = if camp || instinct {
        State::Camp
    } else if skulk {
        State::Stalk
    } else {
        State::Approach
    };
    let mut path_cache: Option<(u64, Vec<(i32, i32)>)> = None;
    let mut dives = 0u32;
    let mut brood_seen = 0u32;
    let mut dumped = false;
    let mut home_x: Option<i32> = None;

    let mut sc = Scenario::load("demo_enemy").expect("demo_enemy ships");
    sc.bed.seed = seed;
    let mut lab = Lab::new(sc.bed.clone());
    lab.show_help = false;
    if lab.stats.showing() {
        lab.stats.toggle();
    }
    if !oldant {
        lab.set_new_ant(true);
    }
    // `Lab::start_demo` loads the scenario from disk; the seed has to be set
    // on it first, so the launcher's three steps are done here by hand.
    let mut demo = Demo::new(DemoSeat::Enemy);
    // Every `key=value` reaches the seat, as `bin/lab.rs` hands them on
    // (`land=` is how far from the door the raider is set down).
    demo.args = std::env::args().skip(1).filter_map(|a| a.split_once('=').map(|(k, v)| (k.to_string(), v.to_string()))).collect();
    lab.demo = Some(demo);
    lab.load_scenario(sc);
    lab.demo_bind_body();

    let mut hold = Hold::default();
    let mut state = start_state;
    let mut state_since = 0u64;
    let mut last_head: Option<(i32, i32)> = None;
    let mut still = 0u64;
    let mut kills_seen = 0u32;
    let mut in_nest_frames = 0u64;
    let mut taken_in_nest = 0u32;
    let mut taken_seen = 0u32;
    let mut deepest = 0i32;
    let mut shots: Vec<image::RgbaImage> = Vec::new();
    let mut buf = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    let mut landed_at: Option<u64> = None;
    let mut start_cells: Option<usize> = None;
    let drawcost = flag("drawcost");
    let gif_from: u64 = arg("gif_from").unwrap_or(0);
    let gif_to: u64 = arg("gif_to").unwrap_or(u64::MAX);
    let snap: Option<String> = arg("snap");
    let mut snapped: Vec<&str> = Vec::new();
    let respawn = flag("respawn");
    let frames: u64 = arg("frames").unwrap_or(u64::MAX);
    let mut cost: Vec<(u64, u64)> = Vec::new();

    loop {
        lab.demo_frame();
        let e = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).cloned().expect("enemy seat");
        if landed_at.is_none() && e.landed.is_some() {
            landed_at = e.landed;
            println!(
                "landed frame={} ants={} trips={} raider_cells={}",
                lab.world.frame,
                e.ants_at_landing,
                e.trips_at_landing,
                e.raider.and_then(|id| lab.world.organism(id)).map_or(0, |s| s.chain.len())
            );
        }
        let Some(t0) = landed_at else {
            // `warm=` past `frames=`: the colony alone, no raider (the control
            // for anything the raider is supposed to cause).
            if lab.world.frame >= frames {
                break;
            }
            lab.tick_for_harness();
            if lab.world.frame.is_multiple_of(5000) {
                let mutants = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).map_or(0, |e| e.mutants);
                println!("warm frame={} ants={} mutants={mutants}", lab.world.frame, ants(&lab.world).len());
            }
            continue;
        };
        // SCRATCH positive control for the rebank: a corpse cell planted ahead of the
        // head at the lair, G held toward it; does the jaw take it and does it bank?
        if policy == "rebanktest" && landed_at.is_some_and(|t0| (lab.world.frame - t0) % 400 == 300 && lab.world.frame - t0 < 4000) {
            let id = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).and_then(|e| e.raider).unwrap();
            let ch = lab.world.organism(id).map(|s| s.chain.clone()).unwrap_or_default();
            if ch.len() >= 2 {
                let (h, n) = (ch[0], ch[1]);
                let dir = (h.0 - n.0, h.1 - n.1);
                let at = (h.0 + dir.0, h.1 + dir.1);
                let corpse = lab.world.materials.known(pixel_physics::sim::material::Known::Corpse).unwrap();
                let was = lab.world.get(at.0, at.1);
                if was.organism_id() == 0 {
                    lab.world.set(at.0, at.1, pixel_physics::sim::cell::Cell::new(corpse, 0));
                }
                let before = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).map_or(0, |e| e.brood);
                let key = match dir { (1, _) => DemoKey::Right, (-1, _) => DemoKey::Left, (_, 1) => DemoKey::Down, _ => DemoKey::Up };
                lab.demo_key(key, true);
                let mut got = None;
                for k in 0..60 {
                    if k % 10 == 0 { lab.demo_key(DemoKey::Letter('G'), true); lab.demo_key(DemoKey::Letter('G'), false); }
                    lab.demo_frame();
                    lab.tick_for_harness();
                    let sp = lab.world.organism(id).and_then(|s| s.spoil).map(|l| lab.world.materials.get(l.cell.material).name.clone());
                    if sp.is_some() && got.is_none() { got = Some((k, sp)); }
                    if let Some(a) = lab.world.steered.as_ref().and_then(|s| s.last_act).filter(|a| a.asked.is_some()) { if k % 10 == 9 { println!("  act k={k} {a:?} cell_now={}", lab.world.materials.get(lab.world.get(at.0, at.1).material).name); } }
                }
                lab.demo_key(key, false);
                let after = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).map_or(0, |e| e.brood);
                let la = lab.world.steered.as_ref().and_then(|s| s.last_act);
                println!("REBANKTEST head={h:?} dir={dir:?} planted_on={} jaw_took={got:?} brood {before}->{after} last_act={la:?}", if was.organism_id()==0 {"free"} else {"body"});
            }
        }
        if instinct && !instinct_set {
            instinct_set = true;
            lab.time.set_preset(3);
            println!("instinct: dial at {}x", lab.time.multiple());
        }
        let now = lab.world.frame;
        // The assist's counters (`EnemyDemo::funnel`, `held_flips`), over every life.
        funnel_frames += u64::from(e.funnel);
        held_flips += if e.held_flips >= held_seen { e.held_flips - held_seen } else { e.held_flips };
        held_seen = e.held_flips;
        door_abs += if e.door_absorbed >= door_seen { e.door_absorbed - door_seen } else { e.door_absorbed };
        door_seen = e.door_absorbed;
        carry_abs += if e.carry_absorbed >= carry_seen { e.carry_absorbed - carry_seen } else { e.carry_absorbed };
        carry_seen = e.carry_absorbed;
        out_frames += if e.funnelled_out >= out_seen { e.funnelled_out - out_seen } else { e.funnelled_out };
        out_seen = e.funnelled_out;
        if now >= t0 + after || (!respawn || e.over) && e.ended.is_some_and(|(f, _)| now > f + 300) {
            break;
        }
        // `respawn`: R 300 frames after each death, as a player would, so one
        // run plays season after season against the same colony.
        if respawn && !e.over && e.ended.is_some_and(|(f, _)| now == f + 300) {
            lab.demo_key(DemoKey::Letter('R'), true);
            state = start_state;
            path_cache = None;
            state_since = now;
            home_x = None;
            start_cells = None;
            kills_seen = 0;
            taken_seen = 0;
            last_head = None;
        }
        let body = e
            .raider
            .and_then(|id| lab.world.organism(id))
            .filter(|_| e.ended.is_none())
            .map(|s| (s.chain.first().copied().unwrap_or((0, 0)), s.chain.len(), s.gnawed, s.crop.map_or(0.0, |c| c.worth()), s.energy));
        let mut want = Hold::default();
        let tank = 1600.0f32;
        let holding = pixel_physics::lab::demo::enemy::carrying_larva(&lab.world, &e);
        if let Some((head, cells, gnawed, crop, energy)) = body {
            let start_cells = *start_cells.get_or_insert(cells);
            let (door_x, surface) = nearest_door(&lab.world, head.0).unwrap_or((head.0, head.1));
            let depth = head.1 - surface;
            deepest = deepest.max(depth);
            if depth >= 3 {
                in_nest_frames += 1;
            }
            if e.bites_taken > taken_seen {
                if depth >= 3 {
                    taken_in_nest += e.bites_taken - taken_seen;
                }
                taken_seen = e.bites_taken;
            }
            // Back off at the first cell lost or a wound half full: the door
            // play used to wait for two cells, and died on the mound first
            // (head off with 1-4 cells lost, 6 of 6, 2026-10-25).
            let hurt = gnawed >= 0.5 || cells < start_cells;
            // The massacre's home is the door itself.
            let home = *home_x.get_or_insert(if massacre { nearest_door(&lab.world, head.0).map_or(head.0, |d| d.0) } else { head.0 });
            // The nearest ant, for backing off: rest only where none is close,
            // because the first door runs rested on the foragers' road to the
            // fruit and were bitten to the head there (2026-10-25).
            let nearest = ants(&lab.world).into_iter().map(|a| (a, (a.0 - head.0).abs() + (a.1 - head.1).abs())).min_by_key(|&(_, d)| d);
            let crowded = nearest.is_some_and(|(_, d)| d <= 10);
            let next = match state {
                State::Stalk if gnawed >= 0.4 || cells < start_cells => State::Flee,
                State::Flee if now - state_since >= 300 && !crowded => State::Heal,
                State::Heal if crowded => State::Flee,
                // Rest until healed and whole (a centipede regrows its tail out
                // of a meal, `CreatureDef::regrows`), or 6,000 frames at most.
                State::Heal if gnawed < 0.05 && now - state_since >= 300 && (cells >= start_cells || now - state_since >= 6000) => State::Stalk,
                State::Approach | State::Hunt if hurt => State::Retreat,
                State::Approach if depth >= 3 => State::Hunt,
                State::Retreat if depth <= 0 && (head.0 - door_x).abs() >= 20 && !crowded => State::Rest,
                State::Rest if crowded => State::Retreat,
                State::Rest if now - state_since >= 1500 && (gnawed < 0.05 && cells >= start_cells || now - state_since >= 6000) => State::Approach,
                // A hungry raider stops resting and hunts (the stakes' hunger).
                State::Heal if energy < 0.35 * tank => State::Stalk,
                // **The heist**: fed, whole, healed and with its wind, it goes in.
                State::Stalk if heist && !larvae(&lab.world).is_empty() && energy >= dive_at * tank && cells >= start_cells && gnawed < 0.05 && e.spent < 0.05 && !e.in_grace(now) => {
                    dives += 1;
                    State::Dive
                }
                State::Dive if holding => State::Escape,
                State::Dive if larvae(&lab.world).is_empty() => State::Stalk,
                // Lost two cells with nothing in the jaws, or too long under: get out.
                State::Dive if cells + 2 <= start_cells || now - state_since >= 4000 => State::Escape,
                State::Escape if rebank && e.brood > brood_seen => State::Rebank,
                State::Rebank if holding => State::Rebank,
                State::Rebank if gnawed >= 0.4 || cells < start_cells || now - state_since >= 3000 => State::Heal,
                State::Escape if e.brood > brood_seen || (!holding && pixel_physics::lab::demo::enemy::cover(&lab.world, head) == 0) || now - state_since >= 4000 => State::Heal,
                st => st,
            };
            brood_seen = e.brood;
            if next != state {
                state = next;
                state_since = now;
                path_cache = None;
            }
            let mut press: Option<char> = None;
            match state {
                State::Camp => {}
                State::Rebank => {
                    let corpse = lab.world.materials.known(pixel_physics::sim::material::Known::Corpse);
                    let lair = e.lair.unwrap_or((home, surface));
                    let mut best: Option<((i32, i32), i32)> = None;
                    for y in lair.1 - 10..=lair.1 + 4 {
                        for x in lair.0 - 8..=lair.0 + 8 {
                            if !lab.world.in_bounds(x, y) { continue; }
                            let c = lab.world.get(x, y);
                            if Some(c.material) == corpse && c.organism_id() == 0 {
                                let d = (x - head.0).abs().max((y - head.1).abs());
                                if best.is_none_or(|b| d < b.1) { best = Some(((x, y), d)); }
                            }
                        }
                    }
                    if (now - state_since).is_multiple_of(60) {
                        let la = lab.world.steered.as_ref().and_then(|s| s.last_act);
                        let sp = e.raider.and_then(|id| lab.world.organism(id)).and_then(|s| s.spoil).map(|l| lab.world.materials.get(l.cell.material).name.clone());
                        println!("REBANK t={} head={head:?} lair={lair:?} target={best:?} spoil={sp:?} last_act={la:?}", now - t0);
                    }
                    if let Some(((cx, cy), d)) = best {
                        want.left = cx < head.0;
                        want.right = cx > head.0;
                        want.up = cy < head.1;
                        want.down = cy > head.1;
                        if d <= 1 && (now - state_since).is_multiple_of(10) {
                            press = Some('G');
                            rebank_digs += 1;
                        }
                    }
                }
                State::Dive | State::Escape if coarse => {
                    let site = lab.world.nest_sites.iter().map(|s| s.x).min_by_key(|x| (x - head.0).abs()).unwrap_or(door_x);
                    let inside = depth >= 3 && (head.0 - site).abs() <= 40;
                    if state == State::Dive {
                        if !inside {
                            // To the door's chevron; within ten columns of it, down and toward it.
                            let dx = site - head.0;
                            want.down = dx.abs() <= 10;
                            want.left = dx < -1;
                            want.right = dx > 1;
                        } else if let Some(l) = larvae(&lab.world).into_iter().min_by_key(|l| (l.0 - head.0).abs().max((l.1 - head.1).abs())) {
                            want.left = l.0 < head.0;
                            want.right = l.0 > head.0;
                            want.up = l.1 < head.1;
                            want.down = l.1 > head.1;
                        } else {
                            want.down = true;
                        }
                        want.sprint = sprints && depth >= -2;
                        // B when the ring says a larva is in reach, or an ant is in the way.
                        let ring = !pixel_physics::lab::demo::enemy::larvae_in_reach(&lab.world, head).is_empty();
                        if ring || nearest.is_some_and(|(_, d)| d <= 2) && cells + 1 >= start_cells {
                            press = Some('B');
                        }
                    } else {
                        // Up and out, then along the surface to the flag.
                        let lair = e.lair.unwrap_or((home, surface));
                        if depth >= 1 && (head.0 - site).abs() <= 40 {
                            want.up = true;
                        } else {
                            want.left = lair.0 < head.0 - 1;
                            want.right = lair.0 > head.0 + 1;
                        }
                        want.sprint = sprints;
                    }
                }
                State::Dive | State::Escape => {
                    // Re-plan every 8 frames: ants move, and so does the body.
                    let stale = path_cache.as_ref().is_none_or(|(f, _)| now >= f + 8);
                    if stale {
                        let me = e.raider.unwrap_or(0);
                        let path = if state == State::Dive {
                            let lv = larvae(&lab.world);
                            crawl_path(&lab.world, me, head, &|x, y| lv.iter().any(|l| (l.0 - x).abs().max((l.1 - y).abs()) <= 2))
                        } else {
                            let lair = e.lair.unwrap_or((home, surface));
                            crawl_path(&lab.world, me, head, &|x, y| (x - lair.0).abs() <= 3 && (y - lair.1).abs() <= 3)
                        };
                        if path.is_none() && flag("dump") && !dumped {
                            dumped = true;
                            let lv = larvae(&lab.world);
                            println!("DUMP head={head:?} larvae={}", lv.len());
                            for y in head.1 - 30..head.1 + 50 {
                                let row: String = (head.0 - 90..head.0 + 30)
                                    .map(|x| {
                                        if !lab.world.in_bounds(x, y) {
                                            return ' ';
                                        }
                                        let c = lab.world.get(x, y);
                                        let o = c.organism_id();
                                        if o == me {
                                            'R'
                                        } else if lv.contains(&(x, y)) {
                                            'L'
                                        } else if o != 0 {
                                            if lab.world.organism(o).is_some_and(|s| s.brood.is_some()) { 'b' } else { 'a' }
                                        } else if c.is_empty() {
                                            '.'
                                        } else {
                                            '#'
                                        }
                                    })
                                    .collect();
                                println!("{y:4} {row}");
                            }
                        }
                        // `dumpat=T`: the ground round the head at the first re-plan at or after T
                        // frames from the landing, the planned path as `p`.
                        if arg::<u64>("dumpat").is_some_and(|t| now - t0 >= t) && !dumped {
                            dumped = true;
                            let p = path.clone().unwrap_or_default();
                            println!("DUMPAT t={} head={head:?} path={}", now - t0, p.len());
                            for y in head.1 - 15..head.1 + 40 {
                                let row: String = (head.0 - 45..head.0 + 45)
                                    .map(|x| {
                                        if !lab.world.in_bounds(x, y) {
                                            return ' ';
                                        }
                                        let c = lab.world.get(x, y);
                                        let o = c.organism_id();
                                        if o == me {
                                            'R'
                                        } else if o != 0 {
                                            if lab.world.organism(o).is_some_and(|s| s.brood.is_some()) { 'b' } else { 'a' }
                                        } else if p.contains(&(x, y)) {
                                            'p'
                                        } else if c.is_empty() {
                                            '.'
                                        } else {
                                            '#'
                                        }
                                    })
                                    .collect();
                                println!("{y:4} {row}");
                            }
                        }
                        path_cache = Some((now, path.unwrap_or_default()));
                    }
                    let path = path_cache.as_ref().map(|(_, p)| p.as_slice()).unwrap_or(&[]);
                    // Follow the path from where the head is on it now.
                    let at = path.iter().position(|&p| p == head).unwrap_or(0);
                    // The next cell on the path; when stuck, a farther one (a corner the body cannot take square).
                    let ahead = if still >= 40 { 2 + (still / 40) as usize % 3 } else { 1 };
                    match path.get((at + ahead).min(path.len().saturating_sub(1))) {
                        Some(&(tx, ty)) if path.len() > 1 => {
                            want.left = tx < head.0;
                            want.right = tx > head.0;
                            want.up = ty < head.1;
                            want.down = ty > head.1;
                        }
                        _ if state == State::Dive => {
                            // No path: at the door, push down.
                            if (head.0 - door_x).abs() > 1 {
                                want.left = head.0 > door_x;
                                want.right = head.0 < door_x;
                            } else {
                                want.down = true;
                            }
                        }
                        _ => {
                            want.up = true;
                        }
                    }
                    want.sprint = sprints && (state == State::Escape || depth >= -2 || (head.0 - door_x).abs() <= 12);
                    if state == State::Dive && larvae(&lab.world).iter().any(|l| (l.0 - head.0).abs().max((l.1 - head.1).abs()) <= pixel_physics::lab::demo::enemy::JAW_REACH) {
                        press = Some('B');
                    } else if state == State::Dive && nearest.is_some_and(|(_, d)| d <= 2) && cells + 1 >= start_cells {
                        // An ant in the way: bite through it.
                        press = Some('B');
                    }
                }
                State::Approach => {
                    if (head.0 - door_x).abs() > 1 {
                        want.left = head.0 > door_x;
                        want.right = head.0 < door_x;
                    } else {
                        want.down = true;
                    }
                }
                State::Hunt => {
                    let target = ants(&lab.world).into_iter().map(|a| (a, (a.0 - head.0).abs() + (a.1 - head.1).abs())).filter(|&(_, d)| d <= 24).min_by_key(|&(_, d)| d);
                    match target {
                        Some((a, d)) => {
                            let (dx, dy) = (a.0 - head.0, a.1 - head.1);
                            if dx.abs() >= dy.abs() {
                                want.left = dx < 0;
                                want.right = dx > 0;
                            } else {
                                want.up = dy < 0;
                                want.down = dy > 0;
                            }
                            if d <= 2 {
                                press = Some('B');
                            }
                        }
                        // Nobody near: go deeper, toward the chambers.
                        None => want.down = true,
                    }
                    if e.kills > kills_seen {
                        kills_seen = e.kills;
                        press = Some('E');
                    }
                }
                State::Retreat => {
                    // Out of the nest first, then away from the door along the
                    // surface, at a sprint when there is wind for it.
                    if depth >= 3 {
                        want.up = true;
                    } else {
                        // Away from the nearest ant if one is close, else from the door.
                        let from = nearest.filter(|&(_, d)| d <= 10).map_or(door_x, |(a, _)| a.0);
                        want.left = head.0 <= from;
                        want.right = head.0 > from;
                    }
                    want.sprint = sprints;
                }
                State::Rest | State::Heal => {}
                State::Stalk => {
                    // A forager is fair game when it walks alone and away
                    // from the mound: no more than one other ant within 6
                    // cells of it, and 12 columns from the door or more.
                    let all = ants(&lab.world);
                    let alone = |a: (i32, i32)| all.iter().filter(|b| **b != a && (b.0 - a.0).abs() + (b.1 - a.1).abs() <= 6).count() <= 1;
                    let target = all.iter().copied().filter(|a| massacre || (a.0 - door_x).abs() >= 12 && alone(*a)).map(|a| (a, (a.0 - head.0).abs() + (a.1 - head.1).abs())).filter(|&(_, d)| d <= 16).min_by_key(|&(_, d)| d);
                    match target {
                        Some((a, d)) => {
                            let dx = a.0 - head.0;
                            want.left = dx < 0;
                            want.right = dx > 0;
                            if d <= 2 {
                                press = Some('B');
                            }
                        }
                        None if (head.0 - home).abs() > 2 => {
                            want.left = head.0 > home;
                            want.right = head.0 < home;
                        }
                        None => {}
                    }
                    if e.kills > kills_seen {
                        kills_seen = e.kills;
                        press = Some('E');
                    }
                }
                State::Flee => {
                    let from = nearest.filter(|&(_, d)| d <= 10).map_or(door_x, |(a, _)| a.0);
                    want.left = head.0 <= from;
                    want.right = head.0 > from;
                    want.sprint = sprints;
                }
            }
            if Some(head) == last_head && (want.left || want.right || want.up || want.down) {
                still += 1;
            } else {
                still = 0;
            }
            last_head = Some(head);
            if still >= 120 && press.is_none() {
                press = Some('G');
                still = 0;
            }
            // SCRATCH `lag=N`: a human's reaction time -- the keys change only every N frames.
            let lag: u64 = arg("lag").unwrap_or(1).max(1);
            if lag > 1 && !(now - t0).is_multiple_of(lag) {
                want = hold;
                press = None;
            }
            // `trace`: every frame of a dive or an escape, keys and all.
            if flag("trace") && matches!(state, State::Dive | State::Escape) {
                let la = lab.world.steered.as_ref().and_then(|s| s.last_act);
                let k = |b: bool, c: char| if b { c } else { '.' };
                println!(
                    "TR t={} {state:?} head={head:?} cells={cells} gn={gnawed:.2} keys={}{}{}{}{} press={press:?} still={still} near={:?} path={} grabbed={} hold={holding} act={:?}",
                    now - t0,
                    k(want.left, 'L'),
                    k(want.right, 'R'),
                    k(want.up, 'U'),
                    k(want.down, 'D'),
                    k(want.sprint, 'S'),
                    nearest.map(|n| n.1),
                    path_cache.as_ref().map_or(0, |(_, p)| p.len()),
                    e.grabbed,
                    la.map(|a| (a.frame, a.asked, a.done, a.ok))
                );
                // The assist's view: the nearest door, its ground line, the funnel's way in, the input it left.
                let site = lab.world.nest_sites.iter().min_by_key(|s| (s.x - head.0).abs()).map(|s| (s.x, s.surface));
                let way = site.and_then(|(_, sf)| pixel_physics::lab::demo::enemy::way_in(&lab.world, e.raider.unwrap_or(0), head, sf));
                let input = lab.world.steered.as_ref().and_then(|s| s.input).map(|i| i.dir);
                println!("   AS site={site:?} way_in={way:?} funnel={} input={input:?} cover_from={:?} door_cells={:.2} carry_cells={:.2}", e.funnel, e.cover_from, e.door_absorbed, e.carry_absorbed);
                if arg::<u64>("mapat") == Some(now - t0) {
                    for y in head.1 - 8..head.1 + 16 {
                        let row: String = (head.0 - 12..head.0 + 16)
                            .map(|x| {
                                let c = lab.world.get(x, y);
                                let o = c.organism_id();
                                if (x, y) == head {
                                    'H'
                                } else if o != 0 && Some(o) == e.raider {
                                    'R'
                                } else if o != 0 {
                                    'a'
                                } else if c.is_empty() {
                                    '.'
                                } else {
                                    '#'
                                }
                            })
                            .collect();
                        println!("   MAP {y:4} {row}");
                    }
                }
            }
            if let Some(c) = press {
                lab.demo_key(DemoKey::Letter(c), true);
                lab.demo_key(DemoKey::Letter(c), false);
            }
            if (now - t0).is_multiple_of(row_every) {
                println!(
                    "row t={} state={state:?} head={head:?} depth={depth} cells={cells} gnawed={gnawed:.2} crop={:.2} energy={:.2} kills={} given={} taken={} taken_in_nest={taken_in_nest} ants={} trips={} grabbed={} brood={} holding={holding} score={} path={:?}",
                    now - t0,
                    crop,
                    energy,
                    e.kills,
                    e.bites_given,
                    e.bites_taken,
                    ants(&lab.world).len(),
                    lab.world.creature_stats.trip_deliveries - e.trips_at_landing,
                    e.grabbed,
                    e.brood,
                    e.score(),
                    path_cache.as_ref().map(|(_, p)| (p.len(), p.get(1).copied(), p.get(4).copied())),
                );
            }
        }
        for (key, on, was) in [(DemoKey::Left, want.left, hold.left), (DemoKey::Right, want.right, hold.right), (DemoKey::Up, want.up, hold.up), (DemoKey::Down, want.down, hold.down), (DemoKey::Sprint, want.sprint, hold.sprint)] {
            if on != was {
                lab.demo_key(key, on);
            }
        }
        hold = want;
        let spoil_before = e.raider.and_then(|id| lab.world.organism(id)).and_then(|s| s.spoil).map(|l| (l.cell.material, l.store));
        lab.tick_for_harness();
        if flag("spoiltrace") {
            let e2 = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).cloned().expect("enemy seat");
            let sp = e2.raider.and_then(|id| lab.world.organism(id)).and_then(|s| s.spoil).map(|l| (l.cell.material, l.store));
            if sp != spoil_before || e2.brood != e.brood {
                let name = |m: Option<(pixel_physics::sim::material::MaterialId, bool)>| m.map(|(m, st)| format!("{}{}", lab.world.materials.get(m).name, if st { "(store)" } else { "" }));
                let la = lab.world.steered.as_ref().and_then(|s| s.last_act);
                let head = e2.raider.and_then(|id| lab.world.organism(id)).and_then(|s| s.chain.first().copied());
                println!("SPOIL t={} {:?} -> {:?} brood {}->{} grabbed {} head={head:?} lair={:?} last_act={la:?}", lab.world.frame - landed_at.unwrap_or(0), name(spoil_before), name(sp), e.brood, e2.brood, e2.grabbed, e2.lair);
            }
        }
        // `drawcost`: every displayed frame drawn twice, the demo's view (the
        // camera following, the marks) and then the plain lab at the same
        // camera with the seat taken away, so the difference is what the
        // seat costs the draw. The plain draw follows a demo draw, so its
        // dirty-rect skip is the box's own between two frames.
        if drawcost {
            let t = std::time::Instant::now();
            lab.draw(&mut buf, 60.0);
            let with = t.elapsed().as_micros() as u64;
            let seat = lab.demo.take();
            let t = std::time::Instant::now();
            lab.draw(&mut buf, 60.0);
            let without = t.elapsed().as_micros() as u64;
            lab.demo = seat;
            cost.push((with, without));
        }
        let since = lab.world.frame - t0;
        if gif.is_some() && since.is_multiple_of(every) && since >= gif_from && since < gif_to {
            lab.draw(&mut buf, 60.0);
            shots.push(image::RgbaImage::from_raw(WIDTH, HEIGHT, buf.clone()).expect("frame size"));
        }
        // `snap=prefix`: stills at the heist's beats -- the first frame a
        // larva is in the jaws (`-grab`), the first with it in the jaws and
        // the head out from under the ground (`-out`), the first bank
        // (`-bank`) -- each with its frame in the name's log line.
        if let Some(prefix) = snap.as_deref() {
            let e = lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).expect("enemy seat");
            let head = e.raider.and_then(|id| lab.world.organism(id)).and_then(|s| s.chain.first().copied());
            let hold = pixel_physics::lab::demo::enemy::carrying_larva(&lab.world, e);
            let out = head.is_some_and(|h| pixel_physics::lab::demo::enemy::cover(&lab.world, h) == 0);
            let banked: u32 = e.game.iter().map(|l| l.brood).sum::<u32>() + e.brood;
            // `-deep`: the first frame with a larva in the jaws under 8 rows of roof.
            let deep = hold && head.is_some_and(|h| pixel_physics::lab::demo::enemy::cover(&lab.world, h) >= 8);
            for (beat, now_true) in [("grab", hold), ("deep", deep), ("out", hold && out), ("bank", banked > 0)] {
                if now_true && !snapped.contains(&beat) {
                    snapped.push(beat);
                    lab.draw(&mut buf, 60.0);
                    let path = format!("{prefix}-{beat}.png");
                    image::RgbaImage::from_raw(WIDTH, HEIGHT, buf.clone()).expect("frame size").save(&path).expect("snap png");
                    println!("snap {path} t={since}");
                }
            }
        }
    }
    if !cost.is_empty() {
        let stat = |v: &mut Vec<u64>| {
            v.sort_unstable();
            (v.iter().sum::<u64>() / v.len() as u64, v[v.len() * 99 / 100], v[v.len() - 1])
        };
        let (mut a, mut b): (Vec<u64>, Vec<u64>) = cost.iter().copied().unzip();
        let ((am, a99, amax), (bm, b99, bmax)) = (stat(&mut a), stat(&mut b));
        println!("drawcost frames={} demo_us mean={am} p99={a99} max={amax}  plain_us mean={bm} p99={b99} max={bmax}", cost.len());
    }
    // `card=x.png`: the last frame with the season card up (Y, as the
    // player opens it; a death has already opened it).
    if let Some(path) = arg::<String>("card") {
        if !lab.demo.as_ref().and_then(|d| d.enemy.as_ref()).is_some_and(|e| e.card) {
            lab.demo_key(DemoKey::Letter('Y'), true);
        }
        lab.draw(&mut buf, 60.0);
        image::RgbaImage::from_raw(WIDTH, HEIGHT, buf.clone()).expect("frame size").save(&path).expect("card png");
        println!("card {path}");
    }
    let e = lab.demo.as_ref().and_then(|d| d.enemy.clone()).expect("enemy seat");
    let t0 = landed_at.unwrap_or(lab.world.frame);
    let ran = lab.world.frame - t0;
    let per_min = if in_nest_frames > 0 { taken_in_nest as f64 * 3600.0 / in_nest_frames as f64 } else { 0.0 };
    println!(
        "RESULT seed={seed} brain={} policy={} ran={ran} end={} in_nest_frames={in_nest_frames} deepest={deepest} kills={} given={} taken={} taken_in_nest={taken_in_nest} taken_per_min_in_nest={per_min:.1} cells_lost={} ants_at_landing={} ants_end={} trips_since={} sprint_frames={} regrown={}",
        if oldant { "hand-built" } else { "trained" },
        policy,
        e.ended.map_or("alive".to_string(), |(_, why)| why.replace(' ', "_")),
        e.kills,
        e.bites_given,
        e.bites_taken,
        e.cells_lost.len(),
        e.ants_at_landing,
        ants(&lab.world).len(),
        lab.world.creature_stats.trip_deliveries - e.trips_at_landing,
        e.sprint_frames,
        lab.world.creature_stats.regrown_cells,
    );
    // One line a season (each raider life), then the one still running.
    let mut seasons = e.seasons.clone();
    if e.ended.is_none() {
        seasons.push(pixel_physics::lab::demo::enemy::Season {
            frames: lab.world.frame.saturating_sub(e.landed.unwrap_or(lab.world.frame)),
            kills: e.kills,
            bites_taken: e.bites_taken,
            contact: e.contact,
            cells_lost: e.cells_lost.len(),
            mutants: e.mutants - e.mutants_at_landing,
        });
    }
    for (i, s) in seasons.iter().enumerate() {
        println!(
            "SEASON seed={seed} brain={} mutate={} n={} frames={} kills={} taken={} contact={} rate={:.2} cells_lost={} mutants={}",
            if oldant { "hand-built" } else { "trained" },
            e.mutating,
            i + 1,
            s.frames,
            s.kills,
            s.bites_taken,
            s.contact,
            s.bite_rate(),
            s.cells_lost,
            s.mutants
        );
    }
    println!("MUTANTS total={} ants_end={}", e.mutants, ants(&lab.world).len());
    // The stakes: one LIFE line a raider of the game (the one still alive
    // last, marked ALIVE), then the GAME line.
    if e.stakes {
        let mut lives = e.game.clone();
        if e.ended.is_none() && e.raider.is_some() {
            lives.push(pixel_physics::lab::demo::enemy::Life {
                frames: lab.world.frame.saturating_sub(e.landed.unwrap_or(lab.world.frame)),
                kills: e.kills,
                meals: e.meals,
                grabbed: e.grabbed,
                brood: e.brood,
                deepest: e.deepest,
                how: "ALIVE",
            });
        }
        for (i, l) in lives.iter().enumerate() {
            println!(
                "LIFE seed={seed} policy={policy} n={} frames={} kills={} meals={} grabbed={} brood={} deepest={} below={} points={} how={}",
                i + 1,
                l.frames,
                l.kills,
                l.meals,
                l.grabbed,
                l.brood,
                l.deepest,
                u8::from(l.deepest >= pixel_physics::lab::demo::enemy::BELOW),
                l.points(),
                l.how
            );
        }
        let lag: u64 = arg("lag").unwrap_or(1).max(1);
        println!(
            "GAME seed={seed} policy={policy} lag={lag} funnel_frames={funnel_frames} held_flips={held_flips} door_cells={door_abs:.2} carry_cells={carry_abs:.2} out_frames={out_frames} lives={} over={} score={} kills={} meals={} grabbed={} brood={} ants_start={} ants_end={} rebank_digs={rebank_digs} dives={dives} hunger_j={:.0} capped_j={:.0}",
            lives.len(),
            e.over,
            e.score().max(lives.iter().map(|l| l.points()).sum()),
            lives.iter().map(|l| l.kills).sum::<u32>(),
            lives.iter().map(|l| l.meals).sum::<u32>(),
            lives.iter().map(|l| l.grabbed).sum::<u32>(),
            lives.iter().map(|l| l.brood).sum::<u32>(),
            e.game_ants_at_start,
            ants(&lab.world).len(),
            e.hunger_j,
            e.capped_j
        );
    }
    if let Some(path) = gif {
        let delay = image::Delay::from_numer_denom_ms(every as u32 * 1000, 60);
        let frames: Vec<image::Frame> = shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, delay)).collect();
        let n = frames.len();
        let file = std::fs::File::create(&path).expect("gif file");
        let mut enc = image::codecs::gif::GifEncoder::new_with_speed(file, 10);
        enc.set_repeat(image::codecs::gif::Repeat::Infinite).expect("repeat");
        enc.encode_frames(frames).expect("gif encode");
        println!("gif {path} frames={n}");
    }
}
