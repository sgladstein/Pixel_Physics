//! **Is a wide void a room or a tunnel into the screen?** -- one roof rule
//! under two readings of the hidden dimension, for the owner to judge by eye
//! (`Reports/soil-physics-realism-2026-10-21.md` §5.6).
//!
//! The engine's own roofs ignore span (§2 of that report). This harness
//! adds, **outside the engine**, one candidate rule and runs it beside the
//! normal sweep: a worked roof cell with open space under it fails (turns
//! back into loose soil, which the sweep then drops) when it is more than
//! `half_span(moisture)` cells from the nearer wall of the opening under it.
//! The span the soil can hold peaks in damp ground and shrinks when dry or
//! soaked -- capillary cohesion (Lu & Likos 2013; Monaenkova et al. 2015).
//! Failure from the middle, row by row, leaves a dome rather than a crater:
//! each row that fails narrows the opening over it, so the sag stops on its
//! own.
//!
//! | reading | span the rule uses |
//! |---|---|
//! | `today` | no span rule (the shipped engine) |
//! | `slice` | the world is a slice of a deep 3D one, so a void is a body-wide tunnel into the screen: `min(width on screen, HIDDEN)` |
//! | `farm` | the world is a thin slab behind glass, an ant farm: the width on screen |
//!
//! Scenes: a strip of lined chambers 2 to 96 wide; and real nests -- s171's
//! in the soil arena (`challenges::Soil`) and a shipped-ant colony's bank --
//! with the animals lifted out and the ground kept. Every scene is run at
//! three wetnesses. Fallen ground is painted red where it lands and the
//! hole it left pink, and the count of failed roof cells is printed per
//! panel (`CLAUDE.md`: did it fire needs a counter).
//!
//! ```text
//! cargo run --release --example span_readings -- out=Reports/img/soil-physics-2026-10-21
//! ```

mod common;

use common::PlantScene;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::material::{self, MaterialKind};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::weather::Pin;
use pixel_physics::sim::{frame, player, Cell, World};

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses"))
    })
}

/// The ant is a chain of two cells (`assets/species/ant.ron`), so one body
/// length is 2 cells; real tunnels are about 1.06 body lengths across
/// (Gravish et al. 2013). The hidden width of a tunnel under the `slice`
/// reading.
const HIDDEN: i32 = 2;

/// Half the span damp soil holds flat, in cells: 12 body lengths across at
/// best, the order of a harvester-ant chamber. A setting, not a measurement.
const DAMP_HALF_SPAN: f32 = 12.0;

