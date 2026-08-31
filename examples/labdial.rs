//! **The speed dial, measured and shown.** The instrument behind
//! `lab::time::MOTION_TICKS_PER_FRAME` and the artifact the owner judges the
//! crossover on.
//!
//! Three questions, three modes, and they are deliberately not the same kind
//! of number:
//!
//! | | | |
//! |---|---|---|
//! | `mode=census` | **where motion stops being motion** | deterministic counters — cells, ticks, displacements. Identical under any machine load |
//! | `mode=rate` | **what this box can actually achieve** | wall clock, and therefore only as trustworthy as the box was quiet |
//! | `mode=gif` | **the same box at five multipliers, animated** | the artifact; a grid of stills cannot answer whether something *moves* right |
//!
//! **Why the crossover is a counter and not a timing.** `CLAUDE.md`: two runs
//! of a byte-identical binary on bit-identical work disagreed 2.42x, and this
//! ran on a box with three other agents compiling on it. So the crossover is
//! derived from **ticks per displayed frame** (`60*M/D`, arithmetic) times
//! **cells moved per tick** (censused in the box, deterministic) — neither of
//! which moves when the machine gets loud. Only `mode=rate` reads a clock,
//! and its numbers carry that caveat wherever they are quoted.
//!
//! **Why the arithmetic alone is not enough.** At multiplier `M` and display
//! rate `D` the world advances `60*M/D` ticks between displayed frames, and a
//! falling cell moves about one cell per tick — so the fastest thing on
//! screen jumps roughly that many cells. That is an **upper bound** and in a
//! sealed lab box it is a loose one: nothing is in free fall for long, and
//! ants and plants both move far slower than sand. `mode=census` measures
//! what actually moves.
//!
//! ```text
//! cargo run --release --example labdial -- mode=census frames=8000
//! cargo run --release --example labdial -- mode=rate
//! cargo run --release --example labdial -- mode=gif out=dial.gif
//! ```

use std::collections::HashMap;
use std::time::{Duration, Instant};

use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::time::{Phase, TICKS_PER_SECOND};
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};
use pixel_physics::render::Renderer;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::frame;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::player;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// The box every mode measures in. **One spec, so the census, the rate and
/// the picture are all about the same bed** — `scene.rs`'s own note: a bed
/// that is not the game's bed produces results that do not transfer.
fn spec() -> LabBox {
    LabBox {
        width: arg("width").unwrap_or(512),
        height: arg("height").unwrap_or(320),
        soil_depth: arg("soil").unwrap_or(80),
        founders: arg("founders").unwrap_or(8),
        colonies: arg("colonies").unwrap_or(1),
        compartments: arg("walls").unwrap_or(1),
        seed: arg("seed").unwrap_or(1),
        ..LabBox::default()
    }
}

fn main() {
    // The harness names its own parameters on the first line, so a log that
    // does not name a setting was written by a binary that never had one --
    // `CLAUDE.md`'s megastudy gotcha, where 24 logs turned out to be three
    // populations because `worldseed=` post-dated the binary.
    let mode: String = arg("mode").unwrap_or_else(|| "census".to_string());
    let s = spec();
    println!(
        "labdial: mode={mode} {}x{} soil={} founders={} colonies={} walls={} seed={}",
        s.width, s.height, s.soil_depth, s.founders, s.colonies, s.compartments, s.seed
    );
    match mode.as_str() {
        "census" => census(),
        "rate" => rate(),
        "gif" => gif(),
        other => panic!("unknown mode={other}; try census, rate or gif"),
    }
}

// ---------------------------------------------------------------------------
// mode=census — where motion stops being motion
// ---------------------------------------------------------------------------

/// Every cell's material, as a flat grid, for the changed-cell count.
fn material_grid(world: &World, w: i32, h: i32) -> Vec<u16> {
    let mut g = vec![0u16; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            g[(y * w + x) as usize] = world.get(x, y).material.0;
        }
    }
    g
}

