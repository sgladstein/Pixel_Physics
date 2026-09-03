//! What [`weather::exposure`] reads across a generated landscape, in numbers.
//!
//! The companion to `filmstrip`'s `channel=exposure` sheet, and deliberately
//! second to it: the sheet answers *what and where* — which side of a hill
//! is dark — and this answers *how much*. Both are needed. A corrected
//! overlay in this repo was still misread as "everything at the ramp floor"
//! when the real value was 40% of scale, and no amount of looking settles a
//! question that is quantitative (`CLAUDE.md`, method).
//!
//! ```text
//! cargo run --release --example wind_probe -- preset=rolling seed=7
//! cargo run --release --example wind_probe -- preset=flat        # the control
//! cargo run --release --example wind_probe -- seeds=32           # the sweep
//! ```
//!
//! **The first line echoes every parameter it ran with.** A harness that
//! does not name its inputs is one nobody can tell is disconnected — a
//! 3.5-hour study here was invalidated exactly that way — and this one has
//! more inputs than usual, because the answer depends on the preset, the
//! seed *and* the wind direction it was asked about.

use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::weather;
use pixel_physics::sim::world::World;
use pixel_physics::worldgen;

const WIDTH: i32 = 512;
const HEIGHT: i32 = 320;

fn arg<T: std::str::FromStr>(key: &str, default: T) -> T
where
    T::Err: std::fmt::Debug,
{
    std::env::args()
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().expect(key)))
        .unwrap_or(default)
}

/// One generated world, at the shipped viewport size.
fn build(preset: &str, seed: u64) -> World {
    let mut w = World::new(Rect::new(0, 0, WIDTH - 1, HEIGHT - 1));
    let (presets, err) = worldgen::WorldgenPresets::load();
    if let Some(e) = err {
        panic!("{e}");
    }
    let name = if preset.is_empty() { presets.default_name().to_string() } else { preset.to_string() };
    let Some(params) = presets.get(&name) else { panic!("unknown preset {name:?}") };
    worldgen::generate(&mut w, worldgen::Spec::Generated { params, seed });
    w
}

/// The surface row of every column, so the transect can print the ground it
/// is talking about next to the number it read there.
fn surface(world: &World, x: i32) -> Option<i32> {
    (0..HEIGHT).find(|&y| world.get(x, y).material != pixel_physics::sim::material::EMPTY)
}

fn main() {
    let preset: String =
        std::env::args().find_map(|a| a.strip_prefix("preset=").map(str::to_string)).unwrap_or_else(|| "rolling".into());
    let seed: u64 = arg("seed", 7);
    let seeds: u64 = arg("seeds", 1);
    let wind: f32 = arg("wind", 1.0);
    let step: i32 = arg("step", 16);
    let gusts: u64 = arg("gusts", 0);

    // **Line one, every parameter.** See the module doc.
    println!(
        "wind_probe: preset={preset} seed={seed} seeds={seeds} wind={wind:+.2} \
         world={WIDTH}x{HEIGHT} step={step} gusts={gusts} fetch_columns={} half_slope={} neutral={}",
        weather::FETCH_COLUMNS,
        weather::EXPOSURE_HALF_SLOPE,
        weather::NEUTRAL_EXPOSURE,
    );

    if seeds > 1 {
        sweep(&preset, seed, seeds, wind);
        return;
    }
    if gusts > 0 {
        gust_census(&preset, seed, gusts);
        return;
    }
    if let Some(n) = std::env::args().find_map(|a| a.strip_prefix("velocity=").map(|v| v.parse::<u64>().expect("velocity"))) {
        velocity_census(&preset, seed, n);
        return;
    }

    let world = build(&preset, seed);
    println!("  x     surf  fetch   shelter  prominence   exposure");
    let mut lo: Option<(i32, f32)> = None;
    let mut hi: Option<(i32, f32)> = None;
    let mut sum = 0.0f64;
    let mut n = 0u32;
    let mut flat_fetch = 0u32;
    for x in (0..WIDTH).step_by(step.max(1) as usize) {
        let Some(s) = surface(&world, x) else {
            println!("{x:5}      --     --         --          --         -- (empty column)");
            continue;
        };
        let e = weather::exposure_detail(&world, x, s, wind);
        println!(
            "{x:5}  {s:6}  {:5}  {:8.3}  {:10.3}  {:9.3}",
            e.fetch, e.shelter, e.prominence, e.value
        );
        sum += e.value as f64;
        n += 1;
        if e.fetch < 8 {
            flat_fetch += 1;
        }
        if lo.is_none_or(|(_, v)| e.value < v) {
            lo = Some((x, e.value));
        }
        if hi.is_none_or(|(_, v)| e.value > v) {
            hi = Some((x, e.value));
        }
    }
    let mean = sum / n.max(1) as f64;
    println!("  columns {n}  mean {mean:.3}");
    if let (Some((lx, lv)), Some((hx, hv))) = (lo, hi) {
        println!("  most sheltered x={lx} exposure {lv:.3}   most exposed x={hx} exposure {hv:.3}   spread {:.3}", hv - lv);
    }
    // **The disconnected-harness check.** A walk that keeps stopping after a
    // column or two is reading cliffs, not fetch, and every number above is
    // then about something else.
    if flat_fetch > 0 {
        println!("  NOTE: {flat_fetch} of {n} columns reached fewer than 8 columns of fetch (cliff or world edge).");
    }
}

