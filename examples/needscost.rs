//! **What the ant's decision costs in the goal box, at the size a colony
//! actually reaches** -- the frame-cost half of the needs walk's slice 0
//! (`creature::needs`), and the bar its later slices are priced against.
//!
//! The figure the design started from was "today's chooser is about 9% of the
//! frame", and it was the wrong scene: README's `ascii scene=foraging`, a
//! hand-built nest with 60 ants, against a goal-box colony of 371-460 ants at
//! frame 50,000 and 560-616 at 300,000 (the needs walk's engineering review,
//! E6). This runs the box the walk will run in -- `nest_goal`, built and fed
//! exactly as `deeptrace` builds and feeds it, so the colony at `at=` is the
//! baseline's colony -- to a frame where the colony is grown, then times a
//! window of whole frames.
//!
//! **Two readings, for two questions.**
//!
//! - **Native**, it prints whole-frame milliseconds per arm, the arms
//!   round-robin inside one process and the published figure the minimum over
//!   reps (`antcost`'s three rules, for the reason `antcost` gives: contention
//!   can only ever make a window slower). `arms=off,passthrough` puts the
//!   needs walk's hand-over switch beside today's ant; each arm is its own
//!   `Lab`, built and run to `at=` from the same seed, so the two are the same
//!   world up to the switch.
//! - **Under callgrind** (`cg=1`), it switches instrumentation on for each
//!   arm's window only and dumps one profile per window, labelled with the
//!   arm. That is how the frame is split *inside* the creature pass --
//!   sensing, the brain, `act`, the step and the chooser -- which no timing
//!   site in the tree does, and it is a count of instructions rather than a
//!   clock, so it is the same on a busy box. Instructions are not time: a
//!   phase that waits on memory costs more time than its share of
//!   instructions says, which is why the native whole-frame figure is printed
//!   beside it and not replaced by it.
//!
//! ```text
//! RAYON_NUM_THREADS=1 cargo run --release --example needscost -- seed=1 at=100000 window=1200 reps=3
//! RAYON_NUM_THREADS=1 valgrind --tool=callgrind --instr-atstart=no \
//!     target/release/examples/needscost seed=1 at=100000 window=1200 reps=1 cg=1
//! callgrind_annotate --inclusive=yes callgrind.out.<pid>.1 | head -60
//! ```
//!
//! The pre-run under valgrind is uninstrumented and costs about what
//! valgrind's own translation costs, so a 100,000-frame run is minutes, not
//! hours; the window is instrumented. Build with
//! `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only` (into its own
//! `--target-dir`) when the split is wanted by source line: thin LTO inlines
//! most of the tick into `creature_tick`, and only the line tables keep the
//! inlined code's own lines.
//!
//! Echoes its own parameters; an unknown argument is ignored.

use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::Lab;
use pixel_physics::sim::cell::Cell;
use pixel_physics::sim::creature;
use pixel_physics::sim::material;
use pixel_physics::sim::world::World;
use std::time::Instant;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

/// `deeptrace`'s food heap: topped up every `TOP_EVERY` frames to `target`
/// cells, `FOOD_REACH` columns either side of a point 30 east of the nest.
/// Copied, not shared, and it must stay the same as `deeptrace`'s -- it is
/// what makes the colony at `at=` the baseline's colony.
const TOP_EVERY: u64 = 250;
const FOOD_REACH: i32 = 12;

struct Geo {
    ground_y: i32,
    food_x: i32,
}

fn top_up(w: &mut World, g: &Geo, target: usize) -> usize {
    let provisions = w.materials.id_of("provisions").expect("provisions ships");
    let mut have = 0;
    for y in (g.ground_y - 40)..=(g.ground_y + 4) {
        for x in (g.food_x - FOOD_REACH)..=(g.food_x + FOOD_REACH) {
            have += usize::from(w.in_bounds(x, y) && w.get(x, y).material == provisions);
        }
    }
    if have >= target {
        return 0;
    }
    let mut want = target - have;
    let mut dropped = 0;
    let top = g.ground_y - 30;
    'rows: for y in top..(top + 6) {
        for k in 0..=4 {
            for x in [g.food_x - k, g.food_x + k] {
                if want == 0 {
                    break 'rows;
                }
                if w.in_bounds(x, y) && w.get(x, y).material == material::EMPTY {
                    w.set(x, y, Cell::new(provisions, 0));
                    want -= 1;
                    dropped += 1;
                }
            }
        }
    }
    dropped
}

