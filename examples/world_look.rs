//! **Does a generated world look like a different place when the preset
//! changes?** The appearance instrument for worldgen.
//!
//! `Reports/plant-appearance-design.md` found that a plant's silhouette is
//! set by **extent, composition and palette**, and that every architectural
//! lever the plant line built moved only *which cell gets a label* — so all
//! three fired, the counters printed beside the sheets, and the owner saw
//! nothing change. `creature_look` carried that finding to creatures.
//! Nothing had carried it to worldgen, where the owner's most repeated
//! verdict across 39 review cards is *"I see no difference between the
//! images"*.
//!
//! This is that instrument. It answers four questions, each with its own
//! control, and it renders **off the shipped `Renderer` at the player's
//! viewport size** so no arm can invent a colour or a scale the engine would
//! not have drawn:
//!
//! ```text
//! cargo run --release --example world_look -- mode=control
//! cargo run --release --example world_look -- mode=composition seeds=4
//! cargo run --release --example world_look -- mode=colour seeds=4
//! cargo run --release --example world_look -- mode=distance seeds=4
//! cargo run --release --example world_look -- mode=passes seeds=3 views=16
//! ```
//!
//! **The unit is a rendered pixel in a player viewport, never a cell.** A
//! world is 8192x2560 and a viewport is 512x320 — 1/128th of it — so a pass
//! that writes 40,000 cells and a pass that writes 400 can be the same
//! picture, and a cell count cannot tell them apart. `pass_ablation` reports
//! cells and answers the interference question; `mode=passes` here reports
//! the same ablation in pixels and answers the visibility one.
//!
//! **Daylight is pinned** (`creature_look`'s rule, and CLAUDE.md's): a
//! colour or luminance number sampled at an arbitrary hour is a statement
//! about the hour. Every render here is at the same noon.

use std::collections::{HashMap, HashSet};

use pixel_physics::app::{HEIGHT, WIDTH, WORLD_HEIGHT, WORLD_WIDTH};
use pixel_physics::render::Renderer;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material::{self, MaterialId};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::world::World;
use pixel_physics::worldgen;

/// Noon, pinned. See the module doc, and `creature_look`'s `DAYLIGHT`: the
/// day/night cycle is a designed oscillator and it aliases into every colour
/// number this file produces.
const DAYLIGHT: f32 = 1.0;

/// Bits dropped per channel when binning a colour. 4 bits kept per channel is
/// 16 levels, 4,096 bins.
///
/// **Chosen against the grain, not for tidiness.** `render.rs` jitters every
/// cell by `JITTER_STRENGTH = 0.12` of its palette colour, which on a
/// mid-grey (128) is about +-15 levels — so an unquantised histogram counts
/// one rock as dozens of colours and reports a palette that does not exist.
/// A 16-level bin is the same width as the jitter, so a family stays one bin
/// or two rather than dozens. `mode=control` prints the raw count beside the
/// binned one, because the gap between them *is* the grain.
const BIN_SHIFT: u32 = 4;
const BINS: usize = 1 << (3 * (8 - BIN_SHIFT));

fn luma(px: &[u8]) -> f32 {
    0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32
}

fn bin_of(px: &[u8]) -> usize {
    let r = (px[0] >> BIN_SHIFT) as usize;
    let g = (px[1] >> BIN_SHIFT) as usize;
    let b = (px[2] >> BIN_SHIFT) as usize;
    (r << (2 * (8 - BIN_SHIFT))) | (g << (8 - BIN_SHIFT)) | b
}

/// The renderer this file uses everywhere. One place, so no two arms can
/// differ in a render setting.
fn renderer() -> Renderer {
    let mut r = Renderer::new();
    r.pinned_light = Some(pixel_physics::sky::frame_for_daylight(DAYLIGHT));
    r
}

/// Topmost cell in this column that holds material. `None` for a column of
/// sky all the way down.
///
/// **`cell.material == EMPTY`, not `Cell::is_empty()`** — CLAUDE.md's gotcha:
/// the managed-aware predicate answers "is this position available", and the
/// question here is "is there anything here to draw".
fn surface_y(world: &World, x: i32, h: i32) -> Option<i32> {
    (0..h).find(|&y| world.get(x, y).material != material::EMPTY)
}

/// Camera top-left corners for `n` player viewports spread across the world,
/// each aimed so the ground at the view's centre column sits mid-screen.
///
/// **Computed from one world and then reused**, which is what makes an
/// ablation arm paired: a pass that lowers the terrain would otherwise move
/// the camera as well, and the diff would be measuring the pan.
fn camera_positions(world: &World, n: usize) -> Vec<(i32, i32)> {
    let (ww, wh) = (WORLD_WIDTH as i32, WORLD_HEIGHT as i32);
    let (vw, vh) = (WIDTH as i32, HEIGHT as i32);
    let span = (ww - vw).max(0);
    (0..n)
        .map(|i| {
            let cam_x = if n <= 1 { 0 } else { span * i as i32 / (n as i32 - 1) };
            let centre = (cam_x + vw / 2).min(ww - 1);
            let ground = surface_y(world, centre, wh).unwrap_or(wh / 2);
            (cam_x, ground - vh / 2)
        })
        .collect()
}

/// One rendered viewport, plus the material of the cell behind every pixel.
struct View {
    frame: Vec<u8>,
    /// `MaterialId` behind each pixel, in the same order as `frame`'s pixels.
    behind: Vec<MaterialId>,
}

