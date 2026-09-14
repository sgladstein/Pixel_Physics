//! **What one ant costs per tick, in the bed the owner actually played** —
//! the headless replay of `Reports/evolution-lab-playtest-2026-09-13.md` §1.
//!
//! That report fitted the owner's own wall clock at
//! `cost ≈ 1.0 ms/tick + ~2.1 µs per ant per tick`, and said outright what it
//! could not do: **localise the cost inside the creature pass**. Its clock was
//! contended — adjacent samples at equal ant count differ 12–14x — so the
//! first job here is not to optimise anything, it is to reproduce that fit on
//! a quiet box. A headless number far from 2.1 µs/ant is not a better
//! measurement of the same thing; it is a measurement of something else, and
//! that is the finding.
//!
//! **Why this is not `lab_cost phases=1`.** That harness sweeps *walls* and
//! *fans* over one bed and prints a phase table; the independent variable
//! here is **standing ant count**, which nothing in `examples/` sweeps. It is
//! also not `creature_scale`, whose axis is cell resolution, nor
//! `ant_ablation`, whose axis is which sub-behaviour is switched off (that is
//! step 2 and this is step 1). The shared machinery is `Lab::tick_for_harness`
//! — the shipped tick, nothing skipped — which is what makes the number
//! comparable to a played session at all.
//!
//! ## The three measurement rules it is built around
//!
//! 1. **Arms are compared inside one run, round-robin, never arm-after-arm.**
//!    Every population keeps its own live `Lab`; a rep times `frames` ticks of
//!    each in turn, and the reported figure is the **minimum** over reps — the
//!    lower envelope, the same estimator the playtest report defends, because
//!    contention can only ever make a run slower.
//! 2. **The x-axis is the *measured* population, not the requested one.**
//!    Ants breed and die while the clock runs, so each arm prints the mean of
//!    its own census across the timed window. A requested count would be a
//!    knob nobody can tell is disconnected.
//! 3. **`ants=0` is the positive control from the quiet side and the whole
//!    point of the intercept.** An empty bed must land near the playtest's
//!    ~1.0 ms and must report zero creature ticks; if it does not, the slope
//!    below it is being fitted through an intercept that is measuring
//!    something this harness does not understand.
//!
//! **`stocking` is the honest half of the readout.** Reaching three thousand
//! ants by breeding takes the owner 560,000 frames, which no `Bash` call here
//! can hold, so the bed is stocked by repeated `found_colony_of` rounds with
//! dispersal frames between them — `colony_stations` lays one row at a body-
//! derived spacing, so a single founding on a 512-wide bed tops out around
//! thirty and the rest is arrival over time. The achieved count is printed for
//! every arm beside the count that was asked for, because a stocking loop that
//! quietly saturates would otherwise publish three arms wearing six labels.
//!
//! ```text
//! cargo run --release --example antcost
//! cargo run --release --example antcost -- ants=0,200,600,1200 frames=300 reps=4
//! RAYON_NUM_THREADS=4 cargo run --release --example antcost -- reps=5
//! SCHED_PASS=200 cargo run --release --example antcost -- ants=1200 reps=1
//! ```
//!
//! `SCHED_PASS=N` (read by `sim::scheduler`) prints the per-kind site
//! breakdown beside these rows and is how the creature phase's share of the
//! frame is read. It puts two `Instant::now()` calls around every site, so a
//! headline whole-frame figure must be quoted from a run **without** it.

