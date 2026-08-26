//! **Does an arch actually outspan a lintel in this engine?**
//!
//! The claim, from `Reports/structural-support-model.md`'s design follow-up:
//! `load::capacity` scales with section depth *squared*, measured
//! perpendicular to wherever support arrives, and `stone.ron` prices a step
//! taken from *below* at 1 against 3 for one taken from *above* — compression
//! cheap, tension dear. Put together that predicts a curved roof should span
//! further than a flat one **with nothing added to the engine**, which would
//! mean a player can discover real structural engineering inside a
//! falling-sand game. It was a prediction, not a measurement, and this is the
//! measurement.
//!
//! # Why a margin sweep and not one scene
//!
//! `Reports/instruments.md` on `anchor_probe`: *"It sweeps for a margin, not
//! an outcome. Past its margin every rule agrees a structure falls; short of
//! it every rule agrees it stands. A rule can only show itself in where the
//! margin is, which is also the quantity a player feels."* Here it is the
//! *geometry* rather than the rule that varies, but the logic is identical —
//! a single span either stands in both arms or falls in both, and says
//! nothing either way. That probe's own first run put its subject where the
//! margin could not reach it and produced a null; this one sweeps until both
//! forms have fallen and refuses to report a margin it did not bracket.
//!
//! # The confound this exists to remove
//!
//! **A semicircular ring is longer than the chord it spans, so an arch of the
//! same thickness is simply more stone.** "More material holds better" is not
//! a discovery. So there are three arms, not two:
//!
//! | arm | what it is |
//! |---|---|
//! | `lintel` | a flat slab of thickness `T` across the opening |
//! | `arch` | a semicircular ring of thickness `T` springing off the same piers |
//! | `lintel=` | a flat slab thickened until it uses **the arch's own cell count** |
//!
//! `lintel=` is the control that matters. If the arch beats it, the geometry
//! is doing the work; if it only beats plain `lintel`, the mass is. And a
//! thicker lintel is also heavier, which is the honest fight: in this model
//! a deeper section buys capacity *and* costs load.
//!
//! # What this does not model, stated so the result is not oversold
//!
//! A real arch works by putting its ring into pure compression and throwing
//! the thrust sideways into its abutments. **This engine has no lateral
//! thrust** — support is a DAG over four neighbours, and nothing pushes
//! outward. So if the arch wins here it wins for a different reason than it
//! does in stone: every voussoir is supported from *below-ish* by the next
//! one down, which is both the cheap direction and a short lever arm, where a
//! lintel's midspan is held only from the side across half the opening. That
//! is still the game-relevant fact — curving the roof lets you span further —
//! but it is not a claim that this is a masonry simulator.
//!
//! ```text
//! cargo run --release --example arch_probe
//! cargo run --release --example arch_probe -- spans=48,64,80,96,112 thickness=3 frames=1200
//! cargo run --release --example arch_probe -- spans=104 thickness=11   # the cost bisect
//! ```

use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material::{self, MaterialKind};
use pixel_physics::sim::parallel;
use pixel_physics::sim::structural;
use pixel_physics::sim::world::World;

/// World height. Tall enough that a semicircular arch over the widest span
/// in the default sweep still has sky above its crown — an arch clipped by
/// the world edge would anchor on it (`Cell::OUT_OF_BOUNDS` is `BEDROCK`)
/// and stand for a reason that has nothing to do with its shape.
const H: i32 = 200;
/// Rows of bedrock at the bottom, so the piers have a real anchor.
const FLOOR_Y: i32 = H - 4;
/// How tall the piers stand. Only has to clear the debris, so the roof is
/// unambiguously in the air.
const PIER_H: i32 = 30;
/// Pier width beyond the arch ring's own footprint, so both forms bear on
/// the same abutment and neither is standing on a knife edge.
const PIER_EXTRA: i32 = 6;
/// Empty columns outside the piers.
const MARGIN: i32 = 10;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Form {
    Lintel,
    LintelEq,
    LintelDeep,
    Arch,
}

impl Form {
    fn label(self) -> &'static str {
        match self {
            Form::Lintel => "lintel",
            Form::LintelEq => "lintel=",
            Form::LintelDeep => "lintel3T",
            Form::Arch => "arch",
        }
    }
}

struct Scene {
    world: World,
    spring_y: i32,
    /// Roof cells placed — everything above the springing line.
    placed: usize,
    /// The clear opening actually built, measured rather than assumed.
    clear_span: i32,
}