fn render_view(world: &World, cam: (i32, i32)) -> View {
    let (vw, vh) = (WIDTH as usize, HEIGHT as usize);
    let mut r = renderer();
    r.set_camera(cam.0, cam.1, (WIDTH, HEIGHT), world.bounds());
    let particles = ParticleSystem::new();
    let mut frame = vec![0u8; vw * vh * 4];
    r.draw(world, &particles, &HashSet::new(), &mut frame, (WIDTH, HEIGHT), true);
    // The camera may have been clamped, so read it back rather than assuming
    // the requested position was taken.
    let (cx, cy) = (r.camera_x, r.camera_y);
    let mut behind = vec![material::EMPTY; vw * vh];
    for py in 0..vh {
        for px in 0..vw {
            behind[py * vw + px] = world.get(cx + px as i32, cy + py as i32).material;
        }
    }
    View { frame, behind }
}

/// Colour and composition tallies over a set of views.
#[derive(Default)]
struct Census {
    /// Binned colour counts over **ground** pixels (a pixel whose cell holds
    /// material). Sky is excluded: it is a third of every frame and it is the
    /// same gradient in every preset, so leaving it in drags every distance
    /// toward zero for a reason that has nothing to do with the ground.
    ground_bins: HashMap<usize, u64>,
    /// The same over every pixel, sky included.
    all_bins: HashMap<usize, u64>,
    /// Exact RGB values seen on ground pixels — the unquantised count.
    exact: HashSet<u32>,
    /// Cells by material over the viewport rect.
    cells: HashMap<u16, u64>,
    /// Cells by material over the top `SKIN` rows of material in each column.
    skin: HashMap<u16, u64>,
    ground_px: u64,
    all_px: u64,
    luma_sum: f64,
    luma_sq: f64,
    /// Per-column skyline step, binned. **The measurement the colour
    /// histogram cannot make.** A TV distance over colour is blind to
    /// arrangement: a world with the same palette laid out completely
    /// differently scores zero against it. This is the shape channel — how
    /// often the skyline steps by how much — so "differs in colour" and
    /// "differs in form" become separate answers instead of one.
    slope: Vec<u64>,
    slope_n: u64,
    /// Surface rows seen, for the relief spread.
    surf: Vec<i32>,
    /// Mean absolute deviation of a ground pixel's luma from its own 3x3
    /// neighbourhood — `pixel_stat`'s speckle number, computed inline so it
    /// does not need a PNG round trip. Texture, as opposed to palette.
    mad_sum: f64,
    mad_n: u64,
}

/// Skyline steps are binned to +-`SLOPE_CAP` cells per column, with an
/// overflow bin at each end for a cliff.
const SLOPE_CAP: i32 = 8;

/// How deep "the near-subsurface the player reads as ground" goes, in cells.
const SKIN: usize = 8;

impl Census {
    fn add_view(&mut self, world: &World, v: &View, cam: (i32, i32)) {
        let (vw, vh) = (WIDTH as usize, HEIGHT as usize);
        if self.slope.is_empty() {
            self.slope = vec![0; (2 * SLOPE_CAP + 3) as usize];
        }
        for i in 0..vw * vh {
            let px = &v.frame[i * 4..i * 4 + 4];
            let b = bin_of(px);
            *self.all_bins.entry(b).or_default() += 1;
            self.all_px += 1;
            if v.behind[i] != material::EMPTY {
                *self.ground_bins.entry(b).or_default() += 1;
                self.exact.insert(u32::from_be_bytes([0, px[0], px[1], px[2]]));
                self.ground_px += 1;
                let l = luma(px) as f64;
                self.luma_sum += l;
                self.luma_sq += l * l;
            }
            *self.cells.entry(v.behind[i].0).or_default() += 1;
        }
        // The skin: walking down each column of the viewport from the first
        // cell that holds material.
        let (cx, cy) = cam;
        for px in 0..vw as i32 {
            let x = cx + px;
            let mut taken = 0usize;
            for py in 0..vh as i32 {
                if taken >= SKIN {
                    break;
                }
                let m = world.get(x, cy + py).material;
                if m == material::EMPTY {
                    continue;
                }
                *self.skin.entry(m.0).or_default() += 1;
                taken += 1;
            }
        }
        // The skyline of this view, and the steps between its columns.
        let mut prev: Option<i32> = None;
        for px in 0..vw as i32 {
            let Some(sy) = surface_y(world, cx + px, WORLD_HEIGHT as i32) else {
                prev = None;
                continue;
            };
            self.surf.push(sy);
            if let Some(p) = prev {
                let d = (sy - p).clamp(-SLOPE_CAP - 1, SLOPE_CAP + 1);
                self.slope[(d + SLOPE_CAP + 1) as usize] += 1;
                self.slope_n += 1;
            }
            prev = Some(sy);
        }
        // Local speckle over ground pixels, `pixel_stat`'s statistic.
        for y in 1..vh - 1 {
            for x in 1..vw - 1 {
                let i = y * vw + x;
                if v.behind[i] == material::EMPTY {
                    continue;
                }
                let mut mean = 0.0f32;
                for dy in 0..3 {
                    for dx in 0..3 {
                        let j = (y + dy - 1) * vw + (x + dx - 1);
                        mean += luma(&v.frame[j * 4..j * 4 + 4]);
                    }
                }
                self.mad_sum += (luma(&v.frame[i * 4..i * 4 + 4]) - mean / 9.0).abs() as f64;
                self.mad_n += 1;
            }
        }
    }

