//! **The owner's nest goal, as one readout: does the colony hold steady, and
//! does it dig separate chambers that keep its food, brood and workers?**
//!
//! The owner, 2026-10-03: a lab colony whose population is stable (or at
//! least not dying) and that over time digs a nest of several actually
//! separate chambers, big enough to hold all the food, brood and workers,
//! with food and brood somewhat organised. No plants; unlimited food in one
//! spot beside the colony, placed with the player's own FOOD brush.
//!
//! Runs the `nest_goal` scenario (`assets/lab_scenarios/nest_goal.ron`: the
//! played bed's box with no plants, the colony landing at frame 6,000) and
//! **keeps its food heap topped up**: every [`TOP_EVERY`] frames, if fewer
//! than `food=` cells of `provisions` stand within [`FOOD_REACH`] columns of
//! the food spot, it drops more onto the surface there. Provisions never
//! rot, so the only way a cell leaves is being eaten or carried.
//!
//! ```text
//! cargo run --release --example nestgoal -- seed=1 frames=300000 every=25000 shots=/tmp/goal
//! ```
//!
//! **Every `every=` frames it prints three lines:**
//!
//! - `POP`: live ants, births and deaths so far (the engine's own counters),
//!   and the food ever dropped by the top-up.
//! - `NEST`: the dug space. *Open* is every non-ground, non-liquid cell
//!   below the old ground line (air, an ant, brood, food, crumbs) that the
//!   colony dug (`World::dug_cells`) or that joins one; *regions* are its
//!   4-connected pieces. A **chamber** is a 4-connected piece of *room
//!   cells* -- open cells with at least [`ROOM_MIN`] of the 9 cells of their
//!   3x3 block open, so a one- or two-cell passage is never a chamber and
//!   two rooms joined only by a passage are two chambers -- of at least
//!   [`CHAMBER_MIN`] cells.
//! - `CHAMBERS`: each chamber's size and what stands in it and its rim
//!   (food cells, brood cells, ant cells), and a **separation score**:
//!   `1 - sum_i min(food_i / food, brood_i / brood)` over chambers, 1 when
//!   no chamber holds both, 0 when food and brood are spread identically.
//!   `n/a` until both are stored in chambers.
//! - `TUNNELS`: how far the open space runs out of the largest chamber, in
//!   steps along the shortest way ([`tunnels`]), and how far out each other
//!   chamber starts.
//!
//! **Positive control** (`control=selftest`): a hand-dug pair of rooms
//! joined by a one-cell passage, one holding food and one brood, must read
//! as two chambers with separation 1.0; filling both with both must read 0.
//! A census that cannot tell those apart is not measuring the goal.

use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::material::{self, MaterialId, MaterialKind};
use pixel_physics::sim::world::World;
use std::collections::{HashSet, VecDeque};

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

/// How often the food heap is topped up, in frames.
const TOP_EVERY: u64 = 250;
/// Columns either side of the food spot counted as the heap.
const FOOD_REACH: i32 = 12;
/// Open cells of a 3x3 block (centre included) for its centre to be a room cell.
const ROOM_MIN: usize = 7;
/// Room cells for a piece to count as a chamber.
const CHAMBER_MIN: usize = 6;

#[derive(Clone, Copy, PartialEq, Eq)]
enum What {
    Ground,
    Liquid,
    Empty,
    Ant,
    Brood,
    Food,
}

struct Census {
    ground_y: i32,
    brood: Option<MaterialId>,
}

impl Census {
    fn what(&self, w: &World, x: i32, y: i32) -> What {
        if !w.in_bounds(x, y) {
            return What::Ground;
        }
        let c = w.get(x, y);
        if c.material == material::EMPTY {
            return What::Empty;
        }
        // Brood first: an egg, larva or pupa carries its own organism id,
        // which is a creature's, so the animal test below would take it.
        if Some(c.material) == self.brood {
            return What::Brood;
        }
        let id = c.organism_id();
        if id != 0 && w.organism(id).is_some_and(|s| w.species.get(s.species).creature.is_some()) {
            return What::Ant;
        }
        if w.materials.kind(c.material) == MaterialKind::Liquid {
            return What::Liquid;
        }
        if id == 0 && w.materials.get(c.material).food_energy > 0.0 {
            return What::Food;
        }
        What::Ground
    }

