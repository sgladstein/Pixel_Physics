//! **What does one plant do to the whole bed's water?**
//!
//! The owner's report, 2026-08-31, from the moisture overlay: the lab starts
//! at 62% everywhere and holds there; the moment a single plant goes in, the
//! surface starts drying, the dryness spreads *sideways* across the whole
//! bed — faster left than right — and settles at 18%, while the bottom of
//! the bed climbs to 100%. Seeds then will not germinate.
//!
//! 62 / 18 / 100 are `SOIL_FIELD_CAPACITY`, `SOIL_WILTING_POINT` and
//! `SOIL_SATURATED`, so the question is whether the profile is being pumped
//! from one end of that range to the other.
//!
//! ```text
//! cargo run --release --example soil_drawdown                 # one plant
//! cargo run --release --example soil_drawdown -- founders=0   # the control: nothing alive
//! cargo run --release --example soil_drawdown -- frames=24000 every=4000
//! cargo run --release --example soil_drawdown -- png=after zoom=2   # the owner's own overlay
//! ```
//!
//! `png=` writes the bed through the shipped renderer with
//! `OrganismOverlay::SoilMoisture` switched on -- **the same picture the
//! report came from**, which is the point: the numbers below say how much and
//! whether it came back, and only the overlay says the profile looks like
//! what the owner is looking at. Off by default, because it costs a frame of
//! rendering and every other use of this probe is numeric.
//!
//! # What it reports, and why these three views
//!
//! - **A horizontal profile along the rooting row.** The claim is that
//!   dryness spreads sideways from one plant to the whole bed, so the
//!   instrument has to be able to see *distance from the plant* — a single
//!   column cannot, and neither can a total.
//! - **A vertical profile down the middle.** The claim is that the bottom
//!   saturates while the top empties. That is a redistribution, not a loss,
//!   and only a column shows it.
//! - **The bed's total water, and how it splits above and below field
//!   capacity.** This is what separates the two candidate stories: a *sink*
//!   (total falls — the plant and the sun really are consuming it) from a
//!   *ratchet* (total holds, but the distribution polarises to the ends of
//!   the range). They look identical on the overlay and want opposite fixes.
//!
//! # `founders=0` is the control and is not optional
//!
//! An empty bed must hold flat at field capacity. The owner reports that it
//! does, and this arm is what makes every figure in the other one mean
//! something: if the bed drifts with nothing alive in it, the plant is not
//! the cause and the rest of the run is measuring the wrong thing.

use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::Lab;
use pixel_physics::sim::material;
use pixel_physics::sim::update;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

fn tick(lab: &mut Lab) {
    pixel_physics::sim::frame::step(
        &mut lab.world,
        &mut lab.particles,
        &mut lab.blasts,
        pixel_physics::sim::player::PlayerInput::default(),
        &pixel_physics::sim::player::Tuning::default(),
    );
}

/// Mean soil moisture over the cells of one row that actually hold water.
fn row_mean(world: &World, spec: &LabBox, y: i32, x0: i32, x1: i32) -> Option<u16> {
    let (mut sum, mut n) = (0u64, 0u64);
    for x in x0.max(0)..x1.min(spec.width) {
        let c = world.get(x, y);
        if world.materials.get(c.material).water_capacity > 0 {
            sum += update::soil_moisture(c) as u64;
            n += 1;
        }
    }
    (n > 0).then(|| (sum / n) as u16)
}

/// The top `reach` rows of one column, averaged -- the band
/// `evaporation::SOIL_DRY_REACH` can actually pull from, which is the band a
/// seed's germination gate reads and the band an overlay shows as "the
/// surface".
///
/// `None` where the column has no water-holding cell in that band at all.
fn column_surface(world: &World, spec: &LabBox, x: i32, reach: i32) -> Option<u16> {
    let (mut sum, mut n) = (0u64, 0u64);
    for d in 0..reach {
        let c = world.get(x, spec.ground_y + d);
        if world.materials.get(c.material).water_capacity > 0 {
            sum += update::soil_moisture(c) as u64;
            n += 1;
        }
    }
    (n > 0).then(|| (sum / n) as u16)
}

