//! **The nest, the brood and the hungry, as chronicle columns.**
//!
//! Built 2026-10-05 for the playtest that follows the evolved-ant-with-nurses
//! and nest-plan merges, because the chronicle on main could answer none of
//! the questions those changes are judged on: it had no brood at all (`ants`
//! leaves eggs, larvae and pupae out by design, and a starved larva books no
//! death, so `starved` never counted one), it could not say whether the nest
//! is one room or several (`roofed` and `dug_*` are cell counts), and it
//! could not say where a hungry ant stands or where a starved one died. The
//! brief is `/mnt/project-files/playtest-2026-10-05/chronicle-gaps.md`.
//!
//! **Nothing here changes the simulation.** Every number is a read of the
//! world, or a counter the engine already keeps (`CreatureStats`'s brood and
//! nest-plan fields, the ones `examples/deeptrace.rs` writes to `stats.csv`);
//! the one thing added to `src/sim` is a tally of *where* an animal died
//! (`GroupDeaths::by_place`), read by no rule.
//!
//! **The definitions are `examples/deeptrace.rs`'s and
//! `scripts/deeptrace_dig.py`'s on purpose**, so a number in a playtest's
//! chronicle and the same number in a goal-box trace are the same quantity:
//!
//! - *open* is any cell below the original ground that is not ground, where
//!   ground is a powder or solid that is not food and belongs to no organism
//!   (so an ant, a brood cell, water, a root and a heap of food are all open);
//! - a *room cell* is an open cell with at least seven of its own 3x3 open
//!   (itself included), which erodes tunnels and the mouth of a shaft away;
//! - a *room* is a 4-connected set of room cells; the *biggest room* is the
//!   largest, and every other room of 30+ cells is reported with its steps
//!   from the biggest through open cells (capped at 60; 999 means further, or
//!   not joined at all) and the brood standing in it;
//! - *under* the ground means strictly below the datum's row (the top ground
//!   row is the doorway), as `examples/deeptrace.rs`'s `zone` has it;
//! - *hungry* is an animal under its start energy, the line `HUNGRY_OUT`
//!   pulls on and the line a larva can be fed above.
//!
//! **Two differences from the deep trace, stated so they are not discovered.**
//! The census walks the whole box, not the 161-column window round one nest,
//! so a box with two colonies gets both colonies' rooms in one list; and the
//! trace's `food` zone does not exist here (a heap is just mound or surface).
//! And brood is counted as live *items*, where a trace map counts brood-coloured
//! *cells*: an ant standing on a brood item's cell hides it from the map, so
//! the same world reads a few percent fewer brood there (8 of 120 items on a
//! 60,000-frame goal-box world, 32 of 172 on a 120,000-frame one). The open
//! cells, the deepest row and the biggest room agree exactly: the Python
//! reader, run on dumps of four such worlds, gave the same 401, 658, 649 and
//! 606 open cells and 322, 552, 555 and 512 room cells.

use crate::lab::census::{extents, surface_of};
use crate::lab::scene::LabBox;
use crate::sim::cell::Cell;
use crate::sim::creature::food_value;
use crate::sim::material::{MaterialId, MaterialKind};
use crate::sim::organism::{BroodStage, DeathCause, CREATURE_TRAITS, DEATH_CAUSES};
use crate::sim::world::{World, DEATH_PLACES, NEAR_NEST_COLS};

/// A brood item this many columns from a nest site or closer is *at the
/// door*: the width of the lane the deep trace found the whole brood laid in.
pub const DOOR_COLS: i32 = 4;
/// The fewest room cells a room needs to be named: below it is a brood
/// pocket or a bulge, not a chamber (`scripts/deeptrace_dig.py`).
pub const ROOM_MIN: usize = 30;
/// Open cells, out of the nine of a 3x3 centred on a cell (itself included),
/// that make it a room cell.
pub const ROOM_OPEN_OF_NINE: usize = 7;
/// How far a room's distance from the biggest is followed, in steps.
pub const ROOM_STEPS_CAP: u32 = 60;
/// A room that the walk from the biggest never reached.
pub const ROOM_UNREACHED: u32 = 999;
/// Rows above an animal's head searched for ground, to tell the inside of
/// the spoil mound from its top.
pub const COVER_ROWS: i32 = 30;
/// The four places an animal is told to be in, in `NestRow`'s field order.
pub const PLACE_NAMES: [&str; 4] = ["under", "mound_in", "mound_top", "afield"];
/// The four cause groups the death columns use, in `NestRow`'s order.
pub const CAUSE_GROUP_NAMES: [&str; 4] = ["starved", "killed", "oldage", "other"];

