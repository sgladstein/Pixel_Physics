//! **What `Lab::draw` costs, box alone and with pages open, paused and
//! running.** `cargo run --release --example labdraw`
//!
//! Built for step 2 of the lab interface redesign (crisp text,
//! `Reports/lab-ui-redesign-2026-10-22.md`), whose rule is that the interface
//! layer ships only if it costs no more than today. So every arm is timed
//! both ways in the same process, on the same warmed box: `1x` is
//! `Lab::draw` into one 512x320 buffer, today's path; `layer` is
//! `Lab::draw_layered`, the box into its buffer and the interface into a
//! 1024x640 layer that is repainted only when something on it changed.
//!
//! Draws are paced at 60 Hz, as the window's are. Arms (each one timed over
//! `draws=` draws after 20 untimed, the whole set
//! repeated `passes=` times in alternating order so a drift in the machine
//! lands on both sides):
//!
//! - `box` -- paused, no pointer, nothing open;
//! - `box+ptr` -- paused, pointer resting over the box (today this forces a
//!   full redraw every frame);
//! - `pages` -- paused, ANTS page and BIOSPHERE open, no pointer;
//! - `pages+move` -- the same with the pointer moving one pixel a frame over
//!   the box, so the layer is repainted on every draw (its worst case);
//! - `run` and `run+pages` -- one tick before every draw (not timed), the
//!   box alone and with both pages open.
//!
//! `box=W,H` builds a bed of that size (the report's step 2 table was read on
//! 1024x640); `frames=N` warms it for N ticks first (default 900).
//!
//! **The `layer` column includes everything the binary does on the CPU for a
//! frame**: clearing and repainting the layer when it is due. The GPU upload
//! of the layer (2.6 MB, only on a repaint) and the composite are not in it;
//! the `repaints` column says how many of the timed draws paid for one.

use std::time::{Duration, Instant};

use pixel_physics::lab::ui::{Action, Panel};
use pixel_physics::lab::{scene::LabBox, Lab, HEIGHT, WIDTH};

#[derive(Clone, Copy, PartialEq)]
enum Arm {
    Box,
    BoxPtr,
    Pages,
    PagesMove,
    Run,
    RunPages,
}

impl Arm {
    fn name(self) -> &'static str {
        match self {
            Arm::Box => "box",
            Arm::BoxPtr => "box+ptr",
            Arm::Pages => "pages",
            Arm::PagesMove => "pages+move",
            Arm::Run => "run",
            Arm::RunPages => "run+pages",
        }
    }
    fn pages(self) -> bool {
        matches!(self, Arm::Pages | Arm::PagesMove | Arm::RunPages)
    }
}

struct Timing {
    mean: f64,
    p90: f64,
    max: f64,
    repaints: usize,
}

