//! **Two soils: what a colony's spoil does to the plants it engulfs, and
//! which world the owner wants** -- built for
//! `Reports/evolution-lab-soil-design-2026-09-12.md` (round thirty, lane U).
//!
//! The owner's question, 2026-09-12: the ground plants make when they rot is
//! loose, the ground ants make when they dig is not, and a mound of the second
//! kind "engulfs plants and creates a ground that new plants don't grow in".
//! Two things had to be measured rather than argued before anything could be
//! recommended, and this file is both of them:
//!
//! **`mode=bury` -- does burial already kill a plant?** The brief's first
//! candidate is "buried plants die of darkness, rot to soil, new plants grow
//! through", and `field.rs` says on its face that it cannot be true as
//! written: only `Solid` and `Plant` cells count as opaque in
//! `rebuild_blocked`'s column scan, so a leaf inside a heap of `packedsoil`
//! -- a `Powder` -- sees the full lamp. This mode grows the played bed's own
//! plants with no colony, then buries every shoot -- a box `margin=` columns
//! wider than the shoot on each side and `cover=` rows over its top, to the
//! surface -- in one of three arms and runs on: `none` (the control),
//! `packed` (a colony's spoil), `stone` (a `Solid` roof -- the **positive
//! control**, which is what says the instrument can see a plant starve of
//! light at all). Per plant it prints cells before and after, income against
//! maintenance, the starvation clock, whether it was marked senescent, and
//! the mean light over its shoot cells.
//!
//! **The first version of this buried each shoot to a fixed twelve rows in a
//! box one column wider than the plant, and its stone arm changed nothing**
//! -- income identical to the control on every grass -- which is the positive
//! control doing its job: the light field is read per 8x8 block and its
//! transmission is the *mean over the block's eight columns*, so a three-
//! column stone pillar passes five-eighths of the lamp, and a fill that stops
//! at the surface leaves the leaf's own block lit from the open block above
//! it. The defaults below (`margin=8`, `cover=8`) are the smallest that make
//! the stone arm go dark, and the number to quote is the stone arm's light,
//! not the geometry.
//!
//! **`mode=fork` -- the fork the 2026-08-31 card never got an answer to.**
//! The played bed with its colony, run to `frames=`, censused, then **forked
//! from one world state** (`World` is `Clone`) into three arms that each run
//! `after=` more frames and are rendered: `keep` (today -- tamped spoil stands
//! wherever it was put), `loose` (every packed cell above the original
//! surface is loose soil again, which is the owner's "placed soil doesn't
//! stay" as an end state), and `weather` (exposed packed cells above the
//! surface revert to loose soil at a litter-like rate, on the decay
//! channel's own 200-frame schedule -- round-30 brief 3's mechanism,
//! staged from the harness so it can be looked at before it is built). The
//! arms share the pre-fork world byte for byte, so the difference between the
//! renders is the rule and nothing else.
//!
//! Every stop prints the counts the picture cannot carry (`CLAUDE.md`: an
//! image says what and where, only a number says whether it fired): packed
//! cells above and below the original surface, roofed void, open pit, the
//! bare columns in the nest band, shoot cells touching packed soil and shoot
//! cells enclosed on all eight sides, and **hanging** packed cells -- spoil
//! above the surface with no path down to the bank through other ground,
//! which is the "dirt floating in mid-air" the owner reported and the crumb
//! rule (`update.rs`) exists to stop.
//!
//! ```text
//! cargo run --release --example soilfork -- mode=bury grow=6000 after=12000 margin=8 cover=8
//! cargo run --release --example soilfork -- mode=fork seed=3 frames=100000 after=6000 out=/tmp/fork
//! ```
//!
//! `RAYON_NUM_THREADS=1` for any count you mean to compare across runs; the
//! creature counters downstream of the checkerboard move with the thread
//! count (`CLAUDE.md`, *a counter is only load-independent at fixed
//! parallelism*).
use pixel_physics::lab::scenario::{Placement, Scenario};
use pixel_physics::lab::scene::LabBox;
use pixel_physics::render::Renderer;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::field;
use pixel_physics::sim::frame;
use pixel_physics::sim::material::{self, MaterialId, MaterialKind};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::update::soil_moisture;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

