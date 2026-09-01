//! **How much resolution can the evolution lab afford?** — the cost of a lab
//! box at 1x, 2x and 4x linear cell resolution, and what `field::FIELD_SCALE`
//! is worth when the box grows.
//!
//! The owner's question is *"could we have a different game resolution and
//! field resolution — some of these things could be a little more coarse"*.
//! `Reports/resolution-step-2026-08-29.md` §"What is left" predicts
//! `FIELD_SCALE` 8 -> 16 makes the field "~4x cheaper" and nobody had
//! measured it. This is the instrument for that.
//!
//! # Why this is not `labbox_cost`
//!
//! `labbox_cost` builds a `PlantScene` — an **open-topped** bed with a held
//! sky. The lab's own bed is `lab::scene::LabBox`, which has a stone
//! **ceiling**, and its own source says the ceiling "is why the field does not
//! have to solve every tile every frame". Those are different field
//! workloads, so the resolution question has to be asked of the real bed.
//! This calls `LabBox::build` rather than a private copy, so what it measures
//! is the game's bed.
//!
//! It also scales `ground_y` with `height`, which `labshot` and `labbox_cost`
//! do not expose: at `height=640` with the default `ground_y=160` the soil
//! sits in the top quarter and 390 rows of the box are empty void below the
//! floor. That is a scene error, and a scene that contradicts the code looks
//! exactly like a bug in the code (`CLAUDE.md`).
//!
//! # The counter that actually moves, and the one that does not
//!
//! **`solved/f` counts `FieldTile`s, and there is exactly one `FieldTile` per
//! `Chunk` at any `FIELD_SCALE`.** So `solved/f` is *invariant* under a
//! `FIELD_SCALE` change by construction — reading it as "the work did not
//! move" would be wrong, and reading it as "the work vanished" would be
//! wrong too. The quantity that moves is **field cells solved per frame**,
//! `solved/f * FIELD_TILE_AREA`, and it is printed beside it. This is
//! `CLAUDE.md`'s *ask what your number counts when nothing is wrong* applied
//! before the run rather than after it.
//!
//! Every line stamps `fs=<FIELD_SCALE>` so two builds' logs cannot be
//! confused — the harness is as stale-able as the constant it was compiled
//! with, and there is no other way to tell the binaries apart.
//!
//! ```text
//! cargo run --release --example lab_resolution -- width=512 height=320 soil=40 founders=8
//! cargo run --release --example lab_resolution -- mode=render width=512 height=320
//! cargo run --release --example lab_resolution -- mode=shot out=lab_fs8.png at=6000
//! cargo run --release --example lab_resolution -- fans=1        # the positive control
//! ```

use pixel_physics::lab::scene::LabBox;
use pixel_physics::render::{FieldOverlay, OrganismOverlay, Renderer};
use pixel_physics::sim::chunk::{CHUNK_SIZE, MAX_REACH};
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::particle::{self, ParticleSystem};
use pixel_physics::sim::world::World;
use pixel_physics::sim::{field, parallel, player, rigid};
use std::time::Instant;

/// The frame's phases, in `frame::step`'s order. `chunk_bodies` carries the
/// liquid-body and player phases too — both measured at ~0.001 ms in a sealed
/// box (feasibility §3c) — so this splits the same tick `labbox_cost` does
/// and the two are comparable row for row.
const PHASES: [&str; 6] =
    ["ca_sweep", "chunk_bodies", "active_sites", "particles", "field", "pheromones"];

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses"))
    })
}

/// Best of N. Every source of noise on this box only ever *adds* time, so the
/// minimum is the closest thing to the true cost — the same reasoning
/// `render_cost::best_of` and `subpixel_cost::best_of` use, mirrored so the
/// three are comparable.
fn best_of(runs: usize, mut f: impl FnMut() -> usize) -> (f64, usize) {
    let mut best = f64::INFINITY;
    let mut check = 0;
    for _ in 0..runs {
        let t = Instant::now();
        check = f();
        best = best.min(t.elapsed().as_secs_f64() * 1000.0);
    }
    (best, check)
}

