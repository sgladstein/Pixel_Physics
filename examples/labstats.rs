//! **The biosphere page, in text and in a picture, with the controls that
//! say whether its numbers can move.**
//!
//! `src/lab/stats.rs` is a page of counters over a living box, which is the
//! exact shape `CLAUDE.md` says lies most readily: *"a number that is
//! arithmetically correct and answers a different question than the one asked
//! looks exactly like a result"*, six occurrences across two sessions. So
//! this harness is built around **positive controls** rather than around a
//! render — each `control=` mode constructs a box whose answer is known in
//! advance, and the run either reports it or the instrument is broken.
//!
//! ```text
//! cargo run --release --example labstats                        # the standard bed, dumped
//! cargo run --release --example labstats -- control=empty       # no founders, no colony
//! cargo run --release --example labstats -- control=plants      # plants only
//! cargo run --release --example labstats -- control=ants        # animals only
//! cargo run --release --example labstats -- control=cull        # kill half the stand mid-run
//! cargo run--release --example labstats -- control=steady       # is anything riding a day-length cycle
//! cargo run --release --example labstats -- control=cost        # what one census costs
//! cargo run --release --example labstats -- png=page.png frames=9000
//! ```
//!
//! **It echoes its own parameters on the first line.** `CLAUDE.md`'s harness
//! rule, from a 3.5-hour study that produced eight byte-identical logs
//! because the binary predated the argument: a knob nobody can see the value
//! of is a knob nobody can tell is disconnected.

use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::stats::{self, Stats};
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

fn main() {
    let control: String = arg("control").unwrap_or_else(|| "run".to_string());
    let frames: u64 = arg("frames").unwrap_or(9_000);
    let png: Option<String> = arg("png");
    let seed: u64 = arg("seed").unwrap_or(1);

    // Each control names the box it needs, so the case whose answer is known
    // and the case being reported are the same object.
    let (founders, colonies) = match control.as_str() {
        "empty" => (0, 0),
        "plants" => (arg("founders").unwrap_or(8), 0),
        "ants" => (0, arg("colonies").unwrap_or(1)),
        _ => (arg("founders").unwrap_or(8), arg("colonies").unwrap_or(1)),
    };
    let spec = LabBox {
        width: arg("width").unwrap_or(512),
        height: arg("height").unwrap_or(320),
        soil_depth: arg("soil").unwrap_or(80),
        founders,
        colonies,
        compartments: arg("walls").unwrap_or(1),
        seed,
        ..LabBox::default()
    };
    println!(
        "labstats: control={control} frames={frames} founders={founders} colonies={colonies} walls={} soil={} seed={seed} png={}",
        spec.compartments,
        spec.soil_depth,
        png.as_deref().unwrap_or("-")
    );

    if control == "cost" {
        return cost(spec, frames);
    }

    let mut lab = Lab::new(spec);
    // The cull control needs a moment of stand to cull; everything else runs
    // straight through.
    let cull_at = if control == "cull" { frames / 2 } else { u64::MAX };
    let mut culled = 0usize;

    for f in 0..=frames {
        if f == cull_at {
            culled = cull_half(&mut lab.world);
            println!("  frame {f:>7}: CULLED {culled} of the living organisms");
        }
        // `Lab::advance` is the real path and it is wall-clock bounded, which
        // makes a headless run non-deterministic. Ticking directly and calling
        // `observe` by hand runs the identical sequence with the clock taken
        // out -- the page never sees the difference, since it samples on
        // `world.frame`.
        lab.stats.observe(&lab.world);
        if f % 900 == 0 || f == frames {
            line(&lab.stats, &lab.world);
        }
        if f < frames {
            tick(&mut lab);
        }
    }

    println!("\n--- the page, as text ---");
    for row in stats::dump(&lab.stats, &lab.world) {
        println!("  {row}");
    }

    check(&control, &lab, culled);

    if let Some(path) = png {
        let mut buf = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        let touched = lab.world.take_touched_chunks();
        lab.renderer.draw(&lab.world, &lab.particles, &touched, &mut buf, (WIDTH, HEIGHT), true);
        // A cursor over the first row, so the hover the page carries is in
        // the picture rather than described in a caption.
        let hover: Option<i32> = arg("hover");
        match hover {
            Some(row) => lab.stats.draw_at(&mut buf, &lab.world, Some((WIDTH as i32 - 200, row))),
            None => lab.stats.draw(&mut buf, &lab.world),
        }
        // **Nearest-neighbour, integer factor.** The page is 5x7 glyphs on a
        // 512-wide framebuffer and the review queue's own note is that the
        // stills the owner has been able to judge are 700-950 px across; a
        // smoothing resize would turn a 5x7 glyph into a smear, which is the
        // exact failure `Reports/lanes/creature-lane-g.md` spent an hour on.
        let zoom: u32 = arg("zoom").unwrap_or(2);
        let (zw, zh) = (WIDTH * zoom, HEIGHT * zoom);
        let mut big = vec![0u8; (zw * zh * 4) as usize];
        for y in 0..zh {
            for x in 0..zw {
                let src = (((y / zoom) * WIDTH + (x / zoom)) * 4) as usize;
                let dst = ((y * zw + x) * 4) as usize;
                big[dst..dst + 4].copy_from_slice(&buf[src..src + 4]);
            }
        }
        image::save_buffer(&path, &big, zw, zh, image::ColorType::Rgba8)
            .expect("writing the page");
        println!("wrote {path} ({zw}x{zh}, {zoom}x)");
    }
}

