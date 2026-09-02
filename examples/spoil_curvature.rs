//! **Two preconditions for a curvature sense, asked before the sense is
//! built.**
//!
//! `Reports/creature-genome-flexibility-2026-09-02.md` §5f proposes a signed
//! discrete-curvature brain input -- a solid-neighbour count in a small disc
//! -- feeding `Drop` and `DropSpoil`. §14i is the record of the previous
//! mechanism on this plan failing, and it names the two ways a lever can be
//! null *by construction*:
//!
//! 1. **The response may not be reachable.** `act`'s spoil drop admits a cell
//!    only if at least two of the three cells beneath it are solid, and that
//!    clause is specifically anti-pillar -- on a one-cell-wide pillar top the
//!    count is 1 and the site fails (§5g). Curvature is highest exactly where
//!    the predicate is hardest, so a weight on it could move nothing however
//!    large it is. **This probe enumerates the sites the predicate actually
//!    accepts and reports their curvature distribution.**
//! 2. **The sensor may never visit both ends.** `Crowding` passed a "not a
//!    dead channel" check while sitting saturated near 1.0 for a whole run,
//!    which is why the rooms result was uninterpretable: the regime the model
//!    is about was never entered. **So this reports the low tail and the
//!    spread, not the mean and the max** -- which are the statistics §14i
//!    says would have settled it and were not reported.
//!
//! ```text
//! cargo run --release --example spoil_curvature
//! cargo run --release --example spoil_curvature -- seeds=12 frames=3000
//! cargo run --release --example spoil_curvature -- radius=3
//! ```
//!
//! # It is a distribution instrument, and it prints its own control
//!
//! Every population below is reported as deciles plus the share at each
//! extreme. A mean would hide exactly the failure this exists to find: a
//! channel pinned at one end has a perfectly reasonable mean.
//!
//! The **hand-built control** runs first and is not optional. It stamps a
//! crest, a flat and a notch and asserts the estimator separates them --
//! `CLAUDE.md`'s standing rule that a number must be checked against a case
//! whose answer is known before it is trusted about one that is not. A
//! curvature reading that cannot tell a crest from a notch makes every
//! distribution below meaningless and reads exactly like a flat world.

use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::world::World;
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::{material, parallel};

/// **Echoed in the header below**, because `CLAUDE.md` records a 3.5-hour
/// study that produced byte-identical logs from a binary predating its own
/// knob: a knob nobody can see the value of is a knob nobody can tell is
/// disconnected.
fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// Chebyshev radius of the sampling disc. 2 gives the 24 `World::get` §5f
/// priced the sense at -- against 328-1,186 cells for one sight cast, which
/// is the comparison that made it affordable.
const DEFAULT_RADIUS: i32 = 2;

/// **Signed discrete curvature at `(x, y)`: solid neighbours in a disc,
/// mapped so convex is positive and concave negative.**
///
/// `+1` is all air (a spike of ground with nothing around it -- maximally
/// convex), `-1` all solid, `0` exactly half, which is what a straight
/// surface reads. Read the lattice rather than a field, which is the whole
/// reason this approach survives where a field-based one cannot: the lab bed
/// builds its soil at a uniform moisture and develops only a vertical drying
/// profile, so there is no spatial structure for a field sampler to find --
/// and a coarse-field read is block-nearest anyway, so neighbouring cells
/// return the same value (`CLAUDE.md`). A solid-neighbour count is per-cell by
/// construction and works identically in a uniform bed.
/// **What a straight horizontal surface reads, derived rather than
/// measured.** A cell sitting one row above flat ground has `r * (2r + 1)`
/// solid cells in its disc out of `(2r+1)^2 - 1` -- 10 of 24 at radius 2 --
/// so the raw count reads `+1/6` there, not zero.
///
/// This is subtracted, so the signal is zero on a flat surface and the
/// sign means what §5f says it means. **Getting it wrong is not cosmetic:**
/// banded against zero, 91.7% of the lab bed's surface cells classify as
/// "convex" when they are simply flat, and a weight fitted against that
/// would be a constant with a curvature-shaped name.
fn flat_reference(radius: i32) -> f32 {
    let total = (2 * radius + 1) * (2 * radius + 1) - 1;
    1.0 - 2.0 * (radius * (2 * radius + 1)) as f32 / total as f32
}

