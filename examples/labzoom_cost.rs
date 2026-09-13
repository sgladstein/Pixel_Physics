//! **Does giving the lab's zoomed-out view its own pixels cost the player any
//! simulation speed?** — the owner's question of 2026-09-13, *"Does this
//! impact performance in the lab?"*, in the units the dial actually shows.
//!
//! **Frame milliseconds are the wrong unit here, and that is the whole point
//! of this harness.** The lab is not a game you watch at 60 Hz; it is a box you
//! run fast and look at occasionally. `time::TimeControl` decouples the display
//! rate from the tick rate, so at the top of the speed dial most passes do not
//! draw at all and spend their whole budget ticking. What the player reads off
//! the screen is `achieved` — **simulated seconds per real second, render
//! included** — and that is what this reports.
//!
//! So the question is not *"what does a bigger buffer cost a frame"* (the
//! sandbox already answered that: 1.29x at x2, 1.66x at x4,
//! `Reports/zoom-out-resolution-2026-09-13.md`) but *"how much of the wall
//! clock is the render entitled to at this dial setting"* — and at 1024x it may
//! round to nothing.
//!
//! **This drives the real loop**, the same three calls `bin/lab.rs` makes:
//! `Lab::advance`, then `Lab::draw` only when the advance says to draw. Arms
//! are the pixel budget; rows are the dial.
//!
//! **The bed has to be bigger than the viewport or the whole thing is a
//! no-op**, and that is a finding rather than a harness detail: a box at or
//! below 512x320 cannot zoom out at all (`max_zoom_out_stride` derives the cap
//! from the world's own bounds), so the rung is 1, no power-of-two budget
//! divides it, and the buffer never grows. `box=` defaults to the 2048x1280 bed
//! the owner judged the pictures on.
//!
//! ```text
//! cargo run --release --example labzoom_cost
//! cargo run --release --example labzoom_cost -- seconds=4 founders=48 colonies=4
//! cargo run --release --example labzoom_cost -- box=512,320   # the no-op control
//! ```

use pixel_physics::lab::{scene::LabBox, time, Lab};
use std::time::{Duration, Instant};

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(str::to_string))
        .and_then(|v| v.parse().ok())
}

fn box_size() -> (i32, i32) {
    match arg::<String>("box") {
        Some(s) => {
            let (w, h) = s.split_once(',').expect("box=W,H");
            (w.trim().parse().expect("box width"), h.trim().parse().expect("box height"))
        }
        None => (2048, 1280),
    }
}

fn build(width: i32, height: i32, grow: u64) -> Lab {
    let base = LabBox::default();
    // The ground rides the height, as it does on the parameters page and in
    // `labzoom`: at the shipped `ground_y` a 1280-row box puts its soil in the
    // top eighth and the harness measures a scene error.
    let ground_y = base.ground_y * height / base.height.max(1);
    let mut lab = Lab::new(LabBox {
        width,
        height,
        ground_y,
        founders: arg("founders").unwrap_or(48),
        colonies: arg("colonies").unwrap_or(4),
        seed: arg("seed").unwrap_or(base.seed),
        ..base
    });
    // **Running, not paused.** `TimeControl` starts paused, which is right
    // for the app and useless here: a paused lab advances zero ticks and every
    // arm reports an achieved rate of zero. Caught by this harness's own tick
    // counter on its first run -- the "did it fire at all" check earning its
    // place.
    lab.time.toggle_phase();
    assert_eq!(lab.time.phase, time::Phase::Running, "the bed must be running or every arm measures nothing");
    for _ in 0..grow {
        lab.advance(Duration::from_millis(16));
    }
    lab
}

