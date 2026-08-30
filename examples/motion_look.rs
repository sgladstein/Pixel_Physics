//! **Does a creature that walks need to be as big as one that stands still?**
//! The motion half of `Reports/creature-appearance-design.md`.
//!
//! That report's whole body-size case rests on one number, `decoys` --
//! *"how many other places in this frame are at least as different from
//! their surroundings as the animal is"* -- and the word doing the work is
//! **frame**. `decoys` is computed on a single still, and a decoy is a rock
//! edge or a leaf: something that **holds still**, while the animal does
//! not. Nothing in that instrument distinguishes a stationary distractor
//! from a moving target, so it cannot see the one cue the owner has now
//! twice said is what actually finds an ant -- *"ants are mostly visible
//! with there motion"* (review card `20260830T031945607Z-7e0999`).
//!
//! This harness adds the missing axis and nothing else. It reuses
//! `creature_look`'s `luma`, `SURROUND`, pinned daylight and window
//! geometry **verbatim**, so its static column is the same number that
//! report published, and then asks of every window it already counts one
//! extra question: *did anything in it change between two frames?*
//!
//! ```text
//! cargo run --release --example motion_look -- mode=probe seed=1 gap=4
//! cargo run --release --example motion_look -- mode=live species=ant count=40 frames=600
//! ```
//!
//! **The definition, stated so it can be argued with.** A window is a
//! *still decoy* if `|inner mean - surround mean| >= contrast`, which is
//! `creature_look::decoys` unchanged. It is a **moving decoy** if it is a
//! still decoy **and** at least one pixel inside it changed by at least
//! `motion` luma between the two frames. Composition, not replacement: a
//! thing that moves but has no contrast is not a candidate for the eye
//! either, and a thing with contrast that holds still is exactly what a
//! moving animal is being separated *from*.
//!
//! **The definition is deliberately generous to decoys.** One changed pixel
//! anywhere in the window qualifies it, so drifting sand, a settling pool
//! and a swaying twig all count -- which is the honest version, because
//! they compete for the same motion channel. A stricter rule (coherent
//! displacement, a whole body moving together) would cut the moving count
//! further and flatter the answer this harness exists to test.
//!
//! **What it can and cannot see.** Render-side animation is *not* in these
//! numbers: `GrainMode::default()` is `Position`, a pure function of screen
//! position, and `render` builds a fresh `Renderer` per frame so the frame
//! counter that `Animated` reads is 0 in both halves of every pair. The
//! frozen-pair control in `mode=probe` proves it -- two renders of a world
//! that was not stepped differ in **zero** pixels. So every changed pixel
//! here is the simulation moving, which is what the question is about.

use std::collections::HashSet;

use pixel_physics::app::{HEIGHT, WIDTH};
use pixel_physics::render::Renderer;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::world::World;
use pixel_physics::sim::{creature, parallel, rng};

/// Noon, pinned, for `creature_look`'s reason and with its evidence: the
/// day/night cycle aliases straight into any luminance number, and that
/// harness's first run on a generated world landed at night and reported a
/// surround luma of 28 where the same world at midday gives 153. It costs
/// this harness twice over -- a pair of frames taken across dusk would read
/// the *whole screen* as moving.
const DAYLIGHT: f32 = 1.0;

/// How far out the surround ring reaches, in cells. `creature_look`'s
/// value, unchanged, because the static column here has to be the same
/// number that report published.
const SURROUND: i32 = 3;

/// Rec. 601 luma -- `creature_look`'s, unchanged.
fn luma(px: &[u8]) -> f32 {
    0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32
}

fn render(world: &World, frame: &mut [u8]) {
    let mut r = Renderer::new();
    r.pinned_light = Some(pixel_physics::sky::frame_for_daylight(DAYLIGHT));
    let particles = ParticleSystem::new();
    r.draw(world, &particles, &HashSet::new(), frame, (WIDTH, HEIGHT), true);
}

fn plane(frame: &[u8]) -> Vec<f32> {
    (0..(WIDTH * HEIGHT) as usize).map(|i| luma(&frame[i * 4..i * 4 + 4])).collect()
}