fn spec_from_args() -> LabBox {
    let width: i32 = arg("width").unwrap_or(512);
    let height: i32 = arg("height").unwrap_or(320);
    // **Defaults derived from `height`, not copied from `LabBox::default`.**
    // The shipped bed is 512x320 with `ground_y` 160 and 80 rows of soil, i.e.
    // ground at half height and soil at a quarter of it. Holding those ratios
    // is what makes a 1024x640 box *the same physical bed at twice the
    // resolution* rather than a different scene.
    LabBox {
        width,
        height,
        ground_y: arg("ground").unwrap_or(height / 2),
        soil_depth: arg("soil").unwrap_or(height / 4),
        founders: arg("founders").unwrap_or(8),
        colonies: arg("colonies").unwrap_or(1),
        // **Not scaled with the box, unlike every length above it.** A count
        // is not a length: the same fifty-two animals in a box of twice the
        // area is the resolution question this harness asks, where twice as
        // many would be a different experiment wearing the same name.
        colony_ants: arg("ants").unwrap_or(pixel_physics::sim::creature::COLONY_ANTS),
        colony_species: arg("colony_species").unwrap_or_else(|| "ant".to_string()),
        predators: arg("predators").unwrap_or(0),
        compartments: arg("walls").unwrap_or(1),
        // Scaled with `width` for the same reason `ground_y` and `soil_depth`
        // are scaled with `height`: a fixture every 128 cells is one fixture
        // every quarter-width on the shipped 512 bed, and holding *that* is
        // what keeps a bigger box the same physical room. Copying the literal
        // 128 would light a 2048-wide box four times as densely and the
        // resolution ladder would be measuring the lamps.
        lamp_spacing: arg("lamps").unwrap_or(width / 4),
        species: arg("species").unwrap_or_else(|| "herb".to_string()),
        seed: arg("seed").unwrap_or(1),
        // None: this harness varies geometry, and a hand-placed wall would be
        // a second axis in a ladder that exists to isolate one.
        extra_walls: Vec::new(),
        // **Exhaustive on purpose -- no `..LabBox::default()`.** Two lanes
        // repaired the same red build here at once, one by adding
        // `lamp_spacing` and one by adding a struct update, and together they
        // tripped `needless_update`. The exhaustive form is the one to keep,
        // and the house precedent is `WorldgenParams::scaled`, which
        // destructures every field with no `..` for exactly this reason: a
        // function whose job is to *scale a scene* must classify each new
        // field as scaled or held, and a struct update answers that question
        // silently and usually wrongly. `lamp_spacing` is the proof -- the
        // shipped literal 128 inherited into a 2048-wide box lights it four
        // times as densely, and the resolution ladder would have been
        // measuring the lamps. So a new `LabBox` field should break this
        // build; that is the compiler asking a question only a person can
        // answer.
    }
}

fn geometry_line() -> String {
    format!(
        "fs={} tile={}x{} area={} chunk={} reach={}",
        field::FIELD_SCALE,
        field::FIELD_TILE_SIZE,
        field::FIELD_TILE_SIZE,
        field::FIELD_TILE_AREA,
        CHUNK_SIZE,
        MAX_REACH
    )
}

fn census(world: &World) -> (usize, usize, u32, usize) {
    let ids = world.live_organism_ids();
    let cells: usize =
        ids.iter().filter_map(|id| world.organism(*id)).map(|s| s.cells.len()).sum();
    let seeds: u32 =
        ids.iter().filter_map(|id| world.organism(*id)).map(|s| s.seeds_set).sum();
    (cells, ids.len(), seeds, world.live_creature_count())
}

fn main() {
    let mode: String = arg("mode").unwrap_or_else(|| "cost".to_string());
    let spec = spec_from_args();
    println!(
        "lab_resolution: mode={mode} {} box={}x{} ground={} soil={} founders={} colonies={} walls={} species={} seed={}",
        geometry_line(),
        spec.width,
        spec.height,
        spec.ground_y,
        spec.soil_depth,
        spec.founders,
        spec.colonies,
        spec.compartments,
        spec.species,
        spec.seed,
    );
    match mode.as_str() {
        "render" => render_mode(&spec),
        "shot" => shot_mode(&spec),
        _ => cost_mode(&spec),
    }
}

// --- cost -------------------------------------------------------------------