/// **Everything the nest census adds to a chronicle row**, flat numbers only:
/// `census::census_csv` reads its columns off this struct's `Debug`, as it
/// does `Sample`'s, so a field added here reaches `census.csv` with no edit
/// there. Counters are cumulative over the run like `births`/`eats`/`digs`;
/// the rest are the box now.
#[derive(Default, Clone, Copy, Debug)]
pub struct NestRow {
    // --- brood standing now -------------------------------------------------
    pub brood_eggs: usize,
    pub brood_larvae: usize,
    /// ...of the larvae, the ones holding less than the bank they must reach
    /// to pupate: the hungry ones.
    pub brood_larvae_hungry: usize,
    pub brood_pupae: usize,
    /// All brood strictly below the original ground, and the median of how
    /// many rows below (0 when none is).
    pub brood_under: usize,
    pub brood_depth_med: i32,
    /// All brood within [`DOOR_COLS`] columns of a nest site: the brood
    /// column the deep trace found the whole brood laid and left in.
    pub brood_near_door: usize,
    // --- rooms --------------------------------------------------------------
    /// Open cells strictly below the original ground, and the deepest row
    /// of one (rows below the datum).
    pub open_under: usize,
    pub deepest_row: i32,
    /// The biggest room (room cells) and the brood standing in it; 0 when
    /// the box has no room cell at all.
    pub room_big: usize,
    pub room_big_brood: usize,
    /// How many OTHER rooms have [`ROOM_MIN`]+ cells, however many are
    /// listed below.
    pub rooms_30: usize,
    /// The three biggest of them: cells, steps from the biggest room
    /// ([`ROOM_UNREACHED`] when not reached), brood in it. All 0 when absent.
    pub room2_cells: usize,
    pub room2_steps: u32,
    pub room2_brood: usize,
    pub room3_cells: usize,
    pub room3_steps: u32,
    pub room3_brood: usize,
    pub room4_cells: usize,
    pub room4_steps: u32,
    pub room4_brood: usize,
    // --- the colony's animals by place (see PLACE_NAMES) ---------------------
    pub ants_under: usize,
    pub ants_mound_in: usize,
    pub ants_mound_top: usize,
    pub ants_afield: usize,
    /// ...of them, the ones under their start energy.
    pub hungry_under: usize,
    pub hungry_mound_in: usize,
    pub hungry_mound_top: usize,
    pub hungry_afield: usize,
    /// Mean energy over start energy in each place (1.0 = exactly at start).
    pub fill_under: f32,
    pub fill_mound_in: f32,
    pub fill_mound_top: f32,
    pub fill_afield: f32,
    // --- the colony's deaths by cause group and place, cumulative ------------
    pub starved_under: u64,
    pub starved_near: u64,
    pub starved_afield: u64,
    pub killed_under: u64,
    pub killed_near: u64,
    pub killed_afield: u64,
    pub oldage_under: u64,
    pub oldage_near: u64,
    pub oldage_afield: u64,
    pub other_under: u64,
    pub other_near: u64,
    pub other_afield: u64,
    // --- the brood's books, cumulative (CreatureStats) -----------------------
    pub eggs_laid: u64,
    pub eggs_to_larvae: u64,
    pub larvae_to_pupae: u64,
    pub larvae_starved: u64,
    pub brood_lost: u64,
    pub hatches_denied: u64,
    pub brood_held: u64,
    pub brood_carried: u64,
    pub brood_spread: u64,
    pub nurse_seeks: u64,
    pub larva_ticks_hungry: u64,
    pub larva_ticks_nursed: u64,
    pub larva_ticks_crop_fed: u64,
    pub brood_crop_fed_j: f64,
    pub brood_shared_j: f64,
    pub brood_nursed_j: f64,
    pub brood_ate_j: f64,
    pub brood_upkeep_j: f64,
    pub brood_corpse_j: f64,
    pub lays_declined: u64,
    pub births_denied_no_space: u64,
    // --- the nest plan's "it fired" counters and the foraging funnel ---------
    pub soil_way_pulls: u64,
    pub hungry_out_pulls: u64,
    pub crop_down_holds: u64,
    pub digs_refused_face: u64,
    pub digs_faced: u64,
    pub spoil_held_below: u64,
    pub spoil_kept_inside: u64,
    pub spoil_dumped: u64,
    pub spoil_lifted: u64,
    pub spoil_lifted_out: u64,
    pub lean_dropped: u64,
    pub pickups: u64,
    pub drops: u64,
    pub forage_trips: u64,
    pub forage_returns: u64,
    pub nest_visits: u64,
}

/// **The rooms of a grid of open cells**, the pure half of the room census
/// (`rooms_of`'s caller does the world reading). `open` is `w` x `h`, row
/// major; `brood` is the `(column, row)` of each standing brood item. Cells
/// on the grid's own border are never room cells, as in
/// `scripts/deeptrace_dig.py`'s `room_census`, which this ports.
pub fn rooms_of(open: &[bool], w: usize, h: usize, brood: &[(usize, usize)]) -> RoomCensus {
    let mut out = RoomCensus::default();
    if w < 3 || h < 3 {
        return out;
    }
    let idx = |x: usize, y: usize| y * w + x;
    let mut room = vec![false; w * h];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            if !open[idx(x, y)] {
                continue;
            }
            let mut n = 0;
            for yy in y - 1..=y + 1 {
                for xx in x - 1..=x + 1 {
                    n += usize::from(open[idx(xx, yy)]);
                }
            }
            room[idx(x, y)] = n >= ROOM_OPEN_OF_NINE;
        }
    }
    // 4-connected components of room cells, discovered in row-major order
    // (the Python reader's order, so equal-sized rooms rank the same).
    let mut label = vec![0u32; w * h];
    let mut comps: Vec<Vec<usize>> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    for sy in 0..h {
        for sx in 0..w {
            if !room[idx(sx, sy)] || label[idx(sx, sy)] != 0 {
                continue;
            }
            let id = comps.len() as u32 + 1;
            let mut cells = Vec::new();
            label[idx(sx, sy)] = id;
            stack.push(idx(sx, sy));
            while let Some(i) = stack.pop() {
                cells.push(i);
                let (x, y) = (i % w, i / w);
                let mut visit = |nx: usize, ny: usize| {
                    let j = idx(nx, ny);
                    if room[j] && label[j] == 0 {
                        label[j] = id;
                        stack.push(j);
                    }
                };
                if x + 1 < w {
                    visit(x + 1, y);
                }
                if x > 0 {
                    visit(x - 1, y);
                }
                if y + 1 < h {
                    visit(x, y + 1);
                }
                if y > 0 {
                    visit(x, y - 1);
                }
            }
            comps.push(cells);
        }
    }
    if comps.is_empty() {
        return out;
    }
    // Biggest first; a stable sort keeps discovery order among equals.
    let mut order: Vec<usize> = (0..comps.len()).collect();
    order.sort_by_key(|&c| std::cmp::Reverse(comps[c].len()));
    let mut brood_in = vec![0usize; comps.len()];
    for &(bx, by) in brood {
        if bx < w && by < h && label[idx(bx, by)] != 0 {
            brood_in[label[idx(bx, by)] as usize - 1] += 1;
        }
    }
    // Steps from the biggest room through open cells, followed to the cap.
    let mut dist = vec![u32::MAX; w * h];
    let mut queue = std::collections::VecDeque::new();
    for &i in &comps[order[0]] {
        dist[i] = 0;
        queue.push_back(i);
    }
    while let Some(i) = queue.pop_front() {
        if dist[i] >= ROOM_STEPS_CAP {
            continue;
        }
        let (x, y) = (i % w, i / w);
        let mut nbrs = [usize::MAX; 4];
        if x + 1 < w {
            nbrs[0] = idx(x + 1, y);
        }
        if x > 0 {
            nbrs[1] = idx(x - 1, y);
        }
        if y + 1 < h {
            nbrs[2] = idx(x, y + 1);
        }
        if y > 0 {
            nbrs[3] = idx(x, y - 1);
        }
        for j in nbrs {
            if j != usize::MAX && open[j] && dist[j] == u32::MAX {
                dist[j] = dist[i] + 1;
                queue.push_back(j);
            }
        }
    }
    out.big = comps[order[0]].len();
    out.big_brood = brood_in[order[0]];
    for &c in &order[1..] {
        if comps[c].len() < ROOM_MIN {
            continue;
        }
        let steps = comps[c]
            .iter()
            .map(|&i| dist[i])
            .min()
            .filter(|&d| d != u32::MAX)
            .unwrap_or(ROOM_UNREACHED);
        out.others.push((comps[c].len(), steps, brood_in[c]));
    }
    out
}

