# Design: judge fetch where it erodes, and add the parents' average to each generation

*Brain lane, 2026-10-21 (evening). Two changes proposed for the next dig round. An independent review (same day, against
the code) found 4 MAJOR and 6 MINOR points, listed at the end. **Outcome: part 1 goes ahead with `calib=96` and
corrected claims; part 2 is deferred** (the trainer version is not the operator that was tested offline).*

## Part 1. The fetch floor reads far food only (`dist=30,30`)

**What was believed.** About 45% of every generation is vetoed by the kept fetch floor (0.8 of s91), and 89% of
vetoed hold-out champions sat within one standard error of the line (handoff log 17:40). The research lane read this
as a gate that vetoes on noise, and proposed a log ramp from x1 at 0.85 to x0.1 at 0.75
(`research/q2-skill-ladder-without-forgetting-2026-10-21.md` section 3).

**What the per-world scores say.** Fetch's 96 test worlds put the food 10, 16, 23 or 30 columns out, a quarter at
each (`Scarce::geometry`: `k/2 % 4`). Share of s91 on the same worlds, and worlds where the brain fetched under half
of s91's mean (`test96-scores/*-fetch.txt`):

| Brain | 10 cols | 16 | 23 | 30 cols |
|---|---|---|---|---|
| s91 (the forager) | 1.04, 0/24 | 1.03, 0/24 | 0.99, 0/24 | 0.95, 0/24 |
| s171 | 0.86, 1/24 | 0.91, 0/24 | 0.85, 0/24 | **0.71, 4/24** |
| copy v2 | 0.87, 1/24 | 0.87, 0/24 | 0.83, 1/24 | 0.75, 2/24 |
| r2 cur192 (best) | 0.99, 0/24 | 0.95, 0/24 | 0.92, 0/24 | 0.81, 1/24 |
| r2 small192 | 0.95, 0/24 | 0.91, 0/24 | 0.84, 1/24 | **0.64, 5/24** |

The loss is almost all at 30 columns. **Traced** (scratch picture and position trace on test worlds 31, 54 and 30;
`Reports/brain-training/investigation/fetch-forays-2026-10-21.png`): after dig training the ant's trips out end
20-27 columns from the nest and it turns home; s91 walks 33 and more. Food 30 columns out (its heap reaches to 26)
is found or missed by a few columns, so each far world is close to pass/fail, and only 12 of a 48-world training
generation are far. That is why the vetoes look like noise: the gate is reading a real, specific loss -- shorter
trips -- through a coarse, lumpy sample of it. The dig-trained ants also sit in the nest for long stretches between
trips; s91 never does.

**Consequence for the ramp.** A ramp lets a brain lose more trip length for the same penalty. It would loosen the
gate on exactly the skill that is eroding. Dropped.