fn cost_mode(spec: &LabBox) {
    let frames: u64 = arg("frames").unwrap_or(3_000);
    let warm: u64 = arg("warm").unwrap_or(300);
    // **`fans=N` is the positive control.** A fan is `World::add_pressure_
    // impulse`, which `labbox_cost` measured as waking every tile in the box —
    // so it is the case whose answer is known to be non-zero, and if the field
    // cost does not move between two `FIELD_SCALE` builds *here*, the
    // instrument is wrong rather than the engine.
    let fans: usize = arg("fans").unwrap_or(0);
    let fan_radius: i32 = arg("fan_radius").unwrap_or(12);
    let fan_force: f32 = arg("fan_force").unwrap_or(0.6);
    println!("  frames={frames} warm={warm} fans={fans} fan_radius={fan_radius} fan_force={fan_force}");

    let mut world = spec.build();
    let mut particles = ParticleSystem::default();
    let mut blasts = Blasts::default();
    let tuning = player::Tuning::default();
    let mut totals: Vec<f64> = Vec::with_capacity(frames as usize);
    let mut phase_sums = [0.0f64; PHASES.len()];
    let mut solved = 0u64;
    let mut awake = 0u64;
    let mut counted = 0u64;

    for f in 0..(warm + frames) {
        let record = f >= warm;
        if record {
            world.field_stats.tiles_solved = 0;
        }
        for i in 0..fans {
            // Offset by a third of a spacing so a fan never sits exactly on a
            // partition — `labbox_cost` records that scene error.
            let spacing = spec.width / (fans as i32 + 1);
            let x = spacing * (i as i32 + 1) + spacing / 3;
            let y = spec.ground_y - spec.height / 13;
            world.add_pressure_impulse(x, y, fan_radius, fan_force);
        }
        let mut marks = [Instant::now(); PHASES.len() + 1];
        parallel::step(&mut world);
        marks[1] = Instant::now();
        world.step_liquid_bodies();
        rigid::step_chunk_bodies(&mut world);
        player::step(&mut world, player::PlayerInput::default(), &tuning);
        marks[2] = Instant::now();
        world.step_active_sites();
        marks[3] = Instant::now();
        blasts.step(&mut world, &mut particles);
        particle::throw_splashes(&mut world, &mut particles);
        particles.step(&mut world);
        marks[4] = Instant::now();
        world.step_fields();
        marks[5] = Instant::now();
        world.step_pheromones();
        marks[6] = Instant::now();

        if record {
            let mut total = 0.0;
            for i in 0..PHASES.len() {
                let ms = marks[i + 1].duration_since(marks[i]).as_secs_f64() * 1000.0;
                phase_sums[i] += ms;
                total += ms;
            }
            totals.push(total);
            solved += world.field_stats.tiles_solved;
            awake += world.active_chunk_count() as u64;
            counted += 1;
        }
    }

    let (cells, orgs, seeds, ants) = census(&world);
    totals.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
    let mean = totals.iter().sum::<f64>() / totals.len() as f64;
    let solved_f = solved as f64 / counted as f64;
    let fieldcells_f = solved_f * field::FIELD_TILE_AREA as f64;
    let boxcells = (spec.width as f64) * (spec.height as f64);

    println!(
        "\n{:>4}  {:>9}  {:>9}  {:>9}  {:>9}  {:>8}  {:>10}  {:>8}  {:>7}  {:>6}  {:>6}  {:>5}  {:>7}",
        "fs", "mean", "p50", "worst", "field", "solved/f", "fcells/f", "awake/f", "x live", "cells",
        "orgs", "seeds", "us/cell"
    );
    println!(
        "{:>4}  {:>7.3}ms  {:>7.3}ms  {:>7.3}ms  {:>7.3}ms  {:>8.1}  {:>10.0}  {:>8.1}  {:>6.2}x  {:>6}  {:>6}  {:>5}  {:>7.2}",
        field::FIELD_SCALE,
        mean,
        totals[totals.len() / 2],
        totals[totals.len() - 1],
        phase_sums[4] / counted as f64,
        solved_f,
        fieldcells_f,
        awake as f64 / counted as f64,
        1000.0 / mean / 60.0,
        cells,
        orgs,
        seeds,
        if cells > 0 { mean * 1000.0 / cells as f64 } else { f64::NAN },
    );
    println!("  ants alive {ants}, box cells {boxcells:.0}, ns per box cell {:.1}", mean * 1e6 / boxcells);
    print!("  per-phase mean ms:");
    for (i, p) in PHASES.iter().enumerate() {
        print!("  {p}={:.3}", phase_sums[i] / counted as f64);
    }
    println!();
    println!("  field share of frame {:.1}%", 100.0 * phase_sums[4] / counted as f64 / mean);
}

// --- render (leg 5) ---------------------------------------------------------