/// Half-width of the nest band, in columns -- `latecensus`'s own value, so
/// the bare-band figure here is the same statistic as the late-game report's.
const BAND: i32 = 64;
/// How far above the original surface ground still counts as the bed's own
/// rather than the lid or a lamp -- `latecensus`'s own value.
const MOUND_REACH: i32 = 48;
/// Frames between weathering checks in the `weather` arm --
/// `decay::DECAY_TICK_INTERVAL`'s own value (it is `pub(crate)`), so the
/// staged rate is on the schedule the real channel would run on.
const WEATHER_TICK: u64 = 200;
/// Per-check chance an exposed packed cell above the surface goes back to
/// loose soil in the `weather` arm. `litter.ron`'s damp rate is 0.5 and its
/// dry rate 0.1; a mound is worked mineral ground, not leaves, so it is
/// staged an order slower than litter and an order faster than ash's 0.05
/// damp rate would need -- a number to sweep, not a number to keep.
const WEATHER_CHANCE: f32 = 0.05;

fn main() {
    let mode: String = arg("mode").unwrap_or_else(|| "fork".to_string());
    match mode.as_str() {
        "bury" => bury(),
        "fork" => fork(),
        other => {
            eprintln!("soilfork: mode={other} is not bury or fork");
            std::process::exit(2);
        }
    }
}

fn load_bed() -> Scenario {
    let scenario_name: String = arg("scenario").unwrap_or_else(|| "played_bed".to_string());
    let mut scenario = Scenario::load(&scenario_name).unwrap_or_else(|e| {
        eprintln!("scenario {scenario_name}: {e}");
        std::process::exit(2);
    });
    if let Some(sd) = arg::<u64>("seed") {
        scenario.bed.seed = sd;
    }
    scenario
}