    fn open(what: What) -> bool {
        matches!(what, What::Empty | What::Ant | What::Brood | What::Food)
    }

    /// The colony's open space below the old ground line: every open cell
    /// 4-connected to a cell the colony dug.
    fn open_space(&self, w: &World) -> HashSet<(i32, i32)> {
        let mut seen = HashSet::new();
        let mut q = VecDeque::new();
        for &(x, y) in w.dug_cells.iter() {
            if y > self.ground_y && Self::open(self.what(w, x, y)) && seen.insert((x, y)) {
                q.push_back((x, y));
            }
        }
        while let Some((x, y)) = q.pop_front() {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let n = (x + dx, y + dy);
                if n.1 > self.ground_y && !seen.contains(&n) && Self::open(self.what(w, n.0, n.1)) {
                    seen.insert(n);
                    q.push_back(n);
                }
            }
        }
        seen
    }
}

fn components(cells: &HashSet<(i32, i32)>) -> Vec<Vec<(i32, i32)>> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let mut sorted: Vec<_> = cells.iter().copied().collect();
    sorted.sort();
    for start in sorted {
        if !seen.insert(start) {
            continue;
        }
        let mut comp = vec![start];
        let mut q = VecDeque::from([start]);
        while let Some((x, y)) = q.pop_front() {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let n = (x + dx, y + dy);
                if cells.contains(&n) && seen.insert(n) {
                    comp.push(n);
                    q.push_back(n);
                }
            }
        }
        out.push(comp);
    }
    out.sort_by_key(|c| std::cmp::Reverse(c.len()));
    out
}

struct Chamber {
    /// Its room cells.
    room: Vec<(i32, i32)>,
    cells: usize,
    food: usize,
    brood: usize,
    ants: usize,
    centre: (i32, i32),
}

struct Nest {
    open: usize,
    regions: usize,
    largest: usize,
    chambers: Vec<Chamber>,
    food_open: usize,
    brood_open: usize,
}

fn nest(census: &Census, w: &World) -> Nest {
    let open = census.open_space(w);
    let regions = components(&open);
    let room: HashSet<(i32, i32)> = open
        .iter()
        .copied()
        .filter(|&(x, y)| {
            let mut n = 0;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    n += usize::from(open.contains(&(x + dx, y + dy)));
                }
            }
            n >= ROOM_MIN
        })
        .collect();
    let mut chambers = Vec::new();
    for comp in components(&room).into_iter().filter(|c| c.len() >= CHAMBER_MIN) {
        // The chamber and its rim: every open cell touching a room cell.
        let mut rim: HashSet<(i32, i32)> = HashSet::new();
        for &(x, y) in &comp {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if open.contains(&(x + dx, y + dy)) {
                        rim.insert((x + dx, y + dy));
                    }
                }
            }
        }
        let count = |k: What| rim.iter().filter(|&&(x, y)| census.what(w, x, y) == k).count();
        let n = comp.len() as i32;
        let centre = (comp.iter().map(|c| c.0).sum::<i32>() / n, comp.iter().map(|c| c.1).sum::<i32>() / n);
        chambers.push(Chamber { room: comp.clone(), cells: comp.len(), food: count(What::Food), brood: count(What::Brood), ants: count(What::Ant), centre });
    }
    let food_open = open.iter().filter(|&&(x, y)| census.what(w, x, y) == What::Food).count();
    let brood_open = open.iter().filter(|&&(x, y)| census.what(w, x, y) == What::Brood).count();
    Nest { open: open.len(), regions: regions.len(), largest: regions.first().map_or(0, |c| c.len()), chambers, food_open, brood_open }
}

/// **How far the tunnels run out of the main room** -- the `TUNNELS` line.
/// Steps through open cells (4-connected, the shortest way) from the
/// largest chamber's room cells: the farthest open cell, how many open cells
/// lie [`FAR`] or more steps out, how many of those are tips (no neighbour
/// farther out), and how far out each other chamber starts. A chamber
/// [`FAR`] or more out is one a tunnel reached before opening into it.
const FAR: u32 = 10;

struct Tunnels {
    reach: u32,
    far_cells: usize,
    far_tips: usize,
    /// (cells, steps out) for every chamber but the main one.
    out: Vec<(usize, u32)>,
}

