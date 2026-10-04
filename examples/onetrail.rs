//! **One ant, one standing trail, nothing else: does it walk the trail?**
//!
//! The bluntest possible form of the stigmergy question. `trailfollow
//! mode=colony` asks it of a *colony* in a lab bed, and `pherolife` asks how
//! long a trail lives; neither can answer "put one animal on a trail that
//! cannot fade and see whether it commutes", because in both of them decay,
//! traffic, food, nestmates and the colony's own deposits are all still live.
//! This removes every one of them.
//!
//! ```text
//! cargo run --release --example onetrail                 # arithmetic, then the bed
//! cargo run --release --example onetrail -- mode=arith   # the arithmetic alone
//! cargo run --release --example onetrail -- seeds=24 frames=6000
//! cargo run --release --example onetrail -- mode=stream lay=odo t=32   # the food trail a stream of returns SHOULD leave
//! ```
//!
//! # What is removed, and why each one had to go
//!
//! * **Decay and diffusion.** `World::step_pheromones` is never called, so
//!   the plane is exactly what was stamped into it, for ever. `set_channel_rho(0.0)`
//!   would *not* have done this: `build_decay_lut` forces every nonzero cell
//!   strictly downward whatever `rho` says, which is the LUT floor
//!   `pheromone.rs` documents, so a rho of zero still empties the plane.
//! * **The ant's own deposits.** Every weight into `EmitA` and `EmitB` is
//!   zeroed, so the animal cannot write to the plane it is being tested on.
//!   Without this the trail under test is partly the ant's own, and a
//!   following ant and a self-reinforcing one look identical from outside.
//! * **Starvation.** `start_energy` is raised out of reach, so `Energy` --
//!   which the engine computes as `energy / start_energy` -- sits at 1.0 for
//!   the whole run and the ant cannot starve. `ant.ron` authors
//!   `(Energy, Move, -1.75)` against a `(Bias, Move, 2.0)`, so a starving ant
//!   walks nearly three times as often as a fed one; an unpinned run measures
//!   hunger as much as trail-reading, and the drift is monotone, so it aliases
//!   straight into net displacement.
//! * **Food, and carrying it.** The `Carrying` input is not a sense of the
//!   world, it is the **gate deciding which channel the animal reads at all**
//!   -- units 0/1 (channel A) open on a full crop, units 2/3 (channel B) on an
//!   empty one. Rather than put food in the world and wait for the ant to pick
//!   it up, the arm holds the gate pair at its full-crop value directly, by
//!   folding the authored `Carrying` weight into the unit's `Bias` and zeroing
//!   it. The unit then sits at exactly the sum it would reach with a full
//!   crop, for an animal that has never seen food -- so the channel under test
//!   is selected without a grain of anything else entering the world.
//! * **Reproduction.** `reproduce_threshold` is set out of reach, so the
//!   population stays at one. A birth would put a second animal on the trail
//!   and `Crowding` into the `Move` sum.
//! * **Everything else in the world.** Bare stone floor, air above. No soil,
//!   no water, no nest material, no food, no plants, no weather, no colony.
//!
//! # The controls, which are the point
//!
//! A net displacement toward the peak proves nothing on its own -- the sweep
//! is chunked left-to-right and the movement rules are not symmetric, so a
//! scene can walk an animal downhill for reasons that have nothing to do with
//! scent (`CLAUDE.md`: a scene that contradicts the code will look like a bug
//! in the code). So every arm is run **mirrored**: the identical ramp with its
//! peak at the left, from the identical start. Trail-following is the
//! *difference* between the two, and a scene bias cancels out of it exactly.
//!
//! Beside that, `shape=flat` is the conceptual control and it is worth
//! stating plainly: the reader is `BrainInput::PheroAAlong`, a *difference*
//! between the cell ahead and the cell underfoot. A trail of uniform
//! concentration is invisible to it by construction, however strong. Only a
//! ramp is followable at all, which is why the shipped odometer lays one.
//!
//! # The arithmetic comes first, on the *unmodified* genome
//!
//! `trailfollow mode=arith` prints the same table, but every one of its
//! presets overwrites **both** gate pairs, and the shipped ant is a mix --
//! units 0/1 (channel A, gated on a full crop) carry the 2026-09-09 `b2`
//! re-weighting and units 2/3 (channel B, gated on an empty one) do not. So
//! no row of that table is the animal in the tree. This reads the genome as
//! `ant.ron` actually authors it and overrides nothing.

use pixel_physics::sim::brain::{self, BrainInput as I, BrainOutput as O};
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::material;
use pixel_physics::sim::pheromone::{self, Channel};
use pixel_physics::sim::{parallel, Cell, World};

fn arg<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{name}=")).map(|v| v.parse().ok().expect("parses")))
        .unwrap_or(default)
}
fn arg_str(name: &str, default: &str) -> String {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{name}=")).map(|v| v.to_string())).unwrap_or_else(|| default.into())
}

