//! **What should zooming *in* look like?** — the candidate magnifiers side
//! by side, at one zoom, on one scene, with what each costs per pixel.
//!
//! `Renderer::zoom` is a nearest-neighbour magnify: at 8x every world cell is
//! a flat 8x8 block of one colour, and the 64 pixels a cell owns are spent
//! saying one thing. The owner asked (2026-09-12) whether zooming in could
//! "look better, smoothing or upsample the resolution, or brainstorm better
//! options". Those are judge-by-eye questions and `CLAUDE.md` says to render
//! them rather than argue them, so this draws one crop of one world through
//! every candidate and lays them out on a sheet:
//!
//! | arm | what it is |
//! |---|---|
//! | `nearest` | today: the shipped 1:1 render, each pixel replicated `zoom`x |
//! | `smooth` | the owner's first candidate: bilinear interpolation between cell centres of the shipped colours |
//! | `contour` | hard-edged corner chamfer chosen from the cell's *class* neighbourhood (air / liquid / powder / solid / plant / creature), never from colour — the staircase on a diagonal becomes a 45° edge, the grain inside a mass is untouched |
//! | `drawn` | `nearest`, plus per-cell state *drawn* at sub-cell resolution instead of encoded in brightness: a liquid cell's fill is a level line inside its block (the bottom `fill` of the block in the undimmed colour, the rest in the colour of the air above) |
//! | `both` | `contour` + `drawn`, the combination the report recommends |
//! | `texture` | `nearest`, plus a per-*pixel* brightness grain keyed on world position, the "texture that only appears past some zoom" idea |
//!
//! **Everything here is a pure function of the cell, its 3x3 neighbourhood,
//! and the pixel's offset inside its block**, so every arm keeps the
//! dirty-rect render skip's identity argument — with one caveat the report
//! prices: `contour` reads neighbours, and a neighbour across a chunk border
//! can change without dirtying this chunk. `FOAM_BLEND`'s own doc in
//! `render.rs` is the precedent for why the shipped path avoids that.
//!
//! **The colours are the shipped renderer's.** Every arm starts from the 1:1
//! frame `Renderer::draw` produced and only decides *which* of those colours
//! a sub-pixel gets (`drawn` inverts the fill dimming to recover the undimmed
//! colour, and that is the one place a colour not on the 1:1 frame can
//! appear). No arm can invent a palette the engine would not have drawn,
//! which is what makes an A/B off this sheet admissible — the same rule
//! `subpixel` and `terrain_shade` follow.
//!
//! **The number beside each tile is what the picture cannot say.** `ns/px`
//! is the arm's own reconstruction cost per output pixel, best-of-`reps`,
//! single-threaded, on this box — an upper bound on what the same rule adds
//! to `cell_colour`, which already pays the cell fetch the arm here repeats.
//! At 8x the viewport is still 512x320 pixels, so a per-pixel cost does not
//! grow with zoom; a per-*cell* cost shrinks 64x. And `partial liquid cells`
//! / `chamfered corners` say whether `drawn` and `contour` had anything to
//! do in the crop at all: a sheet where neither fired looks exactly like a
//! sheet where the rule is dead.
//!
//! ```text
//! cargo run --release --example zoomin -- zoom=8 out=/tmp/zoomin.png
//! cargo run --release --example zoomin -- zoom=4 look=200,150 arms=nearest,contour,both
//! ```
//!
//! The scene is `common::PlantScene` (soil, stone floor, a stand of trees)
//! with a stone basin carved into the soil surface and filled with water,
//! then settled, so one crop holds air, plant, soil, stone and liquid with a
//! part-full surface row. `look=x,y` is the crop's top-left in world cells;
//! the default aims at the basin and the tree beside it.

use pixel_physics::hud;
use pixel_physics::render::Renderer;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::material::{self, MaterialKind};
use pixel_physics::sim::parallel;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::rng;
use pixel_physics::sim::world::World;
use std::time::Instant;

mod common;

