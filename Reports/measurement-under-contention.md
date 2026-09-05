# Measuring frame cost on a machine several agents share

**Status: evidence landed 2026-08-25; the mechanism it describes was built,
verified, and deliberately NOT landed.** Read the next paragraph before
reaching for anything this report names.

**Nothing below exists in the tree.** `src/perf.rs` (733 lines),
`examples/quiet_probe.rs`, `scripts/perf.sh` and the reworked counter gates in
`examples/ascii.rs` live only on the unmerged `perf-lock` branch. This document
is here because the *measurements* are worth keeping and the branch is not
worth landing — see `Reports/dead-ends.md` under `other`, and the decision note
at the end of this header.

**Why it was not landed, decided 2026-08-25.** Every number here was taken on a
**four-core box shared by several concurrent sessions**, and the mechanism is a
machine-wide lock for exactly that situation. Development has since moved
mostly to cloud containers, which do not share cores between sessions, so the
contention the lock arbitrates is largely not the contention that now exists.
Against that, landing it costs 1,546 insertions across 11 files, including a
317-line rework of the CI-gated `examples/ascii.rs` and an edit to
`examples/filmstrip.rs`, the most-collided file in the repo — from a branch
**547 commits behind `main`**, scoring `BxF 6,017` against this project's own
300 bar for "past the point where merges get expensive".

**What was kept instead.** The two findings that generalise past the box they
were measured on are now rules in `CLAUDE.md`'s Method section: *gate on
counters, never on wall clock*, and *measure one scene, not the suite*. Neither
needs the lock to be true, and both were the expensive half of what this
investigation learned.

**The perf lane declined the lock independently, and for a sharper reason than
the one above.** Reviewing this on 2026-08-25 it reported that every timing it
had taken was in a cloud container, and that the contention it actually
suffered was **self-inflicted** — its own `cargo build`, and some spinning
waiter loops — which a machine-wide lock would not have touched. §6 below
already declines making `cargo` take the lock, so the mechanism as built would
not have helped even on a shared box. *Discipline about not building during a
run is the fix, not a mutex.*

**Both rules were then audited against live work, and one found a real gap.**
The same lane checked its four load-bearing conclusions: two rest on counters
(`deferred` 19,674 against 5,134; `max aux` 2,482 against 142) and are
unaffected. The third — a four-way chain-mode comparison at 9.72 / 9.72 / 10.03
/ 9.75 ms — is pure wall clock with a **3% spread**, comfortably inside what
the evidence here says a busy box produces. That conclusion now rests on a
source-level argument instead, and the timings are marked as decoration. The
rule caught a number that was reading as evidence and was not.

**One rule in this report's own account was corrected by that review, and the
correction is in `CLAUDE.md` rather than here.** The blanket claim that an
untrusted worst frame is worth nothing is true for the case §1 draws it from —
a worst buried among thousands of comparable frames — and false when the
expensive event is *rare*, because the mean then contains the worst rather than
being independent of it. The test is `mean × frames ≈ worst`. See the Method
section; this report's §5 numbers are unaffected, but do not quote its
worst-frame reasoning without that qualifier.

**The condition for revisiting**, stated so it can be tested rather than
remembered: if development returns to a machine where concurrent sessions
contend for the same cores, this report is the design and `perf-lock` is the
implementation. Re-derive rather than re-merge — the branch will be further
behind still, and §6 below records what was deliberately left unbuilt.

`CLAUDE.md` carries the working rules that come out of this. This file is the
evidence behind them, and the record of two mechanisms that were built and
found wrong before the third worked.

---

## 1. The problem, stated as a measurement

This tree is worked in by several sessions at once, each with its own
`target/`, on a box with **four logical cores** — against a simulation whose
production driver (`parallel::step`) wants all four.

Two runs of a *byte-identical* `examples/ascii` binary, doing bit-identical
deterministic work:

| scene | run A | run B | ratio |
|---|---|---|---|
| water round a pillar | 0.373 ms | 0.904 ms | **2.42x** |
| stress, parallel | 196.801 ms | 122.412 ms | **0.62x** |
| stress + field, parallel | 102.089 ms | 146.729 ms | 1.44x |
| ants, *mean* | 3.939 ms | 4.152 ms | 1.05x |