    fn slope_hist(&self) -> Vec<f64> {
        let n = self.slope_n.max(1) as f64;
        self.slope.iter().map(|&x| x as f64 / n).collect()
    }

    /// Standard deviation of the skyline row over every column sampled —
    /// relief at viewport scale, in cells.
    fn relief(&self) -> f64 {
        if self.surf.is_empty() {
            return 0.0;
        }
        let n = self.surf.len() as f64;
        let m = self.surf.iter().map(|&y| y as f64).sum::<f64>() / n;
        (self.surf.iter().map(|&y| (y as f64 - m).powi(2)).sum::<f64>() / n).sqrt()
    }

    fn mad(&self) -> f64 {
        self.mad_sum / self.mad_n.max(1) as f64
    }

    fn ground_hist(&self) -> Vec<f64> {
        let mut h = vec![0.0; BINS];
        if self.ground_px == 0 {
            return h;
        }
        for (&b, &n) in &self.ground_bins {
            h[b] = n as f64 / self.ground_px as f64;
        }
        h
    }

    fn material_hist(&self, materials: &pixel_physics::sim::material::MaterialRegistry) -> Vec<f64> {
        let mut h = vec![0.0; materials.len()];
        let total: u64 = self.cells.iter().filter(|(&m, _)| m != material::EMPTY.0).map(|(_, &n)| n).sum();
        if total == 0 {
            return h;
        }
        for (&m, &n) in &self.cells {
            if m != material::EMPTY.0 {
                h[m as usize] = n as f64 / total as f64;
            }
        }
        h
    }
}

/// One world's three comparable descriptions: its ground colour histogram, its
/// material shares, and its skyline-step histogram.
type Descriptors = (Vec<f64>, Vec<f64>, Vec<f64>);

