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
  fails with "failed to remove `pixel-physics.exe`"; `cargo test` still
  works. Separately, stale incremental artifacts produce bogus `LNK2019
  unresolved external symbol anon.…` link errors — `rm -rf
  target/debug/incremental` clears it, and it is not a code error.
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
