//! **Does a laid trail move the colony?** — the positive control
//! `open-bugs-handoff.md` §Z7 asks for, and the instrument that says whether
//! a re-weighted gate is worth racing.
//!
//! §Z7's null is that a real, climbing channel-B trail from the nest to a
//! target moved a 20-ant colony's near-target ant-ticks by **exactly zero,
//! twice**. That tie is the tell: the instrument was not moving. This binary
//! is that measurement made re-runnable, plus the arithmetic underneath it,
//! plus the ability to swap the gate weights without editing a `.ron` and
//! rebuilding (which is the `include_str!` trap `CLAUDE.md` names).
//!
//! ```text
//! cargo run --release --example trailfollow -- mode=arith            # the arithmetic, through the real eval_brain
//! cargo run --release --example trailfollow -- gate=saturated seeds=3  # the null
//! cargo run --release --example trailfollow -- gate=b2 seeds=3       # the candidate
//! cargo run --release --example trailfollow -- gate=b2 spec          # print the arena's hidden= string and stop
//! ```
//!
//! # Two halves, and the first one cannot lie
//!
//! **`mode=arith` runs `brain::eval_brain` itself** on the shipped genome with
//! `Carrying` and `PheroBAlong` set by hand, and prints `P(move)` across the
//! along-gradient range. No world, no seed, no chaos: if a gate delivers
//! nothing here it cannot deliver anything in a bed, and if it delivers here
//! the bed question is whether the animal is in a position to use it. This is
//! the positive control for the *instrument*, run before the instrument.
//!
//! **`mode=colony` is §Z7's own count**: found a colony at the nest end, lay
//! (and keep laying) a channel-B ramp from nest to target, run, and count
//! ant-ticks within `near=` cells of the target — with the trail and without,
//! same seed, same everything else.
//!
//! **The trail is re-laid every `relay=` frames, and that is not a thumb on
//! the scale.** `pheromone.rs`'s own test records that *"a one-shot deposit
//! evaporates before it spreads"*; a trail laid once at frame 0 is gone for
//! most of a 3,000-tick run, so a one-shot arm measures evaporation as much
//! as following. A standing trail is the *most* sensitive arm there is, which
//! is what a positive control wants: if the count will not move against a
//! trail that is always there, it will not move against a real one.
//!
//! **No food at the target.** The only thing pulling an ant to `target=` is
//! the trail, so the count is attributable. Put food there and an arriving
//! ant fills its crop, flips to channel A and walks home, which measures the
//! round trip rather than the gate.

use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::brain::{self, BrainInput as I, BrainOutput as O};
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::pheromone::{self, Channel};
use pixel_physics::sim::{frame, player};

fn arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{name}=")).map(|v| v.parse().ok().expect("parses")))
}
fn arg_str(name: &str) -> Option<String> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{name}=")).map(|v| v.to_string()))
}
fn flag(name: &str) -> bool {
    std::env::args().skip(1).any(|a| a == name)
}

/// One candidate gate, as the three numbers that define it.
///
/// The pairs are mirror images by construction — pair 0/1 is gated **on** a
/// full crop and pair 2/3 **on** an empty one — so a candidate is three
/// numbers and not twelve: `off` is how deep the shut unit sits, `on` is
/// where the open one sits, and `along` is the signal weight. Writing it this
/// way rather than as twelve loose weights is the point: §Z7's whole finding
/// is a statement about `on` relative to `along`, and a form that lets you
/// author them independently of each other is a form that hides it.
#[derive(Clone, Copy)]
struct Gate {
    name: &'static str,
    /// Gate sum when the unit is **shut** (fill 0 for pair 0/1). Negative.
    off: f32,
    /// Gate sum when the unit is **open** (fill 1 for pair 0/1).
    on: f32,
    /// `PheroAAlong`/`PheroBAlong` coefficient, `±along` across the pair.
    along: f32,
}

