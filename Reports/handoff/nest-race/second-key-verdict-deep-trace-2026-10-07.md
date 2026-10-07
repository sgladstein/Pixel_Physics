# Second key on the nest stack going to main (Deep trace, 2026-10-07 ~22:55 UTC)

For Nest race's PR of the whole stack, and for the coordinator's brief to the merge desk. Scott's decision at 22:34, as
the coordinator recorded it: put the stack on main, "defaults on only where they clear the bar; Deep trace written yes".
Measured unless marked *inferred*.

## The answer, switch by switch

| switch | my answer | on what |
|---|---|---|
| `WAY_FOOT` | **Yes, on by default.** Clearly better. | 12 seeds at heap 90, plus my exact reruns of s3 and s4 |
| `NEST_STORE` part `edible` | **Yes to the fix. Off by default until two checks pass:** seed 2's 44 deaths in the mound tunnels are traced, and a 12-seed set. | 4 seeds at heap 90 |
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

## `edible`: yes to the fix, default on after two checks

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

1. **Seed 2 rose from 9 starved to 47,** and 44 of them died in the mound tunnels at 200-300k. That is not
   colony-killing (782 ants at 300k), but it is an untraced rise.
   - It may be the release I warned of: a store that now reads empty lets its hungry go all at once, and they die on
     the way out. That is not checked.
   - I am rerunning s2 with and without `edible`, with the store census, to trace it. The result goes in the section
     below.
2. **Only 4 seeds.** Scott's decision asks for 12.

**Left over, a separate part and Nest race's call:** the store's meal cycle keeps making crumbs no mouth takes, ~190 per
run (`store-edible-review-deep-trace-2026-10-07.md` §3). Under `edible` they no longer hold anyone in, but nothing
removes them, so they pile up in the store's lobe. That is not a blocker.

## The rest of the stack: not mine to vouch for

Every run I traced had the rest of the stack on in both arms, so none of my numbers compares it with main. If the PR
turns any of those switches on by default, the bar is a 12-seed table against main: heap 90, and heap 30 too, per
Scott's note that the heap distance is a confound. I have not seen that table.

## Seed 2's mound-tunnel deaths under `edible`

Pending: the rerun started at 22:50 UTC.

## For the handoff: where my work is

- `store-arms/way-foot/residual-nest-deaths-deep-trace-2026-10-07.md`: why ants still starved in the nest with the new
  routes on (s3, s4).
- `store-edible-review-deep-trace-2026-10-07.md`: the review of `edible`, and where the too-small crumbs come from.
- This file.
- `store-arms/way-foot/tools/`: the measuring patches (probe v4 and v5, measuring only) and the analysis scripts named in
  those write-ups.
