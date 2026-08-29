//! **Does the picture get better if the renderer has more pixels than the
//! simulation has cells?**
//!
//! The owner's complaint is that plants are hard to make look good in pixel
//! graphics, and that raising the simulation resolution is not on the table.
//! This is the experiment that separates those two things, because they are
//! separable: the simulation's cell grid is a *physical* lattice, and the
//! framebuffer does not have to be the same lattice.
//!
//! Two facts make the headroom real rather than hypothetical:
//!
//! - `main.rs` opens the window at `LogicalSize::new(WIDTH * 2, HEIGHT * 2)`
//!   against a `Pixels::new(WIDTH, HEIGHT, ..)` framebuffer, so **every world
//!   cell already occupies at least a 2x2 block of physical screen pixels and
//!   all four are byte-identical**. The pixels exist today and are spent on
//!   replication.
//! - `Renderer::cell_colour` already takes a sub-cell offset (`sub`), and
//!   exactly one thing reads it (the crack strip). The machinery for "this
//!   pixel is *inside* that cell, here" is built and idle at 1:1.
//!
//! So this renders one world twice at the same final size:
//!
//! | arm | what it is |
//! |---|---|
//! | `baseline` | the shipped 1:1 render, nearest-replicated `scale`x -- exactly what the window shows today |
//! | `subpixel` | the same world reconstructed at `scale` pixels per cell |
//!
//! # What the reconstruction is, and why not hqx/xBR
//!
//! The obvious reading of "upsample it" is a pixel-art scaler (EPX, hqx, xBR)
//! run over the finished frame. Those work on **colour** — they infer shape
//! from which neighbouring pixels happen to match — and this world is the
//! worst case for them: rock and soil carry a deliberate per-cell shade
//! jitter, so no two neighbouring pixels match and the filter has nothing to
//! latch onto. It would smear the grain and leave the plants alone, which is
//! backwards.
//!
//! This instead reconstructs from **what the cell is**, which the renderer
//! knows and a filter cannot recover: plant tissue is resampled as a
//! continuous scalar field (a metaball/SPH-style kernel sum over the 5x5
//! neighbourhood) and thresholded with an antialiased edge. A diagonal chain
//! of one-cell twigs becomes a smooth tapered stroke instead of a staircase;
//! an isolated leaf cell becomes a round lobe instead of a hard square; a run
//! of leaf cells merges into a canopy mass with a ragged outline. Terrain is
//! left alone in `arm=plants`, because the grain *is* the look there.
//!
//! ```text
//! cargo run --release --example subpixel -- scale=3 arm=plants out=/tmp/x.png
//! ```

use pixel_physics::render::Renderer;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::ChunkCoord;
use pixel_physics::sim::material::{self, MaterialKind};
use pixel_physics::sim::organism::{self, CellType};
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::parallel;

mod common;

const W: i32 = 512;
const H: i32 = 320;

/// What a cell is, for the reconstruction's purposes. The renderer's own
/// colour is used verbatim; this only decides *shape*.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Empty,
    Foliage,
    Wood,
    Other,
}

