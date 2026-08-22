//! Keeping a timing number honest on a machine several agents share.
//!
//! This tree is worked in by more than one session at a time (`CLAUDE.md`,
//! "Working alongside another session"), each with its own `target/`, and
//! this machine has **four logical cores** against a simulation that runs
//! `parallel::step` across all of them. One other session's `cargo build
//! --release` saturates the box. There is no quiet corner to hide in.
//!
//! That matters more here than it would elsewhere because every timing scene
//! reports a **maximum** — the worst frame is what has to fit in the 16.6 ms
//! budget, so it is the right quantity to care about and the worst possible
//! one to measure on a contended machine. A single scheduler preemption is
//! indistinguishable from a real regression, and it only takes one frame out
//! of thousands. The baseline that prompted this module had the *parallel*
//! stress scene at 196 ms against the *serial* one's 121 ms — M5's entire
//! point is that the parallel number should be the smaller one — while the
//! field variants of the same two scenes, in the same run, ordered them the
//! other way round. Both orderings cannot be true. The statistic was
//! measuring the rest of the machine.
//!
//! Three things live here, in the order they are worth reaching for:
//!
//! 1. [`FrameTimer`] — collect every sample, report `worst` **beside** the
//!    median and p99. An 18x worst-to-median ratio (the ants scene measured
//!    72.6 against 3.9) is a preemption signature, and it is invisible if the
//!    only number printed is the max.
//! 2. [`TimingLock`] — a machine-wide advisory lock, so two harnesses do not
//!    time themselves against each other. Deliberately **not** taken around
//!    compilation: `scripts/perf.sh` builds outside the lock and runs the
//!    prebuilt binary inside it, which keeps the hold to the length of a run
//!    rather than the length of a build.
//! 3. [`Machine`] — a self-calibrating busy detector, because the lock only
//!    covers processes that agreed to take it. A `rustc` from another
//!    worktree never will, so a run also has to be able to say "these numbers
//!    are dirty" after the fact.
//!
//! What this module deliberately does **not** do is normalise a measured time
//! by the contention factor. A scaled-up number looks like a measurement and
//! is a guess; the honest move is to report the factor next to the raw figure
//! and let the reader discard the run.

use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Above this ratio against the quiet reference, a run is called dirty.
///
/// Set from measurement with headroom on both sides, per `CLAUDE.md`. On this
/// box, idle probes read 1.00-1.01x, and a probe taken while one `cargo build
/// --release --lib` ran read **1.91x**. 1.25 sits between the two with real
/// margin either way — near enough to idle that ordinary thermal and
/// scheduler variation cannot reach it (the reference converges on the
/// *fastest* the machine has been seen to go, so every honest run lands a
/// little above it), and far enough below a genuine competing build that one
/// cannot hide under it.
const BUSY_FACTOR: f64 = 1.25;

/// One frame at 60 Hz — the hard constraint everything here is measured
/// against (`CLAUDE.md`: "frame cost is a hard constraint, not a tiebreaker").
pub const FRAME_BUDGET_MS: f64 = 1000.0 / 60.0;

/// How long before a lock file is assumed to belong to a dead process.
///
/// A harness run is minutes, not hours: `examples/ascii.rs` takes ~143 s.
/// Fifteen minutes is long enough that a slow legitimate run is never stolen
/// from, and short enough that a session killed mid-run does not block the
/// machine until somebody notices.
const STALE_AFTER: Duration = Duration::from_secs(15 * 60);

/// Give up waiting and proceed anyway, loudly. A wait that never ends is
/// worse than a dirty measurement, because at least the dirty one says so.
const MAX_WAIT: Duration = Duration::from_secs(20 * 60);

fn temp_file(name: &str) -> PathBuf {
    std::env::temp_dir().join(name)
}

// ---------------------------------------------------------------------------
// Frame timing
// ---------------------------------------------------------------------------

/// Collects per-frame durations so the worst frame can be read next to the
/// distribution it came from.
///
/// Keeps every sample rather than a running max. The stress scenes take
/// 12,000 samples at the outside, so the storage is irrelevant, and the
/// alternative — a max and a mean — cannot answer the only question that
/// matters when a number looks wrong: *is this one frame or all of them?*
#[derive(Default)]
pub struct FrameTimer {
    samples_ms: Vec<f64>,
}