/// Build one arm. `thickness` is the ring/slab depth for this arm, which is
/// what `LintelEq` varies.
fn build(form: Form, span: i32, thickness: i32) -> Scene {
    let r_in = span / 2;
    let half = r_in + thickness + PIER_EXTRA;
    let w = 2 * half + 2 * MARGIN + 1;
    let cx = w / 2;
    let spring_y = FLOOR_Y - PIER_H;

    // **No chain leash.** `chain_reach` decides whether a failure is
    // *licensed* to propagate, not whether the load model believes it — and
    // at the shipped reach an undisturbed hand-built scene cannot fail
    // whatever the model thinks (`filmstrip`'s `capped` records the same
    // trap). This probe is a question about the model, so the leash comes
    // off, exactly as `structural.rs`'s own tests take it off.
    let mut world = World::new(Rect::new(0, 0, w - 1, H - 1)).without_chain_limit();

    for y in FLOOR_Y..H {
        for x in 0..w {
            world.set(x, y, Cell::new(material::BEDROCK, 0));
        }
    }
    // The two abutments, identical in every arm.
    for y in (spring_y + 1)..FLOOR_Y {
        for x in 0..w {
            let d = (x - cx).abs();
            if d >= r_in && d < r_in + thickness + PIER_EXTRA {
                world.set(x, y, Cell::new(material::STONE, 0));
            }
        }
    }

    let mut placed = 0usize;
    for y in 0..=spring_y {
        for x in 0..w {
            let inside = match form {
                // A flat slab occupying the `thickness` rows that end on the
                // springing line, bearing on both pier tops.
                Form::Lintel | Form::LintelEq | Form::LintelDeep => y > spring_y - thickness && (x - cx).abs() < r_in + thickness + PIER_EXTRA,
                // A semicircular ring springing from the same line. Its feet
                // land inside the pier footprint by construction, so the two
                // forms bear on the same stone.
                Form::Arch => {
                    let dx = (x - cx) as f32;
                    let dy = (y - spring_y) as f32;
                    let d = (dx * dx + dy * dy).sqrt();
                    d >= r_in as f32 && d < (r_in + thickness) as f32
                }
            };
            if inside {
                world.set(x, y, Cell::new(material::STONE, 0));
                placed += 1;
            }
        }
    }

    // Measured, not assumed: the widest run of empty cells on the row just
    // below the springing line. The two forms have to be roofing the *same
    // hole* or the comparison is about the hole.
    let mut clear_span = 0;
    let mut run = 0;
    for x in 0..w {
        if world.get(x, spring_y + 1).material == material::EMPTY {
            run += 1;
            clear_span = clear_span.max(run);
        } else {
            run = 0;
        }
    }

    structural::compute_world_distances(&mut world);
    // Hand-placed geometry reaches the heap through nothing, so say it
    // arrived — the same pair `filmstrip`'s `capped` uses, and for the same
    // reason: a scene nothing ever asks about stands because it was never
    // questioned, which is `CLAUDE.md`'s vacuous-test failure in scene form.
    for y in 0..=spring_y {
        for x in 0..w {
            if world.get(x, y).material == material::STONE {
                world.schedule_structural_check(x, y);
            }
        }
    }
    world.record_disturbance(cx - r_in, spring_y, 0);
    world.record_disturbance(cx + r_in, spring_y, 0);

    Scene { world, spring_y, placed, clear_span }
}

/// Stone still standing **above the springing line** — the roof, and only the
/// roof.
///
/// `Solid` and above the line, deliberately. `CLAUDE.md`'s metric trap: a
/// failure count is not a damage count, and a failed cell that became rubble
/// is still sitting there. Rubble is a `Powder`, so it drops out of this
/// count; a piece that came off intact is still `Solid` but has fallen *below*
/// the line, so it drops out too. What is left is what is still in the air.
fn standing(world: &World, spring_y: i32) -> usize {
    let bounds = world.bounds().expect("bounded");
    let mut n = 0;
    for y in 0..=spring_y {
        for x in bounds.min_x..=bounds.max_x {
            let c = world.get(x, y);
            if world.materials.kind(c.material) == MaterialKind::Solid && c.material != material::BEDROCK {
                n += 1;
            }
        }
    }
    n
}

struct Row {
    form: Form,
    span: i32,
    thickness: i32,
    clear_span: i32,
    placed: usize,
    standing: usize,
    overloaded: u32,
    unsupported: u32,
}

impl Row {
    fn share(&self) -> f64 {
        100.0 * self.standing as f64 / self.placed.max(1) as f64
    }
}