const W: i32 = 512;
const H: i32 = 320;
/// The basin: a stone bowl let into the soil surface, filled to the brim
/// and one row over, so after settling the top row is part-full.
const BASIN_X0: i32 = 236;
const BASIN_X1: i32 = 276;
const BASIN_DEPTH: i32 = 7;
const GAP: u32 = 6;
const LABEL: u32 = 10;

/// What a cell *is*, for the contour's purposes. Colour never enters it —
/// that is the whole difference from hqx/xBR, which `dead-ends.md` records
/// as backwards here because the deliberate per-cell grain defeats any
/// colour-matching filter.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Class {
    Air,
    Liquid,
    Powder,
    Solid,
    Plant,
    Creature,
}

fn class_of(world: &World, cell: Cell) -> Class {
    if cell.material == material::EMPTY {
        return Class::Air;
    }
    match world.materials.get(cell.material).kind {
        MaterialKind::Empty | MaterialKind::Gas => Class::Air,
        MaterialKind::Liquid => Class::Liquid,
        MaterialKind::Powder => Class::Powder,
        MaterialKind::Solid => Class::Solid,
        MaterialKind::Plant => Class::Plant,
        MaterialKind::Creature => Class::Creature,
    }
}

struct Args {
    zoom: i32,
    frames: u64,
    settle: u64,
    look: Option<(i32, i32)>,
    /// Crop size in cells; default is one full viewport at `zoom`.
    span: Option<(i32, i32)>,
    arms: Vec<String>,
    cols: usize,
    reps: usize,
    /// `texture`'s amplitude, as a fraction of brightness.
    grain: f32,
    /// Pin the sky to this daylight fraction (`sky::frame_for_daylight`),
    /// so a sheet is not a sample of whatever hour frame `frames` lands on.
    daylight: Option<f32>,
    /// What `contour` does at a concave notch — an air cell whose left and
    /// up neighbours are both a mass. `fill` gives the notch to the mass
    /// whenever the two neighbours agree (the 45° staircase repair, and it
    /// also fires on every one-cell hole in a canopy, which speckles);
    /// `deep` fills only when the diagonal cell agrees too, so a true
    /// inside corner is rounded and a one-cell notch or hole is left alone;
    /// `cut` never fills, only cuts convex corners.
    notch: String,
    /// Also write every arm's tile as its own file beside `out`
    /// (`<stem>_<arm>.png`), which is what a gallery card wants.
    each: bool,
    out: String,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            zoom: 8,
            frames: 6000,
            settle: 400,
            look: None,
            span: None,
            arms: ["nearest", "smooth", "contour", "drawn", "both", "texture"].iter().map(|s| s.to_string()).collect(),
            cols: 3,
            reps: 5,
            grain: 0.08,
            daylight: None,
            notch: "fill".to_string(),
            each: false,
            out: "zoomin.png".to_string(),
        }
    }
}

/// The 1:1 frame plus everything an arm may ask about a cell.
struct Source<'a> {
    world: &'a World,
    frame: Vec<u8>,
    class: Vec<Class>,
}

impl Source<'_> {
    fn colour(&self, x: i32, y: i32) -> [u8; 4] {
        if x < 0 || y < 0 || x >= W || y >= H {
            return [0, 0, 0, 255];
        }
        let i = ((y * W + x) * 4) as usize;
        [self.frame[i], self.frame[i + 1], self.frame[i + 2], 255]
    }
    fn class(&self, x: i32, y: i32) -> Class {
        if x < 0 || y < 0 || x >= W || y >= H {
            return Class::Solid;
        }
        self.class[(y * W + x) as usize]
    }
}

