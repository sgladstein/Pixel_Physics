# The ant lifespan and every played-bed number, re-taken after the seed cull

**Lane A of round thirty-one, 2026-09-13, branch `claude/lab-lifespan-rederive-r31`,
cut from `main` at `16bab295`.** Brief: round 31 task 1.

Until #366 landed on 2026-09-12, an ant that ate a seed was overwritten by the
seed — two fifths of colony deaths on this bed were a write-site bug rather
than an ecology. Every number anyone had about how long an ant lives, how many
survive a session, and whether the bed starves its colony was measured while
that cull was running. This report re-takes them.

**The headline, in the world's words: the colony now survives the session, and
the lifespan is no longer what keeps it alive.** On twelve seeds of the played
bed the shipped colony is still standing at 200,000 frames on **twelve of
twelve**, and on twenty-six of twenty-seven runs across all six shipped beds.
Against that, the setting the last sweep credited with bounding a runaway now
buys almost nothing the population can see: turn ageing off entirely and the
colony is the same size, on the same bed, to within the seed noise. What the
lifespan still does is change *what the ants die of* — age overtakes hunger as
the leading cause of death — and it holds the floor: **halve it to 20,000 and
four colonies in twelve are gone by 200,000 frames.**

## What was run

`examples/latecensus.rs scenario=played_bed frames=200000 sample=20000`,
`RAYON_NUM_THREADS=1`, **twelve seeds**, **five arms, all out of one binary**
(`target/release/examples/latecensus`, built with
`cargo build --release --examples` and its exit code read through
`${PIPESTATUS[0]}`):

| arm | `life_half_life` | dig gate | selected by |
|---|---|---|---|
| **A shipped** | 40,000 (as authored) | on | nothing — the shipped defaults |
| **B immortal** | 0 | on | `lifespan=0` |
| **C no gate** | 40,000 | **off** | `PIXEL_PHYSICS_LAB_ROOM=off` |
| **D half** | 20,000 | on | `lifespan=20000` |
| **E double** | 80,000 | on | `lifespan=80000` |

Sixty runs, plus the shipped arm on the **other five shipped beds**
(`played_bed_longant`, `_understory`, `_flitter`, `_scrambler`,
`_windfall_reach`) at seeds 1–3 — fifteen more, for question 3, because §Z6 is
written about *every shipped bed* and one bed cannot answer it.

Four single-threaded processes at a time on a four-core box. Everything
compared here is a counter or a census, never a wall clock, so contention
cannot move it; `RAYON_NUM_THREADS=1` is pinned so nothing downstream of the
checkerboard moves with the load either.

**Raw digest, every STOP and SUMMARY line of all seventy-five runs, with each
run's own echoed parameters above it:**
[`data/evolution-lab-round-31-lifespan-sweep.txt`](data/evolution-lab-round-31-lifespan-sweep.txt).
The order statistics below are reproduced from that file by
[`data/evolution-lab-round-31-lifespan-stats.py`](data/evolution-lab-round-31-lifespan-stats.py).

### The controls, quoted

- **The positive control is the fix's own published pair.** #366 was measured
  as moving ants alive at 120,000 frames from 0 / 8 / 29 to **52 / 10 / 135**
  on seeds 1–3. The shipped arm here reads **52 / 10 / 135** on those seeds —
  exactly. The harness, the bed and the trunk are the ones that number was
  taken on.
- **The fault put back, for the lifespan knob:** arm B reports `oldage` **0**
  at every stop of every seed, and arms A/D/E report 24–1,126. A knob that
  cannot produce a zero cannot be trusted about a non-zero.
- **The fault put back, for the gate knob:** with the gate on, `dig_input`
  reads `n=758 min=0.257 med=0.599 max=1.000` on seed 1; with it off,
  **`n=0`**. The gate's input is not merely quiet in the off arm, it is not
  sampled.
- **The selftest passes.** `latecensus control=selftest` — every footprint
  column moves for a box whose answer is known, and the engine's per-nest
  roofed count reconciles with the library's whole-world one.
