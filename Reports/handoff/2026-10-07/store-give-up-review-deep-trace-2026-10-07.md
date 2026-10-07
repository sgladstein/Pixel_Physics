# Review: each ant gives up the store on its own (Deep trace, 2026-10-07 06:15 UTC)

Second key on Nest race's `store-give-up-proposal-2026-10-07.md`.

- **Runs.** Measured on my reruns of `b30smell` s1 and `skysmell` s1 (build 95d65cd66, to 75k). Nest race's s2 and
  heap-90 runs I have not read.
- **Labels.** Measured unless a line says inferred.
- **No fixes.** I propose none.

## Verdict

**Not yet a yes.**

- **The shape is right for heap 90, seed 4.** There, ants are held at the store until they starve, and the rule is the
  right shape for that once its clock is fixed (section 4, points a-c).
- **But half the case rests on seeds 1 and 2, and seed 1 shows no hold.** The ants that starved there were not held by
  the store.

**Correction to my message of about 06:00.** I wrote that energy at release decided who got out, and that letting ants
go earlier would help. Both are wrong readings of seed 1:

- the store was not holding those ants;
- their energy at the last store pull measures how long they had already been failing to climb out (section 1).

## 1. Does the rule address what the traces show?

### Seed 1: no. The store was not holding the ants that starved

**What the 42 starvers were doing.** This covers the 5,000 frames before their last store pull (`releasefunnel.py
--before 5000`):

| What they were doing | Share of decisions |
|---|---|
| On the store pull | 3% (median share per ant: 3%) |
| Carrying soil out | 25% |
| Hungry walk out | 18% |
| Standing (lost move rolls) | 52% |

- They held soil 45% of the time.
- Their first hungry walk-out pull came a median 4,150 frames before that last store pull.
- Their energy fell from 0.89 to 0.43 of a grant over those 5,000 frames.

**Their final hunger episode** (`storewait.py`):

- From first being drawn to death took a median 4,632 frames.
- Only a median 80 of those frames were spent on the store pull (p90: 100).

**The survivors let go in the same window looked the same beforehand.**

- They spent 3% on the store pull, 21% carrying soil and 19% on the walk out.
- Their energy fell from 0.89 to 0.45.

**What this means:**

- The "release" at 64.5k was the store pull flickering back on briefly, when the store came back to 8 cells at 64k.
- It flickered over ants that had already been failing to climb out for about 4,000 frames.
- A per-ant patience would not have fired for them. Had it fired, it would only have put them on the walk out, where
  they already were.
- What did hold them was the climb out of the deep room, first with soil and then hungry (`sky-meal/body-s1-trace`
  sections 2-3).
- Inferred, not traced: they were drawn to the bottom by the store pull at 58-59k plus the hatch wave.

### Seed 2: probably the same as seed 1. Check before using it as evidence

47 of 47 starvers had their last store pull at 104,626-104,726, at 0.43 of a grant. That is seed 1's shape.

To check it, run `releasefunnel.py RUN 104000-106000 --to E --before 5000`. If both of these hold, seed 2 is seed 1 and
the rule does not address it:

- the starvers spent a few percent of their time on the store pull;
- their first walk-out pull came thousands of frames earlier.

### Heap 90, seed 4: yes in shape. This is the case the rule is for

**125 of 125 starvers on the store pull 10 frames before death is a hold.** But the eat pull fires only while the ant
has not yet arrived: `store_inward` returns None at store distance 0. So these ants were still walking toward the store
when they died, and never reached it.

**That is an access question, and it needs its own trace:**

- What stops them arriving?
- Can they reach the store's cells at all?
- Does the 8-cell count include cells they cannot eat?

The rule treats the symptom. It will let these ants go, but the store keeps drawing new ones in, each held for one
patience.

**To check the rule fires for them,** run `storewait.py` on that run. It does if the starvers' "on the pull" clock
runs into the thousands. Their energy at the 500, 1,000 and 2,000 marks is the energy they would be let go at.

## 2. Is 1,000 frames sound, and are the sweep points right?

**Successful waits are short on seed 1** (`storewait.py`, hunger episodes first drawn by the eat pull, followed to
75k):

| Drawn at | Fed deep | Frames from first drawn to food, median (p90) | Frames on the pull, median (p90) | Still unfed after 1,000 frames from first drawn |
|---|---|---|---|---|
| 50-58k (store 16-36 cells) | 388 | 75 (396) | 25 (100) | 6 |
| 58-66k (store eaten down, and the edge) | 795 | 70 (375) | 20 (80) | 23, then at 0.48 of a grant |

**What 1,000 frames means against those waits:**

- It is 2.5 times the p90 of a successful wait.
- Counted from first drawn, it would turn away 6 of 388 and 23 of 795 ants that ate deep later.
- Counted as time on the pull, it would turn away none of them.