/// The same reading over many seeds, reported as an order statistic.
///
/// `scripts/seedsweep.sh`'s lesson, applied here: outcomes over procedural
/// content are chaotic in the seed, and a single-seed spread gets
/// rubber-stamped. What has to hold is that *most* worlds of a preset put a
/// real distance between their most sheltered and most exposed ground.
fn sweep(preset: &str, first: u64, count: u64, wind: f32) {
    let mut spreads: Vec<f32> = Vec::new();
    let mut means: Vec<f32> = Vec::new();
    // Raw, pre-saturation slopes, kept so `EXPOSURE_RELIEF` can be set from
    // the terrain's own distribution rather than by eye.
    let mut slopes: Vec<f32> = Vec::new();
    for seed in first..first + count {
        let world = build(preset, seed);
        let (mut lo, mut hi, mut sum, mut n) = (f32::MAX, f32::MIN, 0.0f32, 0u32);
        for x in (0..WIDTH).step_by(4) {
            let Some(s) = surface(&world, x) else { continue };
            let d = weather::exposure_detail(&world, x, s, wind);
            let v = d.value;
            if d.shelter > 0.0 {
                slopes.push(d.shelter);
            }
            if d.prominence > 0.0 {
                slopes.push(d.prominence);
            }
            lo = lo.min(v);
            hi = hi.max(v);
            sum += v;
            n += 1;
        }
        if n == 0 {
            continue;
        }
        spreads.push(hi - lo);
        means.push(sum / n as f32);
        println!("  seed {seed:5}  min {lo:.3}  max {hi:.3}  spread {:.3}  mean {:.3}", hi - lo, sum / n as f32);
    }
    spreads.sort_by(f32::total_cmp);
    means.sort_by(f32::total_cmp);
    let pct = |v: &[f32], p: f32| v[((v.len() as f32 - 1.0) * p).round() as usize];
    println!(
        "  spread over {} seeds: min {:.3}  p10 {:.3}  median {:.3}  p90 {:.3}  max {:.3}",
        spreads.len(),
        pct(&spreads, 0.0),
        pct(&spreads, 0.10),
        pct(&spreads, 0.50),
        pct(&spreads, 0.90),
        pct(&spreads, 1.0),
    );
    println!("  mean exposure: median {:.3}  (level ground reads {:.3})", pct(&means, 0.50), weather::NEUTRAL_EXPOSURE);
    slopes.sort_by(f32::total_cmp);
    if !slopes.is_empty() {
        println!(
            "  raw slope (cells/column), {} nonzero terms: p25 {:.3}  median {:.3}  p75 {:.3}  p90 {:.3}  p99 {:.3}  max {:.3}",
            slopes.len(),
            pct(&slopes, 0.25),
            pct(&slopes, 0.50),
            pct(&slopes, 0.75),
            pct(&slopes, 0.90),
            pct(&slopes, 0.99),
            pct(&slopes, 1.0),
        );
    }
}

