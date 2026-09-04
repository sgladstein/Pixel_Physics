//! **Does the lab box live, and what does it cost?** — Gate 1 and Gate 3 of
//! `Reports/evolution-lab-design-guide-2026-08-30.md`, in one command.
//!
//! `labbox_cost` (concept branch) asked *"what is left of the frame when you
//! take the sky and the weather away"* and answered it on a `PlantScene` bed
//! with no creatures in it. This asks the two questions that come after, in
//! the bed the game is actually played in (`lab::scene::LabBox`, called
//! rather than copied — a bed that is not the game's bed produces results
//! that do not transfer):
//!
//! 1. **Does the biosphere live?** Plants, creatures, and the organism-slot
//!    ceiling, censused over a run long enough to reach rest.
//! 2. **What does it cost, and what speed can it reach?** Whole-frame time at
//!    the population the lab *actually runs* — not at a founder cohort, which
//!    is Gate 3's own warning, because cost follows biomass.
//!
//! ## Why the census columns are the shape they are
//!
//! Every one of them is here because a number that could not have moved has
//! already cost this repo a session (`CLAUDE.md`, *ask what your number
//! counts when nothing is wrong — and when something is*):
//!
//! * **`leaf` sits beside `fruit` and `flower` as the decoder's positive
//!   control.** `Reports/creature-stamp-routes-2026-08-30.md` §5 turns on
//!   whether fruit *stands*, and a fruit census that reads zero because the
//!   material lookup is wrong is indistinguishable from a bed with no fruit
//!   in it. Leaf is a material the herb certainly grows, counted the same
//!   way, so a non-zero leaf beside a zero fruit says the counter works.
//! * **`refused` sits beside `slots`.** A population that stops growing
//!   because it hit the 4,095 slot ceiling and one that stops because it
//!   reached carrying capacity are the same curve; only `organisms_refused`
//!   separates them.
//! * **`solved/f` and `awake/f` sit beside the milliseconds.** A frame that
//!   got cheap because the field converged and one that got cheap because
//!   nothing is asking it anything are identical in a timing (*a cost that
//!   vanishes may be work that vanished*).
//! * **`births` sits beside `richest` and `margin`.** A colony too poor to
//!   reach the bar and a birth path that never fires read the same as
//!   `births 0`; the bank ceiling against the bar is what tells them apart,
//!   and §162's finding is that this one number — `ceiling − bar` — decides
//!   the outcome whatever mechanism delivers it.
//!
//! ## The founder question
//!
//! `labshot` shows 8 founders planted and 5 or 6 visible at frame 900.
//! **Germination failure and invisibility look identical and mean opposite
//! things**, so `founders:` tracks each founder organism *by the id it was
//! given at frame 0* and reports its cell count over time. A founder with 3
//! cells is alive and unrenderable; a founder whose id no longer resolves is
//! dead. Those are different findings and only the id can tell them apart.
//!
//! ## Modes
//!
//! ```text
//! cargo run --release --example lab_cost                       # census + cost, one bed
//! cargo run --release --example lab_cost -- frames=40000 every=2000
//! cargo run --release --example lab_cost -- walls=1,2,4,8,16 fans=1 reps=3
//! cargo run --release --example lab_cost -- gut=-1.0           # the Gate 0 positive control
//! cargo run --release --example lab_cost -- selftest=1         # the instrument's own controls
//! ```
//!
//! **Timings are taken paired and alternating** whenever more than one arm is
//! asked for (`reps=`), because this repo has measured a byte-identical
//! binary disagreeing 2.42x with itself on a loud machine. One arm run to
//! completion and then the next is the shape that does not survive that; a
//! round-robin is.

use pixel_physics::lab::scene::LabBox;
use pixel_physics::render::Renderer;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::creature::{
    birth_cost, diet_yield, food_value, grant_fraction, EAT_YIELD_THRESHOLD,
};
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::organism::{cell_type, CellType, TRAIT_BIRTH_GRANT, TRAIT_GUT_BIAS};
use pixel_physics::sim::particle::{self, ParticleSystem};
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;
use pixel_physics::sim::{frame, material, parallel, rigid};
use std::time::Instant;

/// The shipped tick, split at the seams `sim::frame::step` orders it in.
///
/// **This is a re-typing of `frame::step` and that is exactly the thing
/// `sim/frame.rs`'s module doc warns against**, so it is guarded rather than
/// trusted: `split_tick_matches_frame_step` runs a real lab box both ways and
/// compares a full-grid hash. If the guard is red the breakdown below is a
/// breakdown of a different simulation and must not be quoted.
const PHASES: [&str; 8] = [
    "ca_sweep",
    "liquid_bodies",
    "chunk_bodies",
    "player",
    "active_sites",
    "particles",
    "field",
    "pheromones",
];

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// A cheap order-sensitive digest of the whole grid — the same shape
/// `sim::frame`'s own control test uses, and for the same reason: what it has
/// to catch is a phase running in the wrong order, which moves cells rather
/// than counts them.
/// **What the expensive frames are doing** — the phase table, split by how
/// dear the frame was.
///
/// `Reports/evolution-lab-frame-cost-2026-09-01.md` §11.3: the lab's mean
/// frame runs about twice its median, so roughly half of all time spent is in
/// frames above the median and **the per-phase means describe that tail as
/// much as they describe the typical frame**. Reading them as "what a frame
/// costs" is the error this exists to stop.
///
/// Frames are ranked by their own total and cut at the quartile boundaries
/// that matter for a heavy tail. For each band it prints the share of *all*
/// time the band holds, the mean frame in it, and the per-phase means inside
/// it — so a phase that is flat across the bands is a per-frame cost and one
/// that climbs is the tail.
///
/// **The share column is the one to read**, not the mean: a band holding 4%
/// of the frames and 40% of the time is the whole problem however ordinary
/// its per-phase split looks, and a band that is dear but rare is not.
fn report_tail(frame: u64, rows: &[[f64; PHASES.len()]]) {
    if rows.is_empty() {
        return;
    }
    let total = |r: &[f64; PHASES.len()]| r.iter().sum::<f64>();
    let mut idx: Vec<usize> = (0..rows.len()).collect();
    idx.sort_by(|&a, &b| total(&rows[a]).partial_cmp(&total(&rows[b])).expect("no NaN"));
    let n = rows.len();
    let grand: f64 = idx.iter().map(|&i| total(&rows[i])).sum();
    println!("\n  [tail] frame {frame}: {n} frames, {grand:.0} ms of work in the window");
    println!(
        "  {:>9} {:>7} {:>8} {:>9} | {}",
        "band",
        "frames",
        "% of ms",
        "mean ms",
        PHASES.iter().map(|p| format!("{p:>13}")).collect::<Vec<_>>().join(" ")
    );
    let bands: [(usize, usize, &str); 5] = [
        (0, n / 2, "p0-50"),
        (n / 2, n * 9 / 10, "p50-90"),
        (n * 9 / 10, n * 99 / 100, "p90-99"),
        (n * 99 / 100, n.saturating_sub(1), "p99-99.9"),
        (n.saturating_sub(1), n, "worst"),
    ];
    for (lo, hi, name) in bands {
        if hi <= lo {
            continue;
        }
        let count = hi - lo;
        let band: f64 = idx[lo..hi].iter().map(|&i| total(&rows[i])).sum();
        let per: Vec<String> = (0..PHASES.len())
            .map(|k| format!("{:>13.3}", idx[lo..hi].iter().map(|&i| rows[i][k]).sum::<f64>() / count as f64))
            .collect();
        println!(
            "  {name:>9} {count:>7} {:>7.1}% {:>9.3} | {}",
            band / grand * 100.0,
            band / count as f64,
            per.join(" ")
        );
    }
    // **The single most expensive frames, itemised.** A band mean can hide
    // one 150 ms frame among ninety ordinary ones; these say whether the
    // worst frames are one phase running away or every phase rising at once,
    // which want completely different fixes.
    println!("  the five dearest frames, by phase:");
    for &i in idx.iter().rev().take(5) {
        let per: Vec<String> = rows[i].iter().map(|ms| format!("{ms:>13.3}")).collect();
        println!("  {:>9} {:>7} {:>8} {:>9.3} | {}", "", "", "", total(&rows[i]), per.join(" "));
    }
}