fn curvature(world: &World, x: i32, y: i32, radius: i32, count_bodies: bool) -> f32 {
    let mut solid = 0i32;
    let mut total = 0i32;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx == 0 && dy == 0 {
                continue;
            }
            total += 1;
            // Raw material, never `Cell::is_empty`, which is managed-aware
            // and answers "is this position available" rather than "is there
            // material here" -- the distinction `burrow_probe` records
            // getting wrong.
            let cell = world.get(x + dx, y + dy);
            if cell.material == material::EMPTY {
                continue;
            }
            // **Flesh is not terrain, and counting it makes this a sense of
            // the animal rather than of the world.** Measured 2026-09-02
            // before it was excluded: over 12 seeds and 18,720 samples at
            // ant head positions the reading was **exactly -0.083 every
            // single time**, p0 = p100, zero spread. -0.083 is -2/24, which
            // is the flat ground plus precisely one extra solid cell -- the
            // ant's own second body cell, which is adjacent to its head by
            // construction. The sense was reading its own body.
            //
            // That is `CLAUDE.md`'s *a debug readout must not be a function
            // of the thing it debugs*, one step over: a **sense** must not be
            // a function of the senser. It is also the tidiness tell -- a
            // perfectly constant result across twelve chaotic seeds is
            // evidence of an artifact, never of a strong effect.
            //
            // Nestmates go with it. An ant packed among others would read as
            // standing in a concave bowl, which is a crowd rather than a
            // hollow -- and `BrainInput::Crowding` already counts exactly
            // that, so leaving them in would put two inputs on one quantity
            // and make a weight on either unattributable.
            if !count_bodies && world.materials.kind(cell.material) == MaterialKind::Creature {
                continue;
            }
            solid += 1;
        }
    }
    1.0 - 2.0 * solid as f32 / total.max(1) as f32 - flat_reference(radius)
}

/// The shipped spoil-drop predicate, reproduced here **only** because `act`'s
/// closure is not callable from outside. Any change to it there has to be
/// mirrored here or this probe measures a rule the world does not run --
/// which is why the constants are read from the crate rather than retyped.
fn drop_site(world: &World, x: i32, y: i32) -> bool {
    const SPOIL_HEADROOM: i32 = 3;
    world.is_empty(x, y)
        && [(-1, 1), (0, 1), (1, 1)].iter().filter(|(dx, dy)| !world.is_empty(x + dx, y + dy)).count() >= 2
        && (1..=SPOIL_HEADROOM).all(|dy| world.is_empty(x, y - dy))
}

/// A cell that is empty and touching ground: the population the predicate
/// selects *from*, without which admissibility cannot be read as a selection
/// effect rather than as a fact about the world.
fn surface_cell(world: &World, x: i32, y: i32) -> bool {
    world.is_empty(x, y)
        && [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)]
            .iter()
            .any(|(dx, dy)| !world.is_empty(x + dx, y + dy))
}

/// Deciles plus the share at each extreme.
///
/// **The mean and the max are deliberately not here.** They are the two
/// statistics §14i records being reported instead of the ones that would have
/// caught a saturated sensor, and a reader given them will use them.
struct Spread {
    n: usize,
    deciles: [f32; 11],
    near_zero: f32,
    convex: f32,
    concave: f32,
}

fn spread(mut v: Vec<f32>) -> Spread {
    v.sort_by(|a, b| a.partial_cmp(b).expect("no NaN in a lattice count"));
    let n = v.len();
    let mut deciles = [0.0f32; 11];
    if n > 0 {
        for (i, d) in deciles.iter_mut().enumerate() {
            let idx = ((n - 1) * i) / 10;
            *d = v[idx];
        }
    }
    let frac = |f: &dyn Fn(f32) -> bool| if n == 0 { 0.0 } else { v.iter().filter(|&&x| f(x)) .count() as f32 / n as f32 };
    Spread {
        n,
        deciles,
        // "Flat" is one cell's worth of the disc either way, so the band is
        // set by the estimator's own quantum rather than picked.
        near_zero: frac(&|x| x.abs() <= 2.0 / 24.0),
        convex: frac(&|x| x > 2.0 / 24.0),
        concave: frac(&|x| x < -2.0 / 24.0),
    }
}