struct Args {
    scale: i32,
    frames: u64,
    plants: usize,
    species: String,
    arm: String,
    /// Kernel radius for wood, in cells.
    wood_r: f32,
    /// Kernel radius for foliage, in cells.
    leaf_r: f32,
    /// Isosurface level on the summed kernel.
    level: f32,
    /// Half-width of the antialiased band around `level`, as a fraction of it.
    band: f32,
    /// How the reconstructed pixel takes its colour: `0.0` is the nearest
    /// contributing cell's own colour, `1.0` is the kernel-weighted mean of
    /// all of them.
    ///
    /// **This is the knob that decides crisp against mushy, and the two are
    /// separable.** The first pass here weighted the colour by the same
    /// kernel that decides the shape, which is a 5x5 blur of the palette --
    /// so the silhouette came out smooth and the whole plant came out
    /// soft-focus, and the soft focus is what reads as "worse" even where the
    /// silhouette is plainly better. Shape wants a wide smooth kernel;
    /// colour wants the nearest sample. They do not have to be the same
    /// question.
    colour_blend: f32,
    /// How hard the reconstructed *interior* is darkened.
    ///
    /// **The field that decides the silhouette is also a thickness reading,
    /// and this is the whole reason the reconstruction is worth more than a
    /// smoothing filter.** `cov` sits at `level` exactly on the outline and
    /// climbs with how much tissue is stacked around this point, so
    /// `cov - level` is "how deep inside the mass am I" -- an ambient
    /// occlusion term, already computed, for a canopy that is otherwise
    /// perfectly flat. A crown lit uniformly across its whole area is the
    /// single largest reason foliage here reads as confetti rather than as
    /// volume, and no amount of per-cell colour work can fix it: the cell
    /// has no interior.
    ao: f32,
    /// How hard the reconstructed surface is lit directionally.
    ///
    /// The same field gives a normal: `grad(cov)` is analytic for a sum of
    /// kernels, points into the mass, and its negation is the outward
    /// surface normal at sub-cell resolution. Lambert against a fixed sun
    /// then rounds every lobe and every branch, which is the second thing
    /// the cell lattice cannot express -- at 1:1 a branch is a line of flat
    /// squares and has no side facing anywhere.
    shade: f32,
    /// Sun direction for `shade`, in screen space, pointing *toward* the sun.
    sun: (f32, f32),
    /// **How dark a shell just inside the silhouette is drawn** — a cartoon
    /// ink line, in `0..1` of the fill colour.
    ///
    /// Asked for directly, 2026-08-29: the first tuning was rejected with
    /// *"the edges between color or material look weird, kinda 3d-ish. Could
    /// it be more flat or cartoony"*, which is a verdict on `ao` and `shade`
    /// rather than on the reconstruction — those two are what put a lit side
    /// and a shaded side on every lobe, and reading as rounded volume is
    /// exactly what they are for. Flat wants the opposite: one fill, and the
    /// form carried by a drawn **edge** instead of by shading.
    ///
    /// It rides the same field as everything else. `cov` is an occupancy
    /// fraction, so the shell is just the band immediately above the
    /// threshold — no edge detection, no second pass.
    outline: f32,
    /// How thick that shell is, in occupancy fraction above `level`.
    outline_width: f32,
    crop: Option<(i32, i32, i32, i32)>,
    daylight: Option<f32>,
    out: String,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            scale: 3,
            frames: 6000,
            plants: 0,
            species: String::new(),
            arm: "plants".into(),
            wood_r: 0.95,
            leaf_r: 1.15,
            level: 0.30,
            band: 0.12,
            colour_blend: 0.0,
            ao: 0.30,
            shade: 0.30,
            sun: (-0.55, -0.84),
            outline: 0.0,
            outline_width: 0.22,
            crop: None,
            daylight: Some(1.0),
            out: "/tmp/subpixel.png".into(),
        }
    }
}


/// Kernel value at a lattice offset. `(1 - d^2/r^2)^2` is the standard
/// smooth compact kernel: exactly zero past `r`, so a 5x5 bound is a real
/// bound and not a truncation, and smooth at the centre, so a straight run of
/// cells does not pinch between them.
fn kernel(dx: f32, dy: f32, r: f32) -> f32 {
    let t = 1.0 - (dx * dx + dy * dy) / (r * r);
    if t <= 0.0 {
        0.0
    } else {
        t * t
    }
}

