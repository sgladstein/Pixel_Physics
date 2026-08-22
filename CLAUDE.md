# Working in this repo

This file is for *how to work here*, not what the code does. The codebase is
already heavily documented and the architecture is written up at length
elsewhere — see below. What is not written down anywhere else, and what this
project keeps re-learning the expensive way, is the method.

## The ethos: it has to feel satisfying

**Stated by the owner as a core value, above correctness of any individual
mechanic: everything should feel satisfying.** A mechanic that is right on
paper and dull in the hand has failed, and "the test passes" is not a
defence.

This is not a restatement of "looks good in motion" below — that is about
*appearance*, and this is about *response*. Destroying something should feel
like destroying it: the thing should crack, throw debris, come apart in
pieces of varying size, and react to how it was hit. Two failures that
already cost real rework, both of which passed every test they had:

- **All-or-nothing outcomes.** Structural failure produced either a single
  large coherent body or a uniform dissolve into powder, with nothing in
  between. Real breakage is a *distribution* — a few blocks, more cobbles, a
  lot of grit — and its absence read as fake immediately.
- **No verb behind the effect.** Destruction could only be triggered by
  *erasing* support, which delivers no load and no impulse, so nothing ever
  failed from being *hit*. The mechanic worked and still felt inert, because
  the player had no way to strike anything.

Practical consequences when weighing a change: prefer the version with more
legible feedback even when it is less exact; a graded outcome beats a binary
one; and if a destructive event produces no debris, no impulse and no sound
of consequence, it is not finished regardless of what the simulation
believes. Judge this by playing it, not by reading the diff — the owner's
playtest reports have overturned three separate models that all looked
correct in tests.

## What this project is optimising for

**Looks good and realistic, in motion, at play scale — without ruining
performance.** Stated by the owner directly. Three consequences that have
already changed decisions:

- **Exactness is not a goal.** A mechanism whose measured advantage is
  numerical precision — an exactly flat surface rather than a nearly flat
  one — is not buying anything here, however well argued. Judge liquid work
  by how it looks while it is moving, not by its final residual.
- **The current 512x320 world is a test environment, not the target.** It
  will grow (M10 streaming). So a cost that is invisible today because the
  world is small is still worth taking seriously, and a mechanism whose
  advantage only appears at large width is not automatically useless — but
  it does have to actually *have* that advantage when measured, which is
  not something to take from a report on faith. See
  `Reports/open-bugs-handoff.md` §6 for a case where it did not.