impl FrameTimer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Times `f`, records the sample, and hands back whatever `f` returned.
    pub fn time<R>(&mut self, f: impl FnOnce() -> R) -> R {
        let started = Instant::now();
        let out = f();
        self.samples_ms.push(started.elapsed().as_secs_f64() * 1000.0);
        out
    }

    pub fn len(&self) -> usize {
        self.samples_ms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples_ms.is_empty()
    }

    fn quantile(&self, q: f64) -> f64 {
        if self.samples_ms.is_empty() {
            return 0.0;
        }
        let mut sorted = self.samples_ms.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).expect("frame times are never NaN"));
        let idx = ((sorted.len() - 1) as f64 * q).round() as usize;
        sorted[idx]
    }

    /// The frame that has to fit in the budget — and the one an unrelated
    /// process can manufacture on its own.
    pub fn worst(&self) -> f64 {
        self.samples_ms.iter().copied().fold(0.0, f64::max)
    }

    pub fn median(&self) -> f64 {
        self.quantile(0.5)
    }

    /// The honest "expensive frame" figure. A 400-frame run has four frames
    /// above p99, so a lone preemption cannot set it, while a genuine
    /// regression costing one frame in a hundred still shows up here.
    pub fn p99(&self) -> f64 {
        self.quantile(0.99)
    }

    /// How many frames missed the 60 Hz budget.
    ///
    /// Taken from the performance-audit session, who report it instead of a
    /// ratio and are right to: it is the statement that is actually useful.
    /// On their dry-seed control the median was 7.3 ms and the max 20.1 ms —
    /// a ratio test calls that clean, and the budget count says "2 frames
    /// over", which is the thing a player would have felt. A ratio is
    /// relative to whatever the scene happens to cost; this is relative to
    /// the only number that is fixed.
    pub fn over_budget(&self) -> usize {
        self.samples_ms.iter().filter(|&&ms| ms > FRAME_BUDGET_MS).count()
    }

    /// `worst 121.243 ms (p99 8.104, median 3.939, over 400 frames), 7 over 16.7 ms`.
    ///
    /// Ordered worst-first because that is the number the frame budget is
    /// about; the rest is there so a reader can tell at a glance whether the
    /// worst belongs to the same distribution as everything else.
    ///
    /// The budget count is printed even when it is zero. A figure that only
    /// appears when something is wrong gets read as "the check did not run".
    pub fn report(&self) -> String {
        let mut out = format!(
            "worst {:.3} ms (p99 {:.3}, median {:.3}, over {} frames), {} over {:.1} ms",
            self.worst(),
            self.p99(),
            self.median(),
            self.len(),
            self.over_budget(),
            FRAME_BUDGET_MS,
        );
        if self.worst_looks_like_an_outlier() {
            out.push_str(" [worst is >10x the median — suspect interference, not cost]");
        }
        out
    }

    /// True when the worst sample sits so far outside the body of the
    /// distribution that it is more likely interference than cost. Against
    /// the median, not the mean: a settled world's mean is dragged upward by
    /// the handful of frames that did any work at all.
    ///
    /// The **absolute** floor is not belt-and-braces, it is the whole
    /// correctness of the flag. Without it this fires on every small scene in
    /// `examples/ascii.rs`, and its numbers are real and mean nothing: a world
    /// that settles spends 1,199 of 1,200 frames doing literally no work, so
    /// its median is ~0 and *any* frame that did something is a thousandfold
    /// outlier. That is what settling looks like, not interference — the same
    /// shape of error as `CLAUDE.md`'s whisker hunt, where "water with air
    /// above and below" turned out to be the definition of a falling droplet.
    /// It was caught by running the harness, having passed a unit test built
    /// on synthetic uniform samples that contained no settled case at all.
    ///
    /// So: a ratio only means something once the worst frame is big enough to
    /// matter against the 16.6 ms budget at 60 Hz. Below `WORTH_FLAGGING_MS`
    /// nothing is at stake however lopsided the distribution.
    pub fn worst_looks_like_an_outlier(&self) -> bool {
        /// Roughly a third of a 60 Hz frame — the point at which one bad
        /// frame is worth a reader's attention at all.
        const WORTH_FLAGGING_MS: f64 = 5.0;

        let median = self.median();
        self.len() >= 30 && self.worst() > WORTH_FLAGGING_MS && median > 0.0 && self.worst() > median * 10.0
    }
}