/// The trail's shape along x, in `Scent` units, as a function of `t` in 0..1
/// running left to right. `peak` is in old-u8 units so it reads against
/// `pheromone::DEPOSIT`'s 40.
#[derive(Clone, Copy, PartialEq)]
enum Shape {
    /// Climbing toward whichever end `mirror` puts the peak at.
    Ramp,
    /// Uniform at half the peak. Unfollowable by construction — the control.
    Flat,
    /// Nothing stamped at all.
    None,
}

/// One run's readout. Every field is a count or a measured quantity; nothing
/// here is a verdict.
struct Run {
    /// Signed displacement **toward the peak**, in cells. Mirroring flips the
    /// sign convention with the ramp, so both arms are directly comparable.
    toward_peak: i32,
    /// Furthest the head ever got toward the peak, from the start.
    reach: i32,
    /// Ticks the head spent inside the stamped band.
    on_band: u64,
    /// Decision ticks the ant was alive for.
    ticks: u64,
    /// Mean `PheroAAlong`/`PheroBAlong` the ant actually computed, by the
    /// same arithmetic `creature::sense` uses — **guard `SCALE`, not 1.0**.
    /// The instrument's positive control: a count that did not move against a
    /// gradient that was never there says nothing.
    ///
    /// **Read this column on the arms that did NOT move, and nowhere else.**
    /// An ant that follows the trail parks at the summit, where the forward
    /// sensor is sampling off the end of the stamped band and reads a cliff,
    /// so a successful arm's mean goes *negative* (-0.69 at `span=64`) and is
    /// a statement about where the animal ended up rather than about what the
    /// trail offered it. The number that matters is the stalled arm's, which
    /// is what says the gradient was there and being read.
    along: f64,
    /// Mean `P(move)` over the run, from `CreatureStats::p_move_hist`.
    p_move: f64,
    /// Trail cells still holding the stamped value at the end. The
    /// non-degradation check: this must equal what was stamped.
    intact: bool,
}

/// Stamp the plane once. Returns the (x0, x1) the trail spans.
// Eight arguments against clippy's ceiling of seven: the trail's two ends and
// the row the head walks are three of them, and bundling them into a struct
// would hide that this is the only place the geometry is decided.
#[allow(clippy::too_many_arguments)]
fn stamp(w: &mut World, ch: Channel, shape: Shape, mirror: bool, x0: i32, x1: i32, head_y: i32, peak: f32) {
    if shape == Shape::None {
        return;
    }
    for x in x0..=x1 {
        let t = (x - x0) as f32 / (x1 - x0) as f32;
        let t = if mirror { 1.0 - t } else { t };
        let amount = match shape {
            Shape::Ramp => t * peak * pheromone::SCALE as f32,
            Shape::Flat => 0.5 * peak * pheromone::SCALE as f32,
            Shape::None => 0.0,
        };
        // A band rather than a row: an ant walking a cell or two off the
        // nominal surface still has to be reading the trail, or the arm
        // measures footing rather than scent.
        for y in (head_y - 3)..=(head_y + 1) {
            w.deposit_pheromone(ch, x, y, amount as pheromone::Scent);
        }
    }
}

/// Hold hidden units `units` at the value they would reach with a **full
/// crop**, for an animal that is carrying nothing: fold the authored
/// `Carrying` weight into the unit's `Bias` and zero it. Reads the authored
/// numbers rather than restating them, so it stays correct if `ant.ron`
/// retunes the gate.
fn hold_gate_laden(g: &mut [f32], units: [usize; 2]) {
    for u in units {
        let carry = brain::ih_slot(I::Carrying, u);
        let bias = brain::ih_slot(I::Bias, u);
        g[bias] += g[carry];
        g[carry] = 0.0;
    }
}

/// **The positive control, and the one arm that says what the null is made
/// of.** Give units 2/3 -- channel B, the pair an *empty* ant reads -- the
/// same open-gate value the 2026-09-09 `b2` re-weighting gave units 0/1, by
/// copying it rather than restating it: units 0/1's open sum is
/// `Bias + Carrying` (the full-crop state), and units 2/3 open at `Bias`
/// alone (the empty-crop state), so the fix is one assignment per unit.
///
/// If the empty ant follows the trail with this in and not without it, the
/// null is the gate weights and nothing else -- not the plane, not the
/// sensor, not the geometry, not the animal.
fn regate_channel_b(g: &mut [f32]) {
    let open = g[brain::ih_slot(I::Bias, 0)] + g[brain::ih_slot(I::Carrying, 0)];
    for u in [2usize, 3] {
        g[brain::ih_slot(I::Bias, u)] = open;
    }
}