/// Write one frame of a scene as a PNG, cropped to the roof band and scaled
/// up by an integer factor.
///
/// **Rendered wide, declared tight** is the review queue's rule, and the
/// crop here is the band that holds the whole question: sky above the arch's
/// crown, both abutments, and the floor the debris lands on. Scaled because
/// the scenes are ~143 cells across and the stills the owner has been able to
/// judge are 700-950 px — `image-rendering: pixelated` on the page means an
/// integer upscale costs nothing but bytes and is exactly what a zoom control
/// would have produced anyway.
fn shot(world: &World, path: &std::path::Path, band: (i32, i32), zoom: usize) {
    let bounds = world.bounds().expect("bounded");
    let (w, h) = ((bounds.max_x + 1) as usize, (bounds.max_y + 1) as usize);
    let mut renderer = pixel_physics::render::Renderer::new();
    let particles = pixel_physics::sim::particle::ParticleSystem::new();
    let mut buf = vec![0u8; w * h * 4];
    renderer.draw(world, &particles, &std::collections::HashSet::new(), &mut buf, (w as u32, h as u32), true);

    let (y0, y1) = (band.0.max(0) as usize, (band.1 as usize).min(h));
    let (cw, ch) = (w * zoom, (y1 - y0) * zoom);
    let mut out = vec![0u8; cw * ch * 4];
    for y in 0..ch {
        for x in 0..cw {
            let src = ((y0 + y / zoom) * w + x / zoom) * 4;
            out[(y * cw + x) * 4..(y * cw + x) * 4 + 4].copy_from_slice(&buf[src..src + 4]);
        }
    }
    image::save_buffer(path, &out, cw as u32, ch as u32, image::ColorType::Rgba8).expect("write png");
}

fn run(form: Form, span: i32, thickness: i32, frames: usize) -> Row {
    run_with_shots(form, span, thickness, frames, None)
}

/// `shots = Some((dir, every, count))` writes `<dir>/<form>_NN.png` as the
/// scene runs, so the same run that produces the number produces the picture.
/// `CLAUDE.md`: post the artifact you actually judged by, not a re-render.
fn run_with_shots(form: Form, span: i32, thickness: i32, frames: usize, shots: Option<(&str, usize, usize)>) -> Row {
    let mut scene = build(form, span, thickness);
    let band = (scene.spring_y - (span / 2 + thickness) - 8, FLOOR_Y + 3);
    let mut taken = 0usize;
    for f in 0..frames {
        if let Some((dir, every, count)) = shots {
            if f % every == 0 && taken < count {
                let p = std::path::Path::new(dir).join(format!("{}_{taken:02}.png", form.label().replace('=', "eq")));
                shot(&scene.world, &p, band, 6);
                taken += 1;
            }
        }
        parallel::step(&mut scene.world);
        scene.world.step_active_sites();
        // **`step_chunk_bodies` is only called from `App::update`.** It is
        // not inside `parallel::step` and not inside `step_active_sites`, so
        // a harness that steps those two and stops leaves every promoted
        // rigid body frozen in the air: it never lands, never crushes, and
        // never triggers the secondary collapse a landing causes. The first
        // version of this probe did exactly that. It does not change what
        // this probe measures -- a promoted body's cells have already left
        // the grid, so they count as off the roof either way -- but it
        // understates the cascade, and it renders as slabs hanging in the
        // sky. Verified against the margins either way; see the report.
        pixel_physics::sim::rigid::step_chunk_bodies(&mut scene.world);
    }
    let f = scene.world.structural_failures;
    Row {
        form,
        span,
        thickness,
        clear_span: scene.clear_span,
        placed: scene.placed,
        standing: standing(&scene.world, scene.spring_y),
        overloaded: f.overloaded,
        unsupported: f.unsupported,
    }
}

/// A roof counts as standing if it kept this share of its cells in the air.
///
/// Not 100%: a few cells shedding off an edge is weathering, not a collapse,
/// and a bar sitting on the measured value is what `CLAUDE.md` says never to
/// set. 90% is "the roof is still a roof".
const INTACT: f64 = 90.0;