/// Ticks-per-displayed-frame values to report. Powers of two either side of
/// anything plausible, plus 3/12/48/192/768 — what 1x/4x/16x/64x/256x produce
/// at a 20 Hz display, so the census and the picture line up exactly.
const STOPS: [usize; 18] =
    [1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 128, 192, 256, 384, 768];

/// **The crossover census.** One run of the lab box, sampled every tick, with
/// every reported number a count and not a clock.
///
/// Two instruments, because neither alone answers it:
///
/// - **`step`** — the net displacement of a tracked creature between two
///   displayed frames, in cells. This is the one that models apparent motion
///   directly: the eye follows a feature while it can still match the feature
///   to itself, and it stops being able to somewhere around the feature's own
///   size.
/// - **`changed`** — how many cells hold a different material between those
///   two frames. Says how much of the *picture* is new rather than how far one
///   thing went, and it catches everything the first instrument cannot see:
///   plants appearing, seeds dropping, soil settling.
///
/// **The positive control is printed with the result.** At one tick between
/// frames nothing can have moved more than about a cell and a half, and if the
/// `step` column reads 0.00 there the instrument never fired — a null and a
/// dead probe look identical, which is how this repo has published one before.
fn census() {
    let s = spec();
    let frames: u64 = arg("frames").unwrap_or(6_000);
    let warm: u64 = arg("warm").unwrap_or(3_000);
    let longest = *STOPS.iter().max().expect("STOPS is not empty");

    let mut world = s.build();
    let mut particles = ParticleSystem::new();
    let mut blasts = Blasts::new();
    let tuning = player::Tuning::default();

    println!("  warming {warm} ticks so there is something alive to watch");
    for _ in 0..warm {
        frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
    }

    // The displacement instrument needs a ring of centroid maps: tens of
    // creatures a tick, so a 769-deep ring is kilobytes.
    let mut ring: std::collections::VecDeque<HashMap<u16, (f32, f32)>> =
        std::collections::VecDeque::with_capacity(longest + 1);

    // The changed-cell instrument is **anchored** rather than ringed: one grid
    // is stored, and every stop is compared against it as the run passes that
    // offset. A ring of 769 material grids would be 250 MB and 2.8 million
    // comparisons a tick, which would have made this census cost more than the
    // simulation it is measuring.
    let mut anchor: Vec<u16> = Vec::new();
    let mut anchor_at = 0u64;
    // Strictly greater than the longest stop. Re-anchoring *at* `longest`
    // meant the `f - anchor_at == longest` case was always the re-anchor
    // instead, so the 768 row reported zero samples -- which reads exactly
    // like "nothing changed over 768 ticks" and is really "never measured".
    let anchor_period = longest as u64 + 256;

    let n = STOPS.len();
    let (mut max_step, mut sum_step, mut count_step) = (vec![0f32; n], vec![0f64; n], vec![0u64; n]);
    let mut all_steps: Vec<Vec<f32>> = vec![Vec::new(); n];
    let (mut sum_changed, mut count_changed) = (vec![0f64; n], vec![0u64; n]);

    let (mut body_cells, mut body_n) = (0f64, 0u64);
    let mut peak_creatures = 0usize;

    for f in 0..frames {
        // One walk of the organism table serves both the centroid map and the
        // body-size census, so the per-tick cost stays flat.
        let mut here = HashMap::new();
        for id in world.live_organism_ids() {
            let Some(state) = world.organism(id) else { continue };
            if world.species.get(state.species).creature.is_none() || state.cells.is_empty() {
                continue;
            }
            body_cells += state.cells.len() as f64;
            body_n += 1;
            let count = state.cells.len() as f32;
            let (sx, sy) = state
                .cells
                .keys()
                .fold((0f32, 0f32), |(ax, ay), (x, y)| (ax + *x as f32, ay + *y as f32));
            here.insert(id, (sx / count, sy / count));
        }
        peak_creatures = peak_creatures.max(here.len());
        ring.push_back(here);
        if ring.len() > longest + 1 {
            ring.pop_front();
        }

        let last = ring.len() - 1;
        for (si, gap) in STOPS.iter().enumerate() {
            if last < *gap {
                continue;
            }
            let then = last - gap;
            // Only ids alive at both ends. A creature born or that died inside
            // the window has no displacement, and counting one as a jump would
            // be counting an appearance as motion.
            for (id, (x1, y1)) in &ring[last] {
                let Some((x0, y0)) = ring[then].get(id) else { continue };
                let d = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
                if d > max_step[si] {
                    max_step[si] = d;
                }
                sum_step[si] += d as f64;
                count_step[si] += 1;
                all_steps[si].push(d);
            }
        }

        // The anchored half.
        if f.is_multiple_of(anchor_period) {
            anchor = material_grid(&world, s.width, s.height);
            anchor_at = f;
        } else if !anchor.is_empty() {
            let offset = (f - anchor_at) as usize;
            if let Some(si) = STOPS.iter().position(|g| *g == offset) {
                let now = material_grid(&world, s.width, s.height);
                let changed = now.iter().zip(anchor.iter()).filter(|(a, b)| a != b).count();
                sum_changed[si] += changed as f64;
                count_changed[si] += 1;
            }
        }

        if f + 1 < frames {
            frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        }
    }

    let body = if body_n > 0 { body_cells / body_n as f64 } else { 0.0 };
    println!(
        "  creatures: peak {peak_creatures} alive, mean body {body:.2} cells over {body_n} \
         organism-frames"
    );
    println!(
        "  plants: {} organisms, {} creature organisms, world frame {}",
        world.live_organism_count() - world.live_creature_count(),
        world.live_creature_count(),
        world.frame
    );
    if body_n == 0 {
        println!(
            "\n  !! NO CREATURES EVER LIVED IN THIS BOX. The step column below is a dead probe,\n  \
             not a null result -- do not read a crossover off it."
        );
    }

    println!(
        "\n  ticks/f  is what the world advances between two displayed frames.\n  \
         step     is net creature displacement across that gap, in cells.\n  \
         changed  is cells whose material differs between the two frames.\n"
    );
    println!(
        "  {:>8} {:>9} {:>9} {:>9} {:>9} {:>10} {:>8}",
        "ticks/f", "max step", "p90 step", "mean step", "steps", "changed", "grids"
    );
    for (si, gap) in STOPS.iter().enumerate() {
        let mean = if count_step[si] > 0 { sum_step[si] / count_step[si] as f64 } else { 0.0 };
        let mut v = std::mem::take(&mut all_steps[si]);
        v.sort_by(|a, b| a.total_cmp(b));
        let q = if v.is_empty() {
            0.0
        } else {
            v[((v.len() as f64 * 0.9) as usize).min(v.len() - 1)]
        };
        let changed =
            if count_changed[si] > 0 { sum_changed[si] / count_changed[si] as f64 } else { 0.0 };
        println!(
            "  {gap:>8} {:>9.2} {q:>9.2} {mean:>9.2} {:>9} {changed:>10.0} {:>8}",
            max_step[si], count_step[si], count_changed[si]
        );
    }

    // The positive control, read out rather than left to the reader.
    let one = max_step[0];
    println!(
        "\n  control: at 1 tick between frames the largest displacement any creature managed was\n  \
         {one:.2} cells. A creature moves at most one cell a tick, so a reading of 0.00 would\n  \
         mean the probe never fired and a reading above ~1.5 would mean it is not measuring a\n  \
         displacement."
    );
    println!(
        "\n  criterion: apparent motion holds while the typical mover's net displacement between\n  \
         displayed frames stays inside its own body ({body:.1} cells). Read `p90 step` against\n  \
         that -- `max step` is one ant in the whole run and sets a bar nothing else has to clear.\n  \
         At a 20 HZ display, ticks/frame = 3*M, so a crossover at N ticks/frame is M = N/3."
    );
}

