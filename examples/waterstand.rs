//! **What is holding the standing water up, and why can it not get down?**
//!
//! Censuses every liquid cell in a lab bed by what sits *directly underneath*
//! it -- air, more water, living tissue, ground, or something else -- and then,
//! for the cells resting on tissue, replays `update::drip_through_organism`'s
//! own downward scan and reports **which material refused it**.
//!
//! Built for the owner's report that *water pools on top of the plants instead
//! of going through and soaking into the soil*, where a standing count alone
//! could not tell three different failures apart: water held up by a mat it
//! cannot pass, water standing on ground that is already full, and rain simply
//! in transit. It separated them in one run -- 876 cells on tissue with **8**
//! able to drip, 308 refused by the soil under the mat and 500 by water already
//! trapped in it.
//!
//! **The two halves are the point, and neither works alone.** A standing count
//! says *how much*; only the replayed scan says *why*, and only the material
//! histogram says *what to fix*. `soil_drawdown` owns the bed's water ledger
//! and its overlay; this owns the question of what the water is sitting on.
//!
//! ```text
//! cargo run --release --example waterstand -- scenario=played_bed_scrambler rain=2
//! cargo run --release --example waterstand -- scenario=played_bed rain=1 frames=60000 every=20000
//! cargo run --release --example waterstand -- scenario=played_bed_scrambler png=/tmp/bed
//! ```
//!
//! `rain=<0-3>` is `Rain::from_index` (OFF/LIGHT/STEADY/HEAVY), overriding the
//! scenario's own setting so one file can be read at every rate --
//! `soil_drawdown`'s own flag, and for the same reason. `png=<prefix>` writes
//! the bed twice, in material colours and through the soil-moisture overlay,
//! because the standing sheet and the bed under it are two different pictures.
//! `level=1` sets `World::soil_capillary_levels`, the sideways-levelling dial
//! the lab's parameters page also carries -- the arm for the soil-column
//! question, paired offline by seed.

use pixel_physics::lab::rain::Rain;
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::Lab;
use pixel_physics::sim::material::{self, MaterialKind};
use pixel_physics::sim::update;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

const REACH: i32 = 16;

