//! **The grassfire instrument.** Why a fire that crosses a strip at 1.34
//! cells/frame crawls across a real meadow at 0.12, and whether moisture
//! gates any of it.
//!
//! `Reports/open-bugs-handoff.md` §G carries the owner's standing verdict on
//! the grassfire card — *"The fire looks bad. Just looks like you are cycling
//! colors. It also doesn't spread at all (if we are going to do this,
//! moisture vs dryness should play a role."* — three separable claims. This
//! harness exists for the middle one, because a contact sheet cannot answer
//! it: a burnt band and a smoke haze are the same grey smear at sheet zoom,
//! and the correction already recorded in `plant-implementation-plan.md`
//! ("what spread across the frame was smoke haze, not the burn") is exactly
//! the mistake a picture invites. So this prints numbers, and the numbers it
//! prints are chosen so a *stalled* front and a *slow* one read differently:
//!
//! - `front` is the rightmost **burnt-or-burning fuel column**, never a gas
//!   cell. Smoke drifts the width of the world in a few hundred frames and
//!   is not the fire.
//! - `alight` is the standing count of burning cells. A front that has gone
//!   out reads `alight 0` with a `front` that then never moves again, which
//!   is a different failure from a front that advances one column a minute.
//! - `gap` is the sward's own geometry, printed *before* the ignition: fire
//!   here spreads by 4-neighbour contact only, so the distribution of empty
//!   columns between tussocks is the hypothesis this instrument was built
//!   to test. A meadow is not a strip.
//!
//! Echoes its own parameters on the first line, per `CLAUDE.md`: a 3.5-hour
//! megastudy was once void because the harness took a default seed nobody
//! could see, and a log that does not name its arguments was written by a
//! binary that may not have had them.
//!
//! ```text
//! cargo build --release --examples          # ALWAYS; assets are include_str!'d
//! cargo run --release --example fire_probe -- moisture=1000 plants=64
//! cargo run --release --example fire_probe -- moisture=180  plants=64
//! ```

use pixel_physics::sim::material;
use pixel_physics::sim::update;
use pixel_physics::sim::world::World;
use pixel_physics::sim::{field, parallel, rigid};

mod common;

struct Args {
    /// Soil wetness the bed is built at, on `SOIL_SATURATED`'s scale
    /// (1000 = saturated, 500 = field capacity, 180 = the wilting point).
    /// This is the dry/wet axis the owner's steer asks for.
    moisture: u16,
    /// Grass founders. Density is the sward's connectivity knob and the
    /// prime suspect for the strip-vs-meadow gap.
    plants: usize,
    /// Frames of growth before anything is lit. A fire scene has to start
    /// burning *after* the vegetation is vegetation.
    grow: usize,
    /// Frames of burning after the ignition.
    frames: usize,
    /// Report interval.
    every: usize,
    /// **Soil wetness the bed is reset to after the sward has grown**, and
    /// the reason this is a separate knob from `moisture`.
    ///
    /// A dry meadow cannot be grown: below `SOIL_WILTING_POINT` nothing
    /// germinates, so growing one arm dry and one arm wet compares two
    /// different swards and calls the difference "moisture". Growing both
    /// at the same wetness and then re-wetting or drying the bed just
    /// before the ignition gives **identical fuel and one variable**,
    /// which is the paired comparison `CLAUDE.md` asks for. `settle`
    /// frames then let the moisture field equilibrate to the new ground
    /// before anything is lit.
    ///
    /// `u16::MAX` means "leave it alone".
    burn_moisture: u16,
    /// Frames between the re-wetting and the ignition, for the field to
    /// catch up with the ground.
    settle: usize,
    /// Ignition column, as a fraction of world width.
    at: f32,
    /// Extra soil rows. Left at the shared scene's default.
    soil: i32,
    start_frame: u64,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            moisture: material::SOIL_FIELD_CAPACITY,
            plants: 64,
            grow: 3_000,
            frames: 2_400,
            every: 300,
            burn_moisture: u16::MAX,
            settle: 600,
            at: 0.1,
            soil: common::SOIL_DEPTH,
            start_frame: 0,
        }
    }
}