- **Frame cost is a hard constraint, not a tiebreaker.** A visual
  improvement that costs the dirty-rect render skip, keeps chunks awake, or
  slows the sweep is not automatically worth it — say what it costs when
  proposing it. `examples/ascii.rs` reports worst-frame timings and CI runs
  it; that is the number to quote — but quote it from a run under
  `scripts/perf.sh`, with the `machine:` banner beside it. Unlocked on this
  box the same figure swings 2.4x on identical work, and it is not a number
  until it is quiet (see "A timing number is only as trustworthy as the box
  was quiet" below). The corollary cuts the other way too:
  because exactness is not wanted, *stopping work early* is a legitimate
  optimisation. A pool that is visually flat but still shuffling fill for
  another quarter of an hour is a real cost buying nothing.

## Where knowledge already lives — read it, don't re-derive it

| File | Holds |
|---|---|
| `README.md` | Architecture, and per-milestone status |
| `wiki/*.md` | What a material or mechanic *does*, in plain language — no code, no file names. `Reports/*.md` is *why it's built that way*; this is *what it looks like when it's right*. |
| `PLAN.md` | Roadmap, settled decisions, the issues backlog |
| `Reports/*.md` | Design records and research, one per subsystem |
| `Reports/open-bugs-handoff.md` | **Open bugs.** Working reproductions, what has been ruled out *by measurement*, and what was tried and reverted. Read this before touching a listed area. |
| `Reports/design-philosophy.md` | Settles arguments about constants, hardcoding, and scope boundaries |
| `Reports/fracture-mechanics-design.md` | Why rock breaks the way it does, and why three earlier support models were wrong |
| `Reports/load-model-handoff.md` | **The next step on destruction**, written up to be picked up cold |
| `Reports/measurement-under-contention.md` | **Why a timing number here needs a `TRUSTED` stamp**: how busy this box actually is, three detectors that were wrong first, and the one mechanism deliberately left unbuilt |

**Source comments are load-bearing.** They record *why*, including approaches
that were tried and reverted and must not be retried. Do not strip them when
editing nearby code, and add to them in the same voice when you learn
something that cost effort to find.

## Commands

```
cargo test                                       # unit + integration
cargo clippy --all-targets -- -D warnings        # CI gates this
cargo run --release --example ascii              # headless behaviour + worst-frame timing; CI runs it
cargo run --release --example filmstrip -- scene=fall zoom=2 crop=0,140,256,110
scripts/perf.sh                                  # ascii, but built outside the machine-wide timing lock and measured inside it
scripts/perf.sh filmstrip scene=strike count=4   # same, for any other example
```

`filmstrip` writes a contact-sheet PNG — several frames of one run in a grid —
so an artifact can be judged by eye without a window. For the real app, press
`F7` to the `flat` preset — dead-level bare rock with 200 rows of sky, the
structural test bed — or set
`PIXEL_PHYSICS_CAPTURE_SEQUENCE=<start>,<interval>,<count>`; frames and a GIF
land under `%TEMP%`.

## Working alongside another session

**This tree is worked in concurrently, and often by more than one agent at
once.** Git handles the merges; what it cannot handle is the two failures
below, both of which have cost real hours.

**Work in your own worktree, not the shared checkout.** Two sessions in one
checkout share a `target/`, so one session's half-finished edit makes the
*other* session's `cargo test` and `cargo clippy` fail on code it did not
write and must not fix — and a running sandbox in one session locks the exe
the other needs to link. Both happened in a single afternoon. A worktree
gives each session its own `target/`, so a broken build stays local to
whoever broke it. `.claude/worktrees/` already holds several.

**Know which files are yours.** Collisions are almost never random — they
land in the same few files every time:

| Area | Files |
|---|---|
| Structural / destruction | `src/sim/load.rs`, `structural.rs`, `rigid.rs`, `examples/filmstrip.rs` scenes, `scripts/acceptance.sh` |
| Worldgen | `src/worldgen/*`, `assets/worldgen.ron`, `tests/worldgen.rs` |
| **Contested** | `src/app.rs`, `src/main.rs`, `README.md`, `PLAN.md`, this file |

Everything that has actually collided here collided in `src/app.rs`. So:
**if you touch a contested file, land it quickly** rather than holding a
large diff across a session — the window in which someone else's work
cannot compile is the window you created.

If you find yourself needing to commit while a contested file holds
somebody else's unfinished work, do **not** try to stage around it. Add a
worktree at `origin/master`, re-apply your own change there, verify, commit
and push from it, then bring the main tree's branch pointer forward with
`git reset --mixed origin/master` — which moves the branch and leaves their
working tree untouched.

**That reset strands stale files whenever the main tree was *behind*.** It
moves the pointer and deliberately does not touch the working tree, so every
file the main tree had not yet updated now differs from `HEAD` and appears in
`git status` as a modification — one that is really a **revert of the upstream
commit it missed**. Nobody will recognise it as theirs, because it is not
anyone's edit, and the next session to commit that file silently undoes the
change. This is not specific to `CLAUDE.md`-style contested files; it hits any
file the branch skipped over. Seen for real on `src/sim/structural.rs`, which
came back as the exact inverse of the commit that had just landed it.

So: **note which files are genuinely dirty *before* the reset**, because
afterwards a stale file and an edited one look identical. After it, diff
anything newly modified against the commits you were behind by; if it is their
exact inverse and the file was clean beforehand, `git checkout --` it.

**An empty worktree is not an unclaimed one.** `.claude/worktrees/perf-audit`
was clean, at `master`, with a warm `target/` — and another session started
writing into it twenty minutes later. Their half-written example broke
`cargo clippy --all-targets` for work that had nothing to do with it, which
is the documented failure arriving exactly as documented. Take a *fresh*
worktree on your own branch rather than adopting a tidy one, and if you find
someone has moved in, copy your files out and `git checkout --` what you
touched rather than leaving two sessions' work interleaved.

**And do not *build* in someone else's worktree, which reverting your source
edits does not undo.** The rule above says use your own worktree; it needs
this second half, because the artifact outlives the edit. A session that
built `examples/ascii` in `perf-audit`, then tidily reverted every source
file it had touched, left its **binary** sitting in that tree's `target/`.
The next session ran it, got numbers from code that was not in its checkout,
and discarded a whole profiling run once it worked out why. Reverting
sources is not cleaning up: `cargo build` in a tree you do not own is the
act, and the exe is what does the damage. If you have built somewhere you
should not have, say so and delete the artifact.

**`strings` does not exist in this Git Bash**, and its absence is silent —
`strings x.exe | grep -c foo` prints `0` for every input, including inputs
that certainly match. That false negative nearly closed the investigation
above with the wrong answer. Point `grep -c` straight at the binary instead,
and **run a positive control** on a file you know contains the string before
believing a zero.

### A timing number is only as trustworthy as the box was quiet

**Four logical cores, and any other session's `cargo build --release` takes
all of them.** Two runs of a *byte-identical* `examples/ascii`, doing
bit-identical deterministic work, reported:

| scene | run A | run B |
|---|---|---|
| water round a pillar | 0.373 ms | 0.904 ms (**2.4x**) |
| stress, parallel | 196.8 ms | 122.4 ms (**0.62x**) |
| stress + field, parallel | 102.1 ms | 146.7 ms (1.44x) |
| ants, *mean* | 3.939 ms | 4.152 ms (1.05x) |

Run A had the *parallel* stress scene slower than the serial one — backwards
from M5's entire purpose — and run B reversed it. Nothing in the simulation
changed. A worst-frame figure off a contended box cannot support a claim in
either direction below about 2.5x, and the max is the statistic most exposed
to this: one preemption in ten thousand frames sets it.

**And the box is essentially never quiet.** `examples/quiet_probe` sampled it
for 45 minutes: **8% of samples quiet, median factor 1.99x, p90 9.13x, max
15.09x, longest quiet spell 40 s, longest busy spell 920 s.** Plan around
that rather than hoping. Two consequences that decide how to measure:

- **Measure one scene, not the suite.** A scene is 7-11 s and fits in a 40 s
  window; the full suite is ~143 s and never will. `scene=<substring>` in
  `examples/ascii.rs` exists for this and nothing else. Run the whole suite
  for the counter gates, where load is irrelevant — and one scene when the
  question is milliseconds.
- **Do not wait for quiet for long.** The gate waits 60 s and then measures
  anyway, stamping the run `UNTRUSTED`. A budget long enough to outlast a bad
  spell would be fifteen minutes; a run that never happened is worth less
  than one that is honestly labelled.
- **Re-run until it says `TRUSTED`, and quote nothing else.** That is the
  whole workflow, and it works: three back-to-back attempts at one scene gave
  UNTRUSTED, UNTRUSTED, **TRUSTED**, at about a minute each. What the label
  is worth, on bit-identical work:

  | attempt | verdict | worst | median | over budget |
  |---|---|---|---|---|
  | 1 | UNTRUSTED (load arrived) | 45.868 ms | 3.621 ms | 1 |
  | 2 | UNTRUSTED (busy throughout) | 38.532 ms | 3.700 ms | 1 |
  | 3 | **TRUSTED** | **7.624 ms** | **2.833 ms** | **0** |

  The worst frame moves **6x** with machine state and the median barely
  moves — so an untrusted *median* is worth something and an untrusted
  *worst* is worth nothing at all. Expect roughly one attempt in three to
  land.

- **Run timings through `scripts/perf.sh`.** It builds *outside* a
  machine-wide lock and runs the prebuilt binary *inside* it. `cargo run
  --example ascii` still takes the lock, it just holds it across its own
  compile — which is a lock everyone will route around.
- **The lock does not cover compilation, and compilation is the interferer.**
  Nine `cargo` and four `rustc` processes were live in a single sample. Only
  a harness takes the timing lock; nothing stops another session's build. If
  `TRUSTED` runs ever need to be routinely obtainable, this is the gap to
  close — builds would have to take the lock as readers — and it was left
  open deliberately, because its failure mode blocks other people's builds
  rather than merely degrading a measurement.
- **Read the `machine:` banner before the numbers.** A self-calibrating
  factor against the fastest this box has been seen to go, printed at the
  start *and* the end, because the case a single probe misses is another
  session's build starting halfway through. It also names the competing
  processes, and treats a live `rustc` as busy outright — a direct
  observation beats an inference. Its calibration burst runs on **all
  cores**: the single-threaded version it started as reported a serene
  **1.00x while four `cargo` processes and a `rustc` were running**, because
  a 3 ms single-threaded burst on a four-core box just gets handed a free
  core. It was answering "is *my* core stolen", not "is this machine busy".
- **Gate on counters, never on wall clock.** Everything `examples/ascii.rs`
  now asserts is a deterministic count — settled chunks, unsupported cells,
  tiles processed, chunks redrawn — identical under any load. The one
  wall-clock assertion the repo had (settled pheromone pass under 0.5 ms) is
  gone: the counter immediately above it already proved the pass did *no
  work at all*, which is strictly stronger and cannot flake. A time-based
  restatement of a claim a counter already makes exactly can only fail for
  reasons that are not about the code.
- **Report worst beside p99 and median** (`perf::FrameTimer` does it, and
  flags a worst more than 10x the median in the line itself). The ants scene
  reads worst 72.6 against median 3.9; only one of those is about the
  simulation.

## Method

Nearly every fix in this engine that was judged by test output alone failed to
change what the owner saw on screen. The ones that worked all followed the
same shape.

1. **Look before you measure.** Render the scene and look at it first. Every
   metric written before anyone had looked at the artifact has measured the
   wrong thing.
2. **Reproduce before you fix**, from the owner's description of the *initial
   state*, and confirm the reproduction actually shows the complained-about
   quantity before writing a line of fix.
3. **Verify live before declaring done** — `filmstrip`, or the app's capture
   hook. Tests passing is not evidence that the screen changed.
4. **Look again after the fix, for what you did not measure.** A metric only
   sees the quantity it was written to see. A fix that cleared one artifact
   while introducing a worse one has already shipped and been reverted here
   once, because its test only looked at the rows it expected to be wrong.

An image tells you *what* and *where*. A metric tells you *how much* and
*whether it came back*. Reaching for a metric to answer "what and where" is
the recurring mistake.

**"Did it fire at all" needs a counter, not a picture.** An image shows
*what* and *where*; it cannot show whether the thing you built is what
produced it. A collapse rendered as coherent falling slabs, was read as
"chunks are working," and the harness's own body count said **zero for the
whole run** — the feature had never once executed, and what was on screen
was loose rubble that happened to hold its shape. Two very different
mechanisms look identical at the zoom a contact sheet is read at. When a
change adds a discrete "this happened" event, print the count next to the
image and read both.

**Check that a planned step can demonstrate itself, before promising it
will.** "Cracks weaken rock" was scheduled as an independently shippable,
judge-by-eye milestone. Built, it did almost nothing visible, because
failure was evaluated per cell against *its own* reach and a crack at a
beam's root weakens a cell the criterion never tests. One question asked
earlier — *which cell does this rule actually evaluate?* — would have caught
it before the work.

The same question was missed again later, in a different costume, **by a
session that had this paragraph in front of it**: a bearing rule that is
correct for a *piece* resting on loose ground was applied per cell, so a slab
lying on its own rubble was judged as many separate knife-edge footings and
taken apart one cell at a time. Ask it as **which object does this rule
evaluate — a cell, a section, or a whole piece?** — and check that the
quantities it needs (a centroid, a contact width, a tipping moment) are even
defined for that object.

**Resolve an ambiguous complaint before building anything.** "Flatness at
rest" was read as the surface texture and turned out to mean a screen-wide
tilt — opposite directions, a whole detour spent on the wrong one. When a
report could mean two different things, the cheap move is to measure both
and see which one is actually there, or to ask. It is much cheaper than the
fix you build for the wrong reading.

**Ask what a metric counts when nothing is wrong.** The whisker hunt defined
a "film" as a water cell with air above and below — which is *what falling
water looks like*, so the metric counted every droplet in the world. Its
numbers were real and meant nothing. Sanity-check a new metric against a
case you know is fine, before trusting it about a case you don't.

**When the complaint is about something visible and persistent, measure the
standing state, not the event rate.** Attributing film *creation* blamed the
plain straight-down fall for 76% of them — true, and useless, because those
films existed for one frame each. The artifact that persists came from
somewhere else entirely, and only a standing count showed it.

### A debug readout must not be a function of the thing it debugs

Several channels that decide behaviour are invisible — per-cell scalars, not
occupancy. Build the overlay *before* the mechanism that uses them
(`render.rs`'s `FieldOverlay` / `OrganismOverlay`, `filmstrip`'s `channel=`),
and make it a **full replace on a fixed dark→bright ramp, not a blend into
the cell's own colour**. A magnitude-scaled blend was tried and produced a
canopy-density sheet that read as blank: the ramp was red, wood is brown,
and a mid-range value moved one colour byte from 139 to 155. The obvious
reading — "the mechanism is dead" — would have sent a fix at working code.

### An image says *what and where*; only a number says *how much*

The corollary of "look before you measure", and it bites in the other
direction just as hard. A corrected overlay was still misread as "everything
at the ramp floor" when the real value was 40% of scale — genuinely hard to
judge on a one-cell-wide twig. Pair every debug channel with a probe that
prints the values (`examples/plant_probe.rs`); reach for it the moment the
question turns quantitative.

### Fixing a bug often exposes a constant that was compensating for it

`thicken()`'s flood fill traversed 4 neighbours while growth places cells at
8, so it counted a fragment of a tree rather than the tree. Fixing the
traversal made every cell see the true count and thicken uniformly — because
`pipe_ratio` had been calibrated against the broken quantity. **When a fix
changes what a number *means*, re-deriving the constants that read it is
part of the fix, not scope creep** — and sweeping is how you re-derive it.

Watch for the inverse too: a gate can hide a second bug by making it
unreachable. Infiltration's conservation test passed against a version whose
gate meant infiltration never ran at all. A test can pass because the code
under it is dead, which looks exactly like passing because it is correct.

### When every setting of a sweep fails the same way, suspect the sweep

The sibling of "two fixes failing the same way means the approach is
wrong", and it points the opposite direction. Eight settings across two
*forms* of leaf abscission all collapsed the stand identically — which read
as "the approach is wrong" and was not: a structural check had landed
*with* the mechanism and was constant across every run, and it alone was
the collapse (772 cells against 20,213 at the same setting, from that one
line). A sweep only varies the knob; anything that rode along with the
mechanism is part of every data point. Before condemning an approach, run
the control that isolates it — the mechanism at its gentlest setting with
every rider stripped out.

And two more ways a sweep lies, both of which have produced whole invalid
sweeps: identical *outputs* across settings mean the knob was never
connected at all (see the `include_str!` gotcha below), and a pattern edit
can vary more than its knob — `tree.ron` holds two `crowding_weight`
lines, and a blind `sed` on the field name dragged the root's deliberate
`0.0` along with the shoot's through every data point. Prove the edit
touched only its target before trusting anything downstream of it.

### A channel that oscillates by design must be divided out of decisions

The light channel swings 20:1 over every day/night cycle by design, and a
threshold sampled at an arbitrary phase of a designed oscillator is a
different threshold every hour: the live tip count measured 71 at noon
against 28 at night on the same stand, and any fixed abscission cutoff was
a nightly extinction event. Every economic light read now goes through
`field::noon_equivalent_light` — the oscillator is a pure function of the
frame, so dividing it out costs no storage and is exact at noon.
Temperature oscillates the same way and will need the same treatment the
day anything gates on it. The cycle stays real on screen and in the field;
it just must not alias into decisions.

### Compare two runs, not one run against a remembered number

Outcomes here have enormous spread — twelve identical trees from one genome
span 31 to 153 cells. A bar set from a single run is a sample from a wide
distribution and will flake in whichever direction that run landed. Prefer a
**paired comparison** (rooted bank vs bare bank, loaded branch vs bare
branch): it cancels everything the rule under test is not about.

This applies to frame timings too. A change once looked like a 25–50%
regression against a figure measured an hour earlier; re-measuring the
baseline on the spot showed the machine had slowed and the change cost
nothing. **Always re-measure the baseline in the same session, on the same
machine, before reporting a regression.**

### Guard hot-path work at the call site that already has the data

New per-cell behaviour usually applies to one material. Gating *inside* the
function still pays a `World::get` and a lookup per cell per frame, and
matching a material by `id_of("name")` is a string hash in the sweep. Put the
opt-in on `Material` as a field and test it at the dispatch site, which
already holds the `Cell` — a `Vec` index instead.

### A scene that contradicts the code will look like a bug in the code

Two Phase-2 "root bugs" were scene errors: free water buried in a soil
column sank (soil is a `Powder`, it sinks through `Liquid`) and surfaced, so
every seed germinated onto water; and a soil column with no floor or walls
fell out of the world and toppled, leaving the sampled cells empty. Both read
exactly like "the mechanism does nothing". **When a mechanism appears inert,
check the scene still contains the situation you think it does** before
touching the mechanism.

### Metric traps, each of which has already cost real time

- **Liquids: measure column *volume*, not the topmost cell.** A `Liquid` cell
  holds continuous fill, and near-empty cells fringe every artifact. Topmost-
  cell said chunk seams were 1.7x the interior roughness; volume said 9x.
- **Dark or torn rows: measure *fill*, not occupancy.** `render.rs` dims a
  liquid toward black by fill, so a row can draw as a black line while every
  cell in it is still occupied. An occupancy metric finds literally nothing.
- **Powder faces: measure the face, not the spreading front.** The front
  crosses seams smoothly while a vertical face persists behind it.
- **Destruction: a failure count is not a damage count.** `FailureCounts`
  counts cells that *failed*; a failed cell that became rubble is still
  standing there. Two digs whose event counts look comparable removed 894 and
  23,042 cells. If the question is "how much did this eat", census the
  materials before and after — nothing in the engine measured that until it
  was needed, which is why the regression below went unseen.
- Prefer a **continuous** quantity (a summed deficit) over a **count** of bad
  cells. Counts give knife-edge margins; sums separate cleanly.

### Two drivers, and the app runs the parallel one

`update::step` is serial; `parallel::step` is a four-pass checkerboard, and it
is what `App::update` calls. **Test both.** Behaviour that only the player
sees is behaviour only the parallel driver produces.

`update::step_monolithic` (test-only) sweeps the whole world as a single
region. It is the control for the question that took three wrong hypotheses to
reach the first time: *is this coming from the movement rules, or from how the
sweep is cut into chunks?*

### Chunk decomposition is a recurring root cause

Both drivers sweep chunk by chunk, so every cell in a chunk updates before any
cell in the chunk to its right, and half of all horizontal seams invert the
bottom-to-top row order. Artifacts that line up with the F1 chunk grid are
usually this, not the physics. Suspect it early; the reports on liquids do not
consider it at all.

## Conventions

- **Set bars from measurement with headroom**, never from an aspiration and
  never sitting on the measured value. Where a report asks for a number the
  engine cannot yet hit, record both and leave the gap visible rather than
  relabelling it away.
- **A revert keeps the knowledge.** Keep the reproduction (`#[ignore]` it if
  it now fails), and record what the withdrawn fix was, what it improved, and
  why it went. A reverted fix's genuine improvements become the bar its
  replacement must meet — not the pre-fix baseline.
- **A session that makes a significant change affecting a `wiki/` page must
  update that page (and its freshness note) in the same change.** This is a
  cheap backstop, not the real defence against `wiki/*.md` going stale the
  way early design Reports did — the real defence is that each page
  describes coarse, player-visible behavior, not implementation, which is
  inherently more stable. This rule just shortens the gap on whatever does
  drift.
- **Commit messages carry the measurement**, not just the intent: the number
  before, the number after, and what was tried and rejected on the way.
- **Determinism is required** (same-build, per `PLAN.md`) — it was reversed
  from "not required" and some older comments still say otherwise.
- **A guard test must be able to fail for the *replacement* artifact**, not
  only the original one. A fix that cleared torn seam rows and introduced
  much worse banding passed its own test, because that test only looked at
  rows lying on a seam. If a fix trades one artifact for another, its test
  should be the thing that catches the trade.
- **Check that a guard's inputs actually vary what it guards.** All eight
  acceptance scenes stayed green through a change that made one world seed
  lose 26x more material to a single dig — and a ninth hand-authored scene
  would have been just as blind, because `seed=` reaches only two scenes and
  every structural case builds hand-placed geometry at the default seed. The
  scenes were not too few; they were blind *by construction*. A guard over a
  procedural system has to sweep the procedure, and it should gate an **order
  statistic** (p90 or max over N seeds) rather than any single seed: outcomes
  here are chaotic in the seed, so which one is worst reshuffles on any
  legitimate change and a per-seed baseline gets rubber-stamped. **This
  happened twice in one session** — two different changes to the load model,
  both green on all eight cases, the second eating fifty times more world than
  the bug it was fixing. A seed sweep caught each in one command. So build the
  sweep *before* changing a model that governs procedural content, not after:
  on green alone, both would have shipped.
- **Two fixes failing the same way means the approach is wrong, not the
  tuning.** Two separate attempts to penalise a cell crossing a chunk seam
  both replaced the tear with a throttle at the same seam. That is a signal
  to change the approach — the third attempt fixed the sweep *order* instead
  and cost nothing.
- **A change that moves *nothing* is different evidence from one that moves a
  little.** Gating the granular capacity divisor on `parent.is_none()` was
  recorded as a dead end because not one cell moved, across six presets and
  three seeds. It was not wrong, it was **vacuous**: `structural::tick` rooted
  a cell's distance at 0 the moment powder touched its underside, so every
  powder-backed cell was parentless *by construction* and the gate had nothing
  to discriminate. An exactly-zero delta means suspect the condition you keyed
  on is degenerate, before concluding the lever is dead — and re-test any
  do-not-retry entry of that shape after something changes its condition.
- **A constant nobody can tune in either direction may be a counterweight, not
  a model.** That same divisor existed to cancel the eager rooting above: two
  modelling errors roughly annulling each other. Every attempt to tune it made
  some case worse because it was holding a different mistake in place. When a
  term resists tuning in both directions, ask what it is compensating for.
- **When several knobs move the same number, check what each one trades.**
  `min_transfer` and `HORIZONTAL_TRANSFER_REACH` both make water settle
  sooner; the first does it by giving up on the last of the levelling
  (residual tilt 1 → 5 cells), the second costs no accuracy at all. Knobs
  that look interchangeable on the headline metric usually are not.
- **Measure a cost against the state the optimisation exists for.** An
  animated grain looked free in every moving scene and cost ~10 ms/frame on a
  *settled* one, because what it defeats is the dirty-rect render skip — and
  a settled world is exactly where that skip does its work.
- **For "does this look right", ship a runtime selector rather than choosing.**
  Five grain modes behind one key settled in minutes a question that no
  amount of argument or still images had. Default to current behaviour, name
  the active one on screen, and state what each option *costs*.
- **A size cap must bound work, never gate whether something happens.**
  Written twice in one session: fracture declined any region larger than its
  body-size cap and fell through to per-cell conversion, so the *bigger* the
  collapse the more certain it dissolved into dust. The cap belonged on a
  fragment, not on the decision to break at all. Any `if too_big { return }`
  is a claim that the largest cases deserve the least behaviour — check that
  is what you meant.
- **When a rule must tell apart two things that can look identical, state
  the difference as data.** Four successive support models tried to infer
  "is this held up" from *shape*, and every one was either strong enough to
  hold a mountain or weak enough to let a player's tower break, never both —
  because geometry cannot distinguish a mountain from a wall someone
  stacked. The fix was a bit on the cell saying which it is. If tuning keeps
  trading one case for the other, the rule is reading the wrong quantity;
  more tuning will not find a setting that does not exist.
- **A superseded mechanism's tests keep passing while testing nothing.**
  Distinct from the `#[ignore]` case below — these *run*, pass, and exercise
  nothing, because the scenario is trivially stable once the mechanism they
  were written for is gone. When replacing a mechanism, deliberately break
  the *replacement* and confirm the old tests fail. If they still pass,
  delete them rather than porting them.
- Prefer an independent review before significant commits; batch small ones.

## Gotchas that have each caused a real bug

- **Two conventions for `Cell::aux` point opposite ways.** On a `Liquid`,
  `aux == 0` means **full**. On a `Powder`, `aux == 0` means **dry**
  (`material::SOIL_SATURATED`). Both defaults are deliberate — liquids are
  created full, soil is created dry — and getting either backwards
  manufactures water out of nothing. A partly-drained liquid must be written
  as `with_aux(remaining)`, and a fully-drained one as `Cell::EMPTY`, never
  `with_aux(0)`.
- **A traversal must use the same neighbourhood the writer used.** `Grow`
  places organism cells at 8 neighbours; anything reading a grown organism
  back has to traverse 8 or it sees disconnected fragments. Transport
  (`diffuse_resource`) is the deliberate exception and stays at 4: an
  exchange crosses a shared face, and diagonal cells share only a corner.
- `Cell::is_empty()` is **managed-aware** — a promoted liquid body's container
  cells are materially empty but read as not-empty. Use the raw
  `cell.material == material::EMPTY` when the question is "is there material
  here", not "is this position available".
- `liquid_fill`: `aux == 0` on a `Liquid` cell means **full**, not empty.
  Writing a literal 0 fill manufactures a full cell out of nothing.
- `MAX_REACH == CHUNK_SIZE / 2` exactly, and that equality is load-bearing for
  `parallel.rs`'s cross-chunk write-safety proof *and* for its
  reinsert-then-replay loop. Changing it needs both re-derived.
- **A commit message is not evidence the change is in the file.** A `git
  stash` cycle restored an older blob over a source file, so a commit that
  claimed a behaviour change shipped only its *doc comment* — the code kept
  the old predicate, and nobody noticed for four commits, because the
  message read correctly and the tests still passed. After any stash, rebase
  or merge, re-read the function, not the diff.
- **The app locks its own exe.** While the sandbox is running, `cargo build`
  fails with "failed to remove `pixel-physics.exe`" — and so does plain
  `cargo test`, which builds the bin target to run `main.rs`'s eight tests.
  (This file said `cargo test` still works; it does not, and that cost a
  confusing ten minutes.) `cargo test --lib` works throughout, and is what
  to reach for with the app open. Separately, stale incremental artifacts
  produce bogus `LNK2019 unresolved external symbol anon.…` link errors —
  `rm -rf target/debug/incremental` clears it, and it is not a code error.
- **Never `git add -A` here.** Doing so once swept ~1,200 lines of someone
  else's in-progress work into an unrelated commit. Stage explicit paths,
  and see "Working alongside another session" above — `git add -A` is the
  symptom, a shared checkout is the cause.
- **`cargo fmt` is all-or-nothing.** `cargo fmt -- some/file.rs` formats the
  whole project, not that file — 28 files and ~3,000 lines in one go. The
  full-format pass is deliberately deferred work (`PLAN.md` issue #10) and
  CI keeps `cargo fmt --check` informational for exactly that reason, so do
  not let it ride along with an unrelated change.
- **A green suite does not prove a test ran.** Deleting an `#[ignore]` took
  the `#[test]` above it with it; the test compiled, was never collected, and
  the suite stayed green. Clippy's dead-code warning caught it, not the tests.
- **Editing an asset `.ron` does nothing until the next build.** Materials
  and species are compiled into the binary via `include_str!`; only the
  app's F5 reload reads the directory, and headless harnesses do not. A
  sweep that edits `tree.ron` and re-runs a prebuilt example produces
  bit-identical "runs" — three of them, once, before anyone noticed the
  knob was not connected. Identical output across settings is the tell;
  rebuild between sweep points.
- **A structural check scheduled mid-organism amputates it.** The organism
  support search is hop-bounded, so a check fired high in a crown reads
  everything past the span limit as unsupported and converts it to
  deadwood. Growth deliberately schedules no checks; abscission scheduling
  one collapsed every shedding sweep at every value (26x outcome
  difference from the one line, and it masqueraded as "the mechanism is
  wrong" through eight settings). Until the support search anchors
  properly, do not add `schedule_structural_check_around` to a new
  organism path without measuring what it destroys — and treat Phase 3
  damage results as contaminated by this until it is fixed.
- **Assert the property, not two instants fitted to one trajectory.**
  `a_tree_eventually_stops_growing` compared wood counts at two fixed
  frames and broke the moment genotypes were re-keyed — the tree at that
  spot became a different individual that was simply still growing at the
  first sample. A termination claim is "the count holds still across N
  consecutive windows inside a budget set from a measured curve"; it
  survives redraws and retunes because it asks the question the test is
  named for.
- The liquid heightfield bodies in `liquid.rs` are **test-only today** —
  nothing in production promotes a body, because automatic promotion was
  implemented and reverted over a real architectural gap. Bugs in that
  subsystem are latent, not live, and go live the moment promotion lands.
