# A home sense that stays sharp far out, and the come-home lesson it makes copyable (2026-10-22)

For the Teacher lane. Follows [starve-trace-2026-10-22.md](starve-trace-2026-10-22.md), where the come-home rule
worked when the teacher drove the ant but three copies (hm1-hm3) leaked into foraging, and the maze thread's
[trail-following report](../trail-following/trail-following-2026-10-22.md), where trail rule 2 had the same symptom.
Both named the same suspect: the brain only feels home through `squash(dist / 32)`, which barely separates "far"
from "in the food band".

**Result.** With a sense that reads exactly 0 near home, come-home copies onto hA1 at the teacher's own level, and
food and digging do not move. Stacked with the maze thread's "lay B only while carrying" edit, starvation falls from
4.31 to 1.22 per 24 ants. Trail rule 2 still does not copy.

## The switches (scratch, env-gated, on the compute branch since 75593154)

- **`PP_HOMEVEC=R:W`** (used: `40:30`). `far = clamp((d - R) / W, 0, 1)`, d = distance to the body's home anchor.
  - Context slot 9 (d_alarm, 0 on every bed decision) reads `far`.
  - Every option's feature 40 (`f::LAST_FOOD`, never written until now) reads `cos(step, home) * far`.
  - Both are exactly 0 within R of home. Food sits 15-35 columns out, so nothing in the food band sees either.
- **`PP_HOMEDIR=1`**: every option's feature 18 (`f::BUILDING`, unwritten) reads `cos(step, home)` at every
  distance. Built for trail rule 2; it did not help (below).
- A genome's existing links on these slots must be zeroed first (`scripts/zero.py`): hA1 has 16 on slot 9 and 8 on
  feature 40 (and 11 on feature 18).
- **Off:** inrun hA1 seed 1 is byte-identical to compute 529ff42a in every output except `events.csv`. That file
  gains the shadow-rim patch's `fdrop` rows and is identical with them filtered out. The Teacher lane re-checked this
  after merging.
- The same commit ports the come-home teacher (`PP_CARRY`, `PP_HOMEFAR`) and the maze thread's trail teacher
  (`PP_TRAIL_LABEL` / `PP_TRAIL_OUT` / `PP_TRAIL_ACT`) onto the compute branch. Patch:
  [`homevec-vs-529ff42a.patch`](far-home-sense-2026-10-22/homevec-vs-529ff42a.patch).

## How the copy was made

- **Labels.** hA1v (hA1 with those links zeroed) ran in 8 bed colonies (seeds 41-48) with the sense on. The
  come-home rule (`PP_HOMEFAR=45`) only labelled; it never acted.
- **Data.** Only far or labelled decisions were kept, about 18k per colony (`filt.py`, then `sub.py` at 0.15).
- **Training.** `copier_c.py` gained `only_allow`, which trains the allow-list and nothing else. The allow-list is
  the sense's 48 weights: feature 40 into all 16 scorer units, plus slot 9 into the 16 units, 8 job rows and 8
  hidden units.
- **Why that cannot leak.** Both inputs are 0 inside 40 columns, so every choice made there is hA1v's to the bit.
  Fit to the teacher's steps went from 0.41 to 0.86 (`scripts/hc1.out`).

## Results (bed with come-home settings, selection off, 200k frames, seeds 1-32, 24 ants)

Starved per colony, how many of those died past 60 columns, colonies with 12+ dead, deliveries home, new cuts,
share of ant-time past 60 columns. Paired = colonies with fewer / more deaths than hA1. Raw:
[`results-32-seeds.txt`](far-home-sense-2026-10-22/results-32-seeds.txt).

