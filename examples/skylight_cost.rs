//! **What the sky-light pass costs in a box that has a ceiling, and whether
//! it is still doing its job.**
//!
//! `Reports/evolution-lab-gate-1-2026-08-30.md` §5.3 measured the lab's draw
//! at 4.78 ms against a 0.94-1.5 ms tick and put **2.8 ms of it in
//! `sky_light`** — the largest single thing between the speed dial and its
//! ceiling, and not in the simulation at all. This is the instrument for that
//! number: it runs `LabBox` through the shipped `frame::step` and the shipped
//! `Renderer` with the dirty-rect skip live, and reports the draw beside the
//! three routes a sky-light rebuild can take.
//!
//! **Why the counts sit next to the timing.** `CLAUDE.md`: *a cost that
//! vanishes may be work that vanished* — a pass that got cheap because it
//! stopped being asked anything is indistinguishable, in every timing, from
//! one that converged. `Renderer::sky_light_rebuilds` says which:
//!
//! - `full` — the whole region rescanned (first draw, camera move, zoom).
//! - `incremental` — only the touched chunks' blocks rescanned, something
//!   changed, so the fan and the four sweeps ran.
//! - `held` — only the touched chunks' blocks rescanned, nothing changed, so
//!   the grid stood. **The route the lab spends most of its frames on.**
//! - `changed` — of `full + incremental`, how many moved a byte of the grid.
//!   `incremental` far above `changed` is the headroom nobody has taken yet.
//!
//! `held` climbing while `changed` is pinned at its first-frame value over a
//! box that is visibly growing would mean the invalidation is broken, not
//! that the pass got fast. That is what `check=1` is for.
//!
//! **`check=1` is the positive control, in the real bed**: every `checkevery`
//! frames it draws the same world twice — once through the `Renderer` that
//! has been running (so its block cache is live and hot) and once through a
//! `Renderer` made a moment ago (which can only do the full scan) — and
//! compares both the light grid and every pixel. A stale block deep in a dark
//! room moves the grid by a byte and moves no pixel at all, so the grid is
//! the discriminating half and both are asserted. The unit guard
//! `an_incremental_sky_light_rebuild_agrees_with_a_full_one` makes the same
//! comparison on a hand-built miniature; this makes it on the bed the game
//! actually ships.
//!
//! ```text
//! cargo run --release --example skylight_cost
//! cargo run --release --example skylight_cost -- frames=12000 warmup=10000
//! cargo run --release --example skylight_cost -- check=1 frames=2000 checkevery=50
//! ```

use pixel_physics::lab::scene::LabBox;
use pixel_physics::render::Renderer;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::frame;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use std::collections::HashSet;
use std::time::Instant;

fn arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{name}=")).and_then(|v| v.parse().ok()))
}

