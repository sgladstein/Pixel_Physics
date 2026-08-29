//! **Three genuinely different ideas about why plants look wrong, rendered as
//! growth timelapses so they can be judged in motion.**
//!
//! `subpixel.rs` asked one question — *what if a cell were not a square* — and
//! got the same answer from the owner three times: rounded, then flat, then
//! flat and ragged, all "not clearly better" than what ships. `CLAUDE.md`'s
//! rule for that is explicit: **two fixes failing the same way means the
//! approach is wrong, not the tuning.** So this stops tuning that one and
//! tries three approaches that differ in *what they change*, not in how hard
//! they change it:
//!
//! | arm | the shape primitive is | what it is betting on |
//! |---|---|---|
//! | `shipped` | one cell, one square | the control |
//! | `masses` | **a whole crown region** | a tree's silhouette is a few big foliage masses, and drawing hundreds of little ones is why it reads as speckle |
//! | `stamps` | **an authored leaf-clump sprite** | the sim should say *where* foliage is and art should say *what it looks like* -- which is how 2D games actually draw trees |
//! | `tone` | **nothing — the cells are untouched** | the problem is not shape at all. It is that wood and leaf sit at the same value, every tree is lit identically, and a stand has no depth |
//!
//! `tone` is the one worth reading carefully, because it is the cheapest and
//! it is the hypothesis nobody has tested. It changes **no pixel's shape** --
//! every cell is still exactly its own square, drawn at 1:1 and magnified like
//! the control. If the stand reads better under it, then four rounds of
//! silhouette work were aimed at the wrong quantity.
//!
//! # Why a timelapse
//!
//! Asked for directly, 2026-08-29: *"I need to see a timelapse of it
//! growing."* A still cannot answer it, and the review skill says the same --
//! a grid of stills cannot show whether something *moves* right, and a frame
//! sequence has twice got a diagnosis where stills got a rejection.
//!
//! Every arm shares one simulation, so at any scrubber position the four panes
//! are the same stand at the same instant and differ only in how it is drawn.
//!
//! ```text
//! cargo run --release --example plantlook -- start=600 every=800 count=8 out=/tmp/look
//! ```

use pixel_physics::render::Renderer;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::material::{self, MaterialKind};
use pixel_physics::sim::organism::{self, CellType};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::parallel;

mod common;

const W: i32 = 512;
const H: i32 = 320;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Empty,
    Foliage,
    Wood,
    Other,
}

/// How coarse a "crown region" is for `masses`, in world cells.
///
/// The whole bet of that arm is that the shape primitive is too small, so this
/// is the one number that says how much too small. Six cells is about a third
/// of a crown's width on this stand: big enough that a tree resolves into a
/// handful of masses rather than a hundred, small enough that two neighbouring
/// trees do not fuse into one.
const MASS_BUCKET: i32 = 6;

fn main() {
    let mut start = 600u64;
    let mut every = 800u64;
    let mut count = 8usize;
    let mut scale = 3i32;
    let mut crop = (0i32, 60i32, 300i32, 180i32);
    let mut out = "/tmp/plantlook".to_string();
    for arg in std::env::args().skip(1) {
        let (k, v) = arg.split_once('=').unwrap_or((arg.as_str(), ""));
        match k {
            "start" => start = v.parse().expect("start"),
            "every" => every = v.parse().expect("every"),
            "count" => count = v.parse().expect("count"),
            "scale" => scale = v.parse().expect("scale"),
            "out" => out = v.to_string(),
            "crop" => {
                let n: Vec<i32> = v.split(',').map(|s| s.parse().expect("crop")).collect();
                crop = (n[0], n[1], n[2], n[3]);
            }
            other => panic!("unknown arg {other:?}"),
        }
    }
    std::fs::create_dir_all(&out).expect("out dir");
    println!("plantlook: start={start} every={every} count={count} scale={scale} crop={crop:?} out={out}");

    for i in 0..count {
        let frame = start + every * i as u64;
        // **A fresh world per capture, re-simulated from zero.** `World` is not
        // `Clone`, and the background pass below has to empty every plant cell
        // to see what is behind them -- doing that to a world that then keeps
        // stepping risks unscheduling the very organisms whose growth is the
        // subject. Re-simulating is O(sum of frames), which is seconds here,
        // and it cannot be wrong.
        let shot = capture(frame);
        for arm in ["shipped", "masses", "stamps", "tone"] {
            let img = draw(arm, &shot, scale, crop);
            let (_, _, cw, ch) = crop;
            let path = format!("{out}/{arm}_{i:02}.png");
            image::save_buffer(&path, &img, (cw * scale) as u32, (ch * scale) as u32, image::ColorType::Rgba8)
                .expect("write png");
        }
        println!(
            "  frame {frame}: {} foliage + {} wood = {} plant cells",
            shot.foliage,
            shot.wood,
            shot.foliage + shot.wood
        );
    }
    println!("wrote {count} frames x 4 arms to {out}");
}