Run A reported the **parallel** stress scene as *slower* than the serial one
(196.8 against 121.2) — backwards from M5's entire purpose — and run B
reversed it. Both orderings cannot be true. Nothing in the simulation changed.

The statistic was measuring the rest of the machine, and it was the worst
possible statistic for the job: every timing scene reported a **maximum**,
which one scheduler preemption in ten thousand frames can set by itself.

## 2. How busy is "busy"? (`examples/quiet_probe`)

Guessing a wait budget without this number would have been an aspiration, not
a bar. 78 samples over 45 minutes at 20 s intervals:

```
quiet (under BUSY_FACTOR):     6/78 = 8%
factor: min 1.00x, median 1.99x, p90 9.13x, max 15.09x
longest unbroken quiet spell:   40 s
longest unbroken busy spell:   920 s
```

**The median is 1.99x: this machine's normal condition is running at half
speed.** Up to nine `cargo` and four `rustc` processes were live in a single
sample, from sessions that will never take any lock this repo invents.

Two consequences follow directly, and they are the whole design:

- **A wait-for-quiet gate cannot be strict.** A budget long enough to outlast
  a bad spell is fifteen minutes. The gate waits 60 s and then measures
  anyway, labelling the run.
- **The measured section must be shorter than a quiet window.** The full
  suite is ~143 s against a 40 s longest window, so a full-suite run can
  *never* be trusted however long it waits — which is why every one of them
  came back UNTRUSTED, structurally rather than by luck. A single scene is
  7-11 s and fits. Hence `scene=<substring>`.

The sample includes this session's own builds, so it overstates somewhat —
but `9x cargo` and another session's `filmstrip` show it is not mostly self-
inflicted.

## 3. What was built

**Counters gate; wall clock only reports.** Every assertion in
`examples/ascii.rs` is now a deterministic count — unsupported cells, awake
chunks, tiles processed, chunks redrawn. Identical under any load.

The repo's *only* wall-clock assertion (settled pheromone pass under 0.5 ms)
was deleted rather than fixed: the counter immediately above it already
proved the pass did **no work at all**, which is strictly stronger than "it
did the work quickly" and cannot flake. A time-based restatement of a claim a
counter already makes exactly can only ever fail for reasons that are not
about the code.

**`FrameTimer`** reports worst beside p99, median, and frames over the 16.7 ms
budget. The budget count came from the performance-audit session and is
better than the ratio flag it replaced: a ratio is relative to whatever the
scene happens to cost, and this is relative to the only number that is fixed.

**`TimingLock`** is machine-wide and advisory; `scripts/perf.sh` builds
*outside* it and runs the prebuilt binary *inside*, so the hold is a run and
not a compile.

**`Machine`** is the busy detector, because the lock only binds processes
that opt in.

## 4. Three mechanisms that were wrong first

Each of these passed its tests and was caught by running the harness.

**A single-threaded calibration probe.** Reported a serene **1.00x while four
`cargo` processes and a `rustc` were running.** On four cores a 3 ms
single-threaded burst is simply handed a free core, so it answers "is *my*
core stolen" (usually no), not "is this machine busy" (emphatically yes). The
all-core probe read **1.91x** under the same load. A readout that says quiet
during a compile storm is worse than no readout, because it will be believed.

It also needed a second, independent signal: a sample at 631 s read `1.00x`
with a live `rustc`, and only the compiler check caught it. `rustc` and the
linker exist *only* while something is being built — unlike `cargo`, which
sits around waiting on locks — so seeing one is a direct observation rather
than an inference, and it outranks the calibration.

**An outlier flag with no absolute floor.** "Worst is more than 10x the
median" fired on *every* settled scene. A world that settles spends 1,199 of
1,200 frames doing literally nothing, so its median is ~0 and any frame that
did something is a thousandfold outlier. That is what settling looks like,
not interference — the same shape of error as the whisker hunt in
`CLAUDE.md`, where "water with air above and below" turned out to be the
definition of a falling droplet. The flag now requires the worst frame to
clear 5 ms before the ratio is read at all.

It survived a unit test built on synthetic uniform samples, because that test
contained no settled case. Asking *what does this say when nothing is wrong*
against real output was what caught it.