/// Relative cohesion against soil water (0..1000): weak dry, strongest damp,
/// weakest soaked -- the shape of apparent cohesion, not its size.
fn cohesion(m: u16) -> f32 {
    let m = m as f32;
    if m < 450.0 {
        0.35 + 0.65 * m / 450.0
    } else if m <= 700.0 {
        1.0
    } else {
        1.0 - 0.75 * ((m - 700.0) / 300.0).min(1.0)
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Reading {
    Today,
    Slice,
    Farm,
    /// The ant farm at half the span: 6 body lengths across when damp.
    Strict,
}

impl Reading {
    fn name(self) -> &'static str {
        match self {
            Reading::Today => "today",
            Reading::Slice => "slice",
            Reading::Farm => "farm",
            Reading::Strict => "farm/2",
        }
    }
}

/// Nothing holding a roof up: air, an animal, or liquid.
fn open(w: &World, x: i32, y: i32) -> bool {
    let c = w.get(x, y);
    c.material == material::EMPTY
        || matches!(w.materials.kind(c.material), MaterialKind::Creature | MaterialKind::Liquid)
}

/// One pass of the candidate rule. Returns how many roof cells failed.
fn span_rule(w: &mut World, reading: Reading) -> usize {
    if reading == Reading::Today {
        return 0;
    }
    let b = w.bounds().expect("bounded world");
    let mut fail = Vec::new();
    for y in b.min_y..b.max_y {
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            let m = w.materials.get(c.material);
            if !(m.kind == MaterialKind::Powder && m.self_supporting && m.slumps_into.is_some()) {
                continue;
            }
            if !open(w, x, y + 1) {
                continue;
            }
            let (mut x0, mut x1) = (x, x);
            while x0 > b.min_x && open(w, x0 - 1, y + 1) {
                x0 -= 1;
            }
            while x1 < b.max_x && open(w, x1 + 1, y + 1) {
                x1 += 1;
            }
            let width = x1 - x0 + 1;
            let d = (x - x0).min(x1 - x) + 1;
            let d = match reading {
                Reading::Farm | Reading::Strict => d,
                // A body-wide tunnel seen side-on: the roof spans the hidden
                // width, whatever its length on screen.
                _ => d.min((width.min(HIDDEN) + 1) / 2),
            };
            let half = if reading == Reading::Strict { DAMP_HALF_SPAN / 2.0 } else { DAMP_HALF_SPAN };
            if d as f32 > half * cohesion(c.aux()) {
                fail.push((x, y));
            }
        }
    }
    for &(x, y) in &fail {
        let mut c = w.get(x, y);
        c.material = w.materials.get(c.material).slumps_into.expect("checked");
        w.set(x, y, c);
    }
    // **The ground over a failed roof cell becomes the new roof.** Unworked
    // soil here is a loose powder that stands only because lining holds it,
    // so without this one failed lining cell opens a chimney and the whole
    // column pours in -- a cave-in, which is what the owner declined. Real
    // undisturbed soil keeps its own cohesion; only the over-span part
    // fails. So the three cells above each failure are worked, as a cut
    // works its neighbours, and the next row is judged on its own span.
    if relining() {
        for &(x, y) in &fail {
            for dx in -1..=1 {
                let c = w.get(x + dx, y - 1);
                if let Some(p) = w.materials.get(c.material).packs_into {
                    let mut l = c;
                    l.material = p;
                    w.set(x + dx, y - 1, l);
                }
            }
        }
    }
    fail.len()
}

/// `reline=0` turns the re-lining off, to show the cave-in it prevents.
fn relining() -> bool {
    arg::<u8>("reline").unwrap_or(1) != 0
}

/// Set every water-holding cell to `aux`.
fn wet(w: &mut World, aux: u16) {
    let b = w.bounds().expect("bounded");
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            if w.materials.get(c.material).water_capacity > 0 {
                w.set(x, y, c.with_aux(aux));
            }
        }
    }
}

/// Ground of `w` in a fresh world: animals and anything alive lifted out.
fn ground_only(w: &World) -> World {
    let b = w.bounds().expect("bounded");
    let mut g = World::new(Rect::new(b.min_x, b.min_y, b.max_x, b.max_y));
    g.set_weather_pin(Pin::Clear);
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            if c.material == material::EMPTY
                || c.organism_id() != 0
                || matches!(w.materials.kind(c.material), MaterialKind::Creature | MaterialKind::Plant)
            {
                continue;
            }
            g.set(x, y, c);
        }
    }
    g
}

/// Run `start` under `reading` for `frames`, the rule every 4 frames.
/// Returns the end world and the roof cells failed.
fn run(start: &World, reading: Reading, frames: u64) -> (World, usize) {
    let mut w = start.clone();
    let (mut p, mut bl) = (ParticleSystem::default(), Blasts::default());
    let tuning = player::Tuning::default();
    let mut failed = 0;
    for f in 0..frames {
        if f % 4 == 0 {
            failed += span_rule(&mut w, reading);
        }
        frame::step(&mut w, &mut p, &mut bl, player::PlayerInput::default(), &tuning);
    }
    (w, failed)
}