fn main() {
    let mut a = Args::default();
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "moisture" => a.moisture = v.parse().expect("moisture"),
            "plants" => a.plants = v.parse().expect("plants"),
            "grow" => a.grow = v.parse().expect("grow"),
            "frames" => a.frames = v.parse().expect("frames"),
            "every" => a.every = v.parse().expect("every"),
            "burnmoisture" => a.burn_moisture = v.parse().expect("burnmoisture"),
            "settle" => a.settle = v.parse().expect("settle"),
            "at" => a.at = v.parse().expect("at"),
            "soil" => a.soil = v.parse().expect("soil"),
            "frame0" => a.start_frame = v.parse().expect("frame0"),
            other => panic!("unknown argument `{other}` -- an ignored argument is how a study silently measures its defaults"),
        }
    }
    println!(
        "fire_probe: moisture={} burnmoisture={} settle={} plants={} grow={} frames={} every={} at={} soil={} frame0={}",
        a.moisture, a.burn_moisture, a.settle, a.plants, a.grow, a.frames, a.every, a.at, a.soil, a.start_frame
    );

    // **Echo the fuel's own constants, not just the command line.** A
    // sweep over `grassblade.ron` that was killed by a timeout before its
    // restore line ran left `flammability` at 0.05, and the next four
    // measurements taken here were of a fuel nobody meant to test -- they
    // read as "the moisture gate is inverted", which is a conclusion, not
    // a typo. `.ron` files are `include_str!`d, so the binary carries the
    // values it was built with and nothing on the command line names them.
    // A run whose header does not print these was written by a binary that
    // predates this line.
    {
        let probe = World::new(pixel_physics::sim::chunk::Rect::new(0, 0, 15, 15));
        let g = probe.materials.get(probe.materials.id_of("grassblade").expect("grassblade"));
        println!(
            "  fuel as built: grassblade flammability={} burn_duration={} flame_chance={} flame_into={}",
            g.flammability,
            g.burn_duration,
            g.flame_chance,
            g.flame_into.map_or("(none)".to_string(), |id| probe.materials.get(id).name.clone()),
        );
    }

    let scene = common::PlantScene {
        species: "grass".to_string(),
        trees: a.plants,
        soil_moisture: a.moisture,
        soil_depth: a.soil,
        start_frame: a.start_frame,
        ..common::PlantScene::default()
    };
    let ground_y = scene.ground_y;
    let width = scene.width;
    let mut w = scene.build();

    for step in 0..a.grow {
        advance(&mut w, step);
    }

    // Re-wet or dry the bed the grown sward stands in, then let the field
    // catch up. Every soil cell in the world, not just under the sward:
    // the moisture field diffuses sideways, so a dried patch beside a wet
    // one is a gradient rather than a dry meadow.
    if a.burn_moisture != u16::MAX {
        let Some(b) = w.bounds() else { panic!("empty world") };
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                let c = w.get(x, y);
                if w.materials.get(c.material).water_capacity > 0 {
                    w.set(x, y, c.with_aux(a.burn_moisture));
                }
            }
        }
        for step in 0..a.settle {
            advance(&mut w, a.grow + step);
        }
    }

    let grass = w.materials.id_of("grassblade").expect("grassblade is compiled in");
    let ash = w.materials.id_of("ash").expect("ash is compiled in");
    let smoke = w.materials.id_of("smoke").expect("smoke is compiled in");
    let flame = w.materials.id_of("flame").expect("flame is compiled in");

    // The sward's geometry, before anything is lit. Contact ignition scans
    // 4 neighbours, so what a front has to cross is *empty columns*, and
    // the largest of them is the wall.
    let occupied: Vec<bool> = (0..width)
        .map(|x| (0..ground_y).any(|y| w.get(x, y).material == grass))
        .collect();
    let live: usize = occupied.iter().filter(|&&o| o).count();
    let mut gaps: Vec<usize> = Vec::new();
    let mut run = 0usize;
    let (first, last) = (
        occupied.iter().position(|&o| o).unwrap_or(0),
        occupied.iter().rposition(|&o| o).unwrap_or(0),
    );
    for &here in &occupied[first..=last] {
        if here {
            if run > 0 {
                gaps.push(run);
            }
            run = 0;
        } else {
            run += 1;
        }
    }
    if run > 0 {
        gaps.push(run);
    }
    gaps.sort_unstable();
    let cells = count(&w, grass);
    // **The quantity the column histogram above cannot see.** Contact
    // ignition scans four neighbours, so what a front can reach is one
    // 4-connected component of fuel and nothing else -- two blades in
    // adjacent columns at different heights are as far apart, for fire, as
    // two blades a screen apart. A sward that looks continuous in a column
    // census can still be a scatter of islands, and a fire that burns one
    // island and stops reads on a contact sheet exactly like a fire that
    // spreads slowly.
    let (components, largest) = fuel_components(&w, grass, ground_y);
    println!(
        "  sward: {cells} grass cells over {live}/{width} columns, spanning x {first}..{last}\n  \
         gaps between tussocks: {} of them, median {}, max {}\n  \
         4-connected fuel islands: {components}, largest {largest} cells ({:.1}% of the sward)",
        gaps.len(),
        gaps.get(gaps.len() / 2).copied().unwrap_or(0),
        gaps.last().copied().unwrap_or(0),
        100.0 * largest as f32 / cells.max(1) as f32,
    );

    // What the ignition rule actually reads. `fire::try_ignite` divides
    // `field_moisture_at` by 4.0 and scales flammability by
    // `1 - saturation * 0.9`, so these two lines are the whole of the
    // moisture term's input: if the sward band reads ~0 the term is inert
    // however wet the ground under it is.
    let sward_top = (0..ground_y).find(|&y| (0..width).any(|x| w.get(x, y).material == grass)).unwrap_or(ground_y - 1);
    // **The distribution the ignition rule actually reads**, sampled at
    // the grass cells themselves rather than over a band of mostly air.
    // The band mean below is what a scene report would print; this is what
    // `try_ignite` sees, and the two differ because a sward is thin. A
    // *mean* is the wrong summary for a percolation process anyway -- what
    // decides whether a front crosses is the driest cells it can reach, so
    // the low end of this distribution is the number that matters.
    let mut fuel_moisture: Vec<f32> = Vec::new();
    for y in 0..ground_y {
        for x in 0..width {
            if w.get(x, y).material == grass {
                fuel_moisture.push(w.field_at(x, y).moisture);
            }
        }
    }
    fuel_moisture.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pick = |q: f32| fuel_moisture.get(((fuel_moisture.len() as f32 - 1.0) * q) as usize).copied().unwrap_or(0.0);
    println!(
        "  humidity AT THE FUEL ({} grass cells): mean {:.3}  median {:.3}  p90 {:.3}  max {:.3}  -- {} cells ({:.1}%) read exactly zero",
        fuel_moisture.len(),
        fuel_moisture.iter().sum::<f32>() / fuel_moisture.len().max(1) as f32,
        pick(0.5),
        pick(0.9),
        pick(1.0),
        fuel_moisture.iter().filter(|&&m| m == 0.0).count(),
        100.0 * fuel_moisture.iter().filter(|&&m| m == 0.0).count() as f32 / fuel_moisture.len().max(1) as f32,
    );
    let mut wetness: Vec<f32> = Vec::new();
    for y in 0..ground_y {
        for x in 0..width {
            if w.get(x, y).material == grass {
                wetness.push(w.ground_wetness_at(x, y));
            }
        }
    }
    wetness.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let wpick = |q: f32| wetness.get(((wetness.len() as f32 - 1.0) * q) as usize).copied().unwrap_or(0.0);
    println!(
        "  GROUND WETNESS at the fuel (what try_ignite now gates on): mean {:.3}  min {:.3}  median {:.3}  p90 {:.3}  -- {} cells ({:.1}%) read exactly zero",
        wetness.iter().sum::<f32>() / wetness.len().max(1) as f32,
        wpick(0.0),
        wpick(0.5),
        wpick(0.9),
        wetness.iter().filter(|&&m| m == 0.0).count(),
        100.0 * wetness.iter().filter(|&&m| m == 0.0).count() as f32 / wetness.len().max(1) as f32,
    );
    println!(
        "  field moisture: sward band (rows {sward_top}..{ground_y}) mean {:.3}, soil (rows {}..{}) mean {:.3}  [MOISTURE_SATURATION = 4.0]",
        mean_field_moisture(&w, sward_top, ground_y, width),
        ground_y,
        ground_y + 4,
        mean_field_moisture(&w, ground_y, ground_y + 4, width),
    );
    println!(
        "  soil water under the sward: mean {:.0} / {} saturated  (wilting point {})",
        mean_soil_moisture(&w, ground_y, ground_y + 4, width),
        material::SOIL_SATURATED,
        material::SOIL_WILTING_POINT,
    );

    // **Is there such a thing as a sheltered spot?** Asked by lane S while
    // staging wind-throw (`Reports/physical-trees-design-2026-08-23.md`
    // §11.5): `weather::at(seed, frame)` takes no position, so the
    // *driving* wind is global and time-only, and the open question is
    // whether anything downstream makes it local. Fire is now a second
    // consumer of that channel -- a flame is a `Gas`, and `update_gas`
    // steers every gas cell through `wind_biased_order`, which reads
    // `field_wind_at` -- so this harness can answer it with a number
    // instead of a reading of the source.
    //
    // Sampled across the sward at one instant. A spread of zero means the
    // field carries the global wind unchanged and a fire leans the same
    // way everywhere; a nonzero spread means `weather::gust`'s dipole
    // impulses (radius 26) do make it local, at least while they last.
    {
        let winds: Vec<f32> = (0..width).step_by(field::FIELD_SCALE as usize).map(|x| { let f = w.field_at(x, sward_top); f.vx }).collect();
        let lo = winds.iter().cloned().fold(f32::INFINITY, f32::min);
        let hi = winds.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let mean = winds.iter().sum::<f32>() / winds.len().max(1) as f32;
        println!(
            "  wind across the sward at frame {}: min {lo:.4} mean {mean:.4} max {hi:.4} (spread {:.4}) over {} samples",
            w.frame,
            hi - lo,
            winds.len(),
        );
    }

    let ignite_x = (width as f32 * a.at) as i32;
    let ignite_y = sward_top.max(ground_y - 6);
    let lit = (ignite_y - 3..=ignite_y + 3)
        .flat_map(|cy| (ignite_x - 3..=ignite_x + 3).map(move |cx| (cx, cy)))
        .filter(|&(cx, cy)| {
            let d = (cx - ignite_x, cy - ignite_y);
            d.0 * d.0 + d.1 * d.1 <= 9 && w.get(cx, cy).material != material::EMPTY
        })
        .count();
    w.ignite_circle(ignite_x, ignite_y, 3);
    println!("  ignite: ({ignite_x}, {ignite_y}) r=3 -- lit {lit} cells");

    let mut previous_front = ignite_x;
    let mut previous_frame = 0usize;
    println!("  frame |  alight  burnt   grass    ash  flame  smoke |  front  advance");
    for step in 0..=a.frames {
        if step > 0 {
            advance(&mut w, a.grow + step);
        }
        if step % a.every != 0 {
            continue;
        }
        let mut alight = 0usize;
        let mut front = ignite_x;
        let mut back = ignite_x;
        let mut burnt_columns = 0usize;
        for x in 0..width {
            let mut touched = false;
            for y in 0..ground_y + 4 {
                let c = w.get(x, y);
                if c.is_burning() {
                    alight += 1;
                    touched = true;
                }
                // Burnt fuel is ash *standing where grass was*, which is
                // why this scans the sward band rather than counting ash
                // anywhere: ash is a Powder and falls, so a global ash
                // count answers "how much burned", never "how far".
                if c.material == ash {
                    touched = true;
                }
            }
            if touched {
                burnt_columns += 1;
                front = front.max(x);
                back = back.min(x);
            }
        }
        let advance_rate = if step > previous_frame {
            (front - previous_front) as f32 / (step - previous_frame) as f32
        } else {
            0.0
        };
        println!(
            "  {step:>5} | {alight:>7} {burnt_columns:>6} {:>7} {:>6} {:>6} {:>6} | {front:>6}  {advance_rate:>6.3} c/f  (band x {back}..{front})",
            count(&w, grass),
            count(&w, ash),
            count(&w, flame),
            count(&w, smoke),
        );
        // **What is actually up there.** A plume read as white specks on a
        // contact sheet and the obvious reading -- "the smoke is drawing
        // white" -- is a guess about the renderer. This is the census that
        // answers it instead: everything standing in the sky band, by
        // material.
        {
            let mut by_material: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for y in 0..sward_top {
                for x in 0..width {
                    let c = w.get(x, y);
                    if c.material != material::EMPTY {
                        *by_material.entry(w.materials.get(c.material).name.clone()).or_default() += 1;
                    }
                }
            }
            let mut counts: Vec<(String, usize)> = by_material.into_iter().collect();
            counts.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
            let line: Vec<String> = counts.iter().take(6).map(|(n, c)| format!("{n} {c}")).collect();
            println!("         in the sky band (rows 0..{sward_top}): {}", if line.is_empty() { "(nothing)".to_string() } else { line.join(", ") });
        }
        // Smoke temperature, because the plume's *colour* is a function of
        // it: `render.rs` blends every cell toward the fire tint by
        // `(temperature - ambient) / HEAT_GLOW_RANGE`, and a burnout hands
        // the new cell the old one's temperature. Smoke born of an 780C
        // flame therefore draws at the top of the fire ramp -- a white
        // plume -- until it has cooled, and how fast it cools is the
        // question a picture cannot answer.
        let mut smoke_temps: Vec<i32> = Vec::new();
        for y in 0..ground_y {
            for x in 0..width {
                let c = w.get(x, y);
                if c.material == smoke {
                    smoke_temps.push(c.temperature() as i32);
                }
            }
        }
        if !smoke_temps.is_empty() {
            smoke_temps.sort_unstable();
            println!(
                "         smoke temperature: min {} median {} max {}  ({} cells; the fire ramp saturates {}C above ambient)",
                smoke_temps[0],
                smoke_temps[smoke_temps.len() / 2],
                smoke_temps[smoke_temps.len() - 1],
                smoke_temps.len(),
                400,
            );
        }
        previous_front = front;
        previous_frame = step;
    }

    // **What the ground under the burn was worth by the end.** A wetness
    // arm that dries out mid-run is not the arm it was set up as, and the
    // burn it produced belongs to whatever the ground had become. Sampled
    // over the ash left behind, which is where the fire actually was.
    let mut after: Vec<f32> = Vec::new();
    for y in 0..ground_y {
        for x in 0..width {
            if w.get(x, y).material == ash {
                after.push(w.ground_wetness_at(x, y));
            }
        }
    }
    after.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if after.is_empty() {
        println!("  ground wetness under the burn, at the end: (nothing burned)");
    } else {
        println!(
            "  ground wetness under the burn, at the end: min {:.3}  median {:.3}  max {:.3}  over {} ash cells",
            after[0],
            after[after.len() / 2],
            after[after.len() - 1],
            after.len(),
        );
    }
}

