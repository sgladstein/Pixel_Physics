//! Renders a scene to a **contact sheet**: several frames of one run laid out
//! in a grid, through the same `Renderer` the sandbox itself draws with, with
//! no window and no GPU.
//!
//! This exists because a numeric metric only ever sees the one quantity it was
//! written to see, and choosing that quantity correctly turns out to be the
//! hard part. Repeatedly, a metric written before anyone had looked at the
//! artifact measured the wrong thing and reported "nothing here" on a scene
//! that visibly had something: surface height hid a 9x chunk-seam effect that
//! column volume showed plainly, and occupancy hid torn seam rows that were a
//! fill deficit rather than a hole. Worse, a metric cannot see what it was not
//! asked about, so a fix that cleared one artifact while introducing a larger
//! one passed its own test and had to be reverted from live play
//! (`e816477`). An image has the opposite property: it shows everything at
//! once, including whatever nobody thought to measure.
//!
//! `examples/ascii.rs` already makes this argument ("movement rules are far
//! easier to judge by eye than by assertion") and answers it in the terminal.
//! This answers it in actual pixels, which is what the bug reports are about.
//!
//! # Why a grid rather than a GIF
//!
//! `main.rs`'s capture hook already writes a `sequence.gif`, and it is the
//! right thing for a human watching it. But several of these bugs are only
//! legible *in motion* — a fringe that regenerates every frame reads very
//! differently from one that sits still — and an animation cannot be taken in
//! at a glance or quoted in a report. Laying time out along space solves both:
//! one image, one look, motion visible as difference between neighbouring
//! tiles.
//!
//! # Why crop and zoom are not optional extras
//!
//! At 1:1 a one-pixel-tall dark line across a 512-wide world is genuinely
//! easy to miss — that is exactly how the horizontal chunk-seam tearing went
//! unnoticed until it was pointed out. `crop` and `zoom` are the difference
//! between "I looked at it" and "I saw it".
//!
//! ```text
//! cargo run --release --example filmstrip -- scene=pour start=100 every=60 count=6
//! cargo run --release --example filmstrip -- scene=pour crop=160,180,120,80 zoom=4
//! ```

use std::collections::HashSet;

use pixel_physics::render::{FieldOverlay, GrainMode, OrganismOverlay, Renderer};
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::world::World;
use pixel_physics::sim::rng;
use pixel_physics::sim::{explosion, material, parallel, update};

const WIDTH: i32 = 512;
const HEIGHT: i32 = 320;
const FLOOR_THICKNESS: i32 = 8;

/// Ground level for the plant scenes, in world rows from the top.
///
/// Chosen against `field.rs`'s measured light profile, not by eye: at the
/// current `LIGHT_DECAY` the reading crosses `Germinate`'s `0.1` threshold
/// roughly **75** cells below open sky, and `ambient_light_above` samples a
/// further `FIELD_SCALE` (8) rows *above* the cell it is asked about. `40`
/// leaves most of that band as headroom, so a seed here germinates on
/// light margin rather than on the edge of it -- and so a canopy growing
/// upward from here has somewhere lit to grow into. Do not deepen this
/// without re-reading `LIGHT_DECAY`'s own doc; a scene where nothing
/// germinates looks identical to a scene where growth is broken.
const TREE_GROUND_Y: i32 = 40;

/// Water with a varied `shade`, the way the brush lays it down
/// (`World::paint_capsule` rolls a random shade per cell). The scenes below
/// would otherwise use `Cell::new(WATER, 0)` and give every cell an
/// identical shade — which silently flattens `GrainMode::Cell` to no grain
/// at all, since that mode keys entirely off this byte. Worth knowing as a
/// real caveat of that mode and not just a harness detail: any water created
/// without a varied shade renders flat under it.
fn water_at(x: i32, y: i32) -> Cell {
    Cell::new(material::WATER, (rng::jitter(x, y) * 255.0) as u8)
}