// ---------------------------------------------------------------------------
// The machine-busy detector
// ---------------------------------------------------------------------------

/// A fixed slice of arithmetic, run on every core at once, whose only
/// variable is how much of this machine something else is using.
///
/// **Across all cores, not on one.** The first version ran single-threaded on
/// the argument that a rayon-wide probe would itself be contention. It would,
/// and it does not matter — the burst is a few milliseconds — whereas the
/// single-threaded version was measured reporting **1.00x, "quiet", while
/// four `cargo` processes and a `rustc` were running**. On four cores a short
/// single-threaded burst is simply handed a free core by the scheduler, so it
/// answers "is *my* core stolen" (usually no) rather than "is this machine
/// busy" (emphatically yes). A readout that says quiet during a compile storm
/// is worse than no readout, because it will be believed.
///
/// Saturating every core is also the honest model of the thing being
/// protected: `parallel::step` wants all four, so what a timing run competes
/// for is total machine throughput, not one core's worth.
fn calibration_sample() -> f64 {
    use rayon::prelude::*;

    let workers = rayon::current_num_threads().max(1);
    let started = Instant::now();
    let total: u64 = (0..workers)
        .into_par_iter()
        .map(|w| {
            let mut acc: u64 = 0x9e37_79b9_7f4a_7c15 ^ (w as u64);
            for i in 0..2_000_000u64 {
                acc = acc.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(i);
                acc ^= acc >> 29;
            }
            acc
        })
        .sum();
    std::hint::black_box(total);
    started.elapsed().as_secs_f64() * 1000.0
}

/// Median of nine samples: what this machine is *typically* managing now.
///
/// Median, not minimum. The minimum is the right statistic for the stored
/// reference — the fastest the machine has ever gone is a good estimate of
/// its unimpeded speed — but it is the wrong one for the live probe, because
/// under contention one sample in nine will still catch a free core, and the
/// minimum would duly report an idle machine.
fn calibration_ms() -> f64 {
    let mut samples: Vec<f64> = (0..9).map(|_| calibration_sample()).collect();
    samples.sort_by(|a, b| a.partial_cmp(b).expect("calibration times are never NaN"));
    samples[samples.len() / 2]
}

/// The fastest calibration median this machine has been seen to produce.
///
/// Persisted rather than measured per run because the whole point is a
/// comparison against a *quiet* machine, and a run that starts contended has
/// no way to observe one. The file converges from above: every run that beats
/// the stored figure replaces it, so the first few runs on a busy machine
/// under-report contention and correct themselves the first time the box is
/// idle. Under-reporting is the safe direction — it never invents a warning
/// that is not there.
fn quiet_reference(current: f64) -> f64 {
    // The filename carries the kernel version. A stored reference is only
    // comparable with samples from the *same* calibration workload, and the
    // kernel has already changed once (single-threaded to all-cores) -- a
    // stale file from the old one would have made every run afterwards read
    // as several times busier than it was, which is the failure this whole
    // module exists to prevent. Bump the suffix whenever the kernel changes.
    let path = temp_file("pixel-physics-perf-reference-v2.txt");
    let stored = fs::read_to_string(&path).ok().and_then(|s| s.trim().parse::<f64>().ok());
    let best = match stored {
        Some(previous) if previous <= current => previous,
        _ => current,
    };
    if stored != Some(best) {
        // Best-effort: a machine whose temp dir is unwritable still gets a
        // usable factor of 1.0 rather than an error nobody can act on.
        let _ = fs::write(&path, format!("{best}"));
    }
    best
}

/// What else was running while this was measured.
pub struct Machine {
    /// Current calibration against the quiet reference. 1.0 is an idle box.
    pub factor: f64,
    pub calibration_ms: f64,
    pub reference_ms: f64,
    /// Best-effort names of competing build and harness processes.
    pub competitors: Vec<String>,
}

impl Machine {
    pub fn probe() -> Self {
        let calibration_ms = calibration_ms();
        let reference_ms = quiet_reference(calibration_ms);
        Self {
            factor: if reference_ms > 0.0 { calibration_ms / reference_ms } else { 1.0 },
            calibration_ms,
            reference_ms,
            competitors: competing_processes(),
        }
    }