/// What the lab costs to *draw* at 1 and 2 screen pixels per cell.
///
/// The visible region is asserted identical across arms, not assumed: a
/// supersample that quietly shows a different amount of world is measuring the
/// wrong thing and would look exactly like a result (`subpixel_cost`'s rule,
/// reused).
fn render_mode(spec: &LabBox) {
    let warm: u64 = arg("warm").unwrap_or(3_000);
    let runs: usize = arg("runs").unwrap_or(20);
    println!("  warm={warm} runs={runs} (best-of)");
    let mut world = spec.build();
    let mut particles = ParticleSystem::default();
    let mut blasts = Blasts::default();
    let tuning = player::Tuning::default();
    for _ in 0..warm {
        pixel_physics::sim::frame::step(
            &mut world,
            &mut particles,
            &mut blasts,
            player::PlayerInput::default(),
            &tuning,
        );
    }
    let (cells, orgs, seeds, ants) = census(&world);
    println!("  stand at frame {warm}: cells {cells} orgs {orgs} seeds {seeds} ants {ants}");

    let touched = std::collections::HashSet::new();
    let mut renderer = Renderer::new();
    println!(
        "\n{:>8}  {:>12}  {:>11}  {:>9}  {:>8}  {:>8}",
        "px/cell", "buffer", "pixels", "ms", "vs 1x", "ns/px"
    );
    let mut base_ms = 0.0;
    let mut base_region = (0, 0, 0, 0);
    for zoom in [1, 2, 3] {
        let (w, h) = (spec.width as u32 * zoom as u32, spec.height as u32 * zoom as u32);
        let mut buf = vec![0u8; (w * h * 4) as usize];
        renderer.zoom = zoom;
        let (x0, y0) = renderer.screen_to_world(0, 0);
        let (x1, y1) = renderer.screen_to_world(w as i32 - 1, h as i32 - 1);
        if zoom == 1 {
            base_region = (x0, y0, x1, y1);
        } else {
            assert_eq!(
                (x0, y0, x1, y1),
                base_region,
                "the visible region moved between arms — this is measuring content, not pixels"
            );
        }
        let (ms, drawn) =
            best_of(runs, || renderer.draw(&world, &particles, &touched, &mut buf, (w, h), true));
        if zoom == 1 {
            base_ms = ms;
        }
        println!(
            "{:>8}  {:>12}  {:>11}  {:>7.3}ms  {:>7.2}x  {:>8.1}   (drew {drawn})",
            zoom,
            format!("{w}x{h}"),
            w * h,
            ms,
            ms / base_ms,
            ms * 1e6 / (w * h) as f64,
        );
    }
    println!("  region {base_region:?} held across all arms");
}

// --- shot (leg 4) -----------------------------------------------------------