fn stone_floor(w: &mut World) {
    for x in 0..WIDTH {
        for y in (HEIGHT - FLOOR_THICKNESS)..HEIGHT {
            w.set(x, y, Cell::new(material::STONE, 0));
        }
    }
}

/// The scenes the current bug list is about. Adding one is three lines, and
/// is much preferred to editing an existing one — a scene that quietly
/// changed underneath a recorded measurement is worse than no scene.
fn build(scene: &str) -> World {
    let mut w = World::new(Rect::new(0, 0, WIDTH - 1, HEIGHT - 1));
    let floor_y = HEIGHT - FLOOR_THICKNESS;
    match scene {
        // A large body released against the left wall, spreading right across
        // seven vertical chunk seams. The terracing/banding reproduction.
        "pour" => {
            stone_floor(&mut w);
            for x in 0..200 {
                for y in 30..floor_y {
                    w.set(x, y, water_at(x, y));
                }
            }
        }
        // Falling and spreading, rather than resting on the floor already:
        // the state the horizontal seam tearing shows up in.
        "fall" => {
            stone_floor(&mut w);
            for x in 20..250 {
                for y in 20..200 {
                    w.set(x, y, water_at(x, y));
                }
            }
        }
        // A dense blob dropped into a walled pool: the displacement striping.
        "blob" => {
            stone_floor(&mut w);
            for y in 0..floor_y {
                w.set(120, y, Cell::new(material::STONE, 0));
                w.set(392, y, Cell::new(material::STONE, 0));
            }
            for x in 121..392 {
                for y in 160..floor_y {
                    w.set(x, y, water_at(x, y));
                }
            }
            w.paint_circle(256, 80, 34, material::SAND);
        }
        // Sand blobs dropped on a floor: the original chunk-seam cliffs.
        "sand" => {
            stone_floor(&mut w);
            w.paint_circle(120, 70, 40, material::SAND);
            w.paint_circle(250, 70, 40, material::SAND);
            w.paint_circle(380, 70, 24, material::SAND);
        }
        // M15 explosions. A settled sand pile with a stone slab buried in
        // it and open air above -- the geometry the owner's own live report
        // used ("two explosions in a sand pile"), plus enough solid
        // material for the crater edge and the fireball ring to read
        // against something that cannot avalanche. `explode=` fires into it.
        "boom" => {
            stone_floor(&mut w);
            for x in 100..412 {
                for y in 180..floor_y {
                    w.set(x, y, Cell::new(material::SAND, (rng::jitter(x, y) * 255.0) as u8));
                }
            }
            for x in 150..360 {
                for y in 230..244 {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
        }
        // The same blast against `Solid` only, so nothing can slump in and
        // hide what the blast itself actually did.
        "boom_stone" => {
            stone_floor(&mut w);
            for x in 100..412 {
                for y in 180..floor_y {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
        }
        // A deep, flat sand bed with nothing else in it -- for firing several
        // blasts at different *depths* below the free surface in one image.
        // The owner's own framing of the M15 complaint: material only blasts
        // around when the charge is near the edge, and "it just doesn't
        // happen if you're not really close."
        "sandbed" => {
            stone_floor(&mut w);
            for x in 20..492 {
                for y in 120..floor_y {
                    w.set(x, y, Cell::new(material::SAND, (rng::jitter(x, y) * 255.0) as u8));
                }
            }
        }
        // The same depth sweep in the other material the complaint named.
        // A liquid holds no angle of repose, so a crater here closes by
        // flowing rather than by avalanching.
        "waterbed" => {
            stone_floor(&mut w);
            for x in 20..492 {
                for y in 120..floor_y {
                    w.set(x, y, water_at(x, y));
                }
            }
        }
        // M16 plants. A single seed on a stone shelf with a puddle beside
        // it -- deliberately the same geometry as the committed live
        // verification shots (`docs/screenshots/tree-rewrite-live-
        // verification/`), so a sheet from this scene can be compared
        // directly against the artifact the owner's "still a tiny tree, one
        // cell thick, ~18 cells, no leaves, no roots" report was made about
        // rather than against a fresh scene nobody has a prior on.
        //
        // **The shelf height is load-bearing, not cosmetic.** `Germinate`'s
        // light gate reads `field_at(x, y - FIELD_SCALE).light`, and
        // `field.rs`'s `LIGHT_DECAY` puts the `0.1` crossing roughly 75
        // world cells below open sky. The ordinary `stone_floor` at
        // `HEIGHT - 8` is 300+ cells down and a seed there never germinates
        // at all -- which is exactly how the ported tree tests started
        // failing when they kept the old system's y=100-150 planting depth
        // (see `PLAN.md`'s tree-rewrite step 7 entry). `TREE_GROUND_Y` sits
        // comfortably inside the lit band with headroom, per this repo's
        // "set bars from measurement with headroom" convention.
        "tree" => {
            for x in 0..WIDTH {
                for y in TREE_GROUND_Y..(TREE_GROUND_Y + 6) {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
            }
            w.plant_tree(200, TREE_GROUND_Y - 1);
            w.paint_circle(150, TREE_GROUND_Y - 4, 7, material::WATER);
        }
        // The same shelf, but a soil bed over a water table and several
        // seeds spaced across it. This is the scene the later phases are
        // aimed at -- roots growing into soil, soil moisture being drunk
        // down, canopies competing for light -- and today it should show
        // *none* of that, which is the point of shooting it now: it is the
        // before-picture for work that has not started.
        "forest" => {
            // `soil` has no `material::` constant of its own -- it was
            // appended to `EMBEDDED` deliberately (see that array's own
            // comment on why inserting rather than appending would
            // renumber the well-known ids), so it is looked up by name
            // like `wood` and `moss` already are elsewhere.
            let soil = w.materials.id_of("soil").expect("soil is a compiled-in material");
            for x in 0..WIDTH {
                for y in (TREE_GROUND_Y + 40)..(TREE_GROUND_Y + 46) {
                    w.set(x, y, Cell::new(material::STONE, 0));
                }
                // Soil all the way down to the stone.
                //
                // An earlier version buried a band of free `water` in the
                // lower soil as a stand-in water table. That does not work
                // and the reason is worth keeping: `soil` is a `Powder` and
                // sinks through `Liquid`, so within a few hundred frames the
                // soil had swapped places with the water and the "table" was
                // a film lying on the *surface* — with every seed then
                // germinating onto water rather than soil, and no root ever
                // starting. Correct physics, useless scene, and it read as a
                // root bug.
                //
                // A real water table needs moisture held *inside* soil
                // cells, which is Decision 3 (§4a, per-cell fill in a
                // `Powder`'s own `aux`). Until that lands the honest scene
                // is dry soil, and the puddle below is on the surface where
                // it will stay put.
                // Starts at field capacity -- damp, the way real ground
                // between rain events is, and the state a root system
                // actually lives in. `Powder` aux is moisture now
                // (`material::SOIL_SATURATED`, where 0 means *dry*, the
                // opposite of a liquid's fill), and a scene of bone-dry
                // soil would sit below the wilting point where `Absorb`
                // correctly credits nothing at all.
                for y in TREE_GROUND_Y..(TREE_GROUND_Y + 40) {
                    w.set(x, y, Cell::new(soil, (rng::jitter(x, y) * 255.0) as u8).with_aux(material::SOIL_FIELD_CAPACITY));
                }
            }
            w.paint_circle(260, TREE_GROUND_Y - 3, 6, material::WATER);
            for x in [80, 200, 320, 440] {
                w.plant_tree(x, TREE_GROUND_Y - 1);
            }
        }
        other => panic!(
            "unknown scene {other:?}; known: pour, fall, blob, sand, boom, boom_stone, sandbed, waterbed, tree, forest"
        ),
    }
    w
}

struct Args {
    scene: String,
    start: usize,
    every: usize,
    count: usize,
    cols: usize,
    zoom: i32,
    crop: Rect,
    parallel_driver: bool,
    out: String,
    grain: GrainMode,
    /// `channel=` -- render the sheet through one of `render.rs`'s debug
    /// overlays instead of ordinary material colour. The whole reason the
    /// plant work needs this harness: resource, canopy density and (later)
    /// vein conductance are per-cell scalars that decide plant shape and
    /// have **never been drawn**, which is how `tree-rewrite-design.md`
    /// §2b's self-avoidance mechanism shipped inert past two design
    /// reviews. A contact sheet in a channel shows both what the value is
    /// and how it evolves across the tiles, which is the question.
    organism_overlay: OrganismOverlay,
    field_overlay: FieldOverlay,
    /// Write an animated GIF of every frame in the range instead of a grid.
    /// The grid is for *me* to read; motion is for a human to watch, and
    /// some of these artifacts only read correctly in motion.
    gif: bool,
    /// `explode=x,y,radius,strength,frame` -- fire one `explosion::trigger`
    /// at the given frame. Repeatable, for several blasts in one run.
    explosions: Vec<(i32, i32, i32, f32, usize)>,
}

fn parse() -> Args {
    let mut a = Args {
        scene: "pour".into(),
        start: 100,
        every: 60,
        count: 6,
        cols: 3,
        zoom: 1,
        crop: Rect::new(0, 0, WIDTH - 1, HEIGHT - 1),
        parallel_driver: true,
        out: std::env::temp_dir().join("filmstrip.png").display().to_string(),
        grain: GrainMode::Position,
        organism_overlay: OrganismOverlay::Off,
        field_overlay: FieldOverlay::Off,
        gif: false,
        explosions: Vec::new(),
    };
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "scene" => a.scene = v.into(),
            "start" => a.start = v.parse().expect("start"),
            "every" => a.every = v.parse().expect("every"),
            "count" => a.count = v.parse().expect("count"),
            "cols" => a.cols = v.parse().expect("cols"),
            "zoom" => a.zoom = v.parse().expect("zoom"),
            "driver" => a.parallel_driver = v != "serial",
            "out" => a.out = v.into(),
            "gif" => a.gif = v != "false",
            "grain" => {
                a.grain = match v {
                    "position" => GrainMode::Position,
                    "cell" => GrainMode::Cell,
                    "muted" => GrainMode::Muted,
                    "animated" => GrainMode::Animated,
                    "motion" => GrainMode::Motion,
                    other => panic!("unknown grain {other:?}"),
                }
            }
            // One flag for both overlay families, resolved by name: they are
            // one question ("which channel am I looking at") from the
            // caller's side, and keeping them as two arguments would invite
            // setting both and getting a sheet that is hard to attribute.
            "channel" => match v {
                "off" => {}
                "celltype" => a.organism_overlay = OrganismOverlay::CellType,
                "resource" => a.organism_overlay = OrganismOverlay::Resource,
                "canopy" => a.organism_overlay = OrganismOverlay::CanopyDensity,
                "soil" => a.organism_overlay = OrganismOverlay::SoilMoisture,
                "light" => a.field_overlay = FieldOverlay::Light,
                "moisture" => a.field_overlay = FieldOverlay::Moisture,
                "temperature" => a.field_overlay = FieldOverlay::Temperature,
                "pressure" => a.field_overlay = FieldOverlay::Pressure,
                other => panic!(
                    "unknown channel {other:?}; known: off, celltype, resource, canopy, soil, light, moisture, temperature, pressure"
                ),
            },
            "explode" => {
                let n: Vec<f32> = v.split(',').map(|s| s.parse().expect("explode")).collect();
                assert_eq!(n.len(), 5, "explode=x,y,radius,strength,frame");
                a.explosions.push((n[0] as i32, n[1] as i32, n[2] as i32, n[3], n[4] as usize));
            }
            "crop" => {
                let n: Vec<i32> = v.split(',').map(|s| s.parse().expect("crop")).collect();
                assert_eq!(n.len(), 4, "crop=x,y,w,h");
                a.crop = Rect::new(n[0], n[1], n[0] + n[2] - 1, n[1] + n[3] - 1);
            }
            other => panic!("unknown argument {other:?}"),
        }
    }
    a
}

/// Fire every scheduled explosion whose frame has arrived, removing it from
/// the pending list so it cannot fire twice. Draining rather than
/// index-matching makes this safe to call both inside the stepping loop and
/// immediately before a capture, which is what lets `frame=0` work at all
/// (with `start=0` the loop body never runs before the first tile).
fn fire_due_explosions(
    world: &mut World,
    particles: &mut ParticleSystem,
    pending: &mut Vec<(i32, i32, i32, f32, usize)>,
    now: usize,
) {
    let mut i = 0;
    while i < pending.len() {
        if pending[i].4 <= now {
            let (x, y, r, strength, _) = pending.remove(i);
            println!("  boom: ({x}, {y}) r={r} strength={strength} at frame {now}");
            explosion::trigger(world, particles, x, y, r, strength);
        } else {
            i += 1;
        }
    }
}

/// One full frame, in `App::update`'s own phase order.
///
/// This harness originally ran the CA sweep and nothing else, which is fine
/// for liquids but makes an explosion unviewable: debris lives in
/// `ParticleSystem` and does not move without `particles.step`, and the
/// pressure/heat a blast writes never propagates without `step_fields`. The
/// added phases are no-ops or non-CA-affecting for the four pre-existing
/// scenes (nothing promotes a liquid body, nothing schedules an active site,
/// and no liquid rule reads the field), so they do not move the measurements
/// already recorded against those.
fn advance(world: &mut World, particles: &mut ParticleSystem, parallel_driver: bool) {
    if parallel_driver {
        parallel::step(world);
    } else {
        update::step(world);
    }
    world.step_liquid_bodies();
    world.step_active_sites();
    particles.step(world);
    world.step_fields();
}

fn main() {
    let args = parse();
    let mut world = build(&args.scene);
    let mut renderer = Renderer::new();
    renderer.grain = args.grain;
    renderer.organism_overlay = args.organism_overlay;
    renderer.field_overlay = args.field_overlay;
    let mut particles = ParticleSystem::new();
    let mut pending = args.explosions.clone();
    let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];

    let (cw, ch) = (args.crop.width(), args.crop.height());
    let (tile_w, tile_h) = (cw * args.zoom, ch * args.zoom);
    let gap = 2i32;
    let rows = args.count.div_ceil(args.cols) as i32;
    let sheet_w = args.cols as i32 * tile_w + (args.cols as i32 - 1) * gap;
    let sheet_h = rows * tile_h + (rows - 1) * gap;
    // Mid-grey gutters, so a tile that is legitimately all-black stays
    // distinguishable from the space between tiles.
    let mut sheet = vec![90u8; (sheet_w * sheet_h * 4) as usize];
    for p in sheet.chunks_exact_mut(4) {
        p[3] = 255;
    }

    // GIF branch: motion is for a human to watch, and several of these
    // artifacts (a fringe that regenerates every frame, water that reads as
    // frozen because its grain is nailed to the screen) simply do not survive
    // being sampled into stills. Consecutive frames, real playback speed, and
    // a NETSCAPE loop -- the same reasoning `main.rs`'s capture hook records.
    if args.gif {
        let mut frames = Vec::with_capacity(args.count);
        let mut step_no = 0usize;
        for i in 0..args.count {
            let target = args.start + i * args.every;
            while step_no < target {
                fire_due_explosions(&mut world, &mut particles, &mut pending, step_no);
                advance(&mut world, &mut particles, args.parallel_driver);
                step_no += 1;
            }
            fire_due_explosions(&mut world, &mut particles, &mut pending, step_no);
            let touched: HashSet<_> = world.take_touched_chunks();
            renderer.draw(&world, &particles, &touched, &mut frame, (WIDTH as u32, HEIGHT as u32), true);

            let mut tile = vec![0u8; (tile_w * tile_h * 4) as usize];
            for y in 0..ch {
                for x in 0..cw {
                    let (sx, sy) = (args.crop.min_x + x, args.crop.min_y + y);
                    if sx < 0 || sy < 0 || sx >= WIDTH || sy >= HEIGHT {
                        continue;
                    }
                    let src = (((sy * WIDTH) + sx) * 4) as usize;
                    for zy in 0..args.zoom {
                        for zx in 0..args.zoom {
                            let (dx, dy) = (x * args.zoom + zx, y * args.zoom + zy);
                            let dst = (((dy * tile_w) + dx) * 4) as usize;
                            tile[dst..dst + 4].copy_from_slice(&frame[src..src + 4]);
                        }
                    }
                }
            }
            frames.push(tile);
        }

        // 60 ticks/second is the sandbox's own fixed simulation rate, so
        // `every` ticks between captures maps directly to elapsed time.
        let delay_ms = (args.every as u64 * 1000) / 60;
        let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(delay_ms.max(16)));
        let file = std::fs::File::create(&args.out).expect("creating the gif");
        let mut encoder = image::codecs::gif::GifEncoder::new(file);
        encoder.set_repeat(image::codecs::gif::Repeat::Infinite).expect("gif repeat");
        for tile in frames {
            let buf = image::RgbaImage::from_raw(tile_w as u32, tile_h as u32, tile).expect("gif frame");
            encoder.encode_frame(image::Frame::from_parts(buf, 0, 0, delay)).expect("gif frame");
        }
        drop(encoder);
        println!("animated gif ({tile_w}x{tile_h}, {} frames): {}", args.count, args.out);
        return;
    }

    let mut captured = 0usize;
    let mut step_no = 0usize;
    while captured < args.count {
        let target = args.start + captured * args.every;
        while step_no < target {
            fire_due_explosions(&mut world, &mut particles, &mut pending, step_no);
            advance(&mut world, &mut particles, args.parallel_driver);
            step_no += 1;
        }
        fire_due_explosions(&mut world, &mut particles, &mut pending, step_no);
        // `force_full`, not the dirty-rect path: this must draw the whole
        // world every time regardless of what moved, or a tile would inherit
        // pixels from whichever frame last touched them.
        let touched: HashSet<_> = world.take_touched_chunks();
        renderer.draw(&world, &particles, &touched, &mut frame, (WIDTH as u32, HEIGHT as u32), true);

        let (gx, gy) = (captured as i32 % args.cols as i32, captured as i32 / args.cols as i32);
        let (ox, oy) = (gx * (tile_w + gap), gy * (tile_h + gap));
        for y in 0..ch {
            for x in 0..cw {
                let (sx, sy) = (args.crop.min_x + x, args.crop.min_y + y);
                if sx < 0 || sy < 0 || sx >= WIDTH || sy >= HEIGHT {
                    continue;
                }
                let src = (((sy * WIDTH) + sx) * 4) as usize;
                for zy in 0..args.zoom {
                    for zx in 0..args.zoom {
                        let (dx, dy) = (ox + x * args.zoom + zx, oy + y * args.zoom + zy);
                        let dst = (((dy * sheet_w) + dx) * 4) as usize;
                        sheet[dst..dst + 4].copy_from_slice(&frame[src..src + 4]);
                    }
                }
            }
        }
        println!("  tile {captured}: frame {target}, awake {}/{}, particles {}", world.active_chunk_count(), world.chunk_count(), particles.len());
        captured += 1;
    }

    image::save_buffer(&args.out, &sheet, sheet_w as u32, sheet_h as u32, image::ColorType::Rgba8)
        .expect("writing the contact sheet");
    println!("contact sheet ({sheet_w}x{sheet_h}, {} tiles): {}", args.count, args.out);
}