/// One instant of the stand, in the form every arm draws from: the shipped
/// 1:1 render, the same render with every plant cell emptied (what is
/// *behind* the plants), and the per-cell class and owner.
///
/// Every arm reads the same `Shot`, so at any scrubber position the panes are
/// the same stand at the same instant and can differ only in the drawing.
struct Shot {
    plant: Vec<u8>,
    back: Vec<u8>,
    class: Vec<Class>,
    owner: Vec<u16>,
    foliage: usize,
    wood: usize,
}

/// Simulate to `frame` and take a [`Shot`].
fn capture(frame: u64) -> Shot {
    let base = common::PlantScene::default();
    let mut world = common::PlantScene { ..base }.build();
    let mut particles = ParticleSystem::default();
    for _ in 0..frame {
        parallel::step(&mut world);
        world.step_liquid_bodies();
        world.step_active_sites();
        particles.step(&mut world);
        // Not optional: `step_fields` propagates the light channel and
        // `Germinate` reads it. Without it nothing germinates and the harness
        // renders bare soil while reporting success.
        world.step_fields();
    }

    let mut r = Renderer::new();
    r.pinned_light = Some(pixel_physics::sky::frame_for_daylight(1.0));
    let mut plant_frame = vec![0u8; (W * H * 4) as usize];
    let touched = std::collections::HashSet::new();
    r.draw(&world, &particles, &touched, &mut plant_frame, (W as u32, H as u32), true);

    let mut class = vec![Class::Empty; (W * H) as usize];
    // **Which plant each cell belongs to.** `tone` needs real per-plant
    // identity for its depth bit, and the first attempt derived it by walking
    // left along connected cells -- which only follows a horizontal *run*, a
    // few cells wide inside a crown, so the bit flipped several times within
    // one tree and the stand came out striped mid-canopy rather than layered.
    // The engine already knows the answer.
    let mut owner = vec![0u16; (W * H) as usize];
    let (mut foliage, mut wood) = (0usize, 0usize);
    for y in 0..H {
        for x in 0..W {
            let cell = world.get(x, y);
            owner[(y * W + x) as usize] = cell.organism_id();
            class[(y * W + x) as usize] = if cell.material == material::EMPTY {
                Class::Empty
            } else if world.materials.get(cell.material).kind == MaterialKind::Plant {
                match organism::cell_type(cell.aux()) {
                    Some(CellType::Leaf) => {
                        foliage += 1;
                        Class::Foliage
                    }
                    _ => {
                        wood += 1;
                        Class::Wood
                    }
                }
            } else {
                Class::Other
            };
        }
    }

    for y in 0..H {
        for x in 0..W {
            if matches!(class[(y * W + x) as usize], Class::Foliage | Class::Wood) {
                world.set(x, y, Cell::EMPTY);
            }
        }
    }
    let mut rb = Renderer::new();
    rb.pinned_light = Some(pixel_physics::sky::frame_for_daylight(1.0));
    let mut back_frame = vec![0u8; (W * H * 4) as usize];
    rb.draw(&world, &particles, &touched, &mut back_frame, (W as u32, H as u32), true);

    Shot { plant: plant_frame, back: back_frame, class, owner, foliage, wood }
}

fn px(buf: &[u8], x: i32, y: i32) -> [f32; 3] {
    let x = x.clamp(0, W - 1);
    let y = y.clamp(0, H - 1);
    let i = ((y * W + x) * 4) as usize;
    [buf[i] as f32, buf[i + 1] as f32, buf[i + 2] as f32]
}

fn hash2(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x27d4_eb2d) ^ (y as u32).wrapping_mul(0x1656_67b1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    h
}