/// Every gust `frames` frames of weather would fire, and what each delivered.
///
/// **The discrete count that goes in a review card's `meta`.** A picture of
/// a windy world cannot show whether the exposure scaling is what produced
/// it -- `CLAUDE.md`'s standing example is a collapse read as "chunks are
/// working" whose body count was zero for the whole run. So this counts the
/// gusts, and prints what each one delivered against what it would have
/// delivered before this package existed.
///
/// Reads `weather::planned_gust`, the same function `weather::gust` itself
/// uses to decide, rather than reimplementing the schedule here.
fn gust_census(preset: &str, seed: u64, frames: u64) {
    let mut world = build(preset, seed);
    world.seed = seed;

    // The two ends of this world, found before any gust fires, so the
    // sample points are the terrain's own extremes and not cherry-picked.
    let reference_wind = 1.0;
    let (mut lo, mut hi, mut lo_x, mut hi_x) = (f32::MAX, f32::MIN, 0, 0);
    for x in 0..WIDTH {
        let Some(s) = surface(&world, x) else { continue };
        let v = weather::exposure(&world, x, s, reference_wind);
        if v < lo {
            lo = v;
            lo_x = x;
        }
        if v > hi {
            hi = v;
            hi_x = x;
        }
    }
    println!("  sample points at wind {reference_wind:+.2}: sheltered x={lo_x} exposure {lo:.3} | exposed x={hi_x} exposure {hi:.3}");

    let (mut fired, mut sum_delivered, mut sum_unsheltered) = (0u64, 0.0f64, 0.0f64);
    let (mut weakest, mut strongest) = (f32::MAX, f32::MIN);
    // The frames those two landed on, so a contact sheet can be rendered at
    // the exact moment rather than at a frame chosen by hand and hoped over.
    let (mut weakest_at, mut strongest_at) = ((0u64, 0i32, 0.0f32), (0u64, 0i32, 0.0f32));
    let mut sheltered_half = 0u64;
    for frame in 0..frames {
        world.frame = frame;
        let w = weather::at(seed, frame);
        let Some(g) = weather::planned_gust(&world, w) else { continue };
        fired += 1;
        sum_delivered += g.delivered as f64;
        sum_unsheltered += g.unsheltered as f64;
        let share = g.delivered / g.unsheltered;
        if share < weakest {
            weakest = share;
            weakest_at = (frame, g.x, g.exposure);
        }
        if share > strongest {
            strongest = share;
            strongest_at = (frame, g.x, g.exposure);
        }
        if g.exposure < weather::NEUTRAL_EXPOSURE {
            sheltered_half += 1;
        }
    }
    if fired == 0 {
        println!("  gusts fired: 0 over {frames} frames -- nothing to report, and that is the finding");
        return;
    }
    println!(
        "  gusts fired: {fired} over {frames} frames ({:.2}% of frames), {sheltered_half} of them over ground below neutral",
        100.0 * fired as f64 / frames as f64,
    );
    println!(
        "  delivered pressure: {:.1} total against {:.1} unsheltered ({:.1}% of the old fixed strength)",
        sum_delivered,
        sum_unsheltered,
        100.0 * sum_delivered / sum_unsheltered
    );
    println!("  per-gust share of full strength: weakest {weakest:.3}  strongest {strongest:.3}");
    println!(
        "  weakest gust:   frame {} at x={} over exposure {:.3}",
        weakest_at.0, weakest_at.1, weakest_at.2
    );
    println!(
        "  strongest gust: frame {} at x={} over exposure {:.3}",
        strongest_at.0, strongest_at.1, strongest_at.2
    );
}

