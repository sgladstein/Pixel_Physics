//! **Does terrain get more visually interesting if you light it?** — an A/B
//! testbed for shading the ground, over the shipped world at play scale.
//!
//! Built for the visual-interest lane, 2026-08-29, against a specific
//! hypothesis: *"adding real shading to terrain is the single cheapest large
//! gain in visual interest available to us, and it is not a worldgen change
//! at all."* That is an eyes-only claim, so this exists to put it in front of
//! the owner as a picture rather than as an argument.
//!
//! ```text
//! cargo run --release --example terrain_shade -- preset=canyon seed=1 arms=base,full
//! cargo run --release --example terrain_shade -- arms=base,ao,sun,ink zoom=2 crop=0,100,256,180
//! cargo run --release --example terrain_shade -- vault=1   # the cave case
//! ```
//!
//! **Two prior verdicts bound what this may claim, and both are about
//! shading**, so read them before reading a sheet out of here:
//!
//! - `render.rs`'s `TerrainLight::Off` — a *depth grade* (dim a solid cell by
//!   its distance below the skyline) was built, blind-A/B'd and playtested,
//!   and the owner's answer was *"no question grade off is better"*. That is
//!   a global vertical gradient with **no local geometry in it at all**: a
//!   crevice, an overhang and an open hillside at the same depth are graded
//!   identically. It is not this.
//! - `Reports/subpixel-rendering-2026-08-29.md` §9 — the sub-cell `ao`/`shade`
//!   terms on *plants* came back *"the edges between color or material look
//!   weird, kinda 3d-ish. Could it be more flat or cartoony"*. That is a
//!   verdict on smooth rounded volume, on thin structures. `ink` here is the
//!   direction that verdict pointed **toward**, and it is the arm to take
//!   seriously first.
//!
//! ## What it does, and what it may not do
//!
//! Every arm takes its colours from the shipped `Renderer` at 1:1 — this
//! renders the frame the engine would have drawn and then multiplies a
//! per-pixel scalar over it. **No arm can invent a colour the engine would not
//! have drawn**, which is `subpixel.rs`'s rule and is what makes an A/B off
//! this admissible as evidence about the shipped look.
//!
//! The occupancy the shading reads comes from `World` directly, not from the
//! image: a mask of "is this cell opaque ground", sampled over the viewport
//! plus a margin so the kernel is not truncated at the frame edge. Reading it
//! back out of the rendered RGB was the first design and is wrong — deep air
//! and night rock are both near-black, so the mask would have holes exactly
//! where the caves are, which is the half of the picture this is most for.
//!
//! **This is an instrument, not a renderer.** It does the work per pixel in
//! `f32` over a whole frame with no dirty-rect skip and no chunk-local access,
//! so its own cost says nothing about what the technique would cost shipped.
//! `render.rs`'s `cell_colour` carries the real number for the AO family
//! (~10 ms on the 512x320 stress scene, from four `World::get` HashMap
//! lookups per pixel) and names the fix (chunk-direct access, as `ChunkView`
//! already did for the sweep). Quote that, not this.

use pixel_physics::app::{HEIGHT, WIDTH, WORLD_HEIGHT, WORLD_WIDTH};
use pixel_physics::render::Renderer;
use pixel_physics::sim::material;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::world::World;

/// Kernel margin in cells: how far outside the viewport the occupancy mask is
/// sampled, so a pixel on the frame edge sees the same neighbourhood an
/// interior pixel does. Must exceed the largest radius any arm uses.
const MARGIN: i32 = 24;

