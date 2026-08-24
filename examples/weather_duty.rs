//! How much of the time is it raining, and therefore how much of the time
//! does `Renderer::draw` take its full-screen branch?
//!
//! `weather.is_precipitating()` is one of the triggers that forces a full
//! redraw, and a 1,200-frame run at the default seed measured it firing on
//! 89% of frames. That is a sample from inside one wet epoch, not a duty
//! cycle — `CLAUDE.md` is explicit that a bar set from a single run is a
//! sample from a wide distribution. This sweeps seeds and a long window so
//! the number quoted is the real one.

use pixel_physics::sim::weather;

fn main() {
    let frames: u64 = 200_000;
    let mut total = 0u64;
    println!("{:>8}  {:>10}  {:>12}", "seed", "raining", "longest spell");
    for seed in 0..12u64 {
        let mut wet = 0u64;
        let mut run = 0u64;
        let mut longest = 0u64;
        for f in 0..frames {
            if weather::at(seed, f).is_precipitating() {
                wet += 1;
                run += 1;
                longest = longest.max(run);
            } else {
                run = 0;
            }
        }
        total += wet;
        println!("{seed:>8}  {:>9.1}%  {longest:>12}", wet as f64 * 100.0 / frames as f64);
    }
    println!(
        "\n  across 12 seeds x {frames} frames: raining {:.1}% of the time",
        total as f64 * 100.0 / (12 * frames) as f64
    );
    println!("  every one of those frames repaints all 163,840 pixels, at ~72 ns each");
}