fn tunnels(census: &Census, w: &World, n: &Nest) -> Option<Tunnels> {
    let main = n.chambers.first()?;
    let open = census.open_space(w);
    let mut d: std::collections::HashMap<(i32, i32), u32> = main.room.iter().map(|&c| (c, 0)).collect();
    let mut q: VecDeque<(i32, i32)> = main.room.iter().copied().collect();
    while let Some((x, y)) = q.pop_front() {
        let k = d[&(x, y)];
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let c = (x + dx, y + dy);
            if open.contains(&c) && !d.contains_key(&c) {
                d.insert(c, k + 1);
                q.push_back(c);
            }
        }
    }
    let far: Vec<_> = d.iter().filter(|&(_, &k)| k >= FAR).collect();
    let far_tips = far
        .iter()
        .filter(|&&(&(x, y), &k)| [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().all(|&(dx, dy)| d.get(&(x + dx, y + dy)).is_none_or(|&j| j <= k)))
        .count();
    let out = n.chambers[1..].iter().map(|c| (c.cells, c.room.iter().filter_map(|p| d.get(p).copied()).min().unwrap_or(u32::MAX))).collect();
    Some(Tunnels { reach: d.values().copied().max().unwrap_or(0), far_cells: far.len(), far_tips, out })
}

/// `1 - sum_i min(f_i/F, b_i/B)`; `None` until chambers hold both.
fn separation(chambers: &[Chamber]) -> Option<f32> {
    let f: usize = chambers.iter().map(|c| c.food).sum();
    let b: usize = chambers.iter().map(|c| c.brood).sum();
    if f == 0 || b == 0 {
        return None;
    }
    let overlap: f32 = chambers.iter().map(|c| (c.food as f32 / f as f32).min(c.brood as f32 / b as f32)).sum();
    Some(1.0 - overlap)
}

/// **Where the food goes** -- the `FLOW` line. Food lying underground near the
/// nest (loose food, 80 columns either side of the nest and 80 rows down) is
/// counted every frame; a rise is food arriving, a fall is food leaving
/// (eaten, or carried out). Arrivals and departures in the same frame cancel,
/// so both are lower bounds. Beside them, the engine's own counters over the
/// interval: bites taken anywhere, crop drops and the share counted home,
/// nest-worker store pick-ups, and harvest drops.
#[derive(Default)]
struct Flow {
    last: Option<usize>,
    arrived: u64,
    left: u64,
    prev: [u64; 6],
    /// Food cells underground last frame, to tell where new ones appear.
    cells: HashSet<(i32, i32)>,
    /// New food cells by what lay within two cells of them the frame they
    /// appeared: [beside brood only, beside food only, beside both, neither].
    site: [u64; 4],
    /// Food cells gone from underground by what stands there the frame after:
    /// [empty, an organism (ant or brood), liquid, other].
    gone: [u64; 4],
}

impl Flow {
    fn step(&mut self, census: &Census, w: &World, nest_x: i32) {
        let mut now = HashSet::new();
        for y in census.ground_y + 1..census.ground_y + 80 {
            for x in nest_x - 80..=nest_x + 80 {
                if census.what(w, x, y) == What::Food {
                    now.insert((x, y));
                }
            }
        }
        if self.last.is_some() {
            for &(x, y) in now.difference(&self.cells) {
                let (mut brood, mut food) = (false, false);
                for dy in -2..=2 {
                    for dx in -2..=2 {
                        if (dx, dy) == (0, 0) {
                            continue;
                        }
                        match census.what(w, x + dx, y + dy) {
                            What::Brood => brood = true,
                            What::Food if self.cells.contains(&(x + dx, y + dy)) => food = true,
                            _ => {}
                        }
                    }
                }
                self.site[match (brood, food) {
                    (true, false) => 0,
                    (false, true) => 1,
                    (true, true) => 2,
                    (false, false) => 3,
                }] += 1;
            }
        }
        for &(x, y) in self.cells.difference(&now) {
            let c = w.get(x, y);
            let k = if c.material == material::EMPTY {
                0
            } else if c.organism_id() != 0 {
                1
            } else if w.materials.kind(c.material) == MaterialKind::Liquid {
                2
            } else {
                3
            };
            self.gone[k] += 1;
        }
        let n = now.len();
        self.cells = now;
        if let Some(l) = self.last {
            if n > l {
                self.arrived += (n - l) as u64;
            } else {
                self.left += (l - n) as u64;
            }
        }
        self.last = Some(n);
    }

