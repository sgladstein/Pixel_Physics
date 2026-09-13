# The evolution lab, round thirty-three: the brief

**Handed over by round 32's coordinator, 2026-09-13.** Round 32's record is
[`evolution-lab-round-32-2026-09-13.md`](evolution-lab-round-32-2026-09-13.md).
**Round 32 was the performance round and it did the measuring; this round does
the building.** Read §1 and §1a of
[`evolution-lab-playtest-2026-09-13.md`](evolution-lab-playtest-2026-09-13.md)
and then [`evolution-lab-creature-cost-2026-09-13.md`](evolution-lab-creature-cost-2026-09-13.md)
before task 1. Both are on `main`.

**The owner's priority order, in his own words, still stands:** performance
first (*"the biggest issue is the performance after the creature numbers get
high and that is our #1 priority by far"*), then the zoom lane (*"a priority
for me too. Below performance improvement"*).

**Trunk state this was written against.** Round 32 landed **#385, #387, #388,
#389**, and **#386** was accepted and waiting only on a register merge conflict
— **check whether it landed before assuming any spoil number**. The suite was
**1,701 passed / 0 failed / 85 ignored** on the merged combination.

## Task 1 — parallelise the creature pass. This is the round.

**Round 32 measured this rather than guessing it, and it is the largest single
item on the table**: one rayon thread against four is worth **1.09x on the
background and 1.20x on the ants**, and at the owner's population **~86% of the
frame is running on one core**. Everything else in `creature::tick` is diffuse —
**57,314 instructions per decision split five ways with no term over 31%** — so
**there is no serial lever that halves it**. Round 32 looked; that is the
finding, not a gap in it.

**What makes this hard is not the parallelism, it is the write path.**
`creature::tick` is driven by the active-site scheduler and creatures mutate
shared world cells. `src/sim/parallel.rs`'s four-pass checkerboard is the
existing answer for the cell sweep and is the first thing to read. **The
determinism requirement is not negotiable** (same-build, `PLAN.md`): whatever
lands must reproduce `lab_cost`'s world **and** field hash. Round 32's own
gate values, with its sensitivity checked rather than assumed, are in #388.

**Calibrate above 800 ants.** Below the knee an ant costs ~0.3–0.5 µs and any
harness reads as broken. `examples/antcost.rs` is on `main` and already does
this. **`found_colony_of` saturates a 512-wide bed at ~460 ants** — that did not
block round 32, because its bed is past the knee from 100, but a lane that
needs 2,500 must widen the bed rather than found harder.

**Quote the whole-frame figure, paired and alternating**, never a sub-phase:
this repo has a measured case of a change that removed 91% of a phase's work and
made the frame *slower*.

## Task 2 — the cheapest real millisecond, which is already built

**`PIXEL_PHYSICS_MOISTURE_MARKS=cells` exists and is worth 1.21–1.40x
whole-frame** (`evolution-lab-frame-cost-2026-09-01.md` §17.3). **It needs only
the seed sweep its own report says is owed.** This is small, it is measured, and
it is the best ratio of value to risk in the round.

**Know what it is and is not**: an **intercept** change. It moves the
millisecond *under* the 86%, not the 86%. At 3,000 ants that is a small share —
which is exactly why it should not be allowed to consume the round, and why
task 1 outranks it.

## Task 3 — the three zoom cards, and the default that follows

**Three cards are open on `board=zoom`** and the zoom buffer currently ships
**defaulting off** because the lane's own cards disagree:

- **lab** `20260913T083914900Z-764956` — the finer buffer plainly wins.
- **outdoor** `20260913T083948135Z-0f4767` — **not obvious**: the per-cell shade
  jitter that makes stone read as stone is drawn at 4x4 blocks today and goes
  **smooth** at one cell per pixel. More correct, arguably less characterful,
  against a house style that is chunky on purpose.
- **the real game**, x1 vs x2, HUD included — `20260913T100843436Z-de27a0`.

**Collect them with `review.py get <id>`, never off `inbox`.** If he has
answered, set the default accordingly and say so in the PR body. **If he has
picked the finer buffer, `bin/lab.rs` is the follow-on** — it has its own draw
path and HUD and does not have this yet, **and the lab is where the win is
clearest**.

**Do not re-argue the default from first principles.** *Ship everything on*
governs **behaviours**; this is a **look**, and a look with a disagreeing pair of
cards is his call.

## Task 4 — the one surviving knee candidate

**Three of five were struck off by round 32** against its own §2/§3: not
per-tick allocation, not the active-site list, not an O(ants)-per-ant lookup.
**The one left is cache residency**, and callgrind **cannot see it** because it
counts instructions and not misses. **`--cache-sim=yes` either side of the knee
is the named next measurement.**

Worth doing only if task 1 does not already dissolve it — **a pass spread over
four cores has a different residency story**, so take this *after* task 1, not
beside it.

## Task 5 — make the land come back faster. The owner has ruled.

**Asked at the close of round 32 and answered, 2026-09-13: *"I would like it to
be faster."*** So this is no longer a judgement waiting on him; it is work.

**What is settled**: the land *does* recover — bare ground outside the nest
goes **2% → 56% → 6%** and plants end at **409 against an original peak of
277** — and it takes about **320,000 frames** from the trough, more than half
his 560,000-frame session. **He wants that shorter.** Do not re-open whether it
recovers; that is measured.

**`Reports/plant-reseeding-2026-09-03.md` already measured four causes ahead of
dispersal, and dispersal is only a 1.4x effect. Treat them as one problem** —
fixing dispersal alone buys 1.4x against a 2.8x predation loss:

- the germination gate opens on **two materials in the whole set** (three now —
  `spoil` declares `water_capacity: 1000`);
- the grow lamps leave **32-column dead bands**;
- the colony is a **seed predator**, cutting the stand **2.8x**;
- the largest single sink is seeds stuck **on the parent plant** — 183 of 332
  standing seeds, because a seed does not fall through branches while a
  windfall does, **an inconsistency that was never designed**.

**Use #376's method**: paired arms on the same seeds, a sign test over twelve,
read at **200,000 frames** rather than 120,000 — the same comparison is 7 of 12
and p = 0.77 at the shorter length. **And re-run the dig-gate arm with whatever
you change**: the gate and these four causes all act on the same standing-plant
count, so a fix measured against a trunk with the gate on is not measuring
itself alone.

**The bar is his, not a number**: the stand should read as coming back within a
session he is actually playing, not within one he has to leave running.

## Standing, and not to be re-derived

- **The creature cost is not linear.** ~0.3–0.5 µs/ant below ~400, **2.6–2.9
  µs/ant above ~600**. The played log's `2.1 µs/ant` averages two regimes and
  describes neither. **The ~1.0 ms floor is measured, not fitted.**
- **It is ant count, not session age** — 51 later-and-cheaper pairs in the
  owner's own log. *Round 32's coordinator asserted a headless sweep was needed
  to settle this and was wrong; the log answered it.* **Look before you build
  the instrument.**
- **A repair can remove the picture and leave the mechanism**, and an inherited
  census will hide it. **Census the mechanism, not its consequence.**
- **§Z13 is closed: the marked ants are resting**, never asked to move. Do not
  build a fourth idle animation.
- **§Z18's floating-debris complaint is still unreproduced on any bed here**,
  and the owner's own verdict on the rendered bed was *"everything looks normal
  in all of these pictures"*.
- **Verify live.** The zoom work passed 1,687 tests and **panicked in the real
  app**, because every test applied the budget before drawing and `main.rs` did
  not.
- **The review queue is for visual evaluations only.** Anything else routes
  through the coordinator.
- **Before filing a bug: `python3 scripts/bugindex.py --branches`, never
  `--check`.** The register's conflicts are almost always inside its generated
  index — resolve the *hunk*, then run `python3 scripts/bugindex.py`; never
  `git checkout --theirs` the whole file.
- **The coordinator note is 13,629 B against a 12,000 B advisory cap**, after
  archiving rounds 29–30 and compressing round 31 by 1,666 B. The remainder is
  live rulings from four rounds. **Archive rounds 31 and 32 at this round's
  close** — that is the structural fix, and prose-golf is not.

## Open, and the owner's to decide

**Is ~320,000 frames too long for the land to come back?** His own log refutes
the premise it was first raised on: bare ground outside the nest goes **2% →
56% → 6%**, and plants end at **409 against an original peak of 277**. It
recovers. **Whether that is too slow is a judgement, not a bug — do not build
against it until he says.** Asked at the close of round 32; still unanswered.
