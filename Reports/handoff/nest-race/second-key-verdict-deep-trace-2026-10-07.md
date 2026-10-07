# Second key on the nest stack going to main (Deep trace, 2026-10-07 ~22:55 UTC, seed 2 added 23:45)

For Nest race's PR of the whole stack, and for the coordinator's brief to the merge desk. Scott's decision at 22:34, as
the coordinator recorded it: put the stack on main, "defaults on only where they clear the bar; Deep trace written yes".
Measured unless marked *inferred*.

## The answer, switch by switch

| switch | my answer | on what |
|---|---|---|
| `WAY_FOOT` | **Yes, on by default.** Clearly better. | 12 seeds at heap 90, plus my exact reruns of s3 and s4 |
| `NEST_STORE` part `edible` | **Yes to the fix. On by default after its 12-seed set,** checking deaths in the mound tunnels per seed. Seed 2's 44 are traced (below): ants walled into the mound, not the store. | 4 seeds at heap 90, plus my seed 2 rerun |
| the rest of the stack: `NEEDS_FIRST`+backfill, `CARRY_HOME`, `DOOR_COLUMN`, `LAY_BAR`, `NEST_STORE` | **Not reviewed by me against main.** | Every trace of mine had them on in both arms |

## `WAY_FOOT`: yes

From Nest race's 12-seed bar (`store-arms/way-foot/twelve/results.md`), heap 90, 300k, the rest of the stack on in both
arms, `WAY_FOOT` off against on:

- **Starved:** lower on 12 of 12 seeds, 8,273 against 1,750.
- **Ants that starved after a hungry spell begun deep in the nest:** lower on 12 of 12, 5,165 against 957.
- **Falls:** lower on 11 of 12.
- **Colony at 300k:** bigger on 9 of 12. The three smaller (s6, s8, s10) are not die-offs, and the lowest count after
  50k was 370 off and 444 on.
- **Second entrances:** none in 24 runs, with a positive control that finds them in older runs.

What I checked myself: seeds 3 and 4 rerun exactly (ants 647/748 and 722/736 off/on, starved 1,040/148 and 434/269).
The other ten seeds are Nest race's table.

**The deaths left with `WAY_FOOT` on are the storeroom's, not the routes'.** On s3 and s4 they were traced to the store
counting crumbs too small to bite (`store-arms/way-foot/residual-nest-deaths-deep-trace-2026-10-07.md`). Falls were
1.3-1.8% of those starvers' decisions. The residual starvers on s5, s8 and s11 (364, 276, 297) are not traced; `edible`'s
12-seed set will show whether they are the same cause.

## `edible`: yes to the fix, default on after its 12-seed set

**It fixes the traced cause.** The store's count, its pull and its "can feed" now use the mouth's own test, with the
ants' own gut. Checked in the code at b5922852:

