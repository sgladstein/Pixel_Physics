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
| `PLAN.md` | Roadmap, settled decisions, the issues backlog; the append-only progress log lives beside it in `PLAN-log.md` |
| `Reports/README.md` | **The index of every design report**, with per-report status and an in-flight section for documents still on unmerged branches — check a report's standing there before trusting it or writing a new one |
| `Reports/dead-ends.md` | **Tried-and-reverted approaches** (542 at last census), each with the condition its rejection depended on and where the full record lives — grep your area before proposing or retrying anything in it |
| `Reports/open-bugs-handoff.md` | **Open bugs.** Working reproductions and what has been ruled out *by measurement*. Read this before touching a listed area. (`dead-ends.md` owns "was this tried?"; this owns "is this broken?") |
| `Reports/design-philosophy.md` | Settles arguments about constants, hardcoding, and scope boundaries |
| `.claude/skills/review/SKILL.md` | How to put an artifact in front of the owner and get a verdict back — the primary feedback channel, used constantly |

**Which rules apply to what you are doing right now** (all in this file
unless named otherwise):

- Running a parameter sweep → *When every setting of a sweep fails the same
  way* (Method), *A change that moves nothing* (Conventions), and the
  `include_str!` gotcha. If it censuses a collapse, also *A cascade
  censused before it settles* — `seedsweep.sh`'s default frame budget
  stops mid-cascade.
- Writing or trusting a guard test → the guard bullets in Conventions
  (*fail for the replacement*, *inputs actually vary*, *superseded tests
  keep passing*) and *A green suite does not prove a test ran* (Gotchas).
- Measuring liquids, powders or destruction → *Metric traps*, *A mean over
  events is not the size of the pieces* and *Chunk decomposition is a
  recurring root cause* (Method).
- Adding per-cell work to the sweep → *Guard hot-path work at the call
  site* (Method), and README's Performance section on sweep-scale costs.
- Touching organism code → the structural-check amputation gotcha, *A
  traversal must use the same neighbourhood the writer used* (Gotchas),
  and *A channel that oscillates by design* (Method).
- Proposing, building or retrying any mechanism → `Reports/dead-ends.md`
  first.

**Source comments are load-bearing.** They record *why*, including approaches
that were tried and reverted and must not be retried. Do not strip them when
editing nearby code, and add to them in the same voice when you learn
something that cost effort to find.

## Commands

```
cargo test -- --skip root_and_shoot_branching_read_different_slots   # unit + integration; the --skip is not optional -- see the red-suite gotcha
cargo clippy --all-targets -- -D warnings        # CI gates this
cargo run --release --example ascii              # headless behaviour + worst-frame timing; CI runs it
cargo run --release --example filmstrip -- scene=fall zoom=2 crop=0,140,256,110
python3 scripts/review.py serve --open      # the owner's review queue; see below
python3 scripts/review.py serve --lan       # ...also reachable from a phone on the same Wi-Fi
bash scripts/acceptance.sh                  # the structural acceptance cases; CI gates this
bash scripts/seedsweep.sh                   # the order-statistic seed sweep; run BEFORE changing any model over procedural content
bash scripts/docscheck.sh                   # documentation checks: links, map-vs-tree, freshness notes, report index
bash scripts/branchcheck.sh                 # how far behind main this branch is, and which branches are merged-and-deletable; --gate is the CI trunk check
```

**The real app can be screenshotted headlessly**, which this file previously
assumed impossible — every "verify live" instruction routed through
`filmstrip` because the sandbox has no display. On a headless Linux box:

```
apt-get install -y libxkbcommon-x11-0 mesa-vulkan-drivers   # once per container
xvfb-run -a -s "-screen 0 1280x800x24" \
  env VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
      PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=3 \
  ./target/release/pixel-physics                            # writes %TEMP%/pixel_physics_screenshot.png
```