/// Flat material map of `crop`, with ground that arrived since `before`
/// painted red and ground that left painted pink.
fn paint(before: &World, after: &World, crop: (i32, i32, i32, i32)) -> Vec<u8> {
    let (cx, cy, cw, ch) = crop;
    let ground = |w: &World, x: i32, y: i32| {
        let c = w.get(x, y);
        c.material != material::EMPTY && matches!(w.materials.kind(c.material), MaterialKind::Powder | MaterialKind::Solid)
    };
    let mut out = Vec::with_capacity((cw * ch * 4) as usize);
    for y in cy..cy + ch {
        for x in cx..cx + cw {
            let c = after.get(x, y);
            let name = after.materials.get(c.material).name.as_str();
            let was = ground(before, x, y);
            let is = ground(after, x, y);
            let rgb: [u8; 3] = if is && !was {
                [235, 40, 40]
            } else if was && !is {
                [255, 170, 200]
            } else if c.material == material::EMPTY {
                [12, 12, 16]
            } else {
                match name {
                    "soil" => [110, 78, 48],
                    "packedsoil" => [214, 140, 60],
                    "spoil" => [236, 214, 92],
                    "water" => [60, 120, 230],
                    "stone" => [120, 120, 120],
                    _ => [90, 90, 110],
                }
            };
            out.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
    }
    out
}

/// Stack panels (all `cw` x `ch`) into a grid of `cols` columns, zoomed.
/// A named scene and the crop it is judged in.
type Scene = (String, World, (i32, i32, i32, i32));

fn sheet(panels: &[Vec<u8>], cw: u32, ch: u32, cols: u32, zoom: u32, path: &str) {
    let rows = (panels.len() as u32).div_ceil(cols);
    let gap = 3u32;
    let sw = (cols * cw + (cols - 1) * gap) * zoom;
    let sh = (rows * ch + (rows - 1) * gap) * zoom;
    let mut img = vec![230u8; (sw * sh * 4) as usize];
    for (i, p) in panels.iter().enumerate() {
        let (r, c) = (i as u32 / cols, i as u32 % cols);
        let (ox, oy) = (c * (cw + gap) * zoom, r * (ch + gap) * zoom);
        for y in 0..ch * zoom {
            for x in 0..cw * zoom {
                let s = (((y / zoom) * cw + x / zoom) * 4) as usize;
                let d = (((oy + y) * sw + ox + x) * 4) as usize;
                img[d..d + 4].copy_from_slice(&p[s..s + 4]);
            }
        }
    }
    image::save_buffer(path, &img, sw, sh, image::ColorType::Rgba8).expect("writing the sheet");
    println!("wrote {path} ({sw}x{sh})");
}

const WETS: [(&str, u16); 3] = [("dry", 100), ("damp", 620), ("soaked", 1000)];
const READINGS: [Reading; 4] = [Reading::Today, Reading::Slice, Reading::Farm, Reading::Strict];

fn chambers() -> (World, (i32, i32, i32, i32)) {
    let (wd, ht) = (300, 70);
    let scene = PlantScene {
        width: wd,
        height: ht,
        ground_y: 6,
        soil_depth: 56,
        trees: 0,
        species: "herb".to_string(),
        seed: Some(1),
        ..PlantScene::default()
    };
    let mut w = scene.build();
    w.set_weather_pin(Pin::Clear);
    let roof = 24;
    let mut carved = std::collections::HashSet::new();
    let mut x = 6;
    for span in [2, 6, 12, 24, 48, 96] {
        for y in roof..roof + 3 {
            for cx in x..x + span {
                carved.insert((cx, y));
            }
        }
        x += span + 14;
    }
    for &(cx, cy) in &carved {
        w.set(cx, cy, Cell::EMPTY);
    }
    // `line_burrow`'s lining in one pass (see `soil_cases`).
    let mut ring = Vec::new();
    for &(cx, cy) in &carved {
        for dy in -1..=1 {
            for dx in -1..=1 {
                if !carved.contains(&(cx + dx, cy + dy)) {
                    ring.push((cx + dx, cy + dy));
                }
            }
        }
    }
    for (rx, ry) in ring {
        let c = w.get(rx, ry);
        if let Some(p) = w.materials.get(c.material).packs_into {
            let mut l = c;
            l.material = p;
            w.set(rx, ry, l);
        }
    }
    (w, (0, roof - 20, wd, 28))
}

/// s171's nest at the end of a soil-arena run, ground only.
fn s171_nest(genome: &str, layout_k: u64) -> World {
    use pixel_physics::train::challenges::Soil;
    use pixel_physics::train::{load_genome, sealed_layouts, Challenge};
    let head = std::fs::read_to_string(genome).expect("genome");
    let len: usize = head
        .lines()
        .next()
        .and_then(|l| l.strip_prefix("genome "))
        .and_then(|l| l.split_whitespace().next())
        .and_then(|n| n.parse().ok())
        .expect("sparse genome");
    let g = load_genome(std::path::Path::new(genome), len);
    let soil = Soil { ants: 1, frames: 12_000, step: 4, shaft: 8, budget: Some(1200.0), foot: true, step_near: 0.0, nest: false };
    let layout = sealed_layouts(layout_k as usize + 1)[layout_k as usize];
    let mut last: Option<World> = None;
    soil.run(layout, &[&g], &mut |f, w| {
        if f == 12_000 {
            last = Some(w.clone());
        }
    });
    ground_only(&last.expect("the run reached its last frame"))
}

/// A shipped-ant colony's bank after `frames` (the `burrow_probe` colony
/// scene), ground only.
fn colony_nest(seed: u64, frames: u64) -> World {
    let (wd, ht) = (200i32, 120i32);
    let mut w = World::new(Rect::new(0, 0, wd - 1, ht - 1));
    w.seed = seed;
    w.set_weather_pin(Pin::Clear);
    w.set_sky_hold(Some(pixel_physics::sky::frame_for_daylight(1.0)));
    let soil = w.materials.id_of("soil").expect("soil");
    let nest = w.materials.id_of("nest").expect("nest");
    let floor = ht - 8;
    for x in 0..wd {
        for y in floor..ht {
            w.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
        }
    }
    for x in 40..160 {
        for y in (floor - 30)..floor {
            w.set(x, y, Cell::new(soil, 0).with_attached(true));
        }
    }
    for x in 16..40 {
        w.set(x, floor, Cell::new(nest, 0).with_attached(true));
    }
    let mut slots: Vec<(i32, i32)> = (16..40).step_by(2).flat_map(|x| (1..=6).map(move |r| (x, floor - r))).collect();
    let mut draw = pixel_physics::sim::rng::stream(seed, 0xB0_1707, 0, 0);
    for i in (1..slots.len()).rev() {
        slots.swap(i, draw.below(i as u32 + 1) as usize);
    }
    let mut colony = None;
    for &(x, y) in slots.iter().take(55) {
        if let Some(site) = pixel_physics::sim::creature::plant_creature_seed_in(&mut w, x, y, "ant", colony) {
            colony = colony.or_else(|| pixel_physics::sim::creature::colony_of_site(&w, &site));
            w.schedule_active_site(site);
        }
    }
    let (mut p, mut bl) = (ParticleSystem::default(), Blasts::default());
    let tuning = player::Tuning::default();
    for _ in 0..frames {
        frame::step(&mut w, &mut p, &mut bl, player::PlayerInput::default(), &tuning);
    }
    ground_only(&w)
}

/// The played lab's nest (`LabBox`, one colony) after `frames`, ground only.
/// Plants are lifted out with the animals, roots included, so a bank a root
/// was pinning loses that hold here.
fn lab_nest(seed: u64, frames: u64) -> World {
    use pixel_physics::lab::scene::LabBox;
    let spec = LabBox { colonies: 1, seed, ..LabBox::default() };
    let mut w = spec.build();
    let (mut p, mut bl) = (ParticleSystem::default(), Blasts::default());
    let tuning = player::Tuning::default();
    for _ in 0..frames {
        frame::step(&mut w, &mut p, &mut bl, player::PlayerInput::default(), &tuning);
    }
    ground_only(&w)
}

/// The bounding box of worked ground (packed soil and spoil), padded: what
/// the ants built, which is the part of the world there is to judge.
fn nest_crop(w: &World, pad: i32) -> (i32, i32, i32, i32) {
    let b = w.bounds().expect("bounded");
    let worked: Vec<_> = ["packedsoil", "spoil"].iter().filter_map(|n| w.materials.id_of(n)).collect();
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            if worked.contains(&w.get(x, y).material) {
                x0 = x0.min(x);
                x1 = x1.max(x);
                y0 = y0.min(y);
                y1 = y1.max(y);
            }
        }
    }
    assert!(x0 <= x1, "no worked ground: nothing was dug");
    let x0 = (x0 - pad).max(b.min_x);
    let y0 = (y0 - pad).max(b.min_y);
    let x1 = (x1 + pad).min(b.max_x);
    let y1 = (y1 + pad).min(b.max_y);
    (x0, y0, x1 - x0 + 1, y1 - y0 + 1)
}