fn tick(lab: &mut Lab) {
    // The same sequence `Lab::advance` runs, reached directly so a headless
    // run is deterministic. `frame::step` is the one shared tick both
    // binaries call.
    pixel_physics::sim::frame::step(
        &mut lab.world,
        &mut lab.particles,
        &mut lab.blasts,
        pixel_physics::sim::player::PlayerInput::default(),
        &pixel_physics::sim::player::Tuning::default(),
    );
}

/// One greppable line per stop: everything the page's headline says, plus the
/// counters the rates are built from.
fn line(stats: &Stats, world: &World) {
    let Some(c) = stats.census() else {
        println!("  (no census)");
        return;
    };
    println!(
        "  frame {:>7}: plants {:>5} ({:>6} cells, size {:>3}/{:>3}/{:>4})  bank {:>5}  animals {:>4} ({:>5} cells)  senescent {:>4} | borne {:>5} sprouted {:>5} | born {:>4} died {:>4} refused {:>3} | gen p{} a{} {:?} | slots {}/{} | lines {} top {:.0}%",
        c.frame,
        c.plants,
        c.plant_cells,
        c.plant_size.low as i64,
        c.plant_size.mid as i64,
        c.plant_size.high as i64,
        // **The bank beside the stand, never pooled into it.** Measured
        // 2026-09-01 on the standard bed: at frame 30,000 the pooled figure
        // was 467 over a bed holding 48 plants and 419 ungerminated seeds,
        // which is the owner's *"5-7 obvious plants, but the count is way
        // higher like 200+"* with the arithmetic behind it.
        c.seed_bank,
        c.animals,
        c.animal_cells,
        c.senescent,
        c.seeds_borne,
        c.germinations,
        world.creature_stats.births,
        world.creature_stats.deaths,
        c.refused,
        c.plant_generation,
        c.animal_generation,
        c.generations,
        c.slots_used,
        c.slots_ceiling,
        c.lineages,
        c.top_lineage * 100.0,
    );
}

/// **The controls.** Each asserts what the box was built to guarantee, so a
/// counter that cannot move is caught here rather than believed on screen.
fn check(control: &str, lab: &Lab, culled: usize) {
    let Some(c) = lab.stats.census() else {
        println!("VERDICT: no census at all");
        return;
    };
    let history = lab.stats.history();
    let ok = |name: &str, pass: bool, said: String| {
        println!("  [{}] {name}: {said}", if pass { "PASS" } else { "FAIL" });
        pass
    };
    println!("\n--- control: {control} ---");
    let mut all = true;
    match control {
        // A box with nothing in it. Every population figure must be zero --
        // this is the specificity half: does the instrument stay quiet when
        // there is genuinely nothing to report.
        "empty" => {
            all &= ok("no plants", c.plants == 0, format!("plants {}", c.plants));
            all &= ok("no animals", c.animals == 0, format!("animals {}", c.animals));
            all &= ok("no biomass", c.biomass() == 0, format!("biomass {}", c.biomass()));
            all &= ok("no seeds", c.germinations == 0, format!("sprouted {}", c.germinations));
            all &= ok("no generations", c.generations.iter().sum::<u32>() == 0, format!("{:?}", c.generations));
        }
        // Plants only. The plant side must be non-zero and the animal side
        // must be zero -- the sensitivity half and the specificity half in
        // one box, which is what stops "animals 0" being read as the census
        // being blind.
        "plants" => {
            all &= ok("plants alive", c.plants > 0, format!("plants {}", c.plants));
            all &= ok("plant biomass", c.plant_cells > 0, format!("{} cells", c.plant_cells));
            all &= ok("no animals", c.animals == 0, format!("animals {}", c.animals));
            all &= ok("no animal biomass", c.animal_cells == 0, format!("{} cells", c.animal_cells));
            all &= ok("no breed margin", c.breed.is_none(), format!("{:?}", c.breed));
        }
        // Animals only. The mirror.
        "ants" => {
            all &= ok("animals alive", c.animals > 0, format!("animals {}", c.animals));
            all &= ok("animal biomass", c.animal_cells > 0, format!("{} cells", c.animal_cells));
            all &= ok("no plants", c.plants == 0, format!("plants {}", c.plants));
            all &= ok("a breed margin", c.breed.is_some(), format!("{:?}", c.breed));
            all &= ok("a species name", c.animal_species.is_some(), format!("{:?}", c.animal_species));
        }
        // **The sensitivity control the rest cannot give.** A count that is
        // right about a settled box may still be a constant; killing half the
        // stand mid-run and watching the strip fall is the one check that says
        // the number moves with the thing it names.
        "cull" => {
            let peak = history.iter().map(|s| s.plants).max().unwrap_or(0);
            all &= ok("something was culled", culled > 0, format!("{culled} organisms"));
            all &= ok(
                "the strip fell",
                (c.plants as u32) < peak,
                format!("peak {peak} -> {} now", c.plants),
            );
            let peak_cells = history.iter().map(|s| s.plant_cells).max().unwrap_or(0);
            all &= ok(
                "biomass fell",
                (c.plant_cells as u32) < peak_cells,
                format!("peak {peak_cells} -> {} now", c.plant_cells),
            );
        }
        // **Is anything in a sealed, sky-held box riding a day-length
        // cycle?** `CLAUDE.md`: a designed oscillator must be divided out of
        // every number it reaches, and the lab holds the light so the biggest
        // one is gone by construction. This measures whether that is true
        // rather than assuming it: the plant count is read at the same phase
        // of successive nominal days and the spread across them reported.
        "steady" => {
            let day = 3_600u64;
            let at = |f: u64| history.iter().min_by_key(|s| s.frame.abs_diff(f)).map(|s| s.plants);
            let phases: Vec<(u64, Option<u32>)> =
                (1..=(frames_of(history) / day)).map(|k| (k * day, at(k * day))).collect();
            let quarters: Vec<(u64, Option<u32>)> =
                (1..=(frames_of(history) / day)).map(|k| (k * day + day / 2, at(k * day + day / 2))).collect();
            println!("    on the nominal day boundary: {phases:?}");
            println!("    half a day later:            {quarters:?}");
            println!("    (a systematic gap between the two rows would be a cycle to divide out)");
            all &= ok("history spans a day", frames_of(history) >= day, format!("{} frames", frames_of(history)));
        }
        _ => {
            println!("  (no control asserted -- this is the standard bed)");
        }
    }
    println!("VERDICT: {}", if all { "controls held" } else { "A CONTROL FAILED" });
}