// ---------------------------------------------------------------------------
// mode=rate — what this box achieves, on a clock
// ---------------------------------------------------------------------------

/// **The reference frame loop.** This is exactly what `src/bin/lab.rs` must
/// do, minus `pixels.render()`: `advance`, draw only when the returned
/// `Advance` says to, sleep for the returned idle. Kept here so the change
/// asked of `bin/lab.rs` has a working implementation to copy rather than a
/// description.
fn drive(lab: &mut Lab, buf: &mut [u8], seconds: f32) -> (u64, u64, Duration) {
    let started = Instant::now();
    let mut last = started;
    let mut ticks = 0u64;
    let mut draws = 0u64;
    while started.elapsed().as_secs_f32() < seconds {
        let now = Instant::now();
        let elapsed = now.duration_since(last);
        last = now;
        let advance = lab.advance(elapsed);
        ticks += advance.ticks as u64;
        if advance.draw {
            // `Lab::draw` takes the measured frame rate now (the bar prints
            // it); this harness has no display loop, so it has no rate to
            // report and says so rather than inventing one.
            lab.draw(buf, 0.0);
            draws += 1;
        }
        if !advance.idle.is_zero() {
            std::thread::sleep(advance.idle);
        }
    }
    (ticks, draws, started.elapsed())
}

