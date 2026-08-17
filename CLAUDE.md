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
  it; that is the number to quote. The corollary cuts the other way too:
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

**Ask which *pixels* a lever moves, before ranking it by silhouette.** The
sibling of the question above, and it has now cost a whole phase. Three
discrete architectural levers — sympody, tropism, acrotony — were built,
ranked "very high" and "high" on silhouette impact by a botany review, and
all three demonstrably *fired*: 46–186 sympodial forks per shrub, 1,797–2,750
plagiotropic steps per conifer, counters printed beside the sheets exactly as
this file demands. The owner's reading of those sheets was that nothing had
changed, and the composition numbers agreed — the three species differed in
height and mass and in nothing else. All three levers change **which cell
gets a label**: the order a child inherits, the reference vector it scores
against, which bud flushes. The silhouette was set by two things none of them
touch — every species was ~90% wood and ~5% leaf, and every plant in the
world drew from one four-brown palette and one four-green one. A lever that
relabels a cell cannot move a silhouette that texture and colour set. See
`Reports/plant-appearance-design.md`.

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
- **Two fixes failing the same way means the approach is wrong, not the
  tuning.** Two separate attempts to penalise a cell crossing a chunk seam
  both replaced the tear with a throttle at the same seam. That is a signal
  to change the approach — the third attempt fixed the sweep *order* instead
  and cost nothing.
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
- **The harness is as stale-able as the assets it reads, and an unknown
  argument is silently ignored.** A 3.5-hour detached megastudy (3 species x
  8 world seeds x 16 plants x 45,000 frames) produced eight *byte-identical*
  logs per species: the release binary was built fourteen minutes before
  `worldseed=` was added to `plant_probe.rs`, so every run took the default
  seed and the study was 3 populations wearing 24 logs. It looked exactly
  like a study. **Rebuild before launching anything long, and make the
  harness echo its own parameters** — `plant_probe`'s first line now names
  species/trees/frames/worldseed, so a log that does not name its seed was
  written by a binary that never had one. A knob nobody can see the value of
  is a knob nobody can tell is disconnected.
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