/// Where a body can stand in this column. `creature::colony_ant_site`, not
/// a local copy -- `creature_look` §9 records what the third copy of this
/// predicate cost when it was written as "topmost cell that is not air".
fn surface(world: &World, x: i32) -> Option<i32> {
    creature::colony_ant_site(world, x, 0)
}

/// One census of the whole frame at one body size.
struct Census {
    /// Every window evaluated. The positive control: at `contrast = 0` this
    /// is what `still` must equal, and a `still` that is not the full
    /// population at zero means the counter never fired.
    windows: usize,
    /// `creature_look::decoys` -- windows that beat the contrast threshold.
    still: usize,
    /// ...of which, windows that also contain a pixel that changed.
    moving: usize,
}

/// Slides one body-sized window over the frame and answers both questions
/// in a single pass.
///
/// **Both counts come out of one loop on purpose.** They are compared
/// against each other and against a published static figure, so any
/// difference in which windows are visited, how the ring is summed, or
/// where the bounds fall would show up as a motion effect. Written as two
/// functions that "do the same thing" it would be a comparability bug
/// waiting to happen; written as one, `moving <= still` is true by
/// construction.
///
/// `changed[i]` is whether pixel `i` moved between the pair, thresholded by
/// the caller. `blocked[i]` marks pixels belonging to a real body, so a
/// body can never be its own decoy -- `mode=probe` has no bodies in the
/// censused frames and passes an empty mask; `mode=live` has ants in them
/// and cannot lift them out, so it skips any window that touches one.
fn census(lum: &[f32], changed: &[bool], blocked: &[bool], w: i32, h: i32, contrast: f32) -> Census {
    let (fw, fh) = (WIDTH as i32, HEIGHT as i32);
    let at = |x: i32, y: i32| lum[(y * fw + x) as usize];
    let mut c = Census { windows: 0, still: 0, moving: 0 };
    for y in SURROUND..fh - h - SURROUND {
        'window: for x in SURROUND..fw - w - SURROUND {
            let mut inner = 0.0;
            for dy in 0..h {
                for dx in 0..w {
                    if blocked[((y + dy) * fw + x + dx) as usize] {
                        continue 'window;
                    }
                    inner += at(x + dx, y + dy);
                }
            }
            inner /= (w * h) as f32;
            let (mut outer, mut n) = (0.0, 0);
            for dy in -SURROUND..h + SURROUND {
                for dx in -SURROUND..w + SURROUND {
                    if dx >= 0 && dx < w && dy >= 0 && dy < h {
                        continue;
                    }
                    outer += at(x + dx, y + dy);
                    n += 1;
                }
            }
            c.windows += 1;
            if (inner - outer / n as f32).abs() >= contrast {
                c.still += 1;
                // Only asked of windows that already passed, which is what
                // makes the extra axis nearly free: a few hundred windows
                // out of a hundred and sixty thousand.
                let moved = (0..h).any(|dy| (0..w).any(|dx| changed[((y + dy) * fw + x + dx) as usize]));
                if moved {
                    c.moving += 1;
                }
            }
        }
    }
    c
}

/// The size ladder, `creature_look::shapes()`'s, unchanged -- the rows this
/// question is asked about are the rows that report published.
fn shapes() -> Vec<(&'static str, i32, i32)> {
    vec![
        ("1  (1x1)", 1, 1),
        ("2  (2x1) = shipped ant", 2, 1),
        ("4  (2x2) = shipped beetle", 2, 2),
        ("6  (3x2)", 3, 2),
        ("9  (3x3)", 3, 3),
        ("16 (4x4)", 4, 4),
    ]
}

fn changed_mask(a: &[f32], b: &[f32], motion: f32) -> Vec<bool> {
    a.iter().zip(b).map(|(x, y)| (x - y).abs() >= motion).collect()
}

fn median(mut v: Vec<usize>) -> usize {
    v.sort_unstable();
    v[v.len() / 2]
}