/// The candidates §Z7 names, plus the two states of the real file.
///
/// **`saturated` is what BOTH pairs shipped as until 2026-09-09 and what the
/// food pair still ships as; `landed` is what units 0/1 ship as now.** The
/// preset was called `shipped` and that name went stale the moment half of it
/// landed, which is exactly the kind of label a later session reads as a
/// claim about the file. Every preset here overrides whatever the `.ron`
/// holds, so a run names the twelve numbers it actually used rather than
/// inheriting them.
const GATES: &[Gate] = &[
    // `Bias -45, Carrying +75` is `off = -45, on = +30`.
    Gate { name: "saturated", off: -45.0, on: 30.0, along: 6.0 },
    // §Z7 candidate (a): gate on the slope, symmetric and shallow.
    Gate { name: "a", off: -4.0, on: 4.0, along: 4.0 },
    // §Z7 candidate (b): asymmetric, off deep, on on the slope.
    Gate { name: "b", off: -20.0, on: 4.0, along: 4.0 },
    // (b) with the two numbers `ant.ron`'s own tuning note measured left
    // where they are: the off depth (45, where the shut pair's leak is
    // 0.004) and the along weight (6, which is where the design's `±3.75`
    // Move swing comes from). Only the on-state moves.
    Gate { name: "b2", off: -45.0, on: 0.5, along: 6.0 },
    // The same at a deeper on-state, to bracket what `on` costs.
    Gate { name: "b3", off: -45.0, on: 2.0, along: 6.0 },
];

/// What `ant.ron` actually carries since 2026-09-09: units 0/1 at `b2`, units
/// 2/3 still `saturated`. Named separately rather than added to `GATES`
/// because a `Gate` is by construction a mirrored pair of pairs, and the
/// landed state deliberately is not one -- which is the finding.
const LANDED_NOTE: &str =
    "landed 2026-09-09: units 0/1 = b2 (off -45, on +0.5, along 6); units 2/3 = saturated (off -45, on +30, along 6). See open-bugs-handoff.md Z7.";

fn gate_by_name(n: &str) -> Gate {
    *GATES
        .iter()
        .find(|g| g.name == n)
        .unwrap_or_else(|| panic!("unknown gate {n:?}; known: {:?}", GATES.iter().map(|g| g.name).collect::<Vec<_>>()))
}

impl Gate {
    /// The twelve `(input, unit, weight)` entries this gate authors.
    ///
    /// `Carrying` is crop fill in `0..=1`, so the gate sum for pair 0/1 is
    /// `bias + carry * fill` with `bias = off` and `carry = on - off`; pair
    /// 2/3 is the same function of `1 - fill`.
    fn wires(&self) -> Vec<(I, usize, f32)> {
        let carry = self.on - self.off;
        let mut v = Vec::new();
        for (u, sign) in [(0usize, 1.0f32), (1, -1.0)] {
            v.push((I::Bias, u, self.off));
            v.push((I::Carrying, u, carry));
            v.push((I::PheroAAlong, u, sign * self.along));
        }
        for (u, sign) in [(2usize, 1.0f32), (3, -1.0)] {
            v.push((I::Bias, u, self.on));
            v.push((I::Carrying, u, -carry));
            v.push((I::PheroBAlong, u, sign * self.along));
        }
        v
    }

    /// The `creature_arena` `hidden=` string for this gate, so the race and
    /// this harness provably run the same twelve numbers rather than two
    /// transcriptions of them.
    fn spec(&self) -> String {
        self.wires()
            .iter()
            .map(|&(i, u, w)| format!("{}:{u}:{w}", brain::INPUT_NAMES[i as usize]))
            .collect::<Vec<_>>()
            .join(",")
    }

    fn apply(&self, g: &mut [f32]) -> usize {
        let mut moved = 0;
        for (i, u, w) in self.wires() {
            let s = brain::ih_slot(i, u);
            if g[s] != w {
                g[s] = w;
                moved += 1;
            }
        }
        moved
    }
}

