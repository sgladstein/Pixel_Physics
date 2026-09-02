//! **Does a sense read anything when the animal is underground?**
//!
//! Two channels, one question. `Reports/creature-genome-flexibility-2026-09-02.md`
//! §10 Stage 0 owes three measurements before any of that plan is built, and
//! two of them are the same shape: a sense that works in the open and reads
//! flat below the surface is a mechanism that *looks* like it works, because
//! galleries still appear for other reasons.
//!
//! - **0a, the moisture gradient.** Field diffusion is gated on `blocked`,
//!   and `rebuild_blocked` marks a whole block blocked if **one** cell in it
//!   is `Solid`. The lab's round-three finding is the sibling: *"roots steer
//!   by air humidity, not soil water — hydrotropism reads the coarse field
//!   channel, which does not diffuse inside solid ground, so below the
//!   surface it has no gradient."* If that holds for `moisture_gradient` too,
//!   the termite construction bias is inert **exactly where a nest gets
//!   dug**, and §5's remedy changes from "move the coefficient into the
//!   genome" to "fix the sensor, then move it".
//!
//! - **0c, kin in sight.** §4 replaces the named nest material with "where my
//!   own kind are". `blocks_sight` stops a ray on `Solid | Powder` and a nest
//!   is a hole in soil, so an ant in its own gallery may see no kin at all —
//!   and *where is home when home is a hole* is the case the odometer was
//!   covering. If kin are invisible underground, §4 needs a contact-range
//!   fallback, and it needs it **before** Stage 3, not after.
//!
//! # Why `mode=control` runs first and is not optional
//!
//! Both readings above are **nulls if the mechanism is broken and nulls if
//! the probe is** — `CLAUDE.md`'s standing failure, six occurrences across
//! two sessions. So the control is a positive one and it is built by hand: a
//! convex ridge must read a *high* moisture gradient and a flat plain a low
//! one, and an ant with kin in front of it across open floor must see them.
//! A run whose control does not separate says nothing about the lab bed, and
//! this harness prints that verdict rather than leaving it to the reader.
//!
//! **It calls the shipped `creature::moisture_gradient` and
//! `creature::sighted`, never a copy** — the same rule `labshot` calls
//! `LabBox::build` under. A probe that reimplements the two field samples
//! answers the question about itself.
//!
//! ```text
//! cargo run --release --example field_sense_probe -- mode=control
//! cargo run --release --example field_sense_probe -- mode=lab frames=9000 seeds=2
//! cargo run --release --example field_sense_probe -- mode=lab eye=16
//! ```

use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::creature;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::organism::CreatureDef;
use pixel_physics::sim::{frame, material, player, Cell, ParticleSystem, Rect, World};

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// Every argument this run actually took, echoed.
///
/// `CLAUDE.md`: a 3.5-hour study once produced eight byte-identical logs
/// because the binary predated the knob, and *"a knob nobody can see the
/// value of is a knob nobody can tell is disconnected."*
fn echo(mode: &str, frames: u64, seeds: u64, eye: i32, settle: u64) {
    println!("field_sense_probe: mode={mode} frames={frames} seeds={seeds} eye={eye} settle={settle} FIELD_SCALE={}", pixel_physics::sim::field::FIELD_SCALE);
}

/// Has this ant got ground over its head, inside the bed?
///
/// `labnest`'s `roofed` rule, applied to an animal instead of to a void:
/// what a player calls underground is *covered*, and an ant standing at the
/// bottom of an open pit is not. The distinction matters here because a pit
/// is standing void — the metric trap `CLAUDE.md` records scoring a build
/// with no roof at all above one whose tunnels stand.
fn is_under_cover(world: &World, x: i32, y: i32, bed_top: i32) -> bool {
    (bed_top..y).rev().any(|uy| world.get(x, uy).material != material::EMPTY)
}

/// One class of animal, summarised.
#[derive(Default)]
struct Census {
    n: usize,
    grad_sum: f64,
    grad_nonzero: usize,
    kin_seen: usize,
    kin_dist_sum: f64,
    /// **Cells one cast reads, summed.** Not a curiosity: it is the operand
    /// in `sight_tax = sight_fraction * start_energy * sight_reads`, so an
    /// eye cannot be priced without it -- and it is the half of the eye's
    /// cost that occlusion makes cheap. An animal in a tunnel pays less to
    /// look than one sweeping open ground, so shelter pays for itself twice.
    reads_sum: u64,
}