- `store_food` asks `diet_yield(cell, nest_sites[site].gut) > EAT_YIELD_THRESHOLD`.
- The site's gut is the founder's expressed `TRAIT_GUT_BIAS` (-0.8 for the lab's evolved ant), not the species file's 0.
- Its guard has a 30 J crumb that must count at -0.8 and not at 0, so the wrong gut would fail it.

**First 4 seeds** (`store-arms/edible/results-2026-10-07.md`, base against `edible`):

- **Deep starvers:** 10/3/77/179 -> 0/2/1/1.
- **Starved:** 24/9/148/269 -> 7/47/2/15.
- **Colony at 300k:** 796/771/748/736 -> 729/782/778/727. No die-off; the lowest after 50k was 447-565.
- **Falls:** slightly up, 10.4-12.4% -> 11.7-13.4%.

**What holds back default-on:**

1. **Seed 2 rose from 9 starved to 47,** 44 of them in the mound tunnels.
   - Traced in the section below: ants walled into small pockets of the mound, not the release I warned of.
   - Whether `edible` makes that more likely is not shown by one seed.
2. **Only 4 seeds.** Scott's decision asks for 12.

**Left over, a separate part and Nest race's call:** the store's meal cycle keeps making crumbs no mouth takes, ~190 per
run (`store-edible-review-deep-trace-2026-10-07.md` §3). Under `edible` they no longer hold anyone in, but nothing
removes them, so they pile up in the store's lobe. That is not a blocker.

## The rest of the stack: not mine to vouch for

Every run I traced had the rest of the stack on in both arms, so none of my numbers compares it with main. If the PR
turns any of those switches on by default, the bar is a 12-seed table against main: heap 90, and heap 30 too, per
Scott's note that the heap distance is a confound. I have not seen that table.

## Seed 2's mound-tunnel deaths under `edible` (traced 23:45 UTC)

**They are ants walled into small pockets inside the mound, not the store letting the hungry go.**

Rerun of s2 with and without `edible`, b5922852 + probe v5 (store census only, measuring only). The runs match Nest race's:
782 and 771 ants at 300k. 46 and 8 starved by the ledger, against Nest race's 47 and 9: one fewer on both arms, so a
counting edge, *inferred*. Of `edible`'s 44 deaths in the mound tunnels, 40 were at 200-300k; the base arm had 1 there.
Tools: `store-arms/way-foot/tools/zonedeaths.py`, `moundfood.py`.

**Not the release:**

- None of the 40 began its last hungry spell in the nest, and none entered it during the spell.
- The store held 160-212 cells, always 8 or more, through every one of their spells.

**Who and how:**

- **Foragers:** none was a nest worker.
- **Fed at the start:** each began its spell at about the grant (energy 0.90-1.00), already inside the mound.
- **No bites:** none bit anything from then until it starved, a median 3,550 frames later.
- **No pull fired:** their decisions were 65% plain steps and 35% idle.

**They died in three groups.** Each group was shut into a small pocket of soil (nest maps every 1,000 frames, flood-filled
from each ant's own cell):

| when | ants | the pocket |
|---|---|---|
| 210k and 248k | 2 + 3 | 2 cells, sealed for at least 4-6k frames before death |
| 251.4-251.6k | 15 | open 4k frames before death; a sealed 26-cell pocket from about 2k before |
| 260.5-262.1k | 20 | open only through diagonal cracks. Counting side-by-side steps only, 18 of the 20 were in 16-17-cell pockets 1k frames before death |

Whether an ant can squeeze through a diagonal crack is not checked. If it can, the third group was not sealed in.

**Food round them:**

- No food a mouth can take within 10 cells of any of the 40; the nearest was 12-51 cells away.
- Crumbs too small to bite (median 6 J, at most 14 J) were within 10 cells of all 40, and within 3 of 23 of them.

**The mound's dig-out rule** (`MOUND_OUT=dig`) let 1,257 digs at 248-252k and 146 at 260-264k (cumulative counter).
*Inferred:* the walled-in ants tried to dig out and did not get out in time.

**Is it `edible`'s doing?** Not shown by one seed.

- One measured difference: late in the run, the mound pocket held more crumbs too small to bite under `edible`.
  - The box is x 226-248, y 140-159; the counts are cell-samples per 5k.
  - From 255k: 226-318 under `edible` against 94-157 on base.
  - Before that: 150-238 against 114-189.
- How they got there is not traced. *Inferred:* crumbs that no longer count as store food may be dug out of the store as
  soil. Probe v5's `foodmoves=1` log can check that.
- Only seed 2 of the 4 shows these deaths.

**What this changes in my answer.** The release worry is not borne out on seed 2. Seed 2's deaths are ants shut in the
mound, a known failure. Default-on for `edible` now waits only on its 12-seed set. In that set, count the starved in
`mound_in` (ledger `zone_end`) per seed on both arms. If they rise on several seeds, trace them before default-on.

## For the handoff: where my work is

- `store-arms/way-foot/residual-nest-deaths-deep-trace-2026-10-07.md`: why ants still starved in the nest with the new
  routes on (s3, s4).
- `store-edible-review-deep-trace-2026-10-07.md`: the review of `edible`, and where the too-small crumbs come from.
- This file.
- `store-arms/way-foot/tools/`: the measuring patches (probe v4 and v5, measuring only) and the analysis scripts named in
  those write-ups.
