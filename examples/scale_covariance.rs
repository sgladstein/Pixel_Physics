//! Is worldgen **scale-covariant**? Does the same seed at `k` times the cell
//! resolution build the same landscape, `k` times as large in cells?
//!
//! This is the question the resolution step turns on
//! (`Reports/resolution-step-2026-08-29.md`). Its content half is "make every
//! feature `k` times as many cells across", and the cheapest possible version
//! of that is `WorldgenParams::scaled(k)` — reinterpret the shape parameters
//! and generate a world `k` times as big in cells. That only *works* if the
//! generator is covariant under the substitution: every length in cells and
//! every position sampled through a wavelength, so column `kx` of the big
//! world lands at the same phase of the same noise as column `x` of the small
//! one and comes out `k` times as high.
//!
//! It is not obvious that it is, and it is cheap to check, so: check before
//! building anything on it.
//!
//! ```text
//! cargo run --release --example scale_covariance
//! cargo run --release --example scale_covariance -- k=2 w=512 h=320 preset=canyon
//! ```
//!
//! Reports the elevation profile of the small world against the big one's,
//! rescaled back down. A covariant generator gives residuals at the rounding
//! floor; anything else names which term did not scale.

use pixel_physics::worldgen::{column::Terrain, WorldgenParams, WorldgenPresets};

fn main() {
    let (mut k, mut w, mut h, mut seed) = (2.0f32, 512i32, 320i32, 1u64);
    let mut preset = String::new();
    for arg in std::env::args().skip(1) {
        let Some((name, v)) = arg.split_once('=') else { continue };
        match name {
            "k" => k = v.parse().expect("k=FACTOR"),
            "w" => w = v.parse().expect("w=CELLS"),
            "h" => h = v.parse().expect("h=CELLS"),
            "seed" => seed = v.parse().expect("seed=N"),
            "preset" => preset = v.to_string(),
            _ => panic!("unknown argument {arg:?}"),
        }
    }
    let (presets, err) = WorldgenPresets::load();
    if let Some(e) = err {
        panic!("{e}");
    }
    let name = if preset.is_empty() { presets.default_name() } else { preset.clone() };
    let base = presets.get(&name).expect("known preset").clone();
    let big_params = base.scaled(k);
    // Echoed, per `CLAUDE.md` -- a harness that does not name its own
    // parameters cannot be told from one built before they existed.
    println!("preset {name}, seed {seed}, k={k}, small {w}x{h} -> big {}x{}", (w as f32 * k) as i32, (h as f32 * k) as i32);

    let (soil_tan, sand_tan) = (33.0_f32.to_radians().tan(), 34.0_f32.to_radians().tan());
    let small = Terrain::new(seed, &base, w, h, soil_tan, sand_tan);
    let big = Terrain::new(seed, &big_params, (w as f32 * k) as i32, (h as f32 * k) as i32, soil_tan, sand_tan);

    // Column x of the small world is column kx of the big one, and its
    // elevation should be k times as deep.
    let mut worst = 0.0f32;
    let mut worst_at = 0;
    let mut sum = 0.0f64;
    let mut n = 0usize;
    let mut small_span = (f32::MAX, f32::MIN);
    for x in 0..w {
        let a = small.elev(x);
        let b = big.elev((x as f32 * k) as i32) / k;
        let d = (a - b).abs();
        if d > worst {
            worst = d;
            worst_at = x;
        }
        sum += d as f64;
        n += 1;
        small_span = (small_span.0.min(a), small_span.1.max(a));
    }
    let relief = small_span.1 - small_span.0;
    println!("small world's own relief: {relief:.1} rows (elev {:.1}..{:.1})", small_span.0, small_span.1);
    println!(
        "rescaled residual vs the small world: mean {:.3} rows, worst {worst:.3} rows at column {worst_at}",
        sum / n as f64
    );
    println!("worst as a share of the relief it is measured against: {:.2}%", worst * 100.0 / relief.max(1.0));

    // **Which term broke it.** `region_variation` is documented as "how far
    // the regions of a world stray from the preset -- zero makes a world
    // uniform end to end", and `region.rs` keys its window to a fixed width
    // *in cells*, so a world with twice the cells gets twice the regions
    // rather than the same regions twice as wide. If that is what defeats
    // covariance, turning it off should collapse the residual; if the
    // residual survives, the regions are innocent and something else is
    // keyed to absolute position.
    let flat = WorldgenParams { region_variation: 0.0, ..base.clone() };
    let flat_big = flat.scaled(k);
    let fs = Terrain::new(seed, &flat, w, h, soil_tan, sand_tan);
    let fb = Terrain::new(seed, &flat_big, (w as f32 * k) as i32, (h as f32 * k) as i32, soil_tan, sand_tan);
    let mut flat_sum = 0.0f64;
    let mut flat_span = (f32::MAX, f32::MIN);
    for x in 0..w {
        let a = fs.elev(x);
        flat_sum += (a - fb.elev((x as f32 * k) as i32) / k).abs() as f64;
        flat_span = (flat_span.0.min(a), flat_span.1.max(a));
    }
    println!(
        "with region_variation=0: mean {:.3} rows against its own relief of {:.1}",
        flat_sum / n as f64,
        flat_span.1 - flat_span.0
    );

    // The control this needs, per `CLAUDE.md`: a number that cannot move is
    // not evidence. Compare the small world against a *differently seeded*
    // one the same way -- if the residual above is small because `elev` is
    // flat rather than because the scaling worked, this will be small too.
    let other = Terrain::new(seed.wrapping_add(1), &base, w, h, soil_tan, sand_tan);
    let mut ctrl = 0.0f64;
    for x in 0..w {
        ctrl += (small.elev(x) - other.elev(x)).abs() as f64;
    }
    println!(
        "CONTROL, same size but seed {} instead: mean {:.3} rows -- this is what 'unrelated' looks like",
        seed.wrapping_add(1),
        ctrl / n as f64
    );
}
