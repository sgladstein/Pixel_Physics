//! **The shared performance benchmark: one grown lab box, a timed window, and
//! the hashes that say a speedup changed nothing.**
//!
//! Built 2026-10-21 for the four performance threads (lab world, creatures,
//! plants, training), so that every claimed gain is measured on the same
//! scenes the same way. A speed change here must not change results --
//! training depends on bit-for-bit reproduction -- so the hashes are not an
//! extra: **a timing beside hashes that moved is not a speedup, it is a
//! different simulation.**
//!
//! What one run does: build the box (`LabBox::default()`, or a scenario by
//! name), tick it with `Lab::tick_for_harness` -- the game's own tick, with
//! `regroup_by_scent` and the page observers in it, on the rayon pool as
//! `Lab::advance` runs it -- to frame `at`, then time `window` more ticks.
//! It prints one `RESULT` line:
//!
//! | field | |
//! |---|---|
//! | `ms` | wall milliseconds per tick over the window |
//! | `cells` | order-sensitive digest of every cell in bounds (`frame.rs`'s test digest, same fields) |
//! | `field` | `field::field_hash`, because a field-only change cannot move the grid for many frames |
//! | `rng` | the world generator's next draw: a change that consumes one draw more or less shows here first |
//! | `orgs` | live organisms, plants and animals together |
//!
//! **Compare builds by alternating processes, not one after the other**
//! (`scripts/perfbench.sh` does it): a busy machine moves a wall timing by
//! tens of percent, and min-over-alternating-reps is the shape that survives
//! it. For a number that does not move with load, run it under callgrind:
//! `cg=1` brackets the window with client requests (the `needscost` trick),
//! so with `--instr-atstart=no` only the window is instrumented.
//!
//! ```text
//! cargo run --release --example perfbench                         # shipped box, at=20000 window=1200
//! cargo run --release --example perfbench -- scenario=nest_goal at=100000
//! RAYON_NUM_THREADS=1 valgrind --tool=callgrind --instr-atstart=no \
//!     target/release/examples/perfbench at=20000 window=1200 cg=1
//! ```

use std::time::Instant;

use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::Lab;
use pixel_physics::sim::field;
use pixel_physics::sim::parallel;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

/// `frame.rs`'s test digest, field for field: what it has to catch is a
/// phase running differently, which moves cells rather than counts.
fn cell_hash(w: &World) -> u64 {
    fn fnv1a(h: u64, v: u64) -> u64 {
        (h ^ v).wrapping_mul(0x0000_0100_0000_01b3)
    }
    let b = w.bounds().expect("the lab box sets bounds");
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            h = fnv1a(h, c.material.0 as u64);
            h = fnv1a(h, c.shade as u64);
            h = fnv1a(h, c.aux() as u64);
            h = fnv1a(h, c.organism_id() as u64);
            h = fnv1a(h, c.temperature() as u16 as u64);
            h = fnv1a(h, c.burn_remaining() as u64);
            let flags = (c.is_burning() as u64)
                | (c.flowing() as u64) << 1
                | (c.undercut() as u64) << 2
                | (c.attached() as u64) << 3
                | (c.managed() as u64) << 4;
            h = fnv1a(h, flags);
        }
    }
    h
}

/// A callgrind client request, inline -- copied from `needscost`, whose doc
/// carries the why. Natively two no-op instructions.
#[cfg(target_arch = "x86_64")]
fn callgrind(request: u64, arg1: u64) {
    let args: [u64; 6] = [request, arg1, 0, 0, 0, 0];
    let mut result: u64 = 0;
    // SAFETY: the rotations sum to a whole turn and the exchange is with
    // itself, so every register is left as found; under valgrind it reads
    // `args` through `rax` and writes `rdx`.
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

const CG_ZERO_STATS: u64 = 0x4354_0001;
const CG_START_INSTRUMENTATION: u64 = 0x4354_0004;
const CG_STOP_INSTRUMENTATION: u64 = 0x4354_0005;

fn main() {
    let scenario: Option<String> = arg("scenario");
    let at: u64 = arg("at").unwrap_or(20_000);
    let window: u64 = arg("window").unwrap_or(1_200);
    let seed: Option<u64> = arg("seed");
    let cg: u32 = arg("cg").unwrap_or(0);

    let mut lab = match &scenario {
        Some(name) => {
            let mut sc = Scenario::load(name).unwrap_or_else(|e| {
                eprintln!("scenario {name}: {e}");
                std::process::exit(1);
            });
            if let Some(s) = seed {
                sc.bed.seed = s;
            }
            let mut lab = Lab::new(sc.bed.clone());
            let _ = lab.load_scenario(sc);
            lab
        }
        None => {
            let mut spec = LabBox::default();
            if let Some(s) = seed {
                spec.seed = s;
            }
            Lab::new(spec)
        }
    };
    let threads = rayon::current_num_threads();
    println!(
        "perfbench scenario={} seed={} at={at} window={window} threads={threads} cg={cg}",
        scenario.as_deref().unwrap_or("shipped"),
        lab.spec.seed,
    );

    let pre = Instant::now();
    parallel::on_pool(|| {
        while lab.world.frame < at {
            lab.tick_for_harness();
        }
    });
    let pre_s = pre.elapsed().as_secs_f64();

    if cg == 1 {
        callgrind(CG_ZERO_STATS, 0);
        callgrind(CG_START_INSTRUMENTATION, 0);
    }
    let t = Instant::now();
    parallel::on_pool(|| {
        for _ in 0..window {
            lab.tick_for_harness();
        }
    });
    let ms = t.elapsed().as_secs_f64() * 1000.0 / window as f64;
    if cg == 1 {
        callgrind(CG_STOP_INSTRUMENTATION, 0);
    }

    let w = &lab.world;
    let mut rng = w.rng.clone();
    println!(
        "RESULT ms={ms:.4} cells={:#018x} field={:#018x} rng={:#018x} orgs={} frame={} pre_s={pre_s:.1}",
        cell_hash(w),
        field::field_hash(w),
        rng.next_u64(),
        w.live_organism_count(),
        w.frame,
    );
}