struct Args {
    mode: String,
    preset: String,
    species: String,
    seed: u64,
    warmup: u32,
    gap: u32,
    samples: u32,
    spacing: u32,
    contrast: f32,
    motion: f32,
    count: i32,
    frames: u32,
    out: String,
    crop: (i32, i32, i32, i32),
    zoom: i32,
    gifframes: u32,
    gifevery: u32,
    /// Which sky to hold, via `World::set_weather_pin`.
    ///
    /// **The arm that keeps the headline honest.** Every world this
    /// harness and `creature_look` measure on is *settled* -- warmed up,
    /// no player, and a seeded weather cycle that mostly leaves it alone --
    /// and on a settled world the motion channel is nearly empty by
    /// definition. A downpour is the commonest thing that fills it, it
    /// covers the whole screen rather than one corner, and the renderer
    /// draws its streaks off `world.frame`, so it moves between any two
    /// frames. If the answer survives `weather=storm` it is not an artifact
    /// of a quiet test bed.
    weather: String,
}

fn main() {
    let mut a = Args {
        mode: "probe".into(),
        preset: "rolling".into(),
        species: "ant".into(),
        seed: 1,
        warmup: 2400,
        gap: 4,
        samples: 5,
        spacing: 40,
        contrast: 80.0,
        motion: 12.0,
        count: 40,
        frames: 600,
        out: String::new(),
        crop: (0, 0, WIDTH as i32, HEIGHT as i32),
        zoom: 1,
        gifframes: 24,
        gifevery: 4,
        weather: "live".into(),
    };
    for arg in std::env::args().skip(1) {
        let (k, v) = arg.split_once('=').unwrap_or_else(|| panic!("expected key=value, got {arg:?}"));
        match k {
            "mode" => a.mode = v.into(),
            "preset" => a.preset = v.into(),
            "species" => a.species = v.into(),
            "seed" => a.seed = v.parse().expect("seed"),
            "warmup" => a.warmup = v.parse().expect("warmup"),
            "gap" => a.gap = v.parse().expect("gap"),
            "samples" => a.samples = v.parse().expect("samples"),
            "spacing" => a.spacing = v.parse().expect("spacing"),
            "contrast" => a.contrast = v.parse().expect("contrast"),
            "motion" => a.motion = v.parse().expect("motion"),
            "count" => a.count = v.parse().expect("count"),
            "frames" => a.frames = v.parse().expect("frames"),
            "out" => a.out = v.into(),
            "zoom" => a.zoom = v.parse().expect("zoom"),
            "gifframes" => a.gifframes = v.parse().expect("gifframes"),
            "gifevery" => a.gifevery = v.parse().expect("gifevery"),
            "weather" => a.weather = v.into(),
            "crop" => {
                let n: Vec<i32> = v.split(',').map(|p| p.parse().expect("crop=x,y,w,h")).collect();
                assert_eq!(n.len(), 4, "crop=x,y,w,h");
                a.crop = (n[0], n[1], n[2], n[3]);
            }
            _ => panic!("unknown argument {k:?}"),
        }
    }
    // Echo the parameters. `CLAUDE.md`'s stale-harness rule: a 3.5-hour
    // study once produced byte-identical logs because the binary predated
    // the argument it was passed, and a log that does not name its own
    // settings cannot show that.
    println!(
        "motion_look: mode={} preset={} seed={} warmup={} gap={} samples={} spacing={} contrast={} motion={} species={} count={} frames={}",
        a.mode, a.preset, a.seed, a.warmup, a.gap, a.samples, a.spacing, a.contrast, a.motion, a.species, a.count, a.frames
    );
    println!("motion_look: weather={} zoom={} crop={:?} out={:?}", a.weather, a.zoom, a.crop, a.out);

    let mut world = World::new(Rect::new(0, 0, WIDTH as i32 - 1, HEIGHT as i32 - 1));
    world.seed = a.seed;
    let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
    if let Some(e) = err {
        panic!("worldgen presets unavailable: {e}");
    }
    let params = presets.get(&a.preset).unwrap_or_else(|| panic!("no worldgen preset {:?}", a.preset));
    pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed: a.seed });
    for _ in 0..a.warmup {
        step(&mut world);
    }
    // Pinned *after* the warmup, so the world the arms share is the same
    // settled world and only the sky differs.
    if a.weather != "live" {
        use pixel_physics::sim::weather::Pin;
        let pin = match a.weather.as_str() {
            "clear" => Pin::Clear,
            "breeze" => Pin::Breeze,
            "gale" => Pin::Gale,
            "frost" => Pin::Frost,
            "rain" => Pin::Rain,
            "storm" => Pin::Storm,
            "snow" => Pin::Snow,
            "blizzard" => Pin::Blizzard,
            other => panic!("unknown weather {other:?}"),
        };
        world.set_weather_pin(pin);
        // Let it arrive. A pin that has not yet produced a drop measures
        // the frame before the weather, which is the same world again.
        for _ in 0..240 {
            step(&mut world);
        }
    }

    match a.mode.as_str() {
        "probe" => probe(world, &a),
        "live" => live(world, &a),
        other => panic!("unknown mode {other:?}"),
    }
}

