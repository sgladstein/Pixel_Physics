//! Standing census of one-cell-tall water films — open bugs #1, "whiskers".
//!
//! The companion to `filmstrip`'s picture, and deliberately second to it:
//! the sheet answers *what and where* a comb of detached ledges is, and
//! this answers *how many*, *how full*, and *how long it lasts*.
//!
//! **Three ways this bug has now been measured wrong, and the third is the
//! one this file exists to stop.** `CLAUDE.md` records the first two: a film
//! defined as "water with air above and below" is *what falling water looks
//! like*, so a raw per-frame count counts every droplet in the world; and
//! attributing film *creation* blamed the plain straight-down fall for 76%
//! of them, true and useless, because those films exist for one frame each.
//! The obvious correction — count films that **persist at the same cell**
//! for N frames — is the third mistake, and it is worse than either, because
//! it reads **zero on a scene where the comb is unmistakable by eye**.
//! Measured on `fall` with the `LIQUID_SETTLE_DROP` fix disabled: 296 film
//! cells and 247 of them in horizontal runs of six or more, standing at that
//! level for hundreds of frames, and **not one cell survived 3 frames**
//! (lifetime p50 1, max 2). The comb is a *travelling* structure. Every cell
//! in it is replaced every frame as the front advances one diagonal step,
//! so anything keyed by position sees a world of one-frame droplets.
//!
//! So the headline number here is **per-row streak**: for each row, how many
//! *consecutive* frames it has held a horizontal run of six or more films.
//! That is what "a comb sitting on the front" means as a quantity, and it
//! separates the artifact (rows holding a comb for hundreds of frames) from
//! a shower of droplets (no row holds one twice running).
//!
//! ```text
//! cargo run --release --example film_probe -- scene=fall frames=400
//! cargo run --release --example film_probe -- scene=pour frames=1200 report=200
//! ```
//!
//! Scenes are copied from `filmstrip`'s `build`, cell for cell, including
//! `water_at`'s shade jitter and the attached stone floor — see
//! `examples/common/mod.rs`'s own doc for what it cost the last time two
//! harnesses were compared as if they were the same scene and were not.

use std::collections::HashMap;

use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::{Rect, CHUNK_SIZE};
use pixel_physics::sim::material;
use pixel_physics::sim::parallel;
use pixel_physics::sim::rng;
use pixel_physics::sim::world::World;

const WIDTH: i32 = 512;
const HEIGHT: i32 = 320;
const FLOOR_THICKNESS: i32 = 8;

/// A horizontal run of this many films is a *comb*, not a droplet.
///
/// Six, matching `update.rs`'s
/// `a_spreading_front_does_not_shed_a_comb_of_detached_ledges`, so the two
/// numbers are comparable. That test's own doc records why runs and not
/// cells: `find_lateral_descent` on gave 277 cells in runs of 6+ against 13
/// with it off, where the raw film count barely moved.
const COMB_RUN: usize = 6;

fn water_at(x: i32, y: i32) -> Cell {
    Cell::new(material::WATER, (rng::jitter(x, y) * 255.0) as u8)
}

fn stone_floor(w: &mut World) {
    for x in 0..WIDTH {
        for y in (HEIGHT - FLOOR_THICKNESS)..HEIGHT {
            w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
        }
    }
}