/// **`P(move)` across the along-gradient range, through the real
/// `eval_brain`** — the arithmetic §Z7 states, measured rather than asserted.
///
/// `Energy` is set to 1.0 (a fed ant) because that is what the walk drive is
/// read against: `ant.ron` authors `(Bias, Move, 2.0)` and `(Energy, Move,
/// -1.75)`, so a full ant's residual drive is 0.25 and a starving one's is
/// 2.0. The gate has to be comparable to *that*, and quoting it against the
/// bias alone would flatter it.
fn arithmetic(base: &[f32]) {
    let along: Vec<f32> = vec![0.0, 0.05, 0.1, 0.2, 0.35, 0.5, 1.0];
    println!("{:>9} {:>9} | {}", "gate", "carry", along.iter().map(|a| format!("{a:>8.2}")).collect::<Vec<_>>().join(""));
    println!("{:->9} {:->9} | {:->width$}", "", "", "", width = 8 * along.len());
    for g in GATES {
        let mut genome = base.to_vec();
        g.apply(&mut genome);
        // Laden reads channel A (pair 0/1); empty reads channel B (2/3).
        for (label, fill, slot) in [("laden 1.0", 1.0f32, I::PheroAAlong), ("empty 0.0", 0.0, I::PheroBAlong)] {
            let row: Vec<String> = along
                .iter()
                .map(|&a| {
                    let mut inputs = [0.0f32; brain::BRAIN_INPUTS];
                    inputs[I::Bias as usize] = 1.0;
                    inputs[I::Energy as usize] = 1.0;
                    inputs[I::Carrying as usize] = fill;
                    inputs[slot as usize] = a;
                    let mut state = [0.0f32; brain::BRAIN_HIDDEN];
                    let (out, _) = brain::eval_brain(&genome, &inputs, &mut state);
                    format!("{:>8.3}", out[O::Move as usize])
                })
                .collect();
            println!("{:>9} {label} | {}", g.name, row.join(""));
        }
        // The leak: the *shut* pair's response to its own channel. It must
        // stay near zero, or the gate is not a gate. `ant.ron`'s own tuning
        // note measures this at 0.004 per unit at an offset of 30 and 0.09
        // at 12, and records that 12 collapsed a colony onto its nest.
        let mut inputs = [0.0f32; brain::BRAIN_INPUTS];
        inputs[I::Bias as usize] = 1.0;
        inputs[I::Energy as usize] = 1.0;
        inputs[I::Carrying as usize] = 0.0;
        let mut state = [0.0f32; brain::BRAIN_HIDDEN];
        let (base_out, _) = brain::eval_brain(&genome, &inputs, &mut state);
        inputs[I::PheroAAlong as usize] = 1.0;
        let mut state = [0.0f32; brain::BRAIN_HIDDEN];
        let (leak_out, _) = brain::eval_brain(&genome, &inputs, &mut state);
        println!(
            "{:>9} {:>9} | shut pair's own-channel leak at a=1: {:+.4} on P(move)",
            g.name,
            "leak",
            leak_out[O::Move as usize] - base_out[O::Move as usize],
        );
        println!("{:>9} {:>9} | hidden={}", g.name, "arena", g.spec());
    }
}

/// Lay (or re-lay) the channel-B ramp from the nest to the target, at the
/// `SCENT` tool's own shape: `t * pheromone::DEPOSIT`, climbing toward the
/// target, over a band deep enough that an ant walking a row or two off the
/// nominal surface still reads it.
fn lay(w: &mut pixel_physics::sim::world::World, nest_x: i32, target_x: i32, surface: i32) {
    for x in nest_x..=target_x {
        let t = (x - nest_x) as f32 / (target_x - nest_x) as f32;
        let amount = (t * pheromone::DEPOSIT as f32) as u8;
        for y in (surface - 3)..=(surface + 1) {
            w.deposit_pheromone(Channel::B, x, y, amount);
        }
    }
}