/// Dispatch one arm over the cropped region at `scale` pixels per cell.
fn draw(arm: &str, shot: &Shot, s: i32, (cx, cy, cw, ch): (i32, i32, i32, i32)) -> Vec<u8> {
    let Shot { plant, class, owner, .. } = shot;
    let (ow, oh) = (cw * s, ch * s);
    let mut out = vec![255u8; (ow * oh * 4) as usize];
    // Precomputed once per frame, not per pixel.
    let masses = (arm == "masses").then(|| build_masses(plant, class));
    let tone = (arm == "tone").then(|| build_tone(plant, class, owner));

    for oy in 0..oh {
        for ox in 0..ow {
            let fx = cx as f32 + (ox as f32 + 0.5) / s as f32;
            let fy = cy as f32 + (oy as f32 + 0.5) / s as f32;
            let (bx, by) = (fx.floor() as i32, fy.floor() as i32);
            let rgb = match arm {
                // The control: exactly what the window shows today, magnified
                // by the same factor as everything else so the comparison is
                // over the same screen area.
                "shipped" => px(plant, bx, by),
                "tone" => tone.as_ref().unwrap()[(by.clamp(0, H - 1) * W + bx.clamp(0, W - 1)) as usize],
                "masses" => draw_masses(masses.as_ref().unwrap(), shot, (fx, fy), (bx, by)),
                "stamps" => draw_stamps(shot, (fx, fy), (bx, by)),
                other => panic!("unknown arm {other:?}"),
            };
            let i = ((oy * ow + ox) * 4) as usize;
            for c in 0..3 {
                out[i + c] = rgb[c].round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    out
}

// ---------------------------------------------------------------- masses ---

/// One crown region: where it is, how much foliage is in it, what colour.
struct Mass {
    x: f32,
    y: f32,
    r: f32,
    rgb: [f32; 3],
}

/// **Bet: the shape primitive is too small.** A tree's silhouette is a few
/// overlapping foliage masses -- that is how an illustrator draws one, and it
/// is what the eye reads at play scale. Drawing four hundred individual leaf
/// cells, however smoothly, produces speckle no matter how good each dot is.
///
/// So foliage is bucketed into `MASS_BUCKET`-wide regions and each bucket
/// becomes **one blob** whose area is proportional to the foliage in it
/// (radius as the square root, since area is what a count buys) and whose
/// colour is the mean of the cells it swallowed.
fn build_masses(plant: &[u8], class: &[Class]) -> Vec<Mass> {
    let (bw, bh) = (W / MASS_BUCKET + 1, H / MASS_BUCKET + 1);
    let mut acc = vec![(0.0f32, 0.0f32, 0usize, [0.0f32; 3]); (bw * bh) as usize];
    for y in 0..H {
        for x in 0..W {
            if class[(y * W + x) as usize] != Class::Foliage {
                continue;
            }
            let b = ((y / MASS_BUCKET) * bw + x / MASS_BUCKET) as usize;
            let c = px(plant, x, y);
            acc[b].0 += x as f32 + 0.5;
            acc[b].1 += y as f32 + 0.5;
            acc[b].2 += 1;
            for (sum, v) in acc[b].3.iter_mut().zip(c) {
                *sum += v;
            }
        }
    }
    acc.into_iter()
        .filter(|a| a.2 > 0)
        .map(|(sx, sy, n, srgb)| {
            let n_f = n as f32;
            Mass {
                x: sx / n_f,
                y: sy / n_f,
                // **Area proportional to the count**, which fixes the radius
                // rather than leaving it to taste: a blob standing in for `n`
                // cells should cover about `n` cells, so `r = sqrt(n/pi)`.
                // The first draft used `1.5 * sqrt(n)` -- 4.4x too wide at a
                // full bucket -- and fused the whole stand into one green
                // hedge with no tree in it. `1.15` lets neighbouring masses
                // just touch instead of leaving gaps between them.
                r: 1.15 * (n_f / std::f32::consts::PI).sqrt(),
                rgb: [srgb[0] / n_f, srgb[1] / n_f, srgb[2] / n_f],
            }
        })
        .collect()
}

fn draw_masses(masses: &[Mass], shot: &Shot, (fx, fy): (f32, f32), (bx, by): (i32, i32)) -> [f32; 3] {
    let (plant, back, class) = (&shot.plant, &shot.back, &shot.class);
    let mut rgb = px(back, bx, by);
    // Wood first and per-cell, so the trunk still reads as a branching
    // structure -- the bet is about foliage, and changing both at once would
    // make the arm untestable.
    if class[(by.clamp(0, H - 1) * W + bx.clamp(0, W - 1)) as usize] == Class::Wood {
        rgb = px(plant, bx, by);
    }
    // Crowns over the top. Nearest mass wins rather than a blend, so each
    // reads as its own body of foliage with a defined edge.
    let mut best: Option<(f32, &Mass)> = None;
    for m in masses {
        let d = ((fx - m.x).powi(2) + (fy - m.y).powi(2)).sqrt();
        if d < m.r && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, m));
        }
    }
    if let Some((d, m)) = best {
        // A ragged rim rather than a circle: the same lesson the last round
        // paid for -- a constant threshold on a radial field can only cut
        // arcs, and arcs read as soap bubbles.
        let wobble = (hash2((fx * 3.0) as i32, (fy * 3.0) as i32) & 0xff) as f32 / 255.0;
        let edge = m.r * (0.82 + 0.18 * wobble);
        if d < edge {
            // Darker toward the rim: a drawn edge, not a lit surface.
            let ink = if d > edge * 0.78 { 0.72 } else { 1.0 };
            rgb = [m.rgb[0] * ink, m.rgb[1] * ink, m.rgb[2] * ink];
        }
    }
    rgb
}