impl Census {
    fn add(&mut self, grad: f32, kin: Option<f32>, reads: u64) {
        self.n += 1;
        self.grad_sum += grad as f64;
        self.reads_sum += reads;
        if grad > 0.0 {
            self.grad_nonzero += 1;
        }
        if let Some(d) = kin {
            self.kin_seen += 1;
            self.kin_dist_sum += d as f64;
        }
    }

    fn row(&self, label: &str) -> String {
        let n = self.n.max(1) as f64;
        format!(
            "{label:>10} {:>6} {:>10.4} {:>9.1}% {:>9.1}% {:>9.1} {:>10.0}",
            self.n,
            self.grad_sum / n,
            100.0 * self.grad_nonzero as f64 / n,
            100.0 * self.kin_seen as f64 / n,
            if self.kin_seen > 0 { self.kin_dist_sum / self.kin_seen as f64 } else { 0.0 },
            self.reads_sum as f64 / n,
        )
    }
}

fn header() {
    println!("\n{:>10} {:>6} {:>10} {:>10} {:>10} {:>9} {:>10}", "where", "ants", "grad mean", "grad > 0", "kin seen", "kin dist", "cells read");
}

/// The ant's `CreatureDef` with a hypothetical eye bolted on.
///
/// **No species in the tree authors a `sight_range` but the beetle**, so the
/// question "what would an ant see" cannot be asked of the shipped def at
/// all — it returns before casting. Overriding it here rather than editing
/// `ant.ron` keeps the measurement out of the thing being measured.
fn ant_with_eye(world: &World, eye: i32) -> CreatureDef {
    let species = world.species.id_of("ant").expect("ant species is compiled in");
    let mut def = world.species.get(species).creature.as_ref().expect("ant is a creature").clone();
    assert_eq!(def.sight_range, 0, "ant.ron authors an eye now; this probe's premise has changed");
    def.sight_range = eye;
    def
}