fn step(world: &mut World) {
    parallel::step(world);
    world.step_active_sites();
    world.step_fields();
}

/// The size ladder, with the motion axis added, plus the four controls the
/// answer is worthless without.
fn probe(mut world: World, a: &Args) {
    let mut fa = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    let mut fb = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    let none = vec![false; (WIDTH * HEIGHT) as usize];

    // ---- Control 1: a frozen pair. Two renders of a world that was not
    // stepped. If anything here is non-zero, the numbers below are the
    // renderer animating rather than the world moving, and the whole
    // measurement is about `GrainMode`.
    render(&world, &mut fa);
    render(&world, &mut fb);
    let (pa, pb) = (plane(&fa), plane(&fb));
    let frozen = changed_mask(&pa, &pb, 0.001).iter().filter(|c| **c).count();
    let frozen_census = census(&pb, &changed_mask(&pa, &pb, 0.001), &none, 2, 1, a.contrast);
    println!("\ncontrol 1 (frozen pair, world not stepped): changed pixels = {frozen}   moving decoys at 2 cells = {}", frozen_census.moving);
    assert_eq!(frozen, 0, "the renderer is animating; every motion number below would be render-side");

    // ---- The census itself. Several samples, because outcomes here are
    // chaotic and one pair is a sample from a wide distribution.
    println!("\nsize ladder: still decoys (= creature_look's `decoys`) against moving decoys, |contr|>={:.0}, motion>={:.0}", a.contrast, a.motion);
    println!("one row per sample, {} frames apart, {} frames between samples\n", a.gap, a.spacing);
    let ladder = shapes();
    let mut still_by_shape: Vec<Vec<usize>> = vec![Vec::new(); ladder.len()];
    let mut moving_by_shape: Vec<Vec<usize>> = vec![Vec::new(); ladder.len()];
    let mut ambient: Vec<usize> = Vec::new();
    for s in 0..a.samples {
        render(&world, &mut fa);
        let pa = plane(&fa);
        for _ in 0..a.gap {
            step(&mut world);
        }
        render(&world, &mut fb);
        let pb = plane(&fb);
        let changed = changed_mask(&pa, &pb, a.motion);
        let moved_px = changed.iter().filter(|c| **c).count();
        ambient.push(moved_px);
        print!("  sample {s}: {moved_px:>6} px moved  ");
        for (i, &(_, w, h)) in ladder.iter().enumerate() {
            let c = census(&pb, &changed, &none, w, h, a.contrast);
            still_by_shape[i].push(c.still);
            moving_by_shape[i].push(c.moving);
            print!("{:>5}/{:<5}", c.still, c.moving);
        }
        println!();
        for _ in 0..a.spacing {
            step(&mut world);
        }
    }

    // ---- Control 2 and 3, on the last pair: the counter fires, and the
    // motion axis reduces to the static one when it is switched off.
    render(&world, &mut fa);
    let pa = plane(&fa);
    for _ in 0..a.gap {
        step(&mut world);
    }
    render(&world, &mut fb);
    let pb = plane(&fb);
    let all_moving = changed_mask(&pa, &pb, 0.0);
    let c0 = census(&pb, &all_moving, &none, 2, 1, 0.0);
    println!("\ncontrol 2 (contrast=0, 2-cell window): windows={} still={} -- must be equal", c0.windows, c0.still);
    assert_eq!(c0.windows, c0.still, "the still counter did not fire on every window at threshold 0");
    let c1 = census(&pb, &all_moving, &none, 2, 1, a.contrast);
    println!("control 3 (motion=0): still={} moving={} -- must be equal, the motion axis off is the static instrument", c1.still, c1.moving);
    assert_eq!(c1.still, c1.moving, "motion axis does not reduce to the static instrument at threshold 0");

    println!("\nmedian over {} samples:\n", a.samples);
    println!("{:<28}{:>10}{:>10}{:>10}", "body size", "still", "moving", "moving/still");
    let mut med_still = Vec::new();
    let mut med_moving = Vec::new();
    for (i, &(name, _, _)) in ladder.iter().enumerate() {
        let (s, m) = (median(still_by_shape[i].clone()), median(moving_by_shape[i].clone()));
        med_still.push(s);
        med_moving.push(m);
        println!("{name:<28}{s:>10}{m:>10}{:>10}", format!("{:.2}", m as f32 / s.max(1) as f32));
    }
    println!("\nambient moving pixels per {}-frame gap: median {}  (no creatures in this world -- all of it is", a.gap, median(ambient.clone()));
    println!("falling powder, settling water, growing plants and weather: the noise floor a moving animal competes with)");

    // ---- The answer, stated as an equivalence.
    let shipped = med_moving[1];
    let equiv = ladder.iter().enumerate().find(|(i, _)| med_still[*i] <= shipped);
    match equiv {
        Some((i, (name, _, _))) => {
            println!("\na MOVING 2-cell ant has {shipped} competitors; the smallest STILL body with no more than that is {name}");
            println!("(still ladder: {med_still:?})   so motion is worth about that much size, at this contrast");
            let _ = i;
        }
        None => println!("\nno still body size in the ladder gets down to a moving 2-cell ant's {shipped} competitors"),
    }

    // ---- Control 4, the target side, and the one the whole comparison
    // rests on: does a body that moves get *counted* by this instrument,
    // and does the same body held still get counted as not moving? Without
    // it, `moving` could be a census of a field the animal is not in.
    println!("\ncontrol 4 (target side): one probe per size, painted on the world's own surface,");
    println!("rendered at A, then at B one cell along (moving arm) and at B where it was (still arm).\n");
    println!("{:<28}{:>10}{:>10}{:>12}{:>12}", "probe", "|contr|", "moved px", "moving arm", "still arm");
    let ant_id = world.materials.id_of("ant").expect("no material named ant");
    let shades = world.materials.get(ant_id).palette.len().max(1) as u32;
    let dry: Vec<i32> = (0..WIDTH as i32).filter(|&x| surface(&world, x).is_some()).collect();
    assert!(dry.len() >= ladder.len() * 4, "only {} dry columns for {} probes", dry.len(), ladder.len());
    for (si, &(name, w, h)) in ladder.iter().enumerate() {
        let offs: Vec<(i32, i32)> = (0..w).flat_map(|dx| (0..h).map(move |dy| (-dx, -dy))).collect();
        let x0 = dry[(si * dry.len()) / ladder.len() + dry.len() / (ladder.len() * 2)];
        let fits = |world: &World, x: i32| -> Option<(i32, i32)> {
            let by = surface(world, x)? - 1;
            offs.iter().all(|&(dx, dy)| world.is_empty(x + dx, by + dy)).then_some((x, by))
        };
        // One cell to the right is where a walking body would be next; the
        // arm is void if it has nowhere legal to step, which is itself the
        // mobility cost §5 of the appearance report measures.
        let Some((x, by)) = (0..30).flat_map(|d| [x0 + d, x0 - d]).find_map(|x| fits(&world, x)) else {
            println!("{name:<28}{:>10}", "no site");
            continue;
        };
        let paint = |world: &mut World, x: i32, by: i32| {
            for (i, &(dx, dy)) in offs.iter().enumerate() {
                let shade = rng::stream(world.seed, si as u64, i as u64, 5).below(shades) as u8;
                world.set(x + dx, by + dy, Cell::new(ant_id, shade));
            }
        };
        let erase = |world: &mut World, x: i32, by: i32| {
            for &(dx, dy) in &offs {
                world.set(x + dx, by + dy, Cell::EMPTY);
            }
        };
        paint(&mut world, x, by);
        render(&world, &mut fa);
        erase(&mut world, x, by);
        let pa = plane(&fa);
        for _ in 0..a.gap {
            step(&mut world);
        }
        let Some((nx, nby)) = fits(&world, x + 1) else {
            println!("{name:<28}{:>10}", "no step");
            continue;
        };
        // Moving arm.
        paint(&mut world, nx, nby);
        render(&world, &mut fb);
        erase(&mut world, nx, nby);
        let pm = plane(&fb);
        // Still arm: the same B world, the same paint, the position it
        // started in. The negative control -- if this scores as moving,
        // the instrument is reading the world around the probe rather than
        // the probe.
        let still_site = fits(&world, x);
        let ps = still_site.map(|(sx, sby)| {
            paint(&mut world, sx, sby);
            render(&world, &mut fb);
            erase(&mut world, sx, sby);
            plane(&fb)
        });

        let win = |lum: &[f32], bx: i32, byy: i32| -> f32 {
            let fw = WIDTH as i32;
            let at = |x: i32, y: i32| lum[(y * fw + x) as usize];
            let mut inner = 0.0;
            for dy in 0..h {
                for dx in 0..w {
                    inner += at(bx + dx, byy + dy);
                }
            }
            inner /= (w * h) as f32;
            let (mut outer, mut n) = (0.0, 0);
            for dy in -SURROUND..h + SURROUND {
                for dx in -SURROUND..w + SURROUND {
                    if dx >= 0 && dx < w && dy >= 0 && dy < h {
                        continue;
                    }
                    outer += at(bx + dx, byy + dy);
                    n += 1;
                }
            }
            (inner - outer / n as f32).abs()
        };
        // Top-left of the probe's bounding box: offsets run back and up
        // from the standing cell.
        let (bx, byy) = (nx - w + 1, nby - h + 1);
        let contrast = win(&pm, bx, byy);
        let moved_px = (0..h)
            .flat_map(|dy| (0..w).map(move |dx| (dx, dy)))
            .filter(|&(dx, dy)| (pm[((byy + dy) * WIDTH as i32 + bx + dx) as usize] - pa[((byy + dy) * WIDTH as i32 + bx + dx) as usize]).abs() >= a.motion)
            .count();
        let moving_arm = contrast >= a.contrast && moved_px > 0;
        let still_arm = match (&ps, still_site) {
            (Some(ps), Some((sx, sby))) => {
                let (sbx, sbyy) = (sx - w + 1, sby - h + 1);
                let px = (0..h)
                    .flat_map(|dy| (0..w).map(move |dx| (dx, dy)))
                    .filter(|&(dx, dy)| (ps[((sbyy + dy) * WIDTH as i32 + sbx + dx) as usize] - pa[((sbyy + dy) * WIDTH as i32 + sbx + dx) as usize]).abs() >= a.motion)
                    .count();
                (if px > 0 { "MOVING" } else { "still" }).to_string()
            }
            _ => "no site".into(),
        };
        println!(
            "{name:<28}{contrast:>10.1}{moved_px:>10}{:>12}{still_arm:>12}",
            if moving_arm { "MOVING" } else { "missed" }
        );
    }
}