    fn report(&mut self, frame: u64, w: &World) {
        let s = &w.creature_stats;
        let now = [s.eats, s.drops, s.deliveries, s.store_pickups, s.store_delivered, s.harvest_stored];
        let d: Vec<u64> = now.iter().zip(self.prev).map(|(a, b)| a - b).collect();
        println!(
            "FLOW frame={frame} underground food now {} | arrived {} left {} | bites {} | crop drops {} (home {}) | store pickups {} delivered {} | harvest stored {}",
            self.last.unwrap_or(0), self.arrived, self.left, d[0], d[1], d[2], d[3], d[4], d[5]
        );
        println!(
            "SITES frame={frame} new food cells underground beside: brood only {} | food only {} | both {} | neither {} | sort holds (total) {} | unload ticks (total) {} | store scaled (total) {} | handle held (total) {}",
            self.site[0], self.site[1], self.site[2], self.site[3], s.food_sort_held, s.crop_unload_ticks, s.store_chamber_scaled, s.food_handle_held
        );
        self.prev = now;
        self.arrived = 0;
        self.left = 0;
        println!(
            "GONE frame={frame} underground food cells gone, now holding: empty {} | organism {} | liquid {} | other {}",
            self.gone[0], self.gone[1], self.gone[2], self.gone[3]
        );
        self.site = [0; 4];
        self.gone = [0; 4];
    }
}

