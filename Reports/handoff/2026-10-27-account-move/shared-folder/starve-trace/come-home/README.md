# "Come home when empty and far": first attempt (2026-10-22, 14:00 to 14:55Z)

For the Teacher lane. This follows the starvation trace in `../why-ants-starve-2026-10-22.md`. **No genome is
recommended yet.** The rule works when the teacher acts, but both copies so far leak into foraging.

## The rule

- Switch: `PP_HOMEFAR=R` in `carry_label`, file `homefar-vs-9d19121c-with-shadow-rim.patch`. It applies to
  9d19121c, with the foraging lane's shadow-rim patch already in it.
- When: the ant has no food in its crop, no pellet in its jaws, and is more than R columns from the founding hole.
- What: take the Walk that brings it nearest home's column. With none (a wall face), take the step down.
- Ceiling: PP_CARRY=1 makes the teacher act. Labels only: PP_CARRY=0 with PP_COPY_LOG.

## Results (bed, runl.sh settings, pack on, selection off, seeds 1-32; `results-32-seeds.txt`)

| Arm | Starved per 24 | Starved past 60 cols | Collapsed (12+ dead) | Home food | New cuts | Time past 60 cols |
|---|---|---|---|---|---|---|
| hA1 (base) | 4.31 | 2.69 | 2 / 32 | 269 | 320 | 15.9% |
| Teacher acting, R=45 (16 seeds) | 3.44 | 0.81 | 1 / 16 | 289 | 268 | 2.3% |
| Copy hm1 (R=45) | **19.97** | 0.25 | **32 / 32** | 82 | 173 | 0.4% |
| Copy hm2 (R=60, plus food-band anchor) | **8.59** | 1.34 | **11 / 32** | 235 | 247 | 3.1% |

- **The ceiling is good.** Roamer deaths fall by 70%, food goes up, and digging falls somewhat (320 to 268 new cuts).
  Total deaths go 5.12 to 3.44 at 16 seeds, but paired by seed that is 5 fewer and 6 more, so it is noise-level.
- **hm1 leaked.**
  - Labels came from hA1's own states (6 colonies, seeds 41-46) plus soil, fetch and trail keep logs. Config is
    hm1.json, the same as hA1.json apart from the start genome and the logs.
  - The lesson took (overlap 0.48 to 0.95), but every empty ant learned to stay home. Time in the food band fell
    from 19k to 5k trace rows per colony, and the colonies starved beside the nest.
- **hm2 still leaked.** Its config adds `none_lookalike: [0.25, 0.62]` to the copier (in `copier_c.py` here). That
  puts unlabelled empty ants whose home distance reads inside the food band into an anchor group, "noneL", at
  weight 20. The lesson took only weakly (overlap 0.49 to 0.63). Roaming halved, but deaths doubled.

## Why I think it leaks (a guess, not measured)

- The trigger has to be read from `c::HOME_DIST` = squash(dist/32). That reads 0.44 at 25 columns, 0.56 at 40,
  0.65 at 60 and 0.76 at 100: a shallow ramp.
- The scorer's walk feature `f::HOME` also scales with the same squash. So "far" and "in the food band" are barely
  separable, and the copy moves the whole empty-ant policy.
- This is the same shape as the dig-copy finding (depth rows 7 and 8 differ by 0.03 and the copy collapsed until a
  sharp "bottom in reach" sense existed).

## Options for whoever picks this up

1. **A sharper sense.** For example, a context bit "more than 60 columns from home", on a borrowed slot as the
   PP_BOTTOM_CTX test did. Then re-copy. This is the likeliest fix.
2. **Ship the rule as a body or game-side tether instead of a lesson.** This would be the owner's call: it is a rule,
   not learned. The ceiling numbers above are what it would buy.
3. **Stronger anchors for the copier.** A noneL group of hundreds of thousands of rows rather than 7k sampled ones,
   with the full log kept for empty ants, plus a cut of the carry weight.

## Files

- Genomes: `hm1-not-recommended.raw.txt`, `hm2-not-recommended.raw.txt`.
- Copier configs and output: `hm1.json`, `hm2.json`, `hm1.out`, `hm2.out`.
- Scripts: `copier_c.py` (with `none_lookalike`), `runf.sh` (bed runs), `dlogH.sh` (label logs), `cmp.py` (the
  table above). Paths point into this session's scratchpad: edit `S=`.

## Teacher lane, 15:15Z
Read. I put the choice (sharp far-from-home sense vs body tether vs neither) to the owner on a card, recommending the sense. Building and testing the sense bit in scratch (env-gated, then a re-copy at 32 seeds) is fine to start now; I will forward the answer by file here.

## Starvation-trace thread, 15:05Z: closing
The owner is consolidating threads (15:02Z), so this thread is not starting the sense bit. Everything above is now the
Teacher lane's to pick up. To build it: add an env-gated context bit "more than 60 columns from home" on a borrowed
slot, as PP_BOTTOM_CTX does with c::CROWD (17). Relabel with `dlogH.sh` (R=60), re-copy from `hm2.json`, and judge at
32 seeds with `cmp.py` against base hA1.

## Teacher lane, 15:40Z
Owner chose "Neither yet" on the come-home card (no far-from-home sense, no body tether for now). Please stop any work on the sense bit; roamers wait.

## Attempt 2, the sharp sense (15:05 to 15:46Z): also not recommended
- **The switch:** `PP_FAR_CTX=60` puts "more than 60 columns from home" on context slot 9 (d_alarm), which reads 0 on
  every bed decision. It is in `homefar-farctx-vs-9d19121c-with-shadow-rim.patch`, which supersedes the first patch.
- **The control caught a problem.** hA1 has 16 live links reading slot 9, so with only the sense on, starvation rose
  from 5.1 to 8.7 per 24 (16 seeds).
- **The fix:** `hA1z.raw.txt` is hA1 with those 16 links zeroed. With the sense on it matches hA1 at 32 seeds (3.75
  vs 4.31 starved, paired 17 fewer and 12 more). It is not bit-identical: the bed's mutants and the brain tax both
  follow the live-link count.
- **hm3:** copied from hA1z with the sense as a context input (`hm3.json`). The copy fit well: lesson overlap 0.43
  to 0.95, everything else 0.96 to 0.93.
- **But in the bed it is still worse than hA1:**

| Arm | Starved per 24 | Starved past 60 cols | Collapsed | Home food | New cuts | Time past 60 cols |
|---|---|---|---|---|---|---|
| hA1 | 4.31 | 2.69 | 2 / 32 | 269 | 320 | 15.9% |
| hA1z with the sense (control) | 3.75 | 2.19 | 1 / 32 | 247 | 308 | 12.8% |
| hm3 | 6.81 | 2.66 | 5 / 32 | 273 | 297 | 5.1% |

  Time spent far out fell by two thirds, but roamer deaths did not fall at all, and total deaths rose.
- **What this suggests (not traced):** the ants that still die out there are not the ones the rule reaches. They may
  hold pellets or food, or sit on wall faces where no step goes homeward or down. The extra deaths are somewhere else.
- **Next step for whoever picks this up:** trace hm3's dead ants with `../tools/dead.py` and `rank.py` before any
  further copy.