fn main() {
    let (bw, bh) = box_size();
    let grow: u64 = arg("grow").unwrap_or(3000);
    let seconds: f64 = arg("seconds").unwrap_or(3.0);
    let dials: Vec<u32> = match arg::<String>("dials") {
        Some(s) => s.split(',').filter_map(|v| v.trim().parse().ok()).collect(),
        None => vec![1, 16, 256, 1024],
    };

    println!("labzoom_cost: box={bw}x{bh} grow={grow} seconds={seconds} dials={dials:?}");
    println!("  ACHIEVED is simulated seconds per real second -- what the dial reads out, render");
    println!("  included. That is the number the player feels; frame ms is not.");
    println!("  DRAWS is how many of the passes actually rendered, and TICKS/DRAW is the");
    println!("  amortisation: a render paid once per many ticks is a different proposition");
    println!("  from one paid every tick.\n");

    // One lab per arm, all grown identically, so the arms compare the same box.
    for budget in [1, 2, 4] {
        let mut lab = build(bw, bh, grow);
        lab.pixel_scale_cap = pixel_physics::app::MAX_PIXEL_SCALE;
        lab.pixel_budget = budget;
        // Zoom all the way out -- the only place the budget does anything.
        // Through `zoom_within`, which is what the key does, so the rung the
        // arm ends on is the rung the player can actually reach on this box.
        let bounds = lab.world.bounds();
        for _ in 0..8 {
            lab.renderer.zoom_within(-1, (pixel_physics::lab::WIDTH, pixel_physics::lab::HEIGHT), bounds);
        }
        let scale = lab.pixel_scale();
        let (vw, vh) = lab.viewport();
        let mut frame = vec![0u8; (vw * vh * 4) as usize];
        let b = lab.world.bounds().expect("the bed has bounds");
        println!(
            "=== budget x{budget}: rung {}, scale x{scale}, buffer {vw}x{vh}, box {}x{}",
            lab.renderer.zoom_out_stride,
            b.max_x - b.min_x + 1,
            b.max_y - b.min_y + 1
        );
        if scale == 1 && budget > 1 {
            println!("    (the budget buys nothing at this rung -- see the module doc)");
        }
        println!("{:>8}  {:>10}  {:>9}  {:>8}  {:>11}  {:>10}", "dial", "achieved", "vs x1", "draws", "ticks/draw", "ticks");
        for &dial in &dials {
            lab.time.requested = dial;
            lab.time.set_display_hz(60);
            // Warm: one draw so the buffer is valid and the skip has a frame.
            lab.draw(&mut frame, 60.0);
            let start = Instant::now();
            let (mut ticks, mut draws) = (0u64, 0u64);
            let mut last = Instant::now();
            while start.elapsed().as_secs_f64() < seconds {
                let now = Instant::now();
                let elapsed = now.duration_since(last);
                last = now;
                let advance = lab.advance(elapsed);
                ticks += advance.ticks as u64;
                if advance.draw {
                    lab.draw(&mut frame, 60.0);
                    draws += 1;
                }
            }
            let real = start.elapsed().as_secs_f64();
            // Simulated seconds advanced over real seconds elapsed. `achieved`
            // on `TimeControl` is the same quantity over a rolling window; this
            // is computed here so the window length cannot flatter a short run.
            let achieved = ticks as f64 / 60.0 / real;
            ACH.with(|a| {
                let mut a = a.borrow_mut();
                let base = *a.entry(dial).or_insert(achieved);
                println!(
                    "{dial:>8}  {achieved:>9.1}x  {:>8.2}x  {draws:>8}  {:>11.1}  {ticks:>10}",
                    achieved / base,
                    if draws == 0 { f64::INFINITY } else { ticks as f64 / draws as f64 }
                );
            });
        }
        println!();
    }
    println!("  Read 'vs x1' per row: it is this budget's achieved rate against x1's at the SAME");
    println!("  dial. 1.00x means the buffer cost the player no simulation speed at that setting.");
}

thread_local! {
    /// x1's achieved rate per dial, so every later arm is compared against the
    /// same row rather than against its own first row.
    static ACH: std::cell::RefCell<std::collections::HashMap<u32, f64>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}