fn run(world: &mut World, frames: u64, scenario: Option<&Scenario>, spec: &LabBox) {
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    for _ in 0..frames {
        if let Some(s) = scenario {
            pixel_physics::lab::scenario::tick_timeline(s, world, spec);
        }
        frame::step(world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
    }
}

fn is_plant_cell(world: &World, cell: Cell) -> bool {
    let id = cell.organism_id();
    id != 0 && world.organism(id).is_some_and(|st| world.species.get(st.species).creature.is_none())
}

fn is_ground(world: &World, cell: Cell) -> bool {
    cell.material != material::EMPTY
        && matches!(world.materials.kind(cell.material), MaterialKind::Powder | MaterialKind::Solid)
        && cell.organism_id() == 0
}

// ---------------------------------------------------------------------------
// mode=bury
// ---------------------------------------------------------------------------

struct PlantRow {
    id: u16,
    species: String,
    x: i32,
    shoot_before: usize,
    rows: i32,
}

fn bury() {
    let grow: u64 = arg("grow").unwrap_or(6_000);
    let after: u64 = arg("after").unwrap_or(12_000);
    let margin: i32 = arg("margin").unwrap_or(8);
    let cover: i32 = arg("cover").unwrap_or(8);
    let scenario = load_bed();
    let spec = scenario.bed.clone();
    println!(
        "soilfork bury: scenario={} seed={} grow={grow} after={after} margin={margin} cover={cover} threads={}",
        scenario.name,
        spec.seed,
        std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "default".into())
    );
    let (mut world, planted, _) = scenario.build();
    println!("  bed: {} of {} founders planted; the colony's timeline is NOT ticked in this mode", planted.planted, planted.asked);
    let t = std::time::Instant::now();
    // No timeline: the plants grow with nothing eating them.
    run(&mut world, grow, None, &spec);
    println!("  grew {grow} frames in {:.1} s", t.elapsed().as_secs_f64());

    let mut plants: Vec<PlantRow> = Vec::new();
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        let species = world.species.get(st.species);
        if species.creature.is_some() {
            continue;
        }
        let shoot: Vec<(i32, i32)> = st.cells.keys().copied().filter(|&(_, y)| y < spec.ground_y).collect();
        if shoot.is_empty() {
            continue;
        }
        let top = shoot.iter().map(|&(_, y)| y).min().unwrap_or(spec.ground_y);
        let x = shoot.iter().map(|&(x, _)| x).sum::<i32>() / shoot.len() as i32;
        plants.push(PlantRow { id, species: species.name.clone(), x, shoot_before: shoot.len(), rows: spec.ground_y - top });
    }
    plants.sort_by_key(|p| p.x);
    println!("  {} plant(s) with a shoot above the surface at frame {grow}", plants.len());

    let arms: Vec<String> =
        arg::<String>("arms").unwrap_or_else(|| "none,packed,stone".to_string()).split(',').map(|s| s.to_string()).collect();
    for arm in &arms {
        let mut w = world.clone();
        let fill: Option<MaterialId> = match arm.as_str() {
            "none" => None,
            "packed" => w.materials.id_of("packedsoil"),
            "stone" => w.materials.id_of("stone"),
            "soil" => w.materials.id_of("soil"),
            other => {
                eprintln!("soilfork: arm {other} is not none, packed, stone or soil");
                std::process::exit(2);
            }
        };
        let mut filled = 0usize;
        if let Some(mat) = fill {
            for p in &plants {
                let Some(st) = w.organism(p.id) else { continue };
                let shoot: Vec<(i32, i32)> = st.cells.keys().copied().filter(|&(_, y)| y < spec.ground_y).collect();
                let x0 = shoot.iter().map(|&(x, _)| x).min().unwrap_or(p.x) - margin;
                let x1 = shoot.iter().map(|&(x, _)| x).max().unwrap_or(p.x) + margin;
                let top = shoot.iter().map(|&(_, y)| y).min().unwrap_or(spec.ground_y) - cover;
                for y in top.max(1)..spec.ground_y {
                    for x in x0..=x1 {
                        if w.is_empty(x, y) {
                            w.set(x, y, Cell::new(mat, 0));
                            filled += 1;
                        }
                    }
                }
            }
        }
        let t = std::time::Instant::now();
        run(&mut w, after, None, &spec);
        println!(
            "\narm={arm}: buried {filled} cell(s) (margin {margin}, cover {cover}), ran {after} more frames in {:.1} s",
            t.elapsed().as_secs_f64()
        );
        println!(
            "{:>5} {:>10} {:>4} {:>6} {:>4} | {:>6} {:>6} {:>7} {:>7} {:>5} {:>4} {:>6}",
            "id", "species", "x", "shoot", "rows", "shoot'", "cells'", "income", "maint", "strv", "sen", "light"
        );
        let (mut alive, mut senescent, mut dead) = (0usize, 0usize, 0usize);
        for p in &plants {
            match w.organism(p.id) {
                None => {
                    dead += 1;
                    println!("{:>5} {:>10} {:>4} {:>6} {:>4} | {:>6} {:>6} {:>7} {:>7} {:>5} {:>4} {:>6}", p.id, p.species, p.x, p.shoot_before, p.rows, "-", "dead", "-", "-", "-", "-", "-");
                }
                Some(st) => {
                    let mut shoot_light = 0.0f32;
                    let mut shoot = 0usize;
                    // Over every shoot cell rather than `CellType::Leaf` only:
                    // grass earns through blades that are not `Leaf` cells,
                    // and a per-species leaf test read NaN on every grass.
                    for &(x, y) in st.cells.keys() {
                        if y < spec.ground_y {
                            shoot += 1;
                            // Raw, not noon-equivalent: the lab pins its sky at the
                            // measured-brightest frame, so the raw field is the noon
                            // reading and the crate-private normaliser is not needed.
                            shoot_light += w.field_at(x, y).light / field::MAX_LIGHT;
                        }
                    }
                    let light = if shoot > 0 { shoot_light / shoot as f32 } else { f32::NAN };
                    if st.senescent {
                        senescent += 1;
                    } else {
                        alive += 1;
                    }
                    println!(
                        "{:>5} {:>10} {:>4} {:>6} {:>4} | {:>6} {:>6} {:>7.3} {:>7.3} {:>5} {:>4} {:>6.3}",
                        p.id, p.species, p.x, p.shoot_before, p.rows, shoot, st.cells.len(), st.income, st.maintenance, st.starving_ticks,
                        if st.senescent { "yes" } else { "no" }, light
                    );
                }
            }
        }
        println!("arm={arm} SUMMARY alive={alive} senescent={senescent} gone={dead} of {}", plants.len());
    }
}