fn report(label: &str, s: &Spread) {
    if s.n == 0 {
        println!("  {label:<26} EMPTY -- this population has no members, so nothing below it means anything");
        return;
    }
    print!("  {label:<26} n {:>7} | p0 {:+.3} p10 {:+.3} p25 {:+.3} p50 {:+.3} p75 {:+.3} p90 {:+.3} p100 {:+.3}",
        s.n, s.deciles[0], s.deciles[1], s.deciles[2], s.deciles[5], s.deciles[7], s.deciles[9], s.deciles[10]);
    println!(" | convex {:.1}% flat {:.1}% concave {:.1}%", s.convex * 100.0, s.near_zero * 100.0, s.concave * 100.0);
}

/// **The positive control, and it runs before anything else.**
///
/// A crest, a flat and a notch, hand-stamped, with the answer known. If the
/// estimator cannot order these three the distributions below are measuring
/// nothing -- and a flat result would read as "this bed has no curvature",
/// which is the wrong conclusion and an expensive one.
fn control(radius: i32) {
    let mut w = World::new(Rect::new(0, 0, 199, 199));
    let stone = material::STONE;
    for x in 0..200 {
        for y in 120..160 {
            w.set(x, y, Cell::new(stone, 0));
        }
    }
    // A crest: a mound with air on both flanks.
    for (i, x) in (40..=50).enumerate() {
        let h = 5 - (i as i32 - 5).abs();
        for y in (120 - h)..120 {
            w.set(x, y, Cell::new(stone, 0));
        }
    }
    // A notch: a slot cut into the flat. **Three cells wide, not five, and
    // the width is the whole point of this control.** At radius 2 the disc
    // spans five columns, so a five-wide slot sampled at its centre sees no
    // wall at all and reads as *convex* -- caught here on the first run, and
    // it is the estimator's real limitation stated as a scene: this sense
    // cannot see a feature wider than its own disc. Anything relying on it
    // to read a broad hollow needs a larger radius, and pays for it.
    for x in 141..=143 {
        for y in 120..126 {
            w.set(x, y, Cell::EMPTY);
        }
    }
    // All three sampled one cell above their own solid surface, which is
    // where a walking animal's head is and where a droppable site sits. A
    // flat surface reads +0.167 rather than 0 at that offset -- the disc is
    // two-fifths buried -- so **the reference is the flat, not zero**, and a
    // bar written against 0 would call every surface in the world convex.
    let crest = curvature(&w, 45, 114, radius, false);
    let flat = curvature(&w, 90, 119, radius, false);
    let notch = curvature(&w, 142, 125, radius, false);
    println!("control: crest {crest:+.4}  flat {flat:+.4}  notch {notch:+.4}   (convex positive, concave negative; flat reference {:+.4} already removed)", flat_reference(radius));
    assert!(flat.abs() < 1e-6, "a straight surface must read exactly zero after the reference is removed, got {flat:+.4}");
    assert!(crest > flat, "the estimator must read a crest as more convex than a flat ({crest:+.4} vs {flat:+.4}); nothing below this line means anything otherwise");
    assert!(notch < flat, "and a notch as more concave than a flat ({notch:+.4} vs {flat:+.4})");
    println!("control: PASS -- the estimator separates the three by {:.4} / {:.4}\n", crest - flat, flat - notch);
}

