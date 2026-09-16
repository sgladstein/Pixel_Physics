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
fn run(seed: u64, ch: Channel, shape: Shape, mirror: bool, laden: bool, regate: bool, frames: u64, peak: f32, w_cells: i32, span: i32, decay: bool) -> Run {
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
    world.species.set_genome(species, genome.clone());

    let start_x = (w_world - 1) / 2;
    // **`span` is what sets the gradient, and `peak` is not.** The reader is
    // `(ahead - here) / (ahead + here + SCALE)`, which is scale-free: multiply
    // the whole trail by ten and, once it is well clear of `SCALE`, the
    // reading is unchanged. What moves it is how much of the ramp fits inside
    // one sensor offset, i.e. the span. A `peak=` sweep is the wrong knob and
    // would read as "steepness does not help" when it had never been varied.
    let half = (span / 2).min(w_world / 2 - 9);
    let (x0, x1) = (start_x - half, start_x + half);
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

fn main() {
    let mode = arg_str("mode", "both");
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

    for (label, ch, shape, laden, regate) in [
        ("laden ant, channel A ramp", Channel::A, Shape::Ramp, true, false),
        ("empty ant, channel B ramp", Channel::B, Shape::Ramp, false, false),
        ("empty ant, chan B ramp RE-GATED", Channel::B, Shape::Ramp, false, true),
        ("laden ant, channel A FLAT", Channel::A, Shape::Flat, true, false),
        ("empty ant, channel B FLAT", Channel::B, Shape::Flat, false, false),
        ("laden ant, NO trail", Channel::A, Shape::None, true, false),
        ("empty ant, NO trail", Channel::B, Shape::None, false, false),
    ] {
        let mut nets: Vec<i32> = Vec::new();
        let (mut reach, mut along, mut pm, mut band, mut tick) = (0i64, 0.0f64, 0.0f64, 0u64, 0u64);
        let mut all_intact = true;
        for s in seed0..seed0 + seeds {
            for mirror in [false, true] {
                let r = run(s, ch, shape, mirror, laden, regate, frames, peak, width, span, decay);
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
