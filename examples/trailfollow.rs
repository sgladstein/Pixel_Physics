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
//! cargo run --release --example trailfollow -- btrail btrailevery=500 stop=6000   # the food trail as a SERIES, across the hand-laid handover
//! cargo run --release --example trailfollow -- layfrom=founders       # ...with the hand-laid ramp covering the whole founding band
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
use pixel_physics::sim::creature;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::material::{MaterialId, MaterialKind};
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
        // **`CarryingFood`, since 2026-09-18, and this is a correctness fix
        // rather than a rename.** `ant.ron` re-authored both gated pairs onto
        // the food-only sensor; a preset still writing `I::Carrying` would put
        // its twelve numbers into a slot **the gate no longer reads**, leaving
        // `CarryingFood` at whatever the species file holds. The arm would run,
        // print its name, and be a no-op on the thing it claims to set -- which
        // is exactly how `gate=b2` silently re-imposed the old 0.989 threshold
        // on the first run after the fix and reported `OPEN on 0 of 26,886`.
        for (u, sign) in [(0usize, 1.0f32), (1, -1.0)] {
            v.push((I::Bias, u, self.off));
            v.push((I::CarryingFood, u, carry));
            v.push((I::PheroAAlong, u, sign * self.along));
        }
        for (u, sign) in [(2usize, 1.0f32), (3, -1.0)] {
            v.push((I::Bias, u, self.on));
            v.push((I::CarryingFood, u, -carry));
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
/// **The `Feed`-against-`Drop` contest, through the real `eval_brain`** — what
/// `(Energy, Feed, -w)` would buy, written out before anything is authored.
///
/// `creature::act` does not choose between eating and putting down by comparing
/// the two urges directly: it runs `choose_weighted(&[feed_urge, drop_urge],
/// CHOICE_EXPLORATION_K, ..)`, which scores each option as `(k + s.max(0))^2`
/// at `k = 0.1`. So the number that decides behaviour is
/// `P(drop) = (0.1+d)^2 / ((0.1+f)^2 + (0.1+d)^2)`, and that is what this
/// prints. **Quoting the raw urges instead would be quoting an input to the
/// decision rather than the decision** — `CLAUDE.md`'s "ask what your number
/// counts", in the shape where the arithmetic is one step further on than it
/// looks.
///
/// **Through `eval_brain` with the shipped genome, never hand-summed from the
/// `.ron`.** `ant.ron` wires through hidden units, a weight under `brain::W_EPS`
/// is no connection at all, and §Z5's dead odometer was "verified" by exactly
/// the side-simulation this avoids.
///
/// `FoodAdjacent` is pinned at 1.0 throughout: with nothing to eat the contest
/// does not arise, so the interesting rows are the ones where it does.
fn feed_gate(base: &[f32], candidates: &[f32]) {
    use brain::BrainInput as BI;
    use brain::BrainOutput as BO;
    let energies = [0.0f32, 0.25, 0.5, 0.75, 1.0];
    println!("P(drop) PER TICK -- the contest AND the drop_urge roll, which are two gates.");
    println!("FoodAdjacent = 1.0, Carrying = 1.0 throughout. Lower means the ant holds on.\n");
    println!("Against it: one `fruit` cell is 960 J and `digest_rate` is 3.3/tick, so a cell");
    println!("needs 291 ticks in the crop to be absorbed. Expected ticks held = 1/P(drop).\n");
    println!("{:>8} {:>8} {:>9} {:>9} {:>9} {:>9} {:>9}", "w", "AtNest", "E=0.00", "E=0.25", "E=0.50", "E=0.75", "E=1.00");
    println!("{:->8} {:->8} {:->9} {:->9} {:->9} {:->9} {:->9}", "", "", "", "", "", "", "");
    for &cand in candidates {
        let mut g = base.to_vec();
        // The candidate wire. `-cand` because `Energy` is a FULLNESS reading
        // (`state.energy / start_energy`), so a hungry ant reads 0 and the
        // negative weight is what turns "empty" into "eat".
        g[brain::io_slot(BI::Energy, BO::Feed)] = -cand;
        for at_nest in [0.0f32, 1.0] {
            let mut row = Vec::new();
            for &e in &energies {
                let mut inputs = [0.0f32; brain::BRAIN_INPUTS];
                inputs[BI::Bias as usize] = 1.0;
                inputs[BI::FoodAdjacent as usize] = 1.0;
                inputs[BI::AtNest as usize] = at_nest;
                inputs[BI::Energy as usize] = e;
                // A laden ant, since putting down is only a question for one.
                inputs[BI::Carrying as usize] = 1.0;
                let mut state = [0.0f32; brain::BRAIN_HIDDEN];
                let (out, _) = brain::eval_brain(&g, &inputs, &mut state);
                let f = out[BO::Feed as usize].clamp(0.0, 1.0);
                let d = out[BO::Drop as usize].clamp(0.0, 1.0);
                let k = 0.1f32;
                let (bf, bd) = (k + f, k + d);
                // **Dropping needs BOTH gates and an earlier draft of this
                // readout printed only the first.** `creature::act` sets
                // `prefer_drop` from `choose_weighted`, and then the drop
                // branch rolls again against `drop_urge` itself
                // (`let p = drop_urge; if draw.unit_f32() < p`). So the
                // per-tick probability is the product, and the contest alone
                // overstates it by a factor of `1 / drop_urge`.
                let contest = bd * bd / (bf * bf + bd * bd);
                row.push(format!("{:>9.4}", contest * d));
            }
            println!("{:>8.2} {:>8} {}", cand, if at_nest > 0.5 { "yes" } else { "no" }, row.join(" "));
            if cand == 0.0 {
                // The shipped row is the only one worth converting, and the
                // conversion is the whole question: how long does a cell stay
                // in a crop against the 291 ticks it needs.
                let held: Vec<String> = row
                    .iter()
                    .map(|c| {
                        let pv: f32 = c.trim().parse().unwrap_or(0.0);
                        if pv <= 0.0 { "    inf".to_string() } else { format!("{:>7.0}", 1.0 / pv) }
                    })
                    .collect();
                println!("{:>8} {:>8} {}", "ticks", "held", held.join("   "));
            }
        }
    }
    println!("\nw = 0.00 is the shipped ant: `ant.ron` authors no `Energy -> Feed` wire at all,");
    println!("so hunger does not move this number by a single digit today.");
}

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
/// **A hand-laid channel A ramp peaking at the NEST** -- the homing signal the
/// ant's own odometer is supposed to produce, supplied from outside.
///
/// This exists to test a causal chain the owner named, 2026-09-16: channel B is
/// laid only by laden ants, so if laden ants do not walk home they draw a
/// wander-field rather than a route, and no follower can climb it. Measured and
/// it holds -- laden movement is **-418 net homeward cells over 2,146,526
/// carrying ant-ticks**, i.e. no homeward component at all.
///
/// If that is the whole story, then handing the colony a working homing
/// gradient should let it lay a route-shaped channel B of its own and start
/// recruiting -- which no arm has ever done. If it still does not, homing is
/// not sufficient and something else is missing too.
///
/// Mirror image of `lay`: `t` falls from 1 at the nest to 0 at the food, which
/// is the shape `EmitA` off a nest-charged, distance-decaying unit 4 would
/// draw. The ants' own `EmitA` is left alone in every arm -- only `EmitB` is
/// ever muted -- so this supplements their homing rather than replacing it.
fn lay_home(w: &mut pixel_physics::sim::world::World, nest_x: i32, target_x: i32, surface: i32) {
    for x in nest_x..=target_x {
        let t = 1.0 - (x - nest_x) as f32 / (target_x - nest_x) as f32;
        let amount = (t * pheromone::DEPOSIT as f32) as pheromone::Scent;
        for y in (surface - 3)..=(surface + 1) {
            w.deposit_pheromone(Channel::A, x, y, amount);
        }
    }
}

/// Constant amplitude over part of the route -- **the polarity metric's
/// NEGATIVE control, and the thing it has never had.**
///
/// `homeA` is the metric's positive control and reads +0.115 against an
/// independent prediction of +0.12, which is why it was trusted. But it fills
/// **91 of 91 route cells**, so `here > 0` and `ahead > 0` coincide everywhere
/// and it is *blind by construction* to the defect §5 item 5 accuses the metric
/// of: the admission gate is `||`, so a cell on the edge of a blob is scored
/// against an empty neighbour and reads about -0.97, against an interior cell's
/// +-0.03..0.07. One edge cell is worth roughly twenty interior ones.
/// `CLAUDE.md`: a guard must be able to fail for the *replacement* artifact,
/// and a positive control checks specificity where this needs sensitivity.
///
/// **A flat blob has no gradient anywhere in it, so a correct metric reads 0.**
/// What the `||` metric reads instead is arithmetic rather than opinion, and it
/// is worth writing down before running it, because a prediction that lands is
/// evidence and one written afterwards is a story:
///
/// - `FlatFood` -- constant over the food half. The scan runs `nest_x ..=
///   target_x - sensor_offset`, so the blob's **leading** edge is inside it
///   (`here = 0`, `ahead = A`, about -1) for `sensor_offset` cells and its
///   trailing edge is not, because `ahead` never leaves the painted region.
///   Six cells at -1 over ~51 admitted gives about **-0.12**.
/// - `FlatNest` -- constant over the nest half. Mirror image: only the
///   **trailing** edge is inside the scan (`here = A`, `ahead = 0`, about +1),
///   so about **+0.12**.
///
/// Those two numbers bracket every polarity figure in the report -- a colony
/// that stays home reads +0.018..+0.052, one that forages reads -0.076 -- and
/// they are produced here by a field with **no ramp in it at all**. If they
/// come out, the `||` metric cannot tell a ramp from a blob's position and §7.15
/// falls.
fn lay_flat(w: &mut pixel_physics::sim::world::World, from_x: i32, to_x: i32, surface: i32) {
    for x in from_x..=to_x {
        for y in (surface - 3)..=(surface + 1) {
            w.deposit_pheromone(Channel::A, x, y, pheromone::DEPOSIT as pheromone::Scent);
        }
    }
}

/// Which channel-A pattern this harness paints, if any.
///
/// Was a `home: bool`. It became an enum when the metric acquired a negative
/// control, because "not the homing ramp" stopped meaning "nothing".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PaintA {
    /// The ants' own channel A and nothing else -- every treatment arm.
    None,
    /// `lay_home`'s nest-ward ramp: the metric's POSITIVE control, ~ +0.115.
    Ramp,
    /// Flat over the nest half; the ants' `EmitA` is muted. Predicts ~ +0.12.
    FlatNest,
    /// Flat over the food half; the ants' `EmitA` is muted. Predicts ~ -0.12.
    FlatFood,
}

/// **Every term of `Move`'s pre-squash sum, by name** — the "why" behind one
/// decision, rather than the verdict.
///
/// An output is `squash(sum io[out][i]*inputs[i] + sum ho[out][h]*hidden[h])`
/// (`brain::eval_brain`), and the shipped ant's trail reader lives **entirely in
/// the second half**: `ant.ron` authors `(PheroAAlong, 0, +6.0)` /
/// `(PheroAAlong, 1, -6.0)` into hidden units 0/1 and `(0, Move, +2.5)` /
/// `(1, Move, -2.5)` out of them, and `PheroAAlong` reaches `Move` through **no
/// direct wire at all**. So a decomposition that lists only the input terms —
/// which is all `creature::probe` can give — shows every reason the ant moved
/// *except the trail*, and a low `Move` cannot be told apart from a trail term
/// that is absent, weak, or outvoted.
///
/// **`W_EPS` is applied here because `eval_brain` applies it.** A weight under
/// it is no connection at all, and a decomposition that included those terms
/// would not sum to the number the brain computed — which is the point, since
/// that sum is this function's own check (`squash(sum)` must reproduce
/// `outputs[Move]`, and the caller asserts it).
fn move_terms(
    genome: &[f32],
    inputs: &[f32; brain::BRAIN_INPUTS],
    hidden: &[f32; brain::BRAIN_HIDDEN],
) -> (Vec<(String, f32)>, f32) {
    let mut terms = Vec::new();
    let mut sum = 0.0f32;
    for i in 0..brain::BRAIN_INPUTS {
        let w = genome[brain::io_slot(brain::INPUTS[i], O::Move)];
        if w.abs() >= brain::W_EPS {
            let t = w * inputs[i];
            sum += t;
            terms.push((brain::INPUT_NAMES[i].to_string(), t));
        }
    }
    for h in 0..brain::BRAIN_HIDDEN {
        let w = genome[brain::ho_slot(h, O::Move)];
        if w.abs() >= brain::W_EPS {
            let t = w * hidden[h];
            sum += t;
            terms.push((format!("h{h}"), t));
        }
    }
    (terms, sum)
}