struct Args {
    seed: u32,
    preset: String,
    /// World clock frame to render at. **Pinned by default** — a luminance
    /// number, or a judgement about colour, sampled at an arbitrary hour is a
    /// statement about the hour. 600 matches `viewshot`'s own default so the
    /// two harnesses' sheets are comparable.
    frame: usize,
    settle: usize,
    /// World column to centre on. Defaults to the middle of the world.
    at: Option<i32>,
    /// World row to centre on. Defaults to the skyline at `at`.
    y: Option<i32>,
    /// Aim at the deepest cave air instead of at the skyline.
    vault: bool,
    arms: String,
    /// Occlusion radius in cells.
    ao_r: f32,
    /// How hard occlusion darkens: 1.0 means a fully-enclosed cell goes to
    /// `ao_floor` of its palette colour.
    ao: f32,
    ao_floor: f32,
    /// Directional term strength, and the direction the sun is in (screen
    /// space, +x right, +y down — so a sun up and to the left is `-1,-1`).
    sun: f32,
    sun_dir: (f32, f32),
    /// Radius the surface normal is estimated over, in cells. Larger reads
    /// the *hillside*; smaller reads the individual boulder.
    sun_r: f32,
    /// Cartoon ink: how dark a one-to-two-cell shell just inside the ground's
    /// silhouette is drawn, in `0..1` of the fill colour.
    ink: f32,
    ink_r: f32,
    zoom: usize,
    crop: Option<(usize, usize, usize, usize)>,
    view: (u32, u32),
    gutter: usize,
    labels: bool,
    out: String,
}

