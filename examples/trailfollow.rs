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
use pixel_physics::sim::creature;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::material::MaterialId;
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
    /// Times a body traded places with a nestmate -- the "did it fire" counter
    /// for `kinpass`, which must read 0 when the switch is off.
    kin_swaps: u64,
    /// Blocked move attempts, as the thing `kin_swaps` is meant to reduce.
    blocked: u64,
    /// Distinct ants that ever came within `near` of the food -- recruitment.
    visitors: usize,
    /// Distinct ants that ever lived in this run, as the denominator.
    ants_seen: usize,
    /// Honest round trips summed over the colony. See `Track::trips`.
    round_trips: u64,
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
    trips: u32,
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
    let mut moved = 0;
    for i in 0..brain::BRAIN_INPUTS {
        let slot = brain::io_slot(brain::INPUTS[i], O::EmitB);
        if g[slot] != 0.0 {
            g[slot] = 0.0;
            moved += 1;
        }
    }
    for h in 0..brain::BRAIN_HIDDEN {
        let slot = brain::ho_slot(h, O::EmitB);
        if g[slot] != 0.0 {
            g[slot] = 0.0;
            moved += 1;
        }
    }
    moved
}

#[allow(clippy::too_many_arguments)]
fn run(seed: u64, trail: bool, gate: Gate, frames: u64, ants: i32, relay: u64, near: i32, food: i32, stop: u64, gap: i32, refill: u64, diet: Diet, mute: bool, home: bool) -> Arm {
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
    let species_id = w.species.id_of("ant").expect("the ant species is compiled in");
    let mut genome = w.species.get(species_id).genome.clone();
    let moved = gate.apply(&mut genome);
    assert!(gate.name == "shipped" || moved > 0, "gate {} changed no slot, so both arms carry one genome", gate.name);
    if mute {
        let silenced = mute_channel_b(&mut genome);
        assert!(silenced > 0, "no EmitB weight was zeroed, so the muted arm still lays the plane it is meant to be without");
    }

    let surface = spec.ground_y - 2;
    let (nest_x, target_x) = (half_band, half_band + gap);
    // The species' own sensor reach, not a literal -- see the along readout.
    let sensor_offset = w.species.get(species_id).creature.as_ref().expect("ant is a creature").sensor_offset;
    let placed = w.found_colony_of(nest_x, surface, "ant", ants);
    assert!(placed > 0, "no ants placed at the nest end; there is nothing to measure");
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
    for f in 1..=frames {
        // **`stop` is what turns this from a pull arm into a loop arm.** Up to
        // `stop` the trail is guaranteed, which breaks the circularity -- a
        // naturally laid trail needs commuters, and commuters need a trail
        // worth following. After it, the only channel B in the world is what
        // the ants themselves put down.
        if (stop == 0 || f <= stop) && (f == 1 || f.is_multiple_of(relay)) {
            if trail {
                lay(&mut w, nest_x, target_x, surface);
            }
            if home {
                lay_home(&mut w, nest_x, target_x, surface);
            }
        }
        frame::step(&mut w, &mut particles, &mut blasts, player::PlayerInput::default(), &tuning);
        if food > 0 && refill > 0 && f.is_multiple_of(refill) {
            place_food(&mut w, food, &mut larder_placed);
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
            let carrying_larder = s.crop.is_some_and(|c| c.material == larder);
            {
                let at_food = (hx - target_x).abs() <= near;
                let at_nest = (hx - nest_x).abs() <= 26;
                let t = tracks.entry(id).or_default();
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
                }
                // A trip closes on the return, not the arrival: an ant that
                // reaches the food and dies there has not made a round trip,
                // and counting it as one is how a foraging number turns into
                // an exposure number.
                if at_nest && t.outbound {
                    t.trips += 1;
                    t.outbound = false;
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
        ticks: st.ticks,
        kin_swaps: st.kin_swaps,
        blocked: st.moves_blocked,
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
            .unwrap_or_else(|| vec![90, 150, 220, 300]);
        // **`arms=homeA` runs one arm instead of five**, which is what makes a
        // 36-seed replication affordable: an effect that shows up in 1 seed of
        // 12 needs more seeds, not more arms, and paying for four irrelevant
        // arms is what stops anyone running them.
        let want_arms: Option<Vec<String>> = arg_str("arms").map(|v| v.split(',').map(|t| t.trim().to_string()).collect());
        for g in gaps {
            for s in seed0..seed0 + seeds {
                for (name, trail, mute, home) in [
                    ("hand", true, false, false),
                    ("hmute", true, true, false),
                    ("self", false, false, false),
                    ("mute", false, true, false),
                    // **The homing arm.** No food trail is laid; a channel A
                    // ramp peaking at the nest is, and the ants lay their own
                    // channel B as usual. If the owner's chain is the whole
                    // story, this is the arm where a colony finally builds a
                    // trail for itself.
                    ("homeA", false, false, true),
                ] {
                    if want_arms.as_ref().is_some_and(|w| !w.iter().any(|x| x == name)) {
                        continue;
                    }
                    let a = run(s, trail, gate, frames, ants, relay, near, food, stop, g, refill, diet, mute, home);
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
                    // Second line, because these are the shape readouts and a
                    // shape does not fit in a column. `occupancy` is nest-end
                    // first; `carry->nest` is signed cells, positive homeward.
                    let occ: Vec<String> = a.occupancy.iter().map(|v| format!("{}", v / 1000)).collect();
                    println!(
                        "{:>16}founded x {:>4}..{:<4}  occupancy/1k [{}]  carry->nest {:>7}  born {:>4} died {:>4} (starved {:>4})",
                        "",
                        a.founded.0,
                        a.founded.1,
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
                        "{:>16}own trail: route pk {:>4} end {:>4} along {:>+7.4}   blocked {:>8}  kin swaps {:>7}  ticks {:>9}",
                        "", a.peak_cells, a.live_cells, a.natural_along, a.blocked, a.kin_swaps, a.ticks
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
                let r = run(s, true, gate, frames, a, relay, near, food, stop, gap, refill, diet, false, false);
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
        let on = run(s, true, gate, frames, ants, relay, near, food, stop, gap, refill, diet, false, false);
        let off = run(s, false, gate, frames, ants, relay, near, food, stop, gap, refill, diet, false, false);
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