fn main() {
    let out: String = arg("out").unwrap_or_else(|| ".".into());
    let frames: u64 = arg("frames").unwrap_or(600);
    let genome: String =
        arg("genome").unwrap_or_else(|| "Reports/brain-training/stage2/dig/8/dig-8-s171-hold-best.txt".into());
    let zoom: u32 = arg("zoom").unwrap_or(3);
    if let Some(h) = pixel_physics::sim::update::roof_span() {
        println!("PIXEL_PHYSICS_ROOF_SPAN is on (half-span {h}): the `today` column is the ENGINE's span rule");
    }
    println!(
        "span_readings: frames={frames} hidden={HIDDEN} damp_half_span={DAMP_HALF_SPAN} cohesion dry {:.2} damp {:.2} soaked {:.2}",
        cohesion(100),
        cohesion(620),
        cohesion(1000)
    );

    let mut scenes: Vec<Scene> = Vec::new();
    let (c, crop) = chambers();
    scenes.push(("chambers".into(), c, crop));
    for k in [0u64, 3] {
        let n = s171_nest(&genome, k);
        let crop = nest_crop(&n, 4);
        scenes.push((format!("s171-sealed{k}"), n, crop));
    }
    let colony_frames: u64 = arg("colonyframes").unwrap_or(8_000);
    for seed in [1u64, 2] {
        let n = colony_nest(seed, colony_frames);
        let crop = nest_crop(&n, 3);
        scenes.push((format!("colony{seed}"), n, crop));
    }

    let lab_frames: u64 = arg("labframes").unwrap_or(30_000);
    for seed in (1u64..=2).take(arg::<usize>("labseeds").unwrap_or(2)) {
        let t = std::time::Instant::now();
        let n = lab_nest(seed, lab_frames);
        let crop = nest_crop(&n, 3);
        println!("lab seed {seed}: {lab_frames} frames in {:.2} s", t.elapsed().as_secs_f64());
        scenes.push((format!("lab{seed}"), n, crop));
    }

    for (name, base, crop) in &scenes {
        println!("\n=== {name} (crop {crop:?}) ===  rows: dry, damp, soaked; columns: today, slice, farm, farm/2");
        let mut panels = Vec::new();
        for (wname, aux) in WETS {
            let mut start = base.clone();
            wet(&mut start, aux);
            // **Settle first, under today's rules.** Lifting the animals and
            // plants out leaves pellets and roots' soil unheld, and that
            // falls on any reading; settling it here keeps every panel a
            // picture of the rule alone, so `today` must show no change.
            // `settle=0` skips it: the check that the engine's own rule
            // (`PIXEL_PHYSICS_ROOF_SPAN`) draws what the harness's does,
            // which settling under an engine with the rule on would swallow.
            let start = if arg::<u8>("settle").unwrap_or(1) != 0 { run(&start, Reading::Today, frames).0 } else { start };
            let mut line = format!("  {wname:<7}");
            for r in READINGS {
                let (end, failed) = run(&start, r, frames);
                // Ground standing where there was none: the rubble, counted
                // the same way whichever rule made it -- the harness's or,
                // with `PIXEL_PHYSICS_ROOF_SPAN` set, the engine's own.
                let b = start.bounds().expect("bounded");
                let mut fallen = 0;
                for y in b.min_y..=b.max_y {
                    for x in b.min_x..=b.max_x {
                        let g = |w: &World| {
                            let c = w.get(x, y);
                            c.material != material::EMPTY
                                && matches!(w.materials.kind(c.material), MaterialKind::Powder | MaterialKind::Solid)
                        };
                        if g(&end) && !g(&start) {
                            fallen += 1;
                        }
                    }
                }
                line.push_str(&format!("  {}: {:>4} failed {:>4} fell", r.name(), failed, fallen));
                panels.push(paint(&start, &end, *crop));
            }
            println!("{line}");
        }
        sheet(&panels, crop.2 as u32, crop.3 as u32, 4, zoom, &format!("{out}/span-{name}.png"));
    }
}