// ---------------------------------------------------------------------------
// mode=fork
// ---------------------------------------------------------------------------

#[derive(Default, Debug, Clone, Copy)]
struct Footprint {
    ants: usize,
    plants: usize,
    plant_cells: usize,
    packed_above: usize,
    packed_below: usize,
    soil_above: usize,
    /// Litter and other decomposing matter standing above the original
    /// surface -- the drift the plants make, still rotting. Counted apart
    /// from `soil_above` because only 5% of a litter cell ever becomes soil
    /// (`litter.ron`'s `decay_yield`), so the drift is litter for most of
    /// its life and litter has no `packs_into`.
    litter_above: usize,
    mound_high: i32,
    roofed: usize,
    pit: usize,
    bare_in_band: usize,
    band_cols: usize,
    plant_cells_in_band: usize,
    touching: usize,
    enclosed: usize,
    hanging: usize,
    hanging_pieces: usize,
    /// Mean held water, as a fraction of `water_capacity`, over packed cells
    /// above the original surface -- the mound -- and over loose soil in the
    /// top four rows of the bank, which is where a seed germinates. Round 28
    /// found the garden loop blocked on *water* at the nest patch, not light,
    /// so whether a mound is rootable ground is a question about this
    /// number before it is a question about anything else.
    mound_wet: f32,
    bank_wet: f32,
}

