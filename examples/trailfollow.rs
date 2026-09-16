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
use pixel_physics::sim::Cell;
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
    // The genome exactly as `ant.ron` authors it: units 0/1 at b2
    // (Bias -45, Carrying 45.5) and units 2/3 saturated at Bias 45,
    // Carrying -75. `off`/`on`/`along` are descriptive here and unused --
    // `apply` returns early.
    Gate { name: "shipped", off: -45.0, on: 0.5, along: 6.0 },
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
        // **`shipped` applies nothing, and it exists because every other preset
        // here is a claim about `ant.ron` rather than a read of it.**
        // `saturated` was the shipped animal once and is now stale in three
        // places: it writes `Carrying:0:75` where the file ships **45.5** (so it
        // saturates the *homing* pair, which landed at b2 on 2026-09-09) and
        // `Bias:2:30` where the file ships **45**. Using it as a control makes
        // a channel-B comparison a three-change comparison, and the delta then
        // includes homing. The only honest control is the untouched genome.
        if self.name == "shipped" {
            return 0;
        }
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
        let amount = (t * pheromone::DEPOSIT as f32) as pheromone::Scent;
        for y in (surface - 3)..=(surface + 1) {
            w.deposit_pheromone(Channel::B, x, y, amount);
        }
    }
}

/// What one arm reports.
///
/// **`alive` is here because without it a null is uninterpretable.** A
/// near-target count that does not move has two causes that look identical from
/// outside -- the colony cannot read the trail, or the colony is dead -- and
/// this scene starves its ants by construction (`founders: 0`, and no food
/// unless `food=` places some). `pherolife` learned the same lesson the same
/// way: "7,039 deposits and no trail" became "52 -> 3 ants" the moment the ant
/// count was printed beside the planes.
struct Arm {
    near_ticks: u64,
    ant_ticks: u64,
    along: f32,
    alive_end: usize,
    alive_min: usize,
    /// Round trips the colony actually completed. **The denominator for
    /// everything below**: a natural trail laid by nobody is not a finding
    /// about trail shape, it is a loop that never seeded.
    trips: u64,
    deliveries: u64,
    /// Route cells still holding any channel B at the end. With `stop=` set,
    /// this is the *ants'* trail -- the hand-laid one stopped being refreshed
    /// at frame `stop` and a cell laid at `DEPOSIT` dies in ~144 frames.
    live_cells: usize,
    /// Peak route cells alive at any sample after laying stopped.
    peak_cells: usize,
    /// The ants' own trail, as the gradient a reader would compute along it,
    /// **positive = climbing toward the nest**. This is §1c's single-ant
    /// derivation put to real traffic: positive means an ascending empty ant
    /// is steered home and the food-ward ramp is necessary.
    natural_along: f32,
    /// Share of laden ant-ticks spent in the nest half. The homing
    /// precondition, re-checked inside the scene that matters: if laden ants
    /// wander rather than walk home, their channel B is a wander-field and its
    /// shape says nothing about polarity.
    laden_nest_share: f32,
    /// **Laden ant-ticks by third of the route, and the spoil share -- the
    /// dilution census.** Owner's observation, 2026-09-16: channel B is laid
    /// by *any* laden ant, and only one of the four things that make an ant
    /// laden is route-laying.
    ///
    /// `Carrying` is `crop_fill.max(spoil ? 1.0 : 0.0)`, and a deposit happens
    /// on every successful move (P-11), so `(Carrying, EmitB, 2.5)` fires
    /// identically for an ant walking food home, an ant shuffling at the patch
    /// while it digests, an ant wandering before the unconditional
    /// `(Carrying, Drop, 0.2)` puts it down somewhere arbitrary, and an ant
    /// hauling dig spoil. Three of those four lay noise on the same plane the
    /// fourth is trying to write a route on.
    ///
    /// **The ratio that matters is per MOVE, not per pickup**, and that is why
    /// this is counted here rather than inferred from the bed's
    /// pickups-to-deliveries figure. On the played bed 6,943 pickups produce
    /// 290 deliveries -- 4% -- but a homeward trip from 28 cells is ~28 laden
    /// moves while an eat-in-place is nearly none, so the deposit-weighted
    /// route share could be far above 4% or far below it. Nobody has measured
    /// it either way.
    laden_by_third: [u64; 3],
    spoil_ticks: u64,
}