| arm | starved | past 60 | collapsed | home | cuts | time past 60 | paired vs hA1 |
|---|---|---|---|---|---|---|---|
| hA1 | 4.31 | 2.69 | 2 | 269 | 320 | 15.9% | |
| hA1v + sense (control) | 3.66 | 2.62 | 0 | 253 | 307 | 13.6% | 15 / 13 |
| teacher driving hA1 (ceiling) | 2.28 | 0.75 | 1 | 283 | 271 | 2.3% | 16 / 7 |
| **hc1** (copy) | **2.50** | **0.75** | 0 | 268 | 315 | 3.4% | 19 / 8 |
| hA1-sup (maze thread: lay B only while carrying) | 2.88 | 2.03 | 0 | 283 | 341 | 16.7% | 14 / 11 |
| hA1-sup + rule 2 acting | 2.47 | 2.06 | 0 | 391 | 342 | 20.4% | 19 / 7 |
| **hcs** = hc1 + sup's 10 emit weights | **1.22** | **0.44** | 0 | **305** | **346** | 4.9% | **24 / 3** |
| hcs + rule 2 acting (ceiling) | 0.69 | 0.38 | 0 | 383 | 345 | 7.1% | 23 / 1 |
| hcs, feature 18 zeroed + `PP_HOMEDIR` (control) | 1.88 | 0.50 | 0 | 309 | 328 | 3.8% | 21 / 7 |
| tb: rule 2 copied onto that | 3.12 | 0.91 | **3** | **218** | 353 | 6.3% | 18 / 8 |

The hA1 and sup rows reproduce the starve-trace and maze threads' own numbers exactly, so this tree is the bed they
measured.

- **The copy matches the ceiling on roamers.** Deaths past 60 columns go 2.69 -> 0.75, the same as the teacher
  driving. Unlike the teacher, hc1 keeps digging (315 cuts vs 271 for the teacher, 320 for hA1).
- **hc1 and sup stack.** hcs beats hc1 on 16 seeds and loses on 10. Against hA1 it is better on 24 seeds and worse
  on 3. Food home goes up 13% and no colony collapses.
- **Trail rule 2 still does not copy.**
  - Acting on hcs it is the best arm here, at 0.69 starved and 383 home, so the lesson is worth having.
  - The direction-only sense (`PP_HOMEDIR`, feature 18 trained alone, `ta`) fits nothing: 0.409 -> 0.410.
  - The whole-brain copy (`tb`, `copier_t.py` `target_only`, anchor weight 12) fits 0.41 -> 0.63, the maze
    thread's 55-64% again. In the bed it is worse than its control: food home 309 -> 218, starved 1.88 -> 3.12, and
    3 colonies collapse.
  - So rule 2's blocker is not the home sense. Rule 2 reads "on the trail" (B there at least half the best) as a
    *relative* comparison across options, which a per-option linear scorer cannot express. A per-option feature
    "B there / best B in reach" is the likely next sense, by the same recipe: build the input the rule reads, then
    train only its weights.
  - `PP_HOMEDIR` itself costs a little (hcs 1.22 -> 1.88 with feature 18 zeroed, 8 better / 13 worse seeds). Don't
    use it.

## What to use

- **hcs + `PP_HOMEVEC=40:30`** as the founder candidate, handed to the Teacher lane (genome
  [`genomes/hcs.raw.txt`](far-home-sense-2026-10-22/genomes/hcs.raw.txt)). It must run with the switch on, since
  without it the 48 new links read 0 and it is hA1-sup with 24 links cut.
- Under selection, mutants mutate the 48 new links like any others. Judging whether selection keeps the lesson is
  the Teacher lane's colony batch.
- `tb-not-recommended` is kept as the record of the failed rule-2 copy.

## Files

- `genomes/`: hA1v (links zeroed), hc1, hcs, tb-not-recommended.
- `scripts/`: `run.sh` / `batch.sh` (bed runs), `lab.sh` (label logs), `filt.py` / `sub.py` (log reduction),
  `zero.py` / `layout.py` (slot surgery), `cmp.py` (the table), `copier_c.py` / `copier_t.py` (with `only_allow`),
  plus configs and outputs for hc1, ta and tb. Paths in them point into this session's scratch (`/home/user/hvs`).