/// `spoil` rides beside `packed` rather than replacing it: `packedsoil` is
/// the wall an ant cut in place and `spoil` the pellet it hauled out
/// (`assets/materials/spoil.ron`, §Z18), both are tamped, and a census of
/// worked ground above the surface is about both. A single id here would have
/// read the mound as empty from 2026-09-13 on -- the silent-zero failure this
/// harness's own `hanging` column exists to catch.
fn footprint(world: &World, spec: &LabBox, nest_cols: &[i32], packed: Option<MaterialId>, spoil: Option<MaterialId>, soil: Option<MaterialId>) -> Footprint {
    let is_packed = |m: pixel_physics::sim::material::MaterialId| Some(m) == packed || Some(m) == spoil;
    let mut f = Footprint::default();
    let rotting: Vec<MaterialId> = ["litter", "deadleaf", "deadwood", "ash", "corpse", "log", "windfall"]
        .iter()
        .filter_map(|n| world.materials.id_of(n))
        .collect();
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        if world.species.get(st.species).creature.is_some() {
            f.ants += 1;
        } else {
            f.plants += 1;
        }
    }
    let mut has_plant = vec![false; spec.width as usize];
    let mut plant_cells = vec![0usize; spec.width as usize];
    let (mut mound_wet, mut mound_n, mut bank_wet, mut bank_n) = (0.0f64, 0usize, 0.0f64, 0usize);
    for x in 0..spec.width {
        let mut covered = false;
        for y in 0..spec.height {
            let cell = world.get(x, y);
            let ground = is_ground(world, cell);
            if y < spec.ground_y && y >= spec.ground_y - MOUND_REACH && rotting.contains(&cell.material) {
                f.litter_above += 1;
            }
            if is_plant_cell(world, cell) {
                has_plant[x as usize] = true;
                plant_cells[x as usize] += 1;
                f.plant_cells += 1;
            }
            // Engulfment is a question about the *shoot*: every root cell is
            // enclosed by ground by definition, and the first run of this
            // read 4,523 "enclosed" plant cells on a bed whose mound was 88
            // cells -- all of them roots.
            if is_plant_cell(world, cell) && y < spec.ground_y {
                let mut packed_n = 0usize;
                let mut filled = 0usize;
                for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                    let n = world.get(x + dx, y + dy);
                    if n.material != material::EMPTY {
                        filled += 1;
                    }
                    if is_packed(n.material) {
                        packed_n += 1;
                    }
                }
                if packed_n > 0 {
                    f.touching += 1;
                }
                if filled == 8 {
                    f.enclosed += 1;
                }
            }
            if ground && y >= spec.ground_y - MOUND_REACH {
                covered = true;
                let cap = world.materials.get(cell.material).water_capacity;
                if y < spec.ground_y {
                    if is_packed(cell.material) {
                        f.packed_above += 1;
                        f.mound_high = f.mound_high.max(spec.ground_y - y);
                        if cap > 0 {
                            mound_wet += soil_moisture(cell) as f64 / cap as f64;
                            mound_n += 1;
                        }
                    } else if Some(cell.material) == soil {
                        f.soil_above += 1;
                        f.mound_high = f.mound_high.max(spec.ground_y - y);
                    }
                } else if is_packed(cell.material) {
                    f.packed_below += 1;
                } else if Some(cell.material) == soil && y < spec.ground_y + 4 && cap > 0 {
                    bank_wet += soil_moisture(cell) as f64 / cap as f64;
                    bank_n += 1;
                }
            } else if y >= spec.ground_y && world.is_empty(x, y) {
                if covered {
                    f.roofed += 1;
                } else {
                    f.pit += 1;
                }
            }
        }
    }
    f.mound_wet = if mound_n > 0 { (mound_wet / mound_n as f64) as f32 } else { f32::NAN };
    f.bank_wet = if bank_n > 0 { (bank_wet / bank_n as f64) as f32 } else { f32::NAN };
    for x in 0..spec.width {
        let d = nest_cols.iter().map(|c| (c - x).abs()).min().unwrap_or(i32::MAX);
        if d <= BAND {
            f.band_cols += 1;
            f.plant_cells_in_band += plant_cells[x as usize];
            if !has_plant[x as usize] {
                f.bare_in_band += 1;
            }
        }
    }
    // **Hanging spoil: packed cells above the surface with no path down to
    // the bank through other ground.** Flood from every ground cell on the
    // original surface row upward through 4-connected ground; whatever
    // packed cell above the surface the flood never reaches is standing on
    // nothing. Plant cells are not ground here on purpose: spoil posted
    // into a canopy is held up by leaves, and that is exactly the
    // engulfing the owner is asking about, not a wall.
    let top = (spec.ground_y - MOUND_REACH).max(0);
    let w = spec.width as usize;
    let rows = (spec.ground_y - top + 1) as usize;
    let mut seen = vec![false; w * rows];
    let idx = |x: i32, y: i32| (y - top) as usize * w + x as usize;
    let mut stack: Vec<(i32, i32)> = Vec::new();
    for x in 0..spec.width {
        if is_ground(world, world.get(x, spec.ground_y)) {
            seen[idx(x, spec.ground_y)] = true;
            stack.push((x, spec.ground_y));
        }
    }
    while let Some((x, y)) = stack.pop() {
        for (dx, dy) in [(0, -1), (-1, 0), (1, 0), (0, 1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || nx >= spec.width || ny < top || ny > spec.ground_y || seen[idx(nx, ny)] {
                continue;
            }
            if is_ground(world, world.get(nx, ny)) {
                seen[idx(nx, ny)] = true;
                stack.push((nx, ny));
            }
        }
    }
    let mut piece_seen = vec![false; w * rows];
    for y in top..spec.ground_y {
        for x in 0..spec.width {
            let cell = world.get(x, y);
            if !is_packed(cell.material) || seen[idx(x, y)] {
                continue;
            }
            f.hanging += 1;
            if piece_seen[idx(x, y)] {
                continue;
            }
            f.hanging_pieces += 1;
            piece_seen[idx(x, y)] = true;
            let mut st = vec![(x, y)];
            while let Some((px, py)) = st.pop() {
                for (dx, dy) in [(0, -1), (-1, 0), (1, 0), (0, 1)] {
                    let (nx, ny) = (px + dx, py + dy);
                    if nx < 0 || nx >= spec.width || ny < top || ny >= spec.ground_y || piece_seen[idx(nx, ny)] {
                        continue;
                    }
                    if is_packed(world.get(nx, ny).material) && !seen[idx(nx, ny)] {
                        piece_seen[idx(nx, ny)] = true;
                        st.push((nx, ny));
                    }
                }
            }
        }
    }
    f
}