- **The arm-consistency check earned its place inside one session.** The
  analysis script refuses to run unless every run's *echoed*
  `life_half_life` and `room_gate` match the arm its filename claims. It went
  red the first time the shipped-bed runs were appended to the digest, because
  the parser's cursor was still pointing at arm E and those beds' `40000`
  overwrote its `80000`. The numbers would have been quietly wrong on one arm
  of one comparison, and nothing else in the pipeline would have said so.

## The order statistics

Twelve seeds, `p10 / median / p90`, at both stops. **Read the spread, not the
median**: this bed is chaotic and the arms overlap heavily everywhere except
where it is said below that they do not.

### At 120,000 frames

| quantity | A shipped | B immortal | C no gate | D half | E double |
|---|---|---|---|---|---|
| ants alive | 10 / **52** / 135 | 24 / 83 / 174 | 31 / 81 / 112 | 4 / 75 / 259 | 32 / 86 / 148 |
| plants standing | 93 / **140** / 178 | 85 / 122 / 268 | 87 / 146 / 229 | 63 / 183 / 246 | 78 / 148 / 215 |
| seed bank | 180 / **343** / 576 | 159 / 270 / 458 | 210 / 423 / 773 | 96 / 507 / 660 | 147 / 403 / 555 |
| starved deaths | 46 / **71** / 105 | 73 / 115 / 192 | 39 / 125 / 226 | 25 / 55 / 167 | 56 / 93 / 120 |
| old-age deaths | 26 / **106** / 136 | 0 / 0 / 0 | 59 / 116 / 193 | 45 / 133 / 399 | 15 / 36 / 49 |

### At 200,000 frames

| quantity | A shipped | B immortal | C no gate | D half | E double |
|---|---|---|---|---|---|
| ants alive | 6 / **117** / 290 | 31 / 108 / 452 | 12 / 84 / 205 | **0** / 34 / 251 | 45 / 138 / 644 |
| plants standing | 49 / **78** / 125 | 34 / 83 / 370 | 26 / 40 / 95 | 11 / 67 / 176 | 26 / 118 / 215 |
| seed bank | 71 / **274** / 340 | 17 / 151 / 383 | 21 / 130 / 193 | 30 / 152 / 702 | 14 / 218 / 348 |
| starved deaths | 88 / **159** / 471 | 160 / 256 / 652 | 64 / 336 / 618 | 33 / 363 / 632 | 123 / 217 / 913 |
| old-age deaths | 68 / **187** / 386 | 0 / 0 / 0 | 124 / 217 / 337 | 49 / 645 / 1028 | 42 / 108 / 155 |

**Colonies with at least one ant standing, of twelve, at every stop:**

| arm | 20k | 60k | 100k | 120k | 140k | 160k | 180k | 200k |
|---|---|---|---|---|---|---|---|---|
| A shipped | 12 | 12 | 12 | **12** | 12 | 12 | 12 | **12** |
| B immortal | 12 | 12 | 12 | 12 | 12 | 12 | 12 | 12 |
| C no gate | 12 | 12 | 12 | 12 | 12 | 12 | 12 | 12 |
| **D half** | 12 | 12 | 12 | 12 | **11** | **10** | **9** | **8** |
| E double | 12 | 12 | 12 | 12 | 12 | 12 | 12 | 12 |

## Question 1 — does `life_half_life: 40000` still hold?

**Yes, and it stays at 40,000 — but for a different reason than the one it was
set for, and the reason it *was* set for does not survive this sweep.**

Paired per seed, which cancels everything the rule is not about. "A higher on
_k_ of 12" is a two-sided exact sign test over the twelve seeds:

| comparison, at 200,000 frames | ants | plants | seed bank | starved | old age |
|---|---|---|---|---|---|
| **40,000 against immortal** | 7/12, p=0.77 | 5/12, p=1.00 | 6/12, p=1.00 | **2/12, p=0.071** | 12/12, p<0.001 |
| **40,000 against 20,000** | 8/12, p=0.39 | 5/12, p=1.00 | 5/12, p=1.00 | 6/12, p=1.00 | 4/12, p=0.63 |
| **40,000 against 80,000** | 5/12, p=1.00 | 6/12, p=1.00 | 8/12, p=0.39 | 4/12, p=0.63 | 10/12, p=0.039 |