use pixel_physics::lab::scene::LabBox;
use pixel_physics::sim::world::World;
use pixel_physics::sim::creature;
use pixel_physics::lab::Lab;
use std::time::Instant;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// One population under test, with its own live box.
/// FNV-1a over every cell in the box — material, `aux` and organism id.
///
/// **The determinism gate, and it is here rather than only in `lab_cost`
/// because of what this harness's bed has in it.** `lab_cost colonies=1`
/// stands six ants up; the whole point of round 33's change is what happens
/// when a bed holds hundreds, and a colony that dense is exactly where two
/// animals contend for the same cell in the same tick. Two `par` arms of one
/// ant count are the same bed stocked by the same deterministic loop, so
/// **their hashes must match exactly** — and the `unchecked` arm is the
/// control that shows the comparison can fail.
///
/// The same arithmetic as `lab_cost`'s `world_hash`, deliberately: two gates
/// that hash differently cannot be compared to each other.
fn world_hash(w: &World) -> u64 {
    fn fnv1a(h: u64, v: u64) -> u64 {
        (h ^ v).wrapping_mul(0x0000_0100_0000_01b3)
    }
    let b = w.bounds().expect("the lab box sets bounds");
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            h = fnv1a(h, c.material.0 as u64);
            h = fnv1a(h, c.aux() as u64);
            h = fnv1a(h, c.organism_id() as u64);
        }
    }
    h
}

/// One point for the per-bed fit: which arm it came from (`par` and the
/// `(width, height, founders)` the bed was built at) and the pair being
/// regressed — mean population over the quietest rep, and that rep's µs/tick.
type FitPoint = (creature::ParMode, (i32, i32, usize), f64, f64);