/// Zero every weight into `EmitA` and `EmitB`, direct and via the hidden
/// layer. Returns how many slots moved, so a silent no-op is visible.
fn silence_emission(g: &mut [f32]) -> usize {
    let mut moved = 0;
    for out in [O::EmitA, O::EmitB] {
        for i in 0..brain::BRAIN_INPUTS {
            let slot = brain::io_slot(brain::INPUTS[i], out);
            if g[slot] != 0.0 {
                g[slot] = 0.0;
                moved += 1;
            }
        }
        for h in 0..brain::BRAIN_HIDDEN {
            let slot = brain::ho_slot(h, out);
            if g[slot] != 0.0 {
                g[slot] = 0.0;
                moved += 1;
            }
        }
    }
    moved
}

#[allow(clippy::too_many_arguments)]
fn run(seed: u64, ch: Channel, shape: Shape, mirror: bool, laden: bool, regate: bool, frames: u64, peak: f32, w_cells: i32, span: i32, decay: bool, flip: bool, start_off: i32) -> Run {
    // `laden` selects the channel by holding the gate, not by filling a crop.
    let (w_world, h) = (w_cells, 64i32);
    let mut world = World::new(Rect::new(0, 0, w_world - 1, h - 1));
    world.seed = seed;
    let floor = h - 8;
    let head_y = floor - 1;

    // Bare stone slab, and nothing else in the world at all.
    for x in 0..w_world {
        for y in floor..h {
            world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
        }
    }

    let species = world.species.id_of("ant").expect("the ant species is compiled in");
    {
        let mut def = world.species.get(species).creature.clone().expect("ant is a creature");
        // Out of reach, so the population cannot become two.
        def.reproduce_threshold = 1.0e30;
        // `Energy` is `energy / start_energy`, so this pins a fed ant at 1.0
        // and puts starvation out of reach of a 4,000-frame run.
        def.start_energy = 1.0e9;
        world.species.set_creature(species, def);
    }
    let def = world.species.get(species).creature.clone().expect("ant is a creature");
    let mut genome = world.species.get(species).genome.clone();
    let silenced = silence_emission(&mut genome);
    assert!(silenced > 0, "no EmitA/EmitB weight was zeroed, so the ant is still writing the plane it is being tested on");
    if laden {
        hold_gate_laden(&mut genome, [0, 1]);
    }
    if regate {
        regate_channel_b(&mut genome);
    }
    // **Descend the trail instead of ascending it.** The symmetric pair's
    // contribution is `2.5 * (squash(b + 6a) - squash(b - 6a))`, which is odd
    // in `a` for any bias, so negating both along weights negates the response
    // exactly. This is the "walk down the age gradient toward the older end,
    // which is the food" proposal, made runnable.
    if flip {
        for u in [2usize, 3] {
            let slot = brain::ih_slot(I::PheroBAlong, u);
            genome[slot] = -genome[slot];
        }
    }
    world.species.set_genome(species, genome.clone());

    // `start_off` moves the ant off the trail's midpoint. **A negative value
    // is measured from the trail's low END, not from the midpoint**, so the
    // ant starts genuinely OFF the stamped band -- which is the arm that
    // matters and which every arm in this file until now skipped, because they
    // all began standing on the trail. Measured from the midpoint it is not an
    // off-trail arm at all: at `span=224` in a 256-wide world, twenty cells
    // off centre is still comfortably inside the band, and the first version
    // of this argument made exactly that mistake.
    // **`span` is what sets the gradient, and `peak` is not.** The reader is
    // `(ahead - here) / (ahead + here + SCALE)`, which is scale-free: multiply
    // the whole trail by ten and, once it is well clear of `SCALE`, the
    // reading is unchanged. What moves it is how much of the ramp fits inside
    // one sensor offset, i.e. the span. A `peak=` sweep is the wrong knob and
    // would read as "steepness does not help" when it had never been varied.
    let mid = (w_world - 1) / 2;
    let half = (span / 2).min(w_world / 2 - 9);
    let (x0, x1) = (mid - half, mid + half);
    let start_x = if start_off < 0 { x0 + start_off } else { mid };
    assert!(start_x > 0, "start is off the world; give span= room for the off-trail arm");
    stamp(&mut world, ch, shape, mirror, x0, x1, head_y, peak);
    let stamped_mid = world.pheromone_at(ch, start_x, head_y);

    world.plant_ant(start_x, head_y);

    let (mut on_band, mut ticks, mut along_sum, mut along_n) = (0u64, 0u64, 0.0f64, 0u64);
    let (mut last_x, mut best) = (start_x, 0i32);
    for _ in 0..frames {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        // NO step_pheromones: the trail does not decay, diffuse, or move.
        // `decay=on` puts it back, and is the sensitivity control for the
        // `intact` check below -- a guard whose fault cannot be restored is
        // blind rather than strong (`CLAUDE.md`).
        if decay {
            world.step_pheromones();
        }

        for id in world.live_organism_ids() {
            let Some(s) = world.organism(id) else { continue };
            if s.species != species {
                continue;
            }
            let Some(&(hx, hy)) = s.chain.first() else { continue };
            ticks += 1;
            last_x = hx;
            let signed = if mirror { start_x - hx } else { hx - start_x };
            let _ = &signed;
            best = best.max(signed);
            if hy >= head_y - 3 && hy <= head_y + 1 && hx >= x0 && hx <= x1 {
                on_band += 1;
            }
            // The along-gradient as `sense` computes it, at the real sensor
            // offset, in the direction the trail actually climbs.
            let fwd = if mirror { -def.sensor_offset } else { def.sensor_offset };
            let here = world.pheromone_at(ch, hx, hy) as f64;
            let ahead = world.pheromone_at(ch, hx + fwd, hy) as f64;
            along_sum += (ahead - here) / (ahead + here + pheromone::SCALE as f64);
            along_n += 1;
        }
    }

    let hist = world.creature_stats.p_move_hist;
    let total: u64 = hist.iter().sum();
    // Bucket 0 is the exact zero the clamp manufactures; bucket k>0 is
    // ((k-1)/10, k/10], taken at its midpoint.
    let p_move = if total == 0 {
        0.0
    } else {
        hist.iter().enumerate().map(|(k, &n)| if k == 0 { 0.0 } else { (k as f64 - 0.5) / 10.0 * n as f64 }).sum::<f64>() / total as f64
    };

    Run {
        toward_peak: if mirror { start_x - last_x } else { last_x - start_x },
        reach: best,
        on_band,
        ticks,
        along: if along_n == 0 { 0.0 } else { along_sum / along_n as f64 },
        p_move,
        intact: shape == Shape::None || world.pheromone_at(ch, start_x, head_y) == stamped_mid,
    }
}