fn world_hash(w: &World) -> u64 {
    fn fnv1a(h: u64, v: u64) -> u64 {
        (h ^ v).wrapping_mul(0x0000_0100_0000_01b3)
    }
    let b = w.bounds().expect("the lab box sets bounds");
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            h = fnv1a(h, c.material.0 as u64);
            h = fnv1a(h, c.aux() as u64);
            h = fnv1a(h, c.organism_id() as u64);
        }
    }
    h
}

/// One census tile: everything countable about the box at one frame, plus
/// what the frames since the last tile cost.
#[derive(Default, Clone)]
struct Tile {
    frame: u64,
    plant_orgs: usize,
    plant_cells: usize,
    seeds_set: u32,
    plant_gen: u16,
    fruit: usize,
    flower: usize,
    windfall: usize,
    leaf: usize,
    deadleaf: usize,
    fruit_cells: usize,
    flower_cells: usize,
    ants: usize,
    ant_gen: u16,
    richest: f32,
    ant_energy: f64,
    births: u64,
    deaths: u64,
    denied: u64,
    refused: u64,
    eats: u64,
    slots_alloc: usize,
    slots_live: usize,
    born_total: u64,
    died_total: u64,
    /// Distance from the nearest live ant to the nearest fruit-class food, or
    /// `None` when there is no such food standing. **The `richest` bank is
    /// the stronger reading of the same question** — a bank that never rises
    /// above the leaf ceiling says outright that no ant ever ate a fruit —
    /// and this says *why*.
    nearest_food: Option<f64>,
    /// How high the fruit-class food stands above the soil line, as
    /// (lowest, highest) rows. **This is the column that turns "a foraging
    /// problem" into a mechanism**: an ant walks the ground, and a fruit
    /// still attached to its stem twenty rows up is not somewhere it can go.
    /// Only `windfall` — fruit that has ripened and dropped — is food an ant
    /// can walk to, which is exactly the delivery
    /// `creature-stamp-routes-2026-08-30.md` §5 describes and never counted.
    food_height: Option<(i32, i32)>,
    solved: f64,
    awake: f64,
    /// **Did the moisture phase fire, and how much was it asked for?**
    /// `CLAUDE.md`'s "a cost that vanishes may be work that vanished": a
    /// moisture phase that costs nothing because it stopped transporting
    /// water and one that costs nothing because it only walks the cells that
    /// need it are the same timing and opposite findings. `sw chgd` is what
    /// separates them.
    sw_visited: f64,
    sw_soil: f64,
    sw_changed: f64,
    ms_mean: f64,
    ms_p50: f64,
    ms_worst: f64,
    phase_ms: [f64; PHASES.len()],
    render_ms: f64,
}

/// Standing count of one material across the whole bed.
fn count_material(w: &World, id: material::MaterialId, width: i32, height: i32) -> usize {
    let mut n = 0;
    for y in 0..height {
        for x in 0..width {
            if w.get(x, y).material == id {
                n += 1;
            }
        }
    }
    n
}

struct Ids {
    fruit: material::MaterialId,
    flower: material::MaterialId,
    windfall: material::MaterialId,
    leaf: material::MaterialId,
    deadleaf: material::MaterialId,
}

impl Ids {
    fn of(w: &World) -> Self {
        let g = |n: &str| w.materials.id_of(n).unwrap_or_else(|| panic!("{n} is compiled in"));
        Self {
            fruit: g("fruit"),
            flower: g("flower"),
            windfall: g("windfall"),
            leaf: g("leaf"),
            deadleaf: g("deadleaf"),
        }
    }
}