fn main() {
    // **Defaults that bracket the margin.** 8..48 was the first sweep and
    // every arm held at every span, so a bare run printed the null and no
    // number -- correct behaviour, useless default. The margins measured
    // 2026-08-26 are lintel 56, lintel= 64, lintel3T 96, arch 104, so this
    // range has a standing span and a fallen one for all four.
    let mut spans: Vec<i32> = (48..=120).step_by(8).collect();
    let mut thickness = 3i32;
    let mut frames = 1500usize;
    let mut verbose = false;
    // `shots=<dir>` renders the run as well as measuring it.
    let mut shots_dir: Option<String> = None;
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "spans" => spans = v.split(',').map(|t| t.parse().expect("spans=8,16,24")).collect(),
            "thickness" => thickness = v.parse().expect("thickness=N"),
            "frames" => frames = v.parse().expect("frames=N"),
            "verbose" => verbose = v != "0",
            "shots" => shots_dir = Some(v.to_string()),
            _ => eprintln!("ignoring unknown argument {arg}"),
        }
    }
    // **Echo the parameters**, so a log that does not name its sweep was
    // written by a binary that never had one (`CLAUDE.md`'s megastudy).
    println!("arch_probe: spans {spans:?} thickness {thickness} frames {frames} intact>={INTACT}%\n");

    let mut rows: Vec<Row> = Vec::new();
    println!("{:>8} {:>6} {:>6} {:>4} {:>7} {:>8} {:>8}   failures", "form", "span", "clear", "T", "placed", "standing", "share");
    for &span in &spans {
        // The arch is built first because `lintel=` is defined *by* its cell
        // count. Derived per span rather than fixed: the ring's excess over
        // the chord grows with the span, so one constant would make the
        // control fair at one width and unfair at every other.
        let arch = build(Form::Arch, span, thickness);
        let slab_width = 2 * (span / 2 + thickness + PIER_EXTRA) - 1;
        let eq = ((arch.placed as f64 / slab_width as f64).round() as i32).max(thickness);
        drop(arch);

        for form in [Form::Lintel, Form::LintelEq, Form::LintelDeep, Form::Arch] {
            let t = match form {
                Form::LintelEq => eq,
                // **The control on the alternative explanation.** `capacity`
                // scales with section depth *squared*, so if the arch's
                // advantage were really "more depth where it counts", simply
                // making the lintel far deeper should buy the same thing.
                // Three times the thickness is a 9x capacity term and a 3x
                // load term, which is the direction that should win outright
                // if depth is the story.
                Form::LintelDeep => thickness * 3,
                _ => thickness,
            };
            let r = match shots_dir.as_deref() {
                Some(dir) => run_with_shots(form, span, t, frames, Some((dir, 12, 16))),
                None => run(form, span, t, frames),
            };
            println!(
                "{:>8} {:>6} {:>6} {:>4} {:>7} {:>8} {:>7.1}%   overloaded {} unsupported {}",
                r.form.label(),
                r.span,
                r.clear_span,
                r.thickness,
                r.placed,
                r.standing,
                r.share(),
                r.overloaded,
                r.unsupported
            );
            rows.push(r);
        }
        if verbose {
            println!();
        }
    }

    // ---- the margins, and the controls that say whether they mean anything -
    println!("\n-- margins (widest span whose roof stayed >={INTACT}% intact) --");
    let mut bracketed = true;
    for form in [Form::Lintel, Form::LintelEq, Form::LintelDeep, Form::Arch] {
        let mine: Vec<&Row> = rows.iter().filter(|r| r.form == form).collect();
        let held: Vec<i32> = mine.iter().filter(|r| r.share() >= INTACT).map(|r| r.span).collect();
        let fell: Vec<i32> = mine.iter().filter(|r| r.share() < INTACT).map(|r| r.span).collect();
        match (held.iter().max(), fell.iter().min()) {
            (Some(&h), Some(&f)) => println!("{:>8}: holds to {h}, first failure at {f}", form.label()),
            (Some(&h), None) => {
                bracketed = false;
                println!("{:>8}: held at EVERY span up to {h} -- the sweep never reached its margin", form.label());
            }
            (None, Some(&f)) => {
                bracketed = false;
                println!("{:>8}: fell at EVERY span from {f} -- the margin is below the sweep", form.label());
            }
            (None, None) => println!("{:>8}: no data", form.label()),
        }
    }
    if !bracketed {
        println!(
            "\n**Not a result.** At least one arm never crossed its margin inside this sweep, which is\n\
             exactly the null `anchor_probe`'s first run produced. Widen `spans=` until every arm has\n\
             both a standing span and a fallen one before reading anything above as a comparison."
        );
    }
    // The other half of the control: a comparison of forms is only about form
    // if the material is accounted for. Printed as a ratio per span so the
    // reader can see what `lintel=` is actually holding fixed.
    println!("\n-- material, arch against plain lintel (the confound `lintel=` removes) --");
    for &span in &spans {
        let get = |f: Form| rows.iter().find(|r| r.form == f && r.span == span).map(|r| r.placed).unwrap_or(0);
        let (l, e, d, a) = (get(Form::Lintel), get(Form::LintelEq), get(Form::LintelDeep), get(Form::Arch));
        println!(
            "  span {span:>3}: lintel {l:>5} cells, lintel= {e:>5}, lintel3T {d:>5}, arch {a:>5}  (arch/lintel {:.2}x)",
            a as f64 / l.max(1) as f64
        );
    }
}