/// `P(move)` across the along-gradient range, on the genome `ant.ron`
/// actually ships, with **nothing overridden**.
fn arithmetic(base: &[f32]) {
    let along = [0.0f32, 0.01, 0.03, 0.05, 0.1, 0.2, 0.5, 1.0];
    println!("P(move) for a fed ant (Energy 1.0), by along-gradient reading, on the SHIPPED genome:\n");
    println!("{:>26} | {}", "arm", along.iter().map(|a| format!("{a:>7.2}")).collect::<Vec<_>>().join(""));
    println!("{:->26}-+-{:->width$}", "", "", width = 7 * along.len());
    for (label, fill, slot) in [
        ("laden, channel A (units 0/1)", 1.0f32, I::PheroAAlong),
        ("empty, channel B (units 2/3)", 0.0, I::PheroBAlong),
        // The specificity control: the pair that is *shut* in this state must
        // not respond, or the gate is not a gate.
        ("empty, channel A  [gate shut]", 0.0, I::PheroAAlong),
        ("laden, channel B  [gate shut]", 1.0, I::PheroBAlong),
    ] {
        let row: Vec<String> = along
            .iter()
            .map(|&a| {
                let mut inputs = [0.0f32; brain::BRAIN_INPUTS];
                inputs[I::Bias as usize] = 1.0;
                inputs[I::Energy as usize] = 1.0;
                inputs[I::Carrying as usize] = fill;
                inputs[slot as usize] = a;
                let mut state = [0.0f32; brain::BRAIN_HIDDEN];
                let (out, _) = brain::eval_brain(base, &inputs, &mut state);
                format!("{:>7.3}", out[O::Move as usize])
            })
            .collect();
        println!("{label:>26} | {}", row.join(""));
    }
}