**A strict wait-for-quiet gate.** Discarding a run that could not get a clean
window would have thrown away four runs in five. The gate now never refuses
and never discards: it waits a bounded 60 s, measures regardless, and stamps
the result. A labelled untrustworthy number still carries exact counters; a
run that never happened carries nothing.

## 5. Does it work?

Three back-to-back attempts at one scene, bit-identical work:

| attempt | verdict | worst | median | over budget |
|---|---|---|---|---|
| 1 | UNTRUSTED (load arrived mid-run) | 45.868 ms | 3.621 ms | 1 |
| 2 | UNTRUSTED (busy throughout) | 38.532 ms | 3.700 ms | 1 |
| 3 | **TRUSTED** | **7.624 ms** | **2.833 ms** | **0** |

Roughly one attempt in three lands, at about a minute each. The worst frame
moves **6x** with machine state while the median moves ~30% — so an untrusted
*median* is worth something and an untrusted *worst* is worth nothing.

The trusted run says something no earlier measurement could: **the parallel
driver's worst frame on the full-screen stress scene is 7.6 ms, inside the
16.7 ms budget, with zero frames over.** Every previously quoted figure for
that scene (13.3, 20.6, 23.4, 45.9) was contaminated.

Serial work needs none of this. Its median measured 19.147 ms at 8.39x busy
against 19.107 ms at 1.00x — 0.2% apart — because one thread competes for far
less than four do.

## 6. What is deliberately not built

**Compilation does not take the timing lock, and compilation is the
interferer — decided, 2026-08-19: not building it.** This is the gap that
keeps TRUSTED runs at one-in-three instead of routine. Closing it would mean
builds acquire the lock as readers and measurements as writers, so a
measurement *creates* its window instead of waiting for one — the only
mechanism that can work on a box busy 92% of the time.

The decision is **no**, on this reasoning:

- **It buys speed, not correctness.** The workflow without it already yields
  a sound number: re-run until `TRUSTED`, about one attempt in three, a
  minute each. Three minutes for a trustworthy measurement is not a problem
  worth a protocol. Nothing about the lock would make a wrong number right;
  `Machine` and the verdict already refuse to let a wrong number pass as a
  right one, which is the property that actually matters.
- **It is the only mechanism here whose failure mode costs other people
  work.** Everything else degrades a measurement. A stale writer lock stalls
  every agent's build.
- **It is enforced only by convention, and fails silently.** Every session
  must route `cargo` through a wrapper; one that does not simply competes as
  before, while the mechanism's existence invites the belief that it cannot.
- **`sccache` attacks the same load without any of that.** Installed
  2026-08-19; a cleaned-crate release rebuild went 17.74 s to 0.81 s. It
  needs no cooperation, has no blast radius, and removes work rather than
  scheduling it.

What would reopen it: a measured need for routine TRUSTED runs — a timing
question asked often enough that three-minutes-per-answer is the bottleneck —
or a re-run of `examples/quiet_probe` (45 min, once every session has
restarted into `sccache`) still showing single-digit quiet *and* someone
actually blocked by it. The 8% figure above predates `sccache` and should not
be quoted as if it did not.

**`examples/filmstrip.rs` locks only on an explicit `lock=1`.** The first
attempt keyed it off `max_frame_ms=`, on the reasoning that this is the one
filmstrip expectation which can fail for a reason other than the simulation.
That reasoning was wrong on a fact: **all sixteen acceptance cases set
`max_frame_ms`**, so the conditional locked every one of them and the comment
claiming otherwise was false.

The corrected view is that acceptance is *load*, in the same category as
another session's `cargo build` — what `Machine` is built to notice, not what
`TimingLock` is built to serialise. Its frame bars are deliberately
contention-proof (60 ms against 3-14 ms measured, min-of-`repeat`, sized to
catch a 6,556 ms catastrophe); they do not want a quiet machine, they want to
be immune to a busy one. Pass `lock=1` when a filmstrip run *is* the
measurement.

### The acceptance suite's frame bars are not as contention-proof as they claim

Two consecutive runs of `scripts/acceptance.sh`, same binary, same seeds: the
first reported **one case failed**, the second reported all sixteen passing.