struct Arm {
    want: usize,
    /// Bed size this arm was built at — the second axis, and the one that
    /// separates density from count.
    bw: i32,
    bh: i32,
    /// Herb founders this arm's bed was built with — the plant axis.
    nf: usize,
    /// **Which read-phase schedule this arm runs** — round 33's creature-pass
    /// parallelism. Held per arm rather than per process precisely so both
    /// arms sit in one run: the thing under test is the parallelism, and
    /// `CLAUDE.md` says a counter is only load-independent at fixed
    /// parallelism, so a serial and a parallel arm measured in two processes
    /// are two different machines' worth of noise apart.
    par: creature::ParMode,
    lab: Lab,
    /// What the stocking loop actually got onto the bed.
    stocked: usize,
    /// Summed `Lab` ticks and summed nanoseconds, per rep.
    rep_ns: Vec<u128>,
    /// Mean standing ant count over each rep's timed window.
    rep_ants: Vec<f64>,
    /// `(senses taken from the speculation window, senses recomputed
    /// serially)` over each rep — the "did it fire" pair for the parallel
    /// read phase. A parallel arm whose cached share is near zero is a
    /// serial arm wearing a label.
    rep_spec: Vec<(u64, u64)>,
    /// Wall time inside the parallel read phase, per rep. **The number that
    /// says whether it is parallel at all**: run the same arm at one thread
    /// and at four and read this, not the frame total.
    rep_spec_ns: Vec<u64>,
    /// Creature ticks dispatched over each rep's window — the "did it fire"
    /// counter, paired with the wall clock beside it. An arm whose cost rose
    /// while this stayed flat is not measuring creatures.
    rep_ticks: Vec<u64>,
    /// **The effect counter from the far side of the call**, per
    /// `CLAUDE.md`: `ticks` says the pass ran, and only these say what the
    /// animals got for it. A tick that ends in a refused step is not a
    /// cheaper tick — `step_chain` prices its alternatives before it gives
    /// up — so a bed whose ants are jammed pays *more* per ant than one whose
    /// ants are walking, and the blocked fraction is the column that says
    /// which bed is on the clock.
    rep_moves: Vec<u64>,
    rep_blocked: Vec<u64>,
    /// **Awake chunks and the soil-moisture pass's own walk, summed over the
    /// timed window** — the two quantities that say whether an ant is
    /// expensive *inside* `creature::tick` or expensive because of what it
    /// leaves dirty behind it.
    ///
    /// **They exist because they refuted the hypothesis that added them, and
    /// that is why they are worth keeping.** A callgrind profile of this
    /// harness at 151 ants put `world::visit_soil_water` at **37.8% of all
    /// instructions** against `creature::tick`'s 7.1%, which reads as *ants
    /// dirty soil, the moisture pass walks the row hull of every mark*. These
    /// columns say otherwise on a wall clock: soil-water visits are **highest
    /// at zero ants** (13,591, against 7,804 at 148 and 10,315 at 254) and
    /// awake chunks are flat in ant count (18.1 / 20.9 / 19.3). That profile's
    /// bed had **8 plants in it**, so the ants were the only things marking
    /// soil at all -- `CLAUDE.md`'s worst-recurring failure, a number that is
    /// arithmetically correct and about the wrong question.
    ///
    /// The switch that settles it outright is `PIXEL_PHYSICS_MOISTURE=sweep`
    /// (`update::moisture_phase_enabled`, a control and never a setting),
    /// which puts the pass back inside the CA sweep: it moves the intercept
    /// **1.84x** and the per-ant slope by **0.3%**. So the moisture pass is
    /// the *intercept*, and these columns are what stops the next session
    /// re-deriving that.
    /// `Reports/evolution-lab-creature-cost-2026-09-13.md` §2.
    rep_awake: Vec<u64>,
    rep_swvisited: Vec<u64>,
    rep_swsoil: Vec<u64>,
    /// **Standing depth of the scheduler's site heaps, summed over the timed
    /// window** — `World::active_site_count`, which is
    /// `active_sites.len() + creature_sites.len()`.
    ///
    /// Round 34's first candidate for the knee, and the reason it is a
    /// *standing* count rather than a rate: `scheduler::step` pops until the
    /// soonest-due site is in the future **or** it has taken
    /// `MAX_SITES_PER_FRAME` (2,000) background sites and
    /// `MAX_CREATURE_SITES_PER_FRAME` (256) creature ones. Below saturation
    /// the frame does only what is due; at saturation it does the cap every
    /// frame and a backlog stands behind it. That is a *step change in work
    /// per frame*, not a gradual one, and it is exactly the shape a knee
    /// has. `CLAUDE.md`: measure the standing state, not the event rate.
    rep_sites: Vec<u64>,
    /// Summed creature scheduling lateness over the window, and the
    /// run-cumulative worst single lateness (`CreatureStats::tick_lag_sum`
    /// delta / `tick_lag_max` absolute). Zero is the healthy value — a
    /// creature reschedules itself to an exact frame, so anything above zero
    /// is the creature budget binding. **The pair matters**: a mean near zero
    /// with a large max is a burst, and a burst and a standing starvation
    /// want opposite fixes.
    rep_lag: Vec<u64>,
    rep_lagmax: Vec<u64>,
    /// Standing plant count at the end of each rep's window — the other
    /// population in the bed, and the one an ant-count axis silently varies.
    rep_plants: Vec<u64>,
    /// **Cells inside every awake chunk's expanded dirty rect, summed over the
    /// timed window** — what the CA sweep is *asked* for, the same quantity
    /// `examples/labperf.rs` calls `swept`, and the one that names the half of
    /// an ant's cost that is not in the creature pass.
    ///
    /// Off unless `swept=1`, because unlike the other columns here it
    /// allocates a `Vec` per frame inside the timed loop. Counters are
    /// load-independent, so the honest shape is a second run for the counter
    /// rather than a tax on the headline timing.
    rep_swept: Vec<u64>,
    /// **Plant *cells*, not plant *count*** — summed `OrganismState::cells`
    /// over every live organism that is not a creature, read once at the end
    /// of each rep.
    ///
    /// Here because the plant term is what a cost curve in ant count reads
    /// when nobody controls it, and `plants` is the wrong ruler for it: a bed
    /// whose ants have eaten the canopy has the same number of plants and a
    /// fraction of the tissue. `labbox_cost` prices a plant at ~0.7 µs per
    /// plant cell per tick, which is the unit this column is in.
    rep_pcells: Vec<u64>,
}

/// **Stock the bed to `want` ants using only the shipped founding path.**
///
/// Returns the standing creature count it reached. `found_colony_of` places a
/// single row at `colony_stations`' body-derived spacing, so one call cannot
/// fill a bed; the loop alternates founding with dispersal frames so the
/// stations free up. It gives up after `rounds` rounds rather than spinning —
/// a saturated bed is a real answer and the caller prints it.
fn stock(lab: &mut Lab, species: &str, want: usize, ground_y: i32, width: i32, rounds: usize, settle: u64) -> usize {
    if want == 0 {
        return 0;
    }
    // Four founding columns across the usable width, so the rows overlap
    // less than one centred founding would and the colony does not end up
    // as a single wall of bodies.
    let cols: Vec<i32> = (1..=4).map(|i| width * i / 5).collect();
    for _ in 0..rounds {
        if lab.world.live_creature_count() >= want {
            break;
        }
        for &cx in &cols {
            if lab.world.live_creature_count() >= want {
                break;
            }
            lab.world.found_colony_of(cx, ground_y - 2, species, 16);
        }
        for _ in 0..settle {
            lab.tick_for_harness();
        }
    }
    lab.world.live_creature_count()
}

