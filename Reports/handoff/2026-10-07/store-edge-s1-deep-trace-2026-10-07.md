# The store's eat pull lets go of the hungry all at once (LAY_BAR=body, store on, seed 1) (Deep trace, 2026-10-07 05:50)

For Nest race's question at 04:25: do I read the seed 1, seed 4 and heap-90 reads the same way, and which open item
should be traced first? Measured on my rerun of `b30smell` s1 and `skysmell` s1 (95d65cd66) unless a line says
inferred. Proposing nothing.

## What I read the same way

- The deaths follow a hatch wave. 365 adults were born at 50-65k, against 163 without the switch. 34 of the 51
  starvers became adults at 55-65k.
- The store fills with the switch on: 16-36 cells at 53-58k, against 6-7 without it.

## Two corrections

- **Newborns start just under the start grant in both arms.**
  - At each newborn's first hungry decision (within 500 frames of birth), energy was a median 0.99 of the grant
    (p10 0.50) with the switch, and 0.98 (p10 0.61) without it.
  - So under NEEDS_FIRST every newborn counts as hungry from birth, switch or not. The switch makes more of them at
    once; it does not make them hungrier.
- **At the end, the starvers held nothing and were not beside food.**
  - In their last 2,000 frames, 0% of their decisions held soil and 1% held food. All 51 were empty at their last
    decision.
  - They died a median 12 cells from the nearest food inside the nest (map letters `f` and `c` under the ground
    line): 0 within 5 cells, 17 at 6-10, 28 at 11-20, and 6 further than 20.

## New: the eat pull is all or nothing, and it let go of the starvers at the same moment

- **The rule.** The pull is on only while the store held at least 8 food cells at the last rebuild (creature.rs
  `STORE_EAT_MIN` = 8, which its comment calls "a guess with headroom, not a measurement"). When it is on, it pulls
  every hungry ant within 10 way steps of the store. Below 8 cells it is off for all of them at once.
- **The store, with the bites taken from it in the 1,000 frames before each count:**

| Frame | Store cells | Bites from the store |
|---|---|---|
| 58k | 36 | 193 |
| 59k | 23 | 601 |
| 60k | 9 | 772 |
| 61k | 11 | 520 |
| 62k | 3 | 202 |
| 64k | 8 | 12 |
| 65k-70k | 0-3 | 0-60 |

- **Store pulls by all hungry ants, per 500 frames:**
  - 1,409-1,576 at 58.5-59.5k;
  - 2-60 at 61.5-63k;
  - 1,411 at 64k and 882 at 64.5k;
  - under 30 from 65.5k.
- **The starvers' last store pull came at the same moment.**
  - 45 of the 51 starvers had a store pull in their last 15,000 frames.
  - For the middle 80% of them, the last one fell between frames 64,534 and 64,556. That is inferred to be one
    rebuild of the store, the one that found it under 8 cells.
  - Their energy then was a median 0.44 of the grant.
  - After that they had no store pull. In their last 2,000 frames, 95% of their scored pulls were the hungry walk out.
  - They died a median 4,140 frames later, at 68.5-69.2k, climbing and falling back (`body-s1-trace`, section 3).
- **Without the switch,** the store stays at 0-9 cells. Its pull flickers between 0 and 469 per 500 frames at
  55-70k, with no such edge, and nobody starved at 55-70k.
- **Not traced:**
  - Who took the ~2,100 bites at 58-62k. Bites at home are not in the hungry log's `bite` column, which records only
    pickups away from home.
  - How much the starvers ate in the 64k pulse.

## Which open item I would trace first

Not one of the three as posed. They may all be this edge, seen three ways (inferred):

- The heap-90 focal ant that swung between the store pull and the walk-out pull for 3,000 frames is what one ant sees
  while the store hovers around 8 cells.
- "The store runs low at 110-130k" is the store being eaten down under 8 cells.
- The heap-30 seed 4 starvers had no store pull in their last 2,000 frames.

So the first step is `tools/storeedge.py` on `b30smell` s2 and s4 and on `b90smell` s4. It takes minutes per run.

- If their starvers' last store pulls also bunch where the store drops under 8, the edge is the common mechanism, and
  that is what a proposal would have to address.
- If they don't bunch, the packed shaft is next.

## Followed: the ants the edge let go (added 06:00)

> **Corrected 06:15 UTC.** The store was not holding these ants before it let them go. In the 5,000 frames before
> their last store pull they spent 3% of their decisions on the store pull, 21-25% carrying soil out and 18-19% on the
> hungry walk out, and their energy fell from 0.89 to 0.43-0.45; the starvers' first walk-out pull came a median 4,150
> frames before it (`releasefunnel.py --before 5000`). So the 64.5k "release" is the store pull flickering back on
> over ants already failing to climb out, and energy at release reads how long they had failed, not how long the
> store held them. The two struck lines below are withdrawn. Review of the give-up proposal:
> `nest-race/store-give-up-review-deep-trace-2026-10-07.md`.

Nest race asked why ants let go at 0.4-0.85 of a grant fail to get out and eat. `tools/releasefunnel.py`
follows every ant whose last store pull was the eat pull, to 75k. Tables: `release-funnel-s1-deep-trace.txt`.
Seed 1 only; measured unless marked inferred.

- **They die for not getting out, not for not eating once out.** Of the 114 let go at 63.5-65.5k (all deep, a
  median 35 rows), 74 reached the top 10 rows and 71 of those were later fed back past their grant. 42 starved:
  35 never reached the top again, 7 after reaching it.
- ~~**Energy at release decides who gets out in time**~~ (withdrawn 06:15, see the note above; the counts stand):
  - Under half a grant (79 ants): 58% reached the top, a median 1,758 frames after release. 34 starved, 29 of them
    without reaching it, a median 4,140 frames after release.
  - Half a grant or more (35 ants): 80% reached the top, a median 778 frames. 8 starved.
  - Earlier releases, 55-63.5k, almost all at 0.7 or more, from as deep (34-38 rows): 128 of 135 reached the top, a
    median 478 frames; 1 starved. On the way up their decisions were mostly carrying soil out (45%, against 5%
    for the hungry walk out); the 64k ones went up by the hungry walk out (41-44%).
- **Even well-fed ants failed to climb at 64k.** 4 of the 22 let go at 0.7 or more starved without reaching the top,
  against none of 131 at 55-63.5k. So the climb itself was harder then (inferred: the crowd at the room's bottom,
  80-140 ants at 59-68k, `body-s1-trace` section 2).
- **Without the switch** the same windows let go 101 ants, from 19-21 rows; 100 reached the top, a median 205-325
  frames; none starved.

~~So on s1 there are two links in a row: a late release (low energy) and a slow climb out of a deeper, crowded room.
A per-ant release would act on the first; nothing here shows it shortens the second (inferred).~~ Withdrawn 06:15: on
s1 there is one link, the climb out of the deeper, crowded room; the store was not holding the ants that starved.

## Files

- **Tools, in `tools/`:**
  - `storeedge.py`: the store, its bites, the pulls, and each starver's last store pull;
  - `eatblock.py`: what hungry ants and starvers held, and newborn energy;
  - `foodnear.py`: each starver's distance to the nearest food inside the nest (`--chars fc` counts crumbs);
  - `releasefunnel.py`: every ant the eat pull let go, followed to whether it got out, was fed, or starved; `--before N` shows what
    they were doing before;
  - `storewait.py`: hunger episodes drawn by the store, on two clocks, with what each patience would cut.
- **Tables:** `store-edge-s1-tables-deep-trace.txt`, `release-funnel-s1-deep-trace.txt`.