`lvp_icd.json` is lavapipe, Mesa's software rasteriser; without it `Pixels::
new` fails with "Unable to create a surface" and the panic looks like a code
bug rather than a missing driver. It is slow — seconds per frame — so it is
for *looking at one frame of the real thing*, not for timing anything. Frame
timings still come from `ascii`, and `PIXEL_PHYSICS_CAPTURE_SEQUENCE` still
works for a strip.

`filmstrip` writes a contact-sheet PNG — several frames of one run in a grid —
so an artifact can be judged by eye without a window. Add `gif=1 out=x.gif` and
it encodes an animation instead, still with no window and no GPU: reach for that
when the question is whether something *moves* right, which a grid of stills
cannot answer. For the real app, press
`F7` to the `flat` preset — dead-level bare rock with 200 rows of sky, the
structural test bed — or set
`PIXEL_PHYSICS_CAPTURE_SEQUENCE=<start>,<interval>,<count>`; frames and a GIF
land under `%TEMP%`.

**Having rendered something, show it — don't describe it.** See *Getting the
owner's judgement* below; it is not an occasional tool.

## Getting the owner's judgement

**`scripts/review.py` is the primary way to get feedback from the owner, and it
is meant to be used constantly — not saved for big moments.** Everything this
project optimises for is judged by eye: whether a collapse *feels* like
destruction, whether a fall reads as sand, whether a pool looks flat while it is
still moving. None of that is a test result. Describing it in chat has
repeatedly failed — three separate models were overturned only by the owner's
playtest reports, and nearly every fix judged by test output alone left the
screen unchanged.

So when a change is visible, **post it rather than describe it**. "This looks
better" is precisely the claim the owner has to check, and a sentence is not
checkable.

Post when:

- a change alters anything on screen — including one you are confident about;
- you are about to claim something looks, moves or feels better;
- a complaint could mean two things — render both readings and ask which one it
  is, rather than spending the whole detour on the wrong one;
- a step is "judge by eye" — post *before* declaring it done, not after;
- you are choosing between approaches and the difference is visual: post a blind
  A/B (`review.py ab --blind`) instead of arguing it out. Blinding costs you
  nothing, because the stored verdict names the real option.

Posting is **fire-and-forget**: post, carry on, and collect the verdict with
`review.py inbox` later or in a later session — run it when you pick a thread
back up, including at the start of a new one. Do not stall a session waiting.
`--wait` exists for one case only: a wrong guess would waste the work you are
about to do. A `--wait` that times out changes nothing — the card stays queued
and answerable — so it is only ever your own time at risk.

Two house rules, both from failures already paid for here:

- **Put the discrete event count in the card's `meta`.** The page renders it
  directly under the image. A collapse once read as "chunks are working" from a
  picture whose body count was zero for the whole run; an image says *what* and
  *where*, and only the number says *whether it fired*.
- **Prefer a paired comparison** over one run against a remembered impression.
  Outcomes here have enormous spread, so a single run is a sample from a wide
  distribution.

The owner may be reading the queue on a phone (`serve --lan`), where a card is
one column and an image is judged in the full-screen viewer. Nothing about
posting changes; it is one more reason the card must stand on its own — a title,
a question, and the count in `meta`.

The queue is shared by every worktree of the clone, so a card posted from
`.claude/worktrees/foo` and one from the main checkout land in the same place,
and an answer outlives the session — and the worktree — that asked for it. The
owner views it with the `serve` command above; it does not need to be running
for you to post. Full protocol, including the JSON card spec, in
`.claude/skills/review/SKILL.md`.

## Working alongside another session

**This tree is worked in concurrently, and often by more than one agent at
once.** Git handles the merges; what it cannot handle is the three failures
below, each of which has cost real hours.

**`main` is the trunk. Never integrate against `master`.** `main` began as a
15-byte stub while the project lived on `master`, and the fix was to copy
`master` onto `main` — but the copy left `master` standing, so for a while
both names looked equally plausible and the docs named both. `3d53351`
records the result: a branch merged `master` while `main` was 10 commits
ahead, and silently missed the CLAUDE.md restructure, the map-scroll feature
and the play-button fix. Nothing failed; the session noticed because a diff
made no sense. `main` is the GitHub default, is the only branch CI gates, and
is what the reset procedure below names. `master` is a mirror with nothing of
its own and is on its way out. `scripts/branchcheck.sh --gate` fails if any
commit is ever reachable from `master` but not `main`, and CI runs it.

**Know how far behind you are, before you trust anything you measured on
it.** The worktree rule below keeps two sessions from breaking each other's
build; nothing was ever written down about staying current, so nothing pulled
a branch forward and the drift compounded silently. Measured 2026-08-22
across 27 remote branches: one was current, **ten sat at exactly 160 commits
behind `main`** — cut at the same moment and never once updated — and twelve
more were already fully merged and still standing as clutter. A branch does
not notice it is 160 behind. The merge does, and by then the conflict surface
is the whole session rather than a file or two.

So: run `bash scripts/branchcheck.sh` when you pick up a branch, and pull
`main` in *while* you work rather than saving it for the end. This is not
tidiness — a baseline measured on a 160-behind branch is a measurement of a
tree nobody else has, and the numbers in a report written from it do not
transfer. The one exception the script prints for you: a branch sharing *no*
history with `main` is a deliberate orphan carrying data, not source
(`review-queue`, the review queue's transport — see `review_lib.py`'s
`SYNC_BRANCH`). Never merge `main` into one of those.

**Open a pull request for your work. This paragraph is the owner's standing
authorisation, given 2026-08-23 — you do not need to ask again.** The agent
harness declines to open a PR "unless the user explicitly asks"; this is that
ask, and it stands for every session in this repo. Nothing in the repo ever
said otherwise, which is why sessions kept reporting they had been told not
to: they were reading their own harness, not this file.

What it cost to leave unsaid, measured 2026-08-23: **133 CI runs, every one on
`main` or `master`. Zero on any feature branch, zero from a `pull_request`
event.** No PR ever existed, so the workflow's `pull_request` trigger never
fired and pushes to `claude/**` matched nothing — the first time CI saw a
branch's code was *after* it landed, when a red suite can no longer tell you
whether the branch broke it or the merge resolution did. And a branch nobody
can see is a branch nobody merges: 27 accumulated, ten of them cut in one
fan-out and never once pulled forward.

**When to land**, from this repo's own 49 two-parent merges, each replayed
with `git merge-tree` to count the conflicts it actually produced:

| | |
|---|---|
| `behind x files > 300` | past the point where merges get expensive — act |
| feature complete | open the PR |

Every painful merge in that history (3+ conflicts) scored **above 340**; no
clean merge exceeded 1440 and the clean ones reach p90 at **280**. So 300 sits
in the gap rather than on a measured value. `bash scripts/branchcheck.sh`
prints your two numbers.

**The two terms want different remedies, and this is the part that gets
confused.** If `behind` is driving the product, merge `main` in — that fixes
it in place, costs nothing you were not going to pay at landing time, and
landing does *not* reduce drift: a 337-behind branch that opens a PR still
owes the same 337-commit reconciliation. If `files` is driving it, the branch
has quietly become more than one feature; land it and start another.

Do not read "land early" as "land broken". A half-finished `src/sim/load.rs`
on `main` costs every concurrent session, because they all build on it and
every measurement taken against it is void. And the fastest way to satisfy a
"commit and push now" impulse is `git add -A`, which is banned here for a
reason recorded below. Stage explicit paths, green the gates, then land.

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
worktree at `origin/main`, re-apply your own change there, verify, commit
and push from it, then bring the main tree's branch pointer forward with
`git reset --mixed origin/main` — which moves the branch and leaves their
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
the recurring mistake — and the inverse bites too: a corrected overlay was
still misread as "everything at the ramp floor" when the real value was 40%
of scale, genuinely hard to judge on a one-cell-wide twig. Pair every debug
channel with a probe that prints the values (`examples/plant_probe.rs`),
and reach for it the moment the question turns quantitative.

### "Did it fire at all" needs a counter, not a picture

An image shows
*what* and *where*; it cannot show whether the thing you built is what
produced it. A collapse rendered as coherent falling slabs, was read as
"chunks are working," and the harness's own body count said **zero for the
whole run** — the feature had never once executed, and what was on screen
was loose rubble that happened to hold its shape. Two very different
mechanisms look identical at the zoom a contact sheet is read at. When a
change adds a discrete "this happened" event, print the count next to the
image and read both.

### Check that a planned step can demonstrate itself, before promising it will

"Cracks weaken rock" was scheduled as an independently shippable,
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

### Ask which *pixels* a lever moves, before ranking it by silhouette

The sibling of the question above, and it has now cost a whole phase. Three
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

### Resolve an ambiguous complaint before building anything

"Flatness at rest" was read as the surface texture and turned out to mean a screen-wide
tilt — opposite directions, a whole detour spent on the wrong one. When a
report could mean two different things, the cheap move is to measure both
and see which one is actually there, or to ask. It is much cheaper than the
fix you build for the wrong reading.

### Ask what a metric counts when nothing is wrong

The whisker hunt defined
a "film" as a water cell with air above and below — which is *what falling
water looks like*, so the metric counted every droplet in the world. Its
numbers were real and meant nothing. Sanity-check a new metric against a
case you know is fine, before trusting it about a case you don't.

### When the complaint is visible and persistent, measure the standing state, not the event rate

Attributing film *creation* blamed the
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
as "the approach is wrong" and was not: a rider that had landed *with* the
mechanism was constant across every run and was alone the collapse (the
full account, with its numbers, is the structural-check amputation gotcha
below). A sweep only varies the knob; anything that rode along with the
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

### A cascade censused before it settles reads a *delay* as damage

Both runs have to have **landed**, or the census is comparing mid-air with
on-the-floor. A change that made a room stand two hundred frames longer
measured as `roomcut` losing 251 cells against 1,501 at frame 202, and as
235 against 273 once both runs were given 1,500 frames — a disaster and a
rounding error, from the same two binaries.

**`seedsweep.sh`'s own default does this**, still, today. `FRAMES="start=2
every=400 count=4"` stops at frame 1,202, which is mid-collapse. Measured on
`scene=worldcrack strike=12`, one build, eight preset/seed pairs, frame
1,202 against frame 3,602:

| | rock destroyed @1,202 | @3,602 |
|---|---|---|
| `terraced 1` | 557 | **1,042** |
| `terraced 7` | none — rock *gained* 647 | **260** |
| `flat 1` | 20 | **199** |
| `rolling 7` | none — rock *gained* 223 | **88** |

Four of the eight destroy rock by 3,602. **The default misses two of them
outright** — it reads rock as net *gained* where the collapse has not yet
arrived — and understates the two it does see, by 1.9x on `terraced 1` and
**10x** on `flat 1`. `terraced 7` reverses outright: −634 cells lost at 1,202
becomes +326 at 3,602.

**Read `rock`, not `cells lost`, for the settling question.** `rock`
plateaus — `terraced 1` runs −952, −1,042, −1,042, −1,052, −1,052 across
frames 1,802 to 9,002 — while `cells lost` never settles at all: the same run
goes 849 → 1,109 → 745 → −126 → −1,322.

**That drift is an oscillation, not accumulation, and it is not the cascade.**
An earlier version of this section blamed it on weathering accruing rubble.
The control that settles it is the same scene with **no verb at all**: at
`strike=0`, `terraced 1` reports **zero failures and `rock +0` at every tile**
while `cells lost` swings 0 → 290 → 471 → 44 → −725 → −1,684. Nothing broke,
so no rock became rubble; the rubble census is simply riding something
periodic, and on `wetland` the `rock` column matches the frozen-water count
exactly — `rock +833` against `833 frozen` — which points at the water cycle.
Amplitude is about ±1,700 cells, **larger than most damage figures in the
sweep**, so a `cells lost` reading taken at any single frame is that frame's
phase plus the damage, and the two are not separable. This is the
*divide-the-oscillator-out* problem below, not a too-short budget — and until
it is divided out, `cells lost` cannot be used to compare two models on these
presets at all.

So `awake` and `sites` are a weaker tell than they look: on `rolling` and
`terraced` both sit near 5,000 sites indefinitely and never reach zero. The
tell that works is that **the quantity being censused has stopped moving**
across two consecutive tiles. `every=900 count=5` is enough for `rock`;
no budget is enough for `cells lost`, because the problem is phase, not length.

Worse, and the reason this needs its own heading rather than a footnote: two
runs that diverge on one frame are **different worlds** by the next, so a
single cascade scene cannot compare two models at all, settled or not. One
term measured *ten times worse* on `scene=worldcrack strike=12` and nearly
halved the worst case over 24 seeded runs. Comparisons of cascades belong in
`seedsweep.sh`, run to rest, read at the order statistic.

### A mean over *events* is not the size of the pieces

The sibling of the metric traps below, one level up, and it nearly cost a
correct change. `failing region size: mean` divides cells by **failure
events**, and a change that makes marginal rock fail more often moves that
mean without moving what comes out: `caveshallow` went from mean 10.0 to
4.1 — below `rigid::MIN_FRACTURE_CELLS` (6), which reads as *"the typical
break is now too small to fracture, so it is dust"* — while losing the
identical 64 cells of rock and sending **fewer** cells down the powder path
(30 → 16). The extra events were confined failures cracking in place, which
never reach the fragment ladder at all.

A mean cannot separate "smaller pieces" from "more evaluations of the same
piece". If the question is whether something turned to dust, count the
regions that fell below the fracture threshold and read that against the
total that failed. `FailureCounts::crumbled` counts exactly that -- the
regions `rigid::fracture_failing_region` declined and the cells they took --
and `filmstrip` prints it as `crumbled to grit` beside the mean. Read that,
not the mean, whenever the question is whether something turned to dust.

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
- **A revert keeps the knowledge — and gets an address.** Keep the
  reproduction (`#[ignore]` it if it now fails), record what the withdrawn
  fix was, what it improved, and why it went, and add the entry to
  `Reports/dead-ends.md` in the same change, with the condition the
  rejection depends on. A reverted fix's genuine improvements become the
  bar its replacement must meet — not the pre-fix baseline.
- **A new report gets its line in `Reports/README.md` in the same commit**,
  and a report that supersedes another updates the superseded line.
  `scripts/docscheck.sh` flags omissions, including a merged report still
  listed as in-flight.
- **A shipped milestone or feature gets its README status section before
  the work is called done.** Five features went undocumented for multiple
  milestones and had to be reconstructed after the fact; the Status
  section's "known limitations" also outlived two of the fixes that
  removed them.
- **A session that makes a significant change affecting a `wiki/` page must
  update that page (and its freshness note — a real date, never "this
  build", which can never go stale) in the same change.** This is a
  cheap backstop, not the real defence against `wiki/*.md` going stale the
  way early design Reports did — the real defence is that each page
  describes coarse, player-visible behavior, not implementation, which is
  inherently more stable. This rule just shortens the gap on whatever does
  drift.
- **Commit messages carry the measurement**, not just the intent: the number
  before, the number after, and what was tried and rejected on the way.
- **Determinism is required** (same-build, per `PLAN.md`) — reversed from
  "not required"; the reasoning is `Reports/emergent-world-architecture.md`
  §8. (An earlier version of this bullet warned of stale comments saying
  otherwise; a sweep found none survive.)
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
- **A *red* suite proves even less: `cargo test` stops at the first failing
  test binary, so a known-red lib test hides every integration test from a
  local run.** Bug A lives in the lib target, so plain `cargo test` fails
  there and never runs `tests/worldgen.rs` or `tests/determinism.rs` — they
  do not appear in the output at all, not even as skipped. CI does not have
  this blind spot, because its `test` jobs pass `--skip
  root_and_shoot_branching_read_different_slots`: the lib goes green and the
  integration binaries then run. That asymmetry hid **two gating failures on
  `main` for a whole day** (`Reports/open-bugs-handoff.md` §M). So run the
  gate the way CI runs it — `cargo test --release --locked -- --skip
  root_and_shoot_branching_read_different_slots` — and treat the *absence*
  of `Running tests/worldgen.rs` from the output as the tell, since it reads
  as a pass rather than an error. The general rule: **while any gate is
  quarantined, whatever runs after it is not being run locally.**
- **Editing an asset `.ron` does nothing until the next build.** Materials
  and species are compiled into the binary via `include_str!`; only the
  app's F5 reload reads the directory, and headless harnesses do not. A
  sweep that edits `tree.ron` and re-runs a prebuilt example produces
  bit-identical "runs" — three of them, once, before anyone noticed the
  knob was not connected. Identical output across settings is the tell;
  rebuild between sweep points.
- **`cargo build --release` does not rebuild the examples**, and every
  measurement in this repo comes out of an example. It builds the lib and the
  bin; `--examples` builds them all, `--example NAME` builds one — and a
  stale `target/release/examples/foo` runs happily, prints plausible numbers,
  and has a *newer mtime than the source you just edited*, so the obvious
  sanity check says it is fresh. This bit one session three times in an
  afternoon: a `viewshot` render that showed round-7 formations after the
  4x-formation change had landed; a lens-shape before/after that came back
  **byte-identical** because only the "before" had been rebuilt; and a
  `scale_probe` cell count identical to the pre-change one for the same
  reason. The tell is the same one the `include_str!` gotcha above has:
  **identical output across a change that must have moved something.** When
  you see it, suspect the binary before the code — and prefer
  `cargo build --release --examples` over naming one, because the pass you
  are about to measure with is rarely the only example you will run.

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