fn census(world: &World, spec: &LabBox, frame: u64) {
    let Some(water) = world.materials.id_of("water") else { return };
    let (mut on_air, mut on_water, mut on_plant, mut on_soil, mut on_other) = (0u64, 0u64, 0u64, 0u64, 0u64);
    // Why a cell resting on tissue is refused, replaying the scan.
    let (mut would_drip, mut blocked_soil, mut blocked_other, mut out_of_reach) = (0u64, 0u64, 0u64, 0u64);
    // Is the soil under a puddle actually full?
    let (mut soil_full, mut soil_room) = (0u64, 0u64);
    let mut blockers: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    let (mut soil_blocker_full, mut soil_blocker_room) = (0u64, 0u64);
    let mut fill_on_plant = 0u64;
    let mut total_fill = 0u64;
    let mut highest = spec.height;
    for y in 0..spec.height {
        for x in 0..spec.width {
            let c = world.get(x, y);
            if c.material != water {
                continue;
            }
            total_fill += update::liquid_fill(c) as u64;
            highest = highest.min(y);
            let below = world.get(x, y + 1);
            if below.material == material::EMPTY {
                on_air += 1;
            } else if below.material == water {
                on_water += 1;
            } else if below.organism_id() != 0 {
                on_plant += 1;
                fill_on_plant += update::liquid_fill(c) as u64;
                let mut verdict = 3; // out of reach
                for probe in (y + 1)..(y + 1 + REACH) {
                    let here = world.get(x, probe);
                    if here.material == material::EMPTY {
                        verdict = 0;
                        break;
                    }
                    if here.organism_id() == 0 {
                        if world.materials.get(here.material).water_capacity > 0 {
                            verdict = 1;
                            if update::soil_moisture(here) >= world.materials.get(here.material).water_capacity {
                                soil_blocker_full += 1;
                            } else {
                                soil_blocker_room += 1;
                            }
                        } else {
                            verdict = 2;
                            *blockers.entry(world.materials.get(here.material).name.clone()).or_default() += 1;
                        }
                        break;
                    }
                }
                match verdict {
                    0 => would_drip += 1,
                    1 => blocked_soil += 1,
                    2 => blocked_other += 1,
                    _ => out_of_reach += 1,
                }
            } else if world.materials.get(below.material).water_capacity > 0 {
                on_soil += 1;
                let cap = world.materials.get(below.material).water_capacity;
                if update::soil_moisture(below) >= cap {
                    soil_full += 1;
                } else {
                    soil_room += 1;
                }
            } else {
                on_other += 1;
            }
        }
    }
    let standing: u64 = (0..spec.ground_y).map(|y| (0..spec.width).filter(|&x| world.get(x, y).material == water).count() as u64).sum();
    println!(
        "frame {frame:>7}  water cells: on air {on_air}, on water {on_water}, on PLANT {on_plant}, on soil {on_soil} (full {soil_full} / room {soil_room}), on other {on_other}   [above the soil line {standing}, total fill {total_fill}, top row {highest} of ground {}]",
        spec.ground_y
    );
    println!(
        "                 of the {on_plant} resting on tissue ({fill_on_plant} fill): would drip {would_drip}, blocked by SOIL {blocked_soil} (of which saturated {soil_blocker_full} / has room {soil_blocker_room}), blocked by other {blocked_other} {blockers:?}, tissue deeper than {REACH} {out_of_reach}"
    );
    let sat = (0..spec.width)
        .filter(|&x| {
            let c = world.get(x, spec.ground_y);
            world.materials.get(c.material).water_capacity > 0 && update::soil_moisture(c) >= material::SOIL_SATURATED
        })
        .count();
    println!("                 top soil row at saturation in {sat} of {} columns", spec.width);
    // Where the pool's own floor is, row by row, so "a sheet on the mat" and
    // "a sheet on the soil" are distinguishable.
    let mut rows: Vec<(i32, u64)> = Vec::new();
    for y in 0..spec.height {
        let n = (0..spec.width).filter(|&x| world.get(x, y).material == water).count() as u64;
        if n > 0 {
            rows.push((y, n));
        }
    }
    let head: Vec<String> = rows.iter().take(14).map(|(y, n)| format!("{y}:{n}")).collect();
    println!("                 rows holding water (row:cells) {}", head.join(" "));
    let plants = (0..spec.height)
        .map(|y| (0..spec.width).filter(|&x| world.materials.kind(world.get(x, y).material) == MaterialKind::Plant).count() as u64)
        .sum::<u64>();
    println!("                 plant cells {plants}");
    // **The downstream line**, for the owner's question on the soil-column
    // dial: *are there any downstream effects of the change?* A transport
    // rule that moves water between columns could reach the stand, the
    // reseeding funnel and the colony, and none of those is visible in a
    // water census. One line, so a seed sweep can be read as a table.
    println!(
        "                 DOWNSTREAM plants {} animals {} plantcells {plants} standingwater {standing} saturatedsoil {}",
        world.live_organism_ids().len() - world.live_creature_count(),
        world.live_creature_count(),
        (0..spec.height)
            .flat_map(|y| (0..spec.width).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let c = world.get(x, y);
                world.materials.get(c.material).water_capacity > 0 && update::soil_moisture(c) >= material::SOIL_SATURATED
            })
            .count()
    );
    soil_columns(world, spec);
}