fn main() {
    let mut a = Args {
        seed: 1,
        preset: String::new(),
        frame: 600,
        settle: 60,
        at: None,
        y: None,
        vault: false,
        arms: "base,full".into(),
        ao_r: 6.0,
        ao: 0.55,
        ao_floor: 0.45,
        sun: 0.35,
        sun_dir: (-0.55, -1.0),
        sun_r: 9.0,
        ink: 0.35,
        ink_r: 1.6,
        zoom: 1,
        crop: None,
        view: (WIDTH, HEIGHT),
        gutter: 6,
        labels: true,
        out: "target/laneD/terrain_shade.png".into(),
    };
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "seed" => a.seed = v.parse().expect("seed=N"),
            "preset" => a.preset = v.to_string(),
            "frame" => a.frame = v.parse().expect("frame=N"),
            "settle" => a.settle = v.parse().expect("settle=N"),
            "at" => a.at = Some(v.parse().expect("at=WORLD_X")),
            "y" => a.y = Some(v.parse().expect("y=WORLD_Y")),
            "vault" => a.vault = v != "0",
            "arms" => a.arms = v.to_string(),
            "ao_r" => a.ao_r = v.parse().expect("ao_r"),
            "ao" => a.ao = v.parse().expect("ao"),
            "ao_floor" => a.ao_floor = v.parse().expect("ao_floor"),
            "sun" => a.sun = v.parse().expect("sun"),
            "sun_r" => a.sun_r = v.parse().expect("sun_r"),
            "sun_dir" => {
                let (x, y) = v.split_once(',').expect("sun_dir=X,Y");
                a.sun_dir = (x.parse().expect("sun_dir=X,Y"), y.parse().expect("sun_dir=X,Y"));
            }
            "ink" => a.ink = v.parse().expect("ink"),
            "ink_r" => a.ink_r = v.parse().expect("ink_r"),
            "zoom" => a.zoom = v.parse::<usize>().expect("zoom=K").max(1),
            "crop" => {
                let n: Vec<usize> = v.split(',').map(|t| t.parse().expect("crop=x,y,w,h")).collect();
                assert_eq!(n.len(), 4, "crop=x,y,w,h");
                a.crop = Some((n[0], n[1], n[2], n[3]));
            }
            "view" => {
                let (w, h) = v.split_once('x').expect("view=WxH");
                a.view = (w.parse().expect("view=WxH"), h.parse().expect("view=WxH"));
            }
            "gutter" => a.gutter = v.parse().expect("gutter=N"),
            "labels" => a.labels = v != "0",
            "out" => a.out = v.to_string(),
            _ => {}
        }
    }

    // Echo the parameters. A harness whose knobs are invisible is a harness
    // nobody can tell is disconnected — `plant_probe`'s 3.5-hour byte-identical
    // megastudy is the standing reason this line exists.
    println!(
        "terrain_shade: seed={} preset={} frame={} arms={} | ao={} r={} floor={} | sun={} dir={:?} r={} | ink={} r={}",
        a.seed,
        if a.preset.is_empty() { "<default>" } else { &a.preset },
        a.frame,
        a.arms,
        a.ao,
        a.ao_r,
        a.ao_floor,
        a.sun,
        a.sun_dir,
        a.sun_r,
        a.ink,
        a.ink_r
    );

    let (presets, err) = pixel_physics::worldgen::WorldgenPresets::load();
    if let Some(e) = err {
        panic!("{e}");
    }
    let name = if a.preset.is_empty() { presets.default_name() } else { a.preset.clone() };
    let Some(params) = presets.get(&name) else { panic!("unknown preset {name:?}") };

    let (vw, vh) = (a.view.0 as usize, a.view.1 as usize);
    let bounds = pixel_physics::sim::chunk::Rect::new(0, 0, WORLD_WIDTH as i32 - 1, WORLD_HEIGHT as i32 - 1);
    let mut world = World::new(bounds);
    pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed: a.seed as u64 });
    // `parallel::step` + active sites + fields, matching `App::update`'s own
    // phase order — `viewshot`'s settle loop learned the hard way that
    // omitting `step_fields` dries every lake in the world over a long run,
    // so a picture taken without it is a picture of a harness bug.
    let target_frame = a.frame.max(a.settle) as u64;
    while world.frame < target_frame {
        pixel_physics::sim::parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
    }
    println!("  world {WORLD_WIDTH}x{WORLD_HEIGHT} ({name}, seed {}) at frame {}", a.seed, world.frame);

    // Where to aim. The skyline at `at` normally; the tallest column of deep
    // air under `vault`, which is `viewshot`'s own rule and the only way to
    // frame a cave — a chamber sits 200+ rows down and every skyline-framed
    // view misses it by construction.
    let world_w = WORLD_WIDTH as i32;
    let (target_x, target_y) = if a.vault {
        find_deep_air(&world, world_w, WORLD_HEIGHT as i32)
    } else {
        let x = a.at.unwrap_or(world_w / 2);
        let y = a.y.unwrap_or_else(|| surface_at(&world, x, WORLD_HEIGHT as i32));
        (x, y)
    };

    let mut renderer = Renderer::new();
    renderer.set_camera(target_x - vw as i32 / 2, target_y - vh as i32 / 2, a.view, world.bounds());
    let (cam_x, cam_y) = (renderer.camera_x, renderer.camera_y);
    println!("  camera ({cam_x}, {cam_y}), showing world x {cam_x}..{}, y {cam_y}..{}", cam_x + vw as i32, cam_y + vh as i32);

    let particles = ParticleSystem::new();
    let touched = world.take_touched_chunks();
    let mut base = vec![0u8; vw * vh * 4];
    renderer.draw(&world, &particles, &touched, &mut base, a.view, true);

    // ---- occupancy, straight from the world -------------------------------
    // "Opaque ground": anything that is not empty and not a gas. A liquid
    // counts — a flooded chamber occludes the rock behind it and reads as a
    // filled volume, which is what an occlusion term should say about it.
    let mw = vw + 2 * MARGIN as usize;
    let mh = vh + 2 * MARGIN as usize;
    let mut occ = vec![0f32; mw * mh];
    let mut solid_cells = 0usize;
    for j in 0..mh {
        for i in 0..mw {
            let wx = cam_x + i as i32 - MARGIN;
            let wy = cam_y + j as i32 - MARGIN;
            let c = world.get(wx, wy);
            let k = world.materials.kind(c.material);
            let opaque = c.material != material::EMPTY && k != material::MaterialKind::Gas;
            if opaque {
                occ[j * mw + i] = 1.0;
                if (MARGIN..mw as i32 - MARGIN).contains(&(i as i32)) && (MARGIN..mh as i32 - MARGIN).contains(&(j as i32)) {
                    solid_cells += 1;
                }
            }
        }
    }
    println!(
        "  occupancy: {solid_cells} of {} viewport cells are opaque ground ({:.1}%)",
        vw * vh,
        100.0 * solid_cells as f64 / (vw * vh) as f64
    );

    // **The ceiling on anything a lighting model can buy**, and the reason
    // this harness prints a number at all rather than only a picture.
    // A shading term needs a *surface*: an occlusion reading is constant
    // wherever the neighbourhood is entirely ground, and a normal derived
    // from a coverage field is identically zero there. So the fraction of
    // ground that lies within one kernel of air bounds what any amount of
    // light can reach — measured off the true occupancy rather than off the
    // rendered image, because deep air and night rock are both near-black
    // and an image-derived mask loses exactly the caves.
    let er = 6i32;
    let (mut ground, mut near_edge) = (0usize, 0usize);
    for j in MARGIN..(mh as i32 - MARGIN) {
        for i in MARGIN..(mw as i32 - MARGIN) {
            if occ[j as usize * mw + i as usize] < 0.5 {
                continue;
            }
            ground += 1;
            let mut open = false;
            'scan: for dy in -er..=er {
                for dx in -er..=er {
                    if occ[(j + dy) as usize * mw + (i + dx) as usize] < 0.5 {
                        open = true;
                        break 'scan;
                    }
                }
            }
            if open {
                near_edge += 1;
            }
        }
    }
    println!(
        "  boundary: {near_edge} of {ground} ground cells lie within {er} cells of air ({:.1}%) \
         -- the ceiling on what any shading term can reach",
        100.0 * near_edge as f64 / ground.max(1) as f64
    );

    // Three blurs of the same mask at three radii: the occlusion reading, the
    // hillside normal, and the tight shell the ink line rides. Separate
    // radii because they are separate questions — `subpixel.rs` §5a's finding
    // that shape and colour must not share a kernel, one level up.
    let cov_ao = blur(&occ, mw, mh, a.ao_r);
    let cov_sun = blur(&occ, mw, mh, a.sun_r);
    let cov_ink = blur(&occ, mw, mh, a.ink_r);

    let arms: Vec<&str> = a.arms.split(',').filter(|s| !s.is_empty()).collect();
    let (cx0, cy0, cw, ch) = a.crop.unwrap_or((0, 0, vw, vh));
    assert!(cx0 + cw <= vw && cy0 + ch <= vh, "crop is outside the {vw}x{vh} viewport");
    let (tw, th) = (cw * a.zoom, ch * a.zoom);
    let label_h = if a.labels { 10 } else { 0 };
    let sheet_w = tw * arms.len() + a.gutter * arms.len().saturating_sub(1);
    let sheet_h = th + label_h;
    let mut sheet = vec![0u8; sheet_w * sheet_h * 4];
    sheet.fill(0xff);

    for (n, arm) in arms.iter().enumerate() {
        let (ao, sun, ink) = match *arm {
            "base" => (0.0, 0.0, 0.0),
            "ao" => (a.ao, 0.0, 0.0),
            "sun" => (0.0, a.sun, 0.0),
            "ink" => (0.0, 0.0, a.ink),
            "full" => (a.ao, a.sun, a.ink),
            "aosun" => (a.ao, a.sun, 0.0),
            "flat" => (a.ao, 0.0, a.ink),
            other => panic!("unknown arm {other:?} (base|ao|sun|ink|aosun|flat|full)"),
        };

        // The discrete "did it fire" number, per arm: how many viewport pixels
        // this arm actually moved, and by how much. A sheet whose arms look
        // alike and whose counts are zero is a disconnected knob, not a null
        // result — that distinction has cost this repo whole sessions.
        let mut moved = 0usize;
        let mut total_delta = 0f64;
        let mut shaded = vec![0u8; vw * vh * 4];
        let (sx, sy) = normalise(a.sun_dir);

        for y in 0..vh {
            for x in 0..vw {
                let p = (y * vw + x) * 4;
                let mi = (y + MARGIN as usize) * mw + (x + MARGIN as usize);
                let here = occ[mi];
                let mut f = 1.0f32;

                // Everything below applies to ground only. Sky, and anything
                // drawn in the air, is left exactly as the engine drew it —
                // the sky gradient is the best-looking thing in the frame and
                // nothing here has any business touching it.
                if here > 0.5 {
                    if ao > 0.0 {
                        // Occlusion: how much of the neighbourhood is also
                        // ground. A cell deep in the massif is surrounded and
                        // goes to the floor; a cell on a cave wall, an
                        // overhang lip or the skyline keeps its colour.
                        //
                        // This is the term the rejected `TerrainLight::Depth`
                        // does *not* have. Depth is `y - skyline[x]`, so an
                        // overhang and open hillside at one depth grade the
                        // same. This reads the neighbourhood, so the cave
                        // mouth is bright and the rock two cells behind it is
                        // not, at the same depth.
                        let occl = cov_ao[mi].clamp(0.0, 1.0);
                        f *= 1.0 - ao * (1.0 - a.ao_floor) * occl;
                    }
                    if sun > 0.0 {
                        // The gradient of the coverage field points *into* the
                        // mass, so its negation is the outward surface normal.
                        // **Deliberately not normalised** — deep inside a mass
                        // the neighbouring kernels cancel and the gradient is
                        // lattice residue, and dividing that by its own tiny
                        // length turns noise into a full-strength unit normal.
                        // That bug quilted an interior into squares in
                        // `subpixel.rs` and is recorded in its report §5d; the
                        // raw gradient is already the right shape, large where
                        // the surface genuinely faces somewhere and vanishing
                        // where it does not.
                        let gx = cov_sun[mi + 1] - cov_sun[mi - 1];
                        let gy = cov_sun[mi + mw] - cov_sun[mi - mw];
                        let lambert = -(gx * sx + gy * sy);
                        f *= 1.0 + sun * lambert * 2.0;
                    }
                    if ink > 0.0 {
                        // A dark shell just inside the silhouette. `cov_ink`
                        // is an occupancy fraction over a tight kernel, so the
                        // shell is the band immediately below "fully
                        // surrounded" — no edge detection and no second pass.
                        // This is the *flat* reading of form: the shape is
                        // carried by a drawn line rather than by a gradient,
                        // which is the direction the plant A/B was rejected
                        // toward (`subpixel-rendering-2026-08-29.md` §9).
                        let edge = (1.0 - cov_ink[mi]).clamp(0.0, 1.0);
                        f *= 1.0 - ink * (edge / 0.5).min(1.0);
                    }
                }

                let f = f.clamp(0.0, 2.0);
                for c in 0..3 {
                    let v = (base[p + c] as f32 * f).round().clamp(0.0, 255.0) as u8;
                    if v != base[p + c] {
                        moved += 1;
                        total_delta += (v as f64 - base[p + c] as f64).abs();
                    }
                    shaded[p + c] = v;
                }
                shaded[p + 3] = 0xff;
            }
        }
        println!(
            "  arm {arm:>6}: {} of {} subpixels moved ({:.1}%), mean |delta| over moved {:.1}/255",
            moved,
            vw * vh * 3,
            100.0 * moved as f64 / (vw * vh * 3) as f64,
            if moved > 0 { total_delta / moved as f64 } else { 0.0 }
        );

        let ox = n * (tw + a.gutter);
        for y in 0..th {
            for x in 0..tw {
                let src = ((cy0 + y / a.zoom) * vw + (cx0 + x / a.zoom)) * 4;
                let dst = ((label_h + y) * sheet_w + ox + x) * 4;
                sheet[dst..dst + 4].copy_from_slice(&shaded[src..src + 4]);
            }
        }
        if a.labels {
            // A one-cell tick mark per arm index, so a sheet read on a phone
            // still says which tile is which without a caption.
            for k in 0..=n {
                for y in 3..8 {
                    for x in 0..4 {
                        let dst = (y * sheet_w + ox + 3 + k * 6 + x) * 4;
                        sheet[dst] = 0x20;
                        sheet[dst + 1] = 0x20;
                        sheet[dst + 2] = 0x20;
                    }
                }
            }
        }
    }

    if let Some(dir) = std::path::Path::new(&a.out).parent() {
        std::fs::create_dir_all(dir).expect("create out dir");
    }
    write_png(&a.out, &sheet, sheet_w, sheet_h);
    println!("  sheet ({sheet_w}x{sheet_h}, arms {}): {}", a.arms, a.out);
}