Three readings, in order of how much they change:

- **The lifespan's population effect is gone.** The last sweep credited
  40,000 with bounding a runaway 6.6x (3,182 ants → 483) and then withdrew
  that claim when nest scent drift moved the baseline. This sweep does not
  restore it in any form: against an *immortal* colony, twelve paired seeds
  put ants at 7 of 12, plants at 5 of 12 and the bank at 6 of 12 — three coin
  flips. The only channel that moves is hunger: the ageing colony starves less
  on **ten of twelve seeds** (median −26 deaths, p=0.071 at 200,000; −21,
  p=0.071 at 120,000), which is the designed trade — age thins the colony
  before it eats the bed bare. **A lane reading the pre-fix record will expect
  the lifespan to be a population brake. It is not one on this trunk.**
- **20,000 is now measurably worse, and it was not before.** The pre-fix
  sweep halved the constant, found it "flattened no further" and shipped
  40,000 on that. Post-fix, halving is the only setting in this sweep that
  kills colonies: **four of twelve seeds reach zero ants by 200,000 frames and
  a fifth is down to one**, against twelve of twelve alive at every other
  setting including immortal. Its old-age deaths run to a median of 645
  against 187. The floor under 40,000 is now measured rather than assumed.
- **80,000 is indistinguishable from 40,000 where it matters, so there is no
  case to move.** Every paired outcome is a coin flip except old-age deaths,
  which is the mechanism by definition. The one place the two differ is the
  low tail — ants at 200,000 read p10 6 / min 1 at 40,000 against p10 45 /
  min 27 at 80,000 — and **the tail and the paired test disagree, so the
  paired test wins**: five of twelve seeds run the other way, which is what a
  two-seed tail difference looks like at n=12 on a bed this chaotic. Setting a
  constant from an order statistic that no per-seed direction supports is how
  the six-seed 1.64x became a per-seed median of zero.

**So: before 40,000, after 40,000.** The constant is unchanged in
`assets/species/ant.ron:47` and `assets/species/longant.ron:141`, and it is
now the smallest setting at which no seed's colony dies inside a session, with
nothing above it buying anything a paired test can see.

**What it buys, in the world's words, is the death itself.** On the shipped
arm at 120,000 frames, age is **55% of colony deaths at the median** and
outnumbers hunger on **nine of twelve seeds**; at 200,000 it is 50% and six of
twelve. Before the lifespan shipped, every one of those was starvation. That
is `CLAUDE.md`'s first law holding on the creature line: the colony's fall is a
slope with a corpse at each step, not a hunger cliff.

## Question 2 — does the room/dig gate still earn its place?

**Not on the ground #359 gave it. It survives on a different number, and one
of #359's two counter results has reversed.**

#359 put the gate at **5 beds of 12 against 0** — the gate kept a colony alive
where the off arm had none. **That result is gone, because nothing dies either
way now: twelve of twelve alive at 200,000 frames in both arms.** The
difference the gate was justified by was a difference between two colonies
that were both being culled, and the cull is fixed.

What the paired seeds do say, at 200,000 frames:

| quantity | direction | seeds | median delta | p |
|---|---|---|---|---|
| **plants standing** | **gate on higher** | **10 of 12** | **+24** | **0.039** |
| edible cells | gate on higher | 9 of 12 | +182 | 0.146 |
| ants alive | gate on higher | 8 of 12 | +42 | 0.39 |
| seed bank | gate on higher | 7 of 12 | +115 | 0.77 |
| digs | gate on higher | 5 of 12 | −207 | 1.00 |

