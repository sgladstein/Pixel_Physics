# The evolution lab, round thirty-two: the performance round

*2026-09-13. Handed over by round 31's coordinator with the owner's priority
stated in his own words and already sized by measurement, which is the first
time a round has opened that way.*

> *"The biggest issue is the performance after the creature numbers get high
> and that is our #1 priority by far."* — the owner, 2026-09-13

Brief: [`evolution-lab-round-32-brief-2026-09-13.md`](evolution-lab-round-32-brief-2026-09-13.md).
Measurement of record: [`evolution-lab-playtest-2026-09-13.md`](evolution-lab-playtest-2026-09-13.md),
over his own 560,000-frame session.

## What the round was aimed at

**cost ≈ 1.0 ms/tick + ~2.1 µs per ant per tick.** At 3,000 ants the creatures
are **86% of the frame** and everything else together is about a millisecond.
Not the cell sweep, not the renderer — both refuted from the same log. **The
number to beat was 2.1 µs per ant per tick**, because halving it roughly
doubles the population he can play at.

## The lanes

| lane | job | model |
|---|---|---|
| A | the late-game creature cost — reproduce 2.1 µs headless, localise it inside `creature::tick`, cut it | Opus 5 |
| B | the five dead census columns, which gate every future log he sends | Opus 5 |
| C | the spoil teleport (PR #221, open since 2026-09-03) and whether §Z13's idle ants are resting or stuck | Opus 5 |

Three lanes rather than round 31's five, on the owner's conserve-tokens ruling
and with the account's seven-day limit at `allowed_warning`.

## What the coordinator checked before briefing anyone

**The round's own premise, refit from the raw log.** Three lanes were about to
be pointed at `2.1 µs per ant per tick`, so it was worth one command to see
whether that number survives being re-derived by someone who did not write it.
It mostly does — and the way it fails is the useful part. The full working is
[§1a of the playtest report](evolution-lab-playtest-2026-09-13.md); the short
form:

- **The ~1.0 ms floor is measured, not fitted.** Frames 10,000 / 20,000 /
  30,000 carry `ants 0` at 1,100 and 1,000 µs/tick. A least-squares line
  through the envelope puts the intercept anywhere between **234 and 930 µs**
  depending only on the number of bands — so the one quantity a regression is
  worst at here is the one that did not need regressing.
- **The slope is two slopes.** Below ~450 ants an ant costs **≈ 0.27 µs**;
  above ~600 ants it costs **≈ 2.6–2.9 µs**. The single 2.1 figure is an
  average across a bend and describes neither regime.
- **The slope estimate is otherwise robust** — 4/6/8/10/12-band envelope least
  squares, Theil–Sen over band medians, and the chord between extreme bands all
  land in **2.1–2.8**, centred near 2.4. §1's headline is the right order and
  the right target.

**Why this was worth a message rather than a footnote.** A lane building its
positive control at 100–500 ants would have measured 0.3 µs/ant, disagreed with
its own brief by sevenfold, and gone looking for a harness bug that does not
exist — `CLAUDE.md`'s *a scene that contradicts the code will look like a bug in
the code*, arriving through the brief rather than through the scene. Lane A was
told before it built anything.

**And the knee is worth more than the coefficient.** A constant per-ant cost
says *shave the brain*. A bend says something changes **character** with
population, and it sits almost exactly where the owner stops being able to
play. **The confound is stated rather than resolved**: in his log ant count
rises with session age, so only a controlled headless sweep — population set
rather than grown — separates the two, which is the first thing Lane A was
asked to build.

## Verdicts collected, which existed only in the queue

Round 31 left two answered cards nobody had written down. `CLAUDE.md` warns
that a card can be archived carrying no stored response, so a verdict that
lives only in the queue is a verdict that is about to be lost.

- **§Z13, `20260913T052938222Z-c7bc21`** — three idle-animation candidates,
  posted *after* #380 fixed the clock so the mechanism genuinely ran. **The
  owner's entire answer: _"Are they resting or stuck?"_** He picked no
  candidate and did not say it still looks wrong; **he asked the question back**.
  That closes the look explanation as the thing to work on and makes the
  measurement the deliverable — Lane C's job 2, now with the owner's own words
  behind it. **No fourth animation.**
- **§Z18, `20260913T044639577Z-4e0ecb`** — was the floating material dug soil
  or plant tissue, with every dug cell painted orange. **The owner's entire
  answer: _"Everything looks normal in all of these pictures."_** A null on the
  rendered bed, and a standing warning against using "does the bed look wrong"
  as the acceptance test for the spoil-teleport work.

Both carry a free-text comment and `annotations: []`. **Round 31's published
claim that annotations do not survive the queue is false** — round-29 card
`20260912T045951545Z-6931d4` still returns its three markers in full through
`get`. These two simply have none.

## Open, and the owner's to decide

**Is ~320,000 frames too long for the land to come back?** The premise task 3
was written on — *the abandoned nest never regrows* — is refuted by his own
log: bare ground outside the nest goes 2% → 56% → 6% and the plant count ends
at **409 against an original peak of 277**. It recovers, and it takes about
320,000 frames to do it, which is long enough that every earlier look landed
before it happened. Whether that is too slow is a judgement, not a bug, and
nothing should be built against it until he says.

## What landed

**#387 — the five dead census columns, and there were two causes, not one.**
The brief predicted one (*"the nest band is `0/0`, so everything scoped to it
measures an empty set"*). Lane B found two and said so, which is the right kind
of disagreement with a brief. **The chronicle can describe a nest again**, which
gates every log the owner sends from here on.

**#388 — what one ant costs, and why halving it is not a tuning problem.**
The round's centrepiece.

- **The knee is confirmed independently**, at a different split from the
  coordinator's: **0.53 µs/ant below 378 ants, 2.66 µs/ant above**. And the
  lane's bed is **past the knee from a hundred ants**, which makes it *the
  defect in a box* rather than a replica of the owner's session — the better
  outcome for what the round is for.
- **The confound is settled, and from the owner's own log** — which the
  coordinator had said would need a headless sweep. It does not: frame
  **340,000** carries **2,334 ants at 6,300 µs**, frame **410,000** carries
  **1,252 ants at 3,500 µs**. Later, bigger mound, more worked soil, half the
  ants, half the cost, and **51 such pairs in the file**. Session age is not
  what is being measured. *The coordinator's caveat was over-stated: a
  monotone-looking series had non-monotone stretches in it, and looking was
  cheaper than sweeping.*
- **The cost is diffuse.** One ant's decision is **57,314 instructions** split
  five ways with **no term over 31%** — `sense` 30.4%, `step_chain` 16.6%,
  `eval_brain` 14.1%, `tumble` 12.8%, `act` 11.4%. **There is no lever here
  that halves it**, which is a real answer and not a failure to find one.
- **The lever that would: the creature pass has no parallelism at all.** One
  rayon thread against four is worth 1.09x on the background and 1.20x on the
  ants; at the owner's population ~86% of the frame is on one core. **Worth
  more than everything else combined**, and it is round 33's first task.
- Two **bit-identical** cuts landed: −1,379 instructions per decision,
  **−2.41%**. **No whole-frame speed-up claimed** — the box's own spread is
  1.39x, an order of magnitude larger — so the instruction count is the gate.
  That restraint is the reason to trust the rest of the report.
- Two hypotheses killed by measurement: the ants are **not jammed** (3.8–7.9%
  blocked against a 5.2% control), and it is **not the moisture channel** —
  a callgrind profile putting `visit_soil_water` at 37.8% was taken on a bed
  with **eight plants in it**, and soil-water visits are *highest at zero
  ants*.

**#385 and #389 — the zoom-out buffer, priced before it was built.** At the
widest rung **94% of the cells in view cannot reach a pixel**; §Z11's salience
rule decided *which of sixteen wins* and could not stop a one-cell stem being
drawn four cells wide. The buffer now grows on `+`, up to 16x the dots.
**Sixteen times the pixels costs 1.66x the whole frame**, not sixteen, because
`pixels x stride²` is constant and only per-pixel colour work grows.

**It ships defaulting off, and that is the correct reading of the ruling
rather than an exception to it.** *Ship everything on* governs **behaviours**;
this is a **look**, and the lane's own cards disagree with each other — the lab
plainly wins, while outdoors the per-cell grain that makes stone read as stone
goes **smooth**, against a house style that is chunky on purpose. **Three cards
are open on `board=zoom` and the default is the owner's to set**, which is
exactly the case the review queue exists for.

**The live check earned its place.** Run in the real app under xvfb it
**panicked** — `main.rs` sized the buffer from `viewport()` while `draw` pushed
the budget afterwards, so for one frame the two disagreed. **Every test passed
through it**, because every test applied the budget before drawing. `CLAUDE.md`'s
*verify live before declaring done*, paying for itself.

**#386 — the spoil teleport (§Z19) — accepted, waiting on a merge conflict**
in the register's generated index. A pellet was crossing **up to 116 rows with
nothing carrying it**; the fix stops the scan where the animal could not have
gone, and the median seed's longest lift falls **78.5 → 6**. It costs ~7% of
the digs and **keeps the towers** (452 → 445, where the dig-only clause takes
them to 418).

## What the round overturned

**A repair had already removed the picture and left the mechanism, and the
inherited census hid it.** #221 and §Z18 both argued from pellets standing
50–99 rows up a tree. On today's trunk that census reads **+2/+4/+3/+2 with a
tree against +3/+5/+3/+4 without** — PR #379's footing rule means a pellet
posted into a canopy falls out again. **The heap is gone; the lift was
untouched and still ran to 116 rows.** A branch that trusted the standing
census would have reported the bug fixed. Only a counter could say otherwise,
which is why `spoil_lift_rows` now sits beside `spoil_lifted`/`spoil_lift_max`:
a max alone cannot separate one freak of 107 from a colony routinely posting
pellets fifty rows up.

**§Z13 is answered, and the answer is _resting_.** All three animals the owner
marked were measured from the inside: across 3,000 frames **not one ever tried
to move and failed — they never asked to move at all**. The ordinary two-cell
ant nobody has complained about rests just as long. **What changed is not the
behaviour; the body got big enough to see.** §Z13's *"look problem, not a walk
bug"* stands and its third explanation is closed. The control that makes it
stand up: the table reads the **per-animal** `life.moves_blocked` while §Z13
reasons over the **world** counter, and both increment sites in `step_chain`
pair — so `+0` means *did not ask*, not *asked and was not counted*.

**Five census columns had two causes, not the one the brief predicted.**

## What round 33 inherits

1. **Parallelise the creature pass.** The single largest item on the table,
   and the round measured it rather than guessed it.
2. **`PIXEL_PHYSICS_MOISTURE_MARKS=cells` is already built** and worth
   1.21–1.40x whole-frame. It needs only the seed sweep its own report says is
   owed — **the cheapest real millisecond available**. It is an *intercept*
   change, so it moves the millisecond under the 86% rather than the 86%.
3. **The three zoom cards**, and the default that follows from them.
4. **`bin/lab.rs` has its own draw path** and does not have the zoom buffer,
   though the lab is where the win is clearest.
5. **A bed that reaches 2,500 ants may not be needed**: `found_colony_of` lays
   one row at body-derived spacing, so a 512-wide bed saturates at **~460**.
   The bed is past the knee at 100, so the sub-knee regime is not one that
   harness has.
6. **Cache residency is the one surviving knee candidate.** Three of five were
   struck off; callgrind cannot see it because it counts instructions and not
   misses. `--cache-sim=yes` either side of the knee is the named measurement.