fn frames_of(history: &[stats::Sample]) -> u64 {
    match (history.first(), history.last()) {
        (Some(a), Some(b)) => b.frame.saturating_sub(a.frame),
        _ => 0,
    }
}

/// Kill half the living organisms, oldest slot first.
///
/// `mark_organism_senescent` is the shipped experimental-disturbance seam,
/// and it produces the *graded* death `rot_remains` carries out rather than
/// erasing cells -- so the strip falls the way a real die-back falls, which
/// is the thing the control is checking the page can show.
fn cull_half(world: &mut World) -> usize {
    let ids = world.live_organism_ids();
    let mut killed = 0;
    for id in ids.iter().take(ids.len() / 2) {
        if world.mark_organism_senescent(*id) {
            killed += 1;
        }
    }
    killed
}

/// **What one census costs, and what it costs amortised over a frame.**
///
/// `CLAUDE.md`: the page must not cost the frame. Measured paired and
/// alternating on one binary in one process -- the census against a
/// same-shaped walk that does nothing -- because a single timing on a busy
/// box is the rest of the machine (two runs of a byte-identical `ascii`
/// once disagreed 2.42x).
fn cost(spec: LabBox, frames: u64) {
    let mut lab = Lab::new(spec);
    for _ in 0..frames {
        tick(&mut lab);
    }
    let organisms = lab.world.live_organism_count();
    let mut census_ns = 0u128;
    let mut idle_ns = 0u128;
    const REPS: u32 = 200;
    for _ in 0..REPS {
        // Alternating, so a drift in machine state falls on both arms.
        let t = std::time::Instant::now();
        let mut fresh = Stats::new();
        fresh.observe(&lab.world);
        census_ns += t.elapsed().as_nanos();
        let t = std::time::Instant::now();
        let ids = lab.world.live_organism_ids();
        std::hint::black_box(ids.len());
        idle_ns += t.elapsed().as_nanos();
    }
    // The paint, separately. It is a different bargain from the census --
    // every panel in this repo forces a full redraw while it is open, and
    // that cost belongs to the renderer rather than to this page -- but the
    // page's own painting is its to answer for.
    let mut draw_ns = 0u128;
    let mut buf = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    lab.stats.observe(&lab.world);
    for _ in 0..REPS {
        let t = std::time::Instant::now();
        lab.stats.draw(&mut buf, &lab.world);
        draw_ns += t.elapsed().as_nanos();
    }
    let census = census_ns as f64 / REPS as f64 / 1e6;
    let idle = idle_ns as f64 / REPS as f64 / 1e6;
    let draw = draw_ns as f64 / REPS as f64 / 1e6;
    let cells: usize = lab
        .world
        .live_organism_ids()
        .iter()
        .filter_map(|id| lab.world.organism(*id))
        .map(|s| s.cells.len())
        .sum();
    println!("  settled at frame {frames}: {organisms} live organisms, {cells} living cells");
    println!("  one census                        {census:.4} ms");
    println!("  the id walk alone (the floor)     {idle:.4} ms");
    println!("  one paint of the page             {draw:.4} ms");
    println!(
        "  amortised at 60 fps: census {:.5} ms/frame (one every {} frames) + paint {draw:.4} ms/frame while open",
        census / stats::SAMPLE_INTERVAL as f64,
        stats::SAMPLE_INTERVAL,
    );
}