Every other gate in that suite is a deterministic count on a fixed seed —
`min_overloaded`, `max_failures`, `min_cave`, `min_bodies` — and determinism
is required same-build. By elimination the flake was a `max_frame_ms` bar.

This is a wall-clock gate, in a suite CI runs, on a box that has been sampled
at 15.09x. `acceptance.sh`'s own comment argues the bars cannot flake because
they check the *minimum* of `repeat=` runs and "contention can only make a
frame slower" — which is true, and insufficient: min-of-2 still needs one of
two runs to land in a quiet window, and this box is quiet 8% of the time.

Not fixed here, because `acceptance.sh` is on the structural work's contested
list and its bar tuning is theirs to set. The cheap mitigation is a larger
`repeat=` (min of 3 rather than 2) at the cost of a longer run; the honest one
is the same conclusion this whole document reaches everywhere else — a
wall-clock number should be reported and a load-invariant one should gate. No
counter equivalent exists for "a frame took 6,556 ms", which is exactly why
that bar is still worth having.

**No global `cargo` job cap and no build-priority wrapper.** A cap taxes the
solo build all the time to mitigate a sometimes-problem, and does not fix
oversubscription anyway at N agents on 4 cores. `sccache` is the item worth
having here — thirteen worktrees currently rebuild identical dependencies
from scratch — but installing it is a machine-wide change for the owner to
make, not a repo change.

---

## 7. The derivations behind `CLAUDE.md`'s timing rules

Moved here 2026-09-05 from `CLAUDE.md`'s *A timing number is only as
trustworthy as the box was quiet*, which keeps every rule and every headline
number and routes the worked cases. Nothing below is new; it is the evidence
those rules were derived from, which every session was paying for and almost
none needed.

### 7a. Why "identical under any load" was wrong about counters

The rule read *"identical under any load"* until 2026-08-30, when the burrow
lane measured **610 digs idle and 278 loaded from the same baseline binary**
on `ascii`'s colony scene. The mechanism is rayon's thread count changing with
the box, so how the sweep is cut across workers reaches the result. An A/B by
env switch in a single binary — which is what measured the 130-against-0 the
entry comes from — is immune either way, because both arms see one
parallelism.

Measured again independently 2026-08-25 by the perf lane, from the other
direction: a scheduler census recompiled between two runs of one scene came
back **byte-identical** (`produced 7042 / deferred 61488` at frame 4,800, both
times) while the wall-clock column on the same frame moved **9.54 → 8.16 ms**.
The counters reproduced exactly where the clock moved 17%.

### 7b. The null that was a counter counting calls

A harness probing whether the pick leaks the way a blast does reported 200
cuts and a queue flat at its idle 5,400 — a clean, counter-based negative,
measured two hours after the counters rule landed. **The counter was counting
calls.** `rigid::is_tool_target` accepts `Solid | Plant` and refuses bedrock,
and the harness aimed at the topmost `Solid | Powder` cell, which on a rolling
world is soil, so every swing landed in dirt: 23 swings, **0 cells removed**.
With the aim corrected the same 23 remove **1,157**, and the queue then goes
to the scheduler's cap and stays there.

The asymmetry that makes the pairing rule cheap: `rigid::mine_swept` returns
its own loosened count, `rigid::strike` returns `()`, so the second needs a
census of the neighbourhood either side of the blow.

### 7c. How the worst-frame rule was corrected

The original said flatly that an untrusted worst is worth nothing. True for
the case it was drawn from — the worst is one frame among thousands of
*comparable* ones and a single scheduler preemption can set it; measured
across three attempts at one scene, the worst moved **6x** with machine state
while the median moved ~30%.

It is false when the expensive event is **rare**, because then the mean
contains the worst. One blast per run puts essentially all time ever spent in
the blasts phase into a single frame: the converged-pass figure pins at
**0.97** (mean 0.076 ms × frames = 456 ms against a 440.7 ms worst) and its
bedrock-only control at **0.96**, while the `ascii` case pins at nothing at
all. Two further independent legs held there.

The correction came from the perf lane pushing back with a case the original
got wrong, and the replacement is better because it is *satisfiable* rather
than merely cautionary: it says when a worst may be quoted, not only when it
may not.