fn normalise((x, y): (f32, f32)) -> (f32, f32) {
    let l = (x * x + y * y).sqrt().max(1e-6);
    (x / l, y / l)
}

/// Separable box blur, run twice — two boxes approximate a Gaussian closely
/// enough for a shading kernel and cost two passes instead of a quadratic one.
fn blur(src: &[f32], w: usize, h: usize, radius: f32) -> Vec<f32> {
    let mut a = src.to_vec();
    let r = radius.max(0.5);
    for _ in 0..2 {
        a = blur_pass(&a, w, h, (r / 1.5).max(0.5));
    }
    a
}

fn blur_pass(src: &[f32], w: usize, h: usize, radius: f32) -> Vec<f32> {
    let r = radius.round().max(1.0) as usize;
    let k = (2 * r + 1) as f32;
    let mut tmp = vec![0f32; w * h];
    for y in 0..h {
        let row = y * w;
        for x in 0..w {
            let mut s = 0.0;
            for d in 0..=2 * r {
                let sx = (x + d).saturating_sub(r).min(w - 1);
                s += src[row + sx];
            }
            tmp[row + x] = s / k;
        }
    }
    let mut out = vec![0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut s = 0.0;
            for d in 0..=2 * r {
                let sy = (y + d).saturating_sub(r).min(h - 1);
                s += tmp[sy * w + x];
            }
            out[y * w + x] = s / k;
        }
    }
    out
}