fn census(w: &World, spec: &LabBox, ids: &Ids, tile: &mut Tile) {
    tile.frame = w.frame;
    let (alloc, live) = w.organism_slot_usage();
    tile.slots_alloc = alloc;
    tile.slots_live = live;
    tile.refused = w.organisms_refused();
    let (born, died) = w.organism_turnover();
    tile.born_total = born;
    tile.died_total = died;

    for id in w.live_organism_ids() {
        let Some(state) = w.organism(id) else { continue };
        if w.species.get(state.species).creature.is_some() {
            tile.ants += 1;
            tile.ant_gen = tile.ant_gen.max(state.generation);
            tile.richest = tile.richest.max(state.energy);
            tile.ant_energy += state.energy as f64;
        } else {
            tile.plant_orgs += 1;
            tile.plant_cells += state.cells.len();
            tile.seeds_set += state.seeds_set;
            tile.plant_gen = tile.plant_gen.max(state.generation);
        }
    }

    // Two independent readings of the same quantity: the material standing in
    // the grid, and the organism cell type that produced it. They are here
    // together because #162's claim is about *standing food*, and a fruit
    // still attached to its plant and a fruit lying on the soil are different
    // things to an ant — the first is an organism cell, the second is loose
    // `windfall`.
    tile.fruit = count_material(w, ids.fruit, spec.width, spec.height);
    tile.flower = count_material(w, ids.flower, spec.width, spec.height);
    tile.windfall = count_material(w, ids.windfall, spec.width, spec.height);
    tile.leaf = count_material(w, ids.leaf, spec.width, spec.height);
    tile.deadleaf = count_material(w, ids.deadleaf, spec.width, spec.height);
    for y in 0..spec.height {
        for x in 0..spec.width {
            let c = w.get(x, y);
            if c.organism_id() == 0 {
                continue;
            }
            match cell_type(c.aux()) {
                Some(CellType::Fruit) => tile.fruit_cells += 1,
                Some(CellType::Flower) => tile.flower_cells += 1,
                _ => {}
            }
        }
    }

    // Fruit-class food against the ants that would have to walk to it. Cheap
    // (ants x fruit, both tiny) and it is the half `richest` cannot give:
    // `richest` says no ant reached one, this says how far away it was.
    let fruitish: Vec<material::MaterialId> = [ids.fruit, ids.flower, ids.windfall].to_vec();
    let mut food: Vec<(i32, i32)> = Vec::new();
    let mut ants: Vec<(i32, i32)> = Vec::new();
    for y in 0..spec.height {
        for x in 0..spec.width {
            let c = w.get(x, y);
            if fruitish.contains(&c.material) {
                food.push((x, y));
            } else if cell_type(c.aux()) == Some(CellType::Head) && c.organism_id() != 0 {
                if let Some(st) = w.organism(c.organism_id()) {
                    if w.species.get(st.species).creature.is_some() {
                        ants.push((x, y));
                    }
                }
            }
        }
    }
    if !food.is_empty() {
        let lo = food.iter().map(|(_, y)| spec.ground_y - y).min().expect("food is non-empty");
        let hi = food.iter().map(|(_, y)| spec.ground_y - y).max().expect("food is non-empty");
        tile.food_height = Some((lo, hi));
    }
    if !food.is_empty() && !ants.is_empty() {
        tile.nearest_food = Some(
            ants.iter()
                .map(|(ax, ay)| {
                    food.iter()
                        .map(|(fx, fy)| (((fx - ax).pow(2) + (fy - ay).pow(2)) as f64).sqrt())
                        .fold(f64::INFINITY, f64::min)
                })
                .fold(f64::INFINITY, f64::min),
        );
    }

    let st = w.creature_stats;
    tile.births = st.births;
    tile.deaths = st.deaths;
    tile.denied = st.births_denied_no_space;
    tile.eats = st.eats;
}

/// One arm's whole run. Returns the census tiles and the world it ended on.
#[allow(clippy::too_many_arguments)]
fn run_arm(
    spec: &LabBox,
    plant_load: bool,
    bending: bool,
    size_cadence: bool,
    frames: u64,
    every: u64,
    fans: usize,
    fan_radius: i32,
    fan_force: f32,
    gut: Option<f32>,
    split: bool,
    render_every: u64,
) -> (Vec<Tile>, World, Vec<u16>) {
    // **The gut override founds the colony itself**, because a species-level
    // write has to land before `found_colony` stamps a founder's traits or
    // every reading is of the neutral gut — the failure `stamp_probe` gained
    // its own `gut=` readback for. `LabBox::build` founds inside itself, so
    // an override arm builds the identical bed with `colonies: 0` and founds
    // at the same x the scene would have used. Nothing else differs.
    let mut world;
    if let Some(g) = gut {
        let bare = LabBox { colonies: 0, ..spec.clone() };
        world = bare.build();
        let sp = world.species.id_of("ant").expect("ant species");
        let mut def = world.species.get(sp).creature.clone().expect("ant is a creature");
        def.traits[TRAIT_GUT_BIAS] = g.clamp(-1.0, 1.0);
        world.species.set_creature(sp, def);
        // The same placement `scene.rs` uses: `SHELL` is 4, and colonies are
        // spread across the usable width the same way founders are.
        let usable = spec.width - 8;
        let spacing = usable / (spec.colonies as i32 + 1);
        for i in 0..spec.colonies {
            world.found_colony(4 + spacing * (i as i32 + 1), spec.ground_y - 2);
        }
    } else {
        world = spec.build();
    }
    world.plant_load_failure = plant_load;
    world.plant_bending = bending;
    world.plant_size_cadence = size_cadence;

    let ids = Ids::of(&world);
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    let mut renderer = Renderer::new();
    let mut frame_buf = vec![0u8; (spec.width * spec.height * 4) as usize];

    // The founders, by the id they were given before a single tick ran. This
    // is the whole germination-versus-invisibility instrument: a founder that
    // never germinated is absent from this list, one that germinated and died
    // is in it and no longer resolves, one that is merely small is in it and
    // resolves to a handful of cells.
    let founder_ids = world.live_organism_ids();

    let mut tiles = Vec::new();
    let mut t0 = Tile::default();
    census(&world, spec, &ids, &mut t0);
    tiles.push(t0);

    let mut window: Vec<f64> = Vec::new();
    let mut phase_sums = [0.0f64; PHASES.len()];
    // **Every frame's phase row, kept rather than only summed** -- the phase
    // table above is a mean over the whole window, and a mean cannot say
    // whether a phase is dear on every frame or ruinous on a few. §11.3 asks
    // exactly that question and the summed form cannot answer it. `TAIL=1`.
    let tail_on: bool = std::env::var("TAIL").as_deref() == Ok("1");
    let mut frame_phase: Vec<[f64; PHASES.len()]> = Vec::new();
    let mut solved = 0u64;
    let mut awake = 0u64;
    let mut sw_visited = 0u64;
    let mut sw_soil = 0u64;
    let mut sw_changed = 0u64;
    let mut render_sum = 0.0f64;
    let mut render_n = 0u64;

    for f in 1..=frames {
        world.field_stats.tiles_solved = 0;
        // Fans before the sweep, a few rows above the soil where a bench fan
        // would sit — and **offset by a third of a spacing so a fan is never
        // sitting on a partition**. That scene error is recorded in the
        // design guide §2c: without the offset the single fan at `width/2`
        // straddles two compartments at every power-of-two wall count and
        // containment measures much weaker than it is.
        for i in 0..fans {
            let spacing = spec.width / (fans as i32 + 1);
            let x = spacing * (i as i32 + 1) + spacing / 3;
            world.add_pressure_impulse(x, spec.ground_y - 24, fan_radius, fan_force);
        }

        let ms = if split {
            let mut marks = [Instant::now(); PHASES.len() + 1];
            parallel::step(&mut world);
            marks[1] = Instant::now();
            world.step_liquid_bodies();
            marks[2] = Instant::now();
            rigid::step_chunk_bodies(&mut world);
            marks[3] = Instant::now();
            player::step(&mut world, player::PlayerInput::default(), &tuning);
            marks[4] = Instant::now();
            world.step_active_sites();
            marks[5] = Instant::now();
            blasts.step(&mut world, &mut particles);
            particle::throw_splashes(&mut world, &mut particles);
            particles.step(&mut world);
            marks[6] = Instant::now();
            world.step_fields();
            marks[7] = Instant::now();
            world.step_pheromones();
            marks[8] = Instant::now();
            let mut total = 0.0;
            let mut row = [0.0f64; PHASES.len()];
            for i in 0..PHASES.len() {
                let d = marks[i + 1].duration_since(marks[i]).as_secs_f64() * 1000.0;
                phase_sums[i] += d;
                row[i] = d;
                total += d;
            }
            if tail_on {
                frame_phase.push(row);
            }
            total
        } else {
            let t = Instant::now();
            frame::step(
                &mut world,
                &mut particles,
                &mut blasts,
                player::PlayerInput::default(),
                &tuning,
            );
            t.elapsed().as_secs_f64() * 1000.0
        };
        window.push(ms);
        sw_visited += world.soil_water_stats.visited;
        sw_soil += world.soil_water_stats.soil;
        sw_changed += world.soil_water_stats.changed;
        solved += world.field_stats.tiles_solved;
        awake += world.active_chunk_count() as u64;

        // **The render is measured, not assumed.** Gate 3's whole claim is
        // that the display rate is a design choice worth roughly tripling the
        // multiplier, and that arithmetic needs a real cost for one drawn
        // frame in this bed. Drawn the way the game draws it: dirty-rect, not
        // forced full, because the skip is exactly what a settled box buys.
        if render_every > 0 && f.is_multiple_of(render_every) {
            let touched = world.take_touched_chunks();
            let t = Instant::now();
            renderer.draw(
                &world,
                &particles,
                &touched,
                &mut frame_buf,
                (spec.width as u32, spec.height as u32),
                false,
            );
            render_sum += t.elapsed().as_secs_f64() * 1000.0;
            render_n += 1;
        }

        if f.is_multiple_of(every) || f == frames {
            let mut t = Tile::default();
            census(&world, spec, &ids, &mut t);
            window.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
            let n = window.len().max(1);
            t.ms_mean = window.iter().sum::<f64>() / n as f64;
            t.ms_p50 = window[n / 2];
            t.ms_worst = window[n - 1];
            t.solved = solved as f64 / n as f64;
            t.awake = awake as f64 / n as f64;
            t.sw_visited = sw_visited as f64 / n as f64;
            t.sw_soil = sw_soil as f64 / n as f64;
            t.sw_changed = sw_changed as f64 / n as f64;
            t.phase_ms = phase_sums.map(|s| s / n as f64);
            t.render_ms = if render_n > 0 { render_sum / render_n as f64 } else { 0.0 };
            tiles.push(t);
            if tail_on {
                report_tail(f, &frame_phase);
                frame_phase.clear();
            }
            window.clear();
            phase_sums = [0.0; PHASES.len()];
            solved = 0;
            awake = 0;
            sw_visited = 0;
            sw_soil = 0;
            sw_changed = 0;
            render_sum = 0.0;
            render_n = 0;
        }
    }
    (tiles, world, founder_ids)
}

