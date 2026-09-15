//! **What a pheromone pass costs, at a world size you choose.**
//!
//! Exists to answer one question with one number, for the owner's acceptance
//! condition on widening the planes (2026-09-15): *"More memory is fine as
//! long as speed is unchanged."*
//!
//! **Deliberately written to compile against BOTH the `u8` and the `u16`
//! plane**, which is the whole reason it is a separate file from
//! `pherolife`. Nothing here names a width: every value that touches the
//! plane is inferred from `pheromone::DEPOSIT`, so the same source builds on
//! either side of the change and the two binaries differ only by the library.
//! An instrument that only compiles on the new side cannot measure the old
//! one, and a measurement of the new side alone answers nothing.
//!
//! **`world=` is the load-bearing argument.** At 512x320 a plane is ~160 KB
//! and lives in cache, so a width change is pure arithmetic and the memory
//! cost is invisible. The shipped world is 8192x2560 -- ~20 M cells, ~20 MB
//! at a byte and ~40 MB at two -- which is nowhere near cache, and that is
//! the regime a storage change could actually be slow in. `CLAUDE.md`: the
//! current 512x320 world is a test environment, not the target.
//!
//! ```text
//! RAYON_NUM_THREADS=4 cargo run --release --example pherocost -- world=512x320
//! RAYON_NUM_THREADS=4 cargo run --release --example pherocost -- world=4096x2048 passes=120
//! ```

use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::pheromone::{self, Channel, Pheromones};

fn arg(key: &str) -> Option<String> {
    std::env::args().find_map(|a| a.strip_prefix(key).map(str::to_string))
}

fn main() {
    let dims = arg("world=").unwrap_or_else(|| "512x320".into());
    let (w, h) = dims.split_once('x').map_or((512i32, 320i32), |(a, b)| (a.parse().unwrap_or(512), b.parse().unwrap_or(320)));
    let passes: u64 = arg("passes=").and_then(|v| v.parse().ok()).unwrap_or(600);
    let reps: usize = arg("reps=").and_then(|v| v.parse().ok()).unwrap_or(5);
    let interval = pheromone::PHEROMONE_INTERVAL;

    println!("pherocost: world={w}x{h} ({} cells) passes={passes} reps={reps} threads={:?}", w as i64 * h as i64, std::env::var("RAYON_NUM_THREADS").ok());

    let mut best = f64::MAX;
    let mut tiles = 0u64;
    for _ in 0..reps {
        let mut p = Pheromones::new(Rect::new(0, 0, w - 1, h - 1));
        let t = std::time::Instant::now();
        for pass in 1..=passes {
            // **Re-laid every pass so the tiles stay awake.** A load that
            // decays away measures the sleep check rather than the per-cell
            // work, which is what the first version of this measurement did
            // (744 tiles across 600 passes) before the mistake was spotted.
            // Spread down the world so several tile rows are live, not one.
            let mut y = 8;
            while y < h - 8 {
                for x in 8..(w - 8) {
                    p.deposit(Channel::A, x, y, pheromone::DEPOSIT);
                }
                y += 64;
            }
            p.step(pass * interval, interval);
        }
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        best = best.min(ms);
        tiles = p.stats.tiles_processed;
    }

    // **The minimum, not the mean**: every source of error on a shared box
    // adds time and none removes it, so the fastest run is the closest to the
    // work. `tiles` is the load-independent counter that says the two builds
    // did comparable work -- read it before the clock.
    println!("  {passes} passes: {best:.2} ms (best of {reps})");
    println!("  tiles processed: {tiles}");
    println!("  per tile: {:.5} ms", best / tiles.max(1) as f64);
}