/// **Does sequential laying plus decay give a trail a gradient, and which way
/// does it point?** -- the owner's question of 2026-09-16, which the rest of
/// this file cannot answer because everywhere else the trail is stamped in one
/// instant and then frozen.
///
/// A laden ant lays channel B on every step of its walk **home**, so the cell
/// at the food end is laid first and has been decaying longest by the time the
/// ant arrives. Decay is monotone, so the trail it leaves is a ramp -- and the
/// ramp climbs toward the **nest**, not toward the food. That is the opposite
/// of what an empty ant looking for food needs, and it is not a tuning
/// accident: it follows from *when* the cells were written, so no deposit
/// value or decay rate can turn it around.
///
/// No ant and no brain here: one cell written per `per_cell` frames along the
/// route, the real `Pheromones::step` running throughout, then the profile and
/// the along-gradient a reader would compute at each point. The point of
/// leaving the animal out is that this is a property of the *plane*.
fn timing_mode(per_cell: u64, span: i32, peak: f32, w_cells: i32, reverse: bool) {
    let (w_world, h) = (w_cells, 64i32);
    let mut world = World::new(Rect::new(0, 0, w_world - 1, h - 1));
    let floor = h - 8;
    let head_y = floor - 1;
    for x in 0..w_world {
        for y in floor..h {
            world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
        }
    }
    let x0 = (w_world - span) / 2;
    let x1 = x0 + span - 1;
    let amount = (peak * pheromone::SCALE as f32) as pheromone::Scent;

    // Walk food -> nest, laying as we go: x0 is the food end and is written
    // first, x1 is the nest end and is written last.
    //
    // **`reverse` is the control, and it is the one that says the ramp is a
    // property of laying ORDER rather than of the scene.** The sweep runs
    // chunk by chunk left to right and the movement rules are not symmetric,
    // so "values climb toward +x" is not on its own evidence about decay. Lay
    // the identical trail nest -> food and every reading must mirror: same
    // magnitudes, opposite sign. If it does not, the profile is the harness.
    let order: Vec<i32> = if reverse { (x0..=x1).rev().collect() } else { (x0..=x1).collect() };
    for x in order {
        world.deposit_pheromone(Channel::B, x, head_y, amount);
        for _ in 0..per_cell {
            parallel::step(&mut world);
            world.step_fields();
            world.step_pheromones();
        }
    }

    let read = |x: i32| world.pheromone_at(Channel::B, x, head_y) as f64;
    let alive = (x0..=x1).filter(|&x| read(x) > 0.0).count();
    println!("  per_cell={per_cell:<3} span={span} walk={} frames | trail cells still alive at arrival: {alive} of {span}", per_cell * span as u64);

    // Eight probes evenly along the route, food end first.
    // **Every interior reading, not eight probes.** Eight probes are a
    // picture; the question "is the gradient the ant computes monotone" is
    // about all of them, and the two disagree -- see the `<=0` count.
    let interior: Vec<f64> = (x0..=(x1 - 6))
        .map(|x| {
            let (here, ahead) = (read(x), read(x + 6));
            (ahead - here) / (ahead + here + pheromone::SCALE as f64)
        })
        .collect();
    let n = interior.len().max(1) as f64;
    let mean = interior.iter().sum::<f64>() / n;
    let nonpos = interior.iter().filter(|&&g| g <= 0.0).count();
    println!(
        "    interior gradient (every x, 6-cell offset): mean {mean:+.4}  min {:+.4}  max {:+.4}  |  <=0 in {nonpos} of {} readings",
        interior.iter().cloned().fold(f64::MAX, f64::min),
        interior.iter().cloned().fold(f64::MIN, f64::max),
        interior.len()
    );

    let probes: Vec<i32> = (0..8).map(|i| x0 + i * (span - 1) / 7).collect();
    let vals: Vec<String> = probes.iter().map(|&x| format!("{:>8.0}", read(x))).collect();
    println!("    value    food->nest: {}", vals.join(""));
    // The reader's own arithmetic, looking toward the NEST. Positive means the
    // trail climbs toward the nest.
    //
    // **The last probe is the trail's own end and reads about -0.976**: the
    // 6-cell sensor samples past the stamped span into zero. It is an edge
    // artifact and it is printed rather than trimmed, because it is not
    // negligible -- it is 6 cells of however long the trail is, so 5% at
    // span=112 and **21% at span=28**, which is this bed's actual excursion
    // depth. And it sits at the nest end, which is where empty ants are.
    let grads: Vec<String> = probes
        .iter()
        .map(|&x| {
            let (here, ahead) = (read(x), read(x + 6));
            format!("{:>8.3}", (ahead - here) / (ahead + here + pheromone::SCALE as f64))
        })
        .collect();
    println!("    PheroBAlong facing nest: {}", grads.join(""));
}