/// What [`rooms_of`] found: the biggest room and every other of
/// [`ROOM_MIN`]+ cells, biggest first.
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct RoomCensus {
    /// Room cells of the biggest room; 0 when there is none.
    pub big: usize,
    pub big_brood: usize,
    /// `(room cells, steps from the biggest, brood in it)` for each other.
    pub others: Vec<(usize, u32, usize)>,
}

/// Which of [`PLACE_NAMES`] an animal's head is in, as an index.
///
/// `surface` is the datum row of the head's column, `near_nest` whether a
/// nest site lies within [`NEAR_NEST_COLS`] columns, and `covered` whether
/// ground stands within [`COVER_ROWS`] rows above the head (asked only for an
/// animal above ground near a nest). Under is strictly below the datum row.
pub fn place_index(y: i32, surface: i32, near_nest: bool, covered: impl FnOnce() -> bool) -> usize {
    if y > surface {
        0
    } else if near_nest {
        if covered() {
            1
        } else {
            2
        }
    } else {
        3
    }
}

/// Which of [`CAUSE_GROUP_NAMES`] a death cause belongs to: starvation
/// (in flight too), being killed, old age, everything else.
pub fn cause_group(cause_index: usize) -> usize {
    if cause_index == DeathCause::Starved.index() || cause_index == DeathCause::StarvedInFlight.index() {
        0
    } else if cause_index == DeathCause::Killed.index() {
        1
    } else if cause_index == DeathCause::OldAge.index() {
        2
    } else {
        3
    }
}