/// One seed's un-ablated arm in `mode=passes`: the cameras, the frames they
/// produced, and what every pass wrote.
type Baseline = (Vec<(i32, i32)>, Vec<View>, Vec<(&'static str, usize)>);

/// Total-variation distance between two normalised histograms: 0 identical,
/// 1 disjoint. Read as **the fraction of the picture that would have to be
/// repainted to turn one into the other.**
fn tv(a: &[f64], b: &[f64]) -> f64 {
    0.5 * a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum::<f64>()
}

/// How many bins cover `frac` of the ground, largest first.
fn bins_covering(h: &[f64], frac: f64) -> usize {
    let mut v: Vec<f64> = h.iter().copied().filter(|&x| x > 0.0).collect();
    v.sort_by(|a, b| b.total_cmp(a));
    let mut acc = 0.0;
    for (i, x) in v.iter().enumerate() {
        acc += x;
        if acc >= frac {
            return i + 1;
        }
    }
    v.len()
}

fn shannon(h: &[f64]) -> f64 {
    -h.iter().filter(|&&p| p > 0.0).map(|&p| p * p.log2()).sum::<f64>()
}

fn build(params: &worldgen::WorldgenParams, seed: u64, skip: &str) -> World {
    build_settled(params, seed, skip, 0)
}

/// `settle` frames of the real sweep before looking, matching `viewshot`'s
/// own loop phase for phase — including `step_fields`, without which a long
/// settle dries every lake and the picture is of a bug in the harness.
fn build_settled(params: &worldgen::WorldgenParams, seed: u64, skip: &str, settle: usize) -> World {
    let bounds = Rect::new(0, 0, WORLD_WIDTH as i32 - 1, WORLD_HEIGHT as i32 - 1);
    let mut world = World::new(bounds);
    if skip.is_empty() {
        worldgen::generate(&mut world, worldgen::Spec::Generated { params, seed });
    } else {
        worldgen::generate_ablated(&mut world, worldgen::Spec::Generated { params, seed }, skip);
    }
    for _ in 0..settle {
        pixel_physics::sim::parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
    }
    world
}

/// One viewport as a PNG, **from the same render the numbers come from**, so
/// the picture and the metric cannot be about two different frames.
fn shot(presets: &worldgen::WorldgenPresets, a: &Args) {
    assert!(!a.out.is_empty(), "mode=shot needs out=PATH (one preset) or out=DIR/ (several)");
    for preset in &a.presets {
        let Some(params) = presets.get(preset) else { panic!("no preset {preset}") };
        for seed in 1..=a.seeds {
            let world = build_settled(params, seed, "", a.settle);
            let cams = camera_positions(&world, a.views);
            let cam = cams[a.view.min(cams.len() - 1)];
            let v = render_view(&world, cam);
            let mut rgb = vec![0u8; (WIDTH * HEIGHT * 3) as usize];
            for i in 0..(WIDTH * HEIGHT) as usize {
                rgb[i * 3..i * 3 + 3].copy_from_slice(&v.frame[i * 4..i * 4 + 3]);
            }
            let path = if a.out.ends_with('/') {
                format!("{}{preset}-s{seed}.png", a.out)
            } else {
                a.out.clone()
            };
            if let Some(dir) = std::path::Path::new(&path).parent() {
                std::fs::create_dir_all(dir).ok();
            }
            image::save_buffer(&path, &rgb, WIDTH, HEIGHT, image::ColorType::Rgb8).expect("write png");
            println!("  {path}  {preset} seed {seed} view {} camera {:?} settle {}", a.view, cam, a.settle);
        }
    }
}

/// **The rock-vocabulary A/B, in pixels.** `mode=vocab`.
///
/// `Reports/rock-vocabulary-design-2026-08-29.md`. The question this answers
/// is the one `plant-appearance-design.md` says every appearance proposal
/// owes: *which pixels does it move, and how many.* A pass ablation
/// (`mode=passes`) cannot answer it, because the rock vocabulary is not a
/// pass — it changes what `stone_massif` writes rather than whether it runs.
///
/// **Both arms are built in one process, from one binary, on one machine**,
/// with `worldgen::passes::set_rock_vocab` as the only difference. That is
/// CLAUDE.md's rule for a change of this shape: hold the semantic rule fixed
/// with one switch rather than measuring around the confound. It is also why
/// this is a mode here and not a shell loop over two `mode=shot` runs — those
/// arms would differ by a process.
///
/// Cameras are computed from the **off** arm and reused, so a bed that
/// changes colour cannot be scored for moving the camera.
///
/// ```text
/// cargo run --release --example world_look -- mode=vocab seeds=3 views=16
/// ```
fn vocab(presets: &worldgen::WorldgenPresets, a: &Args) {
    println!("Rock vocabulary A/B. Both arms in one process; only `set_rock_vocab` differs.");
    println!("  'moved' is rendered pixels that differ between the arms, over {} pixels per world.", WIDTH * HEIGHT * a.views as u32);
    println!("  'ground moved' is the same count as a share of pixels that hold material in either arm.");
    println!();
    println!("  preset      seed   moved        % of view   % of ground   b50 off>on   bins off>on   luma off>on     speckle off>on");
    let mut all_moved = 0u64;
    let mut all_px = 0u64;
    for preset in &a.presets {
        let Some(params) = presets.get(preset) else { continue };
        for seed in 1..=a.seeds {
            // `stage=weather` moves the OFF arm to "vocabulary on, weathering
            // off", so the difference prices the weathering half on its own
            // rather than pooling it with the rocks.
            // `out=` names the stage rather than a file here: the three
            // parts of the proposal are priced separately, each against the
            // arm that has everything before it.
            //
            //   (default)  everything off   vs  rocks + weathering + damp
            //   rocks      everything off   vs  rocks only
            //   weather    rocks only       vs  rocks + weathering
            //   damp       rocks + weather  vs  rocks + weathering + damp
            let (v0, w0, d0) = match a.out.as_str() {
                "weather" => (true, false, false),
                "damp" => (true, true, false),
                _ => (false, false, false),
            };
            let (v1, w1, d1) = match a.out.as_str() {
                "rocks" => (true, false, false),
                "weather" => (true, true, false),
                _ => (true, true, true),
            };
            worldgen::passes::set_rock_vocab(v0);
            worldgen::passes::set_rock_weather(w0);
            worldgen::passes::set_rock_damp(d0);
            let off = build_settled(params, seed, "", a.settle);
            let cams = camera_positions(&off, a.views);
            worldgen::passes::set_rock_vocab(v1);
            worldgen::passes::set_rock_weather(w1);
            worldgen::passes::set_rock_damp(d1);
            let on = build_settled(params, seed, "", a.settle);

            let (mut moved, mut total, mut ground) = (0u64, 0u64, 0u64);
            let (mut coff, mut con) = (Census::default(), Census::default());
            for &cam in &cams {
                let a_v = render_view(&off, cam);
                let b_v = render_view(&on, cam);
                coff.add_view(&off, &a_v, cam);
                con.add_view(&on, &b_v, cam);
                for i in 0..(WIDTH * HEIGHT) as usize {
                    total += 1;
                    let solid = a_v.behind[i] != material::EMPTY || b_v.behind[i] != material::EMPTY;
                    if solid {
                        ground += 1;
                    }
                    if a_v.frame[i * 4..i * 4 + 3] != b_v.frame[i * 4..i * 4 + 3] {
                        moved += 1;
                    }
                }
            }
            all_moved += moved;
            all_px += total;
            let stat = |c: &Census| {
                let h = c.ground_hist();
                let n = c.ground_px.max(1) as f64;
                let mean = c.luma_sum / n;
                (bins_covering(&h, 0.5), c.ground_bins.len(), mean, c.mad())
            };
            let (b50a, binsa, lumaa, mada) = stat(&coff);
            let (b50b, binsb, lumab, madb) = stat(&con);
            println!(
                "  {preset:<11} {seed:<5}  {moved:<11}  {:<10.2}  {:<12.2}  {b50a:>3} > {b50b:<4}  {binsa:>4} > {binsb:<5}  {lumaa:>5.1} > {lumab:<7.1} {mada:>5.2} > {madb:.2}",
                100.0 * moved as f64 / total as f64,
                100.0 * moved as f64 / ground.max(1) as f64,
            );
            // Material shares in the viewport, on arm only -- what the rock
            // actually is where the player is standing, as against the
            // whole-world cell counts `massif detail` prints.
            let hist = con.material_hist(&on.materials);
            let mut shares: Vec<(usize, f64)> =
                hist.iter().copied().enumerate().filter(|(_, v)| *v > 0.002).collect();
            shares.sort_by(|x, y| y.1.total_cmp(&x.1));
            let line: Vec<String> = shares
                .iter()
                .map(|(i, v)| {
                    format!("{} {:.1}%", on.materials.get(MaterialId(*i as u16)).name, 100.0 * v)
                })
                .collect();
            println!("        on-arm viewport cells: {}", line.join("  "));
        }
    }
    println!();
    println!("  pooled: {all_moved} of {all_px} pixels moved ({:.2}%)", 100.0 * all_moved as f64 / all_px.max(1) as f64);
}

fn census_of(world: &World, cams: &[(i32, i32)]) -> Census {
    let mut c = Census::default();
    for &cam in cams {
        let v = render_view(world, cam);
        c.add_view(world, &v, cam);
    }
    c
}

fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

struct Args {
    mode: String,
    presets: Vec<String>,
    seeds: u64,
    views: usize,
    out: String,
    /// Frames of simulation before looking. **0 by default and that is a
    /// choice, not an oversight**: every generation pass has run, so the
    /// world is complete, and a settle adds physics rather than content.
    /// `viewshot` settles 60 and the owner's review cards come from it, so
    /// `settle=60` is the control that says whether these numbers describe
    /// the pictures he was actually judging.
    settle: usize,
    /// Which of the `views` viewports `mode=shot` writes.
    view: usize,
}

fn main() {
    let mut a = Args {
        mode: "composition".into(),
        presets: Vec::new(),
        seeds: 4,
        views: 8,
        out: String::new(),
        settle: 0,
        view: 4,
    };
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "mode" => a.mode = v.to_string(),
            "presets" => a.presets = v.split(',').map(|s| s.to_string()).collect(),
            "seeds" => a.seeds = v.parse().expect("seeds=N"),
            "views" => a.views = v.parse().expect("views=N"),
            "out" => a.out = v.to_string(),
            "settle" => a.settle = v.parse().expect("settle=N"),
            "view" => a.view = v.parse().expect("view=N"),
            _ => panic!("unknown argument {arg:?}"),
        }
    }
    let (presets, err) = worldgen::WorldgenPresets::load();
    if let Some(e) = err {
        panic!("{e}");
    }
    if a.presets.is_empty() {
        a.presets = presets.cycle_order().into_iter().filter(|n| presets.get(n).is_some()).collect();
    }
    // **Echoes its own parameters** — CLAUDE.md's rule, after a 3.5-hour
    // megastudy turned out to be three runs wearing 24 logs.
    println!(
        "world_look mode={} presets={} seeds={} views={} settle={} world={}x{} viewport={}x{} daylight={DAYLIGHT} bin={} levels/channel",
        a.mode,
        a.presets.join(","),
        a.seeds,
        a.views,
        a.settle,
        WORLD_WIDTH,
        WORLD_HEIGHT,
        WIDTH,
        HEIGHT,
        1 << (8 - BIN_SHIFT)
    );
    println!();

    match a.mode.as_str() {
        "control" => control(&presets, &a),
        "composition" => composition(&presets, &a),
        "colour" => colour(&presets, &a),
        "distance" => distance(&presets, &a),
        "passes" => passes(&presets, &a),
        "shot" => shot(&presets, &a),
        "vocab" => vocab(&presets, &a),
        m => panic!("unknown mode {m:?}"),
    }
}

