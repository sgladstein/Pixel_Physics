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
use pixel_physics::lab::Lab;
use std::time::Instant;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// One population under test, with its own live box.
struct Arm {
    want: usize,
    lab: Lab,
    /// What the stocking loop actually got onto the bed.
    stocked: usize,
    /// Summed `Lab` ticks and summed nanoseconds, per rep.
    rep_ns: Vec<u128>,
    /// Mean standing ant count over each rep's timed window.
    rep_ants: Vec<f64>,
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
    let frames: u64 = arg("frames").unwrap_or(400);
    let reps: usize = arg("reps").unwrap_or(3);
    let seed: u64 = arg("seed").unwrap_or(1);
    let d = LabBox::default();
    let width: i32 = arg("width").unwrap_or(d.width);
    // **512, because that is what the owner raised the box to during setup**
    // and the log header states it. Not the `LabBox` default.
    let height: i32 = arg("height").unwrap_or(512);
    let soil: i32 = arg("soil").unwrap_or(d.soil_depth);
    let founders: usize = arg("founders").unwrap_or(d.founders);
    let species: String = arg("species").unwrap_or_else(|| d.species.clone());
    let colony_species: String = arg("colony_species").unwrap_or_else(|| "longant".to_string());
    // Frames of plant growth before any ant arrives, so the intercept is
    // measured over a bed with something living in it rather than over eight
    // seeds. The played bed had 264–409 plants.
    let grow: u64 = arg("grow").unwrap_or(6_000);
    let rounds: usize = arg("rounds").unwrap_or(200);
    let settle: u64 = arg("settle").unwrap_or(40);
    // `PLANT_LOAD_FAILURE false` — the one dial off in the played session.
    let plant_load: bool = arg::<u32>("plant_load").unwrap_or(0) == 1;

    let threads = std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "unset".to_string());
    let sched = std::env::var("SCHED_PASS").unwrap_or_else(|_| "0".to_string());
    println!(
        "antcost: ants={wants:?} frames={frames} reps={reps} seed={seed} width={width} height={height} \
         soil={soil} founders={founders} species={species} colony_species={colony_species} grow={grow} \
         rounds={rounds} settle={settle} plant_load={plant_load} RAYON_NUM_THREADS={threads} SCHED_PASS={sched}"
    );

    let spec = LabBox {
        width,
        height,
        soil_depth: soil,
        founders,
        species: species.clone(),
        // **Founded by the stocking loop, never by the scene**, so the
        // `ants=0` arm is an identical bed with nothing standing in it
        // rather than a differently-built one.
        colonies: 0,
        colony_species: colony_species.clone(),
        seed,
        ..d
    };
    let ground_y = spec.ground_y;

    let mut arms: Vec<Arm> = Vec::new();
    for &want in &wants {
        let mut lab = Lab::new(spec.clone());
        lab.world.plant_load_failure = plant_load;
        for _ in 0..grow {
            lab.tick_for_harness();
        }
        let stocked = stock(&mut lab, &colony_species, want, ground_y, width, rounds, settle);
        println!(
            "  stocking: want {want:>5} -> standing {stocked:>5} ants, {:>5} plants, frame {}",
            lab.world.live_organism_count() - stocked,
            lab.world.frame
        );
        arms.push(Arm {
            want,
            lab,
            stocked,
            rep_ns: Vec::new(),
            rep_ants: Vec::new(),
            rep_ticks: Vec::new(),
            rep_moves: Vec::new(),
            rep_blocked: Vec::new(),
        });
    }

    // **Round-robin, and the reps interleave rather than nest per arm.** A
    // machine that gets busy halfway through a run slows whichever arm is in
    // front of it; interleaving spreads that over all of them, and taking the
    // minimum over reps then reads the quietest window each arm saw.
    for rep in 0..reps {
        for arm in arms.iter_mut() {
            let ants_before = arm.lab.world.live_creature_count();
            let ticks_before = arm.lab.world.creature_stats.ticks;
            let moves_before = arm.lab.world.creature_stats.moves;
            let blocked_before = arm.lab.world.creature_stats.moves_blocked;
            let t = Instant::now();
            for _ in 0..frames {
                arm.lab.tick_for_harness();
            }
            let ns = t.elapsed().as_nanos();
            let ants_after = arm.lab.world.live_creature_count();
            arm.rep_ns.push(ns);
            arm.rep_ants.push((ants_before + ants_after) as f64 / 2.0);
            arm.rep_ticks.push(arm.lab.world.creature_stats.ticks - ticks_before);
            arm.rep_moves.push(arm.lab.world.creature_stats.moves - moves_before);
            arm.rep_blocked.push(arm.lab.world.creature_stats.moves_blocked - blocked_before);
        }
        eprintln!("  rep {}/{reps} done", rep + 1);
    }

    println!(
        "\n{:>6} {:>7} {:>8} {:>10} {:>10} {:>9} {:>9} {:>7}",
        "want", "stocked", "ants", "µs/tick", "min/med", "crtick/f", "µs/ant", "spread"
    );
    // Points for the fit: (mean ants over the quietest rep, µs/tick).
    let mut pts: Vec<(f64, f64)> = Vec::new();
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
        println!(
            "{:>6} {:>7} {:>8.0} {:>10.1} {:>10.2} {:>9.1} {:>8.1} {:>6.1} {:>9.3} {:>7.2}",
            arm.want,
            arm.stocked,
            ants,
            best,
            best / med,
            crticks,
            moves,
            blk,
            per,
            us[us.len() - 1] / best
        );
        pts.push((ants, best));
    }

    // **Ordinary least squares over the arms' lower envelope**, which is the
    // same fit the playtest report ran on the owner's octiles. Two arms is
    // the minimum that defines a line and three is the minimum that can
    // disagree with one.
    if pts.len() >= 2 {
        let n = pts.len() as f64;
        let (sx, sy): (f64, f64) = pts.iter().fold((0.0, 0.0), |(a, b), (x, y)| (a + x, b + y));
        let (mx, my) = (sx / n, sy / n);
        let sxy: f64 = pts.iter().map(|(x, y)| (x - mx) * (y - my)).sum();
        let sxx: f64 = pts.iter().map(|(x, _)| (x - mx) * (x - mx)).sum();
        if sxx > 0.0 {
            let slope = sxy / sxx;
            let intercept = my - slope * mx;
            println!(
                "\n  fit: cost ≈ {:.0} µs/tick + {:.3} µs per ant per tick   ({} arms)",
                intercept,
                slope,
                pts.len()
            );
            println!("  playtest §1, on the owner's own wall clock: ≈ 1000 µs/tick + 2.100 µs per ant per tick");
            // Residuals, because a two-point fit through a noisy intercept
            // can reproduce a slope it has not measured.
            print!("  residuals (measured - fitted, µs/tick):");
            for (x, y) in &pts {
                print!(" {:+.0}", y - (intercept + slope * x));
            }
            println!();
        } else {
            println!("\n  fit: every arm reports the same ant count -- nothing to regress");
        }
    }
}