/// Whether anything is sitting on this column's surface cell.
///
/// **This is the predicate that decides whether the sun can touch it.**
/// `evaporation::is_damp_soil_surface` refuses a column whose cell above is
/// not empty, so a column under a stem, a leaf or a scrap of litter does not
/// dry at all while the bare column beside it does. Printed next to the
/// moisture so a dry-vs-wet difference can be read against its cause instead
/// of guessed at.
fn covered(world: &World, spec: &LabBox, x: i32) -> bool {
    !world.get(x, spec.ground_y - 1).is_empty()
}

fn main() {
    let founders: usize = arg("founders").unwrap_or(1);
    let frames: u64 = arg("frames").unwrap_or(24_000);
    let every: u64 = arg("every").unwrap_or(4_000);
    let seed: u64 = arg("seed").unwrap_or(1);
    let png: Option<String> = arg("png");
    let fine: bool = arg::<u32>("fine").unwrap_or(0) > 0;
    let spec = LabBox { founders, colonies: 0, seed, ..LabBox::default() };
    // Echo every parameter, including the ones that default -- `CLAUDE.md`'s
    // megastudy gotcha, where a knob added after the binary was built was
    // silently ignored and produced 24 logs of 3 populations.
    println!(
        "soil_drawdown: founders={founders} frames={frames} every={every} seed={seed} \
         png={} fine={fine} (field capacity {}, wilting point {}, saturated {})",
        png.as_deref().unwrap_or("-"),
        material::SOIL_FIELD_CAPACITY,
        material::SOIL_WILTING_POINT,
        material::SOIL_SATURATED
    );

    let mut lab = Lab::new(spec.clone());
    let start_bank = lab.world.atmospheric_bank;
    let start_total = {
        let w = &lab.world;
        let mut t = 0u64;
        for y in spec.ground_y..(spec.ground_y + spec.soil_depth) {
            for x in 0..spec.width {
                let c = w.get(x, y);
                if w.materials.get(c.material).water_capacity > 0 {
                    t += update::soil_moisture(c) as u64;
                }
            }
        }
        t
    };
    // Where the founder actually is, so "distance from the plant" is read off
    // the scene rather than assumed.
    let planted: Vec<i32> = spec.founder_columns();
    println!("  founder column(s): {planted:?}\n");

    for f in 0..=frames {
        if f % every == 0 {
            let w = &lab.world;
            // Horizontal: the row a root drinks from, in eighths of the bed.
            let row = spec.ground_y + 2;
            let step = spec.width / 8;
            let across: Vec<String> = (0..8)
                .map(|i| {
                    let x0 = i * step;
                    match row_mean(w, &spec, row, x0, x0 + step) {
                        Some(m) => format!("{m:>4}"),
                        None => "   -".to_string(),
                    }
                })
                .collect();
            // Vertical: down the middle of the bed.
            let down: Vec<String> = (0..8)
                .map(|i| {
                    let y = spec.ground_y + i * (spec.soil_depth / 8);
                    match row_mean(w, &spec, y, 0, spec.width) {
                        Some(m) => format!("{m:>4}"),
                        None => "   -".to_string(),
                    }
                })
                .collect();
            // Totals, split by which side of field capacity they sit on --
            // a ratchet moves water between these two without changing the
            // sum, a sink shrinks the sum.
            let (mut total, mut below) = (0u64, 0u64);
            for y in spec.ground_y..(spec.ground_y + spec.soil_depth) {
                for x in 0..spec.width {
                    let c = w.get(x, y);
                    if w.materials.get(c.material).water_capacity == 0 {
                        continue;
                    }
                    let m = update::soil_moisture(c) as u64;
                    total += m;
                    below += (material::SOIL_FIELD_CAPACITY as u64).saturating_sub(m);
                }
            }
            // **Which door the water left by -- and the label is now `net`,
            // which matters.** `evaporation::tick` credits
            // `atmospheric_bank` and `plant::transpire` credits nothing, so
            // bank growth used to be exactly the sun's take. Since
            // `weather::condense_under_a_lid` *spends* that bank to give the
            // water back, bank growth is evaporation **minus** condensation.
            // Reading it as "the sun" after that change would understate the
            // sink by however much came back, which is most of it. The
            // separation the two arms still support is the one that matters
            // here: a bed whose net is near zero is a closed loop, whichever
            // way the two gross flows are running.
            let banked = (w.atmospheric_bank - start_bank) * material::LIQUID_FULL as f64;
            let lost = start_total as f64 - total as f64;
            println!(
                "  frame {f:>6}  across [{}]  down [{}]  total {total:>9}  deficit {below:>8}  \
                 lost {lost:>9.0} = net-to-air {banked:>9.0} + rest {:>9.0}",
                across.join(" "),
                down.join(" "),
                lost - banked
            );
            // **The whole-bed minimum, and where.** The `across` buckets
            // above are means over an eighth of the world, so a dry patch
            // narrower than the bucket is averaged away -- the owner's report
            // is of a patch 15-20 cells wide, which is a third of one bucket.
            // A minimum cannot hide it.
            let reach = 5;
            let mut worst = (u16::MAX, -1i32);
            for x in 0..spec.width {
                if let Some(m) = column_surface(w, &spec, x, reach) {
                    if m < worst.0 {
                        worst = (m, x);
                    }
                }
            }
            // **How big the plant actually is, which is the scene check.**
            // A null about "what one plant does to the bed" is worth nothing
            // if the plant in the scene is a stunted seedling and the one in
            // the report is a mature specimen -- `CLAUDE.md`'s "check the
            // scene still contains the situation you think it does".
            let (mut plant_cells, mut roots) = (0u32, 0u32);
            for y in 0..spec.height {
                for x in 0..spec.width {
                    let c = w.get(x, y);
                    if w.materials.get(c.material).kind == material::MaterialKind::Plant {
                        plant_cells += 1;
                        if y >= spec.ground_y {
                            roots += 1;
                        }
                    }
                }
            }
            println!(
                "           plant cells {plant_cells} (below ground {roots}), organisms {}",
                w.live_organism_ids().len(),
            );
            println!(
                "           driest surface column {} at x={} (founder at {:?});                  columns under the wilting point {}, under half capacity {}",
                worst.0,
                worst.1,
                planted,
                (0..spec.width)
                    .filter(|&x| column_surface(w, &spec, x, reach)
                        .is_some_and(|m| m <= material::SOIL_WILTING_POINT))
                    .count(),
                (0..spec.width)
                    .filter(|&x| column_surface(w, &spec, x, reach)
                        .is_some_and(|m| m < material::SOIL_FIELD_CAPACITY / 2))
                    .count(),
            );
            // **The whole water ledger, because "lost" is not "destroyed".**
            //
            // `total` counts soil moisture only -- cells with a
            // `water_capacity`. Water standing as a `Liquid`, or sitting in
            // the atmospheric bank, has left `total` without leaving the
            // world, so reading a falling `total` as a leak is the mistake
            // `CLAUDE.md` names: a number that is arithmetically right and
            // about the wrong question. Everything the box can hold water in,
            // added up, so the residual is what is genuinely unaccounted.
            let mut liquid = 0u64;
            for y in 0..spec.height {
                for x in 0..spec.width {
                    let c = w.get(x, y);
                    if w.materials.kind(c.material) == material::MaterialKind::Liquid {
                        liquid += update::liquid_fill(c) as u64;
                    }
                }
            }
            let bank = w.atmospheric_bank * material::LIQUID_FULL as f64;
            let start_bank_fill = start_bank * material::LIQUID_FULL as f64;
            let accounted = total as f64 + liquid as f64 + bank;
            let began = start_total as f64 + start_bank_fill;
            println!(
                "           water ledger: soil {total} + standing liquid {liquid} + bank {bank:.0} \
                 = {accounted:.0} against {began:.0} at the start -> unaccounted {:.0}",
                began - accounted
            );
            // **Does the air under the lid actually get humid?**
            //
            // The next repair on the table is condensation -- water the sun
            // took returning to the ground, which is what a sealed box does
            // and why a terrarium never needs watering. The obvious trigger
            // is saturation, and before building a reader for that signal it
            // has to be shown the signal ever arrives: `CLAUDE.md`'s standing
            // check that a channel needs a writer *and* a reader, and that a
            // reader for something never written is dead code that looks
            // alive. Sampled at the ceiling, at mid-air, and just over the
            // ground, against `field::MAX_MOISTURE` of 4.0 and the
            // `HUMID_STOP` of 2.0 that switches drying off.
            let air_at = |y: i32| {
                let (mut sum, mut peak, mut n) = (0.0f32, 0.0f32, 0u32);
                for x in (0..spec.width).step_by(8) {
                    let m = w.field_at(x, y).moisture;
                    sum += m;
                    peak = peak.max(m);
                    n += 1;
                }
                (sum / n.max(1) as f32, peak)
            };
            let (ceil_mean, ceil_peak) = air_at(spec.ground_y - 200);
            let (mid_mean, mid_peak) = air_at(spec.ground_y - 100);
            let (low_mean, low_peak) = air_at(spec.ground_y - 8);
            println!(
                "           air humidity (max 4.0, drying stops at 2.0): ceiling {ceil_mean:.2}/{ceil_peak:.2}  \
                 mid {mid_mean:.2}/{mid_peak:.2}  just above ground {low_mean:.2}/{low_peak:.2}  (mean/peak)"
            );
            // **Covered vs bare, which is the owner's spatial question.**
            // `evaporation::is_damp_soil_surface` refuses any column whose
            // cell above is not empty, so a column under a stem, a leaf or a
            // scrap of litter cannot dry at all while the bare column beside
            // it does. If that is what shapes the patch, these two means
            // separate; if they do not separate, the shape is something else
            // and the shielding story is wrong.
            let (mut cs, mut cn, mut bs, mut bn) = (0u64, 0u64, 0u64, 0u64);
            for x in 0..spec.width {
                if let Some(m) = column_surface(w, &spec, x, reach) {
                    if covered(w, &spec, x) {
                        cs += m as u64;
                        cn += 1;
                    } else {
                        bs += m as u64;
                        bn += 1;
                    }
                }
            }
            println!(
                "           surface under cover {} over {cn} columns vs bare {} over {bn}",
                if cn > 0 { (cs / cn).to_string() } else { "-".into() },
                if bn > 0 { (bs / bn).to_string() } else { "-".into() },
            );
            if fine {
                // Per-column, around the plant, at the resolution the report
                // is about. `#` marks a column something is standing on, so
                // the shape can be read against the one predicate that gates
                // whether the sun reaches it.
                let c0 = planted.first().copied().unwrap_or(spec.width / 2);
                let strip: Vec<String> = (-24..=24)
                    .map(|i| {
                        let x = c0 + i * 4;
                        if x < 0 || x >= spec.width {
                            return "    -".to_string();
                        }
                        let mark = if covered(w, &spec, x) { '#' } else { ' ' };
                        match column_surface(w, &spec, x, reach) {
                            Some(m) => format!("{m:>4}{mark}"),
                            None => "    -".to_string(),
                        }
                    })
                    .collect();
                println!("           x={} +/- 96, every 4 columns ('#' = something standing on it):", c0);
                for chunk in strip.chunks(16) {
                    println!("             {}", chunk.join(""));
                }
            }
        }
        if f < frames {
            tick(&mut lab);
        }
    }

    if let Some(prefix) = png {
        shoot(&mut lab, &format!("{prefix}.png"), arg("zoom").unwrap_or(2));
    }
}