fn time_arm(lab: &mut Lab, arm: Arm, layered: bool, draws: usize, shot: Option<&str>) -> Timing {
    // The pages, opened or shut to match the arm.
    let want_ants = arm.pages();
    if (lab.ui.panel == Some(Panel::Ants)) != want_ants {
        lab.act(Action::Panel(Panel::Ants));
    }
    if lab.stats.showing() != want_ants {
        lab.act(Action::Stats);
    }
    let (w, h) = lab.viewport();
    let mut frame = vec![0u8; (w * h * 4) as usize];
    let mut layer = vec![0u8; (WIDTH * 2 * HEIGHT * 2 * 4) as usize];
    let running = matches!(arm, Arm::Run | Arm::RunPages);
    let mut samples = Vec::with_capacity(draws);
    let mut repaints = 0;
    let mut next = Instant::now();
    for i in 0..(20 + draws) {
        match arm {
            Arm::BoxPtr => lab.set_cursor(Some((256, 150))),
            Arm::PagesMove => lab.set_cursor(Some((300 + (i % 2) as i32, 150))),
            _ => lab.set_cursor(None),
        }
        // **Paced at 60 Hz**, as the window is: the layer's repaint rules
        // read the wall clock (`lab::CHROME_LIVE`), so draws fired back to
        // back would let it skip repaints no real frame could.
        if let Some(wait) = next.checked_duration_since(Instant::now()) {
            std::thread::sleep(wait);
        }
        next = Instant::now() + Duration::from_micros(16_667);
        if running {
            lab.tick_for_harness();
        }
        let t = Instant::now();
        let repainted = if layered {
            lab.draw_layered(&mut frame, &mut layer, 60.0)
        } else {
            lab.draw(&mut frame, 60.0);
            true
        };
        let dt = t.elapsed();
        if i >= 20 {
            samples.push(dt.as_secs_f64() * 1e3);
            repaints += repainted as usize;
        }
    }
    if let Some(prefix) = shot {
        let name = format!("{prefix}-{}-{}.png", arm.name(), if layered { "layer" } else { "1x" });
        let (img, size) = if layered {
            (pixel_physics::render::composite_layer(&frame, (w, h), &layer, (WIDTH * 2, HEIGHT * 2)), (WIDTH * 2, HEIGHT * 2))
        } else {
            (frame.clone(), (w, h))
        };
        image::save_buffer(&name, &img, size.0, size.1, image::ColorType::Rgba8).expect("write a shot");
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Timing {
        mean: samples.iter().sum::<f64>() / samples.len() as f64,
        p90: samples[samples.len() * 9 / 10],
        max: *samples.last().unwrap(),
        repaints,
    }
}

fn main() {
    let mut frames = 900u32;
    let mut draws = 200usize;
    let mut passes = 2usize;
    let mut size: Option<(i32, i32)> = None;
    let mut only: Option<String> = None;
    let mut shot: Option<String> = None;
    let mut path: Option<String> = None;
    for arg in std::env::args().skip(1) {
        match arg.split_once('=') {
            Some(("frames", v)) => frames = v.parse().expect("frames=N"),
            Some(("draws", v)) => draws = v.parse().expect("draws=N"),
            Some(("passes", v)) => passes = v.parse().expect("passes=N"),
            Some(("box", v)) => {
                let (w, h) = v.split_once(',').expect("box=W,H");
                size = Some((w.parse().expect("W"), h.parse().expect("H")));
            }
            // `arm=run+pages` and `path=layer` narrow the run to one arm or
            // one path, for a profiler.
            Some(("arm", v)) => only = Some(v.to_string()),
            Some(("path", v)) => path = Some(v.to_string()),
            // `shot=PREFIX` writes each arm's last 1x frame (and, on the
            // branch with the layer, its composite) as PREFIX-ARM-PATH.png, so
            // two builds can be compared byte for byte on one ticked box.
            Some(("shot", v)) => shot = Some(v.to_string()),
            _ => eprintln!("ignoring unknown argument {arg:?}"),
        }
    }
    let mut spec = LabBox::default();
    if let Some((w, h)) = size {
        spec.ground_y = spec.ground_y * h / spec.height.max(1);
        spec.width = w;
        spec.height = h;
    }
    println!("labdraw: bed {}x{}, frames={frames} draws={draws} passes={passes}", spec.width, spec.height);
    let mut lab = Lab::new(spec);
    lab.show_help = false;
    // Ticked, not advanced: `advance` spends a wall-clock budget, so the box
    // two runs time would be two different boxes.
    for _ in 0..frames {
        lab.tick_for_harness();
    }
    assert!(lab.world.frame > 0, "the box never ticked");
    println!("world frame {}, viewport {:?}", lab.world.frame, lab.viewport());

    let arms = [Arm::Box, Arm::BoxPtr, Arm::Pages, Arm::PagesMove, Arm::Run, Arm::RunPages];
    println!("{:<11} {:>6} {:>8} {:>8} {:>8} {:>9}", "arm", "path", "mean ms", "p90 ms", "max ms", "repaints");
    for pass in 0..passes {
        for &arm in arms.iter().filter(|a| only.as_deref().is_none_or(|o| o == a.name())) {
            let order = if pass % 2 == 0 { [false, true] } else { [true, false] };
            for layered in order {
                if path.as_deref().is_some_and(|p| p != if layered { "layer" } else { "1x" }) {
                    continue;
                }
                let t = time_arm(&mut lab, arm, layered, draws, shot.as_deref());
                println!(
                    "{:<11} {:>6} {:>8.3} {:>8.3} {:>8.3} {:>9}",
                    arm.name(),
                    if layered { "layer" } else { "1x" },
                    t.mean,
                    t.p90,
                    t.max,
                    t.repaints
                );
            }
        }
    }
}