/// Simulated seconds per real second at a given display rate, given one
/// tick's cost and one drawn frame's cost.
///
/// A tick is 1/60th of a simulated second, so a displayed frame's budget
/// (`1000/hz` ms) minus the draw, divided by the tick, is how many ticks fit;
/// times the display rate, divided by 60, is the multiplier. At a zero draw
/// cost the two rates agree, which is the arithmetic's own sanity check —
/// the whole advantage of a slower display is paying the draw less often.
fn multiplier(tick_ms: f64, render_ms: f64, hz: f64) -> f64 {
    let budget = 1000.0 / hz;
    let for_sim = (budget - render_ms).max(0.0);
    (for_sim / tick_ms) * hz / 60.0
}

fn main() {
    let frames: u64 = arg("frames").unwrap_or(20_000);
    let every: u64 = arg("every").unwrap_or(2_000);
    // **Every bed knob defaults to `LabBox`'s own, never to a literal.**
    // Written with literals first, and that was a real bug for a day: this
    // harness pinned `soil` at 80 while the scene's `DEFAULT_SOIL_DEPTH` was
    // re-derived to 40 from a measurement of where herb's roots actually
    // stop, so every "the lab box costs X" figure was taken on a bed the game
    // does not build. It is the `include_str!` gotcha wearing a different
    // hat: a knob nobody can see the value of is a knob nobody can tell is
    // disconnected, and a *defaulted* knob is worse, because it looks
    // connected. The echo line below prints all of them for the same reason.
    let d = LabBox::default();
    let width: i32 = arg("width").unwrap_or(d.width);
    let height: i32 = arg("height").unwrap_or(d.height);
    let soil: i32 = arg("soil").unwrap_or(d.soil_depth);
    let founders: usize = arg("founders").unwrap_or(d.founders);
    let colonies: usize = arg("colonies").unwrap_or(d.colonies);
    let species: String = arg("species").unwrap_or_else(|| d.species.clone());
    let seed: u64 = arg("seed").unwrap_or(d.seed);
    let walls: String = arg("walls").unwrap_or_else(|| "1".to_string());
    let fans: usize = arg("fans").unwrap_or(0);
    let fan_radius: i32 = arg("fan_radius").unwrap_or(12);
    let fan_force: f32 = arg("fan_force").unwrap_or(0.6);
    let reps: usize = arg("reps").unwrap_or(1);
    let split: bool = arg::<u32>("phases").unwrap_or(1) == 1;
    let render_every: u64 = arg("render_every").unwrap_or(20);
    let gut: Option<f32> = arg("gut");
    let selftest: bool = arg::<u32>("selftest").unwrap_or(0) == 1;
    // **The owner's own switch, reachable from the harness.** `plant_load_
    // failure` is the parameters page's `collapse_under_load`, and the owner
    // plays with it OFF -- so a cost measured with it on is not a cost they
    // ever pay. It is a field on `World`, set after the bed is built.
    let plant_load: bool = arg::<u32>("plant_load").unwrap_or(1) == 1;
    // Both default to the engine's own value, so an un-passed knob measures
    // the shipped build -- `LabBox`'s rule, and the reason the echo line
    // below prints them.
    let bending: bool = arg::<u32>("bending").unwrap_or(1) == 1;
    let size_cadence: bool = arg::<u32>("size_cadence").unwrap_or(0) == 1;

    let walls: Vec<usize> = walls.split(',').map(|s| s.parse().expect("a wall count")).collect();

    let base = LabBox {
        width,
        height,
        soil_depth: soil,
        founders,
        colonies,
        species: species.clone(),
        seed,
        ..d
    };

    // Echoes its own parameters, first line, `instruments.md`'s standing rule
    // — a 3.5-hour study once produced eight byte-identical logs because a
    // knob was not connected and nothing printed its value.
    println!(
        "lab_cost: {width}x{height} soil={soil} ground_y={} founders={founders} species={species} \
         colonies={colonies} seed={seed} walls={walls:?} fans={fans} reps={reps} frames={frames} \
         every={every} phases={} render_every={render_every} plant_load={} bending={} \
         size_cadence={} gut={} \
         (bed defaults from LabBox::default(), soil {} rows)",
        base.ground_y,
        if split { 1 } else { 0 },
        u8::from(plant_load),
        u8::from(bending),
        u8::from(size_cadence),
        gut.map_or("(ant.ron)".to_string(), |g| format!("{g:+.2}")),
        LabBox::default().soil_depth,
    );
    println!(
        "  grow light held at frame {} of {}, amplitude {:.3}",
        LabBox::noon(),
        pixel_physics::sim::field::DAY_NIGHT_PERIOD_FRAMES,
        pixel_physics::sim::field::sky_light_amplitude(LabBox::noon()),
    );

    if selftest {
        self_test(&base, split);
        return;
    }

    // **The split-tick control, run before anything is quoted from it.** The
    // per-phase breakdown re-types `frame::step`'s sequence, which is the
    // fork `sim/frame.rs` exists to prevent; this is the positive control on
    // that re-typing, and it is cheap.
    if split {
        let small = LabBox { founders: 2, colonies: 1, ..base.clone() };
        let a = hash_after(&small, 200, false);
        let b = hash_after(&small, 200, true);
        println!(
            "  split-tick control: 200 frames, frame::step {a:#018x} vs split {b:#018x} — {}",
            if a == b { "MATCH" } else { "DIVERGED, do not quote the phase breakdown" }
        );
    }

    let mut all: Vec<(usize, Vec<Vec<Tile>>)> = walls.iter().map(|w| (*w, Vec::new())).collect();

    // **Round-robin, not arm-after-arm.** On a loud box a whole arm run to
    // completion samples one stretch of machine weather; alternating spreads
    // every arm across the same stretch, which is the only shape that
    // survives a drifting machine (`CLAUDE.md`, and this repo has measured a
    // byte-identical binary disagreeing 2.42x with itself).
    let mut last: Option<(World, Vec<u16>, LabBox)> = None;
    for rep in 0..reps {
        for (i, w) in walls.iter().enumerate() {
            let spec = LabBox { compartments: *w, ..base.clone() };
            let (tiles, world, founder_ids) = run_arm(
                &spec,
                plant_load,
                bending,
                size_cadence,
                frames,
                every,
                fans,
                fan_radius,
                fan_force,
                gut,
                split,
                render_every,
            );
            if rep == reps - 1 && i == 0 {
                last = Some((world, founder_ids, spec.clone()));
            }
            all[i].1.push(tiles);
        }
        if reps > 1 {
            println!("  round {} of {reps} done", rep + 1);
        }
    }

    // The census is deterministic, so the first repetition is the census for
    // every repetition; the timing is not, so it is taken as a median across
    // repetitions below.
    for (w, runs) in &all {
        let tiles = &runs[0];
        println!("\n=== compartments {w} — does the box live? ===");
        println!(
            "{:>7} | {:>6} {:>7} {:>6} {:>4} | {:>5} {:>6} {:>6} {:>6} {:>7} | {:>5} {:>4} {:>6} {:>6} {:>5} | {:>9} {:>7}",
            "frame",
            "orgs",
            "cells",
            "seeds",
            "gen",
            "fruit",
            "flower",
            "windfl",
            "leaf",
            "deadlf",
            "ants",
            "gen",
            "births",
            "deaths",
            "refus",
            "slot a/l",
            "eats",
        );
        for t in tiles {
            println!(
                "{:>7} | {:>6} {:>7} {:>6} {:>4} | {:>5} {:>6} {:>6} {:>6} {:>7} | {:>5} {:>4} {:>6} {:>6} {:>5} | {:>4}/{:<4} {:>7}",
                t.frame,
                t.plant_orgs,
                t.plant_cells,
                t.seeds_set,
                t.plant_gen,
                t.fruit,
                t.flower,
                t.windfall,
                t.leaf,
                t.deadleaf,
                t.ants,
                t.ant_gen,
                t.births,
                t.deaths,
                t.refused,
                t.slots_alloc,
                t.slots_live,
                t.eats,
            );
        }
        // **Has it settled?** The tell that works is that the quantity being
        // censused has stopped moving across two consecutive tiles — not that
        // the queue went quiet, and not a budget picked by eye
        // (`CLAUDE.md`, *a cascade censused before it settles*).
        if tiles.len() >= 3 {
            let n = tiles.len();
            let (a, b, c) =
                (tiles[n - 3].plant_cells, tiles[n - 2].plant_cells, tiles[n - 1].plant_cells);
            let d1 = (b as f64 - a as f64) / a.max(1) as f64;
            let d2 = (c as f64 - b as f64) / b.max(1) as f64;
            println!(
                "  plant cells across the last three tiles: {a} -> {b} -> {c} ({:+.1}%, {:+.1}%) — {}",
                d1 * 100.0,
                d2 * 100.0,
                if d1.abs() < 0.05 && d2.abs() < 0.05 {
                    "settled"
                } else {
                    "STILL MOVING, the run is short of rest"
                }
            );
        }
        // The second half of the census: the quantities that say *why* a
        // curve is the shape it is. `born`/`died` is plant turnover, which a
        // standing organism count cannot show — a stand holding at 40
        // organisms because nothing is happening and one holding at 40
        // because 300 germinated and 300 died are the same row above and
        // different worlds. `richest` against the bar is #162's arithmetic,
        // and `denied` separates an energy result from a space one.
        println!(
            "\n{:>7} | {:>7} {:>7} {:>8} | {:>8} {:>10} | {:>7} {:>7} {:>8} | {:>9} {:>9}",
            "frame", "p.born", "p.died", "live", "richest", "ant energy", "fruitC", "flowrC",
            "denied", "ant->food", "food up",
        );
        for t in tiles {
            println!(
                "{:>7} | {:>7} {:>7} {:>8} | {:>8.0} {:>10.0} | {:>7} {:>7} {:>8} | {:>9} {:>9}",
                t.frame,
                t.born_total,
                t.died_total,
                t.slots_live,
                t.richest,
                t.ant_energy,
                t.fruit_cells,
                t.flower_cells,
                t.denied,
                t.nearest_food.map_or("no food".to_string(), |d| format!("{d:.0}")),
                t.food_height.map_or("-".to_string(), |(lo, hi)| format!("{lo}..{hi}")),
            );
        }

        let hw = tiles.last().expect("a tile").slots_alloc;
        println!(
            "  organism slots: high water {hw} of 4095 ({:.1}% of the ceiling), refused {}",
            hw as f64 / 4095.0 * 100.0,
            tiles.last().expect("a tile").refused,
        );
    }

    // --- bit-identity ----------------------------------------------------
    // **The check a pure optimisation is judged by.** `world_hash` is the
    // same full-grid digest `split_tick_matches_frame_step` uses; printed
    // here it lets two builds of this harness be compared for *identical
    // output* rather than merely similar counters. A refactor that claims to
    // change no behaviour and moves this number has changed behaviour, and a
    // census that agrees to five significant figures will not say so.
    if let Some((world, _, _)) = &last {
        println!("\nworld hash at frame {frames}: {:#018x}", world_hash(world));
    }

    // --- the founder question -------------------------------------------
    if let Some((world, founder_ids, spec)) = &last {
        println!("\n=== the founders: germination or invisibility? ===");
        println!(
            "  {} organisms existed before the first tick ({} founders + {} colony head(s) asked for)",
            founder_ids.len(),
            spec.founders,
            spec.colonies,
        );
        println!("  {:>4} {:>8} {:>7} {:>9}  verdict", "#", "id", "cells", "kind");
        for (i, id) in founder_ids.iter().enumerate() {
            match world.organism(*id) {
                Some(s) => {
                    let kind = if world.species.get(s.species).creature.is_some() {
                        "creature"
                    } else {
                        "plant"
                    };
                    let n = s.cells.len();
                    println!(
                        "  {:>4} {:>8} {:>7} {:>9}  {}",
                        i,
                        id,
                        n,
                        kind,
                        match n {
                            0 => "alive with no cells".to_string(),
                            1..=8 => format!("alive, {n} cells — at or under the findable floor"),
                            _ => format!("alive, {n} cells"),
                        }
                    );
                }
                None => println!(
                    "  {:>4} {:>8} {:>7} {:>9}  no longer resolves — this founder is DEAD",
                    i, id, "-", "-"
                ),
            }
        }
    }

    // --- Gate 0 in this bed ---------------------------------------------
    let last_run_richest = all
        .first()
        .and_then(|(_, runs)| runs.first())
        .map(|tiles| tiles.iter().map(|t| t.richest).fold(0.0f32, f32::max))
        .unwrap_or(0.0);
    if let Some((world, _, spec)) = &last {
        println!("\n=== Gate 0 in the lab bed: can an ant here afford a child? ===");
        let sp = world.species.id_of("ant").expect("ant species");
        let def = world.species.get(sp).creature.clone().expect("ant is a creature");
        // **Read the gut back off a live founder, never off the def.** A
        // species-level write that did not reach the founders leaves the run
        // silently measuring the neutral gut — the trap `stamp_probe` gained
        // its readback for, and it applies identically here because this
        // harness can override the gut.
        let founder_gut = live_founder_gut(world, spec);
        let gut_bias = founder_gut.unwrap_or(def.traits[TRAIT_GUT_BIAS]);
        let bar = birth_cost(&def);
        let grant = grant_fraction(def.traits[TRAIT_BIRTH_GRANT]) * def.start_energy;

        // What this gut can draw from each food standing in this bed, and
        // what it *could* draw from the whole material table. #162's finding
        // is that those two differ by a factor of nearly three and only the
        // first is real.
        let mut rows: Vec<(String, usize, f32)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for y in 0..spec.height {
            for x in 0..spec.width {
                let c = world.get(x, y);
                if food_value(world, c) > 0.0 && seen.insert(c.material) {
                    rows.push((
                        world.materials.get(c.material).name.clone(),
                        0,
                        diet_yield(world, Cell::new(c.material, 0), gut_bias),
                    ));
                }
            }
        }
        for r in rows.iter_mut() {
            let id = world.materials.id_of(&r.0).expect("a material we just read");
            r.1 = count_material(world, id, spec.width, spec.height);
        }
        rows.sort_by(|a, b| b.2.total_cmp(&a.2));
        let best_here = rows.first().map_or(0.0, |r| r.2);
        // A rate rather than a ceiling: see `stamp_probe`. `best_here` is a
        // yield, so dividing by its face recovers this gut's conversion.
        let upkeep = def.idle_cost_per_cell * def.body.len() as f32;
        let best_face = rows.first().map_or(0.0, |r| r.2).max(best_here);
        let net = def.digest_rate * if best_face > 0.0 { best_here / best_face } else { 0.0 } - upkeep;
        println!(
            "  gut {gut_bias:+.2} (founder reads {}) | start_energy {:.0} \
             digest {:.2}/tick grant {grant:.0} body_energy {:.0}",
            founder_gut.map_or("NO LIVE FOUNDER".to_string(), |g| format!("{g:+.2}")),
            def.start_energy,
            def.digest_rate,
            def.body_energy,
        );
        println!("  standing food in this bed, and what this gut draws from a mouthful:");
        for (name, n, y) in &rows {
            println!(
                "    {name:>10} x{n:<7} yield {y:>7.1}  {}",
                if *y > EAT_YIELD_THRESHOLD { "edible" } else { "INVISIBLE to this gut" }
            );
        }
        // **The discriminator, and it is the whole of the Gate 0 answer.**
        // `ceiling` is what an ant *could* hold if it ate the best food in
        // the bed; `richest` is what one actually held. A positive margin
        // with a bank that never left the leaf ceiling is not an economy
        // result at all — it says no ant ever reached the food the ceiling
        // was computed from, which is `creature-stamp-routes-2026-08-30.md`
        // §5's named failure case and a foraging finding, not an economy one.
        let ever_richest = last_run_richest;
        println!(
            "  on the best mouthful standing here ({best_here:.0}) an ant nets {net:+.3}/tick after {upkeep:.3} upkeep, against a bar of {bar:.0}  =>  {}",
            if net > 0.0 {
                format!("a child every {:.0} ticks of uninterrupted feeding", bar / net)
            } else {
                "negative: no ant in this bed can out-eat its own upkeep, so no amount of time produces a birth".to_string()
            }
        );
        println!(
            "  richest bank actually reached in this run: {ever_richest:.0} against the {bar:.0} bar \
             => {}",
            if ever_richest >= bar {
                "an ant did reach the bar"
            } else if net > 0.0 {
                "NO ant ever reached it, though the bed contains food that would — \
                 a foraging result, not an economy one"
            } else {
                "as expected: the bed contains nothing that would get it there"
            }
        );
        // If fruit stands and births are still zero the answer is foraging,
        // not economy — #162 §5 names that split explicitly, so the distance
        // is printed rather than left to be inferred.
        report_forage_reach(world, spec);
    }

    // --- cost -------------------------------------------------------------
    println!("\n=== what does it cost? (whole-frame, in the bed, as the stand grows) ===");
    println!(
        "{:>5} {:>7} | {:>8} {:>8} {:>8} {:>8} | {:>8} {:>8} | {:>8} {:>8} {:>8} | {:>9} | {:>7} {:>7} {:>7}",
        "walls",
        "frame",
        "cells",
        "ms mean",
        "ms p50",
        "ms worst",
        "solved/f",
        "awake/f",
        "sw seen",
        "sw soil",
        "sw chgd",
        "us/cell",
        "x sim",
        "x@60Hz",
        "x@20Hz",
    );
    for (w, runs) in &all {
        // Median across repetitions, tile by tile: the paired reading.
        let ntiles = runs[0].len();
        for k in 1..ntiles {
            let mut means: Vec<f64> = runs.iter().map(|r| r[k].ms_mean).collect();
            means.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
            let mean = means[means.len() / 2];
            // **The median across repetitions of each run's own median frame.**
            // Not the first repetition's, which is what this printed until it
            // was caught: a column that ignores every arm but one is a
            // one-sample reading wearing a paired-measurement's label. And
            // the median frame is the statistic to read on a loud box — the
            // mean carries the tail that machine contention writes into it,
            // and `CLAUDE.md` records that an untrusted median is still worth
            // something where an untrusted worst is not.
            let mut p50s: Vec<f64> = runs.iter().map(|r| r[k].ms_p50).collect();
            p50s.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
            let p50 = p50s[p50s.len() / 2];
            let mut rend: Vec<f64> = runs.iter().map(|r| r[k].render_ms).collect();
            rend.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
            let render = rend[rend.len() / 2];
            let t = &runs[0][k];
            let live = t.plant_cells + t.ants;
            println!(
                "{:>5} {:>7} | {:>8} {:>8.3} {:>8.3} {:>8.3} | {:>8.1} {:>8.1} | {:>8.0} {:>8.0} {:>8.1} | {:>9.2} | {:>6.1}x {:>6.1}x {:>6.1}x",
                w,
                t.frame,
                t.plant_cells,
                mean,
                p50,
                t.ms_worst,
                t.solved,
                t.awake,
                t.sw_visited,
                t.sw_soil,
                t.sw_changed,
                mean * 1000.0 / live.max(1) as f64,
                // **`x sim` is the dial's ceiling with the render taken out**
                // — simulated seconds per real second if drawing were free.
                // It is here because the two columns beside it are only as
                // trustworthy as one measured render, and this one is not.
                1000.0 / mean / 60.0,
                multiplier(mean, render, 60.0),
                multiplier(mean, render, 20.0),
            );
        }
        // **`mean x frames ~= worst` before any worst is quoted.** If an
        // aggregate pins the worst it is a real event; if it does not, the
        // worst is an order statistic over many similar frames and is noise
        // wearing a number.
        let t = runs[0].last().expect("a tile");
        let pin = t.ms_mean * every as f64 / t.ms_worst;
        println!(
            "  worst-frame check on the last tile: mean {:.3} x {every} frames = {:.1} ms against a \
             worst of {:.3} ms — ratio {pin:.3}, {}",
            t.ms_mean,
            t.ms_mean * every as f64,
            t.ms_worst,
            if pin < 2.0 {
                "an aggregate pins it, the worst is a real event"
            } else {
                "nothing pins it, the worst is noise and must not be quoted"
            }
        );
        println!("  render (dirty-rect, {}x{}): {:.3} ms/frame", width, height, t.render_ms);
    }

    if split {
        println!("\nper-phase mean, ms (last tile of each arm)");
        print!("{:>5}", "walls");
        for p in PHASES {
            print!("  {p:>13}");
        }
        println!();
        for (w, runs) in &all {
            let t = runs[0].last().expect("a tile");
            print!("{w:>5}");
            for ms in t.phase_ms {
                print!("  {ms:>13.3}");
            }
            println!();
        }
    }

    if all.len() > 1 {
        println!("\ncompartments, at the last tile — the §2c claim, in this bed");
        println!(
            "{:>5} {:>9} {:>10} {:>9} {:>9} {:>9}",
            "walls", "p50 ms", "solved/f", "speed-up", "cells", "vs open"
        );
        let open = {
            let runs = &all[0].1;
            let mut m: Vec<f64> = runs.iter().map(|r| r.last().expect("a tile").ms_p50).collect();
            m.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
            m[m.len() / 2]
        };
        for (w, runs) in &all {
            let mut m: Vec<f64> = runs.iter().map(|r| r.last().expect("a tile").ms_p50).collect();
            m.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
            let mean = m[m.len() / 2];
            let t = runs[0].last().expect("a tile");
            println!(
                "{:>5} {:>8.3}ms {:>10.1} {:>8.1}x {:>9} {:>8.2}x",
                w,
                mean,
                t.solved,
                1000.0 / mean / 60.0,
                t.plant_cells,
                open / mean,
            );
        }
    }
}