**Burn rate.** Over 1,000-frame stretches without food at 55-70k, an ant loses a median 0.10 of a grant (p90: 0.33).

- Ants holding soil lose 0.14; empty ants lose 0.09.
- So "about one grant in about 10,000 frames" is right on seed 1.
- But it does not follow from the figures the proposal quotes: 0.43 of a grant at release and 1,800 frames to death
  gives about 4,200 frames per grant. Quote the measured rate instead.

**The sweep.** 500 and 2,000 bracket 1,000 sensibly. Set the default from `storewait.py` on heap 90, seed 4, the case
it is for:

- long enough to clear the p90 of fed-deep waits there;
- short enough to let starvers go above about half a grant.

## 3. Should gate=1 be in the first test, or wait?

**Wait.** gate=1 changes who is drawn in, which is a second factor.

Read `on` first:

1. Did the latch fire? It needs a counter (section 4, point e).
2. At what energy did it let ants go?
3. Did "starved, never reached the top" at heap 90 move?

Only then add gate=1, compared against `on`.

**A risk to trace before gate=1 counts as dropping the global count (inferred):** with gate=1, a store of one or two
crumbs draws every hungry ant in at the same rebuild. Patience then lets them all go together after one patience. That
is a group release again, only at higher energy.

## 4. Anything in the code being missed?

### a) The wait at the store itself is not on the eat pull

- `nest_store_pull` returns the Eat pull only while `store_inward` has a step to give.
- At the store it returns None. The ant then scores "none" or stands.
- What keeps it there is `store_feeds_here`, which blocks `hungry_out_pull`.
- So a clock that runs only "while on the eat pull" never counts the wait at the store.
- It must also run while the ant is held: hungry, empty, on the way, and within smell of a store that can feed.
  `store_feeds_here` already tests the last three.

### b) Where the clock starts

**As written,** the clock runs from the ant's last food. So any ant under its grant for more than 1,000 frames is
latched the first time the store draws it, whatever kept it from eating. That includes:

- a forager coming home empty;
- a digger that got hungry at the face;
- an ant that climbed out and fell back.

Under NEEDS_FIRST, every ant under its grant counts as hungry.

**If the rule means "an ant that waits at the store without eating",** count only the time drawn or held since its
last food.

- Add it up across spells, so a swing between the store pull and the walk out does not reset it.
- `storewait.py` prints both clocks side by side.

### c) Newborns

Start the clock at birth. Newborns start at a median 0.99 of a grant and count as hungry under NEEDS_FIRST. With the
clock as written, a newborn not fed within 1,000 frames is latched.

### d) Out of the nest

- **Do not test it with `under_cover` or `inside_nest`.** Both are roof tests, and they are true in the spoil mound.
- **Use one of these instead:** off the way (`nest_way_near(..).at(head)` is None), or depth under the ground line.
- **Say whether clearing the latch restarts the clock.** If it does, an ant can swing between the door and the store
  once per patience. Count latches per ant.

### e) A counter

- Add a count of latches set to `creature_stats`, and a latch column to the hungry log. Whether the rule fired needs a
  count. It cannot be inferred from the last store pull once the rule changes when that pull fires.
- Check the latch before the eat pull is offered. Then a latched ant never logs "nest store", and `releasefunnel.py`'s
  "last store pull" still marks the release.

### f) Soil

`hungry_out_gain` returns None while the ant holds soil. So a latched ant carrying soil is not walked out.

- On seed 1, the starvers held soil 45% of the time in their last 5,000 frames before release.
- Fixing that is not this rule's job, but the test should count it.

### g) The meal part

A bite kept under `keeps_home_meal` is crop food, so it resets the clock, as proposed. That is fine.

## What I would say yes to

**The rule, if all of these hold:**

- its clock counts time drawn or held since the last food;
- the clock starts at birth;
- it has a counter and a log column;
- it is off by default.

**The test:**

- Heap 90, seeds 1-4: the case it addresses.
- Heap 30, seeds 1-4: to show it costs nothing where it should not fire.
- First judged on starvers at heap 90 that were held, or that never reached the top.
- Its cost counted as fed-deep waits cut, from `storewait.py` on both arms.

**The proposal's text** should:

- drop seed 1 as evidence;
- drop seed 2 too, unless `--before` shows a hold there;
- say it does not address the climb at heap 30, on seed 1 as well as seed 4.

## Files

**In `store-arms/sky-meal/tools/`:**

- `releasefunnel.py`, updated: `--before N` shows what released ants were doing before release.
- `storewait.py`, new: hunger episodes drawn by the store, with two clocks, outcome, and what each patience would cut.

**Tables:** `store-arms/sky-meal/release-funnel-s1-deep-trace.txt` (a section has been added).