/// One output tile: `cw*z` by `ch*z` RGBA pixels of the crop at `(cx, cy)`.
fn render_arm(src: &Source, arm: &str, crop: (i32, i32, i32, i32), z: i32, grain: f32, notch: &str) -> (Vec<u8>, usize) {
    let (cx, cy, cw, ch) = crop;
    let (tw, th) = (cw * z, ch * z);
    let mut out = vec![0u8; (tw * th * 4) as usize];
    let has = |f: &str| arm == f || arm.split('+').any(|p| p == f) || (f != "nearest" && arm == "both" && matches!(f, "contour" | "drawn"));
    let contour = has("contour");
    let drawn = has("drawn");
    let smooth = has("smooth");
    let texture = has("texture");
    let stamp = has("stamp");
    let sub_grain = (z / 4).max(1);
    let mut fired = 0usize;
    for ty in 0..th {
        for tx in 0..tw {
            let (x, y) = (cx + tx / z, cy + ty / z);
            let (sx, sy) = (tx % z, ty % z);
            let mut c = src.colour(x, y);
            if smooth {
                // Bilinear between cell centres: the sub-pixel's position in
                // cell units, minus the half-cell so a block centre samples
                // its own cell exactly.
                let fx = (tx as f32 + 0.5) / z as f32 - 0.5 + cx as f32;
                let fy = (ty as f32 + 0.5) / z as f32 - 0.5 + cy as f32;
                let (x0, y0) = (fx.floor() as i32, fy.floor() as i32);
                let (u, v) = (fx - x0 as f32, fy - y0 as f32);
                let p = [src.colour(x0, y0), src.colour(x0 + 1, y0), src.colour(x0, y0 + 1), src.colour(x0 + 1, y0 + 1)];
                for k in 0..3 {
                    let top = p[0][k] as f32 * (1.0 - u) + p[1][k] as f32 * u;
                    let bot = p[2][k] as f32 * (1.0 - u) + p[3][k] as f32 * u;
                    c[k] = (top * (1.0 - v) + bot * v).round() as u8;
                }
            }
            if drawn && src.class(x, y) == Class::Liquid && src.class(x, y - 1) != Class::Liquid {
                let cell = src.world.get(x, y);
                let mat = src.world.materials.get(cell.material);
                let fill = (pixel_physics::sim::update::liquid_fill(cell) as f32 / material::LIQUID_FULL as f32).clamp(0.0, 1.0);
                if fill < 1.0 {
                    // Undo `cell_colour`'s fill dimming to recover the colour a
                    // full cell would have drawn; the level line then says
                    // how much water is here, instead of the brightness.
                    let d = mat.fill_dimming.clamp(0.0, 1.0);
                    let strength = (1.0 - d) + d * fill;
                    let level = (fill * z as f32).round() as i32; // rows of liquid, from the bottom
                    if sy >= z - level {
                        for k in c.iter_mut().take(3) {
                            *k = ((*k as f32 / strength.max(0.05)).round()).min(255.0) as u8;
                        }
                    } else {
                        c = src.colour(x, y - 1);
                    }
                    if sx == 0 && sy == 0 {
                        fired += 1;
                    }
                }
            }
            if stamp {
                let me = src.class(x, y);
                let cell = src.world.get(x, y);
                match me {
                    // A mass at 8x is drawn as the material at 1x was: a
                    // field of sub-grains, each picking its own palette entry
                    // by a world-keyed hash, scaled by the shade the 1:1
                    // render already put on this cell so the lighting stays.
                    Class::Powder | Class::Solid => {
                        let pal = &src.world.materials.get(cell.material).palette;
                        let (gx, gy) = (x * z + sx - (sx % sub_grain), y * z + sy - (sy % sub_grain));
                        let k = (rng::jitter(gx, gy) * pal.len() as f32) as usize % pal.len();
                        let own = pal[cell.shade as usize % pal.len()];
                        let lum = |p: [u8; 4]| 0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32;
                        let scale = lum(pal[k]) / lum(own).max(1.0);
                        for q in c.iter_mut().take(3) {
                            *q = (*q as f32 * scale).round().clamp(0.0, 255.0) as u8;
                        }
                    }
                    Class::Plant => {
                        let leaf = pixel_physics::sim::organism::cell_type(cell.aux())
                            == Some(pixel_physics::sim::organism::CellType::Leaf);
                        if leaf {
                            // A leaf is a lobe: any corner facing air on both
                            // sides is given to the air, so a lone leaf cell
                            // is a rounded blob and a run of them a scalloped
                            // edge, never a row of squares.
                            for (dx, dy) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
                                if src.class(x + dx, y) == Class::Air && src.class(x, y + dy) == Class::Air {
                                    let ox = if dx < 0 { sx } else { z - 1 - sx };
                                    let oy = if dy < 0 { sy } else { z - 1 - sy };
                                    if ox + oy < z / 2 {
                                        c = src.colour(x + dx, y);
                                    }
                                }
                            }
                        } else {
                            // Wood: vertical bark strips, one sub-grain wide,
                            // keyed on the column only so a strip runs the
                            // length of a trunk rather than breaking per cell.
                            let pal = &src.world.materials.get(cell.material).palette;
                            let gx = x * z + sx - (sx % sub_grain);
                            let k = (rng::jitter(gx, 7) * pal.len() as f32) as usize % pal.len();
                            let own = pal[cell.shade as usize % pal.len()];
                            let lum = |p: [u8; 4]| 0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32;
                            let scale = 1.0 + 0.5 * (lum(pal[k]) / lum(own).max(1.0) - 1.0);
                            for q in c.iter_mut().take(3) {
                                *q = (*q as f32 * scale).round().clamp(0.0, 255.0) as u8;
                            }
                        }
                    }
                    _ => {}
                }
            }
            if contour {
                let me = src.class(x, y);
                // Each corner: if the two orthogonal neighbours across it
                // share a class that is not mine, that corner triangle is
                // theirs. Convex corners of a mass are cut; the concave
                // notch of a staircase is filled, because from the air
                // cell's side the same rule fires with the mass's colour.
                let corners = [(-1, -1), (1, -1), (-1, 1), (1, 1)];
                for (dx, dy) in corners {
                    let a = src.class(x + dx, y); // horizontal neighbour
                    let b = src.class(x, y + dy); // vertical neighbour
                    if a != me && a == b {
                        // A notch is the case where *I* am the lesser class
                        // (air, or a liquid under a mass) and the corner
                        // would be filled with the mass -- see `notch`.
                        let filling = me == Class::Air || (me == Class::Liquid && a != Class::Air);
                        let d = src.class(x + dx, y + dy);
                        let allowed = match notch {
                            "cut" => !filling,
                            "deep" => !filling || d == a,
                            _ => true,
                        };
                        if !allowed {
                            continue;
                        }
                        // Offset of this pixel from that corner, 0-based.
                        let ox = if dx < 0 { sx } else { z - 1 - sx };
                        let oy = if dy < 0 { sy } else { z - 1 - sy };
                        if ox + oy < z / 2 {
                            c = if ox < oy { src.colour(x + dx, y) } else { src.colour(x, y + dy) };
                            if ox == 0 && oy == 0 {
                                fired += 1;
                            }
                        }
                    }
                }
            }
            if texture && src.class(x, y) != Class::Air {
                let j = rng::jitter(x * z + sx, y * z + sy) - 0.5;
                let s = 1.0 + j * 2.0 * grain;
                for k in c.iter_mut().take(3) {
                    *k = (*k as f32 * s).round().clamp(0.0, 255.0) as u8;
                }
            }
            let i = ((ty * tw + tx) * 4) as usize;
            out[i..i + 4].copy_from_slice(&c);
        }
    }
    (out, fired)
}