/// **Every number in this file, run against a case whose answer is known.**
///
/// CLAUDE.md's rule, both halves: a metric must stay quiet when nothing is
/// wrong (specificity) *and* move when something is (sensitivity). Six checks,
/// each printed with the value it must produce.
fn control(presets: &worldgen::WorldgenPresets, a: &Args) {
    let params = presets.get(&a.presets[0]).expect("preset");
    let world = build(params, 1, "");
    let cams = camera_positions(&world, 2);

    // 1. Determinism / negative control: the same world twice must give
    //    exactly zero on every distance. A metric that cannot report zero
    //    cannot report a small number either.
    let c1 = census_of(&world, &cams);
    let world2 = build(params, 1, "");
    let c2 = census_of(&world2, &cams);
    let d = tv(&c1.ground_hist(), &c2.ground_hist());
    println!("CONTROL 1  same world, twice:      colour TV {d:.6}   (must be 0.000000)");
    let dm = tv(&c1.material_hist(&world.materials), &c2.material_hist(&world.materials));
    println!("CONTROL 1b same world, twice:      material TV {dm:.6}   (must be 0.000000)");

    // 2. Positive control: a world repainted in one material must read as a
    //    huge distance and as one material. If this does not move, the
    //    instrument cannot see a palette change at all.
    let mut mono = build(params, 1, "");
    let sand = mono.materials.id_of("sand").expect("sand");
    for &(cx, cy) in &cams {
        for py in 0..HEIGHT as i32 {
            for px in 0..WIDTH as i32 {
                let (x, y) = (cx + px, cy + py);
                if mono.get(x, y).material != material::EMPTY {
                    mono.set(x, y, Cell::new(sand, 0));
                }
            }
        }
    }
    let cm = census_of(&mono, &cams);
    let dp = tv(&c1.ground_hist(), &cm.ground_hist());
    let dpm = tv(&c1.material_hist(&world.materials), &cm.material_hist(&world.materials));
    println!("CONTROL 2  repainted all-sand:     colour TV {dp:.3}, material TV {dpm:.3}   (must be large)");

    // 3. The pixel<-cell mapping the ground mask depends on. Paint one cell
    //    lava and check exactly one pixel moved, at the position the mask
    //    assumes. A mask that is off by a row would silently misclassify sky
    //    as ground and every colour number with it.
    let mut poked = build(params, 1, "");
    let lava = poked.materials.id_of("lava").expect("lava");
    let (cx, cy) = cams[0];
    let (tx, ty) = (cx + 100, cy + HEIGHT as i32 - 20);
    poked.set(tx, ty, Cell::new(lava, 0));
    let before = render_view(&world, cams[0]);
    let after = render_view(&poked, cams[0]);
    let mut moved: Vec<(usize, usize)> = Vec::new();
    for i in 0..(WIDTH * HEIGHT) as usize {
        if before.frame[i * 4..i * 4 + 3] != after.frame[i * 4..i * 4 + 3] {
            moved.push((i % WIDTH as usize, i / WIDTH as usize));
        }
    }
    println!(
        "CONTROL 3  one cell repainted lava: {} pixels moved, first at {:?}   (expect ~1, at (100,{}))",
        moved.len(),
        moved.first(),
        HEIGHT as i32 - 20
    );

    // 4. The grain, stated as a number: raw distinct RGB against binned.
    println!(
        "CONTROL 4  grain: {} exact RGB values on ground pixels, {} occupied bins   (the gap IS the per-cell jitter)",
        c1.exact.len(),
        c1.ground_bins.len()
    );

    // 5. The ablation arm must be able to report a large change. Switching
    //    off `stone_massif` removes the world; if that does not light up the
    //    pixel diff, `mode=passes` is measuring nothing.
    let gutted = build(params, 1, "stone_massif");
    let mut changed = 0u64;
    for &cam in &cams {
        let x = render_view(&world, cam);
        let y = render_view(&gutted, cam);
        for i in 0..(WIDTH * HEIGHT) as usize {
            if x.frame[i * 4..i * 4 + 3] != y.frame[i * 4..i * 4 + 3] {
                changed += 1;
            }
        }
    }
    let total = cams.len() as u64 * (WIDTH * HEIGHT) as u64;
    println!(
        "CONTROL 5  without stone_massif:   {changed} of {total} pixels differ ({:.1}%)   (must be ~half: the camera puts the ground line mid-screen, so the sky half cannot change)",
        changed as f64 * 100.0 / total as f64
    );

    // 6. ...and the same ablation machinery with nothing ablated must give
    //    exactly zero, or every row of `mode=passes` is noise.
    let again = build(params, 1, "");
    let mut same = 0u64;
    for &cam in &cams {
        let x = render_view(&world, cam);
        let y = render_view(&again, cam);
        for i in 0..(WIDTH * HEIGHT) as usize {
            if x.frame[i * 4..i * 4 + 3] != y.frame[i * 4..i * 4 + 3] {
                same += 1;
            }
        }
    }
    println!("CONTROL 6  ablation of nothing:    {same} pixels differ   (must be 0)");
}