/// Where the colony's ground actually is, by 32-column bin: packed cells
/// above and below the surface and roofed void. Printed because the first
/// tight render of this harness was centred on the founding column and
/// contained none of the 250 roofed cells it had just counted -- a colony
/// digs where its ants are, not where it was founded.
fn profile(world: &World, spec: &LabBox, packed: Option<MaterialId>, spoil: Option<MaterialId>) -> (Vec<(i32, usize, usize, usize)>, i32) {
    let bins = (spec.width / 32) as usize;
    let mut rows = vec![(0i32, 0usize, 0usize, 0usize); bins];
    let (mut sum_x, mut n) = (0i64, 0i64);
    for x in 0..spec.width {
        let b = (x / 32) as usize;
        rows[b].0 = (x / 32) * 32;
        let mut covered = false;
        for y in (spec.ground_y - MOUND_REACH).max(0)..spec.height {
            let cell = world.get(x, y);
            if is_ground(world, cell) {
                covered = true;
                if Some(cell.material) == packed || Some(cell.material) == spoil {
                    if y < spec.ground_y {
                        rows[b].1 += 1;
                    } else {
                        rows[b].2 += 1;
                    }
                    sum_x += x as i64;
                    n += 1;
                }
            } else if y >= spec.ground_y && world.is_empty(x, y) && covered {
                rows[b].3 += 1;
            }
        }
    }
    let centroid = if n > 0 { (sum_x / n) as i32 } else { spec.width / 2 };
    (rows, centroid)
}

fn print_profile(world: &World, spec: &LabBox, packed: Option<MaterialId>, spoil: Option<MaterialId>) -> i32 {
    let (rows, centroid) = profile(world, spec, packed, spoil);
    let line: Vec<String> = rows.iter().filter(|r| r.1 + r.2 + r.3 > 0).map(|r| format!("x{}:{}^/{}</{}v", r.0, r.1, r.2, r.3)).collect();
    println!("  packed and roofed by 32-column bin (above^/below</roofed v): {}  -- packed centroid x={centroid}", line.join(" "));
    centroid
}

fn print_footprint(label: &str, f: &Footprint) {
    println!(
        "{label:<22} ants {:>4} plants {:>4} pcells {:>6} | pack^ {:>5} soil^ {:>5} litter^ {:>5} high {:>3} pack< {:>5} roofed {:>5} pit {:>4} | bare {:>3}/{:<3} pcIn {:>5} | shoot touching {:>5} enclosed {:>5} | hanging {:>4} in {:>3} piece(s) | wet mound {:.2} bank {:.2}",
        f.ants, f.plants, f.plant_cells, f.packed_above, f.soil_above, f.litter_above, f.mound_high, f.packed_below, f.roofed, f.pit,
        f.bare_in_band, f.band_cols, f.plant_cells_in_band, f.touching, f.enclosed, f.hanging, f.hanging_pieces, f.mound_wet, f.bank_wet
    );
}

/// A cheap deterministic per-cell roll for the staged `weather` arm --
/// keyed on position and frame so two runs of one binary agree, and
/// independent of the world's own streams so staging it here perturbs
/// nothing the arms share.
fn roll(x: i32, y: i32, frame: u64) -> f32 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ frame.wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 32;
    (h & 0xFF_FFFF) as f32 / 16_777_216.0
}

fn loosen(world: &mut World, spec: &LabBox, packed: MaterialId, soil: MaterialId, exposed_only: bool, chance: f32, frame: u64) -> usize {
    let mut changed = 0usize;
    let top = (spec.ground_y - MOUND_REACH).max(0);
    let mut sites: Vec<(i32, i32)> = Vec::new();
    for y in top..spec.ground_y {
        for x in 0..spec.width {
            let cell = world.get(x, y);
            if cell.material != packed {
                continue;
            }
            if exposed_only {
                let exposed = [(0, -1), (-1, 0), (1, 0), (0, 1)].iter().any(|(dx, dy)| world.is_empty(x + dx, y + dy));
                if !exposed || roll(x, y, frame) >= chance {
                    continue;
                }
            }
            sites.push((x, y));
        }
    }
    for (x, y) in sites {
        // Everything but the material rides across -- the pellet's moisture,
        // shade and heat -- for the reason `update.rs`'s crumb rule gives:
        // on a `Powder`, `aux == 0` means dry, so a rebuilt cell is
        // destroyed water.
        let mut c = world.get(x, y);
        c.material = soil;
        world.set(x, y, c);
        changed += 1;
    }
    changed
}

