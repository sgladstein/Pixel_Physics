# Working in this repo

This file is for *how to work here*, not what the code does. The codebase is
already heavily documented and the architecture is written up at length
elsewhere — see below. What is not written down anywhere else, and what this
project keeps re-learning the expensive way, is the method.

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
| `PLAN.md` | Roadmap, settled decisions, the issues backlog |
| `Reports/*.md` | Design records and research, one per subsystem |
| `Reports/open-bugs-handoff.md` | **Open bugs.** Working reproductions, what has been ruled out *by measurement*, and what was tried and reverted. Read this before touching a listed area. |
| `Reports/design-philosophy.md` | Settles arguments about constants, hardcoding, and scope boundaries |

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
so an artifact can be judged by eye without a window. For the real app, set
`PIXEL_PHYSICS_CAPTURE_SEQUENCE=<start>,<interval>,<count>` and edit the scene
into `build_terrain`; frames and a GIF land under `%TEMP%`.

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
- **Never `git add -A` here.** This tree gets worked in concurrently; doing
  so once swept ~1,200 lines of someone else's in-progress work into an
  unrelated commit. Stage explicit paths.
- **`cargo fmt` is all-or-nothing.** `cargo fmt -- some/file.rs` formats the
  whole project, not that file — 28 files and ~3,000 lines in one go. The
  full-format pass is deliberately deferred work (`PLAN.md` issue #10) and
  CI keeps `cargo fmt --check` informational for exactly that reason, so do
  not let it ride along with an unrelated change.
- **A green suite does not prove a test ran.** Deleting an `#[ignore]` took
  the `#[test]` above it with it; the test compiled, was never collected, and
  the suite stayed green. Clippy's dead-code warning caught it, not the tests.
- The liquid heightfield bodies in `liquid.rs` are **test-only today** —
  nothing in production promotes a body, because automatic promotion was
  implemented and reverted over a real architectural gap. Bugs in that
  subsystem are latent, not live, and go live the moment promotion lands.