fn main() {
    let mut a = Args::default();
    for arg in std::env::args().skip(1) {
        let (k, v) = arg.split_once('=').unwrap_or((arg.as_str(), ""));
        match k {
            "zoom" => a.zoom = v.parse().expect("zoom"),
            "frames" => a.frames = v.parse().expect("frames"),
            "settle" => a.settle = v.parse().expect("settle"),
            "cols" => a.cols = v.parse().expect("cols"),
            "reps" => a.reps = v.parse().expect("reps"),
            "grain" => a.grain = v.parse().expect("grain"),
            "daylight" => a.daylight = Some(v.parse().expect("daylight")),
            "notch" => a.notch = v.to_string(),
            "each" => a.each = v != "false" && v != "0",
            "arms" => a.arms = v.split(',').map(|s| s.to_string()).collect(),
            "look" => {
                let n: Vec<i32> = v.split(',').map(|s| s.parse().expect("look")).collect();
                a.look = Some((n[0], n[1]));
            }
            "span" => {
                let n: Vec<i32> = v.split(',').map(|s| s.parse().expect("span")).collect();
                a.span = Some((n[0], n[1]));
            }
            "out" => a.out = v.to_string(),
            other => panic!("unknown arg {other:?}"),
        }
    }
    assert!((1..=16).contains(&a.zoom), "zoom wants 1..=16");
    println!(
        "zoomin: zoom={} frames={} settle={} look={:?} arms={:?} cols={} reps={} grain={} daylight={:?} notch={} out={}",
        a.zoom, a.frames, a.settle, a.look, a.arms, a.cols, a.reps, a.grain, a.daylight, a.notch, a.out
    );

    let scene = common::PlantScene::default();
    let ground_y = scene.ground_y;
    let mut world = scene.build();
    let mut particles = ParticleSystem::default();
    let step = |world: &mut World, particles: &mut ParticleSystem| {
        parallel::step(world);
        world.step_liquid_bodies();
        world.step_active_sites();
        particles.step(world);
        world.step_fields();
    };
    for _ in 0..a.frames {
        step(&mut world, &mut particles);
    }
    // The basin: stone walls and floor let into the soil, attached like
    // terrain so the structural check leaves them standing, then water to
    // the brim plus one part-full row so the surface has a level to show.
    let stone = |x: i32, y: i32| Cell::new(material::STONE, (rng::jitter(x, y) * 255.0) as u8).with_attached(true);
    for y in (ground_y - 1)..=(ground_y + BASIN_DEPTH) {
        for x in BASIN_X0..=BASIN_X1 {
            let wall = x == BASIN_X0 || x == BASIN_X1 || y == ground_y + BASIN_DEPTH;
            if wall {
                world.set(x, y, stone(x, y));
            } else if y >= ground_y {
                world.set(x, y, Cell::new(material::WATER, (rng::jitter(x, y) * 255.0) as u8));
            } else {
                world.set(x, y, Cell::new(material::WATER, (rng::jitter(x, y) * 255.0) as u8).with_aux(450));
            }
        }
    }
    for _ in 0..a.settle {
        step(&mut world, &mut particles);
    }

    let mut r = Renderer::new();
    r.pinned_light = a.daylight.map(pixel_physics::sky::frame_for_daylight);
    let mut frame = vec![0u8; (W * H * 4) as usize];
    let touched = pixel_physics::sim::fxhash::ChunkSet::default();
    let t = Instant::now();
    r.draw(&world, &particles, &touched, &mut frame, (W as u32, H as u32), true);
    let draw_ms = t.elapsed().as_secs_f64() * 1e3;
    let mut class = vec![Class::Air; (W * H) as usize];
    for y in 0..H {
        for x in 0..W {
            class[(y * W + x) as usize] = class_of(&world, world.get(x, y));
        }
    }
    let src = Source { world: &world, frame, class };

    let (cw, ch) = a.span.unwrap_or((W / a.zoom, H / a.zoom));
    let (cx, cy) = a.look.unwrap_or((BASIN_X0 - cw * 2 / 3, ground_y - ch * 3 / 4));
    let cx = cx.clamp(0, W - cw);
    let cy = cy.clamp(0, H - ch);
    let crop = (cx, cy, cw, ch);
    let mut partial = 0usize;
    let mut liquid = 0usize;
    let mut plant = 0usize;
    for y in cy..cy + ch {
        for x in cx..cx + cw {
            match src.class(x, y) {
                Class::Liquid => {
                    liquid += 1;
                    if pixel_physics::sim::update::liquid_fill(world.get(x, y)) < material::LIQUID_FULL {
                        partial += 1;
                    }
                }
                Class::Plant => plant += 1,
                _ => {}
            }
        }
    }
    let mut partial_rows: Vec<i32> = Vec::new();
    for y in cy..cy + ch {
        let n = (cx..cx + cw)
            .filter(|&x| src.class(x, y) == Class::Liquid && pixel_physics::sim::update::liquid_fill(world.get(x, y)) < material::LIQUID_FULL)
            .count();
        if n > 0 {
            partial_rows.push(y);
        }
    }
    println!("  part-full liquid rows in crop: {partial_rows:?}");
    println!(
        "  crop {cx},{cy} {cw}x{ch} cells -> {}x{} px | liquid cells {liquid} ({partial} part-full) | plant cells {plant} | 1:1 draw {draw_ms:.2} ms",
        cw * a.zoom,
        ch * a.zoom
    );

    let (tw, th) = ((cw * a.zoom) as u32, (ch * a.zoom) as u32);
    let mut tiles: Vec<(String, Vec<u8>)> = Vec::new();
    for arm in &a.arms {
        let mut best = f64::MAX;
        let mut tile = Vec::new();
        let mut fired = 0;
        for _ in 0..a.reps.max(1) {
            let t = Instant::now();
            let (out, n) = render_arm(&src, arm, crop, a.zoom, a.grain, &a.notch);
            best = best.min(t.elapsed().as_secs_f64() * 1e3);
            tile = out;
            fired = n;
        }
        let ns_px = best * 1e6 / (tw as f64 * th as f64);
        // The "did it fire" counter, named for what it counts: corners the
        // chamfer cut, level lines the fill drew, or both summed when both
        // are on. An arm with neither prints nothing, so a blank here is
        // "nothing to count" and a 0 is "the rule never fired".
        let has = |f: &str| arm.split('+').any(|p| p == f) || (arm == "both" && matches!(f, "contour" | "drawn"));
        let what = match (has("contour"), has("drawn")) {
            (true, true) => format!("corners+levels {fired}"),
            (true, false) => format!("chamfered corners {fired}"),
            (false, true) => format!("level lines {fired}"),
            (false, false) => String::new(),
        };
        let label = format!("{} {}x  {ns_px:.0} ns/px  {what}", arm.to_uppercase(), a.zoom);
        println!("  {arm:8} best {best:7.2} ms  {ns_px:6.1} ns/px  {what}");
        if a.each {
            let stem = a.out.strip_suffix(".png").unwrap_or(&a.out);
            let path = format!("{stem}_{}.png", arm.replace('+', "-"));
            image::save_buffer(&path, &tile, tw, th, image::ColorType::Rgba8).expect("write tile");
            println!("  wrote {path} ({tw}x{th})");
        }
        tiles.push((label, tile));
    }

    let cols = a.cols.clamp(1, tiles.len().max(1)) as u32;
    let rows = tiles.len().div_ceil(cols as usize) as u32;
    let sheet_w = cols * tw + (cols + 1) * GAP;
    let sheet_h = rows * (th + LABEL) + (rows + 1) * GAP;
    let mut sheet = vec![0u8; (sheet_w * sheet_h * 4) as usize];
    for px in sheet.chunks_exact_mut(4) {
        px.copy_from_slice(&[16, 16, 20, 255]);
    }
    for (i, (label, tile)) in tiles.iter().enumerate() {
        let (col, row) = (i as u32 % cols, i as u32 / cols);
        let ox = GAP + col * (tw + GAP);
        let oy = GAP + row * (th + LABEL + GAP);
        hud::draw_text(&mut sheet, sheet_w, sheet_h, ox as i32, oy as i32 + 1, label, [220, 220, 230, 255]);
        for y in 0..th {
            let src_i = (y * tw * 4) as usize;
            let dst_i = (((oy + LABEL + y) * sheet_w + ox) * 4) as usize;
            sheet[dst_i..dst_i + (tw * 4) as usize].copy_from_slice(&tile[src_i..src_i + (tw * 4) as usize]);
        }
    }
    image::save_buffer(&a.out, &sheet, sheet_w, sheet_h, image::ColorType::Rgba8).expect("write sheet");
    println!("wrote {} ({sheet_w}x{sheet_h})", a.out);
}