fn report(frame: u64, census: &Census, w: &World, dropped: usize, food_x: i32) {
    let s = &w.creature_stats;
    // An egg, larva or pupa is an organism too; it is counted as brood, not as an ant.
    let (mut ants, mut brood) = (0, 0);
    let (mut bins, mut body_j, mut crop_j) = ([0u32; 5], 0f64, 0f64);
    for id in w.live_organism_ids() {
        let Some(st) = w.organism(id) else { continue };
        if w.species.get(st.species).creature.is_none() {
            continue;
        }
        let is_brood = st.chain.first().is_some_and(|&(x, y)| Some(w.get(x, y).material) == census.brood);
        if is_brood {
            brood += 1;
        } else {
            ants += 1;
            let e = st.energy;
            bins[match e {
                e if e < 100.0 => 0,
                e if e < 200.0 => 1,
                e if e < 500.0 => 2,
                e if e < 1100.0 => 3,
                _ => 4,
            }] += 1;
            body_j += f64::from(e);
            crop_j += st.crop.map_or(0.0, |c| f64::from(c.worth()));
        }
    }
    println!(
        "ENERGY frame={frame} ants by energy (J): <100 {} | 100-200 {} | 200-500 {} | 500-1100 {} | >=1100 {} | in bodies {:.0} J, in crops {:.0} J",
        bins[0], bins[1], bins[2], bins[3], bins[4], body_j, crop_j
    );
    let heap = heap_count(census, w, food_x);
    println!("POP frame={frame} live ants {ants} | brood {brood} | births {} deaths {} | food heap {heap} cells, ever dropped {dropped} | digs {} refused roof {} flat {} brood-drawn {} | spoil dumped {} lean dropped {}", s.births, s.deaths, s.digs, s.digs_refused_roof, s.digs_refused_flat, s.digs_brood_drawn, s.spoil_dumped, s.lean_dropped);
    // Why they died: the bed has no plants, so every death here is a creature's.
    let causes: Vec<String> = pixel_physics::sim::organism::DEATH_CAUSE_LIST
        .iter()
        .filter(|c| w.deaths_by_cause[c.index()] > 0)
        .map(|c| format!("{} {}", c.label(), w.deaths_by_cause[c.index()]))
        .collect();
    println!("DEATHS frame={frame} {}", causes.join(" | "));
    let n = nest(census, w);
    let in_ch_food: usize = n.chambers.iter().map(|c| c.food).sum();
    let in_ch_brood: usize = n.chambers.iter().map(|c| c.brood).sum();
    let in_ch_ants: usize = n.chambers.iter().map(|c| c.ants).sum();
    println!(
        "NEST frame={frame} open {} cells ({} never cut) in {} region(s), largest {} | ever dug {} | chambers {} ({} cells) | food underground {} ({} in chambers) | brood underground {} ({} in chambers) | ants in chambers {in_ch_ants}",
        n.open,
        census.open_space(w).iter().filter(|p| !w.dug_cells.contains(p)).count(),
        n.regions,
        n.largest,
        w.dug_cells.len(),
        n.chambers.len(),
        n.chambers.iter().map(|c| c.cells).sum::<usize>(),
        n.food_open,
        in_ch_food,
        n.brood_open,
        in_ch_brood
    );
    let list: Vec<String> =
        n.chambers.iter().map(|c| format!("{}c@({},{}) f{} b{} a{}", c.cells, c.centre.0, c.centre.1, c.food, c.brood, c.ants)).collect();
    println!(
        "CHAMBERS frame={frame} separation {} | {}",
        separation(&n.chambers).map_or("n/a".to_string(), |v| format!("{v:.2}")),
        if list.is_empty() { "none".to_string() } else { list.join(" ") }
    );
    // **Is the door open?** Whether the nest's underground space joins the
    // sky through cells an ant can walk (air, an ant, brood, food); and what
    // stands in the founding shaft's columns from 8 rows over its mouth down
    // to the mouth's last row. A colony sealed in, or out, reads `shut`.
    if let Some(cut) = w.nest_sites.first().and_then(|s| s.shaft) {
        let start = census.open_space(w);
        let mut seen: HashSet<(i32, i32)> = start.clone();
        let mut q: VecDeque<(i32, i32)> = start.into_iter().collect();
        let mut open = false;
        while let Some((x, y)) = q.pop_front() {
            if y < cut.top - 20 {
                open = true;
                break;
            }
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let c = (x + dx, y + dy);
                if w.in_bounds(c.0, c.1) && !seen.contains(&c) && Census::open(census.what(w, c.0, c.1)) && seen.len() < 400_000 {
                    seen.insert(c);
                    q.push_back(c);
                }
            }
        }
        let mut plug: std::collections::BTreeMap<String, usize> = Default::default();
        for y in cut.top - 8..=cut.mouth_bottom {
            for x in cut.x0..=cut.x1 {
                let c = w.get(x, y);
                let name = match census.what(w, x, y) {
                    What::Empty => continue,
                    What::Ant => "ant".to_string(),
                    _ => w.materials.get(c.material).name.clone(),
                };
                *plug.entry(name).or_default() += 1;
            }
        }
        // Ants standing over the door, outside: within 3 columns of the
        // shaft and above its mouth row -- the ones a shut door keeps out.
        let (mut near, mut laden, mut pellet, mut hungry) = (0, 0, 0, 0);
        for id in w.live_organism_ids() {
            let Some(st) = w.organism(id) else { continue };
            let Some(cdef) = w.species.get(st.species).creature.as_ref() else { continue };
            let Some(&(hx, hy)) = st.chain.first() else { continue };
            if Some(w.get(hx, hy).material) == census.brood || hx < cut.x0 - 3 || hx > cut.x1 + 3 || hy > cut.top || hy < cut.top - 12 {
                continue;
            }
            near += 1;
            laden += usize::from(st.crop.is_some_and(|c| c.worth() > 0.0));
            pellet += usize::from(st.spoil.is_some());
            hungry += usize::from(st.energy < 0.5 * cdef.start_energy);
        }
        println!(
            "DOOR frame={frame} {} | dug back in: aimed {} cut {} | over the door: {near} ants ({laden} with food in the crop, {pellet} holding a pellet, {hungry} under half start energy) | shaft columns {}..{} mouth row {}, cells from 8 over it to the mouth's foot: {}",
            if open { "open" } else { "shut" },
            w.creature_stats.digs_door_aimed,
            w.creature_stats.digs_door_back,
            cut.x0,
            cut.x1,
            cut.top,
            plug.iter().map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>().join(" | ")
        );
    }
    // Dug cells that are ground again, by what fills them: spoil put down
    // inside, soil slumped in, or packed walls.
    let mut refill: std::collections::BTreeMap<String, usize> = Default::default();
    for &(x, y) in w.dug_cells.iter() {
        if y > census.ground_y && census.what(w, x, y) == What::Ground {
            *refill.entry(w.materials.get(w.get(x, y).material).name.clone()).or_default() += 1;
        }
    }
    println!("REFILL frame={frame} dug cells now ground: {}", refill.iter().map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>().join(" | "));
    if let Some(tu) = tunnels(census, w, &n) {
        let out: Vec<String> = tu.out.iter().map(|&(c, k)| if k == u32::MAX { format!("{c}c cut off") } else { format!("{c}c {k} out") }).collect();
        println!(
            "TUNNELS frame={frame} reach {} | cells {FAR}+ out {} | tips {FAR}+ out {} | other chambers {}",
            tu.reach,
            tu.far_cells,
            tu.far_tips,
            if out.is_empty() { "none".to_string() } else { out.join(" ") }
        );
    }
}