**The change.** Launch the next round with `dist=30,30 calib=96`, so every fetch world puts the food 30 columns out.
Same floor (`forget=0.8`), same reference brain (`keepref=` s91), re-measured at the new distance on the calibration
worlds. No code. `calib=96`, not 24: the reference is fixed once for the whole run, and on 12 far pairs s91's own
s.e. is 0.059, enough to put the floor anywhere in about 0.75-0.85 of s91's true far mean (review, MAJOR 1). Check
the printed `suite reference fetch:` against 20.96 (s91's worse-side mean on the 96 test worlds).

**Measured, 96 test worlds at `dist=30,30`** (`test96-scores/far30/`; reference s91's worse-side mean 20.96):

| Brain | Far-only share of s91 | s.e. over 24 pairs | Worlds failed |
|---|---|---|---|
| s91 | 1.00 | 0.042 | 2/96 |
| s171 | 0.64 | 0.078 | 17/96 |
| copy v2 | 0.70 | 0.069 | 12/96 |
| b181 | 0.74 | 0.063 | 9/96 |
| r2 cur192 | **0.84** | 0.045 | 4/96 |
| r2 small192 | 0.48 | 0.080 | 29/96 |

The brains now spread 0.48-0.84 instead of 0.78-0.90 (mixed distances), but the standard errors grow too (cur192
0.025 -> 0.045, s91 0.015 -> 0.042), so the gate separates brains about 1.8 times better, not 3 (review, MAJOR 2).
**The cost:** the best brain sits closer to the line -- cur192 is 4 s.e. above the mixed floor and 0.9 s.e. above the
far one -- so a child as good as cur192 is vetoed about 20% of the time, where today it almost never is. Its benefit:
a child whose trips shrink is vetoed reliably instead of by luck, and that is the loss that is happening. Roughly,
the mixed floor at 0.8 corresponds to a far-only share of about 0.7 (small192 0.78 -> 0.48, s171 0.78 -> 0.64, copy
v2 0.79 -> 0.70, b181 0.81 -> 0.74), so far-only at 0.8 is also a stricter rule than today's, and the faithful reading
of "keep 80% of where the forager was" on the trip that erodes.

**Risks.**
- The near distances stop being checked. They have not eroded (0.85-0.99 above), and a brain that walks 30 columns
  out passes them on the way; the test-world judging keeps reading all four (`dist=10,30`).
- A start under the new floor (s171, copy v2, possibly tonight's copy arms) spends its first generations vetoed. The
  trainer's pre-flight line prints it; judge the start at `dist=30,30` before launching.
- The test-world fetch numbers in the hand-off stay on `dist=10,30`, so they remain comparable with every earlier
  entry. Far-only is the training gate, not a new headline number.

## Part 2. The parents' average as one extra child (`centroid=1`)

**Deferred after review (MAJOR 3 and 4 below): kept as the record of what was proposed and what it would need.**

**Evidence.** Offline (handoff log 17:40; `scratch-analysis/2026-10-21/averaged-child.sh` on
`scratch/brain-instruments`): 50 children of s171 under each of three mutation settings, the top quarter by one
48-world set kept; the parent plus the mean of the kept children's changes, judged on 96 test worlds, gained +0.42 /
+0.74 / +0.32 full loads against the kept children's own mean +0.03 / +0.22 / +0.22 (each about +-0.5). Pooled over
the three, about +0.3 to +0.4 better than truncation's average pick, roughly 1.7 standard errors: suggestive, not
shown. The field's name for it is intermediate recombination (the (mu/mu, lambda) evolution strategy); its argument
is that averaging the survivors cancels the parts of each mutation that selection did not ask for.

**What.** A setting `centroid=1` (default off for now; see *Default*). In `breed`, after the parents and before the
random children, push one genome: for every slot, the mean of that slot over the parents (the generation's elites,
25 of 100). It is not mutated. It takes the place of one random child, so a generation's cost is unchanged.

- **Pruned links.** A plain mean keeps every link any parent has, so the average has the union of the parents'
  links and pays the wire cost for them (`wired`: at about 620 links, 80 more cost about 4% of fit, more than one
  generation's typical gain). Two candidates: (a) the plain mean, as tested offline; (b) the mean over parents where
  the slot is live, kept only if more than half of the parents have it live, else 0. The offline test was (a). I
  propose (b) for the trainer, because (a) would mostly lose to the wire cost before its digging is read, and say so
  as an untested difference.
- **Protected slots** (`protect=`) are identical in every parent, so their mean is their value; `noprune=` slots are
  live in every parent, so (b) keeps them.
- **Resume.** ` centroid=1` goes into the arena string only when set, so older states resume unchanged. Off, `breed`
  draws exactly the same random numbers as today (the centroid uses none).
- **A counter, per the house rule.** Each generation prints whether the centroid survived the race and its rank, and
  the run's summary counts how often it became a parent and how often it was the champion. If it is never a parent,
  it is doing nothing, whatever the final score says.

**Guards.**
1. With `centroid=1`, child number `elites` of `breed` equals the slot-wise rule (b) over the parents, exactly, on a
   hand-built set of parents with a slot live in 2 of 3 (kept) and in 1 of 3 (dropped). Positive control: the same
   test against rule (a) fails.
2. With `centroid=0`, `breed` is bit-identical to today on a fixed seed (fingerprint recorded before the change).
3. End to end on a ruler arena: the run prints the centroid's rank each generation and its count in the summary.

**Default.** Off until one paired round measures it, then on if it does no harm (the project's default-on rule).

## How the next round would test part 1

Four VMs, `--hours 8`, about $7 at the measured spot rate (the owner's go and cap first). From the better of
tonight's winner and r2 cur192, judged on test worlds at both `dist=10,30` and `dist=30,30`. Two seeds at
`dist=30,30 calib=96` against the same two seeds at today's `dist=10,30`, paired by seed. Judged on the 96 test worlds:
full loads, far-only fetch, mixed fetch. The question is whether the far floor holds the trips at 30 columns without
costing digging. The runs' own `holdout.csv` and `test.csv` read the floor they trained under, so they do not compare
across arms; only the test-world judging does (review, MINOR). Two seeds can only show a large effect; the round
trains the brain forward either way.

## Review and what changed (2026-10-21)

An independent agent reviewed this against `src/train/mod.rs`, `src/train/challenges.rs`, `examples/train.rs` and
`src/sim/creature/mind.rs`.

- **Part 1 mechanics confirmed.** `dist=` reaches only the `Scarce` arms (the suite's kept fetch); the kept fetch arena
  runs at its own 6,000 frames, as the far30 test files do; calibration re-measures `keepref=` s91 at the new
  distance; `geometry` with lo == hi gives 30 on every world with sides from `k` parity; `dist` is in the arena string,
  so a `dist=10,30` state refuses to resume at `30,30`.
- **MAJOR 1, the reference on 12 pairs:** taken -- `calib=96`.
- **MAJOR 2, "three times better" overstated:** taken -- corrected to about 1.8 times, with the cost to the best
  brains stated.
- **MAJOR 3, part 2's evidence tests a different operator:** offline it was one parent plus the mean of its selected
  children's changes; the trainer would average 25 persisting, cross-bred elites, and a mean over an ancestor and its
  descendants pulls back toward the ancestor. The faithful variant (champion plus the mean of its surviving
  children's changes) needs per-child parent indices saved in `RunState`. **Part 2 deferred** on this: the evidence was
  1.7 s.e. for a different operator, and part 1 needs no code.
- **MAJOR 4, rule (b) undefined for setting slots** (switches by sign, sharpness, pull, memory rates, below
  `FIRST_LINK`): recorded for when part 2 is picked up -- average only slots at or above `FIRST_LINK` with `W_EPS` as
  the live test; medians or clamped means for settings; sum in f64; centroid counters saved in `RunState`.
- **MINOR**, all recorded for part 2: f32 summing, a `noprune=` slot's mean can cancel under `W_EPS`, racing can drop the
  centroid on round 1 (soil only), and the wire cost of 80 more links is about 7.7% of the ranked score, not 4%. One
  for part 1, taken above: the run's own hold-out and probe read the far floor, so compare arms on test worlds only.