/// The gut a *live* ant in this bed actually carries.
fn live_founder_gut(world: &World, spec: &LabBox) -> Option<f32> {
    for y in 0..spec.height {
        for x in 0..spec.width {
            let c = world.get(x, y);
            if cell_type(c.aux()) != Some(CellType::Head) {
                continue;
            }
            let Some(state) = world.organism(c.organism_id()) else { continue };
            if world.species.get(state.species).creature.is_some() {
                return Some(state.traits[TRAIT_GUT_BIAS]);
            }
        }
    }
    None
}

/// How far a live ant is from the nearest fruit-class food.
///
/// #162 §5: *"a fruit has to be found. The failure case is a foraging problem
/// rather than an economy one."* Fruit standing and births at zero is a
/// different finding from no fruit at all, and this is what separates them.
fn report_forage_reach(world: &World, spec: &LabBox) {
    let fruitish: Vec<material::MaterialId> = ["fruit", "flower", "windfall"]
        .iter()
        .filter_map(|n| world.materials.id_of(n))
        .collect();
    let mut food: Vec<(i32, i32)> = Vec::new();
    let mut ants: Vec<(i32, i32)> = Vec::new();
    for y in 0..spec.height {
        for x in 0..spec.width {
            let c = world.get(x, y);
            if fruitish.contains(&c.material) {
                food.push((x, y));
            }
            if cell_type(c.aux()) == Some(CellType::Head) {
                if let Some(s) = world.organism(c.organism_id()) {
                    if world.species.get(s.species).creature.is_some() {
                        ants.push((x, y));
                    }
                }
            }
        }
    }
    if food.is_empty() {
        println!(
            "  foraging: {} live ant(s), and NO fruit-class food standing anywhere in the bed — \
             the economy question does not arise yet",
            ants.len()
        );
        return;
    }
    let mut nearest: Vec<f64> = ants
        .iter()
        .map(|(ax, ay)| {
            food.iter()
                .map(|(fx, fy)| (((fx - ax).pow(2) + (fy - ay).pow(2)) as f64).sqrt())
                .fold(f64::INFINITY, f64::min)
        })
        .collect();
    nearest.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
    println!(
        "  foraging: {} live ant(s), {} fruit-class cell(s); nearest food is {:.0} cells away \
         (median {:.0}, worst {:.0})",
        ants.len(),
        food.len(),
        nearest.first().copied().unwrap_or(f64::NAN),
        nearest.get(nearest.len() / 2).copied().unwrap_or(f64::NAN),
        nearest.last().copied().unwrap_or(f64::NAN),
    );
}