fn heap_count(census: &Census, w: &World, food_x: i32) -> usize {
    let provisions = w.materials.id_of("provisions").expect("provisions ships");
    let mut n = 0;
    for y in (census.ground_y - 40)..=(census.ground_y + 4) {
        for x in (food_x - FOOD_REACH)..=(food_x + FOOD_REACH) {
            n += usize::from(w.in_bounds(x, y) && w.get(x, y).material == provisions);
        }
    }
    n
}

/// Drop provisions into empty cells above the food spot until the heap holds
/// `target` cells or this call has dropped `target`. Returns cells dropped.
/// **The bed drains its own surface** (`drain=1`, the default): every
/// [`TOP_EVERY`] frames, standing liquid on or above the ground row between
/// the nest and the food heap (60 columns left of the nest to 30 right of the
/// heap) is removed, and the count is returned. Underground water is left
/// alone; the nest's own flooding is part of what is measured.
///
/// **Why** (lane 2's trace, 2026-10-04): the mister's water pools in the dip
/// between the spoil mound and the food heap, which dams it, and an ant
/// cannot cross liquid (`head_has_foothold` counts solid, powder and plant
/// only). In all five dead goal-box colonies the dip held 13-47 water cells
/// and the ants on the food side fell to 0-2 just before the crash; on seed
/// 4, 88 of 89 hungry ants turned back at the pool's edge. The bed exists to
/// measure the ants' rules, not where a puddle happened to form. The mister
/// stays on, since soil moisture is part of the lab. `drain=0` restores the
/// puddle.
fn drain_surface(census: &Census, w: &mut World, nest_x: i32, food_x: i32) -> usize {
    let mut n = 0;
    for y in census.ground_y - 40..=census.ground_y {
        for x in nest_x - 60..=food_x + 30 {
            if w.in_bounds(x, y) && w.materials.kind(w.get(x, y).material) == MaterialKind::Liquid {
                w.set(x, y, Cell::EMPTY);
                n += 1;
            }
        }
    }
    n
}

fn top_up(census: &Census, w: &mut World, food_x: i32, target: usize) -> usize {
    let provisions = w.materials.id_of("provisions").expect("provisions ships");
    let have = heap_count(census, w, food_x);
    if have >= target {
        return 0;
    }
    let mut want = target - have;
    let mut dropped = 0;
    // Into the air a few rows above the surface, centre outwards: it falls
    // and piles at its own angle, as a brush stroke does.
    let top = census.ground_y - 30;
    'rows: for y in top..(top + 6) {
        for k in 0..=4 {
            for x in [food_x - k, food_x + k] {
                if want == 0 {
                    break 'rows;
                }
                if w.in_bounds(x, y) && w.get(x, y).material == material::EMPTY {
                    w.set(x, y, Cell::new(provisions, 0));
                    want -= 1;
                    dropped += 1;
                }
            }
        }
    }
    dropped
}

fn shot(lab: &mut Lab, dir: &str, frame: u64, centre: (i32, i32)) {
    let (full_w, full_h) = (WIDTH, HEIGHT);
    let bounds = pixel_physics::sim::chunk::Rect::new(0, 0, lab.spec.width - 1, lab.spec.height - 1);
    let (span_x, span_y) = lab.renderer.visible_span((full_w, full_h));
    lab.renderer.set_camera(centre.0 - span_x / 2, centre.1 - span_y / 2, (full_w, full_h), Some(bounds));
    let mut full = vec![0u8; (full_w * full_h * 4) as usize];
    lab.draw(&mut full, 60.0);
    let (y0, rows) = (24u32, 216u32);
    let crop = full[(y0 * full_w * 4) as usize..((y0 + rows) * full_w * 4) as usize].to_vec();
    if let Some(img) = image::RgbaImage::from_raw(full_w, rows, crop) {
        let path = std::path::Path::new(dir).join(format!("goal_f{frame:06}.png"));
        if let Err(e) = image::imageops::resize(&img, full_w * 2, rows * 2, image::imageops::FilterType::Nearest).save(&path) {
            eprintln!("nestgoal: shot {}: {e}", path.display());
        }
    }
}