/// Number of 4-connected components of `id` in the sward band, and the
/// size of the largest. The neighbourhood is 4, deliberately and not 8,
/// because that is the neighbourhood `fire::try_ignite` uses -- a
/// traversal must use the same neighbourhood the writer used
/// (`CLAUDE.md`), and here the "writer" is the ignition rule whose reach
/// this is measuring.
fn fuel_components(w: &World, id: material::MaterialId, ground_y: i32) -> (usize, usize) {
    let Some(b) = w.bounds() else { return (0, 0) };
    let mut seen: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();
    let (mut components, mut largest) = (0usize, 0usize);
    for y in b.min_y..=ground_y {
        for x in b.min_x..=b.max_x {
            if w.get(x, y).material != id || seen.contains(&(x, y)) {
                continue;
            }
            components += 1;
            let mut size = 0usize;
            let mut stack = vec![(x, y)];
            seen.insert((x, y));
            while let Some((cx, cy)) = stack.pop() {
                size += 1;
                for (nx, ny) in [(cx - 1, cy), (cx + 1, cy), (cx, cy - 1), (cx, cy + 1)] {
                    if w.get(nx, ny).material == id && seen.insert((nx, ny)) {
                        stack.push((nx, ny));
                    }
                }
            }
            largest = largest.max(size);
        }
    }
    (components, largest)
}