/// The same question asked of real ants: the shipped body, the shipped
/// walk, and the world's own noise around it.
fn live(mut world: World, a: &Args) {
    let dry: Vec<i32> = (0..WIDTH as i32).filter(|&x| surface(&world, x).is_some()).collect();
    assert!(dry.len() >= a.count as usize * 2, "only {} dry columns for {} creatures", dry.len(), a.count);
    let mut sites = Vec::new();
    let mut refused = 0;
    for i in 0..a.count {
        let x = dry[(i as usize * dry.len()) / a.count as usize];
        let Some(sy) = surface(&world, x) else {
            refused += 1;
            continue;
        };
        match creature::plant_creature_seed(&mut world, x, sy - 1, &a.species) {
            Some(site) => sites.push(site),
            None => refused += 1,
        }
    }
    for s in sites {
        world.schedule_active_site(s);
    }
    println!("placed {} of {} ({refused} refused placement)", a.count - refused, a.count);
    for _ in 0..a.frames {
        step(&mut world);
    }

    let mat = world.materials.id_of(&a.species).unwrap_or_else(|| panic!("no material named {:?}", a.species));
    let body_cells = |world: &World| -> Vec<(i32, i32)> {
        (0..HEIGHT as i32)
            .flat_map(|y| (0..WIDTH as i32).map(move |x| (x, y)))
            .filter(|&(x, y)| world.get(x, y).material == mat)
            .collect()
    };

    let mut fa = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    let mut fb = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    println!("\nreal ants: what fraction of the moving pixels are the animals, and how many of them move at all\n");
    println!("{:>7}{:>10}{:>12}{:>12}{:>14}{:>16}{:>12}", "sample", "px moved", "on a body", "elsewhere", "body share", "ants that moved", "moving dec.");
    let mut shares = Vec::new();
    let mut movers = Vec::new();
    // **The target-side number a decoy census cannot give.** `moving` says
    // how many rivals a *moving* ant has; it says nothing about how often
    // an ant is moving. An animal that steps once a second is invisible to
    // the motion channel the rest of the time, and the owner's own question
    // -- can you tell a dead ant from a live one -- is exactly this
    // quantity. Accumulated over every sampled gap, so an ant that moved
    // in any of them counts.
    let mut ever_moved: HashSet<u16> = HashSet::new();
    let mut ever_seen: HashSet<u16> = HashSet::new();
    let mut still_d = Vec::new();
    let mut moving_d = Vec::new();
    for _ in 0..a.samples {
        let cells_a = body_cells(&world);
        let ids_a: HashSet<u16> = cells_a.iter().map(|&(x, y)| world.get(x, y).organism_id()).collect();
        render(&world, &mut fa);
        let pa = plane(&fa);
        for _ in 0..a.gap {
            step(&mut world);
        }
        let cells_b = body_cells(&world);
        render(&world, &mut fb);
        let pb = plane(&fb);
        let changed = changed_mask(&pa, &pb, a.motion);

        // A pixel counts as the animal's if a body occupied it at either
        // end of the pair -- the cell it left and the cell it arrived in
        // are both the animal's motion.
        let mut is_body = vec![false; (WIDTH * HEIGHT) as usize];
        for &(x, y) in cells_a.iter().chain(cells_b.iter()) {
            is_body[(y * WIDTH as i32 + x) as usize] = true;
        }
        let moved: usize = changed.iter().filter(|c| **c).count();
        let on_body = changed.iter().zip(&is_body).filter(|(c, b)| **c && **b).count();
        shares.push((100 * on_body).checked_div(moved).unwrap_or(0));

        // How many creatures the motion channel actually gets. A body that
        // did not move in this gap is invisible to it -- and that is the
        // number a decoy count cannot show, because it is about the target
        // rather than the field.
        let mut moved_ids: HashSet<u16> = HashSet::new();
        for &(x, y) in cells_a.iter().chain(cells_b.iter()) {
            if changed[(y * WIDTH as i32 + x) as usize] {
                moved_ids.insert(world.get(x, y).organism_id());
            }
        }
        let seen = moved_ids.iter().filter(|i| **i != 0).count();
        movers.push(100 * seen / ids_a.len().max(1));
        ever_moved.extend(moved_ids.iter().filter(|i| **i != 0));
        ever_seen.extend(ids_a.iter().filter(|i| **i != 0));

        // The decoy census, with every window that touches an ant skipped
        // so no ant is its own decoy.
        let c = census(&pb, &changed, &is_body, 2, 1, a.contrast);
        still_d.push(c.still);
        moving_d.push(c.moving);
        println!(
            "{:>7}{moved:>10}{on_body:>12}{:>12}{:>13}%{:>15}%{:>7}/{:<5}",
            still_d.len() - 1,
            moved - on_body,
            shares[shares.len() - 1],
            movers[movers.len() - 1],
            c.still,
            c.moving
        );
        for _ in 0..a.spacing {
            step(&mut world);
        }
    }
    println!(
        "\nmedian: body share of moving pixels {}%   ants the motion channel sees per {}-frame gap {}%   still decoys {}  moving decoys {}",
        median(shares),
        a.gap,
        median(movers),
        median(still_d),
        median(moving_d)
    );
    let horizon = a.samples * (a.gap + a.spacing);
    println!(
        "over the whole {horizon}-frame sampling horizon: {} of {} ants moved at least once ({}% never moved in any sampled gap)",
        ever_moved.len(),
        ever_seen.len(),
        100 - 100 * ever_moved.len() / ever_seen.len().max(1)
    );
    let st = &world.creature_stats;
    let attempts = st.moves + st.moves_blocked;
    println!("creature_stats: moves={} blocked={} ({:.0}% of {attempts}) tumbles={} falls={}", st.moves, st.moves_blocked, 100.0 * st.moves_blocked as f32 / attempts.max(1) as f32, st.tumbles, st.falls);

    // Where the bodies ended up, so a card's crop can be aimed at them
    // rather than guessed -- `creature_look`'s reasoning, and the same
    // densest window.
    let cells = body_cells(&world);
    if !cells.is_empty() {
        let best = (0..WIDTH as i32 - 120).max_by_key(|&x| cells.iter().filter(|&&(cx, _)| cx >= x && cx < x + 120).count()).unwrap_or(0);
        let ys: Vec<i32> = cells.iter().filter(|&&(cx, _)| cx >= best && cx < best + 120).map(|&(_, cy)| cy).collect();
        println!("densest 120-cell window: x={best}  bodies there={}  y range {}..{}", ys.len(), ys.iter().copied().min().unwrap_or(0), ys.iter().copied().max().unwrap_or(0));
    }

    if !a.out.is_empty() {
        write_gif(&mut world, a);
    }
}