fn composition(presets: &worldgen::WorldgenPresets, a: &Args) {
    println!("Composition of a player viewport, aimed at the ground, {} views spread across each world.", a.views);
    println!("  'sky' is the share of viewport cells holding no material; the rest are shares of the cells that do.");
    println!("  'skin' is the top {SKIN} cells of material in every column — what reads as the ground surface.");
    println!();
    for preset in &a.presets {
        let Some(params) = presets.get(preset) else { continue };
        let mut agg = Census::default();
        let mut names: Vec<String> = Vec::new();
        for seed in 1..=a.seeds {
            let world = build_settled(params, seed, "", a.settle);
            if names.is_empty() {
                names = (0..world.materials.len()).map(|i| world.materials.get(MaterialId(i as u16)).name.clone()).collect();
            }
            let cams = camera_positions(&world, a.views);
            let c = census_of(&world, &cams);
            for (k, v) in c.cells {
                *agg.cells.entry(k).or_default() += v;
            }
            for (k, v) in c.skin {
                *agg.skin.entry(k).or_default() += v;
            }
            agg.ground_px += c.ground_px;
            agg.all_px += c.all_px;
        }
        let sky = *agg.cells.get(&material::EMPTY.0).unwrap_or(&0);
        let total: u64 = agg.cells.values().sum();
        let solid = total - sky;
        let mut rows: Vec<(f64, String)> = agg
            .cells
            .iter()
            .filter(|(&m, _)| m != material::EMPTY.0)
            .map(|(&m, &n)| (n as f64 * 100.0 / solid.max(1) as f64, names[m as usize].clone()))
            .collect();
        rows.sort_by(|x, y| y.0.total_cmp(&x.0));
        let skin_total: u64 = agg.skin.values().sum();
        let mut skin_rows: Vec<(f64, String)> = agg
            .skin
            .iter()
            .map(|(&m, &n)| (n as f64 * 100.0 / skin_total.max(1) as f64, names[m as usize].clone()))
            .collect();
        skin_rows.sort_by(|x, y| y.0.total_cmp(&x.0));
        println!("### {preset}   (sky {:.1}% of viewport cells)", sky as f64 * 100.0 / total as f64);
        print!("  viewport:");
        for (p, n) in rows.iter().take(8) {
            if *p >= 0.05 {
                print!("  {n} {p:.1}%");
            }
        }
        let top2: f64 = rows.iter().take(2).map(|r| r.0).sum();
        println!("   [top two = {top2:.1}%]");
        print!("  skin({SKIN}):");
        for (p, n) in skin_rows.iter().take(8) {
            if *p >= 0.05 {
                print!("  {n} {p:.1}%");
            }
        }
        let stop2: f64 = skin_rows.iter().take(2).map(|r| r.0).sum();
        println!("   [top two = {stop2:.1}%]");
        println!();
    }
}