fn count(w: &World, id: material::MaterialId) -> usize {
    let Some(b) = w.bounds() else { return 0 };
    (b.min_y..=b.max_y)
        .map(|y| (b.min_x..=b.max_x).filter(|&x| w.get(x, y).material == id).count())
        .sum()
}

fn mean_field_moisture(w: &World, y0: i32, y1: i32, width: i32) -> f32 {
    let mut total = 0.0;
    let mut n = 0;
    let mut y = y0;
    while y < y1 {
        let mut x = 0;
        while x < width {
            total += w.field_at(x, y).moisture;
            n += 1;
            x += field::FIELD_SCALE;
        }
        y += field::FIELD_SCALE;
    }
    if n == 0 { 0.0 } else { total / n as f32 }
}

fn mean_soil_moisture(w: &World, y0: i32, y1: i32, width: i32) -> f32 {
    let mut total = 0.0;
    let mut n = 0;
    for y in y0..y1 {
        for x in 0..width {
            let c = w.get(x, y);
            if w.materials.get(c.material).water_capacity > 0 {
                total += update::soil_moisture(c) as f32;
                n += 1;
            }
        }
    }
    if n == 0 { 0.0 } else { total / n as f32 }
}

/// `App::update`'s own order, minus the parts a grass fire cannot reach
/// (no player, no blasts, no promoted liquid bodies). The **parallel**
/// driver deliberately: `App::update` calls it, so behaviour only the
/// player sees is behaviour only this driver produces (`CLAUDE.md`).
fn advance(w: &mut World, _step: usize) {
    parallel::step(w);
    w.step_liquid_bodies();
    rigid::step_chunk_bodies(w);
    w.step_active_sites();
    w.step_fields();
}