/// **Is there any wind at seed height?** — the prerequisite for a wind-borne
/// powder, asked before building one.
///
/// `CLAUDE.md`: *check that a planned step can demonstrate itself, before
/// promising it will*. `Reports/plant-engine-rethink-2026-09-03.md` §7 item 6
/// proposes giving a falling seed the same downwind bias `update_gas` already
/// gives a gas cell, so that dispersal distance varies with the weather. The
/// whole proposal rests on one unmeasured quantity: **does the field's
/// velocity channel carry anything in the rows a seed actually falls
/// through?** Not in the sky, where the gust is delivered, and not next to a
/// blast — near the ground, in ordinary windy weather.
///
/// It reads the gas rule's own scale rather than copying its numbers, which
/// is the point of comparing against it: `update::WIND_BIAS_THRESHOLD` is
/// where a gas stops treating the wind as numerical residue, and
/// `update::WIND_BIAS_FULL_SPEED` is where its bias saturates. A powder rule
/// of the same shape would behave the same way, so what those two thresholds
/// catch here is what such a rule would deliver.
///
/// **The negative control runs every time and is not optional.** A pinned
/// `Clear` sky has `wind: 0.0`, so no gust ever fires and the same census
/// must read essentially zero. Without it a windy reading proves only that
/// the field has numbers in it — the solver settles to *near* zero rather
/// than to zero, which is exactly why `WIND_BIAS_THRESHOLD` is not zero.
///
/// ```text
/// cargo run --release --example wind_probe -- velocity=4000 preset=rolling seed=7
/// ```
fn velocity_census(preset: &str, seed: u64, frames: u64) {
    use pixel_physics::sim::parallel;
    use pixel_physics::sim::update;

    // How far above the surface a falling seed is sampled. A seed is borne
    // from a crown and falls; the rows that decide where it lands are the
    // ones between the crown and the ground, so the band is measured from
    // the surface upward rather than at one height.
    const BAND: i32 = 30;

    println!(
        "  band = surface-1 .. surface-{BAND}, every {}th column | gas-rule scale: threshold {} full-speed {} prevailing drift {}",
        8,
        update::WIND_BIAS_THRESHOLD,
        update::WIND_BIAS_FULL_SPEED,
        update::PREVAILING_DRIFT,
    );
    for (label, pin) in [("GALE (wind 0.95)", weather::Pin::Gale), ("CLEAR -- control", weather::Pin::Clear)] {
        let mut world = build(preset, seed);
        world.seed = seed;
        world.set_weather_pin(pin);
        for _ in 0..frames {
            parallel::step(&mut world);
            world.step_active_sites();
            world.step_fields();
        }
        let mut mags: Vec<f32> = Vec::new();
        for x in (0..WIDTH).step_by(8) {
            let Some(surf) = surface(&world, x) else { continue };
            for dy in 1..=BAND {
                let y = surf - dy;
                if y < 0 {
                    continue;
                }
                mags.push(world.field_at(x, y).vx.abs());
            }
        }
        mags.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = mags.len().max(1);
        let mean = mags.iter().sum::<f32>() / n as f32;
        let at = |q: f32| mags[((n - 1) as f32 * q) as usize];
        // The two counts are the answer, not the mean: what a bias rule
        // delivers is decided per cell, so what matters is how many cells
        // are over the threshold at all, and how many are anywhere near
        // saturation.
        let over = mags.iter().filter(|&&v| v > update::WIND_BIAS_THRESHOLD).count();
        let full = mags.iter().filter(|&&v| v >= update::WIND_BIAS_FULL_SPEED).count();
        println!(
            "  {label:<18} |vx| mean {mean:.4} p50 {:.4} p90 {:.4} p99 {:.4} max {:.4} | over threshold {over}/{n} ({:.1}%) at full speed {full} ({:.2}%)",
            at(0.50),
            at(0.90),
            at(0.99),
            mags[n - 1],
            100.0 * over as f32 / n as f32,
            100.0 * full as f32 / n as f32,
        );
    }
    println!(
        "  -- read `over threshold`. A mean is the wrong statistic here: a bias rule decides per cell, \\
         so a channel that is zero almost everywhere and strong in a few places disperses seeds in a few \\
         places, which is what dispersal wants. The CLEAR row is the control and must be ~0; if it is not, \\
         the GALE row is measuring the solver's residue."
    );
}