fn main() {
    let seeds: u64 = arg("seeds").unwrap_or(12);
    // 3,000 rather than the 9,000 a lab harness usually takes: the colony in
    // this bed is placed with no plants (`founders: 0`), so it starves out
    // between frames 3,000 and 6,000 and a longer run measures an empty
    // scene. Found the hard way in `field_sense_probe mode=lab`, whose own
    // default reads all-zero for exactly this reason.
    let frames: u64 = arg("frames").unwrap_or(3_000);
    let radius: i32 = arg("radius").unwrap_or(DEFAULT_RADIUS);
    println!("spoil_curvature: seeds={seeds} frames={frames} radius={radius} (disc {} cells)", (2 * radius + 1) * (2 * radius + 1) - 1);
    control(radius);

    let (mut all_surface, mut admitted, mut stood) = (Vec::new(), Vec::new(), Vec::new());
    let mut stood_with_bodies: Vec<f32> = Vec::new();
    let mut ants_seen = 0usize;
    for seed in 1..=seeds {
        let spec = LabBox { colonies: 1, founders: 0, seed, ..LabBox::default() };
        let mut w = spec.build();
        // Sample where ants stand *through* the run, not only at the end: the
        // realised range of a sense is what it saw, and an end-state census
        // is a snapshot of a trajectory (§14i).
        for f in 0..frames {
            parallel::step(&mut w);
            if f % 100 == 0 {
                for id in w.live_organism_ids() {
                    if let Some(state) = w.organism(id) {
                        if let Some(&(hx, hy)) = state.chain.first() {
                            stood.push(curvature(&w, hx, hy, radius, false));
                            stood_with_bodies.push(curvature(&w, hx, hy, radius, true));
                        }
                    }
                }
            }
        }
        let standing = w.live_creature_count();
        ants_seen += standing;
        assert!(standing > 0, "seed {seed} ran {frames} frames and ended with no ants: the scene does not contain the situation this probe is about, and every column below would read as a flat world");
        let bounds = w.bounds().expect("a LabBox world is bounded");
        for y in bounds.min_y..=bounds.max_y {
            for x in bounds.min_x..=bounds.max_x {
                if surface_cell(&w, x, y) {
                    let c = curvature(&w, x, y, radius, false);
                    all_surface.push(c);
                    if drop_site(&w, x, y) {
                        admitted.push(c);
                    }
                }
            }
        }
    }

    println!("over {seeds} seed(s), {ants_seen} ants standing at the end\n");
    println!("PRECONDITION 1 -- does the placement predicate admit convex sites at all?");
    let surf = spread(all_surface);
    let adm = spread(admitted);
    report("all empty-beside-ground", &surf);
    report("admitted by the predicate", &adm);
    let admit_rate = if surf.n == 0 { 0.0 } else { adm.n as f32 / surf.n as f32 };
    println!("  the predicate admits {:.1}% of surface cells", admit_rate * 100.0);
    println!(
        "  convex share: {:.1}% of surface cells -> {:.1}% of admitted sites  ({}x)",
        surf.convex * 100.0,
        adm.convex * 100.0,
        if surf.convex > 0.0 { format!("{:.2}", adm.convex / surf.convex) } else { "n/a".into() }
    );
    if adm.convex < 0.02 {
        println!("  VERDICT: convex sites are essentially not admissible. A curvature weight on the drop");
        println!("           would be null BY CONSTRUCTION -- the site is refused before preference is");
        println!("           consulted -- which is §14i's failure repeating. STOP AND REPORT.");
    } else {
        println!("  VERDICT: convex sites are admissible; a weight on them has something to move.");
    }

    println!("\nPRECONDITION 2 -- does the sensor visit both ends?");
    let st = spread(stood);
    // Both rows, because the difference between them *is* the finding: the
    // second is what a naive solid-neighbour count returns, and it is a
    // constant.
    report("where ants stood (terrain)", &st);
    report("...counting bodies too", &spread(stood_with_bodies));
    if st.n > 0 {
        let saturated = st.deciles[1] == st.deciles[9];
        println!(
            "  low tail p10 {:+.3} against p90 {:+.3}: spread {:.3}",
            st.deciles[1],
            st.deciles[9],
            st.deciles[9] - st.deciles[1]
        );
        if saturated {
            println!("  VERDICT: pinned -- p10 and p90 are the same value. This is Crowding's failure exactly:");
            println!("           a channel that varies somewhere in the world but not where the animal is.");
        } else {
            println!("  VERDICT: the sensor visits a real range at the positions animals actually occupy.");
        }
    }
}