/// One PNG of the bed, so `FIELD_SCALE` 8 and 16 can be laid side by side.
/// `overlay=light` repaints it through `render.rs`'s light channel on a fixed
/// dark->bright ramp, which is the only way to see whether the *shade* got
/// coarser: at unchanged content the ordinary picture may hide it entirely.
fn shot_mode(spec: &LabBox) {
    let out: String = arg("out").unwrap_or_else(|| "lab_resolution.png".to_string());
    let at: u64 = arg("at").unwrap_or(6_000);
    let zoom: i32 = arg("zoom").unwrap_or(1);
    let overlay: String = arg("overlay").unwrap_or_else(|| "off".to_string());
    let organism: String = arg("organism").unwrap_or_else(|| "off".to_string());
    println!("  out={out} at={at} zoom={zoom} overlay={overlay} organism={organism}");
    let mut world = spec.build();
    let mut particles = ParticleSystem::default();
    let mut blasts = Blasts::default();
    let tuning = player::Tuning::default();
    for _ in 0..at {
        pixel_physics::sim::frame::step(
            &mut world,
            &mut particles,
            &mut blasts,
            player::PlayerInput::default(),
            &tuning,
        );
    }
    let (cells, orgs, seeds, ants) = census(&world);
    println!("  stand at frame {at}: cells {cells} orgs {orgs} seeds {seeds} ants {ants}");
    // **The count beside the picture.** `CLAUDE.md`'s standing rule: an
    // image says what and where, and only a number says whether. A health
    // sheet where every plant happens to be fed and one where the channel
    // is not firing at all are the same photograph.
    {
        let (mut fed, mut starving, mut thirsty) = (0usize, 0usize, 0usize);
        for id in world.live_organism_ids() {
            let Some(st) = world.organism(id) else { continue };
            if world.species.get(st.species).creature.is_some() {
                continue;
            }
            // The same test the overlay uses -- `maintenance > income`, the
            // plant's own book, never the sum of its cells' shortfalls.
            let margin = if st.maintenance > f32::EPSILON {
                st.income * pixel_physics::sim::plant::MEAN_NIGHT_INCOME_FACTOR / st.maintenance
            } else {
                f32::INFINITY
            };
            if margin < 1.0 || st.starving_ticks > 0 {
                starving += 1;
            } else {
                fed += 1;
                if st.water_status < 0.75 {
                    thirsty += 1;
                }
            }
        }
        println!("  plant health: {fed} paying their upkeep ({thirsty} of them short of water), {starving} not");
        // **Split by size, because the headline is dominated by seedlings.**
        // A seed that germinated this minute has no leaf yet and cannot pay
        // anything; counting it beside a grown plant reports a healthy bed
        // as a dying one. `CLAUDE.md`: ask what the number counts when
        // nothing is wrong.
        let (mut est, mut est_starving) = (0usize, 0usize);
        for id in world.live_organism_ids() {
            let Some(st) = world.organism(id) else { continue };
            if world.species.get(st.species).creature.is_some() || st.cells.len() < 20 {
                continue;
            }
            est += 1;
            let margin = if st.maintenance > f32::EPSILON {
                st.income * pixel_physics::sim::plant::MEAN_NIGHT_INCOME_FACTOR / st.maintenance
            } else {
                f32::INFINITY
            };
            if margin < 1.0 || st.starving_ticks > 0 {
                est_starving += 1;
            }
        }
        println!("  of the {est} established (20+ cells): {est_starving} not paying");
        // **The distributions behind the ramp, not just the counts.** The
        // overlay maps `water_status` onto the green ramp and
        // `starving_ticks` onto the red one; if either is degenerate the
        // channel is a two-colour flag wearing a gradient, which is
        // `CLAUDE.md`'s first law ("an outcome is a distribution, not a
        // binary") failing in a readout rather than in a mechanic.
        let q = |mut v: Vec<f32>, name: &str| {
            if v.is_empty() {
                println!("    {name}: none");
                return;
            }
            v.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
            let at = |f: f32| v[((v.len() - 1) as f32 * f) as usize];
            println!(
                "    {name}: min {:.3}  p25 {:.3}  p50 {:.3}  p75 {:.3}  max {:.3}  (n={})",
                v[0], at(0.25), at(0.5), at(0.75), v[v.len() - 1], v.len()
            );
        };
        let (mut wat, mut starv, mut margin) = (Vec::new(), Vec::new(), Vec::new());
        for id in world.live_organism_ids() {
            let Some(st) = world.organism(id) else { continue };
            if world.species.get(st.species).creature.is_some() || st.cells.len() < 20 {
                continue;
            }
            let m = st.income * pixel_physics::sim::plant::MEAN_NIGHT_INCOME_FACTOR / st.maintenance.max(1e-6);
            if m < 1.0 || st.starving_ticks > 0 {
                starv.push(m);
            } else {
                wat.push(((m - 1.0) / 3.0).clamp(0.0, 1.0).min(st.water_status));
            }
            margin.push(m);
        }
        println!("  what the ramp actually sees, over established plants:");
        q(wat, "green ramp input (scarcest resource, payers)");
        q(starv, "margin of failers");
        q(margin, "night-corrected margin (income x 0.49 / upkeep)");
    }

    let (w, h) = (spec.width as u32 * zoom as u32, spec.height as u32 * zoom as u32);
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let mut renderer = Renderer::new();
    renderer.zoom = zoom;
    renderer.field_overlay = match overlay.as_str() {
        "light" => FieldOverlay::Light,
        "moisture" => FieldOverlay::Moisture,
        "temperature" => FieldOverlay::Temperature,
        "pressure" => FieldOverlay::Pressure,
        _ => FieldOverlay::Off,
    };
    // **The `L` channels, which `overlay=` could not reach.** The lab draws
    // two independent overlays -- the field one on `O` and the organism one
    // on `L` -- and this harness only ever set the first, so every question
    // about a plant's own state had to be asked of the outdoor `filmstrip`
    // scene instead of the bed the owner actually plays.
    renderer.organism_overlay = match organism.as_str() {
        "health" => OrganismOverlay::PlantHealth,
        "celltype" => OrganismOverlay::CellType,
        "resource" => OrganismOverlay::Resource,
        "canopy" => OrganismOverlay::CanopyDensity,
        "vein" => OrganismOverlay::VeinConductance,
        "soil" => OrganismOverlay::SoilMoisture,
        "food" => OrganismOverlay::FoodValue,
        "gut" => OrganismOverlay::GutBias,
        "bend" => OrganismOverlay::Stress,
        _ => OrganismOverlay::Off,
    };
    let touched = std::collections::HashSet::new();
    renderer.draw(&world, &particles, &touched, &mut buf, (w, h), true);
    image::save_buffer(&out, &buf, w, h, image::ColorType::Rgba8).expect("writing the shot");
    println!("wrote {out} ({w}x{h}) at {}", geometry_line());
}