/// The card. A still cannot answer a question about motion, so this writes
/// the shipped ant walking in the world the numbers above were taken in --
/// same pinned daylight, same preset, same seed.
///
/// **`out=x.png` writes a numbered frame sequence and `out=x.gif` writes an
/// animation, and the sequence is the one to post.** The review skill
/// measured the two head to head on one card with the same motion in both:
/// the sequence played and the GIF showed the owner a single static frame,
/// despite passing every check available on the posting side. The page's
/// own timer does not depend on the browser's GIF decoding. Either way a
/// `-still.png` of the first frame is written alongside, because the whole
/// claim is a comparison between one frame and several.
fn write_gif(world: &mut World, a: &Args) {
    let (cx, cy, cw, ch) = a.crop;
    let (w, h) = ((cw * a.zoom) as u32, (ch * a.zoom) as u32);
    let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    let as_gif = a.out.to_ascii_lowercase().ends_with(".gif");
    let mut encoder = as_gif.then(|| {
        let file = std::fs::File::create(&a.out).expect("create gif");
        let mut e = image::codecs::gif::GifEncoder::new(file);
        e.set_repeat(image::codecs::gif::Repeat::Infinite).expect("gif repeat");
        e
    });
    for f in 0..a.gifframes {
        render(world, &mut frame);
        let mut buf = vec![0u8; (w * h * 4) as usize];
        for y in 0..ch {
            for x in 0..cw {
                let (sx, sy) = (cx + x, cy + y);
                if sx < 0 || sy < 0 || sx >= WIDTH as i32 || sy >= HEIGHT as i32 {
                    continue;
                }
                let src = ((sy as u32 * WIDTH + sx as u32) * 4) as usize;
                for zy in 0..a.zoom {
                    for zx in 0..a.zoom {
                        let d = (((y * a.zoom + zy) as u32 * w + (x * a.zoom + zx) as u32) * 4) as usize;
                        buf[d..d + 4].copy_from_slice(&frame[src..src + 4]);
                    }
                }
            }
        }
        if f == 0 {
            let still = a.out.rsplit_once('.').map(|(stem, _)| format!("{stem}-still.png")).unwrap_or_else(|| format!("{}-still.png", a.out));
            image::save_buffer(&still, &buf, w, h, image::ColorType::Rgba8).expect("write still");
            println!("wrote {still} -- the same first frame as a single still, for the paired card");
        }
        match &mut encoder {
            Some(e) => {
                let img = image::RgbaImage::from_raw(w, h, buf).expect("gif frame");
                e.encode_frame(image::Frame::from_parts(img, 0, 0, image::Delay::from_numer_denom_ms(80, 1))).expect("gif frame");
            }
            None => {
                let path = match a.out.rfind('.') {
                    Some(i) => format!("{}-{f:02}{}", &a.out[..i], &a.out[i..]),
                    None => format!("{}-{f:02}", a.out),
                };
                image::save_buffer(&path, &buf, w, h, image::ColorType::Rgba8).expect("write frame");
            }
        }
        for _ in 0..a.gifevery {
            step(world);
        }
    }
    drop(encoder);
    println!("{} ({w}x{h}, {} frames, {} sim frames apart): {}", if as_gif { "animated gif" } else { "frame sequence" }, a.gifframes, a.gifevery, a.out);
}