/// **The food trail a stream of returning ants SHOULD leave**, on the real
/// plane -- the design-level expected profile that `trailfollow`'s live
/// `BTRAIL` rows are read against (`Reports/food-trail-plan-2026-09-29.md`
/// Stage 0c). `timing_mode` above is one walk, laid once and read once; this
/// is the standing state, because a trail an empty ant can climb has to exist
/// *between* returns, and only a stream of them says what it looks like then.
///
/// Every piece is the engine's: `Pheromones::new` with channel B's shipped
/// `DECAY_RHO` and `DIFFUSE`, a pass every `PHEROMONE_INTERVAL` frames gated
/// by `Pheromones::step` itself, the deposit landing on the head the ant
/// arrived on, and the same `(emit * DEPOSIT as f32) as Scent` truncation
/// `creature.rs` makes. What is *not* the engine's is the animal: no brain, no
/// RNG, one row, and a pace and interval set by hand -- so a disagreement with
/// the live bed is a statement about the ants, not about the plane.
///
/// `lay=const` is today's rule (every step laid at `emit`); `lay=odo` is the
/// proposed one, `emit * T / (T + ticks since pickup)`, which pays its biggest
/// deposits near the pile and is the only one of the two that can make the
/// trail rise toward the food. `dwell` is the shuffling at the door that the
/// live trace shows, and it lays wherever the odometer has got to.
fn stream_mode() {
    let span: i32 = arg("span", 90);
    let speed: f64 = arg("speed", 0.7);
    let tickframes: u64 = arg("tickframes", 6);
    let every: u64 = arg("every", 432);
    let lay = arg_str("lay", "const");
    let t: f32 = arg("t", 32.0);
    // 0.714 lays 7,311 raw; the live ant's 7,314 is an `EmitB` of 0.71426,
    // which `emit=` reproduces when a bit-level match matters.
    let emit: f32 = arg("emit", 0.714);
    let dwell: u64 = arg("dwell", 0);
    let frames: u64 = arg("frames", 24000);
    let sample: u64 = arg("sample", 500);
    let reverse = arg_str("reverse", "off") == "on";
    assert!(lay == "const" || lay == "odo", "lay= is const or odo, got {lay:?}");
    // The engine moves a head at most one cell a decision, so a faster pace
    // would be a different animal rather than a faster one.
    assert!(speed > 0.0 && speed <= 1.0, "speed= is cells per tick in (0, 1], got {speed}");
    assert!(span > 6 && tickframes > 0 && every > 0 && sample > 0 && t > 0.0, "span > 6, and tickframes, every, sample and t > 0");
    let odo = lay == "odo";

    // One row, with room all round: the plane diffuses in 2D and knows
    // nothing of ground, so the rows above and below and the cells past each
    // end are where the live trail's spill goes too. A margin the spill never
    // reaches keeps the plane's absorbing edge out of every number.
    let margin = 32i32;
    let (w, h) = (span + 2 * margin + 1, 2 * margin + 1);
    let row = margin;
    let mut plane = pheromone::Pheromones::new(Rect::new(0, 0, w - 1, h - 1));
    // `reverse` mirrors x about the middle, so the geometry, the plane's
    // edges and its tiles are the same run seen from the other side. Every
    // statistic is taken in distance-toward-the-food, so it must come out
    // identical; if it does not, the number is the harness.
    let (nest_x, food_x) = if reverse { (margin + span, margin) } else { (margin, margin + span) };
    let toward_food = (food_x - nest_x).signum();
    let (x0, x1) = (nest_x.min(food_x) - 10, nest_x.max(food_x) + 10);
    let rule = if odo { format!("odo{t}") } else { "const".to_string() };
    let laid = |age: u64| -> pheromone::Scent {
        let e = if odo { emit * (t / (t + age as f32)) } else { emit };
        (e.clamp(0.0, 1.0) * pheromone::DEPOSIT as f32) as pheromone::Scent
    };
    // Cells walked by the end of tick `age`. The loop below steps on this and
    // the echo reads it too: at speed < 1 the first step lands on tick 1, not
    // 0, so an echo priced at `laid(0)` claimed a first deposit the odometer
    // never lays (7,311 against 6,881 at t=16).
    let cells_by = |age: u64| (((age + 1) as f64 * speed) + 1e-9).floor() as i32;
    let first_tick = (0u64..).find(|&a| cells_by(a) >= 1).expect("speed > 0 steps eventually");
    let arrive_tick = (0u64..).find(|&a| cells_by(a) >= span).expect("speed > 0 arrives eventually");
    let walk_ticks = arrive_tick + 1;
    println!(
        "onetrail: mode=stream span={span} speed={speed} tickframes={tickframes} every={every} lay={lay} t={t} emit={emit} dwell={dwell} frames={frames} sample={sample} reverse={} nest={nest_x} food={food_x} y={row} interval={} deposit={}",
        if reverse { "on" } else { "off" },
        pheromone::PHEROMONE_INTERVAL,
        pheromone::DEPOSIT
    );
    println!(
        "  one ant leaves the food every {every} frames and walks {span} cells home in ~{walk_ticks} ticks, laying {} raw on its first step and {} on arrival",
        laid(first_tick),
        laid(arrive_tick)
    );

    // An ant is (frame it left the food, cells walked, ticks of dwell left).
    // Its position is recomputed from the tick count rather than accumulated,
    // so 0.7 cells a tick cannot drift a cell over a long walk.
    let mut ants: Vec<(u64, i32, u64)> = Vec::new();
    let read = |p: &pheromone::Pheromones, u: i32| f64::from(p.sample(Channel::B, nest_x + u * toward_food, row));
    let (mut rise_up, mut rise_lit) = (0u64, 0u64);
    let (mut slope_sum, mut pile_sum, mut door_sum, mut r2_sum, mut r6_sum, mut n_samples) = (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64, 0u64);
    for frame in 1..=frames {
        // Departures on frame 1, 1 + every, ... -- so an `every` that is not a
        // multiple of the pass interval gives each ant its own phase against
        // the plane, as the live colony's staggered ticks do.
        if (frame - 1).is_multiple_of(every) {
            ants.push((frame, 0, dwell));
        }
        // Ants act before the pass, the order `frame::step` runs them in.
        for ant in ants.iter_mut() {
            let (left, walked, dwell_left) = ant;
            if !(frame - *left).is_multiple_of(tickframes) {
                continue;
            }
            let age = (frame - *left) / tickframes;
            if *walked < span {
                if cells_by(age) > *walked {
                    *walked += 1;
                    // The head it arrived on, which is where `creature.rs`
                    // lays under the shipped `DEPOSIT_AT=head`.
                    plane.deposit(Channel::B, food_x - *walked * toward_food, row, laid(age));
                }
            } else if *dwell_left > 0 {
                *dwell_left -= 1;
                plane.deposit(Channel::B, nest_x, row, laid(age));
            }
        }
        ants.retain(|&(_, walked, dwell_left)| walked < span || dwell_left > 0);
        plane.step(frame, pheromone::PHEROMONE_INTERVAL);

        if !frame.is_multiple_of(sample) {
            continue;
        }
        let prof: Vec<pheromone::Scent> = (x0..=x1).map(|x| plane.sample(Channel::B, x, row)).collect();
        println!(
            "BTRAIL seed=0 arm=design gate=- ft=- piles={} stop=0 gap={span} layfrom=nest nest={nest_x} target={food_x} ch=B kind=design rule={rule} y={row} x0={x0} x1={x1} frame={frame} hand=0 cells={} peak={} prof={}",
            if reverse { "west" } else { "east" },
            prof.iter().filter(|&&v| v > 0).count(),
            prof.iter().copied().max().unwrap_or(0),
            prof.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",")
        );
        // The standing state, not the build-up: only the second half counts.
        if frame * 2 <= frames {
            continue;
        }
        n_samples += 1;
        // `TRAIL_HALF / 4`: below it the chooser's presence reads under 0.2
        // and an increase is a rounding step, not something an ant could
        // climb.
        let lit = f64::from(pheromone::DEPOSIT) / 10.0 / 4.0;
        let b: Vec<f64> = (0..=span).map(|u| read(&plane, u)).collect();
        for pair in b.windows(2) {
            if pair[0] >= lit && pair[1] >= lit {
                rise_lit += 1;
                rise_up += u64::from(pair[1] > pair[0]);
            }
        }
        // OLS of ln(1 + B) on distance toward the food, over the whole route.
        //
        // **Under `lay=const` this reads near zero, and that is not a bug.**
        // A steady stream lays every cell once per ant, so every cell has
        // the same time-mean age and the time-mean profile is flat: -0.0004
        // per cell at the defaults. The nest-high trail is a *sawtooth*, one
        // tooth per ant on the route, falling toward the food behind each
        // animal, and only a per-snapshot readout sees it -- `rising` (0.108)
        // does. The slope says what the envelope does; the teeth are what an
        // ant stands on.
        //
        // **And that -0.0004 has its sign from one cell.** u = span is the
        // pile's own cell, which no ant lays (the first step lands one off
        // it), so it reads spill only; fitted over the laid cells 0..span-1
        // the same const run reads **+0.00045**. So `slope` cannot tell
        // `lay=const` from a flat trail -- read `rising` for that -- and an
        // odometer's slope has to clear this edge (worth under 0.001) to mean anything
        // (t=16: +0.0221 with the cell, +0.0227 without). Kept in the fit so
        // the design row reads the same nest..target span the live rows do.
        let n = b.len() as f64;
        let mean_u = (n - 1.0) / 2.0;
        let lb: Vec<f64> = b.iter().map(|&v| v.ln_1p()).collect();
        let mean_l = lb.iter().sum::<f64>() / n;
        let (sxy, sxx) = lb.iter().enumerate().fold((0.0, 0.0), |(sxy, sxx), (u, &l)| {
            let du = u as f64 - mean_u;
            (sxy + du * (l - mean_l), sxx + du * du)
        });
        slope_sum += sxy / sxx;
        // Within 3 cells of each end, on both sides of it: the pile and the
        // door are places, and an ant arriving at either reads the spill past
        // it as much as the route before it.
        let window = |c: i32| (-3..=3).map(|d| read(&plane, c + d)).sum::<f64>() / 7.0;
        pile_sum += window(span);
        door_sum += window(0);
        // The door read an empty ant leaving the nest would make, food side
        // against the other: reach 2 is `trail_presence`'s pair of cells,
        // reach 6 the sensor point.
        let r2 = (read(&plane, 1).max(read(&plane, 2)), read(&plane, -1).max(read(&plane, -2)));
        let r6 = (read(&plane, 6), read(&plane, -6));
        let half = f64::from(pheromone::DEPOSIT) / 10.0;
        r2_sum += (r2.0 - r2.1) / (r2.0 + r2.1 + half);
        r6_sum += (r6.0 - r6.1) / (r6.0 + r6.1 + half);
    }
    let ns = n_samples.max(1) as f64;
    println!(
        "STREAM lay={lay} t={t} every={every} speed={speed} dwell={dwell} rising={:.3} slope={:+.5} pile_door={:.3} door_r2={:+.4} door_r6={:+.4} samples={n_samples} lit_pairs={rise_lit} reverse={}",
        if rise_lit == 0 { f64::NAN } else { rise_up as f64 / rise_lit as f64 },
        slope_sum / ns,
        (pile_sum / ns) / (door_sum / ns),
        r2_sum / ns,
        r6_sum / ns,
        if reverse { "on" } else { "off" }
    );
}