/// **A callgrind client request, inline.** The magic sequence `valgrind.h`
/// emits on amd64: four rotations of `rdi` that sum to 128 bits (a no-op) and
/// an `xchg rbx, rbx` (a no-op), which valgrind's translator recognises and
/// turns into a request read from `[rax]`. Natively it does nothing and
/// returns `0`, so the harness needs no flag to run outside valgrind; `cg=1`
/// only stops it making the calls at all.
#[cfg(target_arch = "x86_64")]
fn callgrind(request: u64, arg1: u64) {
    let args: [u64; 6] = [request, arg1, 0, 0, 0, 0];
    let mut result: u64 = 0;
    // SAFETY: the sequence leaves every register as it found it (the
    // rotations sum to a whole turn and the exchange is with itself); it
    // reads `args` through `rax` and, under valgrind, writes `rdx`.
    unsafe {
        std::arch::asm!(
            "rol rdi, 3",
            "rol rdi, 13",
            "rol rdi, 61",
            "rol rdi, 51",
            "xchg rbx, rbx",
            inout("rdx") result,
            in("rax") args.as_ptr(),
            options(nostack),
        );
    }
    let _ = result;
}

#[cfg(not(target_arch = "x86_64"))]
fn callgrind(_request: u64, _arg1: u64) {}

/// `callgrind.h`'s request numbers: `VG_USERREQ_TOOL_BASE('C','T')` and on.
const CG_ZERO_STATS: u64 = 0x4354_0001;
const CG_DUMP_STATS_AT: u64 = 0x4354_0003;
const CG_START_INSTRUMENTATION: u64 = 0x4354_0004;
const CG_STOP_INSTRUMENTATION: u64 = 0x4354_0005;

/// One arm: its own box, ticked exactly as `deeptrace` ticks it.
struct Arm {
    label: String,
    needs: Option<creature::needs::NeedsMode>,
    lab: Lab,
    geo: Option<Geo>,
    f: u64,
    target: usize,
}

impl Arm {
    fn new(label: &str, seed: u64, target: usize) -> Arm {
        let mut sc = Scenario::load("nest_goal").unwrap_or_else(|e| {
            eprintln!("scenario nest_goal: {e}");
            std::process::exit(1);
        });
        sc.bed.seed = seed;
        let mut lab = Lab::new(sc.bed.clone());
        let _ = lab.load_scenario(sc);
        lab.show_help = false;
        if lab.stats.showing() {
            lab.stats.toggle();
        }
        let needs = match label {
            "off" => None,
            other => Some(creature::needs::NeedsMode::parse(other)),
        };
        Arm {
            label: label.to_string(),
            needs,
            lab,
            geo: None,
            f: 0,
            target,
        }
    }

    /// One frame, in `deeptrace`'s order: the heap is found once the nest is,
    /// topped up on the frame before the tick, and the needs walk is handed
    /// the colony on the frame `at` names.
    fn step(&mut self, at: u64) {
        if self.geo.is_none() {
            if let Some(s) = self.lab.world.nest_sites.first() {
                self.geo = Some(Geo {
                    ground_y: self.lab.spec.ground_y,
                    food_x: s.x + 30,
                });
            }
        }
        if let Some(g) = self.geo.as_ref() {
            if self.f.is_multiple_of(TOP_EVERY) {
                top_up(&mut self.lab.world, g, self.target);
            }
            if let Some(mode) = self.needs.filter(|_| self.f == at) {
                self.lab.world.needs = Some(Box::new(creature::needs::NeedsWalk::new(mode, self.lab.world.frame)));
            }
        }
        self.lab.tick_for_harness();
        self.f += 1;
    }