/// One frame of the bed with the soil-moisture overlay on, through the
/// shipped renderer and with no window.
///
/// **A full replace on the overlay's own ramp, which is `render.rs`'s job
/// here and not this file's** -- `CLAUDE.md`'s rule is that a debug readout
/// must not be a function of the thing it debugs, and `apply_organism_
/// overlay` already obeys it. All this does is switch the channel on, so the
/// card and the game show the identical picture; a bespoke ramp here would
/// be a second implementation to disagree with the one the owner is reading.
fn shoot(lab: &mut Lab, path: &str, zoom: u32) {
    let (w, h) = (pixel_physics::lab::WIDTH, pixel_physics::lab::HEIGHT);
    let mut buf = vec![0u8; (w * h * 4) as usize];
    lab.renderer.organism_overlay = pixel_physics::render::OrganismOverlay::SoilMoisture;
    let touched = lab.world.take_touched_chunks();
    lab.renderer.draw(&lab.world, &lab.particles, &touched, &mut buf, (w, h), true);
    let zoom = zoom.max(1);
    let (zw, zh) = (w * zoom, h * zoom);
    let mut out = vec![0u8; (zw * zh * 4) as usize];
    for y in 0..zh {
        for x in 0..zw {
            let src = (((y / zoom) * w + (x / zoom)) * 4) as usize;
            let dst = ((y * zw + x) * 4) as usize;
            out[dst..dst + 4].copy_from_slice(&buf[src..src + 4]);
        }
    }
    image::save_buffer(path, &out, zw, zh, image::ColorType::Rgba8).expect("write png");
    println!("  wrote {path} ({zw}x{zh}, soil-moisture overlay)");
}