/// **Is the bed's water standing in columns, and what is holding them
/// apart?**
///
/// The second half of the owner's report -- *water in the soil builds up in
/// these columns* -- and it needs a per-column read rather than a profile:
/// `soil_drawdown`'s `across` buckets are means over an eighth of the bed, so
/// a stripe one or two cells wide is averaged away before it can be seen.
///
/// What it prints, and why each line is the one that discriminates:
///
/// - **the band census**, because `update_soil_water` has two different rest
///   thresholds and which one a pair of cells gets is decided by whether the
///   *wetter* of them is over field capacity. A bed that is entirely under
///   field capacity cannot stripe for that reason and a bed over it can;
/// - **the adjacent-column difference, split on exactly that test.** A
///   difference that sits under its own pair's rest threshold is a pair the
///   capillary rule has declared level and will never touch again. That is
///   the difference between "the bed has not finished levelling" and "the bed
///   is at rest and looks like this";
/// - **the widest standing gap**, because a mean difference near zero and one
///   large stripe are the same number.
fn soil_columns(world: &World, spec: &LabBox) {
    let bottom = spec.ground_y + spec.soil_depth;
    let mut col: Vec<Option<f32>> = Vec::with_capacity(spec.width as usize);
    let (mut dry, mut usable, mut drainable, mut saturated) = (0u64, 0u64, 0u64, 0u64);
    for x in 0..spec.width {
        let (mut sum, mut n) = (0u64, 0u64);
        for y in spec.ground_y..bottom {
            let c = world.get(x, y);
            if world.materials.get(c.material).water_capacity == 0 {
                continue;
            }
            let m = update::soil_moisture(c);
            sum += m as u64;
            n += 1;
            if m < material::SOIL_WILTING_POINT {
                dry += 1;
            } else if m <= material::SOIL_FIELD_CAPACITY {
                usable += 1;
            } else if m < material::SOIL_SATURATED {
                drainable += 1;
            } else {
                saturated += 1;
            }
        }
        col.push((n > 0).then(|| sum as f32 / n as f32));
    }
    println!(
        "                 soil cells by band: under the wilting point {dry}, usable {usable}, drainable (over field capacity) {drainable}, saturated {saturated}"
    );
    // Pair up neighbouring cells across the bed and ask, of each pair, whether
    // the capillary rule would still move anything between them.
    let (mut at_rest_wide, mut at_rest_narrow, mut moving) = (0u64, 0u64, 0u64);
    let mut widest = (0u16, 0i32, 0i32);
    for y in spec.ground_y..bottom {
        for x in 0..(spec.width - 1) {
            let (a, b) = (world.get(x, y), world.get(x + 1, y));
            if world.materials.get(a.material).water_capacity == 0 || world.materials.get(b.material).water_capacity == 0 {
                continue;
            }
            let (ma, mb) = (update::soil_moisture(a), update::soil_moisture(b));
            let (wetter, drier) = if ma > mb { (ma, mb) } else { (mb, ma) };
            let gap = wetter - drier;
            // Mirrors `update_soil_water`'s own choice, dial included, so the
            // "declared level" column says what the engine in *this* run is
            // doing rather than what the shipped default would do.
            let rest = if wetter > material::SOIL_FIELD_CAPACITY && !world.soil_capillary_levels {
                material::SOIL_SATURATED - material::SOIL_FIELD_CAPACITY
            } else {
                60
            };
            if gap > rest {
                moving += 1;
            } else if rest > 60 {
                at_rest_wide += 1;
                if gap > widest.0 {
                    widest = (gap, x, y);
                }
            } else {
                at_rest_narrow += 1;
            }
        }
    }
    println!(
        "                 side-by-side soil pairs: still exchanging {moving}, declared level under the WIDE threshold (380) {at_rest_wide}, under the narrow one (60) {at_rest_narrow}; widest standing gap {} at ({}, {})",
        widest.0, widest.1, widest.2
    );
    // The column profile itself, coarse enough to read in a log line.
    let strip: Vec<String> = col
        .iter()
        .step_by(16)
        .map(|m| m.map_or("   -".to_string(), |m| format!("{m:>4.0}")))
        .collect();
    println!("                 column means every 16th column [{}]", strip.join(" "));
}

/// One frame of the bed with no window, in both channels.
///
/// Two files rather than one because the question needs both and they are not
/// interchangeable: the material draw is what the owner reported (a sheet of
/// water lying over the plants), and the overlay is what says whether the bed
/// under it has room left. A card carrying only the first cannot distinguish
/// "the mat will not let it through" from "the ground is full".
fn shoot(lab: &mut Lab, prefix: &str, zoom: u32) {
    let (w, h) = (pixel_physics::lab::WIDTH, pixel_physics::lab::HEIGHT);
    let zoom = zoom.max(1);
    for (suffix, overlay) in [
        ("", pixel_physics::render::OrganismOverlay::Off),
        ("-soil", pixel_physics::render::OrganismOverlay::SoilMoisture),
    ] {
        let mut buf = vec![0u8; (w * h * 4) as usize];
        lab.renderer.organism_overlay = overlay;
        // `force_full`, so the second draw does not inherit an empty touched
        // set from the first and come back as the first one's pixels wearing
        // the second one's name -- `take_touched_chunks` drains.
        let touched = lab.world.take_touched_chunks();
        lab.renderer.draw(&lab.world, &lab.particles, &touched, &mut buf, (w, h), true);
        let (zw, zh) = (w * zoom, h * zoom);
        let mut out = vec![0u8; (zw * zh * 4) as usize];
        for y in 0..zh {
            for x in 0..zw {
                let src = (((y / zoom) * w + (x / zoom)) * 4) as usize;
                let dst = ((y * zw + x) * 4) as usize;
                out[dst..dst + 4].copy_from_slice(&buf[src..src + 4]);
            }
        }
        let path = format!("{prefix}{suffix}.png");
        image::save_buffer(&path, &out, zw, zh, image::ColorType::Rgba8).expect("write png");
        println!("  wrote {path} ({zw}x{zh}, {})", if suffix.is_empty() { "material colours" } else { "soil-moisture overlay" });
    }
}