/// **What the kernel sums to when every cell in range is tissue** — the
/// denominator that turns a raw kernel sum into an occupancy fraction.
///
/// Without it the reconstruction quilts. A kernel sum over a *regular*
/// lattice ripples at the lattice period unless the kernel is far wider than
/// the spacing, so deep inside a solid mass -- where the answer should be a
/// flat "completely full" -- the raw sum oscillates between cell centres and
/// cell corners. Read as a thickness that ripple is amplified straight into a
/// visible grid of squares, which is exactly the artifact the whole exercise
/// exists to remove, reintroduced one level down. Measured by rendering with
/// the two terms off: interior smooth; with them on: quilted.
///
/// The denominator carries the *same* ripple, so dividing cancels it, and it
/// depends only on where the pixel sits inside its cell -- so it is a
/// `scale x scale` table computed once, not per-pixel work.
fn partition_table(scale: i32, r: f32) -> Vec<f32> {
    let mut t = vec![0.0f32; (scale * scale) as usize];
    for sy in 0..scale {
        for sx in 0..scale {
            let (u, v) = ((sx as f32 + 0.5) / scale as f32, (sy as f32 + 0.5) / scale as f32);
            let mut sum = 0.0;
            for n in -2..=2 {
                for m in -2..=2 {
                    sum += kernel(u + m as f32 - 0.5, v + n as f32 - 0.5, r);
                }
            }
            t[(sy * scale + sx) as usize] = sum;
        }
    }
    t
}

/// One reconstructed layer's reading at a point: how much tissue of this
/// class is stacked here, its colour, and the field's gradient.
struct Layer {
    /// Occupancy fraction in `0..=1`: 1 is "every cell in range is this
    /// class", which is the interior of a solid mass.
    cov: f32,
    acc: [f32; 3],
    /// The un-normalised kernel sum, the denominator `acc` is a mean over.
    raw: f32,
    nearest: [f32; 3],
    grad: [f32; 2],
}

/// Kernel sum over the 5x5 neighbourhood, restricted to one class, divided by
/// [`partition_table`] so `cov` is an occupancy **fraction** in `0..=1`
/// rather than a lattice-rippled sum.
#[allow(clippy::too_many_arguments)]
fn field(
    class: &[Class],
    colours: &[u8],
    w: i32,
    h: i32,
    bx: i32,
    by: i32,
    fx: f32,
    fy: f32,
    r: f32,
    want: Class,
    partition: f32,
) -> Option<Layer> {
    let mut cov = 0.0f32;
    let mut acc = [0.0f32; 3];
    let mut grad = [0.0f32; 2];
    let mut best = f32::NEG_INFINITY;
    let mut nearest = [0.0f32; 3];
    for ny in (by - 2)..=(by + 2) {
        for nx in (bx - 2)..=(bx + 2) {
            if nx < 0 || ny < 0 || nx >= w || ny >= h || class[(ny * w + nx) as usize] != want {
                continue;
            }
            let dx = fx - (nx as f32 + 0.5);
            let dy = fy - (ny as f32 + 0.5);
            let t = 1.0 - (dx * dx + dy * dy) / (r * r);
            if t <= 0.0 {
                continue;
            }
            let k = t * t;
            cov += k;
            let i = ((ny * w + nx) * 4) as usize;
            let c = [colours[i] as f32, colours[i + 1] as f32, colours[i + 2] as f32];
            for j in 0..3 {
                acc[j] += c[j] * k;
            }
            // Whose colour this pixel wears, when `colour_blend` says
            // nearest. `k` is monotone in distance, so the largest `k` is the
            // nearest cell centre -- no second distance comparison needed.
            if k > best {
                best = k;
                nearest = c;
            }
            // d(k)/d(p) for k = t^2, t = 1 - |p-c|^2/r^2. Points *into* the
            // mass, because `k` grows toward the cell centre; the outward
            // normal is its negation.
            let g = -4.0 * t / (r * r);
            grad[0] += g * dx;
            grad[1] += g * dy;
        }
    }
    (cov > 0.0).then_some(Layer {
        cov: cov / partition,
        // `acc` is only ever read as `acc / cov`, a weighted mean, so it must
        // stay on the raw sum's scale.
        acc,
        raw: cov,
        nearest,
        // Same normalisation as `cov`: the gradient of a normalised field.
        grad: [grad[0] / partition, grad[1] / partition],
    })
}