fn main() {
    // Births inherit exactly unless PIXEL_PHYSICS_MUTATION=on (`creature::mutation_of`).
    pixel_physics::sim::creature::mutation_off_for_measuring();
    let mode = arg_str("mode", "both");
    // Before the shared header: its defaults (frames, span) are the bed's,
    // and the stream's first line has to echo the stream's own.
    if mode == "stream" {
        stream_mode();
        return;
    }
    let frames: u64 = arg("frames", 4000);
    let seeds: u64 = arg("seeds", 12);
    let seed0: u64 = arg("seed", 1);
    let peak: f32 = arg("peak", pheromone::DEPOSIT as f32 / pheromone::SCALE as f32);
    let width: i32 = arg("width", 256);
    let span: i32 = arg("span", 224);
    // The sensitivity control for `intact`: with the pheromone pass back in,
    // the standing-trail assertion must fail.
    let decay: bool = arg_str("decay", "off") == "on";

    println!("onetrail: mode={mode} frames={frames} seeds={seeds} seed0={seed0} peak={peak} width={width} span={span} decay={}", if decay { "ON (control)" } else { "off" });
    println!("  trail never decays (step_pheromones is never called); emission silenced; energy pinned; one ant\n");

    let probe = World::new(Rect::new(0, 0, 15, 15));
    let base = probe.species.get(probe.species.id_of("ant").expect("ant species")).genome.clone();

    if mode == "timing" {
        println!("sequential laying + real decay: one cell per `per_cell` frames.");
        println!("A laden ant lays B only on the way home, so the food end is always the OLDER end.\n");
        let reverse = arg_str("reverse", "off") == "on";
        if reverse {
            println!("  REVERSED: laying nest -> food. Every reading must mirror the forward run.\n");
        }
        for per_cell in [1u64, 2, 4, 8] {
            timing_mode(per_cell, arg("span", 112), peak, width, reverse);
        }
        return;
    }
    if mode == "arith" || mode == "both" {
        arithmetic(&base);
        println!();
    }
    if mode == "arith" {
        return;
    }

    println!("the bed: one ant started at the trail's midpoint, {frames} frames, {seeds} seeds");
    println!("  `toward peak` is signed displacement toward the high end; the mirrored arm");
    println!("  has the identical ramp reversed, so any scene bias cancels between them.\n");
    println!("{:>30} {:>8} {:>8} {:>8} {:>9} {:>8} {:>7}", "arm", "median", "mean", "reach", "along", "P(move)", "on-band");
    println!("{:->30} {:->8} {:->8} {:->8} {:->9} {:->8} {:->7}", "", "", "", "", "", "", "");

    for (label, ch, shape, laden, regate, flip, start_off) in [
        ("laden, channel A ramp (shipped)", Channel::A, Shape::Ramp, true, false, false, 0),
        ("empty, channel B ramp (shipped)", Channel::B, Shape::Ramp, false, false, false, 0),
        ("empty, channel B FLAT", Channel::B, Shape::Flat, false, false, false, 0),
        ("laden, channel A FLAT", Channel::A, Shape::Flat, true, false, false, 0),
        ("re-gated, ASCEND, on trail", Channel::B, Shape::Ramp, false, true, false, 0),
        ("re-gated, DESCEND, on trail", Channel::B, Shape::Ramp, false, true, true, 0),
        ("re-gated, ASCEND, 24 OFF the end", Channel::B, Shape::Ramp, false, true, false, -24),
        ("re-gated, DESCEND, 24 OFF the end", Channel::B, Shape::Ramp, false, true, true, -24),
        ("shipped, 24 OFF the end (control)", Channel::B, Shape::Ramp, false, false, false, -24),
        ("empty ant, NO trail (control)", Channel::B, Shape::None, false, false, false, 0),
    ] {
        let mut nets: Vec<i32> = Vec::new();
        let (mut reach, mut along, mut pm, mut band, mut tick) = (0i64, 0.0f64, 0.0f64, 0u64, 0u64);
        let mut all_intact = true;
        for s in seed0..seed0 + seeds {
            for mirror in [false, true] {
                let r = run(s, ch, shape, mirror, laden, regate, frames, peak, width, span, decay, flip, start_off);
                nets.push(r.toward_peak);
                reach += r.reach as i64;
                along += r.along;
                pm += r.p_move;
                band += r.on_band;
                tick += r.ticks;
                all_intact &= r.intact;
            }
        }
        let n = nets.len() as f64;
        let mean = nets.iter().map(|&v| v as f64).sum::<f64>() / n;
        let mut sorted = nets.clone();
        sorted.sort_unstable();
        let median = sorted[sorted.len() / 2];
        assert!(
            all_intact,
            "arm {label:?}: the trail changed during the run, so it was not the standing trail this harness claims \
             (expected, and the point, under decay=on)"
        );
        println!(
            "{label:>30} {median:>8} {mean:>8.1} {:>8.1} {:>9.4} {:>8.3} {:>6.1}%",
            reach as f64 / n,
            along / n,
            pm / n,
            if tick == 0 { 0.0 } else { 100.0 * band as f64 / tick as f64 }
        );
    }
}