fn main() {
    let frames: u64 = arg("frames").unwrap_or(24_000);
    let every: u64 = arg("every").unwrap_or(8_000);
    let seed: u64 = arg("seed").unwrap_or(1);
    let rain_idx: u8 = arg("rain").unwrap_or(2);
    // **`founders=` builds a bare bed instead of a scenario, and `founders=0`
    // is the control the soil question cannot be read without.** A bed with
    // nothing alive in it answers "would this bed do that on its own", which
    // is what separates a root pattern from a transport rule -- the same
    // reason `soil_drawdown` keeps its own `founders=0` arm, whose note says
    // it is not optional.
    let founders: Option<usize> = arg("founders");
    let name: String = arg("scenario").unwrap_or_else(|| "played_bed".to_string());
    let (mut lab, msg) = match founders {
        Some(n) => {
            let bed = LabBox {
                founders: n,
                colonies: 0,
                seed,
                species: arg::<String>("plant").unwrap_or_else(|| LabBox::default().species),
                soil_depth: arg("soil").unwrap_or(LabBox::default().soil_depth),
                ..LabBox::default()
            };
            let lab = Lab::new(bed);
            (lab, format!("BARE BED -- {n} founder(s), no scenario"))
        }
        None => {
            let mut sc = Scenario::load(&name).unwrap_or_else(|e| {
                eprintln!("scenario {name}: {e}");
                std::process::exit(1);
            });
            sc.bed.seed = seed;
            let mut lab = Lab::new(sc.bed.clone());
            let msg = lab.load_scenario(sc);
            (lab, msg)
        }
    };
    lab.spec.rain = Rain::from_index(rain_idx);
    // **`level=1` is the arm, and it is a field on the world rather than an
    // env var** -- see `World::soil_capillary_levels`. Two runs of one binary,
    // paired offline by seed.
    lab.world.soil_capillary_levels = arg::<i32>("level").unwrap_or(0) != 0;
    let spec = lab.spec.clone();
    // Echo every parameter including the ones that default -- `CLAUDE.md`'s
    // megastudy gotcha, where a knob added after the binary was built was
    // silently ignored and produced 24 logs of 3 populations.
    println!(
        "waterstand: {} seed={seed} rain={} frames={frames} every={every} soil={} width={} water_levels_sideways={}",
        founders.map_or_else(|| format!("scenario={name}"), |n| format!("founders={n}")),
        lab.spec.rain.label(),
        spec.soil_depth,
        spec.width,
        lab.world.soil_capillary_levels
    );
    println!("  {msg}");
    // **What the bed costs while it is doing this**, because the sideways
    // threshold is a churn guard and the only honest question about narrowing
    // it is what the churn costs. `sw chgd` is the soil-moisture writes a tick
    // makes -- the quantity the guard exists to hold down -- and it is a
    // counter, so it is the number to gate on; the timing beside it is
    // reported at the median, because `CLAUDE.md`'s rule is that a worst frame
    // nothing pins is noise wearing a number. Pin `RAYON_NUM_THREADS` and
    // alternate the arms before comparing either.
    let mut next = 0u64;
    let mut ticks: Vec<f64> = Vec::with_capacity(frames as usize);
    let (mut sw_changed, mut sw_visited) = (0u64, 0u64);
    for f in 0..=frames {
        if f == next {
            census(&lab.world, &spec, f);
            next += every;
        }
        let t0 = std::time::Instant::now();
        lab.tick_for_harness();
        ticks.push(t0.elapsed().as_secs_f64() * 1000.0);
        sw_changed += lab.world.soil_water_stats.changed;
        sw_visited += lab.world.soil_water_stats.visited;
    }
    ticks.sort_by(|a, b| a.partial_cmp(b).expect("no NaN from a duration"));
    let n = ticks.len();
    println!(
        "  cost: median {:.3} ms/tick, mean {:.3}, p90 {:.3} over {n} ticks | soil-moisture writes {:.1}/tick (of {:.1} cells walked)",
        ticks[n / 2],
        ticks.iter().sum::<f64>() / n as f64,
        ticks[n * 9 / 10],
        sw_changed as f64 / n as f64,
        sw_visited as f64 / n as f64,
    );
    if let Some(prefix) = arg::<String>("png") {
        shoot(&mut lab, &prefix, arg("zoom").unwrap_or(2));
    }
}