fn colour(presets: &worldgen::WorldgenPresets, a: &Args) {
    println!("Colour census over ground pixels only (sky excluded — it is the same gradient in every preset).");
    println!("  'bins' are {}-level-per-channel colour bins, chosen to be the width of render.rs's own grain jitter.", 1 << (8 - BIN_SHIFT));
    println!("  'b50/b90' is how many bins cover half / nine tenths of the ground.");
    println!();
    println!("  preset      seed   ground px    exact RGB   bins   b50   b90   entropy   mean luma   luma sd   relief   |step|   speckle");
    for preset in &a.presets {
        let Some(params) = presets.get(preset) else { continue };
        for seed in 1..=a.seeds {
            let world = build_settled(params, seed, "", a.settle);
            let cams = camera_positions(&world, a.views);
            let c = census_of(&world, &cams);
            let h = c.ground_hist();
            let n = c.ground_px.max(1) as f64;
            let mean = c.luma_sum / n;
            let sd = (c.luma_sq / n - mean * mean).max(0.0).sqrt();
            println!(
                "  {preset:<11} {seed:<5} {:<11} {:<11} {:<6} {:<5} {:<5} {:<9.3} {:<11.1} {:<9.1} {:<8.1} {:<8.2} {:.3}",
                c.ground_px,
                c.exact.len(),
                c.ground_bins.len(),
                bins_covering(&h, 0.5),
                bins_covering(&h, 0.9),
                shannon(&h),
                mean,
                sd,
                c.relief(),
                {
                    let sh = c.slope_hist();
                    sh.iter().enumerate().map(|(i, p)| p * (i as f64 - SLOPE_CAP as f64 - 1.0).abs()).sum::<f64>()
                },
                c.mad()
            );
        }
    }
}

fn distance(presets: &worldgen::WorldgenPresets, a: &Args) {
    println!("The crux: is preset-to-preset further apart than seed-to-seed within one preset?");
    println!("  TV distance over ground colour bins: 0 = the same histogram, 1 = no colour in common.");
    println!("  Read it as the fraction of the ground that would have to be repainted to swap one for the other.");
    println!();
    let mut hists: HashMap<(String, u64), Descriptors> = HashMap::new();
    for preset in &a.presets {
        let Some(params) = presets.get(preset) else { continue };
        for seed in 1..=a.seeds {
            let world = build_settled(params, seed, "", a.settle);
            let cams = camera_positions(&world, a.views);
            let c = census_of(&world, &cams);
            hists.insert((preset.clone(), seed), (c.ground_hist(), c.material_hist(&world.materials), c.slope_hist()));
        }
    }
    let mut within_c: Vec<f64> = Vec::new();
    let mut within_m: Vec<f64> = Vec::new();
    let mut within_s: Vec<f64> = Vec::new();
    let mut between_c: Vec<f64> = Vec::new();
    let mut between_m: Vec<f64> = Vec::new();
    let mut between_s: Vec<f64> = Vec::new();
    // Per-pair table, because a mean over pairs hides the one preset that
    // does separate. `flat` is expected to: it ships `region_variation = 0`.
    let mut pairs: Vec<(f64, String)> = Vec::new();
    let keys: Vec<&(String, u64)> = hists.keys().collect();
    for (i, k1) in keys.iter().enumerate() {
        for k2 in keys.iter().skip(i + 1) {
            let (h1, m1, s1) = &hists[*k1];
            let (h2, m2, s2) = &hists[*k2];
            let (dc, dm, ds) = (tv(h1, h2), tv(m1, m2), tv(s1, s2));
            if k1.0 == k2.0 {
                within_c.push(dc);
                within_m.push(dm);
                within_s.push(ds);
            } else {
                between_c.push(dc);
                between_m.push(dm);
                between_s.push(ds);
                pairs.push((dc, format!("{} s{} vs {} s{}", k1.0, k1.1, k2.0, k2.1)));
            }
        }
    }
    let (mut wc, mut wm, mut bc, mut bm) = (within_c.clone(), within_m.clone(), between_c.clone(), between_m.clone());
    println!("  same preset, different seed : colour TV median {:.3}  (n={})   material TV median {:.3}", median(&mut wc), within_c.len(), median(&mut wm));
    println!("  different preset            : colour TV median {:.3}  (n={})   material TV median {:.3}", median(&mut bc), between_c.len(), median(&mut bm));
    let (mut wc2, mut bc2) = (within_c.clone(), between_c.clone());
    let (w, b) = (median(&mut wc2), median(&mut bc2));
    println!();
    println!("  RATIO between/within (colour) = {:.2}x     1.0 means presets do not exist to the eye", if w > 0.0 { b / w } else { f64::INFINITY });
    let (mut wm2, mut bm2) = (within_m.clone(), between_m.clone());
    let (w2, b2) = (median(&mut wm2), median(&mut bm2));
    println!("  RATIO between/within (material) = {:.2}x", if w2 > 0.0 { b2 / w2 } else { f64::INFINITY });
    let (mut ws3, mut bs3) = (within_s.clone(), between_s.clone());
    let (w3, b3) = (median(&mut ws3), median(&mut bs3));
    println!("  same preset, different seed : SHAPE TV median {w3:.3}   (skyline step histogram)");
    println!("  different preset            : SHAPE TV median {b3:.3}");
    println!("  RATIO between/within (shape) = {:.2}x", if w3 > 0.0 { b3 / w3 } else { f64::INFINITY });
    println!();
    // **The matrix, because the two medians above can hide the case that
    // matters.** A pooled between-preset median is an average over pairs
    // that behave completely differently: `arid` against `wetland` and
    // `rolling` against `terraced` are both "different presets" and are not
    // the same finding. The diagonal is the within-preset distance for that
    // preset, so a cell can be read against its own row's diagonal.
    let mut names: Vec<String> = a.presets.clone();
    names.retain(|n| hists.keys().any(|k| &k.0 == n));
    let mut cell: HashMap<(String, String), Vec<f64>> = HashMap::new();
    for (i, k1) in keys.iter().enumerate() {
        for k2 in keys.iter().skip(i + 1) {
            let d = tv(&hists[*k1].0, &hists[*k2].0);
            let (p, q) = if k1.0 <= k2.0 { (k1.0.clone(), k2.0.clone()) } else { (k2.0.clone(), k1.0.clone()) };
            cell.entry((p, q)).or_default().push(d);
        }
    }
    println!();
    println!("  median colour TV, every preset against every preset (diagonal = same preset, different seed):");
    print!("  {:<12}", "");
    for n in &names {
        print!("{:>10}", n);
    }
    println!();
    for r in &names {
        print!("  {r:<12}");
        for c in &names {
            let (p, q) = if r <= c { (r.clone(), c.clone()) } else { (c.clone(), r.clone()) };
            match cell.get_mut(&(p, q)) {
                Some(v) => print!("{:>10.3}", median(v)),
                None => print!("{:>10}", "-"),
            }
        }
        println!();
    }
    println!();
    pairs.sort_by(|x, y| y.0.total_cmp(&x.0));
    println!("  furthest preset pairs:");
    for (d, n) in pairs.iter().take(6) {
        println!("    {d:.3}  {n}");
    }
    println!("  closest preset pairs:");
    for (d, n) in pairs.iter().rev().take(6) {
        println!("    {d:.3}  {n}");
    }
    let mut wsorted = within_c.clone();
    wsorted.sort_by(f64::total_cmp);
    if !wsorted.is_empty() {
        println!();
        println!(
            "  within-preset spread: min {:.3} max {:.3}   between-preset: min {:.3} max {:.3}",
            wsorted[0],
            wsorted[wsorted.len() - 1],
            between_c.iter().copied().fold(f64::INFINITY, f64::min),
            between_c.iter().copied().fold(0.0, f64::max)
        );
    }
}