/// **The positive control, and the run is void without it.**
///
/// A hand-built world with three situations whose answers are known by
/// construction: a convex ridge (gradient must be high), a flat plain
/// (must be lower), and two ants facing each other across bare floor
/// (kin must be seen). Prints PASS/FAIL per leg.
fn control(eye: i32, settle: u64) {
    let mut w = World::new(Rect::new(0, 0, 255, 199));
    let soil = w.materials.id_of("soil").expect("soil");

    // **Curvature has to be separated from elevation, and the first version
    // of this scene did not do it.** It compared a ridge crest against a
    // plain 30 rows lower, found the crest read a *third* of the plain, and
    // could not say whether that was convexity or simply being further from
    // the bulk of the wet ground. So the bed now carries three surfaces in
    // one elevation band, with the low plain kept only for scale:
    //
    //   x   0.. 99  flat plain, surface y=120         -- the low reference
    //   x 100..145  flat plateau, surface y=90        -- flat, high
    //   x 150..200  triangular ridge, peak y=90       -- convex, high
    //   x 210..250  plateau with a V notch cut into it -- concave, high
    //
    // The three at y=90 differ **only** in the shape of the ground, which is
    // what makes a comparison between them a curvature reading rather than
    // an altitude one.
    let surface = |x: i32| -> i32 {
        if (100..=145).contains(&x) {
            90
        } else if (150..=200).contains(&x) {
            90 + (x - 175).abs()
        } else if (210..=250).contains(&x) {
            90 + (20 - (x - 230).abs() * 2).max(0)
        } else {
            120
        }
    };
    for x in 0..=255 {
        for y in surface(x)..=160 {
            // Built at field capacity, which is how `LabBox` builds its bed
            // -- a probe on drier soil would be measuring a different world
            // from the one the lab arm reads.
            w.set(x, y, Cell::new(soil, 0).with_aux(material::SOIL_FIELD_CAPACITY));
        }
    }
    // Let the field settle; a gradient read on frame zero is a reading of
    // the initialiser. Measured: this scene is still moving at 600 frames
    // and is identical at 2,000 and 6,000, so the default is 2,000 and
    // shorter runs are a knob for checking that rather than a saving.
    let (mut particles, mut blasts, tuning) = (ParticleSystem::default(), Blasts::default(), player::Tuning::default());
    for _ in 0..settle {
        frame::step(&mut w, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
    }

    // **The raw field beside the derived number.** A gradient of 0.05 is
    // consistent with "the channel is flat here" and with "the channel is
    // steep and the divisor is large", and those are opposite findings.
    // `CLAUDE.md`: pair every debug channel with a probe that prints the
    // values. The dx/dy split is the load-bearing column -- a reading that
    // is all dy is the air/soil step, which every surface in the world has,
    // and is not curvature at all.
    // **The offset sweep, and it is the reason this probe exists rather than
    // one assertion.** The shipped sampler reads `±4` cells and `FIELD_SCALE`
    // is 16: those offsets were chosen when a block was 8 cells (a full block
    // across, a sensible span) and `ca7e9042` doubled the field on
    // 2026-08-30, one day after the lines were last touched. Nobody
    // re-derived them. So the question is not only *does this read
    // curvature* but *would it at a span matched to the field it samples*,
    // and the second cannot be answered by one number.
    //
    // The `±4` row is the **shipped** `creature::moisture_gradient` and is
    // asserted against the local arithmetic, so the sweep is anchored to the
    // engine rather than being a private model of it. Every other row is a
    // hypothetical.
    let grad_at = |w: &World, x: i32, y: i32, off: i32| -> (f32, f32, f32) {
        let m = |px: i32, py: i32| w.field_at_bilinear(px as f32, py as f32).moisture;
        let gx = m(x + off, y) - m(x - off, y);
        let gy = m(x, y + off) - m(x, y - off);
        ((gx * gx + gy * gy).sqrt(), gx, gy)
    };
    let sample = |w: &World, x: i32, y: i32, label: &str| -> Vec<f32> {
        let mut out = Vec::new();
        for off in [4, 8, 16, 24] {
            let (mag, gx, gy) = grad_at(w, x, y, off);
            let shipped = if off == 4 { format!("{:.4}", creature::moisture_gradient(w, x, y)) } else { "     -".into() };
            println!("  {label:<16} at ({x:>3},{y:>3})  +-{off:<3} |grad| {mag:.4}  dx {gx:+.4}  dy {gy:+.4}   shipped {shipped}");
            out.push(mag);
        }
        out
    };
    println!("\ncontrol / moisture: one cell above ground, three curvatures in one elevation band");
    println!("  (dx is the only column curvature could move; dy is the air/soil step every surface has)");
    let plain = sample(&w, 60, 119, "flat plain (low)");
    let plateau = sample(&w, 122, 89, "flat plateau");
    let crest = sample(&w, 175, 89, "convex crest");
    let notch = sample(&w, 230, 109, "concave notch");

    // **Read the magnitude, because the magnitude is what the mechanism
    // consumes.** `moisture_gradient` returns `|grad| / saturation`, and the
    // drop bias multiplies straight into a probability -- so a difference
    // that does not reach the magnitude cannot reach behaviour, whatever it
    // does to a component.
    //
    // **Do not read the `dx` column as the answer, and this cost a wrong
    // verdict before it was caught.** An earlier version of this control put
    // the bar on `|dx|` and reported a clean 0.09x separation at every span,
    // which looks like a strong result and is an artifact twice over: the
    // apex of a *symmetric* ridge has `dx = 0` by symmetry rather than by
    // dryness, and the flat reference at x=122 sits 22 cells from its
    // plateau's edge, so it picks up a lateral gradient the wide plain does
    // not. Two scene artifacts, one tidy number -- `CLAUDE.md`'s tell.
    //
    // **The instrument's own positive control comes first**: depth must move
    // the reading, or a flat curvature result is a statement about the probe.
    let depth_ratio = plain[0] / plateau[0].max(1e-9);
    println!("\ncontrol / moisture:  does this channel move at all? low plain {:.4} against high plateau {:.4} = {depth_ratio:.2}x", plain[0], plateau[0]);
    println!(
        "control / moisture:  {}  -- the instrument must be able to move before a null means anything",
        if depth_ratio >= 1.5 || depth_ratio <= 0.667 { "PASS" } else { "FAIL -- the probe reads the same everywhere; nothing below is interpretable" }
    );

    // **Convex against flat at one elevation is the only clean curvature
    // comparison in this scene.** The notch's floor is 20 rows lower than
    // the plateau, so anything it shows is confounded with depth -- and
    // measured, that is exactly what it is.
    println!("\ncontrol / moisture:  convex crest against flat plateau, SAME elevation -- |grad| by sample span");
    let mut any = false;
    for (i, off) in [4, 8, 16, 24].iter().enumerate() {
        let ratio = crest[i] / plateau[i].max(1e-9);
        let moved = !(0.667..=1.5).contains(&ratio);
        any |= moved;
        println!("  +-{off:<3} crest {:.4}  flat {:.4}  ratio {ratio:.3}x  {}", crest[i], plateau[i], if moved { "separates" } else { "FLAT" });
    }
    println!("  concave notch, for reference, is {:.2}x the flat plateau -- and it is 20 rows lower, so that is depth", notch[0] / plateau[0].max(1e-9));
    println!(
        "control / moisture:  {}",
        if any {
            "PASS -- curvature moves this channel at some sample span"
        } else {
            "FAIL -- curvature does not move this channel at ANY sample span, in a scene built to have curvature. What it returns is the air/soil step, which every surface has: the drop bias is a DEPTH bias, not a curvature bias, and no widening of the sampler fixes that."
        }
    );

    // Two ants on bare floor, well inside `eye`.
    let mut w2 = World::new(Rect::new(0, 0, 255, 199));
    for x in 0..=255 {
        w2.set(x, 101, Cell::new(material::STONE, 0));
    }
    w2.plant_ant(60, 100);
    w2.plant_ant(80, 100);
    let a = w2.get(60, 100).organism_id();
    assert_ne!(a, 0, "the control placed no ant; the scene is wrong, not the sense");
    let def = ant_with_eye(&w2, eye);
    let (seen, reads) = creature::sighted(&w2, 60, 100, a, &def);
    println!("control / kin:       one cast at reach {eye} read {reads} cells on bare open floor -- the operand `sight_fraction` is priced against");
    println!(
        "control / kin:       open floor at 20 cells -> {}",
        seen.kin.map_or("nothing".to_string(), |k| format!("kin at {:.1} cells", k.dist))
    );
    println!("control / kin:       {}  -- an ant must see a nestmate across a bare floor", if seen.kin.is_some() { "PASS" } else { "FAIL (the kin ray finds nothing at all; 0c is void)" });

    // And the negative control on the same pair: a full-height wall between
    // them must take it away, or the sense is not occluded and "invisible
    // underground" could never be measured.
    for cy in 80..101 {
        w2.set(70, cy, Cell::new(material::STONE, 0));
    }
    let (walled, _) = creature::sighted(&w2, 60, 100, a, &def);
    println!("control / kin:       {}  -- a stone wall between them must take it away", if walled.kin.is_none() { "PASS" } else { "FAIL (rays pass through rock; occlusion is off)" });
}

/// The real bed: a `LabBox` with a colony in it, run until galleries exist,
/// every ant classified by whether it has ground over its head.
fn lab(frames: u64, seeds: u64, eye: i32) {
    for seed in 1..=seeds {
        let spec = LabBox { colonies: 1, founders: 0, seed, ..LabBox::default() };
        let mut world = spec.build();
        let (mut particles, mut blasts, tuning) = (ParticleSystem::default(), Blasts::default(), player::Tuning::default());
        for _ in 1..=frames {
            frame::step(&mut world, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        }

        let def = ant_with_eye(&world, eye);
        let (mut surface, mut under) = (Census::default(), Census::default());
        for id in world.live_organism_ids() {
            let Some(state) = world.organism(id) else { continue };
            if world.species.get(state.species).creature.is_none() {
                continue;
            }
            let Some(&(hx, hy)) = state.chain.first() else { continue };
            let grad = creature::moisture_gradient(&world, hx, hy);
            let (seen, reads) = creature::sighted(&world, hx, hy, id, &def);
            let kin = seen.kin.map(|k| k.dist);
            if is_under_cover(&world, hx, hy, spec.ground_y) { &mut under } else { &mut surface }.add(grad, kin, reads);
        }
        println!("\nseed {seed} at frame {frames}: {} ants standing", surface.n + under.n);
        header();
        println!("{}", surface.row("surface"));
        println!("{}", under.row("covered"));
    }
}

fn main() {
    let mode = arg::<String>("mode").unwrap_or_else(|| "control".into());
    let frames: u64 = arg("frames").unwrap_or(9_000);
    let seeds: u64 = arg("seeds").unwrap_or(2);
    // 32, not the beetle's 64: §4 argues a shorter range is both cheaper and
    // more honest for an ant, and `foraging-range-measurement.md` puts real
    // excursions at 12-19 cells.
    let eye: i32 = arg("eye").unwrap_or(32);
    // How long the control's hand-built world runs before it is read. A
    // gradient read on frame zero is a reading of the initialiser.
    let settle: u64 = arg("settle").unwrap_or(600);
    echo(&mode, frames, seeds, eye, settle);

    match mode.as_str() {
        "control" => control(eye, settle),
        "lab" => {
            control(eye, settle);
            lab(frames, seeds, eye);
        }
        other => panic!("unknown mode={other}; try control or lab"),
    }
}