fn main() {
    let mut a = Args::default();
    for arg in std::env::args().skip(1) {
        let (k, v) = arg.split_once('=').unwrap_or((arg.as_str(), ""));
        match k {
            "scale" => a.scale = v.parse().expect("scale"),
            "frames" => a.frames = v.parse().expect("frames"),
            "plants" => a.plants = v.parse().expect("plants"),
            "species" => a.species = v.to_string(),
            "arm" => a.arm = v.to_string(),
            "wood_r" => a.wood_r = v.parse().expect("wood_r"),
            "leaf_r" => a.leaf_r = v.parse().expect("leaf_r"),
            "level" => a.level = v.parse().expect("level"),
            "band" => a.band = v.parse().expect("band"),
            "blend" => a.colour_blend = v.parse().expect("blend"),
            "ao" => a.ao = v.parse().expect("ao"),
            "shade" => a.shade = v.parse().expect("shade"),
            "outline" => a.outline = v.parse().expect("outline"),
            "outline_width" => a.outline_width = v.parse().expect("outline_width"),
            "daylight" => a.daylight = Some(v.parse().expect("daylight")),
            "crop" => {
                let n: Vec<i32> = v.split(',').map(|s| s.parse().expect("crop")).collect();
                a.crop = Some((n[0], n[1], n[2], n[3]));
            }
            "out" => a.out = v.to_string(),
            other => panic!("unknown arg {other:?}"),
        }
    }
    // The harness names its own parameters, per `CLAUDE.md`'s stale-binary
    // rule: a sheet that does not say what it was rendered at was written by
    // a binary that never had the argument.
    println!(
        "subpixel: scale={} arm={} frames={} plants={} species={:?} wood_r={} leaf_r={} level={} band={} blend={}",
        a.scale, a.arm, a.frames, a.plants, a.species, a.wood_r, a.leaf_r, a.level, a.band, a.colour_blend
    );
    println!(
        "          ao={} shade={} sun={:?} outline={} outline_width={}",
        a.ao, a.shade, a.sun, a.outline, a.outline_width
    );

    let base = common::PlantScene::default();
    let mut world = common::PlantScene {
        // Empty means "keep the builder's own default" -- overriding it
        // unconditionally plants a species named "", which builds a world
        // with no plants in it and reports success.
        species: if a.species.is_empty() { base.species.clone() } else { a.species.clone() },
        trees: if a.plants > 0 { a.plants } else { base.trees },
        ..base
    }
    .build();

    // `App::update`'s own order, minus the phases no plant scene reaches
    // (blasts, chunk bodies, the gnome). `step_active_sites` is the one that
    // grows the trees, and omitting it is how a plant harness ends up
    // measuring bare soil.
    let mut particles = ParticleSystem::default();
    for _ in 0..a.frames {
        parallel::step(&mut world);
        world.step_liquid_bodies();
        world.step_active_sites();
        particles.step(&mut world);
        // **Not optional.** `step_fields` is what propagates the light
        // channel, and `Germinate` reads it: without this line nothing
        // germinates and the harness renders bare soil while reporting
        // success. The cell counter below is what caught it.
        world.step_fields();
    }

    // The plant pass: the shipped renderer, 1:1, no changes at all. Its
    // output is the colour source for the reconstruction, so nothing here
    // can invent a colour the engine would not have drawn.
    let mut r = Renderer::new();
    r.pinned_light = a.daylight.map(pixel_physics::sky::frame_for_daylight);
    let mut plant_frame = vec![0u8; (W * H * 4) as usize];
    let touched: std::collections::HashSet<ChunkCoord> = std::collections::HashSet::new();
    r.draw(&world, &particles, &touched, &mut plant_frame, (W as u32, H as u32), true);

    // Per-cell class, read before the strip below removes the evidence.
    let mut class = vec![Class::Empty; (W * H) as usize];
    let mut foliage = 0usize;
    let mut wood = 0usize;
    for y in 0..H {
        for x in 0..W {
            let cell = world.get(x, y);
            let mat = world.materials.get(cell.material);
            class[(y * W + x) as usize] = if cell.material == material::EMPTY {
                Class::Empty
            } else if mat.kind == MaterialKind::Plant {
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

    // The background pass: the same world with every plant cell emptied,
    // drawn by a *fresh* renderer so nothing is carried over from the pass
    // above. This is what is behind a plant pixel the reconstruction erodes
    // away, and taking it from the engine rather than inpainting it is what
    // keeps the sky gradient, the skyline and the ground exactly right.
    for y in 0..H {
        for x in 0..W {
            if matches!(class[(y * W + x) as usize], Class::Foliage | Class::Wood) {
                world.set(x, y, Cell::EMPTY);
            }
        }
    }
    let mut rb = Renderer::new();
    rb.pinned_light = a.daylight.map(pixel_physics::sky::frame_for_daylight);
    let mut back_frame = vec![0u8; (W * H * 4) as usize];
    rb.draw(&world, &particles, &touched, &mut back_frame, (W as u32, H as u32), true);

    let s = a.scale;
    let (cx, cy, cw, ch) = a.crop.unwrap_or((0, 0, W, H));
    let (ow, oh) = (cw * s, ch * s);
    let mut out = vec![0u8; (ow * oh * 4) as usize];
    let started = std::time::Instant::now();

    let px = |buf: &[u8], x: i32, y: i32| -> [f32; 3] {
        let x = x.clamp(0, W - 1);
        let y = y.clamp(0, H - 1);
        let i = ((y * W + x) * 4) as usize;
        [buf[i] as f32, buf[i + 1] as f32, buf[i + 2] as f32]
    };

    let reconstruct = a.arm != "baseline";
    let wood_part = partition_table(s, a.wood_r);
    let leaf_part = partition_table(s, a.leaf_r);

    for oy in 0..oh {
        for ox in 0..ow {
            // Position in continuous cell space, at the centre of this output
            // pixel. This is the whole of the resolution decoupling: the
            // output lattice is `s` times finer than the cell lattice, and
            // every question below is asked at a real-valued position.
            let fx = cx as f32 + (ox as f32 + 0.5) / s as f32;
            let fy = cy as f32 + (oy as f32 + 0.5) / s as f32;
            let bx = fx.floor() as i32;
            let by = fy.floor() as i32;

            let mut rgb = px(&back_frame, bx, by);

            if reconstruct {
                // **Two layers, foliage over wood, not one field over both.**
                // A single field mixes the two palettes at every contact and
                // the nearest-colour rule then renders their boundary as a
                // hard square mosaic inside a smooth outline -- visibly worse
                // than the staircase it replaced. Splitting them is also the
                // physically true reading: leaves hang in front of the twig
                // they grow off, so the twig should pass *behind* the crown
                // rather than tile with it.
                let pi = ((oy.rem_euclid(s)) * s + ox.rem_euclid(s)) as usize;
                // `arm=all` puts terrain through the same reconstruction, to
                // answer the other half of the owner's question -- whether
                // everything else wants this too, or whether the per-cell
                // grain that makes rock read as rock is exactly what a
                // smooth silhouette destroys.
                let ground = (a.arm == "all")
                    .then(|| field(&class, &plant_frame, W, H, bx, by, fx, fy, a.wood_r, Class::Other, wood_part[pi]))
                    .flatten();
                let wood =
                    field(&class, &plant_frame, W, H, bx, by, fx, fy, a.wood_r, Class::Wood, wood_part[pi]);
                let leaf =
                    field(&class, &plant_frame, W, H, bx, by, fx, fy, a.leaf_r, Class::Foliage, leaf_part[pi]);
                for layer in [ground, wood, leaf] {
                    let Some(l) = layer else { continue };
                    let lo = a.level * (1.0 - a.band);
                    let hi = a.level * (1.0 + a.band);
                    let t = ((l.cov - lo) / (hi - lo)).clamp(0.0, 1.0);
                    // Smoothstep, so the outline has a soft shoulder rather
                    // than a one-pixel hard step -- which is the whole
                    // difference between a smooth stroke and a finer
                    // staircase.
                    let alpha = t * t * (3.0 - 2.0 * t);
                    if alpha <= 0.0 {
                        continue;
                    }
                    let b = a.colour_blend;
                    let mean = [l.acc[0] / l.raw, l.acc[1] / l.raw, l.acc[2] / l.raw];
                    let mut plant = [
                        l.nearest[0] + (mean[0] - l.nearest[0]) * b,
                        l.nearest[1] + (mean[1] - l.nearest[1]) * b,
                        l.nearest[2] + (mean[2] - l.nearest[2]) * b,
                    ];
                    // Two terms off the same field, both multiplicative on
                    // the palette colour so a plant never stops being its own
                    // species' green.
                    let mut lift = 1.0f32;
                    if a.ao > 0.0 {
                        // `cov` is a fraction now, so this is literally "how
                        // enclosed is this point", 0 on the outline and 1
                        // where every neighbour is tissue too.
                        let depth = ((l.cov - a.level) / (1.0 - a.level)).clamp(0.0, 1.0);
                        lift *= 1.0 - a.ao * depth;
                    }
                    if a.shade > 0.0 {
                        // **Do not normalise the gradient.** The first
                        // version did, and it quilted every interior into
                        // visible squares: deep inside a mass the kernel
                        // gradients from the surrounding cells cancel, so
                        // `grad` is lattice-scale residue, and dividing by
                        // its own tiny length turns that residue into a
                        // full-strength unit normal. The raw gradient is
                        // already the right shape -- large at the rim where
                        // the surface genuinely faces somewhere, vanishing in
                        // the interior where it genuinely does not -- so
                        // dotting it with the sun and clamping is both
                        // simpler and correct.
                        //
                        // Signed rather than clamped at zero: the point is a
                        // lit side *and* a shadowed side, and clamping would
                        // give a crown a highlight with a flat middle.
                        let ndl = (-l.grad[0] * a.sun.0 - l.grad[1] * a.sun.1).clamp(-1.0, 1.0);
                        lift *= 1.0 + a.shade * ndl;
                    }
                    if a.outline > 0.0 {
                        // The ink line: darkest right at the threshold,
                        // gone by `outline_width` into the mass. Linear
                        // rather than smoothstepped, because a cartoon line
                        // wants a defined inner edge, not a vignette.
                        let into = ((l.cov - a.level) / a.outline_width).clamp(0.0, 1.0);
                        lift *= 1.0 - a.outline * (1.0 - into);
                    }
                    for i in 0..3 {
                        plant[i] *= lift;
                        rgb[i] += (plant[i] - rgb[i]) * alpha;
                    }
                }
            } else {
                // Baseline: nearest replication, which is what the GPU does
                // to the framebuffer today.
                rgb = px(&plant_frame, bx, by);
            }

            let i = ((oy * ow + ox) * 4) as usize;
            out[i] = rgb[0].round().clamp(0.0, 255.0) as u8;
            out[i + 1] = rgb[1].round().clamp(0.0, 255.0) as u8;
            out[i + 2] = rgb[2].round().clamp(0.0, 255.0) as u8;
            out[i + 3] = 255;
        }
    }

    let recon_ms = started.elapsed().as_secs_f64() * 1000.0;
    image::save_buffer(&a.out, &out, ow as u32, oh as u32, image::ColorType::Rgba8).expect("write png");
    // **An upper bound, not the shipped cost.** This loop is serial, rescans
    // the full 5x5 for every output pixel of both layers, and re-derives the
    // partition index per pixel. The draw it would live inside is already
    // parallel over rows, and the scan collapses to nothing for a cell whose
    // 3x3 neighbourhood is uniform -- which is every interior cell of every
    // rock and soil mass in the world. Quote it as "no worse than", and
    // measure the real one in the renderer.
    println!(
        "reconstruction: {recon_ms:.2} ms serial over {} output pixels ({:.1} ns/px, unoptimised upper bound)",
        ow * oh,
        recon_ms * 1e6 / (ow * oh) as f64
    );
    // The discrete counts beside the image, per `CLAUDE.md`: a picture cannot
    // say whether the mechanism reached any cells.
    println!("cells: {foliage} foliage + {wood} wood = {} plant tissue", foliage + wood);
    println!("wrote {} ({}x{})", a.out, ow, oh);
}