/// **The hand-laid food ramp, from `foot_x` up to the larder.**
///
/// `foot_x` is where the ramp's *zero* sits and is not always the nest. The
/// bed founds its colony across a band centred on `nest_x` -- at 20 ants,
/// `x 12..88` around a cursor at 48 -- while this function used to run
/// `nest_x..=target_x`, so **every cell west of the nest carried no channel B
/// at all**. Measured 2026-09-21 on the projection-on arm, 240 ants split on
/// that line:
///
/// | born | reached food | closed two laps | median life |
/// |---|---|---|---|
/// | west of `nest_x` | 40% | 1% | 979 ticks |
/// | on or east of it | 98% | 23% | 3,643 ticks |
///
/// That is a cliff rather than a gradient, and it is the single strongest
/// predictor of an ant's whole life in this bed -- **a property of the scene,
/// not of the colony**, and `CLAUDE.md`'s *a scene that contradicts the code
/// will look like a bug in the code*. Every "half the colony never reaches the
/// food" figure this harness has ever printed, §Z32's included, is
/// substantially this.
///
/// **`layfrom=founders` is the repair and it is opt-in**, because moving the
/// foot changes the ramp's *slope* -- the same `DEPOSIT` spread over a longer
/// span -- so an arm that took it silently would not be comparable with any
/// row measured before today. Default stays `nest`.
///
/// **And it moves two things at once, which the arm's name does not say.**
/// Extending the foot west also lifts the ramp's whole floor, because `t` is
/// renormalised over the longer span: at 20 ants and gap 90 the cell *at the
/// nest cursor* goes from `t = 0` -- an amount of exactly **zero**, so the
/// founders standing on the nest read no trail either -- to `t = 0.29`, about
/// **29% of `DEPOSIT`**. So this arm is not only "the trail reaches further
/// west"; it is also "the trail is readable at the nest at all", and that may
/// well be the larger half. Separating them needs a third arm that extends the
/// foot while holding the slope, and nothing here has measured which half
/// carries the effect -- do not report this arm as though it had.
///
/// **Channel A is deliberately left alone.** The homing ramp (`lay_home`) is
/// what an ant reads walking *back*, and an ant that has not yet reached the
/// food has no homeward leg to be lost on. The stage this fixes is the walk
/// *out*, which is channel B's.
fn lay(w: &mut pixel_physics::sim::world::World, foot_x: i32, target_x: i32, surface: i32) {
    let span = (target_x - foot_x).max(1) as f32;
    for x in foot_x..=target_x {
        let t = (x - foot_x) as f32 / span;
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
    /// **What those cells were worth to this ant**, in joules -- the
    /// provisioning denominator, and the number whose absence produced two
    /// wrong published claims on this branch.
    ///
    /// It is `larder_placed * creature::diet_yield(larder, gut)`, so it carries
    /// the gut filter that face value drops. At the shipped **neutral** gut
    /// `diet_quality` is 0.25 against either end of the food-class axis, which
    /// is a 4x difference from the face value in the material table -- and
    /// reading face value is exactly how `food=200 refill=4000` was published
    /// as 168,000 J when it was about 42,000 J, against a need near 46,800 J
    /// for 52 ants over 24,000 frames. Print the denominator beside the intake
    /// and a starving colony stops looking like a deaf one.
    ///
    /// **It is a nominal figure, not a ceiling, and `ate J` may legitimately
    /// exceed it.** The gut is heritable -- `ant.ron`'s `trait_variance` slot 0
    /// is 0.15 -- so an individual whose gut has drifted toward the plant end
    /// draws more from a class -1.0 larder than the species' authored 0.0 does,
    /// up to 4x at a perfectly matched gut. This column prices the larder at
    /// the **authored** gut because that is the number a scene is designed
    /// against; read it as "roughly how much was put out", never as a
    /// conservation bound.
    supply_j: f64,
    /// **Joules the colony ate off the larder specifically**, from
    /// `ColonyBooks::diet()` -- the provisioning measure `deliveries` cannot be.
    ///
    /// `CreatureStats::deliveries` increments on *any* drop while `at_nest`
    /// (`creature.rs:8012`), whatever was dropped and wherever it came from, so
    /// in a scene where the colony dies in its own nest it mostly counts corpse
    /// shuffling. The disproof is in this harness's own gap sweep: at gap 300
    /// `near on` is **0 in all six seeds** -- not one ant ever came within ten
    /// cells of the food -- while `deliv on` reads 0, 0, 6, 4, 13, 10.
    ///
    /// It is attributable twice over: the diet band names the material, and
    /// with `onlyfood` set the larder is the only thing in the world with a
    /// non-zero food value anyway. **The known-zero control passes**: at
    /// `food=1`, a 240 J larder, both arms read exactly 0 over 12,000 frames
    /// with 52 ants -- so when this column is non-zero it is the larder and
    /// nothing else.
    ///
    /// **It replaced a cell census that was counting rot.** The first version
    /// of this column was `placed - still standing`, which looked principled
    /// and read **240/300 in all four arms of a two-seed control** -- exactly
    /// `CLAUDE.md`'s tidiness tell. `windfall.ron` sets `decays_into: "soil"`,
    /// so the larder rots on its own schedule whether or not an ant is alive to
    /// eat it, and a colony that was already dead scored the same "intake" as
    /// one that was not. The ledger cannot be fooled that way: rot books
    /// nowhere.
    eaten_j: f64,
    /// **Joules eaten off anything that is not the larder, which under
    /// `onlyfood` must be exactly 0** -- the isolation check, printed rather
    /// than asserted so a leak is visible in the table instead of killing a
    /// sweep.
    ///
    /// Read off `ColonyBooks::diet()`, which books intake **by the material it
    /// came out of**. That is a strictly stronger check than the
    /// `EnergyLedger::harvested_corpse` column it replaced: that one could only
    /// see the `worth_in_aux` branch, so it read a clean 0 while any other
    /// material would have gone unnoticed.
    ate_other_j: f64,
    /// **Where the colony actually started, as (min x, max x) at frame 1.**
    ///
    /// Not a curiosity: `World::colony_stations` walks *outward* from the
    /// cursor taking the first column that is a site, so when the world runs
    /// out on the left every remaining founder is placed to the right. At
    /// `nest_x = 40` in a 256-wide box, 52 ants at the colony spacing cannot
    /// fit on the left, and the band runs far enough right to **reach the food
    /// it is supposed to be walking to**. `CLAUDE.md`: check the scene still
    /// contains the situation you think it does, before touching the mechanism.
    founded: (i32, i32),
    /// Where the nest and the food actually sit, so `founded` can be read as a
    /// **distance** rather than as two bare coordinates.
    ///
    /// **`gap=` is the NEST-to-food distance, and the ants are not at the
    /// nest.** `found_colony_of` lays them in a band about `ants * 4` wide
    /// centred on the nest, so at 20 ants and `gap=90` the colony spans
    /// x 12..88 against food at 138 — the nearest founder is **50 cells out,
    /// not 90**, and nothing in this harness's output said so. The existing
    /// assertion only catches the extreme case where a founder lands *on* the
    /// larder; between "on it" and "a gap away" there is a whole range this
    /// prints rather than implies.
    nest_x: i32,
    /// **Where the nest MATERIAL is, and how many founders were born on it.**
    ///
    /// `nest_x` above is the founding cursor and reads like a location; the
    /// comb `paint_nest_patch` lays from it spans `+/- 26` columns with gaps,
    /// so the two are different facts and only these are the one that decides
    /// whether an ant has a home. See the census that builds them.
    nest_span: (i32, i32),
    founders_on_nest: usize,
    target_x: i32,
    /// **Frame the first ant reached the food, or 0 for never** -- and the
    /// column that separates "cannot follow the trail" from "died on the way".
    ///
    /// Owner's question, 2026-09-16: how long does an ant live without food,
    /// how long should it take to walk 220 or 300 cells, and could they be
    /// following the trail and dying before they arrive? Nothing measured so
    /// far could answer it: a zero at gap 300 reads identically for an ant that
    /// ignored the trail and an ant that walked it until it starved. This and
    /// `all_dead_frame` below are the pair that tell them apart -- **read them
    /// together**, because either alone is ambiguous.
    first_arrival: u64,
    /// **Frame the colony hit zero, or 0 if it was still alive at the end.**
    /// The budget side of the same question: if this lands before an arrival
    /// could plausibly have happened, distance is not being tested at all --
    /// lifespan is.
    all_dead_frame: u64,
    /// **Creature sites the scheduler actually dispatched** -- the
    /// "was it asked at all" counter. A colony that survives while eating
    /// nothing is either being fed or is not being ticked, and only this
    /// separates them.
    ticks: u64,
    /// **Nest-material cells in the world, and the share of sampled ant-ticks
    /// that read `AtNest`** -- the "did it fire at all" counter the homing
    /// mechanism never had.
    ///
    /// `AtNest` is `adjacent_nest`, an 8-neighbour test for `nest` material
    /// (`creature.rs:7048`). It is the ONLY thing that charges hidden unit 4,
    /// the odometer that lays channel A. If it is rarely true the odometer
    /// never charges, no channel A is laid, and the homing reader -- which
    /// works, and lifts `P(move)` 0.200 -> 0.641 on a real gradient -- has
    /// nothing to read. Measured before assuming the odometer's range was the
    /// problem, because a flat ramp and an absent one want different fixes.
    nest_cells: usize,
    atnest_ticks: u64,
    probe_ticks: u64,
    /// **Highest channel A ever seen on the route, and the most route cells
    /// ever holding any** -- running maxima, because an end-of-run sample
    /// cannot tell "never laid" from "laid and decayed".
    a_peak_amt: u32,
    a_peak_cells: usize,
    /// **Which way the ants' channel A ramp points, averaged over the run.**
    /// Positive = taller at the NEST, the shape homing needs. Negative = taller
    /// at the FOOD, i.e. a homing reader ascending it is driven away from home.
    a_polarity: f32,
    /// **The same figure with the admission gate tightened from `||` to `&&`,
    /// and the mean number of cells the difference rests on.**
    ///
    /// Read them together and against the `flatN`/`flatF` controls. If
    /// `a_polarity` and `a_polarity_both` agree, the `||` gate is harmless here
    /// and §3's magnitudes are real. If they disagree by about the amount the
    /// flat controls predict (~ -+0.12 from no ramp at all), the inversion is
    /// the blob's *position* being read as a *shape*.
    a_polarity_both: f32,
    a_edge_cells: f32,
    /// **The polarity figure that passes both of its controls, and the only one
    /// to quote.**
    ///
    /// Measured 2026-09-17 on `arms=homeA,flatN,flatF`, 6 seeds, gap 90,
    /// `refill=2000`. A FLAT channel-A blob -- no ramp in it anywhere -- read
    /// **+0.19498 over the nest half and -0.19498 over the food half** under
    /// the `||` gate, and `&&` moved that only to **+-0.18061**. Both magnitudes
    /// are LARGER than a perfect nest-ward ramp's +0.115 and far larger than the
    /// -0.076 that `pheromone-master-2026-09-17.md` §3.5 reports as "the ramp
    /// inverts". So the defect is not the admission gate §5 item 5 names: it is
    /// that the scan window is fixed to the route while the trail is not, and a
    /// blob's two shoulders (put there by `DIFFUSE`, genuine interior
    /// gradients) are counted asymmetrically according to where the blob sits.
    ///
    /// Anchoring the window to the trail's own `[lo, hi]` removes it. Keep all
    /// three columns: the first two are what the report's numbers were taken
    /// with, and deleting them would make this file disagree with its own
    /// archived logs for no stated reason.
    a_polarity_span: f32,
    /// **The ants' own channel A ramp, sampled at five points along the route
    /// and as the gradient a real reader computes.** See the fill site for the
    /// arithmetic that predicts it is two orders of magnitude too flat to read.
    a_profile: [u32; 5],
    /// Channel B at the same five points -- the column that says whether the
    /// colony's own food trail peaks at the FOOD or at the NEST, which
    /// `route pk` and `along` both average away.
    ///
    /// **It is a MEAN OVER THE WHOLE RUN and `a_profile` beside it is not** --
    /// that one is an instantaneous end-of-run sample. Same name shape, two
    /// different quantities, and the difference decides what the column can be
    /// asked. Measured 2026-09-21: with `stop=6000` in a 24,000-frame bed, 60
    /// of the 240 samples are taken while the HAND-LAID ramp is still being
    /// refreshed at full `DEPOSIT`, which is far above anything the ants lay --
    /// so this column prints healthy numbers for a run whose plane is empty
    /// for its last 18,000 frames. **A healthy `b_profile` is therefore not
    /// evidence against the trail having decayed away**, and reading it as one
    /// is `CLAUDE.md`'s *ask what your number counts*. For the current
    /// reading, and for the hand-laid-to-ant-laid handover, use `btrail=`.
    b_profile: [u32; 5],
    /// Times a body traded places with a nestmate -- the "did it fire" counter
    /// for `kinpass`, which must read 0 when the switch is off.
    kin_swaps: u64,
    /// Blocked move attempts, as the thing `kin_swaps` is meant to reduce.
    blocked: u64,
    /// Heading re-rolls, and the share of them `home_bias` aimed at the nest --
    /// the "did it fire" counter for the fill-weighted tumble, which must read
    /// **0** at the shipped `home_bias: 0.0`. Printed as a pair because the
    /// aim alone cannot say whether the colony is commuting: a homeward tumble
    /// into a wall and one down an open corridor count the same here, and what
    /// came of them is `P(home)` per crop-fill bin in the trace.
    tumbles: u64,
    tumbles_homeward: u64,
    /// Cells put back down out of the crop.
    ///
    /// **A drop no longer forfeits anything — 2026-09-20.** It used to: the
    /// crop paid out in a 291-tick lump, so putting a cell down at tick 290
    /// threw away the whole meal, and `digest_parked`/`digest_resumed` existed
    /// to repair that by parking the remainder. Under the continuous payout
    /// the animal has already been credited every joule it chewed, the cell
    /// leaves at `unit - digesting`, and there is no remainder to park -- so
    /// those three counters retired with the mechanism they measured.
    ///
    /// **Read it against the per-commute trace, never alone.** Measured 12
    /// seeds, 240 ants: **1,998 of 2,132 laden legs are 12-frame pickups and
    /// putdowns at the comb**, so this column is dominated by nest loitering
    /// and a change can move it 64% while real commutes fall.
    drops: u64,
    /// **What the appetite gate withheld, against what the gut actually
    /// absorbed** -- the pair that says whether scaling the rate by hunger did
    /// anything. Held near zero with `digested_face` healthy means the colony
    /// was hungry throughout and the gate never had a surplus to protect,
    /// which is a true reading of a starving bed rather than a dead mechanism.
    digest_appetite_held: f64,
    digested_face: f64,

    /// Distinct ants that ever came within `near` of the food -- recruitment.
    visitors: usize,
    /// Distinct ants that ever lived in this run, as the denominator.
    ants_seen: usize,
    /// **Round trips split by whether the ant was born on the comb** -- the
    /// within-run control for §7.37. An ant born off it homes to its own birth
    /// cell for life, so if the anchor is the blocker these two diverge; if
    /// they do not, the anchor is real and is not what is stopping the loop.
    trips_on_nest: u64,
    trips_off_nest: u64,
    /// The denominator for the pair above -- trips alone cannot be read without
    /// how many ants were in each group.
    ants_on_nest: usize,
    /// Honest round trips summed over the colony. See `Track::trips`.
    round_trips: u64,
    /// **The homeward leg in frames: n / median / p90, over closed round
    /// trips**, and the same for the subset that closed with larder in the
    /// crop. Medians rather than means because the tail here is long.
    ///
    /// `leg_n` is the pairing `CLAUDE.md` asks for: it must equal
    /// `round_trips`, and a gap between them means legs are being dropped
    /// rather than journeys being short.
    /// **The return ledger.** `reached` is ants that got within `near` of the
    /// food at all; `returned` is those that then reached the nest band; the
    /// last two split those returns by whether anything was being carried.
    read_ok: u64,
    read_away: u64,
    read_n: u64,
    lit: u64,
    lit_n: u64,
    reached: usize,
    returned: usize,
    /// **Distinct ants that completed the loop at least once, and the ones
    /// that did it more than once.** `trips_laden` is a sum over ants, so it
    /// cannot tell eight ants doing one loop from two ants doing four -- and
    /// "is this a repeating loop or a one-off" is exactly the question the
    /// sum hides. Asked by the owner 2026-09-20; nothing in the harness could
    /// answer it before.
    /// **The foraging loop as a funnel** -- `funnel[n]` is how many ants ever
    /// reached stage `n` or beyond, so `funnel[0]` is every ant that lived.
    /// Monotone and per ANT, never per event: see `Track::stage`.
    funnel: [usize; FUNNEL.len()],
    loopers: usize,
    repeat_loopers: usize,
    max_loops: u32,
    /// **The within-run control §7.37 asks for, and the test it says is next.**
    /// That section established that nine of twenty founders are born off the
    /// comb and carry `forage_anchor` = their birth cell for life, then says
    /// plainly what it does *not* explain: *"those founders are born on the
    /// comb, anchor correctly, and the loop still does not close for them. So
    /// the anchor is a real defect and not, by itself, the blocker."*
    ///
    /// Splitting the loop count by birth site answers that inside one run,
    /// which cancels seed, supply, gap and crowding together -- a narrow-bed
    /// arm changes all four at once. If the two groups complete the loop at
    /// the same rate the anchor is not what blocks it; if the born-on-comb
    /// group runs away with it, it is.
    loopers_on_comb: usize,
    loopers_off_comb: usize,
    loops_anchor_ok: u64,
    loops_anchor_bad: u64,
    pickups_anchor_ok: u64,
    pickups_anchor_bad: u64,
    reached_on_comb: usize,
    reached_off_comb: usize,
    trips_laden: u64,
    trips_empty: u64,
    leg_n: usize,
    leg_med: u64,
    leg_p90: u64,
    laden_leg_n: usize,
    laden_leg_med: u64,
    laden_leg_p90: u64,
    /// **The raw legs, because a median of per-run medians is not a median.**
    /// Closed round trips are rare here -- tens across a whole 18-seed sweep --
    /// so each run contributes one or two samples and re-medianing the per-run
    /// figures reports the typical *run* rather than the typical journey. These
    /// are emitted so the pool can be taken across seeds where it belongs.
    legs_raw: Vec<u64>,
    laden_raw: Vec<u64>,
    /// Excursion histogram: how many ants got 0-25 / 25-50 / 50-75 / 75-100 /
    /// over 100 percent of the way to the food, by their furthest point.
    reach: [usize; 5],
    /// **Net cells moved while carrying larder, signed toward the nest.**
    ///
    /// The direct answer to "why does food not come home": if carrying is
    /// homeward-directed this is strongly positive, and if an ant picks food up
    /// and wanders it sits near zero. A `carry@nest` of 0 says food does not
    /// arrive; this says whether it was ever *aimed* here.
    carry_toward_nest: i64,
    /// **Where the colony actually spends its time**, as ant-ticks in eight
    /// equal bands from the nest to the food. Band 0 holds the nest, band 7 the
    /// food, and anything past the food lands in band 7 as well.
    ///
    /// The one readout that shows dispersal and commuting apart at a glance:
    /// a commuting colony is bimodal with mass at both ends, a dispersing one
    /// slides its mass outward, and a colony that never leaves is a spike at 0.
    occupancy: [u64; 8],
    /// Births and deaths over the run. `alive` alone cannot tell a stable
    /// colony from one churning hard, and those are different worlds.
    births: u64,
    deaths: u64,
    /// Deaths attributed to starvation, the cause that matters for the
    /// "did they die on the way" question.
    starved: u64,
    /// **Ant-ticks spent carrying larder in the crop** -- the transport
    /// counter, and the one that says whether food moves at all.
    ///
    /// `larder_home_peak` below counts larder *cells standing* in the nest
    /// band, and it reads 0 or 1 almost everywhere, which has two readings: no
    /// food comes home, or food comes home and is eaten before any sample sees
    /// it standing. A cell census cannot separate those. Crop contents can: an
    /// ant inside the nest band with larder in its crop has carried it there,
    /// whatever happens to it next.
    carry_ticks: u64,
    /// ...of which, inside the nest band. **This is "is food being carried back
    /// to the nest", measured.** Against `carry_ticks` it is the share of
    /// transport that ends up at home rather than circling the patch.
    carry_home_ticks: u64,
    /// Route cells still holding any channel B at the end. With `stop=` set,
    /// this is the *ants'* trail -- the hand-laid one stopped being refreshed
    /// at frame `stop` and a cell laid at `DEPOSIT` dies in ~144 frames.
    live_cells: usize,
    /// Peak route cells alive at any sample after laying stopped.
    ///
    /// **Read against the `hmute` arm, never alone.** The sampling window does
    /// not clear the hand-laid trail -- see the arm table in `main` -- so this
    /// column includes our own decaying deposit. `hmute` is that deposit with
    /// the ants silenced, and the difference is what the colony maintained.
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
    /// **Ant-ticks holding anything at all** -- the denominator `spoil_ticks`
    /// is meaningless without.
    ///
    /// `Carrying` is what gates the channel A reader (units 0/1) *and* drives
    /// the channel B emitter, and it is true for dig spoil as well as food. So
    /// `spoil_ticks / laden_ticks` is the share of both mechanisms that is
    /// being driven by tailings rather than by forage -- the number behind the
    /// `homeA` arm clipping exploration, since a scene with no food to find
    /// makes spoil nearly the whole of `Carrying`.
    laden_ticks: u64,
}

/// One arm: one seed, trail on or off, one gate.
// Eight against clippy's ceiling of seven: these are the arm's axes, and a
// struct would hide that each one is independently swept.
/// **What the colony is allowed to eat**, and why it is a knob rather than a
/// constant.
///
/// Owner's question, 2026-09-16: *"can we turn off corpses (or anything other
/// than the intentionally placed larder) counting as food, so that ants can
/// only eat what we intend and we can only record if they are eating what we
/// intend?"* Yes -- and the arithmetic it exposes is worse than the tidiness
/// problem it was asked about.
///
/// At the shipped ant's **neutral gut** (`ant.ron` `traits` slot 0 = 0.0)
/// `creature::diet_quality` returns `(1 - |0 - class|/2)^2` = **0.25** against
/// either end of the axis, so a cell is worth a quarter of its face value. The
/// two kinds of corpse in this scene are then not worth the same thing at all:
///
/// | cell | priced by | face | yield at a neutral gut |
/// |---|---|---|---|
/// | larder placed here, `Cell::new(corpse, 0)` | `food_energy`, since `aux` is 0 | 120 | **30 J** |
/// | a dead ant's corpse | its `aux` stamp, `body_energy` | 480 | **120 J** |
///
/// **A dead nestmate is worth four larder cells.** So every `mode=gap` row
/// recorded before this knob existed was a colony with a richer food source
/// lying inside its own nest than the one it was being asked to walk to -- and
/// the energy figure published for those runs took face value and dropped the
/// 0.25 entirely: `food=200 refill=4000` is about **42,000 J**, not the
/// 168,000 J claimed, against a need near 46,800 J. Under-provisioned, not
/// 3.6x over.
#[derive(Clone, Copy)]
struct Diet {
    /// The material placed at the target. **Never `corpse`** when `only` is
    /// set -- see `isolate`.
    larder: &'static str,
    /// Zero every other material's food value, so intake is attributable.
    only: bool,
}

impl Diet {
    /// **Make the placed larder the only food in the world.**
    ///
    /// Two steps, and the second is what a `food_energy = 0.0` sweep on its own
    /// gets wrong. `creature::food_value` prefers the *cell's* `aux` stamp
    /// wherever the material sets `worth_in_aux`, and **`corpse` is the only
    /// material in the tree that sets it**. A starved ant's corpse is stamped
    /// with its `body_energy` (480) at death, so zeroing `corpse.food_energy`
    /// leaves every corpse in the world worth exactly what it was worth before.
    /// The flag has to come off too, and that is not a detail: it is the whole
    /// difference between isolating the larder and appearing to.
    ///
    /// Enumerated from the table rather than listed by name, per `CLAUDE.md`'s
    /// "adding a member to a set enrols it in every rule over that set" -- a
    /// food added tomorrow is covered without anyone remembering to come back.
    fn isolate(self, w: &mut pixel_physics::sim::world::World, larder: MaterialId) {
        if !self.only {
            return;
        }
        let dead = w.materials.id_of("corpse").expect("corpse is compiled in");
        assert_ne!(
            larder, dead,
            "larder=corpse defeats onlyfood: with `worth_in_aux` cleared a dead ant's corpse is \
             priced by the same `food_energy` as a placed larder cell, so the colony still eats \
             itself and intake is still unattributable. Use a material the ants cannot produce -- \
             fruit is the default for exactly this reason."
        );
        let keep = w.materials.get(larder).food_energy;
        assert!(keep > 0.0, "the larder material carries no food_energy; there would be nothing to eat");
        for id in (0..w.materials.len() as u16).map(MaterialId) {
            let m = w.materials.get_mut(id);
            m.food_energy = 0.0;
            m.worth_in_aux = false;
        }
        w.materials.get_mut(larder).food_energy = keep;
    }
}

/// **What the colony ate, split into the larder and everything else.**
///
/// Summed over every colony's books rather than read off `EnergyLedger`,
/// because the ledger's `harvested_plant` cannot say *what* was eaten -- and
/// because it sums two paths booked on different bases. The birth path
/// consumes cells outright without their passing through a crop
/// (`creature.rs:20982`), so a ledger figure can exceed a `diet_yield`-based
/// supply estimate and look like a leak when nothing has leaked. Measured:
/// `harvested_plant` 59,003 J against 200 placed cells worth 48,000 J.
fn diet_by_material(w: &pixel_physics::sim::world::World, larder: MaterialId) -> (f64, f64) {
    let (mut mine, mut other) = (0.0, 0.0);
    for books in w.all_colony_books() {
        for (m, j) in books.diet() {
            if m == larder {
                mine += j;
            } else {
                other += j;
            }
        }
    }
    (mine, other)
}

/// **What one ant did over its whole life**, kept per `OrganismId` so the
/// colony-level counters can be read as behaviour rather than as exposure.
///
/// The counter this exists to replace is `near_ticks`, which sums ant-ticks
/// within `near=` of the food and therefore **cannot tell fifty ants visiting
/// once from one ant standing there for the whole run** -- `CLAUDE.md`'s "a
/// mean over events is not a mean over the thing you care about", in the shape
/// that matters most here. Recruitment is the entire stigmergic claim: one
/// scout finds food, lays a trail, and *many* follow. Only a distinct-ant count
/// can see it.
/// **One bin of the response-vs-fill curve: what a laden ant actually DID, at
/// this much crop in it.**
///
/// The `Carrying` histogram this replaces counted how often an ant was at each
/// fill and threw the outcome away, so it could say the modal laden ant sits at
/// 0.7 and nothing at all about whether 0.7 behaves differently from 0.3. The
/// question the curve exists for is the owner's: **a full ant should be
/// near-certain to head home and a half-full one about half as likely**, which
/// is a claim about a *shape over fill* and is unreadable from any aggregate.
///
/// `home`/`away` are steps, not ticks -- `P(move)` is the brain's output and
/// these are what came of it, which is `CLAUDE.md`'s "pair every 'it fired'
/// counter with an effect counter from the far side of the call". A bin can
/// have a high `P(move)` and no net displacement if the ant is stepping
/// somewhere that is not home, and that is exactly the failure being hunted.
#[derive(Default, Clone, Copy)]
struct FillBin {
    /// Decisions landing in this bin.
    n: u64,
    /// Sum of `P(move)`, the brain's own step probability.
    p_move: f64,
    /// Sum of signed cells homeward (`+1` toward the nest, `-1` away).
    dx: i64,
    /// Steps that went homeward.
    home: u64,
    /// Steps that went away from the nest.
    away: u64,
}

impl FillBin {
    fn add(&mut self, p_move: f64, dx: i32) {
        self.n += 1;
        self.p_move += p_move;
        self.dx += dx as i64;
        if dx > 0 {
            self.home += 1;
        } else if dx < 0 {
            self.away += 1;
        }
    }
}

/// **The stages of the foraging loop, in order.** An ant is booked at the
/// furthest one it ever reached, so each row is a subset of the one above and
/// the percentages compose.
const FUNNEL: [&str; 8] = [
    "lived",
    "reached the food",
    "picked food up out there",
    "turned for home with it",
    "got back to the nest still holding it",
    "PUT IT DOWN there",
    "went back out again",
    "reached the food a SECOND time",
];

#[derive(Default, Clone, Copy)]
struct Track {
    /// Furthest this ant ever got from the nest, in cells. The excursion
    /// histogram is built from these, and it answers a question no total can:
    /// whether a colony has a commuting *population* or two wanderers and a
    /// crowd at home.
    far: i32,
    /// Did it ever come within `near` of the food.
    visited: bool,
    /// Currently outbound, i.e. has reached the food and not yet been home.
    outbound: bool,
    /// Last x seen, so the step between samples can be signed.
    last_x: i32,
    /// Whether `last_x` has been set (an ant's first sample has no step).
    seen: bool,
    /// **Honest round trips: nest band -> within `near` of the food -> nest
    /// band.** Counted here rather than read off `CreatureStats::forage_trips`
    /// because this harness can state its own definition, and the loop is the
    /// thing the whole investigation is about.
    ///
    /// **`at_nest` here is a +/-26 band around `nest_x`, which is NOT what the
    /// ant's own `AtNest` sensor reads.** `creature.rs` answers that with
    /// `adjacent_nest`, an 8-neighbour test for nest *material*, and the comb
    /// has gaps. So a trip can close on a column the animal itself never
    /// registered as home. Kept because it is the generous reading and a
    /// generous reading of zero is still zero -- but it must not be quoted as
    /// the ant's experience of arriving.
    trips: u32,
    /// **Was this ant born within one cell of nest material**, and where.
    ///
    /// The split that separates "the anchor is the blocker" from "the anchor is
    /// one of several". `forage_anchor` is the birth cell, so an ant born off
    /// the comb homes to a private wrong place for life (§7.37) -- and eleven of
    /// twenty are born on it, so the colony contains its own control. Comparing
    /// the two groups inside one run cancels seed, supply, gap and crowding,
    /// which a colony-size ladder would confound with all four.
    born_on_nest: bool,
    born_x: i32,
    /// **The frame of the most recent sample within `near` of the food** — the
    /// start of the walk home, and half of the only direct measurement of the
    /// return leg this line has.
    ///
    /// Updated on *every* frame the ant is at the food rather than the first,
    /// so at trip close it holds the moment the animal actually left the
    /// larder. Taking the first arrival instead measures the visit plus the
    /// walk, which on a bed where ants linger at food is mostly the visit.
    left_food: u64,
    /// **Round trips that closed with larder in the crop, and without.** The
    /// pair answers the question `trips` alone cannot: an ant that reaches the
    /// food, turns round and walks home EMPTY has made a round trip and
    /// provisioned nothing. Counting those as foraging is how an exposure
    /// number turns into a foraging one.
    trips_laden: u32,
    trips_empty: u32,
    /// The frame this ant last picked larder up, `0` for never. Paired with
    /// `left_food` so the leg can be split by whether there was anything in
    /// the crop to carry — a walk home with an empty crop is not the laden
    /// leg, and averaging the two together is how a journey turns into a
    /// number about wandering.
    laden_since: u64,
    /// **Was `forage_anchor` actually on nest material when this ant picked up
    /// its load** -- the causal variable, which birth site is NOT.
    ///
    /// The 2026-09-20 within-run control split loop completions by
    /// `born_on_nest` and read the result as "the anchor is not the blocker".
    /// That split is invalid and the reason is one line of `creature.rs`:
    /// `forage_anchor` is **re-set on every nest contact**, so an off-comb-born
    /// ant that touches the comb once carries a CORRECT anchor from then on.
    /// The born-off group is therefore contaminated with corrected ants -- and
    /// they are the ones likeliest to complete a lap, which biases that group
    /// upward, i.e. straight toward the null the split reported.
    ///
    /// Sampled at the pickup because that is when the return leg's target is
    /// decided; an anchor corrected *after* the load is already on board is a
    /// different experiment.
    anchor_on_comb_at_pickup: Option<bool>,
    /// Loops completed while the anchor was on nest material at pickup, and
    /// while it was not. Per ant, so one ant doing four cannot stand in for
    /// four ants doing one.
    loops_anchor_ok: u32,
    loops_anchor_bad: u32,
    /// **How far down the foraging loop this ant ever got**, monotone, one
    /// stage per `FUNNEL` row below. Owner's instruction, 2026-09-20: *"count
    /// the number of ants that make it to the food, how many of those then go
    /// pick up the food and return it to the nest, how many of those get all
    /// the way back and drop it, how many of those start the next loop -- all
    /// of these should be both counts and percentages. Something that doubles
    /// from two ants to four sometimes looks really good, but actually still
    /// 90% of the ants aren't doing anything."**
    ///
    /// Monotone so an ant is counted at its high-water mark and cannot be
    /// double-counted by a later relapse -- which is what every rate in this
    /// harness that divides events by events gets wrong.
    stage: u8,
    /// Distance from the anchor at the moment this ant last picked food up,
    /// so stage 3 can ask whether it actually TURNED AROUND rather than
    /// wandering off with a full crop.
    pickup_dist: i32,
    /// Crop cells last seen, to tell a delivery from a digestion -- since
    /// 2026-09-20 the crop also empties by being eaten, so a falling cell
    /// count is no longer a drop by itself.
    last_cells: u8,
    picked_up_anchor_ok: u32,
    picked_up_anchor_bad: u32,
}

/// **An order statistic over a sample, 0 when there is nothing to order.**
///
/// A median rather than a mean because this line's distributions have long
/// tails: §7.38's own bracket came from three points, and a mean over a
/// handful of very long trips reports a journey nobody made.
fn order_stat(v: &mut [u64], q: f64) -> u64 {
    if v.is_empty() {
        return 0;
    }
    v.sort_unstable();
    let i = ((v.len() - 1) as f64 * q).round() as usize;
    v[i]
}

/// **Zero every weight into `EmitB`, direct and through the hidden layer** --
/// the arm that makes a real no-trail control possible.
///
/// Owner's objection, 2026-09-16, and it was right: this harness's "off" arm
/// was never a no-trail control. It withholds the *hand-laid* ramp and changes
/// nothing about the ants, who still carry the shipped
/// `(Carrying, EmitB, 2.5)` wire and lay channel B on every laden move -- and
/// who, with `gate=b2`, can now read it. So "off" is a **self-organised trail**
/// arm, and "on vs off" compares a hand-laid trail against whatever the colony
/// bootstraps for itself. That is a real question but it is not the one the
/// column headings implied, and it cannot say whether trails work at all: a
/// null at gap 90 reads equally well as "the trail does nothing" and as "the
/// ants' own trail already did it".
///
/// **`EmitA` is deliberately left alone.** It is the nest odometer -- the
/// homing channel -- and zeroing it would fold a homing ablation into a
/// foraging measurement. The question here is channel B.
fn mute_channel_b(g: &mut [f32]) -> usize {
    mute_channel(g, O::EmitB)
}

/// `mute_channel_b`'s body, with the channel as an argument.
///
/// **Channel A gets muted for exactly one purpose and it is not an ablation**:
/// the `flatN`/`flatF` metric controls below need the plane to hold *only* what
/// this harness painted. An ant laying its own channel A on top of a synthetic
/// blob would make the control a measurement of the ants again, which is the
/// one thing a control may not be. Everywhere else `EmitA` is left alone for
/// the reason `mute_channel_b` gives.
/// Author a `Bias` weight that makes an output land on a chosen *behavioural*
/// value, rather than asking the caller to pre-compose `squash` and
/// `unit_scale` in their head.
///
/// `Bias` is 1.0 (`creature.rs:4887`), so with no other wire onto this output
/// the creature uses `unit_scale(squash(w), scale)` = `((w / (1 + |w|)) + 1) / 2
/// * scale`. Inverting: `s = 2 * value / scale - 1`, `w = s / (1 - |s|)`.
///
/// **The inverse is checked forward before it is trusted**, because a solve
/// that is quietly wrong produces an arm that ran at a value nobody chose and
/// says nothing -- the tidiest possible way to lose a night. The assertion is
/// the positive control `CLAUDE.md` asks for, run on every call rather than
/// once in a test.
fn set_via_bias(g: &mut [f32], out: O, value: f32, scale: f32, name: &str) {
    assert!(
        value > 0.0 && value < scale,
        "{name}={value} is outside the open range (0, {scale}) this output can reach: the endpoints need an infinite weight, and a saturated arm is not the value it is named for"
    );
    let s = 2.0 * value / scale - 1.0;
    let w = s / (1.0 - s.abs());
    let got = brain::unit_scale(brain::squash(w), scale);
    assert!(
        (got - value).abs() < 1e-3,
        "{name}: solved weight {w} gives {got}, not {value} -- the inverse is wrong and this arm would run at a value nobody chose"
    );
    let slot = brain::io_slot(brain::BrainInput::Bias, out);
    assert!(
        (g[slot] - w).abs() > f32::EPSILON,
        "{name}={value} is already what the Bias->{out:?} slot produces, so this arm is the shipped one wearing a different name"
    );
    // A weight under `W_EPS` is no connection at all -- `eval_brain` skips it
    // -- so a value that solves to a near-zero weight is silently the default.
    // That is exactly `value == scale / 2`, which is what both these outputs
    // already are, so it is the likeliest thing a caller types by accident.
    assert!(
        w.abs() >= brain::W_EPS,
        "{name}={value} solves to weight {w}, inside W_EPS ({}) -- eval_brain would skip the wire and the creature would run at the silent default",
        brain::W_EPS
    );
    g[slot] = w;
}

fn mute_channel(g: &mut [f32], out: O) -> usize {
    let mut moved = 0;
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
    moved
}

#[allow(clippy::too_many_arguments)]
fn run(seed: u64, trail: bool, gate: Gate, frames: u64, ants: i32, relay: u64, near: i32, food: i32, stop: u64, gap: i32, refill: u64, diet: Diet, mute: bool, paint: PaintA) -> Arm {
    // **The box grows with the gap.** `far_larder` pins food 363 cells from
    // its colony and every one of its 52 ants starves by frame 20,000 --
    // measured, `latecensus scenario=far_larder`: ants 52 -> 0, eats 87 in
    // 30,000 frames. The shipped 90-cell run here survives. So the distance at
    // which a food trail is *both* necessary and survivable is somewhere
    // between, and nobody has swept it. A fixed-width box cannot ask.
    // **The nest needs room on its LEFT for the colony to stand in, and this
    // line is a bug fix rather than tidying.** `World::colony_stations` walks
    // outward from the cursor taking the first column that is a site, so
    // whatever will not fit on the left is placed on the right. With the nest
    // pinned at x = 40, only ten of 52 founders fit to the left at the
    // `COLONY_ANT_SPACING` of 4, and the other 42 march out to **x = 208** --
    // past the food at every gap below 220.
    //
    // Measured, gap 150: `founded x 8..208, food at 190`. The colony was
    // founded ON TOP of the larder, so there was no journey, `arrive@` read
    // frame **1**, and all three arms scored alike because arrival was
    // placement and not navigation. Every gap-90 and gap-150 row taken before
    // this is void; 220 and 300 were clear and stand.
    //
    // The band is `ants * spacing` wide, so half of it plus a margin is what
    // the left needs.
    let half_band = ants.max(1) * 4 / 2 + 8;
    let width = (half_band + gap + 60).max(256);
    let spec = LabBox { width, height: 192, ground_y: 96, soil_depth: 48, founders: 0, colonies: 0, seed, ..LabBox::default() };
    let mut w = spec.build();
    // **Channel A's persistence, before any ant walks.** See the rider docs.
    if let Some(r) = arg::<f32>("arho") {
        w.pheromones.set_channel_rho(Channel::A, r);
    }
    if let Some(d) = arg::<f32>("adiffuse") {
        w.pheromones.set_channel_diffuse(Channel::A, d);
    }
    // **`brho` exists to keep the shipping question honest.** If the homing
    // plane wants a longer life than the food trail, the engine has to say
    // which plane is which -- and the whole point of the 2026-09-02 genome
    // refactor was that A is the homing plane only because a species wires it
    // that way. So the alternative worth measuring is that *neither* trail
    // plane decays and diffusion alone sets both lifetimes, which needs no
    // per-channel rule at all. `set_channel_rho`'s own doc wants decay as §Z7's
    // lever against a trail that outlives its patch, which is the argument on
    // the other side; this rider is what lets the two be compared rather than
    // argued.
    if let Some(r) = arg::<f32>("brho") {
        w.pheromones.set_channel_rho(Channel::B, r);
    }
    let species_id = w.species.id_of("ant").expect("the ant species is compiled in");
    // The ant's own sensor reach, so the readability metric asks what THIS
    // animal reads rather than what a chosen constant would.
    let sensor_span: i32 =
        w.species.get(species_id).creature.as_ref().map_or(6, |c| c.sensor_offset);
    let mut genome = w.species.get(species_id).genome.clone();
    let moved = gate.apply(&mut genome);
    assert!(gate.name == "shipped" || moved > 0, "gate {} changed no slot, so both arms carry one genome", gate.name);
    if matches!(paint, PaintA::FlatNest | PaintA::FlatFood) {
        // The control is only a control if the plane holds what we painted and
        // nothing else -- see `lay_flat`.
        let silenced = mute_channel(&mut genome, O::EmitA);
        assert!(silenced > 0, "no EmitA weight was zeroed, so the flat control's plane is part ours and part the ants'");
    }
    if mute {
        let silenced = mute_channel_b(&mut genome);
        assert!(silenced > 0, "no EmitB weight was zeroed, so the muted arm still lays the plane it is meant to be without");
    }
    // **The two weights that shape the homing ramp, as runtime riders.**
    //
    // §7.15: channel A inverts in exactly the colonies that forage. Hidden unit
    // 4 is an odometer fitted for a **3,000-tick** decay, but a 90-cell trip is
    // 141 ticks at a trail-following `P(move)` of 0.64 -- 4.7% of its range --
    // so the charge falls 0.7% over the whole journey and the ant lays channel A
    // at near-constant strength wherever it goes. A plane laid at constant
    // strength records WHERE ANTS SPENT TIME, and a foraging colony spends it at
    // the food, so the ramp ends up taller at the food end. Polarity is positive
    // in 5 of 5 homebound colonies and negative in 6 of 7 foraging ones,
    // r = +0.77 over 12 seeds.
    //
    // **The tension that makes this a sweep and not a patch**: the charge only
    // replenishes at the nest, and a foraging ant is `AtNest` ~1.5% of the time.
    // Decay too slowly and the signal is laid everywhere and inverts, which is
    // today. Decay too fast and the charge is near zero almost always, so
    // channel A collapses to nothing -- an inverted signal traded for no signal.
    // Read `a_polarity` and `a_peak_amt` together or this measures half of it.
    //
    // `emita` is here because `ant.ron` authors **32.0** into `EmitA` where the
    // odometer's own fitting test (`brain.rs`'s ignored `what_an_odometer_emits`)
    // names **900** in its chosen fit -- a real mismatch, and the obvious lever
    // if faster decay costs signal strength.
    if let Some(r) = arg::<f32>("recur") {
        let slot = brain::hh_slot(4);
        assert!(
            (genome[slot] - r).abs() > f32::EPSILON,
            "recur={r} is already what unit 4's decay slot holds, so this arm is the shipped one wearing a different name"
        );
        genome[slot] = r;
    }
    if let Some(e) = arg::<f32>("emita") {
        let slot = brain::ho_slot(4, O::EmitA);
        assert!(
            (genome[slot] - e).abs() > f32::EPSILON,
            "emita={e} is already what unit 4's EmitA slot holds, so this arm is the shipped one wearing a different name"
        );
        genome[slot] = e;
    }
    // `biasa` restores the deleted emission floor, and it is the lever the
    // `recur` sweep pointed at rather than the one it swept.
    //
    // **`recur` is not what sets this odometer's decay**, which is why that
    // sweep found nothing: `brain.rs`'s own fitting comment says the decay is
    // dominated by `squash`, not by `w_rec` -- `eval_brain` computes
    // `h = squash(w_rec * h)` and `squash` takes ~8%/tick off a level near
    // 0.08, swamping a `w_rec` of 0.99995. Simulated against the engine's own
    // readout (`what_an_odometer_emits`, which this reproduces to three
    // decimals), the shipped emission runs **0.819 -> 0.177 across a 141-tick
    // trip -- a 78% fall**, not the 0.7% that `w_rec^141` alone suggests. The
    // per-ant charge was never the flat thing the first reading of it claimed.
    //
    // **What inverts the plane is that the plane integrates traffic.** A
    // foraging colony's `occupancy/1k` reads `[30 33 14 16 7 22 77 627]`: the
    // food end takes ~21x the ant-ticks of the nest end, against a per-visit
    // strength ratio of only 4.6x. Traffic wins by 4.5x and the ramp points at
    // the food. Sweeping `recur` to 0.98 lifts grading to 25:1 against
    // traffic's 21:1, which is exactly why 0.98 came closest to flipping and
    // still did not.
    //
    // A *floor* is the term that can beat an occupancy ratio, because it makes
    // the ratio unbounded rather than 4.6:1 -- a dwelling ant lays **nothing**
    // instead of a little, 627 ticks in 1,000. `ant.ron` deleted exactly this
    // wire, `(Bias, EmitA, -0.35)`, on the argument that the offset "would only
    // clip the bottom of the gradient". The bottom of the gradient is the part
    // a dwelling ant lays; clipping it is the point.
    if let Some(b) = arg::<f32>("biasa") {
        let slot = brain::io_slot(brain::BrainInput::Bias, O::EmitA);
        assert!(
            (genome[slot] - b).abs() > f32::EPSILON,
            "biasa={b} is already what the Bias->EmitA slot holds, so this arm is the shipped one wearing a different name"
        );
        // A weight under `W_EPS` is no connection at all (`eval_brain` skips
        // it), so a rider set below the gate is silently dead -- the failure
        // this harness has already paid for twice with an ignored argument.
        assert!(
            b.abs() >= brain::W_EPS,
            "biasa={b} is inside W_EPS ({}), so eval_brain would skip the wire and this arm would be the shipped one",
            brain::W_EPS
        );
        genome[slot] = b;
    }

    // --- the food-charged channel B odometer, as riders -------------------
    //
    // **The one mechanism in this line that costs nothing structural**, which is
    // why it is measured here before it is authored anywhere. Channel A ramps
    // because `ant.ron` hidden unit 4 is a nest-charged odometer -- three
    // weights, `(AtNest, 4, 0.05)` / recurrence `0.99995` / `(4, EmitA, 32.0)`.
    // Channel B has **no distance term at all**: one wire,
    // `(Carrying, EmitB, 2.5)`, emitting a flat `squash(2.5) = 0.714` on every
    // laden tick. So a food trail's shape comes only from the order its cells
    // were laid in -- laden means homeward, so the food end is the older end,
    // so after decay it is tallest at the NEST and the shipped ascending reader
    // walks an empty ant home. `pheromone-master-2026-09-17.md` §1 states the
    // defect ("the same laying rule, and they need opposite ones") and proposes
    // no repair for it.
    //
    // These riders author the mirror: `FoodAdjacent` charges hidden unit 7 --
    // free in `ant.ron`, units 0-6 being the two reader pairs, the odometer and
    // the dig gate -- which fades into `EmitB`. `FoodAdjacent` is the exact
    // structural twin of `AtNest`: both are contact booleans
    // (`creature.rs:4677` against the `adjacent_nest` scan).
    //
    // **Fitted through `eval_brain`, not simulated beside it** -- §Z5's dead
    // odometer was verified by side-simulation and the `W_EPS` gate was never
    // seen. `brain.rs`'s ignored `what_a_food_odometer_emits` prints the grid
    // and the candidate; the mirror of unit 4's own weights reproduces the
    // shipped `EmitA` curve at **rms 0.0000** (it is the same curve), holds
    // `t141` at 0.164/0.177/0.178/0.178 across touch durations 1/5/30/400, and
    // averages **0.349** over a 141-tick trip against the shipped wire's flat
    // 0.714 -- so it lays about **half** as much channel B, not more.
    //
    // `carryb=0` is the other half and is not optional: leaving
    // `(Carrying, EmitB, 2.5)` in place keeps the flat smear the odometer is
    // meant to replace, and the two would sum.
    // **The riders run AFTER `mute_channel_b`, so on a muted arm `emitb=` would
    // hand back the emitter `mute` had just taken away** -- and every assertion
    // would still pass, because `mute` asserts only that it zeroed something.
    // That is `CLAUDE.md`'s "a control that validates the knob does not validate
    // the scene" with the scene being the control arm itself. Refused outright
    // rather than ordered around, because the silent version of this is a `mute`
    // row that is not a no-trail control and says so nowhere.
    assert!(
        !(mute && (arg::<f32>("charb").is_some() || arg::<f32>("emitb").is_some() || arg::<f32>("recurb").is_some() || arg::<f32>("biasb").is_some())),
        "the channel B odometer riders would re-arm EmitB on a muted arm, which is no longer a no-trail control -- run the riders with arms=hand,self and take `mute` from the shipped baseline"
    );
    if let Some(c) = arg::<f32>("charb") {
        let slot = brain::ih_slot(brain::BrainInput::FoodAdjacent, 7);
        assert!(
            (genome[slot] - c).abs() > f32::EPSILON,
            "charb={c} is already what the FoodAdjacent->unit 7 slot holds, so this arm is the shipped one wearing a different name"
        );
        assert!(
            c.abs() >= brain::W_EPS,
            "charb={c} is inside W_EPS ({}), so eval_brain would skip the wire and unit 7 would never charge",
            brain::W_EPS
        );
        genome[slot] = c;
    }
    if let Some(r) = arg::<f32>("recurb") {
        let slot = brain::hh_slot(7);
        assert!(
            (genome[slot] - r).abs() > f32::EPSILON,
            "recurb={r} is already what unit 7's decay slot holds, so this arm is the shipped one wearing a different name"
        );
        genome[slot] = r;
    }
    if let Some(e) = arg::<f32>("emitb") {
        let slot = brain::ho_slot(7, O::EmitB);
        assert!(
            (genome[slot] - e).abs() > f32::EPSILON,
            "emitb={e} is already what unit 7's EmitB slot holds, so this arm is the shipped one wearing a different name"
        );
        assert!(
            e.abs() >= brain::W_EPS,
            "emitb={e} is inside W_EPS ({}), so eval_brain would skip the wire and unit 7 would emit nothing",
            brain::W_EPS
        );
        genome[slot] = e;
    }
    if let Some(b) = arg::<f32>("biasb") {
        let slot = brain::io_slot(brain::BrainInput::Bias, O::EmitB);
        assert!(
            (genome[slot] - b).abs() > f32::EPSILON,
            "biasb={b} is already what the Bias->EmitB slot holds, so this arm is the shipped one wearing a different name"
        );
        assert!(
            b.abs() >= brain::W_EPS,
            "biasb={b} is inside W_EPS ({}), so eval_brain would skip the wire and this arm would be the shipped one",
            brain::W_EPS
        );
        genome[slot] = b;
    }

    // --- run length: the two outputs nothing has ever authored -------------
    //
    // **`Tumble` is not a new lever, it is a measured constant that lost its
    // value in a refactor.** `dead-ends.md:1083` records the number:
    // re-orienting on *every* failed move roll took food discovery from 33
    // pickups to **1**, and `TUMBLE_ON_FAILED_MOVE = 0.35` was the fix. That
    // const became `BrainOutput::Tumble`, whose silent output is
    // `unit_scale(0.0, 1.0)` = **0.5** -- and `ant.ron` authors no `Tumble`
    // wire, so the shipped ant re-rolls its heading on half of its failed move
    // rolls against an authored answer of 0.35. `creature.rs`'s own note says
    // the point was to let a creature be selected for its answer; nothing ever
    // selected one, so the ant inherited the default rather than the finding.
    //
    // **And the failed move roll is most of this ant's life.** Measured over
    // five cohort traces, the engine's `P(move)` is exactly zero on **48-72%**
    // of ticks, so a 0.5 tumble is a heading re-roll about every third tick
    // *while standing still*. That is what the run-length census reads back:
    // mean run **3.1-3.4 ticks and 0.35 cells** -- the heading turns over six
    // times faster than the body moves. A run-and-tumble ratchet that works by
    // stalling an ant pointed the wrong way cannot accumulate anything if the
    // stalled ant immediately re-rolls the heading it was stalled for.
    //
    // `persist` is the same shape one level down: the straight-ahead score in
    // `step_chain`'s three-candidate choice, an anonymous `0.15` until it
    // became `BrainOutput::Persist` with a silent **1.0** (`PERSIST_MAX` 2.0,
    // half scale). Its own doc calls it "the number that decides whether a
    // creature commutes or mills" and says handing it to measurement was the
    // entire point.
    //
    // Both riders take the value the creature will *use*, not a weight, and
    // solve for the `Bias` weight that produces it -- `squash` and `unit_scale`
    // between the author and the behaviour is exactly the gap this section's
    // headline bug lived in, and a rider quoting weights would reopen it.
    // **The temporal reading, as two one-weight riders -- `arise=` into
    // `Move` and `arisetumble=` into `Tumble`.**
    //
    // `BrainInput::PheroARise` is `(live - lagged) / (live + lagged + guard)`
    // on the animal's own cell, computed in `sense`, so there is nothing to
    // wire but the output weight. That replaces the 2026-09-19 `tcomp=` rider,
    // which spent hidden unit 7 as the fading memory and asked the brain to
    // subtract two raw levels -- measured null then, and still null on the
    // tree that ships `(HomeAligned, Move, 3.0)`: a=32 takes closed laps
    // 91 -> 81, a=8 is a coin flip. **Unit 7 is free again.**
    //
    // **The two outputs are different mechanisms and the signs differ.**
    // `Move` is how long a run lasts, so "the smell is improving" means step
    // MORE and the weight is positive. `Tumble` is whether a failed step
    // throws the current heading away for a random one, so "improving" means
    // tumble LESS and the weight is negative -- which is the actual bacterial
    // rule (Segall/Block/Berg 1986), and the one §7.48 never tried. **No ant
    // genome in this repo authors a single weight into `Tumble`**, so that arm
    // starts from the unauthored midpoint `unit_scale(0, 1.0) = 0.5`.
    for (name, out) in [("arise", O::Move), ("arisetumble", O::Tumble)] {
        if let Some(w) = arg::<f32>(name) {
            assert!(
                w.abs() >= brain::W_EPS,
                "{name}={w} is inside W_EPS ({}), so eval_brain would skip it and this arm would be the shipped one",
                brain::W_EPS
            );
            genome[brain::io_slot(brain::BrainInput::PheroARise, out)] = w;
        }
    }
    // **`homewire=` -- the return leg's throttle**, `ant.ron`'s
    // `(HomeAligned, Move, 3.0)`. The one wire in the shipped genome that
    // reads a quantity which knows where home is; `w` is its authority over
    // `P(move)`, and `squash(w)` is the rate a laden ant pointed straight at
    // the nest runs at once the ant's other `Move` terms are near zero.
    // `homewire=0` is the control arm -- the engine as it was before 2026-09-20
    // with the input present and unread, which is the right baseline because it
    // holds `mutation_rate` and every genome dimension fixed across the pair.
    if let Some(w) = arg::<f32>("homewire") {
        let slot = brain::io_slot(brain::BrainInput::HomeAligned, O::Move);
        assert!(
            (genome[slot] - w).abs() > f32::EPSILON,
            "homewire={w} is already what ant.ron holds, so this arm is the shipped one wearing a different name"
        );
        // **No `W_EPS` assertion here, unlike every rider above**, and
        // deliberately: 0 is this knob's control arm and `eval_brain` skipping
        // the slot is exactly what the control wants. Every non-zero value a
        // sweep would use clears `W_EPS` (1e-3) by three orders of magnitude.
        genome[slot] = w;
    }
    if let Some(v) = arg::<f32>("tumble") {
        set_via_bias(&mut genome, O::Tumble, v, 1.0, "tumble");
    }
    if let Some(v) = arg::<f32>("persist") {
        set_via_bias(&mut genome, O::Persist, v, pixel_physics::sim::creature::PERSIST_MAX, "persist");
    }
    // **The gradient into `Tumble`, which is what `BrainOutput::Tumble`'s own
    // doc asks for**: "tumble more when crowded, *less when on a good
    // gradient*". Negative weight, so a positive `along` -- the ant facing up
    // the homing ramp -- suppresses the re-roll and the run survives to be
    // walked. This is the cheap spatial form; the temporal comparator is the
    // version that works where the plane is flat, and it needs a hidden unit.
    if let Some(a) = arg::<f32>("tumblegrad") {
        let slot = brain::io_slot(brain::BrainInput::PheroAAlong, O::Tumble);
        assert!(
            (genome[slot] - a).abs() > f32::EPSILON,
            "tumblegrad={a} is already what the PheroAAlong->Tumble slot holds, so this arm is the shipped one wearing a different name"
        );
        assert!(
            a.abs() >= brain::W_EPS,
            "tumblegrad={a} is inside W_EPS ({}), so eval_brain would skip the wire and this arm would be the shipped one",
            brain::W_EPS
        );
        genome[slot] = a;
    }

    // **Takes a value so it cannot be a silent no-op**, and asserts the wire it
    // is removing was actually there: `carryb=0` on a genome that has already
    // lost that wire is an arm wearing a name for something it did not do.
    //
    // **`CarryingFood`, not `Carrying` -- corrected 2026-09-19, and the
    // assertion above is what found it.** `ant.ron`'s emitter was
    // `(Carrying, EmitB, 2.5)` when this rider was written and became
    // `(CarryingFood, EmitB, 2.5)` in `1f7b6f95` the next day ("the homing
    // gate reads food, not dirt"). Those are two different `BrainInput`
    // variants -- 12 and 30 -- so the rider went on zeroing a slot that was
    // already zero, which is exactly the "arm wearing a name for something it
    // did not do" this assertion exists to refuse. `dead-ends.md`'s
    // food-odometer entry says re-running it costs one command; it did not,
    // and the only reason that is a five-minute correction rather than a
    // silently wrong table is that the check was written to take a value and
    // fail loudly on a no-op.
    if let Some(c) = arg::<f32>("carryb") {
        let slot = brain::io_slot(brain::BrainInput::CarryingFood, O::EmitB);
        assert!(
            (genome[slot] - c).abs() > f32::EPSILON,
            "carryb={c} is already what the Carrying->EmitB slot holds, so this arm is the shipped one wearing a different name"
        );
        genome[slot] = c;
    }

    let surface = spec.ground_y - 2;
    let (nest_x, target_x) = (half_band, half_band + gap);
    // The species' own sensor reach, not a literal -- see the along readout.
    let sensor_offset = w.species.get(species_id).creature.as_ref().expect("ant is a creature").sensor_offset;
    let placed = w.found_colony_of(nest_x, surface, "ant", ants);
    assert!(placed > 0, "no ants placed at the nest end; there is nothing to measure");
    // **A SHORT COLONY IS NOT A SMALLER EXPERIMENT, IT IS A DIFFERENT ONE**, and
    // it used to pass silently: only `placed > 0` was checked, so an arm that
    // founded half the animals was compared, seed for seed, against one that
    // founded all of them. Every per-colony total is then a measurement of the
    // founding, which is the denominator failure the `funnel` skill exists to
    // prevent.
    //
    // Measured 2026-09-21, the run that produced this line:
    // `PIXEL_PHYSICS_COLONY_SPACING=1` at `ants=20` places **10**. The corridor
    // admits the column and the placement then refuses it, because the ant is
    // two cells nose-to-tail and its second cell lands on the neighbour -- so
    // every other station fails and the count nobody was reading went to half.
    // Spacing 2 is the floor that founds the full colony (see
    // `creature::colony_spacing_override` for the table).
    assert_eq!(
        placed, ants as usize,
        "the bed asked for {ants} ants and founded {placed}. A short colony is a different experiment, not a          smaller one -- every total below would be a measurement of the founding. If this is PIXEL_PHYSICS_COLONY_SPACING,          2 is the narrowest that still founds a full colony of two-cell ants; if it is terrain, widen the box."
    );
    // **The guard that would have caught the scene bug above**, checked
    // against the founders' real positions rather than against the arithmetic
    // that was supposed to produce them. A colony wider than the gap is not a
    // foraging experiment, and it fails in the direction that looks like a
    // result: every arm reaches the food, so the trail appears not to matter.
    let (fl, fh) = w
        .live_organism_ids()
        .into_iter()
        .filter_map(|id| w.organism(id).filter(|s| s.species == species_id).and_then(|s| s.chain.first().copied()))
        .fold((i32::MAX, i32::MIN), |(lo, hi), (x, _)| (lo.min(x), hi.max(x)));
    assert!(
        fh < target_x - near,
        "the colony is founded across x {fl}..{fh} and the food is at {target_x} (+-{near}): the ants start ON the larder, \
         so this scene has no journey in it. A colony of {ants} at spacing 4 is about {} cells wide, so the gap must exceed \
         roughly half of that. Use a larger gap= or fewer ants=.",
        ants * 4
    );

    // **Where the food ramp's zero sits**, decided here because this is the
    // first point the founders' *real* span is known -- `fl` above is measured
    // off the placed animals, not off the arithmetic that was meant to produce
    // them, which is the distinction the guard above exists for.
    //
    // `layfrom=founders` walks the ramp's foot west to the furthest-west ant
    // actually placed, so every founder stands on a gradient that climbs
    // toward the larder. `layfrom=nest` (the default) leaves it on the nest
    // cursor and reproduces every row measured before 2026-09-21. See `lay`
    // for the 40%-against-98% split that makes this worth a switch.
    // Read here rather than threaded through `run`'s parameter list, as
    // `aprofile` and its siblings already are: it is a scene constant that no
    // sweep varies per arm, and the list is long enough.
    let layfrom: String = arg_str("layfrom").unwrap_or_else(|| "nest".to_string());
    let lay_foot_x = match layfrom.as_str() {
        "founders" => fl.min(nest_x),
        "nest" => nest_x,
        other => panic!("layfrom={other} is not a thing: use layfrom=nest (default) or layfrom=founders"),
    };

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
    let larder = w
        .materials
        .id_of(diet.larder)
        .unwrap_or_else(|| panic!("larder material {:?} is not compiled in", diet.larder));
    diet.isolate(&mut w, larder);
    // **`kinpass=on` lets a blocked ant trade places with a nestmate.** A
    // whole-run switch rather than a fourth arm: it is a question about the
    // engine's traffic rule, so it wants the same three arms run twice, not a
    // fourth arm that confounds it with the trail.
    if flag("kinpass") {
        let mut cdef = w.species.get(species_id).creature.clone().expect("ant is a creature");
        cdef.passes_through_kin = true;
        w.species.set_creature(species_id, cdef);
    }
    // **`homebias=` -- the fill-weighted homeward tumble, the arm §7.26
    // designed.** Refused when it matches the file, for the reason every
    // genome rider here is: a rider that silently re-authors the shipped value
    // is indistinguishable from one that is not wired to the field it names,
    // and this harness has already shipped two knobs that were being ignored.
    if let Some(hb) = arg::<f32>("homebias") {
        let mut cdef = w.species.get(species_id).creature.clone().expect("ant is a creature");
        assert!(
            (cdef.home_bias - hb).abs() > f32::EPSILON,
            "homebias={hb} is already what ant.ron holds, so this arm is the shipped one wearing a different name"
        );
        cdef.home_bias = hb;
        w.species.set_creature(species_id, cdef);
    }
    // **`cropcap=` -- how many cells of the larder an ant can hold at once**,
    // which is the one variable that decides whether a foraging trip can
    // deliver anything at all.
    //
    // `ant.ron` authors `crop_capacity: 1440.0` and says in the same breath why:
    // *"1440 is three leaves at the shipped table (480 each), and three is a
    // floor rather than a taste. Food only leaves the crop a whole cell at a
    // time, so an ant that can hold exactly one leaf is under one leaf within a
    // tick of ingesting and can never deliver again."* That floor holds for a
    // **480 J** food. This bed's default larder is **fruit at 960 J**, so one
    // cell is 0.667 of the crop, two do not fit, and the ant is in exactly the
    // state the comment forbids: it eats its cargo on the walk and arrives
    // empty. Measured on the focal ant -- `crop_cells` was 1 on 1,740 ticks, 0
    // on 16,079, and never 2.
    //
    // The rider exists so that capacity can be varied **against the same bed and
    // the same food**. Switching the larder to a 480 J one instead changes the
    // food's *physics* too (`deadleaf` is a Powder and the pile flows west to
    // meet the ants: 6/6 seeds survived at gap 90 with `visitors 0/20`), which
    // is a different experiment wearing this one's clothes.
    if let Some(cc) = arg::<f32>("cropcap") {
        let mut cdef = w.species.get(species_id).creature.clone().expect("ant is a creature");
        assert!(
            (cdef.crop_capacity - cc).abs() > f32::EPSILON,
            "cropcap={cc} is already what ant.ron holds, so this arm is the shipped one wearing a different name"
        );
        cdef.crop_capacity = cc;
        w.species.set_creature(species_id, cdef);
    }
    // **`hungergate=` -- how far digestion follows appetite**, the control arm
    // for `CreatureDef::digest_hunger_weight`. `hungergate=0` restores the
    // clock-driven gut exactly, which is the baseline every number for this
    // mechanism has to be read against.
    if let Some(hg) = arg::<f32>("hungergate") {
        let mut cdef = w.species.get(species_id).creature.clone().expect("ant is a creature");
        assert!(
            (cdef.digest_hunger_weight - hg).abs() > f32::EPSILON,
            "hungergate={hg} is already what ant.ron holds, so this arm is the shipped one wearing a different name"
        );
        cdef.digest_hunger_weight = hg;
        w.species.set_creature(species_id, cdef);
    }
    // **Placed as a closure because it has to be REPLENISHED, and the arithmetic
    // says why.** 52 ants at two cells, `idle_cost_per_cell` 0.05 and
    // `move_cost_per_cell` 0.125 on a 6-frame tick, need roughly **46,800 J**
    // over 24,000 frames. A one-shot `food=60` of corpse at 120 J is **7,200 J
    // -- 15% of that**, so the colony starves whatever the gap is, and the
    // first run of `mode=gap` duly read `alive 0/0` in all twelve rows.
    //
    // `far_larder.ron` is under-provisioned the same way: 60 cells plus 30
    // every 6,000 frames is 21,600 J over the same window, **46% of need**. So
    // its colony dying is not evidence that 363 cells is too far -- it would
    // die at any distance. That bracket has to be re-derived.
    //
    // Over-provisioning *at the target* is deliberate and not a thumb on the
    // scale: all food sits at one far coordinate, so abundance there does not
    // shorten the journey. It isolates distance from scarcity, which is the
    // whole point of the sweep.
    //
    // **It counts what it actually introduced**, not what it was asked for: a
    // refill overwrites coordinates that may still hold larder from last time,
    // and counting the request would book those twice and make `taken` read
    // high for ever.
    let place_food = |w: &mut pixel_physics::sim::world::World, n: i32, placed: &mut u64| {
        for i in 0..n {
            let (fx, fy) = (target_x + (i % 12) - 6, surface - (i / 12));
            if w.get(fx, fy).material != larder {
                *placed += 1;
            }
            w.set(fx, fy, Cell::new(larder, 0));
        }
    };
    let mut larder_placed = 0u64;
    if food > 0 {
        place_food(&mut w, food, &mut larder_placed);
    }
    // The larder priced the way the animal prices it, not the way the material
    // table reads. See `Arm::supply_j`.
    let gut_bias = w.species.get(species_id).creature.as_ref().expect("ant is a creature").traits[0];
    let per_cell_j = creature::diet_yield(&w, Cell::new(larder, 0), gut_bias) as f64;
    for id in w.live_organism_ids() {
        if w.organism(id).is_some_and(|s| s.species == species_id) {
            assert!(w.set_organism_genome(id, genome.clone()), "the founder must be live when its genome is set");
        }
    }

    let (mut particles, mut blasts, tuning) = (ParticleSystem::default(), Blasts::default(), player::Tuning::default());
    let (mut near_ticks, mut ant_ticks) = (0u64, 0u64);
    let mut alive_min = usize::MAX;
    let (mut laden_ticks, mut laden_nest_ticks) = (0u64, 0u64);
    let (mut carry_ticks, mut carry_home_ticks) = (0u64, 0u64);
    let mut tracks: std::collections::HashMap<u32, Track> = std::collections::HashMap::new();
    let (mut atnest_ticks, mut probe_ticks) = (0u64, 0u64);
    // **Peak, not end-of-run.** Sampling the plane at the finish cannot tell a
    // channel that is never laid from one that is laid and decays -- the exact
    // error `route pk` made for channel B, in reverse. Tracked as a running
    // maximum over the whole run instead.
    let (mut a_peak_amt, mut a_peak_cells) = (0u32, 0usize);
    // **The polarity, averaged over the run rather than read at the end.**
    // A dead colony's plane has decayed to nothing by the finish, so an
    // end-of-run `along` reports 0.0 for most seeds and the sign is invisible.
    // Averaged over every sample that had a trail to measure, it is not.
    let (mut a_pol_sum, mut a_pol_n) = (0.0f64, 0u64);
    // The same statistic with the admission gate tightened to `&&`, plus how
    // many cells the `||` gate admitted that `&&` would not. See `lay_flat`:
    // an edge cell scores about -0.97 against an interior cell's +-0.03..0.07,
    // so the two figures diverging IS the artifact, and the count is what says
    // how much of the `||` figure is edge.
    let (mut a_pol_both_sum, mut a_pol_both_n) = (0.0f64, 0u64);
    let (mut a_edge_sum, mut a_edge_n) = (0u64, 0u64);
    // **The one of the three that survives its own controls.** See
    // `a_polarity_span` on `Arm` for what the other two do to a blob.
    let (mut a_pol_span_sum, mut a_pol_span_n) = (0.0f64, 0u64);
    // **Where the colony's own channel B mass STANDS, as a time average, and it
    // has to be a time average.** The first cut of this sampled the plane at the
    // end of the run and printed `[0,0,0,0,0]` in every arm of every seed --
    // the trail had decayed by then, which is precisely the failure
    // `a_peak_cells` carries a running maximum to avoid: an end-of-run sample
    // cannot tell "never laid" from "laid and gone".
    //
    // A mean rather than a max, because the question is where the mass SITS
    // rather than how high it ever got; one transient spike sets a max.
    // Read it against `occupancy/1k`: if B peaks wherever the ants ARE rather
    // than wherever the food is, the plane is integrating traffic, and the
    // arithmetic that decides it is 21:1 traffic against 4.6:1 grading.
    let mut b_prof_sum = [0.0f64; 5];
    let mut b_prof_n = 0u64;

    // --- the decision trace ------------------------------------------------
    //
    // **What a LADEN ant in a real colony reads on the homing plane, and what
    // it does about it.** Nobody has printed this: the `along` column above is
    // `Channel::B`, and `onetrail`'s +104-of-112 figure is for a hand-stamped
    // ramp on a bare slab with no colony around it. The gap between that and
    // this bed's `carry->nest` is about 750x and nothing localises it.
    //
    // Off by default and it must stay that way: `probe_full` costs a whole
    // `sense` per ant per frame, against the 100-frame cadence the columns
    // above are sampled on.
    let tracing = flag("trace");
    // **Trace an ant that never finds food.** Without this the focal ant is
    // drawn from larder-carriers, so the seeds where foraging fails outright
    // produce an empty CSV -- see the selection site.
    let focal_any = flag("focalany");
    // **Which ant to follow, by where it starts.** `focalany` takes the first
    // one seen, which is the westernmost founder -- and the west of this colony
    // is off the hand-laid trail entirely (it runs `nest_x..=target_x`, while
    // founders span the band around the nest). Tracing only that ant answers
    // "what does an ant with no trail under it do" and cannot answer "does an
    // ant standing ON the trail follow it", which is the question the arm is
    // for. `focalx=N` picks the ant nearest x = N at selection time.
    let focal_x: Option<i32> = arg("focalx");
    // **`focaln=N` follows a COHORT instead of one ant, and one ant is the
    // thing this instrument could never answer with.** A single trace is n=1 in
    // a chaotic system: it says what *an* ant did, and the question "why does
    // nobody reach food at gap 200" is about the population, not about a
    // protagonist. The rows carry an `id` column so they split per animal.
    //
    // The cohort is taken at first sighting, spread across the founding band
    // rather than all from one end -- `focalany` takes the westernmost founder,
    // which is off the hand-laid trail entirely, so a cohort drawn the same way
    // would be five ants all answering the same unrepresentative question.
    let focal_n: usize = arg("focaln").unwrap_or(0);
    // **A real tick discriminator, because the row cannot infer one.** Focal
    // rows are written every FRAME while the ant decides every `tick_interval`
    // (6), so consecutive rows are mostly re-reads of one decision and the only
    // tell is that `x`/`heading` happened not to change -- which is also what a
    // *stalled* ant looks like, i.e. exactly the case this trace exists to
    // study. `OrganismState::since_nest` is incremented once per creature tick
    // (`creature.rs:4647`) and reset only at the nest, so a CHANGE in it marks
    // a tick unambiguously, reset included. (`age_ticks` looks like the right
    // field and is not: grep says only `plant.rs` ever increments it, so it
    // reads 0 for every ant for ever.)
    let mut tick_of: std::collections::HashMap<pixel_physics::sim::cell::OrganismId, (u16, u64)> =
        std::collections::HashMap::new();
    // **The channel A amplitude profile along the route, every `aprofevery`
    // frames.** See the dump site for why a gradient reading cannot answer it.
    let a_profile = flag("aprofile");
    // **The channel-B trail as a time series** -- see the emit site. Off by
    // default: it is one line per sample and would drown a sweep's table.
    let btrail = flag("btrail");
    // **Asserted rather than documented, because the failure is silent.** The
    // emit site sits inside the existing every-100-frames sample block, so a
    // value that is not a multiple of 100 quietly samples at the lowest common
    // multiple instead -- `btrailevery=250` would give every 500 -- and prints
    // a complete, plausible series at half the rate asked for. `CLAUDE.md`: a
    // knob nobody can see the value of is a knob nobody can tell is
    // disconnected.
    let btrail_every: u64 = arg("btrailevery").unwrap_or(500);
    assert!(
        btrail_every > 0 && btrail_every.is_multiple_of(100),
        "btrailevery={btrail_every} must be a positive multiple of 100: the sampler it rides on runs \
         every 100 frames, so anything else silently samples at the lowest common multiple instead"
    );
    let a_prof_every: u64 = arg("aprofevery").unwrap_or(2000);
    let a_prof_step: usize = arg("aprofstep").unwrap_or(10);
    // **`arho=` / `adiffuse=` -- channel A's persistence, per plane.**
    // `Pheromones::set_channel_rho` / `set_channel_diffuse` reach either trail
    // plane individually and had no caller outside the pheromone harnesses.
    // They are here because the register's re-test conditions on both
    // constants are met: `dead-ends.md:1202` holds them "for a u8 plane with a
    // 3x3 mean kernel; a wider-precision plane would need re-sweeping", and
    // the `u8` -> `u16` widening landed 2026-09-15 without either being
    // re-swept. **B is deliberately untouched** -- a food trail and a homing
    // ramp want different lifetimes, and moving both at once measures neither.

    let mut tr_n = 0u64;
    let mut tr_along_sum = 0.0f64;
    // **The magnitude, separately, because the signed mean cannot answer "is
    // there a gradient".** `PheroAAlong` is `(ahead - here)` along the ANT'S
    // HEADING, not along the world's x -- so on a perfectly good nest-ward ramp
    // a population with uniformly distributed headings averages to zero by
    // symmetry. A signed mean of 0.000 is therefore produced both by "no ramp
    // exists" and by "a fine ramp, read by ants facing every way", and those
    // want opposite work. `|along|` separates them.
    let mut tr_abs_along_sum = 0.0f64;
    let mut tr_along_hist = [0u64; 9]; // -1..1 in ninths, 4 = the zero bucket
    let mut tr_pmove_sum = 0.0f64;
    let mut tr_trail_sum = 0.0f64; // the h0/h1 -> Move contribution alone
    // **Keyed by NAME, not by position, and that is not a nicety.** The term
    // set is a property of the individual's genome: `eval_brain` skips any
    // weight under `W_EPS`, so a mutated offspring with one more live wire
    // decomposes into one more term than its parent. Indexing a fixed `Vec` by
    // position panicked on the first bred colony -- the first ant gave 11
    // terms and a later one gave 12.
    let mut tr_terms: std::collections::BTreeMap<String, (f64, u64)> = std::collections::BTreeMap::new();
    let mut tr_dx_home = 0i64;
    // **Split by the sign of `along`, which is what makes this a test of the
    // MECHANISM rather than of the plane.** The shipped homing circuit is
    // run-and-tumble: reading up-gradient raises `P(move)` so the ant runs,
    // reading down-gradient drops it so the ant stalls and tumbles onto a new
    // heading. Two things have to be true for that to carry food home, and
    // they fail differently:
    //   1. `P(move | along > 0)` must exceed `P(move | along < 0)` -- the
    //      reader is connected and the gate is open. If not, the circuit is
    //      inert whatever the plane looks like.
    //   2. steps taken while `along > 0` must actually go HOME -- the ramp
    //      points the right way. If (1) holds and (2) does not, the ant is
    //      faithfully following a gradient to the wrong place.
    // n, sum P(move), sum cells homeward, **ticks the engine's P(move) is not
    // zero**. The fourth field is the half the first three could not see: the
    // roll `creature.rs` makes is `clamp(out, 0, 1)`, so most of this ant's
    // ticks sit at a hard zero and a mean over them averages in cells where the
    // homing term is disconnected. See the `p_move` note at its assignment.
    // **The two censuses the sensor-geometry diagnosis rests on**, so it is a
    // readout rather than a post-hoc script over a CSV.
    //
    // `tr_by_kind` splits every laden decision by WHAT THE NOSE IS POINTING AT
    // -- `creature::sense` samples `(x + dx*so, y + dy*so)`, and with +y down,
    // six of the eight `DIRS` entries put that six rows off the ant's own row:
    // three in open air, three inside the ground. A walking creature only ever
    // lays a trail at its body cell, so those six read exactly 0, and
    // `(0 - here)/(0 + here + SCALE)` is a confident STRONG NEGATIVE where the
    // honest answer is "I am looking at the sky and know nothing".
    // Index: 0 air, 1 solid, 2 surface. Fields: n, sum along, n usable, n frozen.
    let mut tr_by_kind = [(0u64, 0.0f64, 0u64, 0u64); 3];
    // Freeze runs: how long a laden ant sits at `P(move)` exactly zero. The
    // mean cannot show this -- a 9-tick median with a 157-tick tail is a
    // different animal from one that pauses evenly, and only the run length
    // says which.
    let mut tr_freeze_runs: Vec<u32> = Vec::new();
    let mut tr_freeze_open: std::collections::HashMap<pixel_physics::sim::cell::OrganismId, u32> =
        std::collections::HashMap::new();
    let mut tr_up = (0u64, 0.0f64, 0i64, 0u64);
    let mut tr_down = (0u64, 0.0f64, 0i64, 0u64);
    let mut tr_flat = (0u64, 0.0f64, 0i64, 0u64);
    // **The same split again, restricted to decisions where the homing gate is
    // actually OPEN** -- and that is the one that can answer whether the ramp
    // points the right way. With the gate shut the pair is saturated and cannot
    // respond to `PheroAAlong` at all, so any correlation between the gradient
    // and where the ant went is something else moving it, and reading a
    // direction off the pooled rows would be reading a confound.
    let mut tr_up_open = (0u64, 0.0f64, 0i64, 0u64);
    let mut tr_down_open = (0u64, 0.0f64, 0i64, 0u64);
    // **Is the homing gate even OPEN while the ant carries food?**
    //
    // Units 0/1 are a gated pair: `Bias + Carrying*w`, and the pair only leaves
    // saturation when that sum approaches zero, i.e. at
    // `Carrying >= -Bias / w`. **`Carrying` is not a boolean** -- it is
    // `crop.worth() / crop_capacity` (`creature.rs`), so an ant holding one
    // cell of a 960 J food against `ant.ron`'s `crop_capacity: 1440.0` reads
    // **0.667**, not 1.0.
    //
    // The threshold is computed from the genome rather than restated, the same
    // discipline `onetrail::hold_gate_laden` uses, so it stays right if
    // `ant.ron` retunes the gate.
    let mut tr_carry_hist = [FillBin::default(); 10];
    // **Does pointing at home actually produce a positive reading?** The one
    // question that separates three candidate fixes, and nothing measured it.
    //
    // The compass (`home_weighted_pick`) aims the body from the exact home
    // vector; `P(move)` is set by `PheroAAlong`, the trail gradient under the
    // nose. They are different signals and they may disagree. Binned by
    // alignment between the ant's heading and its home vector, -1..+1:
    //
    //   high alignment -> `along` POSITIVE      the trail points home; the
    //                                           throttle simply will not act
    //                                           on it (authority is the fix)
    //   high alignment -> positive but TINY     the constant guard is crushing
    //                                           it (fold-change is the fix)
    //   high alignment -> `along` NEGATIVE      the trail does not point home
    //                                           where ants walk; both of the
    //                                           above are treating symptoms
    //
    // `(n, sum along, n with along > 1e-3, sum P(move))`.
    let mut tr_align = [(0u64, 0.0f64, 0u64, 0.0f64); 5];
    // **Why a laden ant is not putting its load down -- read off its own brain,
    // not inferred from an outcome.** Owner's rule, 2026-09-20: the test is to
    // check the brains at every tick that mattered and every decision and why.
    //
    // Bucketed by the ONE input that decides `Drop`: how far the nearest nest
    // material is from the head. `(AtNest, Drop, 1.0889)` is the only positive
    // term in the row against `(Bias, Drop, -0.2)`, so `P(drop)` is EXACTLY
    // zero at any distance above adjacency, at any crop fill -- including a
    // full one. Buckets: 0 = adjacent (`AtNest` true), then 2, 4, 8, 16, 32,
    // further. Each carries laden ticks, summed `drop_urge`, and the drops
    // that actually fired, so "never got there" and "got there and did not
    // drop" cannot be confused for one another.
    let mut tr_drop: [(u64, f64, u64); 7] = [(0, 0.0, 0); 7];
    // **When a laden ant misses the comb, does it miss SIDEWAYS or UPWARD?**
    // The two want opposite fixes and the distance alone cannot tell them
    // apart. The comb is a single row of cells at the terrain SURFACE, and
    // `adjacent_nest` reads the 8-neighbourhood of the HEAD -- so a `Chain(2)`
    // ant standing on its own doorstep with its head two rows up reads
    // `AtNest` false while being, in every sense a player would use, at home.
    // Indexed [dx.abs().min(4)][dy.abs().min(4)] over near misses only
    // (nearest material within 4 cells), because a miss by 32 is a navigation
    // question and not this one.
    let mut tr_miss = [[0u64; 5]; 5];
    // **Can a laden ant SMELL its way the last few cells to the comb?** The
    // question option C turns on: two-phase homing -- run the path-integration
    // vector far out, then close the last cells on a sensory cue -- needs the
    // cue to exist AND to be readable, and neither is obvious here. §Z29
    // established that a laden ant cannot read channel A *on the route*,
    // because `here` is its own freshest deposit; whether that also holds
    // **beside the nest**, where the comb's own odometer emission is strongest
    // and the ant's own mark is one tick old, is a different question and
    // nobody has asked it.
    //
    // Two halves, because they fail independently:
    //   `.0/.1/.2/.3/.4` IS THE SIGNAL THERE -- channel A one step toward the
    //   comb against one step away, read off the plane, nothing to do with the
    //   ant's sensor.
    //   `tr_smell_along` CAN THE ANT READ IT -- the ant's own `PheroAAlong`,
    //   split by whether its heading points at the comb. A signal that exists
    //   and is invisible to its reader is what §Z29 already found once.
    let mut tr_smell: (u64, f64, f64, u64, u64) = (0, 0.0, 0.0, 0, 0);
    // **Can a TWO-FORWARD-SAMPLE comparator read the homing plane?** §Z29's
    // third repair candidate, never built: *"compare two forward samples (`so`
    // and `2*so`) so neither term carries the animal's own mark"*. It removes
    // `here` -- the ant's own freshest deposit -- from `(ahead - here)`, which
    // is the whole of the defect.
    //
    // **Measured BEFORE building it, because it may not be able to work.**
    // §7.47 found the single sensor at `so = 6` lands in open sky or solid rock
    // on ~70% of ticks; a comparator needs TWO samples to land, and the literal
    // `so`/`2*so` pair reaches 12 cells out on a two-cell animal. So the
    // precondition is measured across candidate offsets first: a pair that is
    // blind most of the time is not a repair however good its arithmetic.
    //
    // Per pair: [both-zero ticks, n pointed home, sum along home, n pointed
    // away, sum along away]. Both-zero is "no information", which an average
    // over `along` hides by reading 0.0 -- the exhausted-representation
    // signature `CLAUDE.md` warns reads as a working-but-weak mechanism.
    const CMP_PAIRS: [(i32, i32); 5] = [(1, 2), (1, 3), (2, 4), (3, 6), (6, 12)];
    let mut tr_cmp: [(u64, u64, f64, u64, f64); 5] = [(0, 0, 0.0, 0, 0.0); 5];
    // **The temporal pre-check's four bins**, indexed
    // `homeward + 2*moved`: [away&frozen, home&frozen, away&moved, home&moved].
    // Per bin: [ticks, sum of d(PheroAHere), ticks where it rose, sum of the
    // level]. The level is carried because a difference is only a gradient
    // reading if it is not just tracking how bright the cell is -- the same
    // level-term trap `brain.rs`'s fit found in the wiring (§7.48).
    /// **The temporal pre-check, bucketed by DISTANCE FROM HOME and normalised
    /// -- and the first cut of it was neither, which made it unreadable.**
    ///
    /// Measured 2026-09-20: split only by heading, a raw `d(PheroAHere)` in
    /// scent units said an ant walking AWAY from home sees a bigger rise than
    /// one walking toward it (`-483` separation over 58,522 ticks). It is a
    /// confound, not a finding: the level column gave it away at **2,724
    /// against 1,292**. An ant pointed home is typically FAR out in dim
    /// country; an ant pointed away has typically just left the nest and is
    /// standing in the brightest part of the ramp. The split was measuring
    /// where the two groups stand, not what they can smell.
    ///
    /// Two repairs, both needed. **Bucket by distance**, so home and away are
    /// compared where the plane is equally bright. And **normalise**, as
    /// `PheroAAlong` and `tr_cmp` already do -- `(live - lagged) / (live +
    /// lagged + guard)` is scale-free, so a band that is dim overall does not
    /// read as a weaker mechanism.
    ///
    /// `[band][moved][homeward]` -> (ticks, sum of normalised exp-lag
    /// difference, ticks where it rose).
    const DIST_BANDS: [i32; 4] = [8, 20, 45, i32::MAX];
    let mut tr_temporal = [[[(0u64, 0f64, 0u64, 0f64); 2]; 2]; 4];
    /// **The engine's own `PHERO_A_MEM_RECURRENCE`** (`creature.rs`, beside
    /// `since_nest`), so this oracle lags exactly as `BrainInput::PheroARise`
    /// does. It is restated rather than imported because `creature.rs` keeps it
    /// private; if the two ever drift, this census stops describing the sensor
    /// it was built to justify.
    const W_REC: f64 = 0.995;
    let mut tr_here_mem: std::collections::HashMap<u32, f64> = std::collections::HashMap::new();
    // **WHY AN EMPTY ANT READS THE FOOD TRAIL AS EXACTLY ZERO** --
    // `open-bugs-handoff.md` §Z32, the largest loss in the loop. Per EMPTY
    // tick: what channel B holds under the animal, what it holds at the cell
    // the nose actually samples, and how far apart those two cells are
    // vertically. Split by whether the heading is a cardinal or a diagonal,
    // because `trail_sample_point` with projection OFF -- which is shipped --
    // takes a diagonal `so` cells along BOTH axes, six rows up or down, while
    // the hand-laid trail is a five-row band (`lay`: `surface-3 ..= surface+1`).
    //
    // `[cardinal, diagonal]` -> (ticks, sum under the ant, sum at the nose,
    // ticks the nose read zero while the ant's own cell did not, sum |dy|).
    let mut tr_bsniff: [(u64, f64, f64, u64, i64); 2] = [(0, 0.0, 0.0, 0, 0); 2];
    let mut tr_here_prev: std::collections::HashMap<u32, (pixel_physics::sim::pheromone::Scent, i32, i32)> =
        std::collections::HashMap::new();
    let mut tr_smell_along: (f64, u64, f64, u64) = (0.0, 0, 0.0, 0);
    let mut tr_drop_prev: std::collections::HashMap<u32, u8> = std::collections::HashMap::new();
    let mut tr_gate_open = 0u64;
    // **The gate-open and gate-shut populations, pooled across gradient
    // direction** -- the paired arm for "does an open gate turn into homeward
    // motion at all". `tr_up_open`/`tr_down_open` split the open half by which
    // way the ramp points and so cannot be compared against anything: there is
    // no shut counterpart to subtract. These two can, they are two halves of
    // one run rather than two runs, and they are the same `(n, P(move), cells
    // homeward)` triple the `row` helper already prints.
    //
    // **What they are for.** The corpus holds two numbers that do not
    // obviously fit: the gate is open on ~1.84% of laden decisions and the
    // up-minus-down `P(move)` swing when it is open is ~+0.658, yet net
    // homeward motion over every carrying tick is ~+0.0002 cells. If the open
    // decisions converted at anything like that bias the pooled figure would be
    // an order of magnitude larger, so either they do not convert or the shut
    // 98% is cancelling them. Those want different repairs and no aggregate
    // printed so far can tell them apart.
    let mut tr_open = (0u64, 0.0f64, 0i64, 0u64);
    let mut tr_shut = (0u64, 0.0f64, 0i64, 0u64);
    // **Of the gate-open decisions, how many are an ant holding DIRT.**
    // `SPOIL_IS_CARGO` is a measurement switch (default ON) rather than the
    // food/spoil split the roadmap remembers, so `Carrying` is
    // `crop_fill.max(spoil ? 1.0 : 0.0)`: an ant with a pellet of dig tailings
    // reads **1.0 and opens the homing gate**, while an ant with one food item
    // reads 0.667 and does not. Every ant counted here is carrying larder --
    // the trace's own condition -- so this is the share of the open gate that
    // is owed to spoil the ant happens to be holding as well.
    let mut tr_gate_open_spoil = 0u64;
    let gate_threshold = {
        let g = &genome;
        let bias = g[brain::ih_slot(I::Bias, 0)];
        // **`CarryingFood`, since 2026-09-18.** The gate was re-authored onto
        // the food-only sensor; reading `Carrying` here would find a zero
        // weight, fall through the `W_EPS` guard and report a threshold of
        // 0.0 -- a instrument silently answering about a wire that no longer
        // exists, which is the failure this file has already had twice.
        let wcarry = g[brain::ih_slot(I::CarryingFood, 0)];
        if wcarry.abs() < brain::W_EPS {
            0.0
        } else {
            -bias / wcarry
        }
    };
    let mut focal = None;
    // **The cohort, and the stride that spreads it.** `cohort_stride` is set
    // once the founding span is known; until then members are admitted by
    // column so the five are not five neighbours.
    let mut cohort: Vec<pixel_physics::sim::cell::OrganismId> = Vec::new();
    let mut cohort_next_x = i32::MIN;
    let mut focal_rows: Vec<String> = Vec::new();
    let nest_cells = {
        let nest = w.materials.id_of("nest");
        match nest {
            None => 0,
            Some(id) => (0..width).flat_map(|x| (0..spec.height).map(move |y| (x, y))).filter(|&(x, y)| w.get(x, y).material == id).count(),
        }
    };
    // **Where the nest material actually is, as columns.**
    //
    // `nest_x` is the founding *cursor*, and the header printed it as though
    // it were the nest. It is not: `paint_nest_patch` lays a masked comb over
    // `x +/- COLONY_HALF_WIDTH` (26), so at `nest_x = 48` the nest reaches
    // x 22..74 with teeth and gaps, and 25 cells spread over 53 columns is
    // not a place -- it is a fence. Reading "nest 48" and an ant's `AtNest`
    // together produced a confidently wrong extent (2026-09-18): a census of
    // where one ant *went* was reported as where the nest *is*.
    //
    // **`on_nest` is the number this bed actually turns on**: how many
    // founders are born within one cell of nest material. `forage_anchor` is
    // set to the birth cell on the stated grounds that a newly hatched ant
    // has just been at home, so a founder placed off the comb carries a
    // private wrong home for life and `home_bias` steers it there correctly.
    // Every other number here is conditional on this one.
    let (nest_lo, nest_hi, on_nest) = {
        let nest = w.materials.id_of("nest");
        match nest {
            None => (0, 0, 0usize),
            Some(id) => {
                let cols: Vec<i32> = (0..width)
                    .filter(|&x| (0..spec.height).any(|y| w.get(x, y).material == id))
                    .collect();
                let on = w
                    .live_organism_ids()
                    .into_iter()
                    .filter(|&oid| w.organism(oid).is_some_and(|s| s.species == species_id))
                    .filter(|&oid| {
                        let Some((hx, hy)) = w.organism(oid).and_then(|s| s.chain.first().copied()) else {
                            return false;
                        };
                        (-1..=1).any(|dx| (-1..=1).any(|dy| w.get(hx + dx, hy + dy).material == id))
                    })
                    .count();
                (cols.first().copied().unwrap_or(0), cols.last().copied().unwrap_or(0), on)
            }
        }
    };
    // Resolved once, outside the per-tick loop: `born_on_nest` needs it on
    // every ant's first sighting, and a `id_of` per ant per frame is a string
    // hash in the sweep -- `CLAUDE.md`'s guard-at-the-call-site rule.
    let nest_id = w.materials.id_of("nest");
    let (mut first_arrival, mut all_dead_frame) = (0u64, 0u64);
    let mut carry_toward_nest = 0i64;
    let mut occupancy = [0u64; 8];
    let mut founded = (i32::MAX, i32::MIN);
    let (mut laden_by_third, mut spoil_ticks) = ([0u64; 3], 0u64);
    let mut peak_cells = 0usize;
    let midpoint = (nest_x + target_x) / 2;
    // The along reading a real ant would get, averaged over every sample —
    // the instrument's own positive control, because a count that does not
    // move against a gradient that was never there says nothing.
    let (mut along_sum, mut along_n) = (0.0f64, 0u64);
    // **Every closed round trip's homeward leg, and the laden subset.** Kept
    // as the raw samples rather than a running mean: outcomes here have
    // enormous spread, so the median and p90 are what can be quoted and a mean
    // over a long tail is not.
    // **What an ant walking home could actually READ, accumulated over the
    // run.** The time-averaged amplitude profile is not what an animal sees:
    // measured 2026-09-19, the plane is a scatter of decaying bursts whose
    // MEDIAN is 0 from x=78 outward, so a mean profile describes a ramp no ant
    // ever stands on. This counts, per sampled frame and per route cell, the
    // reading an ant facing the nest would get -- and whether it clears a bar
    // an animal has been observed to act on (the cohort member that homed did
    // it on `along` ~0.01).
    let mut read_ok = 0u64;
    let mut read_away = 0u64;
    let mut read_n = 0u64;
    let mut live_cells_n = 0u64;
    let mut live_cells_lit = 0u64;
    let mut legs: Vec<u64> = Vec::new();
    let mut laden_legs: Vec<u64> = Vec::new();
    for f in 1..=frames {
        // **`stop` is what turns this from a pull arm into a loop arm.** Up to
        // `stop` the trail is guaranteed, which breaks the circularity -- a
        // naturally laid trail needs commuters, and commuters need a trail
        // worth following. After it, the only channel B in the world is what
        // the ants themselves put down.
        if (stop == 0 || f <= stop) && (f == 1 || f.is_multiple_of(relay)) {
            if trail {
                lay(&mut w, lay_foot_x, target_x, surface);
            }
            match paint {
                PaintA::None => {}
                PaintA::Ramp => lay_home(&mut w, nest_x, target_x, surface),
                // Half the route each, split at the midpoint the occupancy
                // bands already use, so the blob sits where a real colony's
                // channel A sits in each of the two cases the metric separates.
                PaintA::FlatNest => lay_flat(&mut w, nest_x, (nest_x + target_x) / 2, surface),
                PaintA::FlatFood => lay_flat(&mut w, (nest_x + target_x) / 2, target_x, surface),
            }
        }
        frame::step(&mut w, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        if food > 0 && refill > 0 && f.is_multiple_of(refill) {
            place_food(&mut w, food, &mut larder_placed);
        }
        if f.is_multiple_of(100) {
            for (i, acc) in b_prof_sum.iter_mut().enumerate() {
                let x = nest_x + (target_x - nest_x) * i as i32 / 4;
                *acc += w.pheromone_at(Channel::B, x, surface) as f64;
            }
            b_prof_n += 1;
            // **The food trail as a SERIES, which is the only form that can
            // answer "what happens when the hand-laid one is withdrawn".**
            //
            // `b_profile` above is a mean over the whole run and is dominated
            // by the hand-laid era whenever `stop=` is set -- see its doc. This
            // prints the *current* plane, so the handover at frame `stop` is
            // visible as a cliff rather than averaged into a healthy-looking
            // ramp. Sampled along `lay_foot_x..target_x` so it covers the
            // extended band under `layfrom=founders` too.
            //
            // Emitted as `BTRAIL` rows on stdout for a parser to pick up. The
            // row carries `stop` in every line rather than once in a header:
            // `CLAUDE.md`'s harness-echoes-its-own-parameters rule, after a
            // 3.5-hour study turned out to be one parameter wearing 24 logs.
            if btrail && f.is_multiple_of(btrail_every) {
                // **Every cell of the route, not a handful of sample points.**
                // The question is *where* the trail survives once the hand-laid
                // one stops being refreshed, and a colony's own channel B is a
                // scatter of decaying bursts a few cells wide -- five or nine
                // probes spaced 20-odd cells apart step straight over it and
                // report a clean empty plane. The row is long; it is a log for
                // a parser (`scripts/btrailchart.py`), not for reading by eye,
                // and the summary columns beside it are the by-eye version.
                let mut prof: Vec<String> = Vec::with_capacity((target_x - lay_foot_x + 1).max(0) as usize);
                let (mut live, mut peak) = (0usize, 0u32);
                for x in lay_foot_x..=target_x {
                    let v = w.pheromone_at(Channel::B, x, surface) as u32;
                    if v > 0 {
                        live += 1;
                    }
                    peak = peak.max(v);
                    prof.push(v.to_string());
                }
                println!(
                    "BTRAIL seed={seed} arm={} stop={stop} gap={gap} layfrom={layfrom} x0={lay_foot_x} x1={target_x} \
                     frame={f} hand={} cells={live} peak={peak} prof={}",
                    gate.name,
                    u8::from(stop == 0 || f <= stop),
                    prof.join(",")
                );
            }
            let mut amt = 0u32;
            let mut cells = 0usize;
            for x in nest_x..=target_x {
                let v = w.pheromone_at(Channel::A, x, surface) as u32;
                amt = amt.max(v);
                if v > 0 {
                    cells += 1;
                }
            }
            a_peak_amt = a_peak_amt.max(amt);
            a_peak_cells = a_peak_cells.max(cells);
            // Facing home is -x here, so `ahead` is the cell one sensor length
            // toward the nest. The guard matches `sense`'s.
            for x in nest_x..=target_x {
                let here = w.pheromone_at(Channel::A, x, surface) as f32;
                let ahead = w.pheromone_at(Channel::A, x - sensor_span, surface) as f32;
                let along = (ahead - here) / (ahead + here + pixel_physics::sim::pheromone::SCALE as f32);
                read_n += 1;
                if along >= 0.02 {
                    read_ok += 1;
                }
                // **The same reading taken facing the other way, and it is a
                // discriminator rather than a second statistic.** The homeward
                // figure alone cannot tell two very different planes apart: a
                // ramp that points at the FOOD (§7.15's polarity inversion --
                // then foodward is high and homeward low) and a plane that is
                // a scatter of local maxima (then BOTH are low, because an ant
                // standing on a mound reads downhill in every direction). The
                // laden traces put the down:up ratio at 14-20:1, which needs
                // one of those two explanations and the columns as they stood
                // could not say which.
                let behind = w.pheromone_at(Channel::A, x + sensor_span, surface) as f32;
                let away = (behind - here) / (behind + here + pixel_physics::sim::pheromone::SCALE as f32);
                if away >= 0.02 {
                    read_away += 1;
                }
                live_cells_n += 1;
                if here > 0.0 {
                    live_cells_lit += 1;
                }
            }
            // **`aprofile` dumps the plane itself, not what an ant read off
            // it.** `PheroAAlong` is a GRADIENT -- ahead minus here -- so a
            // 0.0000 reading means *flat*, which a plane that is absent and a
            // plane that is saturated both produce. Reading the amplitude
            // against x is the only thing that tells those two apart, and the
            // question "does an outbound ant lay this all the way to the food"
            // is about the amplitude.
            if a_profile && f.is_multiple_of(a_prof_every) {
                let cols: Vec<String> = (nest_x..=target_x)
                    .step_by(a_prof_step)
                    .map(|x| format!("{}:{}", x, w.pheromone_at(Channel::A, x, surface)))
                    .collect();
                println!("    APROF f={f} {}", cols.join(" "));
            }
            if cells > 0 {
                let (mut sm, mut n) = (0.0f64, 0u64);
                let (mut sm_both, mut n_both) = (0.0f64, 0u64);
                let mut edge = 0u64;
                for x in nest_x..=(target_x - sensor_offset) {
                    let here = w.pheromone_at(Channel::A, x, surface) as f64;
                    let ahead = w.pheromone_at(Channel::A, x + sensor_offset, surface) as f64;
                    if here > 0.0 || ahead > 0.0 {
                        // Negated so POSITIVE = taller at the nest, which is
                        // the shape a homing reader needs. Negative means the
                        // ramp points at the food.
                        let v = -((ahead - here) / (ahead + here + pheromone::SCALE as f64));
                        sm += v;
                        n += 1;
                        // **`&&` is the same statistic over interior cells
                        // only.** A cell with an empty neighbour is not
                        // reporting a gradient, it is reporting where the blob
                        // ends -- and it reports it at about twenty times an
                        // interior cell's weight. Both figures are carried
                        // because neither is obviously the right one: `&&`
                        // cannot see a genuine ramp that runs off the end of
                        // the trail, and `||` cannot tell that ramp from a
                        // blob that merely sits nearer the food.
                        if here > 0.0 && ahead > 0.0 {
                            sm_both += v;
                            n_both += 1;
                        } else {
                            edge += 1;
                        }
                    }
                }
                if n > 0 {
                    a_pol_sum += sm / n as f64;
                    a_pol_n += 1;
                    a_edge_sum += edge;
                    a_edge_n += 1;
                }
                if n_both > 0 {
                    a_pol_both_sum += sm_both / n_both as f64;
                    a_pol_both_n += 1;
                }
                // **The same statistic over the trail's OWN span**, which is
                // the one the flat controls do not fool. Both figures above
                // scan `nest_x ..= target_x - sensor_offset`, a window fixed to
                // the ROUTE; a blob has two shoulders and that window decides
                // which of them gets counted, so it reads the blob's POSITION
                // as a shape. Anchored to `[lo, hi]` instead, both shoulders
                // are always inside and a blob with no ramp in it cancels.
                // **Searched over the whole row, NOT over the route, and that
                // distinction is the entire finding.** Anchored inside
                // `[nest_x, target_x]` this reproduced the `&&` figure exactly
                // -- +-0.18061 on 6 of 6 seeds of both flat arms -- because a
                // blob diffuses past the route's ends, so the shoulder that
                // would cancel the one being counted is off the measured
                // segment and no window drawn inside it can ever find it.
                let mut lo = i32::MAX;
                let mut hi = i32::MIN;
                for x in 0..width {
                    if w.pheromone_at(Channel::A, x, surface) > 0 {
                        lo = lo.min(x);
                        hi = hi.max(x);
                    }
                }
                if hi - lo > 2 * sensor_offset {
                    let (mut sm_s, mut n_s) = (0.0f64, 0u64);
                    for x in lo..=(hi - sensor_offset) {
                        let here = w.pheromone_at(Channel::A, x, surface) as f64;
                        let ahead = w.pheromone_at(Channel::A, x + sensor_offset, surface) as f64;
                        if here > 0.0 && ahead > 0.0 {
                            sm_s += -((ahead - here) / (ahead + here + pheromone::SCALE as f64));
                            n_s += 1;
                        }
                    }
                    if n_s > 0 {
                        a_pol_span_sum += sm_s / n_s as f64;
                        a_pol_span_n += 1;
                    }
                }
            }
        }
        let live_now = w.live_creature_count();
        alive_min = alive_min.min(live_now);
        if live_now == 0 && all_dead_frame == 0 {
            all_dead_frame = f;
        }
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
        //
        // **Gated on whether THIS ARM hand-lays, not on `stop`, since
        // 2026-09-17 -- and the old gate made the column dead.** `stop`
        // defaults to 0, which means "hand-lay for the whole run"; the
        // condition `stop > 0` therefore reported **`route pk 0` in 60 of 60
        // rows** of a default 12-seed five-arm sweep, including in `self`,
        // `mute` and `homeA`, which hand-lay nothing at all and had a real
        // number to give. A column that is structurally 0 reads exactly like a
        // colony that laid nothing, which is the finding it sits next to.
        //
        // The original intent is kept and is right for the arm it was written
        // for: on a hand-laid arm the count must wait out our own trail or it
        // measures us. `1500` rather than the original `288` because that came
        // from a `u8`-era lifetime the `u16` widening invalidated the day after
        // it was written -- see the note above. On an arm that lays nothing of
        // ours, every frame is fair game.
        let ours_is_down = trail || paint != PaintA::None;
        // **3,000, not 1,500, and the old margin was inside the noise.**
        // `pherolife` on current `main` reports a laid trail **gone at 1,476
        // frames**, so a window opening at `stop + 1500` opened 24 frames after
        // our own trail died -- a 1.6% margin on a decay curve, which is no
        // margin at all. It is why `route pk` once read **88 of 91 in all four
        // arms alike, including an arm where the ants emit no channel B**:
        // that was our residue being counted as theirs. At `stop + 3000` the
        // margin is 2x the measured lifetime.
        let past_our_trail = if ours_is_down { stop > 0 && f > stop + 3000 } else { true };
        if past_our_trail && f.is_multiple_of(100) {
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
            let carrying_larder = s.crop.is_some_and(|c| c.material == larder);
            // **`focalany` traces an ant that never finds food, and without it
            // the failing case is invisible.** The focal ant was chosen from
            // larder-carriers only, so in a seed where nobody reaches the food
            // there is no focal ant and the CSV is empty -- the instrument
            // could see every run except the ones that fail. Measured
            // 2026-09-18: at gap 90 with a hand-laid trail, four of six seeds
            // put **0 or 1** ants of 20 on the larder, and those four produced
            // no per-tick record at all.
            let take_as_focal = match (focal_x, focal_any) {
                // Nearest to the requested column, re-evaluated while no ant has
                // been chosen yet: the first tick's sweep settles it.
                (Some(fx), _) => focal.is_none() && (hx - fx).abs() <= 2,
                (None, true) => focal.is_none(),
                (None, false) => carrying_larder && focal.is_none(),
            };
            // **Cohort admission, by column, at first sighting.** Spread over
            // the founding band: an ant is admitted only if it stands at least
            // `relay/focal_n` columns east of the last one taken, so the five
            // sample the colony rather than its western edge.
            if focal_n > 0 && cohort.len() < focal_n && !cohort.contains(&id) {
                // **The stride spans the FOUNDING BAND, not `relay`.** The
                // first cut divided `relay` here, which is the trail re-laying
                // interval in *frames* -- a number with no business setting a
                // distance in columns. It happened to give 10 and spread the
                // cohort over x 12..72, so it looked right; `relay=600` would
                // have put all six on the same ant. `ants * 3` is the span
                // `plant_creature_seed_in` actually lays founders over below.
                let stride = ((ants * 3) / focal_n as i32).max(1);
                if cohort_next_x == i32::MIN || hx >= cohort_next_x {
                    cohort.push(id);
                    cohort_next_x = hx + stride;
                }
            }
            let in_cohort = cohort.contains(&id);
            if tracing && (in_cohort || carrying_larder || focal == Some(id) || take_as_focal) {
                // **The focal ant is the first to pick larder up**, traced for
                // the rest of its life -- or, under `focalany`, simply the first
                // ant seen, carrying or not. One ant is n=1 in a chaotic system,
                // so it is the illustration and the aggregate below is the
                // result; both are printed because a mean over a bimodal
                // population describes no ant that exists.
                if take_as_focal {
                    focal = Some(id);
                }
                let cdef = w.species.get(species_id).creature.clone().expect("ant is a creature");
                let (tin, thid, tout, _) = pixel_physics::sim::creature::probe_full(&w, hx, hy, id, &cdef);
                let (terms, presquash) = move_terms(&s.genome, &tin, &thid);
                // **The decomposition's own check, and it is free.** If the
                // named terms do not reproduce the number the brain computed,
                // the decomposition is wrong and every conclusion drawn from
                // it is about arithmetic this file invented.
                let rebuilt = brain::squash(presquash);
                assert!(
                    (rebuilt - tout[O::Move as usize]).abs() < 1e-4,
                    "move_terms does not reproduce eval_brain: rebuilt {rebuilt} against {}; the decomposition is wrong, not the colony",
                    tout[O::Move as usize]
                );
                for (n, v) in &terms {
                    let e = tr_terms.entry(n.clone()).or_insert((0.0, 0));
                    e.0 += *v as f64;
                    e.1 += 1;
                }
                // **What kind of place the nose is pointing at, for every
                // traced ant** -- hoisted out of the focal-only block below
                // because the census needs it for the population, not for one
                // illustration. `ahead` reproduces `creature::sense`'s own
                // sample point, `(x + dx*so, y + dy*so)` from `DIRS`; `sense`
                // is private so this is a copy, and the guard test in
                // `creature.rs` is what keeps the two honest.
                // **The engine's own helper, not a copy of its arithmetic.** The
                // first version of this census restated `(x + dx*so, y + dy*so)`
                // from `DIRS`, and the day `sense` stopped sampling there the
                // census went on labelling ticks by a cell nothing reads --
                // reporting the repair as inert. `false` because a laden forager
                // is walking, never airborne.
                let (ax, ay) = creature::trail_sample_point(hx, hy, s.heading, sensor_offset, false, creature::sensor_projected());
                let here_a = w.pheromone_at(Channel::A, hx, hy);
                let ahead_a = w.pheromone_at(Channel::A, ax, ay);
                let solid_at = |cx: i32, cy: i32| {
                    matches!(
                        w.materials.kind(w.get(cx, cy).material),
                        MaterialKind::Solid | MaterialKind::Powder | MaterialKind::Plant
                    )
                };
                // `surface` is the walkability test `step_chain` uses (P-25):
                // not solid itself, and 8-adjacent to something to stand on.
                // It is the only one of the three where a trail could be.
                let kind_idx = if solid_at(ax, ay) {
                    1usize
                } else if (-1..=1).any(|ddx| (-1..=1).any(|ddy| (ddx, ddy) != (0, 0) && solid_at(ax + ddx, ay + ddy))) {
                    2usize
                } else {
                    0usize
                };
                let sensor_kind = ["air", "solid", "surface"][kind_idx];
                // **One creature tick, not one frame.** `since_nest` is
                // incremented once per tick (`creature.rs:4647`) and reset only
                // at the nest, so a change in it marks a tick including the
                // reset. Rows are written every frame, so a census that did not
                // gate on this would count each decision six times -- harmless
                // for a ratio and wrong for a run LENGTH, which is the number
                // the freeze census exists to report.
                let tick_e = tick_of.entry(id).or_insert((u16::MAX, 0));
                let is_tick = u8::from(tick_e.0 != s.since_nest);
                if is_tick == 1 {
                    tick_e.0 = s.since_nest;
                    tick_e.1 += 1;
                }
                let tick = tick_e.1;
                // **Two gates had to be cleared for this to run at all, and
                // the first two attempts printed nothing.** It needs `focaln=`,
                // because the tracing block is `in_cohort || carrying_larder ||
                // ...` and without a cohort an empty ant is never traced. And
                // it has to sit ABOVE the `if carrying_larder {` that opens
                // further down and does not close until past the temporal
                // census -- nested inside that, a `!carrying_larder` census is
                // dead by construction. Both are the same shape as a guard
                // whose inputs cannot vary what it guards, arriving in a
                // measurement; the tell was a counter flat at exactly 0.
                if !carrying_larder {
                    tr_bsniff[0].3 += 0; // reached
                }
                if !carrying_larder && is_tick == 1 {
                    let (nx, ny) = creature::trail_sample_point(hx, hy, s.heading, sensor_offset, false, creature::sensor_projected());
                    let under = f64::from(w.pheromone_at(Channel::B, hx, hy));
                    let nose = f64::from(w.pheromone_at(Channel::B, nx, ny));
                    let diag = usize::from(creature::DIRS[s.heading as usize % 8].1 != 0 && creature::DIRS[s.heading as usize % 8].0 != 0);
                    let e = &mut tr_bsniff[diag];
                    e.0 += 1;
                    e.1 += under;
                    e.2 += nose;
                    e.3 += u64::from(nose == 0.0 && under > 0.0);
                    e.4 += i64::from((ny - hy).abs());
                }
                let along = tin[I::PheroAAlong as usize];
                // **`clamp`, not `unit_scale` -- corrected 2026-09-19, and every
                // `P(move)` figure in §7.41-§7.44 was in the wrong unit.**
                // `creature.rs`'s move roll is
                // `outputs[Move].clamp(0.0, 1.0)`; this line used
                // `unit_scale(out, 1.0)` = `(out + 1) / 2`, which is the
                // convention `Tumble`, `Persist` and `Caution` are read with and
                // `Move` is not. The two differ most exactly where this ant
                // lives: **every negative `Move` output prints as something
                // between 0 and 0.5 under `unit_scale` and is rolled as a hard
                // zero.**
                //
                // It was caught by the control that costs nothing -- the traces
                // already carry positions, so the step rate per bucket says
                // which function the engine is using. Weighted absolute error
                // over 13,248 ticks: **1.4 points for `clamp`, 27.7 for
                // `unit_scale`**. In the four lowest buckets the column claimed
                // 5-35% and the ants stepped **0 times in 6,819 ticks**.
                //
                // The correction is not cosmetic. Under `clamp` the engine's
                // `P(move)` is **exactly zero on 48-72% of ticks**, and on those
                // ticks the homing term cannot express itself at all: zero plus
                // a small number is still zero. So the ratchet that §7.44
                // reported as a smooth +0.15 is really two mechanisms, and the
                // harness was reading their sum through a lens that hid the
                // split -- see §7.45.
                let p_move = tout[O::Move as usize].clamp(0.0, 1.0) as f64;
                // The trail's whole contribution: hidden 0/1 are the channel A
                // pair and nothing else drives `Move` from them.
                let trail = terms.iter().filter(|(n, _)| n == "h0" || n == "h1").map(|(_, v)| *v).sum::<f32>();
                // Signed step toward the nest, needed by both the aggregate
                // below and the focal row, so it lives outside the laden guard.
                let dx = tracks.get(&id).filter(|t| t.seen).map_or(0, |t| t.last_x - hx);
                // **Every aggregate below counts LADEN decisions only.** The
                // focal ant may now be an empty one (`focalany`), and letting it
                // into these sums would quietly redefine `n` from "laden
                // decisions" to "laden decisions plus one ant's whole life" --
                // a denominator change that moves every rate in the block and
                // looks like a result.
                if carrying_larder {
                // **Per TICK, unlike the accumulators below**, which have always
                // counted frames -- harmless for the ratios they report and
                // wrong for a run length, which is what the freeze census is.
                if is_tick == 1 {
                    let k = &mut tr_by_kind[kind_idx];
                    k.0 += 1;
                    k.1 += along as f64;
                    k.2 += u64::from(along >= 0.02);
                    let frozen = p_move == 0.0;
                    k.3 += u64::from(frozen);
                    let run = tr_freeze_open.entry(id).or_insert(0);
                    if frozen {
                        *run += 1;
                    } else if *run > 0 {
                        tr_freeze_runs.push(*run);
                        *run = 0;
                    }
                }
                tr_n += 1;
                tr_along_sum += along as f64;
                tr_abs_along_sum += along.abs() as f64;
                tr_along_hist[(((along + 1.0) * 4.5) as usize).min(8)] += 1;
                tr_pmove_sum += p_move;
                tr_trail_sum += trail as f64;
                tr_dx_home += dx as i64;
                // The threshold is `creature::sense`'s own guard scale expressed
                // back as an `along`: below this the reader is looking at two
                // cells it cannot tell apart, so it is neither up nor down.
                // **The alignment census** -- see `tr_align`. Guarded on a
                // vector of at least one cell for the same reason
                // `home_weighted_pick` is: standing on the anchor, every
                // heading scores alike and the bearing is meaningless.
                // **The drop census, read from this ant's own brain.** Only
                // laden ticks: an empty ant has nothing to put down and would
                // swamp every bucket.
                if carrying_larder {
                    // Nearest nest material to the HEAD, which is the quantity
                    // `AtNest` answers at radius 1. Bounded and widening, so a
                    // hit costs the small ring rather than the whole box.
                    let mut dist = u8::MAX;
                    if let Some(nid) = nest_id {
                        'ring: for r in [1i32, 2, 4, 8, 16, 32] {
                            for dy in -r..=r {
                                for dx in -r..=r {
                                    if dx.abs() != r && dy.abs() != r {
                                        continue;
                                    }
                                    if w.get(hx + dx, hy + dy).material == nid {
                                        dist = r.min(255) as u8;
                                        break 'ring;
                                    }
                                }
                            }
                        }
                    }
                    // The offset to that nearest material, kept only for near
                    // misses -- see `tr_miss`.
                    if dist != u8::MAX && dist <= 4 && dist > 1 {
                        if let Some(nid) = nest_id {
                            let mut best: Option<(i32, i32)> = None;
                            for dy in -4i32..=4 {
                                for dx in -4i32..=4 {
                                    if w.get(hx + dx, hy + dy).material == nid {
                                        let d = dx.abs().max(dy.abs());
                                        if best.is_none_or(|(bx, by)| d < bx.abs().max(by.abs())) {
                                            best = Some((dx, dy));
                                        }
                                    }
                                }
                            }
                            if let Some((dx, dy)) = best {
                                tr_miss[dx.unsigned_abs().min(4) as usize][dy.unsigned_abs().min(4) as usize] += 1;
                                // One step toward the comb against one step
                                // away -- the plane's own answer, no sensor.
                                let (sx, sy) = (dx.signum(), dy.signum());
                                let toward = f64::from(w.pheromone_at(Channel::A, hx + sx, hy + sy));
                                let away = f64::from(w.pheromone_at(Channel::A, hx - sx, hy - sy));
                                tr_smell.0 += 1;
                                tr_smell.1 += toward;
                                tr_smell.2 += away;
                                tr_smell.3 += u64::from(toward > away);
                                tr_smell.4 += u64::from((toward - away).abs() < f64::EPSILON);
                                // ...and what the ANT reads, split by whether
                                // it is already pointed at the comb.
                                let (hdx, hdy) = creature::DIRS[s.heading as usize % 8];
                                let along = f64::from(tin[I::PheroAAlong as usize]);
                                if hdx * dx + hdy * dy > 0 {
                                    tr_smell_along.0 += along;
                                    tr_smell_along.1 += 1;
                                } else {
                                    tr_smell_along.2 += along;
                                    tr_smell_along.3 += 1;
                                }
                            }
                        }
                    }
                    let b = match dist {
                        1 => 0,
                        2 => 1,
                        4 => 2,
                        8 => 3,
                        16 => 4,
                        32 => 5,
                        _ => 6,
                    };
                    let urge = tout[O::Drop as usize].clamp(0.0, 1.0) as f64;
                    tr_drop[b].0 += 1;
                    tr_drop[b].1 += urge;
                    // A drop is a fall in crop cells between two laden ticks --
                    // the effect counter from the far side of the call, so the
                    // urge and what it produced are never read apart.
                    let cells = s.crop.map_or(0, |c| c.cells.min(255) as u8);
                    if let Some(&prev) = tr_drop_prev.get(&id) {
                        if cells < prev {
                            tr_drop[b].2 += 1;
                        }
                    }
                    tr_drop_prev.insert(id, cells);
                } else {
                    tr_drop_prev.remove(&id);
                }
                // **The comparator pre-check** -- every laden tick, for each
                // candidate offset pair, what a two-forward-sample reading
                // WOULD say. Read off the plane directly: this asks whether
                // the signal is there to be had, not whether today's sensor
                // sees it.
                if carrying_larder {
                    let (anx, any) = s.forage_anchor;
                    let (vx, vy) = (anx - hx, any - hy);
                    if vx != 0 || vy != 0 {
                        let (hdx, hdy) = creature::DIRS[s.heading as usize % 8];
                        let homeward = hdx * vx + hdy * vy > 0;
                        let guard = f64::from(pixel_physics::sim::pheromone::SCALE);
                        for (i, (near_off, far_off)) in CMP_PAIRS.iter().enumerate() {
                            let near = f64::from(w.pheromone_at(Channel::A, hx + hdx * near_off, hy + hdy * near_off));
                            let far = f64::from(w.pheromone_at(Channel::A, hx + hdx * far_off, hy + hdy * far_off));
                            if near == 0.0 && far == 0.0 {
                                tr_cmp[i].0 += 1;
                                continue;
                            }
                            let along = (far - near) / (far + near + guard);
                            if homeward {
                                tr_cmp[i].1 += 1;
                                tr_cmp[i].2 += along;
                            } else {
                                tr_cmp[i].3 += 1;
                                tr_cmp[i].4 += along;
                            }
                        }
                    }
                }
                // **THE TEMPORAL PRE-CHECK -- does smelling over TIME carry
                // the direction the spatial read cannot?** Owner's ruling,
                // 2026-09-20: reopen §7.48. This is the precondition, asked the
                // same way `tr_cmp` asks the spatial one -- off the plane
                // directly, before any wiring -- because a signal the plane
                // does not carry cannot be rescued by a gain.
                //
                // **The reason it is worth asking again is structural, not a
                // retune.** `other:134` measured that every SPATIAL repair
                // erodes the ramp it reads, because channel A is written by the
                // same animals that read it. A difference between two cells at
                // one instant keeps the animal's own mark on one side only; a
                // difference of ONE cell across time has that mark on both
                // sides, where a slowly-varying contribution cancels. The
                // odometer's output moves at `recurrence 0.99995`, so the
                // self-deposit is exactly that kind of term.
                //
                // Split three ways, because the confound is as interesting as
                // the signal: pointed home against pointed away is the signal;
                // MOVED against FROZEN is the confound, since an ant that does
                // not step keeps depositing on the cell it is standing on and
                // watches its own mark climb, which reads as up-gradient while
                // it goes nowhere. `P(move)` is exactly 0 on 48-72% of laden
                // ticks, so that arm is most of the data.
                if carrying_larder && is_tick == 1 {
                    let (anx, any) = s.forage_anchor;
                    let (vx, vy) = (anx - hx, any - hy);
                    // **Two lags, because they are different claims.** The
                    // ONE-TICK difference is the harshest reading and the one
                    // that exposes the self-deposit: an ant that steps deposits
                    // on the cell it arrives at, so its own mark sits on the
                    // NEW side only and does not cancel -- the same asymmetry
                    // that kills the spatial repair, arriving through the back
                    // door. The EXPONENTIAL memory is what the wiring actually
                    // delivers (`PHERO_A_MEM_RECURRENCE`), and there the self-deposit
                    // is in both terms, so it can cancel. Measuring only the
                    // first would condemn a mechanism nobody proposed.
                    let mem = tr_here_mem.entry(id).or_insert(f64::from(here_a));
                    let lagged = *mem;
                    *mem = W_REC * *mem + (1.0 - W_REC) * f64::from(here_a);
                    let prev = tr_here_prev.insert(id, (here_a, hx, hy));
                    if let Some((_, px_, py_)) = prev {
                        if vx != 0 || vy != 0 {
                            let (hdx, hdy) = creature::DIRS[s.heading as usize % 8];
                            let homeward = usize::from(hdx * vx + hdy * vy > 0);
                            let moved = usize::from((px_, py_) != (hx, hy));
                            let dist = ((vx * vx + vy * vy) as f64).sqrt() as i32;
                            let band = DIST_BANDS.iter().position(|&b| dist <= b).unwrap_or(3);
                            let live = f64::from(here_a);
                            let guard = f64::from(pixel_physics::sim::pheromone::SCALE);
                            let d = (live - lagged) / (live + lagged + guard);
                            let e = &mut tr_temporal[band][moved][homeward];
                            e.0 += 1;
                            e.1 += d;
                            e.2 += u64::from(d > 0.0);
                            // **The LEVEL, per band, and it is the crux.**
                            // §7.48's fit cancels the level term at one value
                            // of the level; channel A is a ramp, so the level
                            // is a function of distance from the nest. If it
                            // varies across these bands, a constant `w_in`
                            // cannot cancel it everywhere and the wiring is
                            // correct at exactly one distance from home.
                            e.3 += live;
                        }
                    }
                }
                {
                    let (anx, any) = s.forage_anchor;
                    let (vx, vy) = ((anx - hx) as f32, (any - hy) as f32);
                    let vlen = (vx * vx + vy * vy).sqrt();
                    if vlen >= 1.0 {
                        let (hdx, hdy) = creature::DIRS[s.heading as usize % 8];
                        let align = (hdx as f32 * vx + hdy as f32 * vy) / vlen;
                        let b = &mut tr_align[(((align + 1.0) * 2.5) as usize).min(4)];
                        b.0 += 1;
                        b.1 += along as f64;
                        b.2 += u64::from(along > 1e-3);
                        b.3 += p_move;
                    }
                }
                let bucket = if along > 1e-3 {
                    &mut tr_up
                } else if along < -1e-3 {
                    &mut tr_down
                } else {
                    &mut tr_flat
                };
                bucket.0 += 1;
                bucket.1 += p_move;
                bucket.2 += dx as i64;
                bucket.3 += u64::from(p_move > 0.0);
                // The gate's own input, not the mandibles-full one.
                let carry = tin[I::CarryingFood as usize];
                tr_carry_hist[((carry * 10.0) as usize).min(9)].add(p_move, dx);
                let open = carry >= gate_threshold;
                let pooled = if open { &mut tr_open } else { &mut tr_shut };
                pooled.0 += 1;
                pooled.1 += p_move;
                pooled.2 += dx as i64;
                pooled.3 += u64::from(p_move > 0.0);
                if open {
                    tr_gate_open += 1;
                    if s.spoil.is_some() {
                        tr_gate_open_spoil += 1;
                    }
                    if along > 1e-3 {
                        tr_up_open.0 += 1;
                        tr_up_open.1 += p_move;
                        tr_up_open.2 += dx as i64;
                        tr_up_open.3 += u64::from(p_move > 0.0);
                    } else if along < -1e-3 {
                        tr_down_open.0 += 1;
                        tr_down_open.1 += p_move;
                        tr_down_open.2 += dx as i64;
                        tr_down_open.3 += u64::from(p_move > 0.0);
                    }
                }
                }
                if focal == Some(id) || in_cohort {
                    focal_rows.push(format!(
                        "{id:?},{},{f},{hx},{hy},{dx},{},{},{:.5},{:.5},{along:.5},{:.5},{:.4},{:.4},{},{},{},{:.4},{:.4},{:.4},{:.4},{:.4},{:.5},{:.5},{:.5},{:.5},{:.5},{:.5},{p_move:.5},{:.5},{trail:.5},{presquash:.5},{:.5},{:.5},{:.5},{tick},{is_tick},{here_a},{ahead_a},{sensor_kind}",
                        // **Where this ant thinks home is, and how stale that
                        // is** -- `OrganismState::forage_anchor` / `since_nest`.
                        //
                        tracks.get(&id).map_or(0, |t| t.stage),
                        // Here because `home_bias` aims the tumble at the
                        // ANCHOR, not at the nest, and the two are only the
                        // same cell for an ant that has touched nest material.
                        // A founder is placed where the harness spreads it and
                        // anchors *there* at birth, so without this column a
                        // laden ant walking confidently to the wrong place is
                        // indistinguishable from one that will not steer at
                        // all -- and `tumbles_homeward` reads the same either
                        // way, because the aim fired correctly both times.
                        s.forage_anchor.0,
                        s.since_nest,
                        // **The two inputs the return leg now runs on**, so a
                        // per-tick row can say whether the ant could see home
                        // and whether the trail under it was rising. Without
                        // them the brain columns describe a decision made on
                        // numbers the row does not contain.
                        tin[I::PheroARise as usize],
                        tin[I::HomeAligned as usize],
                        tin[I::PheroAFront as usize],
                        tin[I::Carrying as usize],
                        // **`CarryingFood` is the column that decides the gate**
                        // since 2026-09-18, and a per-tick record without it
                        // cannot say why the homing pair was open or shut. The
                        // old `Carrying` stays beside it: the two disagreeing is
                        // exactly the spoil case, and seeing them differ on one
                        // row is worth more than either alone.
                        tin[I::CarryingFood as usize],
                        s.crop.map_or(0, |c| c.cells),
                        u8::from(s.spoil.is_some()),
                        // Which way the body is pointing, so a heading change
                        // is visible as an event rather than inferred from `dx`.
                        s.heading,
                        tin[I::Energy as usize],
                        tin[I::Crowding as usize],
                        tin[I::AtNest as usize],
                        tin[I::FoodAdjacent as usize],
                        tin[I::Stillness as usize],
                        thid[0],
                        thid[1],
                        // **Channel B and its reader pair.** The homing half
                        // (h0/h1, `PheroAAlong`) was all this row carried, so it
                        // could not answer "why did this ant not follow the food
                        // trail" -- which is the question four of six seeds are
                        // asking.
                        tin[I::PheroBAlong as usize],
                        tin[I::PheroBFront as usize],
                        thid[2],
                        thid[3],
                        // The run-or-tumble roll's other side: `p_move` is the
                        // chance of stepping along the current heading, and this
                        // is the chance of re-rolling it. Reading one without
                        // the other cannot tell "stood still" from "turned".
                        // **`unit_scale`, not `clamp` -- fixed 2026-09-19, and
                        // the bug was not cosmetic.** `creature.rs` rolls the
                        // re-heading against
                        // `brain::unit_scale(outputs[Tumble], 1.0)`, which is
                        // `(out + 1) / 2`, so a raw output of **0.0 is a 50%
                        // tumble chance**. Clamping the raw output instead
                        // reported **0.0000 on every row of every ant**, and the
                        // obvious reading of that column -- "the ants never
                        // change direction, so of course they never find
                        // anything" -- is the opposite of the truth. `p_move`
                        // one line up had always scaled correctly, which is what
                        // made the pair look consistent enough to trust.
                        //
                        // **It is a conditional probability and the column
                        // cannot say so**: `step` only reaches the tumble roll
                        // in the `else` of a move that did not happen, so this
                        // is P(re-roll | did not move), not P(re-roll).
                        brain::unit_scale(tout[O::Tumble as usize], 1.0),
                        // **The drop verb and the two terms that drive it away
                        // from the nest.** `mode=feedgate` computes `drop_urge`
                        // with `MoistureGrad` and `SurfaceCurvature` set to
                        // zero, which is a floor rather than a field value --
                        // and `ant.ron` authors both into `Drop` at 0.169. At a
                        // food *heap* the curvature term is exactly what is not
                        // zero, so the synthetic reading cannot explain a drop
                        // that happens there. Log the real ones.
                        tout[O::Drop as usize].clamp(0.0, 1.0),
                        tin[I::MoistureGrad as usize],
                        tin[I::SurfaceCurvature as usize],
                    ));
                }
            }
            {
                let at_food = (hx - target_x).abs() <= near;
                let at_nest = (hx - nest_x).abs() <= 26;
                let first_sighting = !tracks.contains_key(&id);
                let t = tracks.entry(id).or_default();
                if first_sighting {
                    t.born_x = hx;
                    t.born_on_nest = nest_id.is_some_and(|nid| {
                        (-1..=1).any(|dx| (-1..=1).any(|dy| w.get(hx + dx, hy + dy).material == nid))
                    });
                }
                t.far = t.far.max(hx - nest_x);
                if carrying_larder && t.seen {
                    // Toward the nest is -x, so negate: positive means the
                    // step carried food homeward.
                    carry_toward_nest += (t.last_x - hx) as i64;
                }
                t.last_x = hx;
                t.seen = true;
                if f == 1 {
                    founded = (founded.0.min(hx), founded.1.max(hx));
                }
                let band = (((hx - nest_x) * 8) / (target_x - nest_x).max(1)).clamp(0, 7) as usize;
                occupancy[band] += 1;
                if at_food {
                    if !t.visited && first_arrival == 0 {
                        first_arrival = f;
                    }
                    t.visited = true;
                    t.outbound = true;
                    t.left_food = f;
                }
                // **Only a pickup that STARTS A RETURN LEG counts**, i.e. one
                // made by an ant that has reached the food. Without `outbound`
                // the denominator is swamped by nest-local churn -- ants
                // loitering on the comb picking the same cells up and putting
                // them down, which can never close a loop by construction and
                // which lands almost entirely in the anchor-ok group because
                // those ants have just touched nest material. Measured
                // 2026-09-20 before the guard: 3,600 anchor-ok pickups against
                // 137 anchor-bad, and the rates came out 37x the WRONG way.
                if carrying_larder && t.laden_since == 0 && t.outbound {
                    t.laden_since = f;
                    // **The causal reading, taken at the pickup.** Same
                    // 8-neighbour test for nest material `creature::sense`
                    // answers `AtNest` with, applied to the cell the homing
                    // vector actually points at.
                    let (anx, any) = s.forage_anchor;
                    let ok = nest_id.is_some_and(|nid| {
                        (-1..=1).any(|dx| (-1..=1).any(|dy| w.get(anx + dx, any + dy).material == nid))
                    });
                    t.anchor_on_comb_at_pickup = Some(ok);
                    if ok { t.picked_up_anchor_ok += 1 } else { t.picked_up_anchor_bad += 1 }
                }
                // **THE FUNNEL, advanced here and nowhere else.** Monotone:
                // `t.stage = t.stage.max(n)` so an ant is booked at its
                // high-water mark. See `Track::stage`.
                let anchor_dist = {
                    let (anx, any) = s.forage_anchor;
                    (anx - hx).abs().max((any - hy).abs())
                };
                let cells = s.crop.map_or(0, |c| c.cells.min(255) as u8);
                if at_food {
                    t.stage = t.stage.max(1);
                }
                if carrying_larder && t.outbound {
                    if t.stage < 2 {
                        t.pickup_dist = anchor_dist;
                    }
                    t.stage = t.stage.max(2);
                }
                // Turned for home: half the distance it picked up at, closed
                // while still holding. A wanderer with a full crop does not
                // clear this and should not.
                if carrying_larder && t.stage >= 2 && t.pickup_dist > 4 && anchor_dist * 2 <= t.pickup_dist {
                    t.stage = t.stage.max(3);
                }
                if carrying_larder && at_nest && t.stage >= 2 {
                    t.stage = t.stage.max(4);
                }
                // **A delivery, not a digestion.** Since the crop pays out as
                // it is chewed, a falling cell count away from the nest is the
                // ant EATING its cargo; only one inside the nest band is a
                // drop. Conflating the two is the same shape as counting
                // nest-loitering pickups as commutes.
                if cells < t.last_cells && at_nest && t.stage >= 4 {
                    t.stage = t.stage.max(5);
                }
                if t.stage >= 5 && !at_nest {
                    t.stage = t.stage.max(6);
                }
                if t.stage >= 6 && at_food {
                    t.stage = t.stage.max(7);
                }
                t.last_cells = cells;
                if !carrying_larder {
                    t.laden_since = 0;
                }
                // A trip closes on the return, not the arrival: an ant that
                // reaches the food and dies there has not made a round trip,
                // and counting it as one is how a foraging number turns into
                // an exposure number.
                if at_nest && t.outbound {
                    t.trips += 1;
                    t.outbound = false;
                    // **The homeward leg, measured rather than bracketed.**
                    // §7.38 left it at "436-873 ticks by three data points";
                    // these are the two frames it needs, and they were already
                    // being computed for the trip counter.
                    if t.laden_since > 0 {
                        t.trips_laden += 1;
                        match t.anchor_on_comb_at_pickup {
                            Some(true) => t.loops_anchor_ok += 1,
                            Some(false) => t.loops_anchor_bad += 1,
                            None => {}
                        }
                    } else {
                        t.trips_empty += 1;
                    }
                    if t.left_food > 0 && f >= t.left_food {
                        legs.push(f - t.left_food);
                        if t.laden_since > 0 {
                            laden_legs.push(f - t.left_food);
                        }
                    }
                }
            }
            if carrying_larder {
                carry_ticks += 1;
                // The same +-26 band `creature.rs` judges `AtNest` against.
                if (hx - nest_x).abs() <= 26 {
                    carry_home_ticks += 1;
                }
            }
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
                // `probe` is non-mutating by construction (`creature.rs`: "so
                // probing cannot perturb the run it is measuring"), and it is
                // the only way to read what the animal's own `sense` produced.
                // Sampled on the same 100-frame cadence as the `along` column
                // because it costs a full `sense` per ant.
                let cdef = w.species.get(species_id).creature.clone().expect("ant is a creature");
                let (probe_in, _, _) = pixel_physics::sim::creature::probe(&w, hx, hy, id, &cdef);
                probe_ticks += 1;
                if probe_in[I::AtNest as usize] > 0.0 {
                    atnest_ticks += 1;
                }
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

    // **The channel A homing ramp the ants built for themselves.**
    //
    // The arithmetic says it cannot be readable: unit 4 is an odometer fitted
    // for a **3,000-tick** decay (`ant.ron`, "0.992 -> 0.072 across 3,000
    // ticks"), and a 90-cell trip is 141 ticks at a trail-following `P(move)`
    // of 0.64 -- **4.7% of its range**. The charge falls 0.7% over the whole
    // journey and **0.0005 over the reader's 6-cell sensor offset**, against
    // the `along` of ~0.05 a laden ant needs to lift `P(move)` off its 0.200
    // baseline (`onetrail mode=arith`). Two orders of magnitude short.
    //
    // But that is arithmetic on the *internal state*, and the plane is not the
    // state: deposits pass through `squash` and `EmitA`'s weight of 32, then
    // accumulate from many ants and decay on the plane's own schedule -- which
    // could manufacture a ramp out of laying *order*, exactly as channel B's
    // age ramp does. `CLAUDE.md`: measure the number the consumer computes,
    // never the stored value. So this samples the plane at five points along
    // the route and reports the gradient a real reader would get.
    let a_profile: [u32; 5] = std::array::from_fn(|i| {
        let x = nest_x + (target_x - nest_x) * i as i32 / 4;
        w.pheromone_at(Channel::A, x, surface) as u32
    });


    // --- the decision trace's readout -------------------------------------
    if tracing {
        if tr_n == 0 {
            println!("    TRACE: no ant ever carried larder in this arm, so there is nothing to trace.");
            println!("           That is a statement about the SCENE, not about homing -- pick an arm");
            println!("           where ants reach food (`arms=hand`) before reading a null here.");
        } else {
            let mean_along = tr_along_sum / tr_n as f64;
            println!(
                "    TRACE laden ants: n {tr_n}  mean PheroAAlong {:+.5}  MEAN |PheroAAlong| {:.5}  mean P(move) {:.4}  mean trail term (h0+h1 -> Move) {:+.5}  net cells homeward {tr_dx_home} ({:+.6}/tick)",
                mean_along,
                tr_abs_along_sum / tr_n as f64,
                tr_pmove_sum / tr_n as f64,
                tr_trail_sum / tr_n as f64,
                tr_dx_home as f64 / tr_n as f64
            );
            // **Read `|along|` for "is there a gradient" and the split below for
            // "does it steer".** The signed mean is heading-relative and
            // averages toward zero on a perfectly good ramp -- see the
            // accumulator's note. It is printed only so a future reader can see
            // it is near zero for the harmless reason.
            let row = |label: &str, b: (u64, f64, i64, u64)| {
                if b.0 == 0 {
                    println!("      {label:<22} n 0");
                } else {
                    // **`not resting` is the column to read first.** The mean
                    // P(move) mixes two states the engine keeps strictly apart:
                    // a tick the ant could step on, and a tick where the clamp
                    // has already decided it will not. `P(move)|>0` is the mean
                    // over only the first kind, so the pair separates "the
                    // homing term stopped an ant" from "it sped one up".
                    println!(
                        "      {label:<22} n {:>8}   P(move) {:.4}   not resting {:>6.1}%   P(move)|>0 {:.4}   cells homeward {:>8} ({:+.6}/tick)",
                        b.0,
                        b.1 / b.0 as f64,
                        100.0 * b.3 as f64 / b.0 as f64,
                        if b.3 > 0 { b.1 / b.3 as f64 } else { 0.0 },
                        b.2,
                        b.2 as f64 / b.0 as f64
                    );
                }
            };
            // **What the nose was pointing at, and what it read there.** The
            // homing circuit is fed by `PheroAAlong`, and `sense` samples
            // `(x + dx*so, y + dy*so)` -- so on flat ground six of the eight
            // headings sample six rows into open air or into the ground, where
            // a walking creature has never laid anything. A zero there is not
            // "no trail", it is "no place a trail could be", and the formula
            // turns it into a confident large negative. Read the `usable`
            // column against `surface`: if `air` and `solid` are near zero
            // while `surface` is not, the ant is being told a story about six
            // directions in eight.
            {
                let labels = ["air (nose in open sky)", "solid (nose in ground)", "surface (a trail could be here)"];
                let tot: u64 = tr_by_kind.iter().map(|k| k.0).sum();
                if tot > 0 {
                    // **The tick discriminator's own positive control, printed
                    // rather than trusted.** Rows are written every frame and
                    // the ant decides every `tick_interval`, so this ratio must
                    // come out at exactly that -- 6 for the shipped ant. A
                    // discriminator that silently stopped working would read
                    // 1.0 here and the freeze-run lengths would be six times
                    // too long with nothing to say so.
                    println!(
                        "    TRACE laden TICKS by what the sensor cell is -- the geometry test  [{:.2} frames/tick, must equal tick_interval]:",
                        tr_n as f64 / tot as f64
                    );
                    for (i, lbl) in labels.iter().enumerate() {
                        let k = tr_by_kind[i];
                        if k.0 == 0 {
                            println!("      {lbl:<34} n 0");
                            continue;
                        }
                        println!(
                            "      {lbl:<34} n {:>8} ({:>5.1}% of ticks)   mean along {:+.4}   usable (>= +0.02) {:>5.1}%   frozen {:>5.1}%",
                            k.0,
                            100.0 * k.0 as f64 / tot as f64,
                            k.1 / k.0 as f64,
                            100.0 * k.2 as f64 / k.0 as f64,
                            100.0 * k.3 as f64 / k.0 as f64
                        );
                    }
                    let frozen: u64 = tr_by_kind.iter().map(|k| k.3).sum();
                    if frozen > 0 {
                        println!(
                            "      => of {frozen} FROZEN laden ticks (P(move) exactly 0), {:.0}% had the nose in sky or ground",
                            100.0 * (tr_by_kind[0].3 + tr_by_kind[1].3) as f64 / frozen as f64
                        );
                    }
                }
                // A freeze is a RUN, and the mean cannot show one: a median of
                // 9 ticks with a 157-tick tail is a different animal from one
                // that pauses evenly.
                let mut runs = tr_freeze_runs.clone();
                runs.extend(tr_freeze_open.values().copied().filter(|&r| r > 0));
                if !runs.is_empty() {
                    runs.sort_unstable();
                    println!(
                        "    TRACE freeze runs: n {}   median {} ticks   p90 {}   longest {}",
                        runs.len(),
                        runs[runs.len() / 2],
                        runs[runs.len() * 9 / 10],
                        runs[runs.len() - 1]
                    );
                }
            }
            println!("    TRACE split by the sign of `along` -- the run-and-tumble test:");
            row("facing UP-gradient", tr_up);
            row("facing DOWN-gradient", tr_down);
            row("no readable gradient", tr_flat);
            if tr_up.0 > 0 && tr_down.0 > 0 {
                println!(
                    "      => P(move) up-gradient minus down-gradient: {:+.4}  (the homing drive; ~0 means the circuit is inert)",
                    tr_up.1 / tr_up.0 as f64 - tr_down.1 / tr_down.0 as f64
                );
            }
            // **The histogram is the column that separates "weak" from
            // "absent".** A mean of 0.000 is produced both by a gradient that
            // is never there and by one that is symmetric about zero, and
            // those want opposite work. `CLAUDE.md`: exactly zero is the
            // signature of an exhausted representation; a spread around zero
            // is a real signal the ant cannot act on.
            print!("    TRACE PheroAAlong histogram (-1..+1 in ninths):");
            for (i, c) in tr_along_hist.iter().enumerate() {
                print!(" [{:+.2}]{c}", -1.0 + (i as f32 + 0.5) * 2.0 / 9.0);
            }
            println!();
            // **The gate, and it is upstream of everything above.** If the
            // homing pair never leaves saturation, the trail's magnitude and
            // direction are both beside the point -- units 0/1 cannot respond
            // to `PheroAAlong` at all, however good the ramp is.
            println!(
                "    TRACE homing gate: opens at Carrying >= {gate_threshold:.4}; OPEN on {tr_gate_open} of {tr_n} laden decisions ({:.2}%)",
                100.0 * tr_gate_open as f64 / tr_n as f64
            );
            if tr_up_open.0 > 0 || tr_down_open.0 > 0 {
                println!("    TRACE the same split, GATE OPEN only -- the only rows where the pair can respond at all:");
                row("  facing UP-gradient", tr_up_open);
                row("  facing DOWN-gradient", tr_down_open);
                if tr_up_open.0 > 0 && tr_down_open.0 > 0 {
                    println!(
                        "      => with the gate open, P(move) up minus down: {:+.4}; cells homeward up minus down: {:+.6}/tick",
                        tr_up_open.1 / tr_up_open.0 as f64 - tr_down_open.1 / tr_down_open.0 as f64,
                        tr_up_open.2 as f64 / tr_up_open.0 as f64 - tr_down_open.2 as f64 / tr_down_open.0 as f64
                    );
                    println!("         (positive cells-homeward means the ramp the colony built points at the NEST)");
                }
            }
            if tr_gate_open > 0 {
                println!(
                    "    TRACE of the {tr_gate_open} gate-open decisions, {tr_gate_open_spoil} ({:.1}%) are ants ALSO holding spoil -- see `tr_gate_open_spoil`",
                    100.0 * tr_gate_open_spoil as f64 / tr_gate_open as f64
                );
            }
            // **The paired arm: does an open gate become homeward motion.**
            // Both halves come out of one run, so everything the arms are not
            // about -- seed, colony, geometry, the weather of the bed -- is
            // cancelled. Read the `/tick` figures against each other, not the
            // totals: the shut population is ~50x larger by construction.
            println!("    TRACE split by whether the homing gate is OPEN -- the conversion test:");
            row("gate OPEN", tr_open);
            row("gate SHUT", tr_shut);
            if tr_open.0 > 0 && tr_shut.0 > 0 {
                println!(
                    "      => open minus shut: P(move) {:+.4}   cells homeward {:+.6}/tick  (~0 on the right-hand figure means the open gate is NOT converting into motion)",
                    tr_open.1 / tr_open.0 as f64 - tr_shut.1 / tr_shut.0 as f64,
                    tr_open.2 as f64 / tr_open.0 as f64 - tr_shut.2 as f64 / tr_shut.0 as f64
                );
            }
            // **The response-vs-fill curve.** The design target stated by the
            // owner, 2026-09-18: a full ant should be near-certain to head
            // home and a half-full one about half as likely, so `P(home)` read
            // down this column should climb with fill. **A step is not the
            // target and neither is a flat line** -- today the gate is a
            // threshold at `Carrying >= gate_threshold`, so the prediction is
            // flat everywhere below it, and whatever an unsteered ant does is
            // the floor this has to be read against.
            //
            // `P(home)` and `P(away)` do not sum to 1: a decision where the ant
            // did not step, or stepped vertically, is neither, and that share
            // is the third thing the curve has to show. A bin where both rise
            // together is an ant moving MORE, not an ant moving home.
            println!("    TRACE WHY A LADEN ANT IS NOT PUTTING ITS LOAD DOWN -- its own `Drop` output, by how far the nearest nest material is:");
            println!("      {:>22} {:>12} {:>12} {:>8} {:>10}", "nearest nest material", "laden ticks", "% of laden", "P(drop)", "drops");
            for (i, b) in tr_drop.iter().enumerate() {
                if b.0 == 0 {
                    continue;
                }
                let label = ["ADJACENT (AtNest)", "2 cells", "4 cells", "8 cells", "16 cells", "32 cells", "further / none"][i];
                println!(
                    "      {:>22} {:>12} {:>11.1}% {:>8.4} {:>10}{}",
                    label,
                    b.0,
                    100.0 * b.0 as f64 / tr_drop.iter().map(|x| x.0).sum::<u64>().max(1) as f64,
                    b.1 / b.0 as f64,
                    b.2,
                    if i == 0 { "   <- the only row where Drop can fire" } else { "" }
                );
            }
            {
                let tot: u64 = tr_miss.iter().flatten().sum();
                if tot > 0 {
                    println!("    TRACE ...and when it misses by 2-4 cells, is the miss SIDEWAYS or UPWARD? (rows = |dx|, cols = |dy|)");
                    println!("      {:>8} {:>9} {:>9} {:>9} {:>9} {:>9}", "", "|dy|=0", "1", "2", "3", "4+");
                    for (dx, row) in tr_miss.iter().enumerate() {
                        if row.iter().sum::<u64>() == 0 {
                            continue;
                        }
                        print!("      {:>8}", format!("|dx|={dx}"));
                        for v in row {
                            print!(" {:>8.1}%", 100.0 * *v as f64 / tot as f64);
                        }
                        println!();
                    }
                    let vertical: u64 = tr_miss.iter().enumerate().map(|(dx, r)| if dx <= 1 { r.iter().skip(2).sum::<u64>() } else { 0 }).sum();
                    println!("      MISSED ONLY UPWARD (|dx|<=1, |dy|>=2) -- standing on the doorstep, head too high: {:.1}%", 100.0 * vertical as f64 / tot as f64);
                }
            }
            if tr_smell.0 > 0 {
                let n = tr_smell.0 as f64;
                println!("    TRACE CAN IT SMELL THE LAST FEW CELLS? -- channel A one step TOWARD the comb vs one step AWAY, at a 2-4 cell miss:");
                println!(
                    "      n {}  |  mean toward {:.1}  mean away {:.1}  |  toward is STRONGER on {:.1}% of ticks, equal on {:.1}%",
                    tr_smell.0,
                    tr_smell.1 / n,
                    tr_smell.2 / n,
                    100.0 * tr_smell.3 as f64 / n,
                    100.0 * tr_smell.4 as f64 / n
                );
                let (a, an, b, bn) = tr_smell_along;
                println!(
                    "      and what the ANT reads there -- its own PheroAAlong: pointed AT the comb {:+.4} (n {})  |  pointed away {:+.4} (n {})",
                    if an > 0 { a / an as f64 } else { 0.0 },
                    an,
                    if bn > 0 { b / bn as f64 } else { 0.0 },
                    bn
                );
                println!("      A cue only works if BOTH lines are good: the plane has to carry it AND the nose has to see it.");
            }
            if tr_cmp.iter().any(|c| c.1 + c.3 > 0) {
                println!("    TRACE WOULD A TWO-FORWARD-SAMPLE COMPARATOR READ THE PLANE? -- `(far - near) / (far + near + guard)`, neither term the ant's own cell:");
                println!("      {:>10} {:>12} {:>14} {:>14} {:>10}", "near/far", "blind (both 0)", "along POINTED HOME", "along AWAY", "separation");
                for (i, (n, f)) in CMP_PAIRS.iter().enumerate() {
                    let c = tr_cmp[i];
                    let tot = c.0 + c.1 + c.3;
                    if tot == 0 {
                        continue;
                    }
                    let home = if c.1 > 0 { c.2 / c.1 as f64 } else { 0.0 };
                    let away = if c.3 > 0 { c.4 / c.3 as f64 } else { 0.0 };
                    println!(
                        "      {:>10} {:>11.1}% {:>14.4} {:>14.4} {:>+10.4}{}",
                        format!("{n} / {f}"),
                        100.0 * c.0 as f64 / tot as f64,
                        home,
                        away,
                        home - away,
                        if *f == 12 { "   <- Z29's literal suggestion" } else { "" }
                    );
                }
                println!("      SEPARATION IS THE COLUMN. The shipped `(ahead - here)` reads -0.20 home against -0.24 away:");
                println!("      a gap of 0.04 and NEGATIVE in both. A comparator earns its place by making that gap real");
                println!("      AND by not being blind -- a pair that reads 0.0 most of the time is not a sensor.");
            }
            if tr_bsniff.iter().any(|b| b.0 > 0) {
                println!("    TRACE WHY AN EMPTY ANT CANNOT SMELL THE FOOD TRAIL (§Z32) -- channel B under the animal against channel B at the nose:");
                println!("      {:>10} {:>10} {:>14} {:>14} {:>26} {:>12}", "heading", "ticks", "under the ant", "at the nose", "nose BLIND, ant on trail", "mean |dy|");
                for (i, nm) in ["cardinal", "diagonal"].iter().enumerate() {
                    let b = tr_bsniff[i];
                    if b.0 == 0 {
                        continue;
                    }
                    println!(
                        "      {:>10} {:>10} {:>14.1} {:>14.1} {:>25.1}% {:>12.2}",
                        nm,
                        b.0,
                        b.1 / b.0 as f64,
                        b.2 / b.0 as f64,
                        100.0 * b.3 as f64 / b.0 as f64,
                        b.4 as f64 / b.0 as f64
                    );
                }
                println!("      The hand-laid trail is a FIVE-ROW band (`lay`: surface-3 ..= surface+1). With projection off --");
                println!("      which is shipped -- a diagonal heading samples `so` cells along BOTH axes, so the nose is six rows");
                println!("      above or below the animal and outside that band by construction. `mean |dy|` is the test.");
            }
            if tr_temporal.iter().flatten().flatten().any(|b| b.0 > 0) {
                println!("    TRACE DOES SMELLING OVER TIME CARRY THE DIRECTION? -- `(live - lagged) / (live + lagged + guard)` on the ant's OWN cell,");
                println!("      exponential lag {W_REC}, read off the plane with no wiring. Bucketed by distance from home, because heading and");
                println!("      distance are correlated: an ant pointed away has usually just left the nest and stands in the bright end of the ramp.");
                println!(
                    "      {:>14} {:>8} {:>9} {:>13} {:>9} {:>13} {:>12} {:>12}",
                    "distance", "state", "ticks", "along HOME", "ticks", "along AWAY", "separation", "mean level"
                );
                let mut lo = 0;
                for (bi, &hi) in DIST_BANDS.iter().enumerate() {
                    let label = if hi == i32::MAX { format!("{lo}+ cells") } else { format!("{lo}-{hi} cells") };
                    for (mi, mname) in [(1usize, "moved"), (0usize, "frozen")] {
                        let h = tr_temporal[bi][mi][1];
                        let a = tr_temporal[bi][mi][0];
                        if h.0 == 0 || a.0 == 0 {
                            continue;
                        }
                        let (hm, am) = (h.1 / h.0 as f64, a.1 / a.0 as f64);
                        let lvl = (h.3 + a.3) / (h.0 + a.0) as f64;
                        println!(
                            "      {:>14} {:>8} {:>9} {:>+13.4} {:>9} {:>+13.4} {:>+12.4} {:>12.1}",
                            label, mname, h.0, hm, a.0, am, hm - am, lvl
                        );
                    }
                    lo = hi;
                }
                println!("      SEPARATION IS THE COLUMN, and `moved` is the row that matters -- a frozen ant is watching its OWN");
                println!("      deposit climb on a cell it never left, which reads as up-gradient while it goes nowhere.");
                println!("      Compare against the spatial table above: that one separates +0.0617 at 6/12 on the shipped world.");
                println!("      AND READ THE LEVEL COLUMN. Channel A is a ramp, so the level is a function of distance -- and §7.48's");
                println!("      fit cancels the wiring's level term at ONE value of it (`w_in` 0.12). If the level moves across these");
                println!("      bands, no constant `w_in` cancels it everywhere and the wiring is right at one distance from home.");
            }
            println!("      READ THE TOP ROW'S SHARE. `(AtNest, Drop, 1.0889)` against `(Bias, Drop, -0.2)` puts P(drop) at");
            println!("      EXACTLY 0 anywhere below adjacency, at any crop fill. So a small top row means the ants never");
            println!("      REACH the comb, and a large one with few drops would mean they reach it and decline.");
            println!("    TRACE does pointing HOME give a POSITIVE reading? -- the three-way fix test, see `tr_align`:");
            println!("      {:>12} {:>10} {:>11} {:>10} {:>9}", "heading vs home", "n", "mean along", "% positive", "P(move)");
            for (i, b) in tr_align.iter().enumerate() {
                if b.0 == 0 {
                    continue;
                }
                let n = b.0 as f64;
                println!(
                    "      {:>12} {:>10} {:>11.4} {:>9.1}% {:>9.4}{}",
                    format!("{:+.1}..{:+.1}", i as f32 / 2.5 - 1.0, (i + 1) as f32 / 2.5 - 1.0),
                    b.0,
                    b.1 / n,
                    100.0 * b.2 as f64 / n,
                    b.3 / n,
                    if i == 4 { "   <- POINTED AT HOME: this row is the answer" } else { "" }
                );
            }
            println!("    TRACE response vs crop fill -- P(home) should CLIMB with fill; a step or a flat line is the defect:");
            println!("      {:>9} {:>10} {:>9} {:>9} {:>9} {:>13}", "fill", "n", "P(move)", "P(home)", "P(away)", "cells/tick");
            for (i, b) in tr_carry_hist.iter().enumerate() {
                if b.n == 0 {
                    continue;
                }
                let n = b.n as f64;
                println!(
                    "      {:>9} {:>10} {:>9.4} {:>9.4} {:>9.4} {:>+13.6}{}",
                    format!("{:.1}-{:.1}", i as f32 / 10.0, (i + 1) as f32 / 10.0),
                    b.n,
                    b.p_move / n,
                    b.home as f64 / n,
                    b.away as f64 / n,
                    b.dx as f64 / n,
                    if (i as f32 / 10.0) < gate_threshold && ((i + 1) as f32 / 10.0) > gate_threshold { "   <- the gate threshold falls inside this bin" } else { "" }
                );
            }
            println!("    TRACE Move pre-squash terms, mean over the decisions each one was live for:");
            let mut ranked: Vec<(f64, &String, u64)> = tr_terms.iter().map(|(n, (s, c))| (s / *c as f64, n, *c)).collect();
            ranked.sort_by(|a, b| b.0.abs().total_cmp(&a.0.abs()));
            for (v, n, c) in &ranked {
                let note = match n.as_str() {
                    "h0" | "h1" => "   <- THE TRAIL. `PheroAAlong` reaches `Move` here and nowhere else.",
                    _ => "",
                };
                // `n/` is printed because the term set is per-genome: a wire
                // under `W_EPS` in one individual and over it in another is
                // live for a subset of the decisions, and a mean over the
                // wrong denominator would understate it.
                println!("      {n:<16} {v:+.5}   (live in {c} of {tr_n}){note}");
            }
        }
        if !focal_rows.is_empty() {
            let path = format!("/tmp/trailfollow-focal-seed{seed}-gap{gap}.csv");
            let mut out = String::from(
                "id,stage,frame,x,y,dx_home,anchor_x,since_nest,PheroARise,HomeAligned,PheroAAlong,PheroAFront,Carrying,CarryingFood,crop_cells,spoil,heading,Energy,Crowding,AtNest,FoodAdjacent,Stillness,h0,h1,PheroBAlong,PheroBFront,h2,h3,p_move,p_tumble,trail_term,move_presquash,drop_urge,MoistureGrad,SurfaceCurvature,tick,is_tick,here_a,ahead_a,sensor_kind\n",
            );
            out.push_str(&focal_rows.join("\n"));
            out.push('\n');
            match std::fs::write(&path, out) {
                Ok(()) => println!("    TRACE focal ant: {} decisions written to {path}", focal_rows.len()),
                Err(e) => println!("    TRACE focal ant: could not write {path}: {e}"),
            }
        }
    }

    // `dietdump` names every material the colony actually booked intake
    // against, which is the only thing that can say *what* an unexpected
    // `other J` is. A total is a number; this is an answer.
    if flag("dietdump") {
        for books in w.all_colony_books() {
            for (m, j) in books.diet() {
                println!("    diet: {:<14} {:>12.0} J", w.materials.get(m).name, j);
            }
        }
    }
    let span = (target_x - nest_x).max(1) as f32;
    let mut reach = [0usize; 5];
    for t in tracks.values() {
        let frac = t.far as f32 / span;
        let b = ((frac * 4.0).floor().max(0.0) as usize).min(4);
        reach[b] += 1;
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
        supply_j: larder_placed as f64 * per_cell_j,
        eaten_j: diet_by_material(&w, larder).0,
        ate_other_j: diet_by_material(&w, larder).1,
        founded: if founded.0 == i32::MAX { (0, 0) } else { founded },
        nest_x,
        target_x,
        ticks: st.ticks,
        nest_cells,
        nest_span: (nest_lo, nest_hi),
        founders_on_nest: on_nest,
        atnest_ticks,
        probe_ticks,
        a_peak_amt,
        a_peak_cells,
        a_polarity: if a_pol_n == 0 { 0.0 } else { (a_pol_sum / a_pol_n as f64) as f32 },
        a_polarity_both: if a_pol_both_n == 0 { 0.0 } else { (a_pol_both_sum / a_pol_both_n as f64) as f32 },
        a_edge_cells: if a_edge_n == 0 { 0.0 } else { a_edge_sum as f32 / a_edge_n as f32 },
        a_polarity_span: if a_pol_span_n == 0 { 0.0 } else { (a_pol_span_sum / a_pol_span_n as f64) as f32 },
        a_profile,
        b_profile: std::array::from_fn(|i| if b_prof_n == 0 { 0 } else { (b_prof_sum[i] / b_prof_n as f64) as u32 }),
        kin_swaps: st.kin_swaps,
        blocked: st.moves_blocked,
        tumbles: st.tumbles,
        tumbles_homeward: st.tumbles_homeward,
        drops: st.drops,
        digest_appetite_held: st.digest_appetite_held,
        digested_face: st.digested_face,
        first_arrival,
        all_dead_frame,
        carry_toward_nest,
        occupancy,
        births: st.births,
        deaths: st.deaths,
        starved: w.deaths_by_cause[pixel_physics::sim::organism::DeathCause::Starved.index()],
        visitors: tracks.values().filter(|t| t.visited).count(),
        ants_seen: tracks.len(),
        round_trips: tracks.values().map(|t| t.trips as u64).sum(),
        read_ok,
        read_away,
        read_n,
        lit: live_cells_lit,
        lit_n: live_cells_n,
        reached: tracks.values().filter(|t| t.visited).count(),
        returned: tracks.values().filter(|t| t.trips > 0).count(),
        funnel: std::array::from_fn(|n| tracks.values().filter(|t| t.stage as usize >= n).count()),
        loopers: tracks.values().filter(|t| t.trips_laden >= 1).count(),
        loops_anchor_ok: tracks.values().map(|t| u64::from(t.loops_anchor_ok)).sum(),
        loops_anchor_bad: tracks.values().map(|t| u64::from(t.loops_anchor_bad)).sum(),
        pickups_anchor_ok: tracks.values().map(|t| u64::from(t.picked_up_anchor_ok)).sum(),
        pickups_anchor_bad: tracks.values().map(|t| u64::from(t.picked_up_anchor_bad)).sum(),
        loopers_on_comb: tracks.values().filter(|t| t.born_on_nest && t.trips_laden >= 1).count(),
        loopers_off_comb: tracks.values().filter(|t| !t.born_on_nest && t.trips_laden >= 1).count(),
        reached_on_comb: tracks.values().filter(|t| t.born_on_nest && t.visited).count(),
        reached_off_comb: tracks.values().filter(|t| !t.born_on_nest && t.visited).count(),
        repeat_loopers: tracks.values().filter(|t| t.trips_laden >= 2).count(),
        max_loops: tracks.values().map(|t| t.trips_laden).max().unwrap_or(0),
        trips_laden: tracks.values().map(|t| u64::from(t.trips_laden)).sum(),
        trips_empty: tracks.values().map(|t| u64::from(t.trips_empty)).sum(),
        leg_n: legs.len(),
        leg_med: order_stat(&mut legs.clone(), 0.5),
        leg_p90: order_stat(&mut legs.clone(), 0.9),
        laden_leg_n: laden_legs.len(),
        laden_leg_med: order_stat(&mut laden_legs.clone(), 0.5),
        laden_leg_p90: order_stat(&mut laden_legs.clone(), 0.9),
        legs_raw: legs.clone(),
        laden_raw: laden_legs.clone(),
        trips_on_nest: tracks.values().filter(|t| t.born_on_nest).map(|t| t.trips as u64).sum(),
        trips_off_nest: tracks.values().filter(|t| !t.born_on_nest).map(|t| t.trips as u64).sum(),
        ants_on_nest: tracks.values().filter(|t| t.born_on_nest).count(),
        reach,
        carry_ticks,
        carry_home_ticks,
        live_cells,
        peak_cells,
        natural_along: if nat_n == 0 { 0.0 } else { (nat_sum / nat_n as f64) as f32 },
        laden_nest_share: if laden_ticks == 0 { 0.0 } else { 100.0 * laden_nest_ticks as f32 / laden_ticks as f32 },
        laden_ticks,
        laden_by_third,
        spoil_ticks,
    }
}

fn main() {
    let mode = arg_str("mode").unwrap_or_else(|| "colony".into());
    let gate = gate_by_name(&arg_str("gate").unwrap_or_else(|| "saturated".into()));
    let frames: u64 = arg("frames").unwrap_or(3000);
    let seeds: u64 = arg("seeds").unwrap_or(3);
    // **Accepts the name it prints.** The header below echoes `seed0=`, the
    // parser read `seed=`, and nothing in between complained -- so
    // `seed0=13 seeds=36`, meant as an independent replication on seeds 13-48,
    // silently re-ran seeds 1-36 and returned the very seed it was built to
    // avoid. Third instance of `CLAUDE.md`'s "an unknown argument is silently
    // ignored" in one session (`gaps=`, `seed0=`), and the first two were
    // caught only because the output looked wrong. A knob whose echoed name is
    // not its accepted name is worse than an unknown one, because the header
    // reads as confirmation.
    let seed0: u64 = arg("seed0").or_else(|| arg("seed")).unwrap_or(1);
    let ants: i32 = arg("ants").unwrap_or(20);
    let relay: u64 = arg("relay").unwrap_or(60);
    let near: i32 = arg("near").unwrap_or(10);
    // Cells of corpse at the target. 0 keeps this file's original "nothing but
    // the trail pulls an ant there" design; >0 is the readout's positive
    // control and the round-trip arm. See `run`.
    let food: i32 = arg("food").unwrap_or(0);
    // **`CreatureDef::home_bias` for the measurement arm** -- how hard a laden
    // ant's tumble is aimed at the nest. `-1` (the default) leaves the species
    // file alone, so an unpassed run is the shipped animal and no RNG draw
    // moves; `0.0` asserts the shipped value explicitly and is refused as a
    // no-op the way the genome riders are, because a rider that silently
    // matches the file is a knob nobody can tell is disconnected.
    // Frame at which hand-laying stops. 0 (the default) keeps the trail
    // standing for the whole run, which is the pull question. Any positive
    // value turns this into the loop question: seed it, then let go.
    let stop: u64 = arg("stop").unwrap_or(0);
    // Cells between the nest and the food. The box widens to fit it.
    let gap: i32 = arg("gap").unwrap_or(90);
    // Frames between food replenishments at the target. 0 places it once,
    // which the arithmetic above shows is a guaranteed starvation.
    let refill: u64 = arg("refill").unwrap_or(0);
    // **`onlyfood` defaults ON, and that changes what every earlier row of this
    // harness meant.** See `Diet`: a dead ant's corpse is worth four placed
    // larder cells, so a colony measured without this was being fed mostly by
    // its own dead and no intake figure from it was attributable. To reproduce
    // a row recorded before 2026-09-16, pass `onlyfood=off larder=corpse`.
    let onlyfood = match arg_str("onlyfood").as_deref() {
        None | Some("on") => true,
        Some("off") => false,
        Some(other) => panic!("onlyfood={other:?}; expected on or off"),
    };
    // **`fruit`, and the choice is measured rather than picked.** Two
    // properties are needed and only three materials have both.
    //
    // *Nothing in the world may produce it*, or the larder is not isolable --
    // `Diet::isolate`'s assert. With `founders: 0` there are no plants here, so
    // any plant food qualifies and `corpse` does not.
    //
    // *It must not rot*, and this is the one that had to be caught by running
    // it. `windfall` was the first default and it sets `decays_into: "soil"`;
    // `decay.rs` checks every 200 ticks at a 0.05 chance once damp, so over
    // 8,000 frames about 87% of a placement is gone on its own. Measured on a
    // two-seed control: **240 of 300 cells vanished while the colony ate 5.**
    // A larder that rots fifty times faster than the colony eats it cannot
    // measure foraging at any distance.
    //
    // `fruit`, `seed` and `moss` are the three edible materials in the tree
    // with no `decays_into`. `fruit` is the richest at 960 face -- 240 J at the
    // shipped neutral gut, the same per-cell value `windfall` had, so a food=
    // setting means what it meant before.
    // Leaked rather than whitelisted: a match arm per material is a list that
    // goes stale the moment somebody adds a food, and `Diet::isolate` already
    // panics by name for anything not in the table.
    let larder: &'static str = arg_str("larder").map_or("fruit", |v| &*Box::leak(v.into_boxed_str()));
    let diet = Diet { larder, only: onlyfood };
    // Echo it, because a knob nobody can see the value of is a knob nobody can
    // tell is disconnected -- `CLAUDE.md`, after a 3.5-hour study came back as
    // three populations wearing 24 logs.
    println!("  diet: larder={larder} onlyfood={}", if onlyfood { "on" } else { "off" });
    // Echoed because a knob whose value is not printed is a knob nobody can tell
    // is disconnected -- `CLAUDE.md`, after a 3.5-hour study came back as three
    // populations wearing 24 logs.
    println!(
        "  odometer A: recur={} emita={} biasa={}   odometer B: charb={} recurb={} emitb={} biasb={} carryb={}",
        arg::<f32>("recur").map_or("shipped".to_string(), |v| format!("{v}")),
        arg::<f32>("emita").map_or("shipped".to_string(), |v| format!("{v}")),
        arg::<f32>("biasa").map_or("shipped".to_string(), |v| format!("{v}")),
        arg::<f32>("charb").map_or("shipped".to_string(), |v| format!("{v}")),
        arg::<f32>("recurb").map_or("shipped".to_string(), |v| format!("{v}")),
        arg::<f32>("emitb").map_or("shipped".to_string(), |v| format!("{v}")),
        arg::<f32>("biasb").map_or("shipped".to_string(), |v| format!("{v}")),
        arg::<f32>("carryb").map_or("shipped".to_string(), |v| format!("{v}"))
    );

    if flag("spec") {
        println!("{}", gate.spec());
        return;
    }

    // The harness names its own parameters, so a log that does not name a
    // knob was written by a binary that never had one — `CLAUDE.md`'s
    // stale-harness gotcha, which cost a 3.5-hour study.
    // **`refill` is echoed because leaving it out cost a whole pass.** It
    // defaults to 0 -- a one-shot larder -- and at 0 no colony in this scene
    // forages, so the question §7.15 asks cannot be posed. A run with it set
    // and a run without it printed **identical headers**, and the two were
    // chased through a determinism check, a six-value thread scan, a
    // contention test and a clean worktree rebuild before the scene was
    // suspected. `CLAUDE.md`: a knob nobody can see the value of is a knob
    // nobody can tell is disconnected -- and the same is true of one nobody
    // can tell is *connected*.
    // **`stop` is echoed for the reason `refill` is, and it is the same fault
    // caught twice.** `stop` decides whether the hand-laid trail stands for the
    // whole run (`0`, the default -- the *pull* question) or is seeded and let
    // go (`>0` -- the *loop* question), which are two different experiments on
    // the same arm. Unechoed, they printed **identical parameter lines**: a
    // 6-seed `hand` trace at `stop=6000` reports n 570,660 laden decisions and
    // a 1.84% open gate where the same command at the default reports 639,100
    // and 1.25%, and nothing in the header said why. Found 2026-09-18 by an
    // archived log failing to reproduce against a binary that was correct.
    println!("trailfollow: mode={mode} gate={} frames={frames} seeds={seeds} seed0={seed0} ants={ants} relay={relay} near={near} food={food} refill={refill} stop={stop} homebias={} cropcap={} hungergate={} arho={} brho={} adiffuse={} arise={}/{} tumble={} persist={} tumblegrad={} homewire={}", gate.name, arg::<f32>("homebias").map_or("shipped".to_string(), |v| format!("{v}")), arg::<f32>("cropcap").map_or("shipped".to_string(), |v| format!("{v}")), arg::<f32>("hungergate").map_or("shipped".to_string(), |v| format!("{v}")), arg::<f32>("arho").map_or("shipped".to_string(), |v| format!("{v}")), arg::<f32>("brho").map_or("shipped".to_string(), |v| format!("{v}")), arg::<f32>("adiffuse").map_or("shipped".to_string(), |v| format!("{v}")), arg::<f32>("arise").map_or("off".to_string(), |v| format!("{v}")), arg::<f32>("arisetumble").map_or("off".to_string(), |v| format!("{v}")), arg::<f32>("tumble").map_or("shipped".to_string(), |v| format!("{v}")), arg::<f32>("persist").map_or("shipped".to_string(), |v| format!("{v}")), arg::<f32>("tumblegrad").map_or("shipped".to_string(), |v| format!("{v}")), arg::<f32>("homewire").map_or("shipped".to_string(), |v| format!("{v}")));
    println!("  gate {}: off {:+.1}  on {:+.1}  along ±{:.1}", gate.name, gate.off, gate.on, gate.along);
    println!("  {LANDED_NOTE}\n");

    let spec = LabBox { width: 256, height: 192, ground_y: 96, soil_depth: 48, founders: 0, colonies: 0, seed: seed0, ..LabBox::default() };
    let w = spec.build();
    let base = w.species.get(w.species.id_of("ant").expect("the ant species is compiled in")).genome.clone();

    if mode == "feedgate" {
        feed_gate(&base, &[0.0, 0.5, 1.0, 1.5, 2.0, 3.0]);
        return;
    }

    if mode == "arith" {
        println!("P(move) for a fed ant (Energy 1.0), by gate and by along-gradient reading:\n");
        arithmetic(&base);
        return;
    }

    if mode == "gap" {
        // **How far can food be before the colony cannot reach it, and how
        // near before a trail stops mattering?** Both ends are measured and
        // neither is where the work has been happening:
        //
        //   ~0 cells   `played_bed` after ~frame 30,000 -- the deliberate bare
        //              band 210..310 fills in completely (`bare/band` 129/129
        //              -> 0/129 by 30,000 frames, 18 plants -> 259), so food
        //              ends up underfoot and a trail has no distance to serve.
        //   90 cells   this box -- the colony survives (15-30 of 40-80 alive).
        //   363 cells  `far_larder` -- every one of 52 ants starves by frame
        //              20,000; `eats` 87 in 30,000 frames. Never arrives.
        //
        // The band where a trail is BOTH necessary and survivable lies
        // between, and this is the sweep that finds it. Each gap is run twice
        // on the same seed: with a hand-laid trail and without.
        //
        // Read it as three columns, not one. `alive` says whether the colony
        // can live there at all; `ate J off` says whether it reaches the food
        // *unaided*, so a gap where that is already healthy cannot show a
        // trail doing anything; `ate J on` against it says whether the trail
        // buys reach. The testable band is where the colony lives, the
        // unaided arm is near zero, and the trail arm is not.
        assert!(food > 0, "mode=gap needs food= at the target, or there is nothing to reach");
        // **Three arms, not two, and the third is the one that was missing.**
        // Owner's objection, 2026-09-16: withholding the hand-laid ramp does
        // not give a no-trail control, because the ants still carry
        // `(Carrying, EmitB, 2.5)` and lay their own. So a null between the
        // first two arms reads equally well as "the trail does nothing" and as
        // "the colony's own trail already did it", and those want opposite
        // conclusions.
        //
        //   hand   the ramp is laid for them until `stop`, then released
        //   hmute  the ramp is laid, but `EmitB` is zeroed: the DECAY BASELINE
        //   self   no ramp; the ants lay and read their own channel B
        //   mute   no ramp, and `EmitB` is zeroed -- no channel B can exist
        //
        // **`hmute` exists because `route pk` could not tell a maintained trail
        // from a dying one.** That column read the full route length -- 88 at
        // gap 90, 146 at 150, 213 at 220 -- in **12 of 12 seeds at every gap**,
        // including gap 220 where eleven colonies of twelve die and only two
        // seeds ever land an ant on the food. A full-route trail in a seed
        // where nobody walked the route is not the ants' trail, and a value
        // that exact in every seed is `CLAUDE.md`'s tidiness signature.
        //
        // The cause was the sampling margin, and it was mis-derived rather than
        // merely tight. `stop + 1500` was set against the measured ~1,476-frame
        // lifetime of a cell laid once at `DEPOSIT` -- but `lay` re-deposits
        // every `relay` frames for the whole of `stop`, about a hundred times,
        // so those cells saturate far above a single deposit and take far
        // longer to fall. The window was sized against the wrong constant.
        //
        // `hmute` fixes it by subtraction rather than by guessing a longer
        // window: it lays the identical ramp and forbids the ants to lay, so
        // every route cell it still shows is pure decay of OUR trail. Whatever
        // `hand` holds above that baseline is the colony's own.
        //
        // `self` vs `mute` is the question this whole investigation is named
        // for: **do the ants' own trails do anything?** `hand` vs `self` only
        // says whether a hand-laid ramp beats what they bootstrap.
        println!(
            "{:>5} {:>4} {:>5} {:>9} {:>9} {:>8} {:>8} {:>8} {:>8} {:>9} {:>6} {:>18}",
            "gap", "seed", "arm", "alive", "ate J", "arrive@", "dead@", "carry", "@nest", "visitors", "trips", "reach 0-25-50-75-100"
        );
        println!("{:->5} {:->4} {:->5} {:->9} {:->9} {:->8} {:->8} {:->8} {:->8} {:->9} {:->6} {:->18}", "", "", "", "", "", "", "", "", "", "", "", "");
        // **The isolation check survives losing its column as an assertion,
        // not as an assumption.** It read 0 in all 24 rows of the first sweep,
        // and a column that is always 0 is worth more as something that stops
        // the run than as something to scan past. `supply J` is likewise
        // summarised below rather than dropped -- losing the provisioning
        // denominator is exactly how §7.5 and §7.6 read a starving colony as a
        // deaf one.
        let (mut supply_lo, mut supply_hi) = (f64::INFINITY, 0.0f64);
        // **A knob, because it was silently ignored as one.** `gaps=90` on the
        // command line did nothing and the run swept the hardcoded four --
        // `CLAUDE.md`'s "an unknown argument is silently ignored", which cost a
        // 3.5-hour study once already.
        let gaps: Vec<i32> = arg_str("gaps")
            .map(|v| v.split(',').map(|t| t.trim().parse().expect("gaps= takes a comma-separated list of integers")).collect())
            // **90, 140, 200 on every run — owner's standing instruction,
            // 2026-09-18, and it is a statement about what the experiment IS.**
            //
            // 90 is the only distance a colony currently survives (§7.30: at a
            // real founder distance of 50, 2 of 3 seeds live; at 100 and 160,
            // **0 of 3** and the food is sometimes never reached at all). The
            // temptation is to read that as "the bed only works at 90, so test
            // there" — which is what an earlier draft of §7.30 concluded, and
            // it is backwards. **The colony dies at 140 and 200 BECAUSE
            // recruitment does not work.** A lone scout cannot keep a colony
            // alive a hundred cells out; a recruited column can. So survival at
            // 140 and 200 is the SUCCESS SIGNAL of the thing being built, not a
            // precondition for measuring it, and a run that omits them cannot
            // see the result it is looking for.
            //
            // Structural rather than a discipline (`CLAUDE.md`: make it a
            // command, not a habit) — the old default started at 90 and ran
            // out to 300, so the three that matter were never one list.
            .unwrap_or_else(|| vec![90, 140, 200]);
        // **`arms=homeA` runs one arm instead of five**, which is what makes a
        // 36-seed replication affordable: an effect that shows up in 1 seed of
        // 12 needs more seeds, not more arms, and paying for four irrelevant
        // arms is what stops anyone running them.
        let want_arms: Option<Vec<String>> = arg_str("arms").map(|v| v.split(',').map(|t| t.trim().to_string()).collect());
        for g in gaps {
            for s in seed0..seed0 + seeds {
                for (name, trail, mute, paint) in [
                    ("hand", true, false, PaintA::None),
                    ("hmute", true, true, PaintA::None),
                    ("self", false, false, PaintA::None),
                    ("mute", false, true, PaintA::None),
                    // **The homing arm.** No food trail is laid; a channel A
                    // ramp peaking at the nest is, and the ants lay their own
                    // channel B as usual. If the owner's chain is the whole
                    // story, this is the arm where a colony finally builds a
                    // trail for itself.
                    ("homeA", false, false, PaintA::Ramp),
                    // **The metric's two NEGATIVE controls, and they are not
                    // treatment arms** -- no ramp is painted in either, the
                    // ants' own `EmitA` is silenced, and the only question is
                    // what the polarity statistic says about a field that has
                    // no polarity. Never pool them with anything; see
                    // `lay_flat` for the two numbers they predict.
                    ("flatN", false, true, PaintA::FlatNest),
                    ("flatF", false, true, PaintA::FlatFood),
                ] {
                    if want_arms.as_ref().is_some_and(|w| !w.iter().any(|x| x == name)) {
                        continue;
                    }
                    let a = run(s, trail, gate, frames, ants, relay, near, food, stop, g, refill, diet, mute, paint);
                    if diet.only {
                        assert_eq!(a.ate_other_j, 0.0, "gap {g} seed {s} arm {name}: {} J eaten off something that is not the larder, so onlyfood did not hold and no column in this row is attributable", a.ate_other_j);
                    }
                    supply_lo = supply_lo.min(a.supply_j);
                    supply_hi = supply_hi.max(a.supply_j);
                    println!(
                        "{g:>5} {s:>4} {name:>5} {:>4}/{:<4} {:>9.0} {:>8} {:>8} {:>8} {:>8} {:>4}/{:<4} {:>6} {:>18}",
                        a.alive_end,
                        a.alive_min,
                        a.eaten_j,
                        a.first_arrival,
                        a.all_dead_frame,
                        a.carry_ticks,
                        a.carry_home_ticks,
                        // Distinct ants that reached the food, over distinct
                        // ants that ever lived. `near_ticks` cannot separate
                        // fifty visitors from one resident; this can.
                        a.visitors,
                        a.ants_seen,
                        a.round_trips,
                        format!("{} {} {} {} {}", a.reach[0], a.reach[1], a.reach[2], a.reach[3], a.reach[4])
                    );
                    println!(
                        "{:>16}A READ  ant-readable homeward along >= 0.02 on {:>5.1}% of route-cell samples | FOODWARD {:>5.1}% | plane lit on {:>5.1}% | peak amt {:>6} cells {:>3}",
                        "", 100.0 * a.read_ok as f64 / a.read_n.max(1) as f64,
                        100.0 * a.read_away as f64 / a.read_n.max(1) as f64,
                        100.0 * a.lit as f64 / a.lit_n.max(1) as f64, a.a_peak_amt, a.a_peak_cells
                    );
                    // **THE FUNNEL -- counts AND percentages, per ant, at
                    // the ant's high-water mark.** Owner's instruction,
                    // 2026-09-20. Two percentage columns on purpose: `of
                    // prev` is where ants are LOST, and `of all` is whether
                    // the colony is doing anything at all. A change that
                    // doubles a stage looks like a win in the first column
                    // and can still be 4 ants out of 40 in the second, which
                    // is the reading that keeps getting missed.
                    println!("{:>16}THE LOOP, ANT BY ANT:", "");
                    for (n, name) in FUNNEL.iter().enumerate() {
                        let c = a.funnel[n];
                        let prev = if n == 0 { c } else { a.funnel[n - 1] };
                        let all = a.funnel[0];
                        println!(
                            "{:>16}  {n}. {name:<38} {c:>5}   {:>6.1}% of prev   {:>6.1}% of all",
                            "",
                            if prev == 0 { 0.0 } else { 100.0 * c as f64 / prev as f64 },
                            if all == 0 { 0.0 } else { 100.0 * c as f64 / all as f64 },
                        );
                    }
                    println!(
                        "{:>16}RETURN LEDGER reached food {:>4} of {:>4} ants | came back {:>4} | trips laden {:>4} empty {:>4} | LOOPERS {:>4} of {:>4} ants ({:>4.1}% of those that reached food), repeat {:>4}, most loops by one ant {:>3}",
                        "", a.reached, a.ants_seen, a.returned, a.trips_laden, a.trips_empty,
                        a.loopers, a.ants_seen, 100.0 * a.loopers as f64 / a.reached.max(1) as f64,
                        a.repeat_loopers, a.max_loops
                    );
                    // **§7.37's within-run control.** Printed beside the total
                    // rather than instead of it: the total is what moved, and
                    // this is the split that says whether the anchor is why.
                    // **The causal split.** Birth site is a proxy that
                    // `forage_anchor`'s re-set on nest contact invalidates; this
                    // is the reading taken at the moment the return leg's target
                    // is chosen. Printed as LOOPS per PICKUP, so an ant that
                    // picks up twice contributes twice on both sides.
                    println!(
                        "{:>16}BY ANCHOR AT PICKUP  anchor ON nest material: {:>4} loops / {:>4} pickups ({:>5.1}%) | anchor OFF it: {:>4} / {:>4} ({:>5.1}%)",
                        "",
                        a.loops_anchor_ok, a.pickups_anchor_ok,
                        100.0 * a.loops_anchor_ok as f64 / a.pickups_anchor_ok.max(1) as f64,
                        a.loops_anchor_bad, a.pickups_anchor_bad,
                        100.0 * a.loops_anchor_bad as f64 / a.pickups_anchor_bad.max(1) as f64
                    );
                    println!(
                        "{:>16}BY BIRTH SITE  born ON comb: {:>3} of {:>3} that reached food looped ({:>5.1}%) | born OFF comb: {:>3} of {:>3} ({:>5.1}%)",
                        "",
                        a.loopers_on_comb, a.reached_on_comb,
                        100.0 * a.loopers_on_comb as f64 / a.reached_on_comb.max(1) as f64,
                        a.loopers_off_comb, a.reached_off_comb,
                        100.0 * a.loopers_off_comb as f64 / a.reached_off_comb.max(1) as f64
                    );
                    if !a.legs_raw.is_empty() {
                        let f = |v: &Vec<u64>| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");
                        println!("{:>16}LEGS legs[{}] laden[{}]", "", f(&a.legs_raw), f(&a.laden_raw));
                    }
                    // Second line, because these are the shape readouts and a
                    // shape does not fit in a column. `occupancy` is nest-end
                    // first; `carry->nest` is signed cells, positive homeward.
                    let occ: Vec<String> = a.occupancy.iter().map(|v| format!("{}", v / 1000)).collect();
                    println!(
                        "{:>16}founded x {:>4}..{:<4} (nest cursor {} MATERIAL x {}..{} {} of {} founders born on it; food {}; nearest founder {} cells out, farthest {}, nominal gap {})  occupancy/1k [{}]  carry->nest {:>7}  born {:>4} died {:>4} (starved {:>4})",
                        "",
                        a.founded.0,
                        a.founded.1,
                        a.nest_x,
                        a.nest_span.0,
                        a.nest_span.1,
                        a.founders_on_nest,
                        a.ants_seen,
                        a.target_x,
                        a.target_x - a.founded.1,
                        a.target_x - a.founded.0,
                        a.target_x - a.nest_x,
                        occ.join(" "),
                        a.carry_toward_nest,
                        a.births,
                        a.deaths,
                        a.starved
                    );
                    // **Do the survivors keep the trail up once we stop laying
                    // it?** Owner's ask. `stop` releases the hand-laid ramp at
                    // frame 6,000 and these are sampled only well after that
                    // (`stop + 1500`, past the ~1,476-frame lifetime of a cell
                    // laid at `DEPOSIT`), so they describe the ANTS' trail and
                    // not ours. `route pk` is the most of the route that ever
                    // held channel B at one sample, `end` is what is left at
                    // the finish, and `along` is which way it climbs --
                    // positive means it rises toward the NEST, which is §1c's
                    // prediction and the wrong way round for finding food.
                    println!(
                        "{:>16}own trail: route pk {:>4} end {:>4} along {:>+7.4}  B nest->food [{}]  blocked {:>8}  kin swaps {:>7}  ticks {:>9}  tumbles {:>9} (homeward {:>8}, {:.2}%)  drops {:>7}  DELIVERED {:>5}  leg home n {:>4} med {:>5} p90 {:>5} (laden n {:>4} med {:>5} p90 {:>5})  appetite held {:>9.0} J of {:>9.0}  trips born-on-comb {:>4} ({} ants) / born-off {:>4} ({} ants)",
                        "",
                        a.peak_cells,
                        a.live_cells,
                        a.natural_along,
                        a.b_profile.iter().map(|v| format!("{v}")).collect::<Vec<_>>().join(","),
                        a.blocked,
                        a.kin_swaps,
                        a.ticks,
                        a.tumbles,
                        a.tumbles_homeward,
                        if a.tumbles == 0 { 0.0 } else { 100.0 * a.tumbles_homeward as f64 / a.tumbles as f64 },
                        a.drops,
                        a.deliveries,
                        a.leg_n,
                        a.leg_med,
                        a.leg_p90,
                        a.laden_leg_n,
                        a.laden_leg_med,
                        a.laden_leg_p90,
                        a.digest_appetite_held,
                        a.digested_face,
                        a.trips_on_nest,
                        a.ants_on_nest,
                        a.trips_off_nest,
                        a.ants_seen - a.ants_on_nest
                    );
                    // **How much of `Carrying` is dig tailings rather than
                    // food.** `Carrying` gates the channel A reader (units 0/1)
                    // and drives the channel B emitter, and it is true for
                    // spoil, so this one share prices both. A high figure in an
                    // arm with no food to find is the proposed explanation for
                    // `homeA` clipping exploration: a spoil-hauling ant opens
                    // the homing gate legitimately, and a laden ant on a
                    // standing A ramp runs at P(move) 0.641 against a baseline
                    // of 0.200 (`onetrail mode=arith`).
                    println!(
                        "{:>16}channel A: PEAK amt {:>6} cells {:>4} (of {} route)  POLARITY|| {:>+8.5} && {:>+8.5} SPAN {:>+8.5} edge {:>5.1}  end nest->food [{}]  nest {:>4}  AtNest {:>5.2}%",
                        "",
                        a.a_peak_amt,
                        a.a_peak_cells,
                        g + 1,
                        a.a_polarity,
                        a.a_polarity_both,
                        a.a_polarity_span,
                        a.a_edge_cells,
                        a.a_profile.iter().map(|v| format!("{v}")).collect::<Vec<_>>().join(","),
                        a.nest_cells,
                        if a.probe_ticks == 0 { 0.0 } else { 100.0 * a.atnest_ticks as f64 / a.probe_ticks as f64 }
                    );
                    println!(
                        "{:>16}Carrying: laden {:>9}  of which SPOIL {:>9} ({:>5.1}%)",
                        "",
                        a.laden_ticks,
                        a.spoil_ticks,
                        if a.laden_ticks == 0 { 0.0 } else { 100.0 * a.spoil_ticks as f64 / a.laden_ticks as f64 }
                    );
                }
            }
        }
        println!("\n  larder put out, over the whole sweep: {supply_lo:.0}-{supply_hi:.0} J, against a need near 46,800 J");
        println!("    for 52 ants over 24,000 frames -- so these colonies are provisioned, not starved.");
        println!("  intake off anything that is not the larder is ASSERTED to be 0 in every row, not printed.");
        println!("\n  `self` vs `mute` is the real question: do the ants' OWN trails do anything?");
        println!("  `visitors` is DISTINCT ants that reached the food over distinct ants that ever lived --");
        println!("    recruitment, which `near` ant-ticks cannot see: 50 visitors and 1 resident read alike.");
        println!("  `trips` is nest -> food -> nest, closed on the RETURN. `reach` buckets every ant by how");
        println!("  `occupancy` is ant-ticks in 8 equal bands, NEST END FIRST, food end last -- a commuting");
        println!("    colony is bimodal, a dispersing one slides its mass right, a homebody is a spike at 0.");
        println!("  `carry->nest` is signed cells moved while holding larder, POSITIVE = homeward. Near zero");
        println!("    means food is picked up and wandered with, which is a different fault from not finding it.");
        println!("    far it ever got, as a share of the gap, so a commuting population is visible as a shape.");
        println!("  `hand` vs `self` only says whether a laid ramp beats what they bootstrap.");
        println!("  `hmute` is the DECAY BASELINE for `route pk`: our ramp laid, the ants silenced.");
        println!("  `flatN`/`flatF` are the POLARITY metric's negative controls: a FLAT channel A blob over");
        println!("    the nest half / the food half, ants' EmitA silenced. A field with no ramp in it at all.");
        println!("    A correct metric reads 0 on both. POLARITY|| is predicted to read ~+0.12 and ~-0.12,");
        println!("    which brackets every polarity figure in the report -- see `lay_flat`.");
        println!("  `homeA` lays a channel A HOMING ramp and no food trail -- if laden ants only fail");
        println!("    to route because they cannot find their way home, this is where their own B appears.");
        println!("  `carry` is ant-ticks holding larder in the crop; `carry@nest` is those inside the +-26 nest band --");
        println!("    that pair is the answer to \"is food being carried back\", which a cell census cannot give.");
        println!("  `other J` must read 0 in every row: it is the check that `onlyfood` held.");
        println!("  A gap where `alive` reaches 0 in every arm is measuring a dead colony, not a deaf one.");
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
                let r = run(s, true, gate, frames, a, relay, near, food, stop, gap, refill, diet, false, PaintA::None);
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
        let on = run(s, true, gate, frames, ants, relay, near, food, stop, gap, refill, diet, false, PaintA::None);
        let off = run(s, false, gate, frames, ants, relay, near, food, stop, gap, refill, diet, false, PaintA::None);
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