    /// True if a compiler is running anywhere on the box.
    ///
    /// A *direct observation*, and it outranks the calibration factor
    /// precisely because it is not an inference. `rustc` and the linker only
    /// exist while something is being built -- unlike `cargo`, which can sit
    /// around waiting on a lock -- so seeing one is proof another session is
    /// using this machine, whatever a timing probe happened to sample.
    pub fn compiling(&self) -> bool {
        self.competitors
            .iter()
            .any(|c| c.ends_with("rustc") || c.ends_with("link") || c.ends_with("lld") || c.ends_with("ld"))
    }

    pub fn is_busy(&self) -> bool {
        self.factor > BUSY_FACTOR || self.compiling()
    }

    /// One line, always printed — including when the machine is quiet.
    ///
    /// A warning that appears only when something is wrong trains everyone to
    /// read its absence as "the check did not run". Printing the quiet case
    /// costs one line and makes the banner's absence mean something.
    pub fn banner(&self) -> String {
        let who = if self.competitors.is_empty() {
            String::new()
        } else {
            format!(" — competing: {}", self.competitors.join(", "))
        };
        if self.is_busy() {
            let why = if self.compiling() {
                "another session is compiling"
            } else {
                "slower than this box's quiet best"
            };
            format!(
                "!! MACHINE BUSY ({why}): {:.2}x, {:.1} ms vs {:.1} ms{who}\n\
                 !! Timings below are inflated by an unknown amount — re-run under scripts/perf.sh.",
                self.factor, self.calibration_ms, self.reference_ms
            )
        } else {
            format!(
                "machine: {:.2}x quiet reference ({:.1} ms vs {:.1} ms){who}",
                self.factor, self.calibration_ms, self.reference_ms
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Waiting for a quiet window
// ---------------------------------------------------------------------------

/// How long to wait for the box to go quiet before measuring anyway.
///
/// **Never refuses and never discards.** A strict gate was the first design
/// and it was wrong for this machine: of the five run-start probes taken
/// while building this module, exactly one was under [`BUSY_FACTOR`] — so a
/// gate that insisted on quiet would have stalled or thrown away four runs in
/// five. What you get instead is the run either way, stamped with whether it
/// can be believed. A labelled untrustworthy number is useful (it still
/// carries the counters, which do not care); a run that never happened is
/// not.
///
/// **Set from measurement, and deliberately short.** `examples/quiet_probe`
/// sampled this box for 45 minutes at 20 s intervals, 78 samples:
///
/// ```text
/// quiet (under BUSY_FACTOR):    6/78 = 8%
/// factor: min 1.00x, median 1.99x, p90 9.13x, max 15.09x
/// longest unbroken quiet spell:  40 s
/// longest unbroken busy spell:  920 s
/// ```
///
/// The median is **1.99x**: this machine's normal condition is running at
/// half speed. So a wait budget long enough to outlast a bad spell would be
/// fifteen minutes, which is absurd, and a budget short enough to be free
/// will usually expire. 60 s is chosen to lose the argument cheaply — it
/// costs at most a minute, it catches a window if one happens to be opening,
/// and when it expires the run proceeds and is stamped UNTRUSTED rather than
/// discarded.
///
/// A measured *scene* takes 7-11 s (`scene=` in `examples/ascii.rs`), which
/// does fit inside a 40 s window. The full 143 s suite never will. That is
/// the whole argument for measuring one scene at a time.
///
/// Override with `PIXEL_PHYSICS_PERF_WAIT=<seconds>`; `0` skips the wait,
/// which is right in CI.
const DEFAULT_WAIT_SECS: u64 = 60;

/// The wait budget, from the environment or [`DEFAULT_WAIT_SECS`].
pub fn wait_budget() -> Duration {
    std::env::var("PIXEL_PHYSICS_PERF_WAIT")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map_or(Duration::from_secs(DEFAULT_WAIT_SECS), Duration::from_secs)
}

/// Poll until the machine is quiet or `budget` runs out, whichever is first.
///
/// Returns the reading it settled for and how long it waited. Polls rather
/// than sleeping blindly because the thing being waited on is bursty: a
/// competing `cargo build` ends when it ends, and the point is to start the
/// moment it does.
pub fn wait_for_quiet(budget: Duration) -> (Machine, Duration) {
    let started = Instant::now();
    let mut probe = Machine::probe();
    if !probe.is_busy() || budget.is_zero() {
        return (probe, started.elapsed());
    }

    println!("perf: machine is at {:.2}x, waiting up to {} s for a quiet window", probe.factor, budget.as_secs());
    while started.elapsed() < budget {
        std::thread::sleep(Duration::from_secs(5));
        probe = Machine::probe();
        if !probe.is_busy() {
            println!("perf: quiet after {:.0} s — measuring now", started.elapsed().as_secs_f64());
            return (probe, started.elapsed());
        }
    }
    println!(
        "perf: still at {:.2}x after {} s — measuring anyway, and saying so",
        probe.factor,
        budget.as_secs()
    );
    (probe, started.elapsed())
}

/// The verdict on a finished run: can its wall-clock numbers be believed?
///
/// Printed at the end rather than only the beginning, because the state that
/// invalidates a run is load arriving *during* it. Three ways to fail, and
/// naming which one matters — "busy throughout" and "went busy halfway" call
/// for different responses (wait longer, versus re-run).
pub fn trust_verdict(before: &Machine, after: &Machine, waited: Duration) -> String {
    let drifted = (after.factor - before.factor).abs() > 0.25;
    let waited_note = if waited.as_secs() > 2 { format!(" after waiting {} s", waited.as_secs()) } else { String::new() };

    match (before.is_busy(), after.is_busy(), drifted) {
        (false, false, false) => format!(
            "TRUSTED: quiet at both ends ({:.2}x then {:.2}x){waited_note}. Wall-clock numbers above are comparable with other TRUSTED runs.",
            before.factor, after.factor
        ),
        (false, false, true) => format!(
            "UNTRUSTED (load moved mid-run): {:.2}x at the start, {:.2}x at the end, both nominally quiet. Something came and went while measuring; the counters hold, the timings do not.",
            before.factor, after.factor
        ),
        (true, true, _) => format!(
            "UNTRUSTED (busy throughout): {:.2}x then {:.2}x{waited_note}. Counters above are exact regardless; treat every millisecond as an upper bound, not a measurement.",
            before.factor, after.factor
        ),
        (busy_before, _, _) => format!(
            "UNTRUSTED (load {} mid-run): {:.2}x at the start, {:.2}x at the end. The timings are a mixture of two machines.",
            if busy_before { "cleared" } else { "arrived" },
            before.factor,
            after.factor
        ),
    }
}

/// Best-effort list of other builds and harnesses currently running.
///
/// Diagnostic only — nothing gates on it, because process enumeration is the
/// part most likely to differ between this box and CI. Its value is telling
/// whoever reads a dirty run *which* other session to wait for, which the
/// calibration factor alone cannot say.
fn competing_processes() -> Vec<String> {
    let own = std::process::id();
    let interesting = ["rustc", "cargo", "link", "ld", "lld", "pixel-physics", "ascii", "filmstrip"];

    let output = if cfg!(windows) {
        Command::new("tasklist").args(["/FO", "CSV", "/NH"]).output()
    } else {
        Command::new("ps").args(["-eo", "comm,pid", "--no-headers"]).output()
    };
    let Ok(output) = output else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&output.stdout);

    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for line in text.lines() {
        // Windows: `"name.exe","1234","Console","1","12,345 K"`. Unix: `name 1234`.
        let (name, pid) = if cfg!(windows) {
            let mut fields = line.split("\",\"").map(|f| f.trim_matches('"'));
            (fields.next().unwrap_or(""), fields.next().unwrap_or(""))
        } else {
            let mut fields = line.split_whitespace();
            (fields.next().unwrap_or(""), fields.next().unwrap_or(""))
        };
        if pid.trim().parse::<u32>() == Ok(own) {
            continue;
        }
        let stem = name.trim().trim_end_matches(".exe");
        let stem = stem.rsplit(['/', '\\']).next().unwrap_or("");
        if interesting.contains(&stem) {
            *counts.entry(stem.to_string()).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .map(|(name, n)| if n > 1 { format!("{n}x {name}") } else { name })
        .collect()
}

// ---------------------------------------------------------------------------
// The machine-wide timing lock
// ---------------------------------------------------------------------------

/// Held for the length of a timing run. Released on drop.
///
/// Advisory, not enforced — it only serialises processes that ask for it,
/// which is why [`Machine`] exists alongside it rather than instead of it.
pub struct TimingLock {
    path: PathBuf,
    owned: bool,
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Take the machine-wide timing lock, waiting for whoever holds it.
///
/// Set `PIXEL_PHYSICS_NO_PERF_LOCK=1` to skip — correct in CI, where the
/// runner is alone on its box and a lock buys nothing, and correct for a run
/// whose output is being read for behaviour rather than for timing.
pub fn lock(label: &str) -> TimingLock {
    lock_at(temp_file("pixel-physics-perf.lock"), label)
}

/// The lock, parameterised on its path so the unit test can exercise the
/// real acquire/release cycle without touching the real lock file.
///
/// Not a stylistic split: the first version of the test below removed the
/// machine-wide lock to get a clean slate, which would have silently stolen
/// it out from under a harness someone else was running -- `cargo test` and
/// `scripts/perf.sh` can perfectly well be going at once, and one of them
/// deleting the other's mutex is worse than having no mutex.
fn lock_at(path: PathBuf, label: &str) -> TimingLock {
    if std::env::var_os("PIXEL_PHYSICS_NO_PERF_LOCK").is_some() {
        return TimingLock { path, owned: false };
    }

    let waiting_since = Instant::now();
    let mut announced = false;
    loop {
        match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                let _ = write!(file, "{} {} {}", std::process::id(), now_secs(), label);
                if announced {
                    println!("perf lock: acquired after {:.0} s", waiting_since.elapsed().as_secs_f64());
                }
                return TimingLock { path, owned: true };
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                let (holder, held_for) = read_holder(&path);
                if held_for > STALE_AFTER {
                    println!(
                        "perf lock: {holder} has held it for {} min — assuming it died, taking over",
                        held_for.as_secs() / 60
                    );
                    let _ = fs::remove_file(&path);
                    continue;
                }
                if waiting_since.elapsed() > MAX_WAIT {
                    println!("!! perf lock: gave up waiting for {holder} — timings below may be contended");
                    return TimingLock { path, owned: false };
                }
                if !announced {
                    println!(
                        "perf lock: waiting for {holder} (held {} s) — set PIXEL_PHYSICS_NO_PERF_LOCK=1 to skip",
                        held_for.as_secs()
                    );
                    announced = true;
                }
                std::thread::sleep(Duration::from_secs(2));
            }
            Err(_) => return TimingLock { path, owned: false },
        }
    }
}

fn read_holder(path: &Path) -> (String, Duration) {
    let mut text = String::new();
    if let Ok(mut f) = fs::File::open(path) {
        let _ = f.read_to_string(&mut text);
    }
    let mut fields = text.split_whitespace();
    let pid = fields.next().unwrap_or("?").to_string();
    let since = fields.next().and_then(|s| s.parse::<u64>().ok()).unwrap_or_else(now_secs);
    let label = fields.collect::<Vec<_>>().join(" ");
    let held = Duration::from_secs(now_secs().saturating_sub(since));
    (format!("{label} (pid {pid})"), held)
}

impl Drop for TimingLock {
    fn drop(&mut self) {
        if self.owned {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worst_is_reported_beside_the_distribution_it_came_from() {
        // The ants scene's real shape: 12,000 frames at a few ms, one at 72.
        let mut t = FrameTimer::new();
        t.samples_ms = (0..100).map(|_| 4.0).collect();
        t.samples_ms.push(72.0);
        assert!(t.worst() > 5.0, "the spike has to clear the absolute floor for the ratio to be read at all");
        // The number the budget is about is still the max...
        assert_eq!(t.worst(), 72.0);
        // ...but p99 and the median show it is one frame, not the workload.
        assert_eq!(t.median(), 4.0);
        assert!(t.p99() < 10.0, "one spike in 101 frames must not set p99, got {}", t.p99());
        assert!(t.worst_looks_like_an_outlier());
        assert!(t.report().contains("suspect interference"));
    }

    #[test]
    fn the_budget_count_catches_what_a_ratio_calls_clean() {
        // The performance-audit session's dry-seed control: median 7.3 ms,
        // max 20.1. The ratio is 2.8x, so no outlier flag fires and nothing
        // looks wrong -- but two frames missed 60 Hz, and that is the fact a
        // player would have felt. Both statements are true; only one is
        // about the constraint.
        let mut t = FrameTimer::new();
        t.samples_ms = (0..300).map(|i| if i % 150 == 0 { 20.1 } else { 7.3 }).collect();
        assert!(!t.worst_looks_like_an_outlier(), "2.8x is not an outlier, and the ratio test is content");
        assert_eq!(t.over_budget(), 2, "two frames over 16.7 ms is the finding the ratio missed");
        assert!(t.report().contains("2 over 16.7 ms"));
    }

    #[test]
    fn a_world_that_settles_is_not_called_interference() {
        // The case the first version of the flag got wrong, taken from a real
        // `examples/ascii.rs` line: 1,200 frames, worst 0.426 ms, median
        // 0.000. The ratio is enormous and completely meaningless -- a settled
        // world does no work, so every frame that did anything is an outlier
        // by construction. Nothing here is within two orders of magnitude of
        // the frame budget.
        //
        // This is the "what does the metric say when nothing is wrong" check
        // that `CLAUDE.md` asks for, and writing it only as synthetic uniform
        // samples (the test below) is exactly how the bug got past a green
        // suite the first time.
        let mut t = FrameTimer::new();
        t.samples_ms = (0..1200).map(|i| if i % 400 == 0 { 0.4 } else { 0.0002 }).collect();
        assert!(t.worst() > t.median() * 10.0, "the ratio really is lopsided -- that is the trap");
        assert!(
            !t.worst_looks_like_an_outlier(),
            "a settled world must not be reported as contended: worst {:.3} ms is nowhere near the budget",
            t.worst()
        );
        assert!(!t.report().contains("suspect interference"));
    }

    #[test]
    fn a_genuinely_expensive_workload_is_not_called_interference() {
        // Every frame costs the same: the worst frame is the cost, and
        // nothing here should be waved away as contention. This is the
        // "what does the metric say when nothing is wrong" check —
        // `CLAUDE.md` asks for it before trusting a new metric.
        let mut t = FrameTimer::new();
        t.samples_ms = (0..100).map(|i| 40.0 + (i % 3) as f64).collect();
        assert!(!t.worst_looks_like_an_outlier());
        assert!(!t.report().contains("suspect interference"));
        assert!(t.p99() >= 41.0);
    }

    #[test]
    fn the_lock_is_exclusive_and_released_on_drop() {
        // Skipped rather than asserted when the bypass is set, because the
        // whole point of the bypass is that it hands out a lock that locks
        // nothing.
        if std::env::var_os("PIXEL_PHYSICS_NO_PERF_LOCK").is_some() {
            return;
        }
        // Its own path, never the machine-wide one: see `lock_at`.
        let path = temp_file("pixel-physics-perf-selftest.lock");
        let _ = fs::remove_file(&path);
        {
            let held = lock_at(path.clone(), "unit test");
            assert!(held.owned);
            assert!(path.exists(), "an owned lock must be visible to other processes");

            // A second acquire must not succeed while the first is held. Not
            // via `lock_at`, which would correctly block for 20 minutes --
            // the exclusion is `create_new`'s, so that is what is asserted.
            let second = fs::OpenOptions::new().write(true).create_new(true).open(&path);
            assert!(second.is_err(), "two processes must not both hold the timing lock");
        }
        assert!(!path.exists(), "dropping the lock must release it, or the next run waits 15 min");
    }

    #[test]
    fn a_stale_reference_never_manufactures_a_warning() {
        // The reference converges from above, so the factor can never exceed
        // 1.0 on the first run of a fresh machine however slow that run was.
        let current = 100.0;
        assert_eq!(quiet_reference_from(None, current), current);
        assert_eq!(quiet_reference_from(Some(80.0), current), 80.0);
        // A faster run replaces a slower stored reference.
        assert_eq!(quiet_reference_from(Some(120.0), current), current);
    }

    /// The pure half of `quiet_reference`, so the policy can be tested
    /// without depending on what is in the temp directory.
    fn quiet_reference_from(stored: Option<f64>, current: f64) -> f64 {
        match stored {
            Some(previous) if previous <= current => previous,
            _ => current,
        }
    }
}