fn build(scene: &str) -> World {
    let mut w = World::new(Rect::new(0, 0, WIDTH - 1, HEIGHT - 1));
    let floor_y = HEIGHT - FLOOR_THICKNESS;
    match scene {
        "pour" => {
            stone_floor(&mut w);
            for x in 0..200 {
                for y in 30..floor_y {
                    w.set(x, y, water_at(x, y));
                }
            }
        }
        "fall" => {
            stone_floor(&mut w);
            for x in 20..250 {
                for y in 20..200 {
                    w.set(x, y, water_at(x, y));
                }
            }
        }
        "waterbed" => {
            stone_floor(&mut w);
            for x in 20..492 {
                for y in 120..floor_y {
                    w.set(x, y, water_at(x, y));
                }
            }
        }
        // A shelf pour: water dropped onto a narrow unwalled ledge in open
        // air, so a front spreads with *nothing under it* for most of its
        // length. The geometry the render-side reading of this bug assumes —
        // if a persistent film lives anywhere it lives here, not on a floor.
        "shelf" => {
            stone_floor(&mut w);
            for x in 180..332 {
                for y in 200..204 {
                    w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
                }
            }
            for x in 236..276 {
                for y in 120..190 {
                    w.set(x, y, water_at(x, y));
                }
            }
        }
        "evaporate" => {
            let floor = 160;
            for x in 0..WIDTH {
                for y in floor..(floor + 6) {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            for (x0, width) in [(40, 6), (120, 240)] {
                for y in (floor - 4)..floor {
                    w.set(x0 - 1, y, Cell::new(material::STONE, 0));
                    w.set(x0 + width, y, Cell::new(material::STONE, 0));
                    for x in x0..(x0 + width) {
                        w.set(x, y, water_at(x, y));
                    }
                }
            }
        }
        other => panic!("unknown scene {other:?}; known: pour, fall, waterbed, shelf, evaporate"),
    }
    w
}

/// The raw material test, not `Cell::is_empty()`, per `CLAUDE.md`: the
/// managed-aware version answers "is this position available", and the
/// question here is "is there material here".
fn is_air(w: &World, x: i32, y: i32) -> bool {
    w.get(x, y).material == material::EMPTY
}

fn is_film(w: &World, x: i32, y: i32) -> bool {
    w.get(x, y).material == material::WATER && is_air(w, x, y - 1) && is_air(w, x, y + 1)
}

/// `update::liquid_fill` is crate-private; this is the same rule, and the
/// same trap: `aux == 0` on a `Liquid` means **full**, not empty.
fn fill(c: Cell) -> u16 {
    if c.aux() == 0 {
        material::LIQUID_FULL
    } else {
        c.aux()
    }
}

/// Rows of open air between a film and the first thing under it. `None`
/// means nothing under it at all before the world edge.
fn drop_below(w: &World, x: i32, y: i32) -> Option<i32> {
    let mut d = 0;
    let mut yy = y + 1;
    while yy < HEIGHT {
        if !is_air(w, x, yy) {
            return Some(d);
        }
        d += 1;
        yy += 1;
    }
    None
}

fn main() {
    let arg = |k: &str| std::env::args().skip(1).find_map(|a| a.strip_prefix(k).map(|v| v.to_string()));
    let scene = arg("scene=").unwrap_or_else(|| "fall".into());
    let frames: usize = arg("frames=").map(|v| v.parse().expect("frames")).unwrap_or(400);
    let report_every: usize = arg("report=").map(|v| v.parse().expect("report")).unwrap_or(50);

    let mut w = build(&scene);
    // The wrong instrument, kept and reported so the next session does not
    // re-invent it: film age keyed by cell position. See the module doc.
    let mut age: HashMap<(i32, i32), u32> = HashMap::new();
    let mut lifetimes: Vec<u32> = Vec::new();
    // The right one: how many consecutive frames each row has held a comb.
    let mut row_streak = vec![0u32; HEIGHT as usize];
    let mut longest_streak = vec![0u32; HEIGHT as usize];

    let mut peak_comb = 0usize;
    let mut peak_comb_frame = 0usize;
    let mut comb_frames = 0usize;
    let mut comb_cell_total = 0u64;
    // Fill histogram over every comb cell seen in the whole run, in tenths.
    let mut fill_hist = [0u64; 11];
    let mut seam_cells = 0u64;
    // The same histogram over *every* film cell, comb or not. The question
    // it answers is whether a render-side treatment keyed on fill -- draw a
    // sub-threshold film as a partial row, or dim it toward the sky -- has
    // anything to act on. If the films are full cells, it does not.
    let mut all_fill_hist = [0u64; 11];
    let mut hang_hist = [0u64; 5];
    let (mut last_films, mut last_comb) = (0usize, 0usize);

    for frame in 0..frames {
        parallel::step(&mut w);
        w.step_liquid_bodies();
        w.step_active_sites();
        w.step_fields();

        let mut next: HashMap<(i32, i32), u32> = HashMap::new();
        let mut films = 0usize;
        let mut comb_cells = 0usize;
        let mut comb_rows = 0usize;
        let mut comb_positions: Vec<(i32, i32)> = Vec::new();

        for y in 1..HEIGHT - 1 {
            let mut run: Vec<i32> = Vec::new();
            let mut row_has_comb = false;
            // The trailing `None` flushes a run that reaches the right edge.
            for x in 1..WIDTH {
                let filmy = x < WIDTH - 1 && is_film(&w, x, y);
                if filmy {
                    films += 1;
                    next.insert((x, y), age.get(&(x, y)).copied().unwrap_or(0) + 1);
                    let f = fill(w.get(x, y)) as usize * 10 / material::LIQUID_FULL as usize;
                    all_fill_hist[f.min(10)] += 1;
                    run.push(x);
                } else {
                    if run.len() >= COMB_RUN {
                        row_has_comb = true;
                        comb_cells += run.len();
                        for &rx in &run {
                            comb_positions.push((rx, y));
                        }
                    }
                    run.clear();
                }
            }
            if row_has_comb {
                comb_rows += 1;
                row_streak[y as usize] += 1;
                longest_streak[y as usize] = longest_streak[y as usize].max(row_streak[y as usize]);
            } else {
                row_streak[y as usize] = 0;
            }
        }

        for (pos, a) in age.iter() {
            if !next.contains_key(pos) {
                lifetimes.push(*a);
            }
        }
        age = next;

        for &(x, y) in &comb_positions {
            let f = fill(w.get(x, y)) as usize * 10 / material::LIQUID_FULL as usize;
            fill_hist[f.min(10)] += 1;
            if y % CHUNK_SIZE == 0 || (y + 1) % CHUNK_SIZE == 0 {
                seam_cells += 1;
            }
            let bucket = match drop_below(&w, x, y) {
                Some(0..=1) => 0,
                Some(2..=3) => 1,
                Some(4..=9) => 2,
                Some(_) => 3,
                None => 4,
            };
            hang_hist[bucket] += 1;
        }
        comb_cell_total += comb_cells as u64;
        if comb_cells > 0 {
            comb_frames += 1;
        }
        if comb_cells > peak_comb {
            peak_comb = comb_cells;
            peak_comb_frame = frame;
        }
        last_films = films;
        last_comb = comb_cells;

        if frame % report_every == 0 || frame + 1 == frames {
            let live_streaks: Vec<u32> = row_streak.iter().copied().filter(|s| *s > 0).collect();
            let max_live = live_streaks.iter().copied().max().unwrap_or(0);
            println!(
                "frame {frame:5}: films {films:5}  comb cells {comb_cells:5} in {comb_rows:3} rows  \
                 rows holding a comb right now {:3} (longest current streak {max_live} frames)",
                live_streaks.len()
            );
            if !comb_positions.is_empty() {
                let (mut x0, mut x1, mut y0, mut y1) = (WIDTH, 0, HEIGHT, 0);
                for &(x, y) in &comb_positions {
                    x0 = x0.min(x);
                    x1 = x1.max(x);
                    y0 = y0.min(y);
                    y1 = y1.max(y);
                }
                println!("    comb bbox x {x0}..{x1} y {y0}..{y1}");
            }
        }
    }

    for a in age.values() {
        lifetimes.push(*a);
    }
    lifetimes.sort_unstable();
    let n = lifetimes.len();
    let pct = |p: usize| if n == 0 { 0 } else { lifetimes[(n * p / 100).min(n - 1)] };

    println!("\n== {scene}, {frames} frames ==");
    println!(
        "comb cells: peak {peak_comb} (frame {peak_comb_frame}), mean over the run {:.1}, \
         present in {comb_frames}/{frames} frames ({:.0}%)",
        comb_cell_total as f64 / frames as f64,
        100.0 * comb_frames as f64 / frames as f64
    );
    let best_row = longest_streak.iter().enumerate().max_by_key(|(_, s)| **s).map(|(y, s)| (y, *s)).unwrap_or((0, 0));
    let rows_30 = longest_streak.iter().filter(|s| **s >= 30).count();
    let rows_100 = longest_streak.iter().filter(|s| **s >= 100).count();
    println!(
        "row streaks (consecutive frames a row held a comb): longest {} frames at row {}; \
         rows reaching 30 frames {rows_30}, 100 frames {rows_100}",
        best_row.1, best_row.0
    );
    println!(
        "cell lifetimes (the wrong instrument, reported to show it is wrong): \
         {n} episodes, p50 {} p90 {} p99 {} max {}",
        pct(50),
        pct(90),
        pct(99),
        lifetimes.last().copied().unwrap_or(0)
    );
    let total: u64 = fill_hist.iter().sum();
    if total > 0 {
        let hist: Vec<String> = fill_hist
            .iter()
            .enumerate()
            .filter(|(_, c)| **c > 0)
            .map(|(i, c)| format!("{}0-{}0%:{:.1}%", i, i + 1, 100.0 * *c as f64 / total as f64))
            .collect();
        println!("comb-cell fill: {}", hist.join(" "));
        println!(
            "comb cells by drop to the first thing below: 0-1 rows {:.1}%, 2-3 {:.1}%, 4-9 {:.1}%, 10+ {:.1}%, nothing below {:.1}%",
            100.0 * hang_hist[0] as f64 / total as f64,
            100.0 * hang_hist[1] as f64 / total as f64,
            100.0 * hang_hist[2] as f64 / total as f64,
            100.0 * hang_hist[3] as f64 / total as f64,
            100.0 * hang_hist[4] as f64 / total as f64,
        );
        println!("comb cells on a horizontal chunk-seam row: {:.1}%", 100.0 * seam_cells as f64 / total as f64);
    }
    let all_total: u64 = all_fill_hist.iter().sum();
    if all_total > 0 {
        let hist: Vec<String> = all_fill_hist
            .iter()
            .enumerate()
            .filter(|(_, c)| **c > 0)
            .map(|(i, c)| format!("{}0-{}0%:{:.1}%", i, i + 1, 100.0 * *c as f64 / all_total as f64))
            .collect();
        println!("ALL film cells ({all_total} seen): fill {}", hist.join(" "));
        let sub: u64 = all_fill_hist[..4].iter().sum();
        println!("  of those, below 40% fill: {:.1}% -- what a fill-keyed render treatment could act on", 100.0 * sub as f64 / all_total as f64);
    }
    println!("final frame: films {last_films}, comb cells {last_comb}");
    // The control `CLAUDE.md` asks for: proof this was a liquid run at all,
    // and not an empty world quietly reporting zero of everything.
    let mut water = 0u64;
    let mut volume = 0u64;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let c = w.get(x, y);
            if c.material == material::WATER {
                water += 1;
                volume += fill(c) as u64;
            }
        }
    }
    println!("water cells {water}, volume {:.1} full-cell equivalents", volume as f64 / material::LIQUID_FULL as f64);
}