fn rate() {
    let seconds: f32 = arg("seconds").unwrap_or(3.0);
    let warm: u64 = arg("warm").unwrap_or(6_000);
    let mut buf = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    let tuning = player::Tuning::default();

    println!(
        "\n  WALL CLOCK. These are the only numbers here that move when the box gets loud,\n  \
         and this one is shared with other agents. Quote them with that caveat.\n"
    );
    println!(
        "  {:>8} {:>7} {:>10} {:>10} {:>9} {:>9} {:>9} {:>5}",
        "asked", "displ", "achieved", "ticks/f", "draws/s", "ticks", "readout", "orgs"
    );
    for hz in [60u32, 30, 20, 10] {
        for preset in [4usize, 5, 6] {
            // A fresh box per configuration, warmed with *plain ticks* rather
            // than through the dial. Warming through `advance` would run the
            // catch-up loop, which is the thing under test -- and at 1x with a
            // one-second `elapsed` it would ask for 60 ticks a call and quietly
            // run 360,000 of them.
            let mut lab = Lab::new(spec());
            for _ in 0..warm {
                frame::step(
                    &mut lab.world,
                    &mut lab.particles,
                    &mut lab.blasts,
                    player::PlayerInput::default(),
                    &tuning,
                );
            }
            let orgs = lab.world.live_organism_count();
            lab.time.set_preset(preset);
            lab.time.set_display_hz(hz);
            assert_eq!(lab.time.phase, Phase::Running, "preset {preset} should be Running");
            let asked = lab.time.requested;
            let (ticks, draws, real) = drive(&mut lab, &mut buf, seconds);
            let achieved = ticks as f64 / TICKS_PER_SECOND as f64 / real.as_secs_f64();
            let per_frame = if draws > 0 { ticks as f64 / draws as f64 } else { 0.0 };
            // The readout the player sees, beside the count the harness took
            // independently: if these two ever disagree the dial is lying.
            println!(
                "  {asked:>8} {hz:>6}HZ {achieved:>9.1}X {per_frame:>10.0} {:>9.1} {ticks:>9} \
                 {:>8.1}X {orgs:>5}",
                draws as f64 / real.as_secs_f64(),
                lab.time.achieved
            );
        }
        println!();
    }
}

// ---------------------------------------------------------------------------
// mode=gif — the artifact
// ---------------------------------------------------------------------------