/// **Take the nest census** off `world` right now: the [`NestRow`], and the
/// mean of each heritable trait over the colony species' living animals
/// (zeros when there are none).
///
/// `nest_cols` are the columns of the nests (`census::nest_columns`'s answer),
/// `colony_species` the species whose animals count as the colony -- the
/// same name the `starved`/`killd`/`oldag` columns read. Costs one pass over
/// the box and one over the organisms; the lab calls it once per census row
/// (every 10,000 frames).
pub fn nest_row(
    world: &World,
    spec: &LabBox,
    nest_cols: &[i32],
    colony_species: &str,
) -> (NestRow, [f32; CREATURE_TRAITS]) {
    let mut r = NestRow::default();
    let (xs, ys) = extents(world, spec);
    let (x0, y0) = (*xs.start(), *ys.start());
    let w = (xs.end() - xs.start() + 1).max(0) as usize;
    let h = (ys.end() - ys.start() + 1).max(0) as usize;
    let surface: Vec<i32> = xs.clone().map(|x| surface_of(world, spec, x)).collect();
    let near_nest = |x: i32| nest_cols.iter().any(|c| (c - x).abs() <= NEAR_NEST_COLS);

    // Ground, the deep trace's way: a powder or solid that is not food.
    let ground: Vec<bool> = (0..world.materials.len())
        .map(|i| {
            let m = MaterialId(i as u16);
            matches!(world.materials.kind(m), MaterialKind::Powder | MaterialKind::Solid)
                && food_value(world, Cell::new(m, 0)) <= 0.0
        })
        .collect();

    // --- open cells below the original ground --------------------------------
    let mut open = vec![false; w * h];
    for (col, x) in xs.clone().enumerate() {
        let s = surface[col];
        for y in (s + 1).max(y0)..=*ys.end() {
            let c = world.get(x, y);
            if c.organism_id() == 0 && ground[c.material.0 as usize] {
                continue;
            }
            open[(y - y0) as usize * w + col] = true;
            r.open_under += 1;
            r.deepest_row = r.deepest_row.max(y - s);
        }
    }

    // --- brood standing now -----------------------------------------------------
    let mut brood_cells: Vec<(usize, usize)> = Vec::new();
    let mut depths: Vec<i32> = Vec::new();
    for id in world.live_brood_ids() {
        let Some(st) = world.organism(id) else { continue };
        let Some(b) = st.brood else { continue };
        match b.stage {
            BroodStage::Egg => r.brood_eggs += 1,
            BroodStage::Larva => {
                r.brood_larvae += 1;
                r.brood_larvae_hungry += usize::from(st.energy < b.target);
            }
            BroodStage::Pupa => r.brood_pupae += 1,
        }
        // One cell, wherever it fell to; a brood item held out of the grid by
        // a walker still owns the cell it was in.
        let Some(&(x, y)) = st.cells.keys().next() else {
            continue;
        };
        if !xs.contains(&x) || !ys.contains(&y) {
            continue;
        }
        let col = (x - x0) as usize;
        if y > surface[col] {
            r.brood_under += 1;
            depths.push(y - surface[col]);
        }
        if nest_cols.iter().any(|c| (c - x).abs() <= DOOR_COLS) {
            r.brood_near_door += 1;
        }
        brood_cells.push((col, (y - y0) as usize));
    }
    depths.sort_unstable();
    r.brood_depth_med = depths.get(depths.len() / 2).copied().unwrap_or(0);

    // --- rooms -----------------------------------------------------------------------
    let rooms = rooms_of(&open, w, h, &brood_cells);
    r.room_big = rooms.big;
    r.room_big_brood = rooms.big_brood;
    r.rooms_30 = rooms.others.len();
    let at = |i: usize| rooms.others.get(i).copied().unwrap_or((0, 0, 0));
    (r.room2_cells, r.room2_steps, r.room2_brood) = at(0);
    (r.room3_cells, r.room3_steps, r.room3_brood) = at(1);
    (r.room4_cells, r.room4_steps, r.room4_brood) = at(2);

    // --- the colony's animals by place, and their traits ------------------------------
    let mut count = [0usize; 4];
    let mut hungry = [0usize; 4];
    let mut fill = [0f64; 4];
    let mut trait_sum = [0f64; CREATURE_TRAITS];
    let mut n = 0u32;
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        let sp = world.species.get(st.species);
        let Some(def) = sp.creature.as_ref() else { continue };
        if sp.name != colony_species {
            continue;
        }
        let Some(&(hx, hy)) = st.chain.first() else { continue };
        let place = place_index(hy, surface_of(world, spec, hx), near_nest(hx), || {
            (1..=COVER_ROWS).any(|k| {
                world.in_bounds(hx, hy - k) && {
                    let c = world.get(hx, hy - k);
                    c.material != crate::sim::material::EMPTY
                        && c.organism_id() == 0
                        && matches!(
                            world.materials.kind(c.material),
                            MaterialKind::Powder | MaterialKind::Solid
                        )
                }
            })
        });
        count[place] += 1;
        let start = f64::from(def.start_energy);
        hungry[place] += usize::from(f64::from(st.energy) < start);
        fill[place] += if start > 0.0 { f64::from(st.energy) / start } else { 0.0 };
        for (sum, t) in trait_sum.iter_mut().zip(st.traits.iter()) {
            *sum += f64::from(*t);
        }
        n += 1;
    }
    let mean = |p: usize| {
        if count[p] == 0 {
            0.0
        } else {
            (fill[p] / count[p] as f64) as f32
        }
    };
    (r.ants_under, r.ants_mound_in, r.ants_mound_top, r.ants_afield) = (count[0], count[1], count[2], count[3]);
    (r.hungry_under, r.hungry_mound_in, r.hungry_mound_top, r.hungry_afield) =
        (hungry[0], hungry[1], hungry[2], hungry[3]);
    (r.fill_under, r.fill_mound_in, r.fill_mound_top, r.fill_afield) = (mean(0), mean(1), mean(2), mean(3));
    let mut traits = [0f32; CREATURE_TRAITS];
    if n > 0 {
        for (t, sum) in traits.iter_mut().zip(trait_sum) {
            *t = (sum / f64::from(n)) as f32;
        }
    }

    // --- the colony's deaths, by cause group and place -------------------------------
    let mut deaths = [[0u64; DEATH_PLACES]; 4];
    for g in world
        .group_deaths
        .iter()
        .filter(|g| world.species.get(g.species).name == colony_species)
    {
        for (place, row) in g.by_place.iter().enumerate() {
            for (cause, n) in row.iter().enumerate().take(DEATH_CAUSES) {
                deaths[cause_group(cause)][place] += n;
            }
        }
    }
    (r.starved_under, r.starved_near, r.starved_afield) = (deaths[0][0], deaths[0][1], deaths[0][2]);
    (r.killed_under, r.killed_near, r.killed_afield) = (deaths[1][0], deaths[1][1], deaths[1][2]);
    (r.oldage_under, r.oldage_near, r.oldage_afield) = (deaths[2][0], deaths[2][1], deaths[2][2]);
    (r.other_under, r.other_near, r.other_afield) = (deaths[3][0], deaths[3][1], deaths[3][2]);

    // --- the engine's own books ------------------------------------------------------
    let s = &world.creature_stats;
    r.eggs_laid = s.eggs_laid;
    r.eggs_to_larvae = s.larvae;
    r.larvae_to_pupae = s.pupae;
    r.larvae_starved = s.larvae_starved;
    r.brood_lost = s.brood_lost;
    r.hatches_denied = s.hatches_denied;
    r.brood_held = s.brood_held;
    r.brood_carried = s.brood_carried;
    r.brood_spread = s.brood_spread;
    r.nurse_seeks = s.nurse_seeks;
    r.larva_ticks_hungry = s.larva_ticks_hungry;
    r.larva_ticks_nursed = s.larva_ticks_nursed;
    r.larva_ticks_crop_fed = s.larva_ticks_crop_fed;
    r.brood_crop_fed_j = s.brood_crop_fed_j;
    r.brood_shared_j = s.brood_shared_j;
    r.brood_nursed_j = s.brood_nursed_j;
    r.brood_ate_j = s.brood_ate_j;
    r.brood_upkeep_j = s.brood_upkeep_j;
    r.brood_corpse_j = s.brood_corpse_j;
    r.lays_declined = s.lays_declined;
    r.births_denied_no_space = s.births_denied_no_space;
    r.soil_way_pulls = s.soil_way_pulls;
    r.hungry_out_pulls = s.hungry_out_pulls;
    r.crop_down_holds = s.crop_down_holds;
    r.digs_refused_face = s.digs_refused_face;
    r.digs_faced = s.digs_faced;
    r.spoil_held_below = s.spoil_held_below;
    r.spoil_kept_inside = s.spoil_kept_inside;
    r.spoil_dumped = s.spoil_dumped;
    r.spoil_lifted = s.spoil_lifted;
    r.spoil_lifted_out = s.spoil_lifted_out;
    r.lean_dropped = s.lean_dropped;
    r.pickups = s.pickups;
    r.drops = s.drops;
    r.forage_trips = s.forage_trips;
    r.forage_returns = s.forage_returns;
    r.nest_visits = s.nest_visits;
    (r, traits)
}

/// The `trait_<name>` column names, one per `CREATURE_TRAITS` slot, in slot
/// order -- the parameters page's own names (`batch::trait_name`).
pub fn trait_columns() -> Vec<String> {
    (0..CREATURE_TRAITS)
        .map(|slot| format!("trait_{}", crate::lab::batch::trait_name(slot)))
        .collect()
}