/// One arm: one seed, trail on or off, one gate.
// Eight against clippy's ceiling of seven: these are the arm's axes, and a
// struct would hide that each one is independently swept.
#[allow(clippy::too_many_arguments)]
fn run(seed: u64, trail: bool, gate: Gate, frames: u64, ants: i32, relay: u64, near: i32, food: i32, stop: u64) -> Arm {
    let spec = LabBox { width: 256, height: 192, ground_y: 96, soil_depth: 48, founders: 0, colonies: 0, seed, ..LabBox::default() };
    let mut w = spec.build();
    let species_id = w.species.id_of("ant").expect("the ant species is compiled in");
    let mut genome = w.species.get(species_id).genome.clone();
    let moved = gate.apply(&mut genome);
    assert!(gate.name == "shipped" || moved > 0, "gate {} changed no slot, so both arms carry one genome", gate.name);

    let surface = spec.ground_y - 2;
    let (nest_x, target_x) = (40, 130);
    // The species' own sensor reach, not a literal -- see the along readout.
    let sensor_offset = w.species.get(species_id).creature.as_ref().expect("ant is a creature").sensor_offset;
    let placed = w.found_colony_of(nest_x, surface, "ant", ants);
    assert!(placed > 0, "no ants placed at the nest end; there is nothing to measure");

    // **`food=` is the positive control for the READOUT, not for the trail.**
    // Every recorded result from this harness is an exact tie -- 595 = 595 here,
    // 1,903 = 1,903 on the tools lane's independent harness -- so nothing has
    // ever shown that `near_ticks` *can* move. Food at the target draws ants by
    // a mechanism that has nothing to do with channel B, so if the count does
    // not rise with it, the instrument is blind and no null from it means
    // anything (`CLAUDE.md`: a guard that cannot go red is blind, not strong).
    //
    // It is also what a round-trip arm needs, and that is a deliberate
    // departure from this file's "no food at the target" rule: that rule exists
    // so the count is attributable to the trail alone, which is right for the
    // pull question and wrong for the loop question.
    if food > 0 {
        let corpse = w.materials.id_of("corpse").expect("corpse is compiled in");
        for i in 0..food {
            let (fx, fy) = (target_x + (i % 8) - 4, surface - (i / 8));
            w.set(fx, fy, Cell::new(corpse, 0));
        }
    }
    for id in w.live_organism_ids() {
        if w.organism(id).is_some_and(|s| s.species == species_id) {
            assert!(w.set_organism_genome(id, genome.clone()), "the founder must be live when its genome is set");
        }
    }

    let (mut particles, mut blasts, tuning) = (ParticleSystem::default(), Blasts::default(), player::Tuning::default());
    let (mut near_ticks, mut ant_ticks) = (0u64, 0u64);
    let mut alive_min = usize::MAX;
    let (mut laden_ticks, mut laden_nest_ticks) = (0u64, 0u64);
    let (mut laden_by_third, mut spoil_ticks) = ([0u64; 3], 0u64);
    let mut peak_cells = 0usize;
    let midpoint = (nest_x + target_x) / 2;
    // The along reading a real ant would get, averaged over every sample —
    // the instrument's own positive control, because a count that does not
    // move against a gradient that was never there says nothing.
    let (mut along_sum, mut along_n) = (0.0f64, 0u64);
    for f in 1..=frames {
        // **`stop` is what turns this from a pull arm into a loop arm.** Up to
        // `stop` the trail is guaranteed, which breaks the circularity -- a
        // naturally laid trail needs commuters, and commuters need a trail
        // worth following. After it, the only channel B in the world is what
        // the ants themselves put down.
        if trail && (stop == 0 || f <= stop) && (f == 1 || f.is_multiple_of(relay)) {
            lay(&mut w, nest_x, target_x, surface);
        }
        frame::step(&mut w, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        alive_min = alive_min.min(w.live_creature_count());
        // Only sampled once hand-laying has stopped, so this counts the ants'
        // own trail rather than the one we kept refreshing.
        // **Not `f > stop` -- that catches the hand-laid trail before it has
        // decayed, and every arm then reports a peak of exactly the route
        // length, which is `CLAUDE.md`'s tidiness tell rather than a result.**
        //
        // **The window is 1,500 frames, not the 288 an earlier version used,
        // and the correction is a finding rather than a tweak.** That 288 came
        // from doubling the "a cell laid at `DEPOSIT` dies in ~144 frames"
        // figure in `pheromone-lifetime-and-wiring-2026-09-14.md` -- **a u8-era
        // measurement that the `u16` widening (#450, 2026-09-15) invalidated
        // the day after it was written.** `DEPOSIT` is now `40 * SCALE` =
        // 10,240 rather than 40, and `pherolife` on current `main` reports
        // `trail gone` at **1,476 frames** and the ant's drive falling under
        // threshold at **1,080** -- 0.67x and 0.49x of a 2,200-frame round
        // trip, against the 0.065x that report records. Ten times longer.
        //
        // Sampling at `stop + 288` therefore still caught the hand-laid trail
        // fully intact, and every arm reported a peak of exactly the route
        // length twice over before this was caught.
        if stop > 0 && f > stop + 1500 && f.is_multiple_of(100) {
            let live = (nest_x..=target_x).filter(|&x| w.pheromone_at(Channel::B, x, surface) > 0).count();
            peak_cells = peak_cells.max(live);
        }
        for id in w.live_organism_ids() {
            let Some(s) = w.organism(id) else { continue };
            if s.species != species_id {
                continue;
            }
            let Some(&(hx, hy)) = s.chain.first() else { continue };
            ant_ticks += 1;
            if s.crop.is_some() || s.spoil.is_some() {
                laden_ticks += 1;
                if hx < midpoint {
                    laden_nest_ticks += 1;
                }
                // Thirds of the route: 0 = the nest end, 2 = the food end.
                // Route-laying is the middle; both ends are mostly local
                // shuffle, which is the noise the middle has to be read out of.
                let t = (((hx - nest_x) * 3) / (target_x - nest_x).max(1)).clamp(0, 2) as usize;
                laden_by_third[t] += 1;
                if s.spoil.is_some() {
                    spoil_ticks += 1;
                }
            }
            if (hx - target_x).abs() <= near {
                near_ticks += 1;
            }
            if f.is_multiple_of(100) {
                // **Guard `SCALE`, offset `sensor_offset` -- both were wrong.**
                // This read `+ 1.0` and `hx + 4` from before the planes widened
                // to `u16`. `creature::sense` uses `pheromone::SCALE` (256) and
                // the species' `sensor_offset` (6), and the difference is not
                // cosmetic: at guard 1.0 a single faint cell against an empty
                // one reads 0.996 instead of what the animal actually computes.
                // A positive-control column that does not use the consumer's
                // own arithmetic is not a control.
                let here = w.pheromone_at(Channel::B, hx, hy) as f32;
                let ahead = w.pheromone_at(Channel::B, hx + sensor_offset, hy) as f32;
                along_sum += ((ahead - here) / (ahead + here + pheromone::SCALE as f32)) as f64;
                along_n += 1;
            }
        }
    }
    // The ants' own trail at the end: how much of the route still holds
    // anything, and which way it climbs. Read on the surface row, where it was
    // laid. **Positive `natural_along` = the trail climbs toward the NEST**,
    // which is what §1c's single-ant derivation predicts and what makes a
    // food-ward ramp necessary.
    let live_cells = (nest_x..=target_x).filter(|&x| w.pheromone_at(Channel::B, x, surface) > 0).count();
    let (mut nat_sum, mut nat_n) = (0.0f64, 0u64);
    for x in nest_x..=(target_x - sensor_offset) {
        let here = w.pheromone_at(Channel::B, x, surface) as f64;
        let ahead = w.pheromone_at(Channel::B, x + sensor_offset, surface) as f64;
        if here > 0.0 || ahead > 0.0 {
            // Toward the nest is -x here, so negate: the column reads positive
            // when the trail is taller at the nest end.
            nat_sum += -((ahead - here) / (ahead + here + pheromone::SCALE as f64));
            nat_n += 1;
        }
    }

    let st = w.creature_stats;
    Arm {
        near_ticks,
        ant_ticks,
        along: if along_n == 0 { 0.0 } else { (along_sum / along_n as f64) as f32 },
        alive_end: w.live_creature_count(),
        alive_min: if alive_min == usize::MAX { 0 } else { alive_min },
        trips: st.forage_trips,
        deliveries: st.deliveries,
        live_cells,
        peak_cells,
        natural_along: if nat_n == 0 { 0.0 } else { (nat_sum / nat_n as f64) as f32 },
        laden_nest_share: if laden_ticks == 0 { 0.0 } else { 100.0 * laden_nest_ticks as f32 / laden_ticks as f32 },
        laden_by_third,
        spoil_ticks,
    }
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
    // Cells of corpse at the target. 0 keeps this file's original "nothing but
    // the trail pulls an ant there" design; >0 is the readout's positive
    // control and the round-trip arm. See `run`.
    let food: i32 = arg("food").unwrap_or(0);
    // Frame at which hand-laying stops. 0 (the default) keeps the trail
    // standing for the whole run, which is the pull question. Any positive
    // value turns this into the loop question: seed it, then let go.
    let stop: u64 = arg("stop").unwrap_or(0);

    if flag("spec") {
        println!("{}", gate.spec());
        return;
    }

    // The harness names its own parameters, so a log that does not name a
    // knob was written by a binary that never had one — `CLAUDE.md`'s
    // stale-harness gotcha, which cost a 3.5-hour study.
    println!("trailfollow: mode={mode} gate={} frames={frames} seeds={seeds} seed0={seed0} ants={ants} relay={relay} near={near} food={food}", gate.name);
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

    if mode == "loop" {
        // **Phase 1b: seed the loop, then let go of it.** Hand-lay to frame
        // `stop` with food at the target, then read what the ants' own trail
        // looks like.
        //
        // **`ants=` is swept because trail persistence was predicted to be the
        // binding constraint, and that prediction was WRONG -- recorded here
        // because being wrong for a nameable reason is the useful part.** It
        // read `round_trip / 144` ~= 19 commuters, from the u8-era "a cell
        // laid at `DEPOSIT` dies in ~144 frames". The `u16` widening took
        // `DEPOSIT` from 40 to 10,240 and `pherolife` now measures the trail
        // gone at **1,476 frames**, so the real figure is `2,700 / 1,476` ~=
        // **2 commuters**. Persistence is not the constraint; it was never
        // close. The sweep is kept because it is now measuring something else
        // worth having -- how the dilution split and the colony's survival
        // move with population.
        assert!(stop > 0, "mode=loop needs stop= (the frame hand-laying stops); without it nothing is ever the ants' own trail");
        assert!(food > 0, "mode=loop needs food= at the target, or there is no round trip to close");
        println!(
            "{:>5} {:>5} {:>7} {:>7} {:>9} {:>9} {:>10} {:>9} {:>20} {:>6}",
            "ants", "seed", "trips", "deliv", "cells end", "cells pk", "nat along", "laden@nest", "laden ticks n/m/f", "spoil"
        );
        println!("{:->5} {:->5} {:->7} {:->7} {:->9} {:->9} {:->10} {:->9} {:->20} {:->6}", "", "", "", "", "", "", "", "", "", "");
        for a in [10, 20, 40, 80] {
            for s in seed0..seed0 + seeds {
                let r = run(s, true, gate, frames, a, relay, near, food, stop);
                let lt: u64 = r.laden_by_third.iter().sum();
                let pc = |n: u64| if lt == 0 { 0.0 } else { 100.0 * n as f64 / lt as f64 };
                println!(
                    "{a:>5} {s:>5} {:>7} {:>7} {:>9} {:>9} {:>10.4} {:>8.1}% {:>5.1}/{:>4.1}/{:<4.1}% {:>5.1}% (alive {}/{})",
                    r.trips,
                    r.deliveries,
                    r.live_cells,
                    r.peak_cells,
                    r.natural_along,
                    r.laden_nest_share,
                    pc(r.laden_by_third[0]),
                    pc(r.laden_by_third[1]),
                    pc(r.laden_by_third[2]),
                    pc(r.spoil_ticks),
                    r.alive_end,
                    r.alive_min
                );
            }
        }
        println!("\n  `nat along` is the ants' OWN trail, positive = climbing toward the NEST.");
        println!("  Read `trips` first: a natural trail laid by nobody is not a finding about shape.");
        println!("  `laden ticks n/m/f` splits laden ant-ticks across nest/middle/food thirds, and");
        println!("  `spoil` is the share of them hauling dig spoil rather than food. Channel B is laid");
        println!("  by ALL of these at one strength, so the middle third is the route signal and the");
        println!("  rest is noise on the same plane -- the dilution the deposit-weighted split measures.");
        return;
    }

    println!("{:>5} {:>10} {:>10} {:>9} {:>10} {:>8} {:>14}", "seed", "trail on", "trail off", "delta", "ratio", "along", "alive end/min");
    let (mut on_tot, mut off_tot) = (0u64, 0u64);
    let mut moved_up = 0;
    for s in seed0..seed0 + seeds {
        let on = run(s, true, gate, frames, ants, relay, near, food, stop);
        let off = run(s, false, gate, frames, ants, relay, near, food, stop);
        // Ant-ticks differ between arms if one arm's ants die sooner, so the
        // share is what compares: a raw count that fell because the colony
        // shrank is not a colony that stopped following.
        let share = |n: u64, d: u64| if d == 0 { 0.0 } else { 100.0 * n as f64 / d as f64 };
        println!(
            "{s:>5} {:>10} {:>10} {:>+9} {:>9.2}x {:>8.3}   {:>3}/{:<3} vs {:>3}/{:<3}   (share {:.2}% vs {:.2}%)",
            on.near_ticks,
            off.near_ticks,
            on.near_ticks as i64 - off.near_ticks as i64,
            if off.near_ticks == 0 { f64::INFINITY } else { on.near_ticks as f64 / off.near_ticks as f64 },
            on.along,
            on.alive_end,
            on.alive_min,
            off.alive_end,
            off.alive_min,
            share(on.near_ticks, on.ant_ticks),
            share(off.near_ticks, off.ant_ticks)
        );
        on_tot += on.near_ticks;
        off_tot += off.near_ticks;
        if on.near_ticks > off.near_ticks {
            moved_up += 1;
        }
    }
    println!(
        "\n  pooled: {on_tot} with the trail against {off_tot} without ({:+}), {moved_up} of {seeds} seeds up",
        on_tot as i64 - off_tot as i64
    );
    if gate.name == "shipped" {
        // `spec()` prints what a preset *would* write, and `shipped` writes
        // nothing -- printing its nominal numbers would be a readout that lies
        // about the genome under test.
        println!("  arena:  genome untouched -- `ant.ron` as authored (units 0/1 Bias -45 / Carrying 45.5; units 2/3 Bias 45 / Carrying -75)");
    } else {
        println!("  arena:  hidden={}", gate.spec());
    }
}