/// Draw the crop as the game draws it, and -- when `tint` is set -- a second
/// copy with the two soils painted apart: packed soil (what the colony
/// made) in orange, loose soil standing above the original surface (what
/// rot made -- litter and bodies weathered back to ground -- or what
/// weathering made of the spoil) in magenta. A
/// full replace, not a blend, per `CLAUDE.md`'s overlay rule: at play zoom
/// worked ground is a slightly darker brown on brown and reads as nothing,
/// which is how a 541-cell mound came back from the first render of this
/// harness as an ordinary bank.
fn render(world: &mut World, spec: &LabBox, crop: (i32, i32, i32, i32), zoom: u32, out: &str, tint: Option<(MaterialId, MaterialId)>) {
    let (vw, vh) = (spec.width as u32, spec.height as u32);
    let mut renderer = Renderer::new();
    renderer.creature_colour = pixel_physics::render::CreatureColour::Colony;
    let particles = ParticleSystem::new();
    let mut buf = vec![0u8; (vw * vh * 4) as usize];
    let touched = world.take_touched_chunks();
    renderer.draw(world, &particles, &touched, &mut buf, (vw, vh), true);
    let (cx, cy, cw, ch) = crop;
    let (ow, oh) = (cw as u32 * zoom, ch as u32 * zoom);
    let passes: Vec<(String, bool)> = match tint {
        Some(_) => vec![(out.to_string(), false), (out.replace(".png", "-tint.png"), true)],
        None => vec![(out.to_string(), false)],
    };
    for (path, tinted) in passes {
        let mut img = vec![0u8; (ow * oh * 4) as usize];
        for oy in 0..oh {
            for ox in 0..ow {
                let wx = (cx + (ox / zoom) as i32).clamp(0, spec.width - 1);
                let wy = (cy + (oy / zoom) as i32).clamp(0, spec.height - 1);
                let s = ((wy as u32 * vw + wx as u32) * 4) as usize;
                let d = ((oy * ow + ox) * 4) as usize;
                img[d..d + 4].copy_from_slice(&buf[s..s + 4]);
                if let (true, Some((packed, soil))) = (tinted, tint) {
                    let m = world.get(wx, wy).material;
                    if m == packed {
                        img[d..d + 4].copy_from_slice(&[255, 128, 0, 255]);
                    } else if m == soil && wy < spec.ground_y {
                        // Magenta, not yellow: the lab's first colony wears
                        // yellow, and a 3,000-ant heap in yellow beside
                        // loose soil in yellow was one blob on the first card.
                        img[d..d + 4].copy_from_slice(&[255, 0, 200, 255]);
                    }
                }
            }
        }
        image::save_buffer(&path, &img, ow, oh, image::ColorType::Rgba8).expect("writing the render");
        println!("  wrote {path} ({ow}x{oh}, crop {cx},{cy},{cw},{ch} at {zoom}x{})", if tinted { ", two soils tinted" } else { "" });
    }
}