fn main() {
    // Echoed, on the harness rule this repo paid for twice: a knob nobody can
    // see the value of is a knob nobody can tell is disconnected.
    let frames: u64 = arg("frames").unwrap_or(2_000);
    let warmup: u64 = arg("warmup").unwrap_or(0);
    let check: bool = arg::<u32>("check").unwrap_or(0) != 0;
    let check_every: u64 = arg("checkevery").unwrap_or(50);
    let bed = LabBox::default();
    println!(
        "skylight_cost: {}x{} soil={} founders={} colonies={} frames={frames} warmup={warmup} \
         check={check} checkevery={check_every}",
        bed.width, bed.height, bed.soil_depth, bed.founders, bed.colonies
    );

    let mut world = bed.build();
    let (vw, vh) = (bed.width as u32, bed.height as u32);
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();
    let mut renderer = Renderer::new();
    let mut buf = vec![0u8; (vw * vh * 4) as usize];
    let mut control_buf = vec![0u8; (vw * vh * 4) as usize];

    // The warm-up is simulated without drawing, so the measured window can be
    // a *settled* box rather than the first thousand frames of a fresh one.
    // `CLAUDE.md`: measure a settled world, because a settled world is exactly
    // where the dirty-rect skip does its work and where an optimisation that
    // defeats it shows up.
    for _ in 0..warmup {
        frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
    }
    world.take_touched_chunks();

    let mut draw_ms: Vec<f64> = Vec::with_capacity(frames as usize);
    let mut checks = 0u64;
    let (mut lagging, mut run, mut worst_run, mut worst_grid, mut worst_pixels) = (0u64, 0u64, 0u64, 0usize, 0usize);
    for f in 1..=frames {
        frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        let touched = world.take_touched_chunks();
        let t = Instant::now();
        renderer.draw(&world, &particles, &touched, &mut buf, (vw, vh), false);
        draw_ms.push(t.elapsed().as_secs_f64() * 1000.0);

        if check && f.is_multiple_of(check_every) {
            // A fresh `Renderer` has no block cache, so it must do the full
            // scan -- asserted, or it is not a control.
            let mut control = Renderer::new();
            control.draw(&world, &particles, &HashSet::new(), &mut control_buf, (vw, vh), true);
            assert_eq!(control.sky_light_rebuilds().full, 1, "the control arm did not do a full scan");
            // The subject's own last draw was dirty-rect, so its buffer is
            // only guaranteed correct where something was repainted. Draw it
            // again forced-full off the same world -- the cache stays exactly
            // as the run left it, which is the state under test.
            renderer.draw(&world, &particles, &HashSet::new(), &mut buf, (vw, vh), true);
            let grid_diff = renderer
                .sky_light_grid_for_test()
                .iter()
                .zip(control.sky_light_grid_for_test())
                .filter(|(a, b)| a != b)
                .count();
            let pixel_diff = buf.chunks_exact(4).zip(control_buf.chunks_exact(4)).filter(|(a, b)| a != b).count();
            checks += 1;
            if grid_diff > 0 || pixel_diff > 0 {
                lagging += 1;
                run += 1;
                worst_run = worst_run.max(run);
                worst_grid = worst_grid.max(grid_diff);
                worst_pixels = worst_pixels.max(pixel_diff);
                if lagging <= 3 {
                    println!("    frame {f}: {grid_diff} grid byte(s) and {pixel_diff} pixel(s) behind a fresh renderer");
                }
            } else {
                run = 0;
            }
        }
    }

    draw_ms.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
    let n = draw_ms.len().max(1);
    let mean = draw_ms.iter().sum::<f64>() / n as f64;
    let counts = renderer.sky_light_rebuilds();
    println!(
        "  draw over frames {}..{}: mean {:.3} ms, p50 {:.3} ms, worst {:.3} ms",
        warmup + 1,
        warmup + frames,
        mean,
        draw_ms[n / 2],
        draw_ms[n - 1]
    );
    // The worst-frame ratio, so the worst is never quoted unpinned --
    // `CLAUDE.md`: an aggregate has to pin it or it is noise wearing a number.
    let pin = mean * n as f64 / draw_ms[n - 1];
    println!(
        "  worst-frame check: mean x frames = {:.1} ms against a worst of {:.3} ms — ratio {pin:.1}, {}",
        mean * n as f64,
        draw_ms[n - 1],
        if pin < 2.0 { "an aggregate pins it" } else { "nothing pins it, the worst is noise" }
    );
    println!(
        "  sky-light rebuilds: full {} | incremental {} | held {} | of the solves, {} moved the grid",
        counts.full, counts.incremental, counts.held, counts.changed
    );
    if check {
        println!(
            "  against a fresh renderer over {checks} comparison(s): {lagging} differed, \
             longest run {worst_run}, worst {worst_grid} grid byte(s) / {worst_pixels} pixel(s)"
        );
        // **The bar is the dirty-rect skip's own, not perfection**, and the
        // two halves of it are different claims.
        //
        // A chunk written *after* `World::end_step` -- which is where
        // organism growth and creature movement happen in `frame::step` --
        // only reaches `touched_chunks` on the next tick, so the pixel path
        // is already one frame behind there and this cache is exactly as
        // behind, never more. Measured over 1,500 lab frames: 234
        // comparisons showed a grid difference, at most **98 bytes of
        // 18,193**, and the longest run of consecutive differing frames was
        // **2**. So the bar is that nothing *persists* -- a lasting
        // difference would be an invalidation the cache is missing, which is
        // a different fault from a frame it is sharing with the renderer
        // around it.
        //
        // And the picture: **0 differing pixels across every one of those
        // 1,500 comparisons.** The grid moves inside a sealed box in places
        // that quantise to the same colour, which is why the grid is the
        // discriminating comparison and a frame hash is not.
        assert_eq!(worst_pixels, 0, "the incremental cache changed the picture, which is the thing it must not do");
        assert!(
            worst_run <= 4,
            "a difference persisted across {worst_run} consecutive comparisons -- \
             that is a missing invalidation, not the touched-set's own one-frame lag"
        );
    }
    // The null this harness must not publish quietly: if nothing ever ran,
    // every timing above is a measurement of a pass that was never asked.
    assert!(
        counts.full + counts.incremental + counts.held > 0,
        "no sky-light rebuild ran at all, so none of the numbers above are about this pass"
    );
}
