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

    // **Line one, every parameter.** See the module doc.
    println!(
        "wind_probe: preset={preset} seed={seed} seeds={seeds} wind={wind:+.2} \
         world={WIDTH}x{HEIGHT} step={step} fetch_columns={} half_slope={} neutral={}",
        weather::FETCH_COLUMNS,
        weather::EXPOSURE_HALF_SLOPE,
        weather::NEUTRAL_EXPOSURE,
    );

    if seeds > 1 {
        sweep(&preset, seed, seeds, wind);
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