fn passes(presets: &worldgen::WorldgenPresets, a: &Args) {
    println!("Every generation pass, ablated, measured in RENDERED PIXELS rather than in cells.");
    println!("  Cameras are computed from the un-ablated world and reused, so both arms look at the same place.");
    println!("  'px' is pixels whose colour differs, over {} views x {}x{} = {} pixels per world.", a.views, WIDTH, HEIGHT, a.views as u64 * (WIDTH * HEIGHT) as u64);
    println!("  Median over {} seeds.", a.seeds);
    println!();
    let names = worldgen::pass_names();
    let bounds = Rect::new(0, 0, WORLD_WIDTH as i32 - 1, WORLD_HEIGHT as i32 - 1);
    for preset in &a.presets {
        let Some(params) = presets.get(preset) else { continue };
        println!("### {preset}");
        println!("  pass             visible px   % of view   mean |dluma|   cells the pass wrote");
        // **Both arms go through `generate_ablated`**, the un-ablated one with
        // an empty `skip`. Building the baseline with `generate` instead would
        // leave the comparison resting on the two entry points agreeing, which
        // is an assumption the diff would silently absorb.
        let mut baselines: Vec<Baseline> = Vec::new();
        for seed in 1..=a.seeds {
            let mut base = World::new(bounds);
            let report = worldgen::generate_ablated(&mut base, worldgen::Spec::Generated { params, seed }, "");
            let cams = camera_positions(&base, a.views);
            let views: Vec<View> = cams.iter().map(|&c| render_view(&base, c)).collect();
            baselines.push((cams, views, report));
        }
        let mut rows: Vec<(f64, String)> = Vec::new();
        for skipped in &names {
            let mut px: Vec<f64> = Vec::new();
            let mut dl: Vec<f64> = Vec::new();
            let mut wrote: Vec<f64> = Vec::new();
            for (si, seed) in (1..=a.seeds).enumerate() {
                let (cams, base_views, report) = &baselines[si];
                wrote.push(report.iter().find(|(n, _)| n == skipped).map(|(_, n)| *n).unwrap_or(0) as f64);
                let mut abl = World::new(bounds);
                worldgen::generate_ablated(&mut abl, worldgen::Spec::Generated { params, seed }, skipped);
                let mut changed = 0u64;
                let mut sum = 0.0f64;
                for (vi, &cam) in cams.iter().enumerate() {
                    let y = render_view(&abl, cam);
                    let x = &base_views[vi];
                    for i in 0..(WIDTH * HEIGHT) as usize {
                        let (p, q) = (&x.frame[i * 4..i * 4 + 3], &y.frame[i * 4..i * 4 + 3]);
                        if p != q {
                            changed += 1;
                            sum += (luma(p) - luma(q)).abs() as f64;
                        }
                    }
                }
                px.push(changed as f64);
                dl.push(sum / (cams.len() as f64 * (WIDTH * HEIGHT) as f64));
            }
            let total = a.views as f64 * (WIDTH * HEIGHT) as f64;
            let m = median(&mut px);
            rows.push((
                m,
                format!(
                    "  {skipped:<16} {:<12.0} {:<11.3} {:<14.4} {:.0}",
                    m,
                    m * 100.0 / total,
                    median(&mut dl),
                    median(&mut wrote)
                ),
            ));
        }
        rows.sort_by(|x, y| y.0.total_cmp(&x.0));
        for (_, line) in rows {
            println!("{line}");
        }
        println!();
    }
}