/// **The brood, the rooms, the places and the traits as text lines** under a
/// CENSUS row, most important first. `births` is the row's hatchings. The
/// full set is in `census.csv`; this is the part a reader wants without a
/// parser, in the words the lanes use.
pub fn addendum(n: &NestRow, traits: &[f32; CREATURE_TRAITS], births: u64) -> String {
    use std::fmt::Write as _;
    let pct = |a: u64, b: u64| if b == 0 { 0.0 } else { 100.0 * a as f64 / b as f64 };
    let mut s = String::new();
    let _ = write!(
        s,
        "\n        brood now: {} eggs, {} larvae ({} hungry), {} pupae | under ground {}, median {} rows down, {} within {} columns of the door | so far: {} laid, {} to larvae, {} to pupae, {} hatched, {} larvae starved ({:.0}% of eggs laid), {} lost, {} hatches denied",
        n.brood_eggs, n.brood_larvae, n.brood_larvae_hungry, n.brood_pupae, n.brood_under, n.brood_depth_med, n.brood_near_door, DOOR_COLS,
        n.eggs_laid, n.eggs_to_larvae, n.larvae_to_pupae, births, n.larvae_starved, pct(n.larvae_starved, n.eggs_laid), n.brood_lost, n.hatches_denied
    );
    let _ = write!(
        s,
        "\n        brood fed (J): ate {:.0}, crop {:.0}, nursed {:.0}, shared {:.0}, upkeep burned {:.0}, starved to corpse {:.0} | larva ticks hungry {}, nursed {} ({:.0}%), crop fed {} | brood carried {}, spread {}, nurse seeks {} | lays declined {}, births denied for room {}",
        n.brood_ate_j, n.brood_crop_fed_j, n.brood_nursed_j, n.brood_shared_j, n.brood_upkeep_j, n.brood_corpse_j,
        n.larva_ticks_hungry, n.larva_ticks_nursed, pct(n.larva_ticks_nursed, n.larva_ticks_hungry), n.larva_ticks_crop_fed,
        n.brood_carried, n.brood_spread, n.nurse_seeks, n.lays_declined, n.births_denied_no_space
    );
    let _ = write!(
        s,
        "\n        rooms: {} open cells under ground, deepest {} rows | biggest room {} cells ({} brood in it) | other rooms of {}+ cells: {}",
        n.open_under, n.deepest_row, n.room_big, n.room_big_brood, ROOM_MIN, n.rooms_30
    );
    for (cells, steps, brood) in [
        (n.room2_cells, n.room2_steps, n.room2_brood),
        (n.room3_cells, n.room3_steps, n.room3_brood),
        (n.room4_cells, n.room4_steps, n.room4_brood),
    ] {
        if cells > 0 {
            let _ = write!(
                s,
                " | {cells} cells, {} from the biggest, {brood} brood",
                if steps >= ROOM_UNREACHED {
                    "not joined within 60 steps".to_string()
                } else {
                    format!("{steps} steps")
                }
            );
        }
    }
    let _ = write!(
        s,
        "\n        the colony by place (under / mound inside / mound top / afield): {} / {} / {} / {} animals, {} / {} / {} / {} under their start energy, mean fill {:.2} / {:.2} / {:.2} / {:.2}",
        n.ants_under, n.ants_mound_in, n.ants_mound_top, n.ants_afield,
        n.hungry_under, n.hungry_mound_in, n.hungry_mound_top, n.hungry_afield,
        n.fill_under, n.fill_mound_in, n.fill_mound_top, n.fill_afield
    );
    let _ = write!(
        s,
        "\n        the colony's deaths by place (under / near the nest / afield): starved {} / {} / {} | killed {} / {} / {} | old age {} / {} / {} | other {} / {} / {}",
        n.starved_under, n.starved_near, n.starved_afield, n.killed_under, n.killed_near, n.killed_afield,
        n.oldage_under, n.oldage_near, n.oldage_afield, n.other_under, n.other_near, n.other_afield
    );
    let _ = write!(
        s,
        "\n        nest plan fired: soil way pulls {}, hungry out pulls {}, crop down holds {}, digs refused away from the digger's face {}, digs turned to a face {}, spoil kept inside {} / held below {} / dumped {} / lifted {} (out {}), lean dropped {} | foraging: {} trips, {} returns, {} pickups, {} drops, {} nest visits",
        n.soil_way_pulls, n.hungry_out_pulls, n.crop_down_holds, n.digs_refused_face, n.digs_faced, n.spoil_kept_inside, n.spoil_held_below, n.spoil_dumped, n.spoil_lifted, n.spoil_lifted_out,
        n.lean_dropped, n.forage_trips, n.forage_returns, n.pickups, n.drops, n.nest_visits
    );
    // The six traits the evolved founder sets; all of them are in census.csv.
    let six = [
        crate::sim::organism::TRAIT_GUT_BIAS,
        crate::sim::organism::TRAIT_BIRTH_GRANT,
        crate::sim::organism::TRAIT_REPRODUCE_AT,
        crate::sim::organism::TRAIT_PACE,
        crate::sim::organism::TRAIT_CURVATURE_RADIUS,
        crate::sim::organism::TRAIT_DIGEST_RATE,
    ];
    let _ = write!(s, "\n        traits (mean over the colony's animals):");
    for slot in six {
        let _ = write!(s, " {} {:+.2},", crate::lab::batch::trait_name(slot), traits[slot]);
    }
    s.pop();
    let _ = write!(s, " | all {CREATURE_TRAITS} in census.csv");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `w` x `h` grid, closed, with each inclusive rectangle `(x0, y0, x1,
    /// y1)` opened. The reference scenarios below are the same rectangles run
    /// through `scripts/deeptrace_dig.py`'s `room_census` (2026-10-05), whose
    /// answers are the numbers asserted: this module is a port, and a port
    /// is only as good as the agreement it is held to. (A wider check ran at
    /// the same time: 700 random and tunnelled nests, Python and Rust
    /// identical on every one, 325 rooms with a finite step count from 2 to
    /// the cap.)
    fn carved(w: usize, h: usize, rects: &[(usize, usize, usize, usize)]) -> Vec<bool> {
        let mut open = vec![false; w * h];
        for &(x0, y0, x1, y1) in rects {
            for y in y0..=y1 {
                for x in x0..=x1 {
                    open[y * w + x] = true;
                }
            }
        }
        open
    }

    /// **Two chambers joined by a tunnel**: the 10x10 chamber has 64 interior
    /// room cells plus the six at the tunnel's mouth (a mouth cell has seven
    /// of its nine open), the 8x8 one 36 plus six; the tunnel itself, two
    /// wide, has none. They are 12 steps apart, and the brood standing in the
    /// tunnel is in neither room.
    #[test]
    fn two_chambers_joined_by_a_tunnel() {
        let open = carved(40, 20, &[(2, 3, 11, 12), (12, 7, 24, 8), (25, 4, 32, 11)]);
        let brood = [(5, 6), (6, 6), (8, 9), (28, 6), (29, 8), (18, 7)];
        let r = rooms_of(&open, 40, 20, &brood);
        assert_eq!((r.big, r.big_brood), (70, 3), "{r:?}");
        assert_eq!(r.others, vec![(42, 12, 2)], "{r:?}");
    }

    /// **Walled apart is not joined at all**: the same two chambers with no
    /// tunnel. `ROOM_UNREACHED`, not a long distance -- there is no path.
    #[test]
    fn chambers_with_no_tunnel_are_not_joined() {
        let open = carved(40, 20, &[(2, 3, 11, 12), (25, 4, 32, 11)]);
        let r = rooms_of(&open, 40, 20, &[(5, 6), (28, 6)]);
        assert_eq!((r.big, r.big_brood), (64, 1), "{r:?}");
        assert_eq!(r.others, vec![(36, ROOM_UNREACHED, 1)], "{r:?}");
    }

    /// **A wide opening makes one room, not two** -- the positive control for
    /// the other direction: a census that called a corridor-less hole two
    /// chambers would pass the test above and fail this one.
    #[test]
    fn a_hole_with_no_waist_is_one_room() {
        let open = carved(40, 20, &[(2, 3, 11, 12), (12, 3, 22, 12), (23, 4, 32, 11)]);
        let r = rooms_of(&open, 40, 20, &[]);
        assert_eq!((r.big, r.others.len()), (216, 0), "{r:?}");
    }

    /// **A tunnel longer than the cap is joined and still reads 999**: the
    /// walk from the biggest room stops at [`ROOM_STEPS_CAP`] steps, so a
    /// room 70 columns down a tunnel is "further than 60", the same as one
    /// with no path. (The Python reader does the same; so does the trace's
    /// `rooms` line.)
    #[test]
    fn a_room_beyond_the_cap_reads_unreached() {
        let open = carved(120, 20, &[(2, 3, 11, 12), (12, 7, 80, 8), (81, 4, 90, 11)]);
        let r = rooms_of(&open, 120, 20, &[(85, 6)]);
        assert_eq!((r.big, r.big_brood), (70, 0), "{r:?}");
        assert_eq!(r.others, vec![(54, ROOM_UNREACHED, 1)], "{r:?}");
    }

    /// **A two-wide tunnel alone is no room**, and an empty grid is none
    /// either: scratches are not chambers.
    #[test]
    fn a_scratch_is_no_room() {
        let open = carved(20, 8, &[(2, 3, 17, 4)]);
        assert_eq!(rooms_of(&open, 20, 8, &[]), RoomCensus::default());
        assert_eq!(
            rooms_of(&[], 0, 0, &[]),
            RoomCensus::default(),
            "an empty box is not a panic"
        );
    }

    /// The four places, in the deep trace's words.
    #[test]
    fn places_are_under_inside_the_mound_on_top_of_it_or_afield() {
        assert_eq!(
            place_index(171, 170, true, || false),
            0,
            "strictly below the datum row is under"
        );
        assert_eq!(place_index(170, 170, true, || false), 2, "the doorway row is not under");
        assert_eq!(
            place_index(165, 170, true, || true),
            1,
            "ground over the head, near the nest: inside the mound"
        );
        assert_eq!(
            place_index(165, 170, true, || false),
            2,
            "open sky, near the nest: top of the mound"
        );
        assert_eq!(
            place_index(165, 170, false, || panic!(
                "far from any nest the cover test is never asked"
            )),
            3
        );
    }

    // ---- the census over a real world -------------------------------------

    use crate::sim::cell::Cell;
    use crate::sim::organism::{pack_cell_type, Brood, CellType, SpeciesId, TRAIT_GUT_BIAS, TRAIT_PACE};

    /// A bed with no founders, nobody planted and the world never stepped:
    /// the datum falls back to `spec.ground_y`, so the tests' rows are
    /// `ground_y` plus an offset. **The default height stays**: the world is
    /// `height` rows tall and the soil starts at `ground_y` (160), so a
    /// shorter box has no ground in it at all and every row a test digs
    /// falls outside the world (the first draft of these tests did exactly
    /// that, and read `open_under` 0 for it).
    fn bed() -> (LabBox, World) {
        let spec = LabBox {
            founders: 0,
            colonies: 0,
            width: 128,
            ..LabBox::default()
        };
        let world = spec.build();
        (spec, world)
    }

    /// Open the cells of each inclusive rectangle `(x0, y0, x1, y1)`, with
    /// rows given below the ground line.
    fn dig(world: &mut World, spec: &LabBox, rects: &[(i32, i32, i32, i32)]) -> usize {
        let mut n = 0;
        for &(x0, y0, x1, y1) in rects {
            for y in y0..=y1 {
                for x in x0..=x1 {
                    world.set(x, spec.ground_y + y, Cell::EMPTY);
                    n += 1;
                }
            }
        }
        n
    }

    /// Fill the cells of each inclusive rectangle back in with soil -- the
    /// control for [`dig`], rows given below the ground line.
    fn fill(world: &mut World, spec: &LabBox, rects: &[(i32, i32, i32, i32)]) {
        let soil = world.materials.id_of("soil").expect("soil is a compiled-in material");
        for &(x0, y0, x1, y1) in rects {
            for y in y0..=y1 {
                for x in x0..=x1 {
                    world.set(x, spec.ground_y + y, Cell::new(soil, 0));
                }
            }
        }
    }

    /// One brood item, laid the way `brood::lay_egg` does (an organism of the
    /// laying species, one cell of the brood material), at `(x, y)`.
    fn lay(world: &mut World, species: SpeciesId, (x, y): (i32, i32), stage: BroodStage, energy: f32, target: f32) {
        let material = world.materials.id_of("brood").expect("the brood material ships");
        let id = world.push_organism(species).expect("an organism slot");
        let st = world.organism_mut(id).expect("just made");
        st.energy = energy;
        st.brood = Some(Brood {
            stage,
            since: 0,
            target,
            parent: 0,
            last_tick: 0,
        });
        world.set(
            x,
            y,
            Cell::new(material, stage as u8)
                .with_organism_id(id)
                .with_aux(pack_cell_type(CellType::Seed)),
        );
    }

    /// **Rooms and brood, read off a carved bed**: the rectangles of
    /// `two_chambers_joined_by_a_tunnel`, dug into soil, with brood laid in
    /// them. Everything the Python reader and the pure test above agree on
    /// (70 room cells with 3 brood, one other room of 42 cells 12 steps
    /// away with 2) must come out of the world read, so the glue between the
    /// grid and the census -- the datum, the ground test, the brood's cell --
    /// is held to it too. Also the standing-brood columns: stages, the
    /// hungry larva, depth and the door lane.
    #[test]
    fn the_census_reads_rooms_and_brood_off_a_carved_bed() {
        let (spec, mut world) = bed();
        let g = spec.ground_y;
        let dug = dig(
            &mut world,
            &spec,
            &[(12, 13, 21, 22), (22, 17, 34, 18), (35, 14, 42, 21)],
        );
        let ant = world.species.id_of("ant").expect("the lab ships an ant");
        lay(&mut world, ant, (15, g + 16), BroodStage::Egg, 5.0, 100.0);
        lay(&mut world, ant, (16, g + 16), BroodStage::Larva, 5.0, 100.0); // hungry: 5 of 100
        lay(&mut world, ant, (18, g + 19), BroodStage::Larva, 150.0, 100.0); // fed
        lay(&mut world, ant, (38, g + 16), BroodStage::Pupa, 5.0, 100.0);
        lay(&mut world, ant, (39, g + 18), BroodStage::Egg, 5.0, 100.0);
        lay(&mut world, ant, (28, g + 17), BroodStage::Egg, 5.0, 100.0); // in the tunnel
        lay(&mut world, ant, (14, g - 3), BroodStage::Egg, 5.0, 100.0); // above ground, at the door
        let (n, _) = nest_row(&world, &spec, &[14], "ant");
        assert_eq!(
            n.open_under, dug,
            "every carved cell is open under ground, and a brood cell laid in one is still open: {n:?}"
        );
        assert_eq!(n.deepest_row, 22);
        assert_eq!(
            (n.brood_eggs, n.brood_larvae, n.brood_larvae_hungry, n.brood_pupae),
            (4, 2, 1, 1),
            "{n:?}"
        );
        assert_eq!(n.brood_under, 6, "the egg laid above ground is not under it: {n:?}");
        assert_eq!(
            n.brood_near_door, 4,
            "the egg above the door (column 14), the egg at 15 and the larvae at 16 and 18 are within four columns of the door at 14, and the three further out are not: {n:?}"
        );
        assert_eq!(n.brood_depth_med, 17, "rows below the ground line of the six under it, sorted: 16 16 16 17 18 19; the one at index 3 (the upper median) is 17: {n:?}");
        assert_eq!((n.room_big, n.room_big_brood), (70, 3), "{n:?}");
        assert_eq!(
            (n.rooms_30, n.room2_cells, n.room2_steps, n.room2_brood),
            (1, 42, 12, 2),
            "{n:?}"
        );
        assert_eq!(n.room3_cells, 0);
        // Positive control for the world read: fill the tunnel back in and
        // the second room is a room apart, not one joined to the first.
        let mut walled = world;
        fill(&mut walled, &spec, &[(22, 17, 34, 18)]);
        let (cut, _) = nest_row(&walled, &spec, &[14], "ant");
        assert_eq!(
            (cut.room_big, cut.rooms_30, cut.room2_cells, cut.room2_steps),
            (64, 1, 36, ROOM_UNREACHED),
            "a filled tunnel leaves two rooms apart: {cut:?}"
        );
    }

    /// **Hungry animals are counted where they stand, and traits are the
    /// colony's mean.** Six ants founded at column 60 are each put at a head
    /// of the test's choosing -- the founding shaft stands one of them under
    /// the ground on its own, so what the founding does is not the thing
    /// under test -- two under the ground, two on the open top of the mound,
    /// one with a roof of soil over it, one far from the nest. Every place
    /// gets a different count, hungry count and mean fill, so a pair of
    /// swapped places cannot pass, and one ant stands at exactly its start
    /// energy: *hungry* is strictly under it, as `hungry_out_pull` reads it.
    #[test]
    fn hungry_ants_are_counted_by_place_and_traits_averaged() {
        let (spec, mut world) = bed();
        let g = spec.ground_y;
        let placed = world.found_colony_of(60, g - 2, "ant", 6);
        let ids: Vec<_> = world
            .live_organism_ids()
            .into_iter()
            .filter(|id| world.organism(*id).is_some_and(|s| !s.chain.is_empty()))
            .collect();
        assert_eq!(
            (placed, ids.len()),
            (6, 6),
            "the founding placed {placed} and {} animals have a head",
            ids.len()
        );
        let start = world
            .species
            .get(world.organism(ids[0]).unwrap().species)
            .creature
            .as_ref()
            .unwrap()
            .start_energy;
        // Where each ant's head is, and its energy in start energies.
        let plan: [((i32, i32), f32); 6] = [
            ((60, g + 10), 0.5), // under, hungry
            ((62, g + 12), 1.1), // under, fed
            ((64, g - 3), 0.25), // top of the mound, hungry
            ((66, g - 3), 0.75), // top of the mound, hungry
            ((70, g - 3), 1.0),  // inside the mound, exactly at its start energy: not hungry
            ((115, g - 3), 0.9), // 55 columns from the nest: afield, hungry
        ];
        // The roof over the fifth: ground within thirty rows above its head.
        let soil = world.materials.id_of("soil").expect("soil");
        world.set(70, g - 8, Cell::new(soil, 0));
        for (i, (id, (head, in_starts))) in ids.iter().zip(plan).enumerate() {
            let st = world.organism_mut(*id).unwrap();
            st.chain[0] = head;
            st.energy = start * in_starts;
            st.traits[TRAIT_PACE] = 0.5;
            st.traits[TRAIT_GUT_BIAS] = if i % 2 == 0 { -0.8 } else { 0.0 };
        }
        let (n, traits) = nest_row(&world, &spec, &[60], "ant");
        assert_eq!(
            (n.ants_under, n.ants_mound_in, n.ants_mound_top, n.ants_afield),
            (2, 1, 2, 1),
            "{n:?}"
        );
        assert_eq!(
            (n.hungry_under, n.hungry_mound_in, n.hungry_mound_top, n.hungry_afield),
            (1, 0, 2, 1),
            "{n:?}"
        );
        for (got, want, place) in [
            (n.fill_under, 0.8, "under"),
            (n.fill_mound_in, 1.0, "mound inside"),
            (n.fill_mound_top, 0.5, "mound top"),
            (n.fill_afield, 0.9, "afield"),
        ] {
            assert!((got - want).abs() < 1e-4, "mean fill {place}: {got} against {want}");
        }
        assert!((traits[TRAIT_PACE] - 0.5).abs() < 1e-6, "{}", traits[TRAIT_PACE]);
        assert!(
            (traits[TRAIT_GUT_BIAS] - (-0.4)).abs() < 1e-5,
            "three of six carry -0.8: {}",
            traits[TRAIT_GUT_BIAS]
        );
        // Another species' animals are not the colony: three beetles stood
        // beside it change none of the above, and are the "beetle" row's own.
        let beetles = world.found_colony_of(100, g - 2, "beetle", 3);
        assert!(beetles > 0, "no beetles to leave out of the count");
        let (with_beetles, _) = nest_row(&world, &spec, &[60], "ant");
        assert_eq!(
            (
                with_beetles.ants_under,
                with_beetles.ants_mound_in,
                with_beetles.ants_mound_top,
                with_beetles.ants_afield
            ),
            (2, 1, 2, 1),
            "beetles are not the ants' colony: {with_beetles:?}"
        );
        let (b, _) = nest_row(&world, &spec, &[100], "beetle");
        assert_eq!(
            b.ants_under + b.ants_mound_in + b.ants_mound_top + b.ants_afield,
            beetles,
            "{b:?}"
        );
        // Far from every nest the same ants are afield, the ones under the
        // ground still under it.
        let (far, _) = nest_row(&world, &spec, &[], "ant");
        assert_eq!(
            (far.ants_under, far.ants_mound_in, far.ants_mound_top, far.ants_afield),
            (2, 0, 0, 4),
            "{far:?}"
        );
    }

    /// **A death is tallied where the head was**, by cause group: a starved
    /// ant above ground near a nest, a starved one under the ground, an old
    /// one far from any nest. The control for the read: the same deaths are
    /// all in `by_cause`, so the place split must sum to it.
    #[test]
    fn deaths_are_tallied_by_cause_and_place() {
        let (spec, mut world) = bed();
        world.register_nest_site(60, spec.ground_y, 4);
        world.found_colony_of(60, spec.ground_y - 2, "ant", 8);
        let ids: Vec<_> = world
            .live_organism_ids()
            .into_iter()
            .filter(|id| world.organism(*id).is_some_and(|s| !s.chain.is_empty()))
            .collect();
        assert!(ids.len() >= 3);
        let kill = |world: &mut World, id, cause: DeathCause, head: Option<(i32, i32)>| {
            let st = world.organism_mut(id).unwrap();
            st.senescence_cause = cause;
            if let Some(h) = head {
                st.chain[0] = h;
            }
            world.free_organism(id);
        };
        kill(&mut world, ids[0], DeathCause::Starved, None); // above ground, beside the nest
        kill(&mut world, ids[1], DeathCause::Starved, Some((60, spec.ground_y + 12))); // under it
        kill(&mut world, ids[2], DeathCause::OldAge, Some((120, spec.ground_y - 2))); // far from the nest
        let (n, _) = nest_row(&world, &spec, &[60], "ant");
        assert_eq!((n.starved_under, n.starved_near, n.starved_afield), (1, 1, 0), "{n:?}");
        assert_eq!((n.oldage_under, n.oldage_near, n.oldage_afield), (0, 0, 1), "{n:?}");
        assert_eq!(
            n.killed_under + n.killed_near + n.killed_afield + n.other_under + n.other_near + n.other_afield,
            0
        );
        let (starved, killed, oldage, other) = crate::lab::census::colony_deaths(&world, "ant");
        assert_eq!((starved, killed, oldage, other), (2, 0, 1, 0));
        let placed: u64 = world
            .group_deaths
            .iter()
            .map(|g| g.by_place.iter().flatten().sum::<u64>())
            .sum();
        assert_eq!(
            placed, 3,
            "every death with a head to place is in by_place as well as by_cause"
        );
    }

    #[test]
    fn causes_fall_into_the_four_groups() {
        assert_eq!(cause_group(DeathCause::Starved.index()), 0);
        assert_eq!(cause_group(DeathCause::StarvedInFlight.index()), 0);
        assert_eq!(cause_group(DeathCause::Killed.index()), 1);
        assert_eq!(cause_group(DeathCause::OldAge.index()), 2);
        assert_eq!(cause_group(DeathCause::Unknown.index()), 3);
    }
}