fn surface_at(world: &World, x: i32, world_h: i32) -> i32 {
    for y in 0..world_h {
        if world.get(x, y).material != material::EMPTY {
            return y;
        }
    }
    world_h / 2
}

/// The tallest run of deep air anywhere in the world — `viewshot`'s vault
/// rule. Found rather than passed in: a chamber's position is a noise draw
/// and a hardcoded coordinate goes stale the moment anything upstream of the
/// vault pass changes.
fn find_deep_air(world: &World, world_w: i32, world_h: i32) -> (i32, i32) {
    let (mut best, mut best_at) = (0, (world_w / 2, world_h / 2));
    for x in (0..world_w).step_by(8) {
        let (mut run, mut start) = (0, 0);
        for y in world_h / 4..world_h {
            if world.get(x, y).material == material::EMPTY {
                if run == 0 {
                    start = y;
                }
                run += 1;
                if run > best {
                    best = run;
                    best_at = (x, start + run / 2);
                }
            } else {
                run = 0;
            }
        }
    }
    println!("  vault: tallest deep-air column is {best} rows at {best_at:?}");
    best_at
}

/// Minimal PNG writer — one IDAT, stored (uncompressed) deflate blocks. The
/// repo's other sheet-writers do the same; pulling an encoder in for a debug
/// harness is not worth the dependency.
fn write_png(path: &str, rgba: &[u8], w: usize, h: usize) {
    fn crc32(data: &[u8]) -> u32 {
        let mut table = [0u32; 256];
        for (i, e) in table.iter_mut().enumerate() {
            let mut c = i as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
            *e = c;
        }
        let mut c = 0xFFFF_FFFFu32;
        for &b in data {
            c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
        }
        c ^ 0xFFFF_FFFF
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(body);
        let mut crc_in = kind.to_vec();
        crc_in.extend_from_slice(body);
        out.extend_from_slice(&crc32(&crc_in).to_be_bytes());
    }
    let mut raw = Vec::with_capacity((w * 4 + 1) * h);
    for y in 0..h {
        raw.push(0);
        raw.extend_from_slice(&rgba[y * w * 4..(y + 1) * w * 4]);
    }
    let mut z = vec![0x78, 0x01];
    for (i, block) in raw.chunks(65535).enumerate() {
        let last = if (i + 1) * 65535 >= raw.len() { 1 } else { 0 };
        z.push(last);
        z.extend_from_slice(&(block.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        z.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in &raw {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut png, b"IHDR", &ihdr);
    chunk(&mut png, b"IDAT", &z);
    chunk(&mut png, b"IEND", &[]);
    std::fs::write(path, png).expect("write png");
}