/// Scratch (lane 3): a categorical map of the nest, one colour per kind of
/// cell, so the dug shape is visible without the lab's panels over it.
fn map(census: &Census, w: &World, dir: &str, frame: u64, nest_x: i32) {
    let open = census.open_space(w);
    let room: HashSet<(i32, i32)> = open
        .iter()
        .copied()
        .filter(|&(x, y)| {
            let mut n = 0;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    n += usize::from(open.contains(&(x + dx, y + dy)));
                }
            }
            n >= ROOM_MIN
        })
        .collect();
    let (x0, x1, y0, y1) = (nest_x - 50, nest_x + 50, census.ground_y - 8, census.ground_y + 52);
    let k = 6u32;
    let (wd, ht) = (((x1 - x0 + 1) as u32) * k, ((y1 - y0 + 1) as u32) * k);
    let mut img = image::RgbaImage::new(wd, ht);
    let packed = w.materials.id_of("packedsoil");
    let soil = w.materials.id_of("soil");
    for y in y0..=y1 {
        for x in x0..=x1 {
            let what = census.what(w, x, y);
            let m = w.get(x, y).material;
            let col: [u8; 3] = match what {
                What::Ant => [26, 26, 26],
                What::Brood => [255, 110, 200],
                What::Food => [70, 230, 70],
                What::Liquid => [40, 90, 220],
                What::Empty => {
                    if room.contains(&(x, y)) {
                        [236, 227, 204]
                    } else if open.contains(&(x, y)) {
                        [196, 180, 146]
                    } else if y <= census.ground_y {
                        [18, 18, 30]
                    } else {
                        [120, 0, 0]
                    }
                }
                What::Ground => {
                    if Some(m) == soil {
                        if w.dug_cells.contains(&(x, y)) { [150, 100, 40] } else { [70, 48, 30] }
                    } else if Some(m) == packed {
                        [125, 95, 70]
                    } else {
                        [200, 180, 60]
                    }
                }
            };
            for py in 0..k {
                for px in 0..k {
                    img.put_pixel(((x - x0) as u32) * k + px, ((y - y0) as u32) * k + py, image::Rgba([col[0], col[1], col[2], 255]));
                }
            }
        }
    }
    let path = std::path::Path::new(dir).join(format!("map_f{frame:06}.png"));
    if let Err(e) = img.save(&path) {
        eprintln!("nestgoal: map {}: {e}", path.display());
    }
}

fn selftest() {
    let mut w = World::new(pixel_physics::sim::chunk::Rect::new(0, 0, 99, 99));
    let soil = w.materials.id_of("soil").expect("soil");
    let food = w.materials.id_of("provisions").expect("provisions");
    let brood = w.materials.id_of("brood");
    for y in 50..100 {
        for x in 0..100 {
            w.set(x, y, Cell::new(soil, 0));
        }
    }
    let census = Census { ground_y: 50, brood };
    // Two 6x5 rooms joined by a one-cell passage.
    let dig = |w: &mut World, x0: i32, y0: i32, wd: i32, h: i32| {
        for y in y0..y0 + h {
            for x in x0..x0 + wd {
                w.set(x, y, Cell::EMPTY);
                w.dug_cells.insert((x, y));
            }
        }
    };
    dig(&mut w, 20, 60, 6, 5);
    dig(&mut w, 40, 60, 6, 5);
    dig(&mut w, 26, 62, 14, 1);
    let brood_m = brood.expect("brood ships");
    for x in 20..26 {
        w.set(x, 64, Cell::new(food, 0));
    }
    for x in 40..46 {
        w.set(x, 64, Cell::new(brood_m, 0));
    }
    let n = nest(&census, &w);
    let sep = separation(&n.chambers);
    println!("SELFTEST apart: regions {} chambers {} separation {:?}", n.regions, n.chambers.len(), sep);
    assert_eq!(n.regions, 1, "the pair is one connected region");
    assert_eq!(n.chambers.len(), 2, "two rooms joined by a passage must read as two chambers");
    assert!(sep.is_some_and(|s| s > 0.99), "food in one, brood in the other must read separation 1");
    let tu = tunnels(&census, &w, &n).expect("chambers");
    println!("SELFTEST tunnels: reach {} far cells {} far tips {} out {:?}", tu.reach, tu.far_cells, tu.far_tips, tu.out);
    assert!(tu.out.len() == 1 && tu.out[0].1 >= 14, "a room 14 cells down a passage must read 14+ out");
    assert!(tu.far_cells > 0, "the passage and far room must count as far cells");
    for x in 20..23 {
        w.set(x, 64, Cell::new(brood_m, 0));
    }
    for x in 40..43 {
        w.set(x, 64, Cell::new(food, 0));
    }
    let n = nest(&census, &w);
    let sep = separation(&n.chambers);
    println!("SELFTEST mixed: chambers {} separation {:?}", n.chambers.len(), sep);
    assert!(sep.is_some_and(|s| s < 0.01), "both in both must read separation 0");
    println!("SELFTEST ok");
}