fn hash_after(spec: &LabBox, frames: u64, split: bool) -> u64 {
    let mut world = spec.build();
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    for _ in 0..frames {
        if split {
            parallel::step(&mut world);
            world.step_liquid_bodies();
            rigid::step_chunk_bodies(&mut world);
            player::step(&mut world, player::PlayerInput::default(), &tuning);
            world.step_active_sites();
            blasts.step(&mut world, &mut particles);
            particle::throw_splashes(&mut world, &mut particles);
            particles.step(&mut world);
            world.step_fields();
            world.step_pheromones();
        } else {
            frame::step(
                &mut world,
                &mut particles,
                &mut blasts,
                player::PlayerInput::default(),
                &tuning,
            );
        }
    }
    world_hash(&world)
}

/// **The instrument's own positive controls.** Every counter this harness
/// reports is checked against a case whose answer is known to be non-zero,
/// because a number that is arithmetically correct and cannot move looks
/// exactly like a result (`CLAUDE.md`, the worst-recurring failure here).
fn self_test(base: &LabBox, split: bool) {
    println!("\n=== selftest: can these numbers move? ===");
    let mut fails = 0;

    // 1. The split tick is the shipped tick.
    if split {
        let small = LabBox { founders: 2, ..base.clone() };
        let a = hash_after(&small, 200, false);
        let b = hash_after(&small, 200, true);
        let ok = a == b;
        fails += !ok as i32;
        println!("  [{}] split tick reproduces frame::step over 200 frames", tick(ok));
    }

    // 2. The material census can count. A bed with founders grows leaves; a
    //    bed with none must read zero. Both directions, because a counter
    //    that always reads non-zero is as blind as one that always reads
    //    zero.
    let ids;
    let grown = {
        let spec = LabBox { founders: 8, colonies: 0, ..base.clone() };
        let mut w = spec.build();
        ids = Ids::of(&w);
        let mut p = ParticleSystem::new();
        let mut b = Blasts::new();
        let t = player::Tuning::default();
        for _ in 0..4_000 {
            frame::step(&mut w, &mut p, &mut b, player::PlayerInput::default(), &t);
        }
        count_material(&w, ids.leaf, spec.width, spec.height)
    };
    let bare = {
        let spec = LabBox { founders: 0, colonies: 0, ..base.clone() };
        let w = spec.build();
        count_material(&w, ids.leaf, spec.width, spec.height)
    };
    let ok = grown > 0 && bare == 0;
    fails += !ok as i32;
    println!(
        "  [{}] leaf census: {grown} in a planted bed at frame 4000, {bare} in an unplanted one",
        tick(ok)
    );

    // 3. `solved/f` responds to a fan. This is the control for the whole
    //    partition question: if one fan does not raise the solve set, the fan
    //    is not connected and every walls= row below it is meaningless.
    let quiet = fan_probe(base, 0);
    let windy = fan_probe(base, 1);
    let ok = windy > quiet * 1.2;
    fails += !ok as i32;
    println!(
        "  [{}] one fan wakes the box: solved/f {quiet:.1} with no fan, {windy:.1} with one",
        tick(ok)
    );

    // 4. The organism-slot high water tracks the live count. A bed with more
    //    founders must allocate more slots, or the ceiling reading is inert.
    let few = LabBox { founders: 2, colonies: 0, ..base.clone() }.build().organism_slot_usage().0;
    let many = LabBox { founders: 16, colonies: 0, ..base.clone() }.build().organism_slot_usage().0;
    let ok = many > few;
    fails += !ok as i32;
    println!("  [{}] slot high water moves with founders: {few} at 2, {many} at 16", tick(ok));

    println!(
        "\n  {} control(s) failed. {}",
        fails,
        if fails == 0 {
            "Every number this harness prints has been shown able to move."
        } else {
            "DO NOT QUOTE the readings a failed control covers."
        }
    );
}

fn fan_probe(base: &LabBox, fans: usize) -> f64 {
    let spec = LabBox { founders: 0, colonies: 0, ..base.clone() };
    let mut w = spec.build();
    let mut p = ParticleSystem::new();
    let mut b = Blasts::new();
    let t = player::Tuning::default();
    let mut solved = 0u64;
    let n = 400u64;
    for _ in 0..n {
        w.field_stats.tiles_solved = 0;
        for i in 0..fans {
            let spacing = spec.width / (fans as i32 + 1);
            let x = spacing * (i as i32 + 1) + spacing / 3;
            w.add_pressure_impulse(x, spec.ground_y - 24, 12, 0.6);
        }
        frame::step(&mut w, &mut p, &mut b, player::PlayerInput::default(), &t);
        solved += w.field_stats.tiles_solved;
    }
    solved as f64 / n as f64
}

fn tick(ok: bool) -> &'static str {
    if ok {
        "PASS"
    } else {
        "FAIL"
    }
}