// ---------------------------------------------------------------- stamps ---

/// Four hand-drawn 7x7 leaf clumps, as bitmasks.
///
/// **Bet: the simulation should say *where* foliage is and art should say what
/// it looks like.** Every approach so far has tried to *derive* the shape of a
/// leaf from the cell grid, which is the one thing the cell grid does not
/// know. This is what 2D games actually do, it costs no simulation state, and
/// the stamps are the only authored pixels anywhere in this world.
const LEAF_STAMPS: [[u8; 7]; 4] = [
    [0b0011100, 0b0111110, 0b1111111, 0b1111111, 0b0111110, 0b0011100, 0b0001000],
    [0b0001000, 0b0011100, 0b0111110, 0b1111110, 0b1111100, 0b0111000, 0b0010000],
    [0b0111000, 0b1111100, 0b1111110, 0b0111111, 0b0011110, 0b0001100, 0b0000100],
    [0b0010000, 0b0111100, 0b1111110, 0b1111111, 0b1111110, 0b0111100, 0b0011000],
];

fn draw_stamps(shot: &Shot, (fx, fy): (f32, f32), (bx, by): (i32, i32)) -> [f32; 3] {
    let (plant, back, class) = (&shot.plant, &shot.back, &shot.class);
    let mut rgb = px(back, bx, by);
    if class[(by.clamp(0, H - 1) * W + bx.clamp(0, W - 1)) as usize] == Class::Wood {
        rgb = px(plant, bx, by);
    }
    // A stamp is 7x7 stamp-pixels covering `spread` world cells, so it hangs
    // outside the cell that placed it -- which is the point: one leaf cell
    // paints a clump, not a square.
    let spread = 2.2f32;
    let reach = spread.ceil() as i32;
    let mut painted: Option<[f32; 3]> = None;
    for ny in (by - reach)..=(by + reach) {
        for nx in (bx - reach)..=(bx + reach) {
            if nx < 0 || ny < 0 || nx >= W || ny >= H || class[(ny * W + nx) as usize] != Class::Foliage {
                continue;
            }
            let h = hash2(nx, ny);
            let stamp = &LEAF_STAMPS[(h % LEAF_STAMPS.len() as u32) as usize];
            // Jitter the placement so a run of leaf cells does not lay its
            // clumps on a visible lattice.
            let jx = ((h >> 8) & 0xff) as f32 / 255.0 - 0.5;
            let jy = ((h >> 16) & 0xff) as f32 / 255.0 - 0.5;
            let u = (fx - (nx as f32 + 0.5 + jx * 0.6)) / spread + 0.5;
            let v = (fy - (ny as f32 + 0.5 + jy * 0.6)) / spread + 0.5;
            if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
                continue;
            }
            let (sx, sy) = ((u * 7.0) as usize, (v * 7.0) as usize);
            if stamp[sy.min(6)] & (0b1000000 >> sx.min(6)) == 0 {
                continue;
            }
            let c = px(plant, nx, ny);
            // Nearest-to-front: a later cell overpaints, so clumps overlap
            // like real foliage rather than averaging into a wash.
            painted = Some(c);
        }
    }
    if let Some(c) = painted {
        rgb = c;
    }
    rgb
}