fn main() {
    let ants_arg: String = arg("ants").unwrap_or_else(|| "0,150,400,800,1400".to_string());
    let wants: Vec<usize> = ants_arg.split(',').map(|s| s.parse().expect("an ant count")).collect();
    // **`par=on,off` is the paired arm round 33 exists to measure**, and the
    // pairing is the point: the two arms are the same bed, stocked by the
    // same deterministic loop, interleaved rep by rep on one box. Anything
    // less is a timing from one machine against a timing from another one.
    let par_arg: String = arg("par").unwrap_or_else(|| "on".to_string());
    let pars: Vec<creature::ParMode> = par_arg
        .split(',')
        .map(|m| match m {
            "off" => creature::ParMode::Off,
            "unchecked" => creature::ParMode::Unchecked,
            "verify" => creature::ParMode::Verify,
            "on" => creature::ParMode::Checked,
            other => panic!("par= takes on, off or unchecked, not {other:?}"),
        })
        .collect();
    let frames: u64 = arg("frames").unwrap_or(400);
    let reps: usize = arg("reps").unwrap_or(3);
    let seed: u64 = arg("seed").unwrap_or(1);
    let d = LabBox::default();
    let width: i32 = arg("width").unwrap_or(d.width);
    // **`widths=` and `heights=` put the bed-size axis in the same process as
    // the ant-count axis, and that is the round-34 discriminator.**
    //
    // Hold the animal count fixed and vary the bed: if the per-ant cost falls
    // as the bed widens, what is being measured is *density* — crowding, a
    // neighbourhood query, ants queueing for the same cell. If it does not
    // move, it is raw count. That comparison is worthless across two
    // processes (`CLAUDE.md`: a timing is only as trustworthy as the box was
    // quiet, and two runs of a byte-identical binary once disagreed 2.42x),
    // so the widths become arms of one round-robin like everything else here.
    let widths: Vec<i32> = arg::<String>("widths")
        .map(|v| v.split(',').map(|s| s.parse().expect("a width")).collect())
        .unwrap_or_else(|| vec![width]);
    // **512, because that is what the owner raised the box to during setup**
    // and the log header states it. Not the `LabBox` default.
    let height: i32 = arg("height").unwrap_or(512);
    let heights: Vec<i32> = arg::<String>("heights")
        .map(|v| v.split(',').map(|s| s.parse().expect("a height")).collect())
        .unwrap_or_else(|| vec![height]);
    let soil: i32 = arg("soil").unwrap_or(d.soil_depth);
    let founders: usize = arg("founders").unwrap_or(d.founders);
    // **`plants=` is the third axis, and it exists because the second regime
    // of round 32's knee is a plant count, not an ant count.**
    //
    // Ants eat. In a planted bed the arms with more ants have *fewer* plants,
    // so the ant term and the plant term move in opposite directions and a
    // cost curve fitted in ant count alone reads the difference. Crossing
    // herb founders with ant count breaks that collinearity: it is the only
    // way to ask what a plant costs in the same run that asks what an ant
    // costs, and `CLAUDE.md`'s rule about a term in a weighted sum applies to
    // a regression exactly as it does to a brain.
    let founder_arms: Vec<usize> = arg::<String>("plants")
        .map(|v| v.split(',').map(|s| s.parse().expect("a founder count")).collect())
        .unwrap_or_else(|| vec![founders]);
    let species: String = arg("species").unwrap_or_else(|| d.species.clone());
    let colony_species: String = arg("colony_species").unwrap_or_else(|| "longant".to_string());
    // Frames of plant growth before any ant arrives, so the intercept is
    // measured over a bed with something living in it rather than over eight
    // seeds. The played bed had 264–409 plants.
    let grow: u64 = arg("grow").unwrap_or(6_000);
    let rounds: usize = arg("rounds").unwrap_or(200);
    // **`age=N` equalises bed age across arms, and without it the ant count
    // is confounded with it.** The stocking loop alternates founding with
    // `settle` dispersal frames, so a 400-ant arm leaves the loop thousands
    // of frames older than the `ants=0` arm — measured at frame 4,440 against
    // 1,000, with 27 plants standing against 8. Every arm is then a different
    // *bed* as well as a different population, and the difference is charged
    // to the ants because they are the x-axis. `CLAUDE.md`'s worst-recurring
    // failure, arriving through the harness rather than through the metric.
    //
    // Set it above the oldest arm's post-stocking frame and every arm is
    // ticked forward to it before the clock starts. It does **not** equalise
    // plant *count* — ants eat, and that is a real consequence of the
    // population rather than an artifact of the loop — but it removes the
    // part that is purely bookkeeping.
    let age: u64 = arg("age").unwrap_or(0);
    // See `Arm::rep_swept`. Quote a headline timing from a run without it.
    let swept_on: bool = arg::<u32>("swept").unwrap_or(0) == 1;
    let settle: u64 = arg("settle").unwrap_or(40);
    // `PLANT_LOAD_FAILURE false` — the one dial off in the played session.
    let plant_load: bool = arg::<u32>("plant_load").unwrap_or(0) == 1;

    let threads = std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "unset".to_string());
    let sched = std::env::var("SCHED_PASS").unwrap_or_else(|_| "0".to_string());
    // **Echoed because it is the one switch that can silently halve the
    // frame.** `update::moisture_phase_enabled` reads it once through a
    // `OnceLock`, so it cannot be set per arm and a run that has it set looks
    // exactly like a run that does not unless the header says so.
    let moisture = std::env::var("PIXEL_PHYSICS_MOISTURE").unwrap_or_else(|_| "on".to_string());
    println!(
        "antcost: ants={wants:?} frames={frames} reps={reps} seed={seed} widths={widths:?} heights={heights:?} \
         soil={soil} founders={founder_arms:?} species={species} colony_species={colony_species} grow={grow} \
         rounds={rounds} settle={settle} plant_load={plant_load} RAYON_NUM_THREADS={threads} SCHED_PASS={sched} \
         PIXEL_PHYSICS_MOISTURE={moisture}"
    );

    let spec_for = |width: i32, height: i32, founders: usize| LabBox {
        founders,
        width,
        height,
        soil_depth: soil,
        species: species.clone(),
        // **Founded by the stocking loop, never by the scene**, so the
        // `ants=0` arm is an identical bed with nothing standing in it
        // rather than a differently-built one.
        colonies: 0,
        colony_species: colony_species.clone(),
        seed,
        ..d.clone()
    };

    let mut arms: Vec<Arm> = Vec::new();
    for &bw in &widths {
    for &bh in &heights {
    for &nf in &founder_arms {
    for &want in &wants {
        for &par in &pars {
            let spec = spec_for(bw, bh, nf);
            let ground_y = spec.ground_y;
            let width = bw;
            let mut lab = Lab::new(spec);
            lab.world.plant_load_failure = plant_load;
            lab.world.creature_par.mode = par;
            for _ in 0..grow {
                lab.tick_for_harness();
            }
            let stocked = stock(&mut lab, &colony_species, want, ground_y, width, rounds, settle);
            let stocked_at = lab.world.frame;
            while lab.world.frame < age {
                lab.tick_for_harness();
            }
            let stocked = stocked.max(lab.world.live_creature_count());
            println!(
                "  stocking: {bw}x{bh} founders {nf:>3} want {want:>5} par {par:?} -> standing {:>5} ants, {:>5} plants, stocked at frame {stocked_at}, aged to {}",
                lab.world.live_creature_count(),
                lab.world.live_organism_count() - lab.world.live_creature_count(),
                lab.world.frame
            );
            arms.push(Arm {
                want,
                bw,
                bh,
                nf,
                par,
                lab,
                stocked,
                rep_ns: Vec::new(),
                rep_ants: Vec::new(),
                rep_spec: Vec::new(),
                rep_spec_ns: Vec::new(),
                rep_ticks: Vec::new(),
                rep_moves: Vec::new(),
                rep_blocked: Vec::new(),
                rep_awake: Vec::new(),
                rep_swvisited: Vec::new(),
                rep_swsoil: Vec::new(),
                rep_sites: Vec::new(),
                rep_lag: Vec::new(),
                rep_lagmax: Vec::new(),
                rep_plants: Vec::new(),
                rep_swept: Vec::new(),
                rep_pcells: Vec::new(),
            });
        }
    }
    }
    }
    }

    // **Round-robin, and the reps interleave rather than nest per arm.** A
    // machine that gets busy halfway through a run slows whichever arm is in
    // front of it; interleaving spreads that over all of them, and taking the
    // minimum over reps then reads the quietest window each arm saw.
    for rep in 0..reps {
        for arm in arms.iter_mut() {
            let spec_before = creature::speculation_census();
            let spec_ns_before = creature::speculation_nanos();
            let ants_before = arm.lab.world.live_creature_count();
            let ticks_before = arm.lab.world.creature_stats.ticks;
            let moves_before = arm.lab.world.creature_stats.moves;
            let blocked_before = arm.lab.world.creature_stats.moves_blocked;
            let lag_before = arm.lab.world.creature_stats.tick_lag_sum;
            // **Accumulated inside the timed loop, and that is a real cost
            // this harness pays.** `soil_water_stats` is overwritten every
            // frame, so it cannot be read afterwards; three `u64` adds and one
            // `active_chunk_count` per frame is identical work in every arm,
            // so it cannot tilt a comparison between them, and it is ~0.1% of
            // a 2,500 µs frame. Timed rather than excluded because excluding
            // it would need a second clock inside the loop, which costs more
            // than the thing it was measuring.
            let mut awake = 0u64;
            let mut swv = 0u64;
            let mut sws = 0u64;
            let mut sites = 0u64;
            let mut swept = 0u64;
            let t = Instant::now();
            for _ in 0..frames {
                arm.lab.tick_for_harness();
                awake += arm.lab.world.active_chunk_count() as u64;
                swv += arm.lab.world.soil_water_stats.visited;
                sws += arm.lab.world.soil_water_stats.soil;
                sites += arm.lab.world.active_site_count() as u64;
                if swept_on {
                    for c in arm.lab.world.chunks_to_sweep() {
                        if let Some(r) = arm.lab.world.sweep_region(c) {
                            swept += ((r.max_x - r.min_x + 1) as i64 * (r.max_y - r.min_y + 1) as i64) as u64;
                        }
                    }
                }
            }
            let ns = t.elapsed().as_nanos();
            let ants_after = arm.lab.world.live_creature_count();
            arm.rep_ns.push(ns);
            arm.rep_ants.push((ants_before + ants_after) as f64 / 2.0);
            let spec_after = creature::speculation_census();
            arm.rep_spec.push((spec_after.0 - spec_before.0, spec_after.1 - spec_before.1));
            arm.rep_spec_ns.push(creature::speculation_nanos() - spec_ns_before);
            arm.rep_ticks.push(arm.lab.world.creature_stats.ticks - ticks_before);
            arm.rep_moves.push(arm.lab.world.creature_stats.moves - moves_before);
            arm.rep_blocked.push(arm.lab.world.creature_stats.moves_blocked - blocked_before);
            arm.rep_awake.push(awake);
            arm.rep_swvisited.push(swv);
            arm.rep_swsoil.push(sws);
            arm.rep_sites.push(sites);
            arm.rep_swept.push(swept);
            // Outside the timed loop: one walk of the organism table per rep.
            let pcells: u64 = arm
                .lab
                .world
                .live_organism_ids()
                .into_iter()
                .filter_map(|id| arm.lab.world.organism_state(id))
                .filter(|st| arm.lab.world.species.get(st.species).creature.is_none())
                .map(|st| st.cells.len() as u64)
                .sum();
            arm.rep_pcells.push(pcells);
            arm.rep_plants.push((arm.lab.world.live_organism_count() - arm.lab.world.live_creature_count()) as u64);
            arm.rep_lag.push(arm.lab.world.creature_stats.tick_lag_sum - lag_before);
            // **Absolute, not a delta.** `tick_lag_max` is a running maximum,
            // so a window's "growth" reads zero whenever stocking already saw
            // something worse — which is most of the time and would look like
            // a healthy scheduler. Quoted as what it is: the worst lateness
            // this arm's bed has ever seen, stocking included.
            arm.rep_lagmax.push(arm.lab.world.creature_stats.tick_lag_max);
        }
        eprintln!("  rep {}/{reps} done", rep + 1);
    }

    println!(
        "\n{:>6} {:>12} {:>9} {:>7} {:>8} {:>7} {:>8} {:>10} {:>8} {:>9} {:>8} {:>6} {:>8} {:>10} {:>10} {:>10} {:>9} {:>8} {:>9} {:>7} {:>8} {:>10} {:>10}",
        "want", "bed", "par", "stocked", "ants", "plants", "pcells", "µs/tick", "min/med", "crtick/f", "moves/f", "blk%", "awake/f", "swept/f", "sites/f", "lag/tick", "lag max", "µs/ant", "spread", "cached%", "spec µs/f", "sw seen", "sw soil"
    );
    let mut beds: Vec<(i32, i32, usize)> = arms.iter().map(|a| (a.bw, a.bh, a.nf)).collect();
    beds.dedup();
    // Points for the fit: (mean ants over the quietest rep, µs/tick).
    let mut pts: Vec<FitPoint> = Vec::new();
    for arm in &arms {
        let mut us: Vec<f64> = arm.rep_ns.iter().map(|&n| n as f64 / 1000.0 / frames as f64).collect();
        let best = us.iter().cloned().fold(f64::INFINITY, f64::min);
        let best_i = us.iter().position(|&v| v == best).expect("a rep");
        us.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
        let med = us[us.len() / 2];
        let ants = arm.rep_ants[best_i];
        let crticks = arm.rep_ticks[best_i] as f64 / frames as f64;
        // Per-ant cost against this arm alone, which is *not* the fit: it
        // charges the whole ~1 ms of background to the ants. Printed anyway
        // because a column that disagrees with the slope by an order of
        // magnitude is how a broken fit announces itself.
        let per = if ants > 0.0 { best / ants } else { f64::NAN };
        let moves = arm.rep_moves[best_i] as f64 / frames as f64;
        let blocked = arm.rep_blocked[best_i] as f64;
        let blk = if moves > 0.0 || blocked > 0.0 {
            100.0 * blocked / (arm.rep_moves[best_i] + arm.rep_blocked[best_i]) as f64
        } else {
            f64::NAN
        };
        // **The "did it fire" pair for the parallel read phase.** A parallel
        // arm whose cached share is near zero has measured the overhead and
        // none of the benefit, and the wall clock alone cannot say so.
        // Mean creature scheduling lateness in frames: zero means every
        // dispatched creature ran on the frame it asked for.
        let lag_per_tick = if arm.rep_ticks[best_i] > 0 { arm.rep_lag[best_i] as f64 / arm.rep_ticks[best_i] as f64 } else { 0.0 };
        let (cached, fresh) = arm.rep_spec[best_i];
        let spec_us = arm.rep_spec_ns[best_i] as f64 / 1000.0 / frames as f64;
        let cached_pct = if cached + fresh > 0 { 100.0 * cached as f64 / (cached + fresh) as f64 } else { f64::NAN };
        println!(
            "{:>6} {:>12} {:>9} {:>7} {:>8.0} {:>7} {:>8} {:>10.1} {:>8.2} {:>9.1} {:>8.1} {:>6.1} {:>8.1} {:>10.0} {:>10.0} {:>10.3} {:>9} {:>8.3} {:>9.2} {:>7.1} {:>8.1} {:>10.0} {:>10.0}",
            arm.want,
            format!("{}x{}f{}", arm.bw, arm.bh, arm.nf),
            format!("{:?}", arm.par),
            arm.stocked,
            ants,
            arm.rep_plants[best_i],
            arm.rep_pcells[best_i],
            best,
            best / med,
            crticks,
            moves,
            blk,
            arm.rep_awake[best_i] as f64 / frames as f64,
            arm.rep_swept[best_i] as f64 / frames as f64,
            arm.rep_sites[best_i] as f64 / frames as f64,
            lag_per_tick,
            arm.rep_lagmax[best_i],
            per,
            us[us.len() - 1] / best,
            cached_pct,
            spec_us,
            arm.rep_swvisited[best_i] as f64 / frames as f64,
            arm.rep_swsoil[best_i] as f64 / frames as f64
        );
        pts.push((arm.par, (arm.bw, arm.bh, arm.nf), ants, best));
    }

    // **The gate, printed per arm and read across them.** Arms of one ant
    // count must agree bit for bit; see `world_hash`.
    println!("\n  world hash after the run, by arm (arms of one ant count must match):");
    for arm in &arms {
        println!("    want {:>5} bed {:>10} par {:>9} -> {:#018x}", arm.want, format!("{}x{}f{}", arm.bw, arm.bh, arm.nf), format!("{:?}", arm.par), world_hash(&arm.lab.world));
    }

    // **Ordinary least squares over the arms' lower envelope**, which is the
    // same fit the playtest report ran on the owner's octiles. Two arms is
    // the minimum that defines a line and three is the minimum that can
    // disagree with one.
    // **One fit per `par` arm, never one fit across them.** Pooling a serial
    // and a parallel arm into one regression measures their average and
    // describes neither -- the same error the round-32 report had to repair
    // when it fitted one line through the owner's two regimes.
    // **One fit per (bed, par) pair, never one across beds either.** A wider
    // bed is a different intercept — more cells to sweep, more soil for the
    // moisture pass — so pooling two bed sizes fits a line through two
    // backgrounds and calls the difference an ant.
    for &(bw, bh, nf) in &beds {
    for &mode in &pars {
        let pts: Vec<(f64, f64)> = pts.iter().filter(|(m, b, _, _)| *m == mode && *b == (bw, bh, nf)).map(|&(_, _, x, y)| (x, y)).collect();
        if pts.len() < 2 {
            continue;
        }
        let n = pts.len() as f64;
        let (sx, sy): (f64, f64) = pts.iter().fold((0.0, 0.0), |(a, b), (x, y)| (a + x, b + y));
        let (mx, my) = (sx / n, sy / n);
        let sxy: f64 = pts.iter().map(|(x, y)| (x - mx) * (y - my)).sum();
        let sxx: f64 = pts.iter().map(|(x, _)| (x - mx) * (x - mx)).sum();
        if sxx > 0.0 {
            let slope = sxy / sxx;
            let intercept = my - slope * mx;
            println!(
                "\n  fit [{bw}x{bh} f{nf} {mode:?}]: cost ≈ {:.0} µs/tick + {:.3} µs per ant per tick   ({} arms)",
                intercept,
                slope,
                pts.len()
            );
            // Residuals, because a two-point fit through a noisy intercept
            // can reproduce a slope it has not measured.
            print!("  residuals (measured - fitted, µs/tick):");
            for (x, y) in &pts {
                print!(" {:+.0}", y - (intercept + slope * x));
            }
            println!();
        } else {
            println!("\n  fit [{bw}x{bh} f{nf} {mode:?}]: every arm reports the same ant count -- nothing to regress");
        }
    }
    }
    println!("  playtest §1, on the owner's own wall clock: ≈ 1000 µs/tick + 2.100 µs per ant per tick");
    let (absent, moved, dirty, dirty_org, dirty_field) = creature::speculation_misses();
    if absent + moved + dirty > 0 {
        println!("  speculation misses over the whole run: no speculation {absent}, moved or turned {moved}, written into {dirty} (of which state {dirty_org}, field {dirty_field})");
    }
}
