//! How often is this machine actually quiet, and for how long at a stretch?
//!
//! The question that decides how the timing harness should behave. A
//! wait-for-quiet gate needs a maximum wait, and a maximum wait picked
//! without knowing the gap distribution is an aspiration, not a bar
//! (`CLAUDE.md`: set bars from measurement with headroom). If quiet windows
//! are common and long, waiting is nearly free; if they are rare and short,
//! waiting mostly burns time and the run should be stamped untrusted and got
//! on with.
//!
//! Cheap enough to leave running: one `Machine::probe()` is nine short
//! all-core bursts plus a process listing, a few tens of milliseconds. At the
//! default 30 s interval that is well under a tenth of a percent of one core,
//! so the sampler does not meaningfully disturb what it is sampling — but it
//! is not free either, which is why it is a separate example you start
//! deliberately rather than something the harness does in the background.
//!
//! ```text
//! cargo run --release --example quiet_probe -- minutes=45
//! cargo run --release --example quiet_probe -- minutes=5 every=10
//! ```
//!
//! Prints one line per sample and a summary: the fraction of samples under
//! the busy threshold, the longest unbroken quiet spell, and the longest
//! unbroken busy spell. The last of those is the number a max-wait has to be
//! set against.

use pixel_physics::perf::Machine;
use std::time::{Duration, Instant};

fn main() {
    let mut minutes = 30.0f64;
    let mut every = 30.0f64;
    for arg in std::env::args().skip(1) {
        let Some((key, value)) = arg.split_once('=') else { continue };
        match key {
            "minutes" => minutes = value.parse().expect("minutes"),
            "every" => every = value.parse().expect("every"),
            other => panic!("unknown argument `{other}` -- expected minutes= or every="),
        }
    }

    // Deliberately does *not* take the timing lock. This is a passive
    // observer of the machine; taking the lock would block a real
    // measurement for the whole sampling window, and would also mean the
    // sampler could never observe the machine during a harness run, which is
    // one of the states worth knowing about.
    println!("sampling every {every:.0} s for {minutes:.0} min -- factor, then competitors");
    let started = Instant::now();
    let deadline = Duration::from_secs_f64(minutes * 60.0);
    let interval = Duration::from_secs_f64(every);

    let mut samples: Vec<(f64, bool)> = Vec::new();
    while started.elapsed() < deadline {
        let m = Machine::probe();
        let quiet = !m.is_busy();
        samples.push((m.factor, quiet));
        println!(
            "{:>6.0}s  {:>6.2}x  {:<7} {}",
            started.elapsed().as_secs_f64(),
            m.factor,
            if quiet { "quiet" } else { "BUSY" },
            m.competitors.join(", ")
        );
        std::thread::sleep(interval);
    }

    if samples.is_empty() {
        println!("no samples -- minutes= was too small for one interval");
        return;
    }

    // Run lengths, in samples, converted to seconds by the interval. The
    // longest *busy* spell is the operative number: it is how long a
    // wait-for-quiet gate would have had to wait, at worst, to get a clean
    // window during this observation.
    let (mut longest_quiet, mut longest_busy, mut run, mut run_quiet) = (0usize, 0usize, 0usize, samples[0].1);
    for &(_, quiet) in &samples {
        if quiet == run_quiet {
            run += 1;
        } else {
            if run_quiet {
                longest_quiet = longest_quiet.max(run);
            } else {
                longest_busy = longest_busy.max(run);
            }
            run = 1;
            run_quiet = quiet;
        }
    }
    if run_quiet {
        longest_quiet = longest_quiet.max(run);
    } else {
        longest_busy = longest_busy.max(run);
    }

    let quiet_count = samples.iter().filter(|&&(_, q)| q).count();
    let mut factors: Vec<f64> = samples.iter().map(|&(f, _)| f).collect();
    factors.sort_by(|a, b| a.partial_cmp(b).expect("factors are never NaN"));

    println!();
    println!("=== {} samples over {:.0} min at {every:.0} s ===", samples.len(), minutes);
    println!(
        "quiet (under the busy threshold): {quiet_count}/{} = {:.0}%",
        samples.len(),
        100.0 * quiet_count as f64 / samples.len() as f64
    );
    println!(
        "factor: min {:.2}x, median {:.2}x, p90 {:.2}x, max {:.2}x",
        factors[0],
        factors[factors.len() / 2],
        factors[factors.len() * 9 / 10],
        factors[factors.len() - 1]
    );
    println!("longest unbroken quiet spell: {:.0} s", longest_quiet as f64 * every);
    println!("longest unbroken busy spell:  {:.0} s  <-- a wait-for-quiet max-wait has to be read against this", longest_busy as f64 * every);
}