/// One arm: one seed, trail on or off, one gate.
fn run(seed: u64, trail: bool, gate: Gate, frames: u64, ants: i32, relay: u64, near: i32) -> (u64, u64, f32) {
    let spec = LabBox { width: 256, height: 192, ground_y: 96, soil_depth: 48, founders: 0, colonies: 0, seed, ..LabBox::default() };
    let mut w = spec.build();
    let species_id = w.species.id_of("ant").expect("the ant species is compiled in");
    let mut genome = w.species.get(species_id).genome.clone();
    let moved = gate.apply(&mut genome);
    assert!(gate.name == "saturated" || moved > 0, "gate {} changed no slot, so both arms carry one genome", gate.name);

    let surface = spec.ground_y - 2;
    let (nest_x, target_x) = (40, 130);
    let placed = w.found_colony_of(nest_x, surface, "ant", ants);
    assert!(placed > 0, "no ants placed at the nest end; there is nothing to measure");
    for id in w.live_organism_ids() {
        if w.organism(id).is_some_and(|s| s.species == species_id) {
            assert!(w.set_organism_genome(id, genome.clone()), "the founder must be live when its genome is set");
        }
    }

    let (mut particles, mut blasts, tuning) = (ParticleSystem::default(), Blasts::default(), player::Tuning::default());
    let (mut near_ticks, mut ant_ticks) = (0u64, 0u64);
    // The along reading a real ant would get, averaged over every sample —
    // the instrument's own positive control, because a count that does not
    // move against a gradient that was never there says nothing.
    let (mut along_sum, mut along_n) = (0.0f64, 0u64);
    for f in 1..=frames {
        if trail && (f == 1 || f.is_multiple_of(relay)) {
            lay(&mut w, nest_x, target_x, surface);
        }
        frame::step(&mut w, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        for id in w.live_organism_ids() {
            let Some(s) = w.organism(id) else { continue };
            if s.species != species_id {
                continue;
            }
            let Some(&(hx, hy)) = s.chain.first() else { continue };
            ant_ticks += 1;
            if (hx - target_x).abs() <= near {
                near_ticks += 1;
            }
            if f.is_multiple_of(100) {
                let here = w.pheromone_at(Channel::B, hx, hy) as f32;
                let ahead = w.pheromone_at(Channel::B, hx + 4, hy) as f32;
                along_sum += ((ahead - here) / (ahead + here + 1.0)) as f64;
                along_n += 1;
            }
        }
    }
    (near_ticks, ant_ticks, if along_n == 0 { 0.0 } else { (along_sum / along_n as f64) as f32 })
}

fn main() {
    let mode = arg_str("mode").unwrap_or_else(|| "colony".into());
    let gate = gate_by_name(&arg_str("gate").unwrap_or_else(|| "saturated".into()));
    let frames: u64 = arg("frames").unwrap_or(3000);
    let seeds: u64 = arg("seeds").unwrap_or(3);
    let seed0: u64 = arg("seed").unwrap_or(1);
    let ants: i32 = arg("ants").unwrap_or(20);
    let relay: u64 = arg("relay").unwrap_or(60);
    let near: i32 = arg("near").unwrap_or(10);

    if flag("spec") {
        println!("{}", gate.spec());
        return;
    }

    // The harness names its own parameters, so a log that does not name a
    // knob was written by a binary that never had one — `CLAUDE.md`'s
    // stale-harness gotcha, which cost a 3.5-hour study.
    println!("trailfollow: mode={mode} gate={} frames={frames} seeds={seeds} seed0={seed0} ants={ants} relay={relay} near={near}", gate.name);
    println!("  gate {}: off {:+.1}  on {:+.1}  along ±{:.1}", gate.name, gate.off, gate.on, gate.along);
    println!("  {LANDED_NOTE}\n");

    let spec = LabBox { width: 256, height: 192, ground_y: 96, soil_depth: 48, founders: 0, colonies: 0, seed: seed0, ..LabBox::default() };
    let w = spec.build();
    let base = w.species.get(w.species.id_of("ant").expect("the ant species is compiled in")).genome.clone();

    if mode == "arith" {
        println!("P(move) for a fed ant (Energy 1.0), by gate and by along-gradient reading:\n");
        arithmetic(&base);
        return;
    }

    println!("{:>5} {:>10} {:>10} {:>9} {:>10} {:>8}", "seed", "trail on", "trail off", "delta", "ratio", "along");
    let (mut on_tot, mut off_tot) = (0u64, 0u64);
    let mut moved_up = 0;
    for s in seed0..seed0 + seeds {
        let (on, on_ants, along) = run(s, true, gate, frames, ants, relay, near);
        let (off, off_ants, _) = run(s, false, gate, frames, ants, relay, near);
        // Ant-ticks differ between arms if one arm's ants die sooner, so the
        // share is what compares: a raw count that fell because the colony
        // shrank is not a colony that stopped following.
        let share = |n: u64, d: u64| if d == 0 { 0.0 } else { 100.0 * n as f64 / d as f64 };
        println!(
            "{s:>5} {on:>10} {off:>10} {:>+9} {:>9.2}x {:>8.3}   (share {:.2}% vs {:.2}%)",
            on as i64 - off as i64,
            if off == 0 { f64::INFINITY } else { on as f64 / off as f64 },
            along,
            share(on, on_ants),
            share(off, off_ants)
        );
        on_tot += on;
        off_tot += off;
        if on > off {
            moved_up += 1;
        }
    }
    println!(
        "\n  pooled: {on_tot} with the trail against {off_tot} without ({:+}), {moved_up} of {seeds} seeds up",
        on_tot as i64 - off_tot as i64
    );
    println!("  arena:  hidden={}", gate.spec());
}