fn main() {
    if arg::<String>("control").as_deref() == Some("selftest") {
        selftest();
        return;
    }
    let seed: u64 = arg("seed").unwrap_or(1);
    let frames: u64 = arg("frames").unwrap_or(300_000);
    let every: u64 = arg("every").unwrap_or(25_000);
    let target: usize = arg("food").unwrap_or(120);
    let scenario: String = arg("scenario").unwrap_or_else(|| "nest_goal".to_string());
    let shots: Option<String> = arg("shots");
    let maps: Option<String> = arg("map");
    if let Some(dir) = &maps {
        let _ = std::fs::create_dir_all(dir);
    }
    let zoom: u32 = arg("zoom").unwrap_or(3).max(1);
    println!("nestgoal: scenario={scenario} seed={seed} frames={frames} every={every} food={target} shots={}", shots.as_deref().unwrap_or("-"));
    let mut sc = Scenario::load(&scenario).unwrap_or_else(|e| {
        eprintln!("scenario {scenario}: {e}");
        std::process::exit(1);
    });
    sc.bed.seed = seed;
    let mut lab = Lab::new(sc.bed.clone());
    let msg = lab.load_scenario(sc);
    // The help card opens over a fresh box and would cover every shot.
    lab.show_help = false;
    // ...and the biosphere readout opens showing after a load.
    if lab.stats.showing() {
        lab.stats.toggle();
    }
    println!("  {msg}");
    if let Some(dir) = &shots {
        let _ = std::fs::create_dir_all(dir);
        for _ in 1..zoom {
            lab.renderer.adjust_zoom(1);
        }
    }
    let census = Census { ground_y: lab.spec.ground_y, brood: lab.world.materials.id_of("brood") };
    let mut food_x: Option<i32> = None;
    let mut dropped = 0usize;
    let mut flow = Flow::default();
    let drain = arg::<u8>("drain").unwrap_or(1) != 0;
    let mut drained = 0usize;
    for f in 0..=frames {
        if food_x.is_none() {
            // The spot is fixed once the colony has founded: 30 columns east
            // of its nest, where the scenario's heap lands.
            food_x = lab.world.nest_sites.first().map(|s| s.x + 30);
            if let Some(x) = food_x {
                println!("  colony founded by frame {f}: nest at x {}, food spot x {x}", x - 30);
            }
        }
        if let Some(x) = food_x {
            if f % TOP_EVERY == 0 {
                dropped += top_up(&census, &mut lab.world, x, target);
                if drain {
                    drained += drain_surface(&census, &mut lab.world, x - 30, x);
                }
            }
            flow.step(&census, &lab.world, x - 30);
            if f % every == 0 && f > 0 {
                report(f, &census, &lab.world, dropped, x);
                println!("BED frame={f} surface water drained {drained} cells (drain={})", u8::from(drain));
                flow.report(f, &lab.world);
                if let Some(dir) = &maps {
                    map(&census, &lab.world, dir, f, x - 30);
                }
                if let Some(dir) = &shots {
                    let centre = lab.world.nest_sites.first().map_or((x, census.ground_y), |s| (s.x + 10, census.ground_y + 12));
                    shot(&mut lab, dir, f, centre);
                }
            }
        }
        if f < frames {
            lab.tick_for_harness();
        }
    }
}