// ------------------------------------------------------------------ tone ---

/// **Bet: the problem is not shape at all.**
///
/// This arm changes **no pixel's shape**. Every cell is still exactly its own
/// square, drawn at 1:1 and magnified like the control, so if it reads better
/// then four rounds of silhouette work were aimed at the wrong quantity.
///
/// Three things move, all of them value rather than form:
///
/// - **Depth.** Each plant gets a near/far bit from a hash of its own trunk
///   column, and far plants are lifted toward the sky and flattened in
///   contrast -- atmospheric perspective. `TreeDepth::Weave` already assigns
///   exactly such a per-tree bit for the gnome to walk through and **nothing
///   else reads it**; a stand that reads as one flat hedge is the complaint
///   this answers.
/// - **Canopy shading at crown scale, not cell scale.** Foliage density is
///   measured over a coarse window, so the interior of a crown darkens and its
///   lit upper edge stays bright. Per-cell shading was what read as "3d-ish";
///   this is the same idea moved up to the scale the eye actually groups at,
///   where it reads as a canopy having a shaded side rather than as every leaf
///   being a little ball.
/// - **Value separation.** Wood and leaf sit at nearly the same lightness
///   here, which is what turns a crown into mud at play scale. Wood goes down,
///   lit foliage goes up.
fn build_tone(plant: &[u8], class: &[Class], owner: &[u16]) -> Vec<[f32; 3]> {
    // Coarse foliage density, and the height of the canopy top per column.
    const WIN: i32 = 5;
    let mut density = vec![0f32; (W * H) as usize];
    let mut top = vec![i32::MAX; W as usize];
    for y in 0..H {
        for x in 0..W {
            if class[(y * W + x) as usize] == Class::Foliage && y < top[x as usize] {
                top[x as usize] = y;
            }
        }
    }
    for y in 0..H {
        for x in 0..W {
            let mut n = 0;
            for dy in -WIN..=WIN {
                for dx in -WIN..=WIN {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx >= 0
                        && ny >= 0
                        && nx < W
                        && ny < H
                        && class[(ny * W + nx) as usize] == Class::Foliage
                    {
                        n += 1;
                    }
                }
            }
            let area = ((2 * WIN + 1) * (2 * WIN + 1)) as f32;
            density[(y * W + x) as usize] = n as f32 / area;
        }
    }

    let mut out = vec![[0f32; 3]; (W * H) as usize];
    for y in 0..H {
        for x in 0..W {
            let i = (y * W + x) as usize;
            let mut c = px(plant, x, y);
            match class[i] {
                Class::Foliage | Class::Wood => {
                    // Depth: one stable bit per *plant*, from the id the
                    // engine already keeps. Anything derived from local
                    // geometry instead flips inside a single crown -- see
                    // `owner`'s own note.
                    let far = hash2(owner[i] as i32, 7) & 1 == 0;

                    if class[i] == Class::Wood {
                        // Value separation: bark down, away from foliage.
                        for v in c.iter_mut() {
                            *v *= 0.80;
                        }
                    } else {
                        // Crown-scale shading. `lit` is how close this cell is
                        // to the canopy top of its own column, so the top of a
                        // crown catches light and its interior does not --
                        // measured over the crown, never over the cell.
                        let d = density[i];
                        let depth_in = (d - 0.25).clamp(0.0, 1.0);
                        let above = (y - top[x as usize]).max(0) as f32;
                        let lit = (1.0 - above / 18.0).clamp(0.0, 1.0);
                        let k = 1.0 + 0.22 * lit - 0.42 * depth_in;
                        for v in c.iter_mut() {
                            *v *= k;
                        }
                    }

                    if far {
                        // Atmospheric perspective: toward the sky, and
                        // contrast flattened. The sky is sampled from the
                        // frame's own top row so this cannot disagree with
                        // whatever the renderer decided the sky is.
                        let sky = px(plant, x, 2);
                        // Gentle. At 0.34 the back layer read as fog rather
                        // than as distance, and greyed the stand out.
                        let t = 0.20;
                        for (v, s) in c.iter_mut().zip(sky) {
                            *v += (s - *v) * t;
                        }
                    }
                }
                _ => {}
            }
            out[i] = [c[0].clamp(0.0, 255.0), c[1].clamp(0.0, 255.0), c[2].clamp(0.0, 255.0)];
        }
    }
    out
}