fn fork() {
    let frames: u64 = arg("frames").unwrap_or(100_000);
    let after: u64 = arg("after").unwrap_or(6_000);
    let zoom: u32 = arg("zoom").unwrap_or(5);
    let out: String = arg("out").unwrap_or_else(|| "soilfork".to_string());
    let scenario = load_bed();
    let spec = scenario.bed.clone();
    let nest_cols: Vec<i32> = {
        let mut v: Vec<i32> = scenario
            .placements
            .iter()
            .chain(scenario.timeline.iter().map(|e| &e.what))
            .filter_map(|p| match p {
                Placement::Colony { x, .. } => Some(*x),
                _ => None,
            })
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    // Render about twice the region of interest (the review skill's
    // "render wide, declare tight"): the nest with its mound and chambers is
    // the middle half by area, and the band either side is the margin.
    // 384x200 at 3x was the first cut and the nest could not be found in it
    // at all -- a colony's whole footprint is a few dozen cells across.
    let crop_arg: Option<(i32, i32, i32, i32)> = arg::<String>("crop").map(|s| {
        let v: Vec<i32> = s.split(',').map(|p| p.trim().parse().expect("crop wants x,y,w,h")).collect();
        assert_eq!(v.len(), 4, "crop wants exactly x,y,w,h");
        (v[0], v[1], v[2], v[3])
    });
    println!(
        "soilfork fork: scenario={} seed={} frames={frames} after={after} nests={nest_cols:?} crop={} zoom={zoom} threads={}",
        scenario.name,
        spec.seed,
        crop_arg.map_or("auto (centred on the packed cells)".to_string(), |c| format!("{c:?}")),
        std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "default".into())
    );
    let (mut world, planted, placed) = scenario.build();
    println!(
        "  bed: {} of {} founders planted; scenario placed {} cells, {} plants, {} animals",
        planted.planted, planted.asked, placed.cells, placed.plants, placed.animals
    );
    let packed = world.materials.id_of("packedsoil");
    // **The hauled pellet, beside the wall cut in place** -- see `footprint`.
    let spoil = world.materials.id_of("spoil");
    let soil = world.materials.id_of("soil");
    let t = std::time::Instant::now();
    run(&mut world, frames, Some(&scenario), &spec);
    println!("  ran {frames} frames in {:.1} s ({:.2} ms/frame)", t.elapsed().as_secs_f64(), t.elapsed().as_secs_f64() * 1000.0 / frames.max(1) as f64);
    let st = world.creature_stats;
    println!("  colony so far: digs {} spoil_dumped {} eats {} births {} deaths {}", st.digs, st.spoil_dumped, st.eats, st.births, st.deaths);
    let before = footprint(&world, &spec, &nest_cols, packed, spoil, soil);
    print_footprint(&format!("frame {frames} (shared)"), &before);
    let centroid = print_profile(&world, &spec, packed, spoil);
    // The crop is fixed at the fork so every arm shows the same cells.
    let crop = crop_arg.unwrap_or((centroid.clamp(96, spec.width - 96) - 96, spec.ground_y - 70, 192, 120));
    let shared = world.clone();
    let tint = packed.zip(soil);
    render(&mut world, &spec, crop, zoom, &format!("{out}-shared.png"), tint);

    let arms: Vec<String> =
        arg::<String>("arms").unwrap_or_else(|| "keep,loose,weather,weather_slow".to_string()).split(',').map(|s| s.to_string()).collect();
    // `weather=` is the per-check chance; `weather_slow` runs at a tenth of
    // it, so one run brackets the dial.
    let weather_chance: f32 = arg("weather").unwrap_or(WEATHER_CHANCE);
    for arm in &arms {
        let mut w = shared.clone();
        let (Some(packed), Some(soil)) = (packed, soil) else {
            eprintln!("soilfork: no packedsoil or soil material in this world");
            std::process::exit(2);
        };
        let t = std::time::Instant::now();
        let mut changed = 0usize;
        match arm.as_str() {
            "keep" => run(&mut w, after, Some(&scenario), &spec),
            "loose" => {
                changed = loosen(&mut w, &spec, packed, soil, false, 1.0, frames);
                run(&mut w, after, Some(&scenario), &spec);
            }
            "weather" | "weather_slow" => {
                let chance = if arm == "weather" { weather_chance } else { weather_chance / 10.0 };
                let mut done = 0u64;
                while done < after {
                    let step = WEATHER_TICK.min(after - done);
                    run(&mut w, step, Some(&scenario), &spec);
                    done += step;
                    changed += loosen(&mut w, &spec, packed, soil, true, chance, frames + done);
                }
            }
            other => {
                eprintln!("soilfork: arm {other} is not keep, loose, weather or weather_slow");
                std::process::exit(2);
            }
        }
        println!(
            "\narm={arm}: loosened {changed} packed cell(s){}, ran {after} more frames in {:.1} s",
            match arm.as_str() {
                "weather" => format!(" at {weather_chance} per {WEATHER_TICK}-frame check"),
                "weather_slow" => format!(" at {} per {WEATHER_TICK}-frame check", weather_chance / 10.0),
                _ => String::new(),
            },
            t.elapsed().as_secs_f64()
        );
        let f = footprint(&w, &spec, &nest_cols, Some(packed), spoil, Some(soil));
        print_footprint(&format!("arm={arm} +{after}"), &f);
        print_profile(&w, &spec, Some(packed), spoil);
        render(&mut w, &spec, crop, zoom, &format!("{out}-{arm}.png"), tint);
    }
}