    /// Ants standing (not brood), counted as `deeptrace`'s `is_ant` does.
    fn ants(&self) -> usize {
        let w = &self.lab.world;
        let Some(sid) = w.species.id_of("ant") else {
            return 0;
        };
        w.live_organism_ids()
            .into_iter()
            .filter(|&id| {
                w.organism(id)
                    .is_some_and(|s| s.species == sid && s.brood.is_none() && !s.chain.is_empty())
            })
            .count()
    }
}

fn main() {
    creature::mutation_off_for_measuring();
    let seed: u64 = arg("seed").unwrap_or(1);
    let at: u64 = arg("at").unwrap_or(100_000);
    let window: u64 = arg("window").unwrap_or(1_200);
    let reps: usize = arg("reps").unwrap_or(3);
    let target: usize = arg("food").unwrap_or(120);
    let cg = arg::<u8>("cg").unwrap_or(0) == 1;
    let arms_arg: String = arg("arms").unwrap_or_else(|| "off".to_string());
    let threads = std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "unset".to_string());
    println!(
        "needscost: scenario=nest_goal seed={seed} at={at} window={window} reps={reps} food={target} arms={arms_arg} cg={} RAYON_NUM_THREADS={threads}",
        u8::from(cg)
    );
    let mut arms: Vec<Arm> = arms_arg.split(',').map(|a| Arm::new(a.trim(), seed, target)).collect();
    let started = Instant::now();
    for arm in arms.iter_mut() {
        while arm.f < at {
            arm.step(at);
        }
        println!(
            "  {} at frame {}: {} ants, {} organisms, world frame {}, nest {}",
            arm.label,
            arm.f,
            arm.ants(),
            arm.lab.world.live_organism_ids().len(),
            arm.lab.world.frame,
            if arm.geo.is_some() { "founded" } else { "NOT FOUNDED" }
        );
    }
    println!("  reached frame {at} in {:.0}s", started.elapsed().as_secs_f64());
    // **Round-robin, reps interleaved**, as `antcost` does: a minimum over
    // reps reads the quietest window each arm saw.
    let mut ms: Vec<Vec<f64>> = vec![Vec::new(); arms.len()];
    let mut census: Vec<Vec<usize>> = vec![Vec::new(); arms.len()];
    for rep in 0..reps {
        for (k, arm) in arms.iter_mut().enumerate() {
            let before = arm.ants();
            if cg {
                callgrind(CG_ZERO_STATS, 0);
                callgrind(CG_START_INSTRUMENTATION, 0);
            }
            let t0 = Instant::now();
            for _ in 0..window {
                arm.step(at);
            }
            let dt = t0.elapsed().as_secs_f64();
            if cg {
                let name = std::ffi::CString::new(format!("{}-rep{rep}-f{}", arm.label, arm.f)).expect("no nul");
                callgrind(CG_DUMP_STATS_AT, name.as_ptr() as u64);
                callgrind(CG_STOP_INSTRUMENTATION, 0);
            }
            let after = arm.ants();
            ms[k].push(dt * 1000.0 / window as f64);
            census[k].push((before + after) / 2);
            println!(
                "  rep {rep} {:<12} frames {}..{}: {:.3} ms/frame, ants {before} -> {after}",
                arm.label,
                arm.f - window,
                arm.f,
                dt * 1000.0 / window as f64
            );
        }
    }
    println!("arm           min ms/frame  median ms/frame  mean ants");
    for (k, arm) in arms.iter().enumerate() {
        let mut v = ms[k].clone();
        v.sort_by(f64::total_cmp);
        let ants = census[k].iter().sum::<usize>() as f64 / census[k].len().max(1) as f64;
        println!(
            "{:<13} {:>12.3}  {:>15.3}  {:>9.0}",
            arm.label,
            v[0],
            v[v.len() / 2],
            ants
        );
    }
}