/// The same box at every stop on the dial, side by side, animated at the
/// display rate they would really be shown at.
///
/// **A grid of stills cannot answer whether something moves right**, which is
/// the whole question the crossover asks, so this writes an animation. Every
/// tile is the same spec at the same seed with the same warm-up, so they are
/// the same box; they diverge only because they have run for different amounts
/// of simulated time, which is exactly what the dial does.
///
/// The tiles are the six multipliers `bin/lab.rs` binds to the number row, so
/// what the owner judges here is the dial they will actually press.
fn gif() {
    let s = spec();
    let out: String = arg("out").unwrap_or_else(|| "labdial.gif".to_string());
    // **A frame sequence as well as the gif, and it is the preferred one.**
    // `.claude/skills/review/SKILL.md`: tested head to head on one card with
    // the same motion posted both ways, the sequence played for the owner and
    // the gif showed as a single static frame -- the sequence uses the review
    // page's own timer instead of the browser's gif decoding, so it does not
    // depend on any of the things a gif can quietly fail at.
    let png: Option<String> = arg("png");
    let hz: u32 = arg("hz").unwrap_or(20);
    let count: usize = arg("count").unwrap_or(40);
    let warm: u64 = arg("warm").unwrap_or(12_000);
    let cols: usize = arg("cols").unwrap_or(3);
    let scale: u32 = arg("scale").unwrap_or(3);
    let mults: String = arg("mult").unwrap_or_else(|| "1,2,4,16,64,256".to_string());
    let mults: Vec<u32> = mults.split(',').map(|m| m.parse().expect("a multiplier")).collect();
    // Tiles are a crop of the real box, not a smaller box: a narrower bed is a
    // different bed and its stand would not transfer.
    let crop_x: i32 = arg("cropx").unwrap_or(192);
    let crop_w: i32 = arg("cropw").unwrap_or(144);
    let crop_y: i32 = arg("cropy").unwrap_or(112);
    let crop_h: i32 = arg("croph").unwrap_or(88);

    let tuning = player::Tuning::default();
    let (vw, vh) = (s.width as u32, s.height as u32);

    struct Tile {
        mult: u32,
        per_frame: u64,
        world: World,
        particles: ParticleSystem,
        blasts: Blasts,
        ticks: u64,
    }
    let mut tiles: Vec<Tile> = Vec::new();
    for m in &mults {
        let mut world = s.build();
        let mut particles = ParticleSystem::new();
        let mut blasts = Blasts::new();
        for _ in 0..warm {
            frame::step(
                &mut world,
                &mut particles,
                &mut blasts,
                player::PlayerInput::default(),
                &tuning,
            );
        }
        tiles.push(Tile {
            mult: *m,
            per_frame: (TICKS_PER_SECOND as u64 * *m as u64) / hz as u64,
            world,
            particles,
            blasts,
            ticks: 0,
        });
    }
    let rows = tiles.len().div_ceil(cols);
    println!(
        "  {} tiles in {cols}x{rows} at {hz} HZ, {count} frames, scale {scale}: ticks/frame {:?}",
        tiles.len(),
        tiles.iter().map(|t| t.per_frame).collect::<Vec<_>>()
    );

    let mut renderer = Renderer::new();
    // One pixel of gutter, so six crops of one bed read as six panels rather
    // than as one wide picture with seams in it.
    let tile_w = crop_w as u32 + 1;
    let tile_h = crop_h as u32 + 1;
    let sheet_w = tile_w * cols as u32;
    let sheet_h = tile_h * rows as u32;
    let mut sheets: Vec<Vec<u8>> = Vec::with_capacity(count);
    let mut full = vec![0u8; (vw * vh * 4) as usize];

    for f in 0..count {
        let mut sheet = vec![0u8; (sheet_w * sheet_h * 4) as usize];
        for (ti, tile) in tiles.iter_mut().enumerate() {
            let touched = tile.world.take_touched_chunks();
            renderer.draw(&tile.world, &tile.particles, &touched, &mut full, (vw, vh), true);
            let x0 = (ti % cols) as u32 * tile_w;
            let y0 = (ti / cols) as u32 * tile_h;
            for y in 0..crop_h as u32 {
                let src = (((crop_y as u32 + y) * vw + crop_x as u32) * 4) as usize;
                let dst = (((y0 + y) * sheet_w + x0) * 4) as usize;
                sheet[dst..dst + (crop_w * 4) as usize]
                    .copy_from_slice(&full[src..src + (crop_w * 4) as usize]);
            }
            // The multiplier and the tick gap, on the tile. Deliberately NOT
            // the crossover verdict: the card asks the owner where motion
            // stops, and labelling the answer would be a leading question.
            // Drawn inside the tile over the ceiling air, not on a strip under
            // it: a caption on a black band came out grey after the gif's
            // palette quantisation and was barely readable. Two lines, because
            // one at this crop width ran off the tile and into its neighbour.
            for (li, line) in [
                format!("{}X", tile.mult),
                format!("{} TICKS PER FRAME", tile.per_frame),
            ]
            .iter()
            .enumerate()
            {
                assert!(
                    pixel_physics::hud::text_width(line) + 8 <= crop_w,
                    "caption {line:?} is wider than the {crop_w}-cell tile"
                );
                pixel_physics::hud::draw_text(
                    &mut sheet,
                    sheet_w,
                    sheet_h,
                    x0 as i32 + 4,
                    y0 as i32 + 4 + 9 * li as i32,
                    line,
                    [255, 245, 200, 255],
                );
            }
            for _ in 0..tile.per_frame {
                frame::step(
                    &mut tile.world,
                    &mut tile.particles,
                    &mut tile.blasts,
                    player::PlayerInput::default(),
                    &tuning,
                );
                tile.ticks += 1;
            }
        }
        sheets.push(sheet);
        if f.is_multiple_of(8) {
            println!("  frame {f}/{count}");
        }
    }

    // **The counts that go beside the picture**, per `CLAUDE.md`: a collapse
    // once read as "chunks are working" from a sheet whose body count was zero
    // for the whole run.
    for tile in &tiles {
        let ids = tile.world.live_organism_ids();
        let cells: usize =
            ids.iter().filter_map(|id| tile.world.organism(*id)).map(|o| o.cells.len()).sum();
        println!(
            "  {:>4}x: {:>4} ticks/frame, {:>8} ticks run, {:>4} orgs, {:>3} ants, {cells:>6} cells",
            tile.mult,
            tile.per_frame,
            tile.ticks,
            ids.len(),
            tile.world.live_creature_count()
        );
    }

    let (out_w, out_h) = (sheet_w * scale, sheet_h * scale);
    let delay =
        image::Delay::from_saturating_duration(Duration::from_millis((1000 / hz.max(1)) as u64));
    let file = std::fs::File::create(&out).expect("creating the gif");
    let mut encoder = image::codecs::gif::GifEncoder::new(file);
    encoder.set_repeat(image::codecs::gif::Repeat::Infinite).expect("gif repeat");
    let mut written = 0usize;
    for (i, sheet) in sheets.into_iter().enumerate() {
        let sheet = if scale <= 1 { sheet } else { upscale(&sheet, sheet_w, sheet_h, scale) };
        if let Some(prefix) = &png {
            let path = format!("{prefix}{i:03}.png");
            image::save_buffer(&path, &sheet, out_w, out_h, image::ColorType::Rgba8)
                .expect("writing a frame");
            written += 1;
        }
        let img = image::RgbaImage::from_raw(out_w, out_h, sheet).expect("gif frame");
        encoder.encode_frame(image::Frame::from_parts(img, 0, 0, delay)).expect("gif frame");
    }
    drop(encoder);
    println!("animated gif ({out_w}x{out_h}, {count} frames at {hz} HZ): {out}");
    if let Some(prefix) = &png {
        println!("frame sequence ({out_w}x{out_h}, {written} frames): {prefix}NNN.png");
    }
}

/// Nearest-neighbour, because the world is a cell grid and anything smoother
/// would blur a two-cell ant into the soil behind it.
fn upscale(src: &[u8], w: u32, h: u32, scale: u32) -> Vec<u8> {
    let (dw, dh) = (w * scale, h * scale);
    let mut out = vec![0u8; (dw * dh * 4) as usize];
    for y in 0..dh {
        let sy = y / scale;
        for x in 0..dw {
            let sx = x / scale;
            let s = ((sy * w + sx) * 4) as usize;
            let d = ((y * dw + x) * 4) as usize;
            out[d..d + 4].copy_from_slice(&src[s..s + 4]);
        }
    }
    out
}