- **The one thing that clears its own bar is the bed, not the colony.** With
  the gate on the stand is greener on ten of twelve seeds at 200,000 frames
  (median 78 plants against 40 across the arms' order statistics). That is a
  new justification, not #359's, and at 120,000 frames it is not there yet
  (7 of 12, p=0.77) — the gate's effect arrives in the second half of a
  session, which is the length a control has to reach to see it.
- **`#359`'s digging result does not reproduce.** It measured the room arm
  digging **more on 11 of 12 seeds, median 1.90x**, at 300,000 frames on
  `main` `7bfb4e54`. Here, at 200,000 frames on `16bab295`, it is 5 of 12 and
  p=1.00 — no direction at all. Two things differ (the seed cull, and 200,000
  against 300,000 frames) and this sweep cannot separate them, so read this as
  *"the digging cost that argued against the gate is not visible at session
  length on this trunk"*, not as a refutation of the earlier measurement.
- **The owner already picked this arm by eye** (blind A/B, card
  `20260912T134505685Z-b518ab`, *"A is bad. B is good"* with the room arm in
  pane B), and that is the standing order of evidence here. Nothing in this
  sweep argues against it. **The recommendation is that the gate stays and its
  justification is rewritten**: it is no longer "the colony lives", it is "the
  bed stays green".

## Question 3 — is §Z6 still true?

**No, as written, at session length.** §Z6 says *every shipped bed starves its
ant colony inside one play session*, on a table where **two of nine runs** had
any ant alive at the end and *every death in every run was starvation*.

The shipped arm, at 200,000 frames, across all six shipped beds:

| bed | seeds | live colonies at 200,000 | ants alive |
|---|---|---|---|
| `played_bed` | 1–12 | **12 of 12** | 1 – 408, median 117 |
| `played_bed_longant` | 1–3 | 3 of 3 | 368, 891, 1,198 |
| `played_bed_understory` | 1–3 | 2 of 3 | 0, 65, 250 |
| `played_bed_flitter` | 1–3 | 3 of 3 | 101, 118, 140 |
| `played_bed_scrambler` | 1–3 | 3 of 3 | 33, 181, 397 |
| `played_bed_windfall_reach` | 1–3 | 3 of 3 | 2, 69, 203 |
| **all six beds** | | **26 of 27** | |

And starvation is no longer the only channel, nor the largest: on the played
bed at 200,000 frames age is half of colony deaths at the median.

**Three things keep it from being closed outright, and the section should be
rewritten rather than ticked off:**

1. **The bar is written at 300,000 frames and this sweep stops at 200,000** —
   the round's brief caps runs there. Everything above says the colony is
   alive at two thirds of the bar's length; nothing above says it is alive at
   the bar.
2. **The bar names two beds this sweep does not run** — `labstats`'s default
   box (8 herb, 1 colony) and full box (256 herb, 3 colonies). Six *shipped*
   beds is the player's question; those two are the register's own
   reproduction and were not re-taken.
3. **"A colony" is not "an ant".** Three of the twenty-seven runs end in
   single figures (1, 2, 33). On those seeds the bed is arguably still failing
   the spirit of the bar while passing its letter.

**Recommended disposition: §Z6 stays OPEN with its numbers replaced.** The
2026-09-07 table is now known to have been measured through the seed cull and
should not be quoted again; the reproduction to run is the two boxes at
300,000 frames, and the claim to keep is the narrow one — *some shipped beds
end a session with a colony too small to be one.*

## What this round did not do, and why

- **The hazard-interval arithmetic was not re-derived.**
  `plant::old_age_chance_over(age, T, interval)` and
  `the_hazard_is_a_property_of_the_half_life_not_the_interval` are unchanged
  and untouched; the reason is in
  [`lanes/evolution-lab-lifespan.md`](lanes/evolution-lab-lifespan.md) and the
  brief forbids it.
- **No run reached 500,000 frames**, so nothing here speaks to what the
  previous sweep found there (every colony at every setting extinct by
  500,000). That number was taken through the cull and is now unknown too.
- **`labstats` was not re-run.** Its published paired arms (ants 18 → 92,
  plants 34 → 101, bank 85 → 166 at 120,000 frames on seed 1) were measured
  through the cull, on `labstats`'s own bed rather than the played bed, and
  are stale in the same way everything else was. They are a one-seed pair and
  should be re-taken as a sweep or dropped.
- **Nothing was posted to the review queue.** Every question this round asked
  is a count, and the round's own standing rule is that a question needing no
  visual is asked in chat rather than in the queue.
