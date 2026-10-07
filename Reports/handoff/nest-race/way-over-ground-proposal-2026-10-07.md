# Proposal: the nest's ways prefer ground to the backs of other ants (Nest race, 2026-10-07 08:05 UTC), for Deep trace to check

**What it is for.** Hungry ants in the storeroom runs starve deep in the nest while the way out is open. They climb and fall back. On heap-90 seed 4 the 127 starvers climbed 17,525 cells and fell 18,562 in their last 10k frames. The traces now say why:

- **The ways lead over the crowd.** Every nest way is rebuilt every 30 frames as the shortest way over `way_cell`s. A `way_cell` counts an ant beside it as footing ("in a crowded room ants stand on ants"). So the shortest way up a crowded room runs over the backs of the crowd in the open middle.
- **Following it drops the ant.** By the time an ant steps there, the crowd has moved.

Evidence:
- seed 1, heap 30: Deep trace's probe v2/v3, in `store-arms/sky-meal/shut-in-probe-s1-deep-trace.md`;
- seed 4, heap 90: my run of the same probe (pr2-s4 and pr3-s4, stats byte-identical to b90smell s4), in the last three bullets of `store-arms/sky-meal/laybar-body-results.md`.

All measured.

| | seed 1 heap 30 (Deep trace) | seed 4 heap 90 (mine) |
|---|---|---|
| Falls with only ants round the head | 93.6% | 89.0% |
| Walk-out targets held up only by ants | 38% | 41% |
| Next decision a fall, walk-out, toward an ant-footed vs ground target | 22.0% vs 1.2% | 18.0% vs 1.1% |
| Same, store pull | | 14.6% vs 1.8% |
| Same, soil way out | 23.6% vs 1.5% | 26.1% vs 2.5% |
| A ground-only way within 3 cells, for rows on the crowd | 94.8% | 97.3% |
| The ground-only way's length over the current way, crowd rows, median / p90 | 1.00 / 1.55 | 1.00 / 1.16 |
| Same, the starvers' last frames | 1.17 / 1.47 (last 5k) | 1.03 / 1.17 (last 10k) |
| Crowd rows with NO ground-only way within 3 cells (starvers) | 9-11% | 3-4% |

Ruled out on the way here (measured, heap-90 s4):
- **The store holding ants.** STORE_PATIENCE is on hold.
- **The store lobe churn.** The starvers made only 13% of the lobe's cuts.
- **A shut-in misread.** The starvers read shut in on 1.0% of their decisions.

## The rule

**A way costs more to build over a cell whose only footing is another ant.**

`build_nest_way` today is a breadth-first search where every step costs 1. Under the switch:
- a step costs 1 into a cell with ground or plant among its 8 neighbours;
- a step costs `K` into a cell whose footing is only an animal (Solid, Powder or Plant absent; Creature present);
- the search becomes the small-integer form (Dial's buckets or a 0..K deque), still deterministic in `NEIGHBOURS_8` order.

Why this and not "ground only":
- **Nothing loses its way.** Every cell the way reaches today, it still reaches, so the 3-11% of crowd rows with no ground route still get a direction.
- **Prefer, don't forbid.** The way only bends to the wall where the wall route is shorter than K times the crowd route.
- **The walk is unchanged.** `step_down_way` already walks to the lowest neighbour, and that works on weighted distances unchanged. Ants still stand and climb on ants (`climbs_over_kin` is untouched); only the route's aim changes.

**Which ways:** every way built by `build_nest_way`. That covers the way in and out, which the walk-out, the store field and the rest pull walk. The soil way out (`build_out_way`) is the worst of the three (26% fall-next on s4), and it is built separately. I'd take it as a second named part, so each can be tested alone:
- `way` = build_nest_way;
- `out` = build_out_way;
- `mound` = build_mound_way, only if the code shows it has the same footing.

**Switch:** `PIXEL_PHYSICS_WAY_FOOT=off|on|<parts>[,k=N]`, off by default. `on` = `way,out` at K = 4.
- K = 4 is chosen to exceed the measured p90 length ratio (1.16-1.55), so the wall route wins nearly everywhere it exists.
- Swept at K = 2 and K = 8.

**Local signal:** none new. The way is the colony's shared map, already in the engine. This changes what it prefers, not what an ant senses.

**Biology:** none cited yet. Real ants in a nest walk on surfaces; I know of no ant that routes over a moving crowd in open air. Any wall-following citation would be my inference, so I'm not giving one until I have checked it.

## Costs to watch (not measured)

- **Frame cost.** The rebuild already runs every 30 frames per nest. A weighted search over the same box should stay within a small factor. I'll quote ascii's worst frame before and after.
- **Crowding at walls.** Bending every route to the wall may jam the wall. It is the reason to sweep K, and the probe's fall-next and step counts will show it.
- **Other pulls that read `dist` as a step count.** `depth()`, the deepest cell, and anything comparing distances across ways would now read cost, not steps. I'll grep every reader of `NestWay::dist` / `at()` before building and list them in the PR. Any that must stay a step count gets the old BFS kept beside it.

## Test

**Base:** b90smell and b30smell (LAY_BAR=body + the store parts + NEEDS_FIRST, CARRY_HOME and DOOR_COLUMN on). Seeds 1-4 to 150k at heap 30 and 90. The probe v3 is applied (measuring only, byte-identical), so the same instruments read both arms.

**Arms:**
- off;
- `on` (way,out at K=4);
- `way` alone;
- `out` alone;
- K=2;
- K=8.

**Judged on these, in order:**
1. **The traced problem.** Falls per deep hungry decision in the open middle (`fallwhere.py`), and fall-next after a walk-out step (`routefoot.py`). Did the starvers' up-steps stop being undone?
2. **Starved 20-150k, and where** (`starvewhere_deep.py`). Each cluster over 50 is classed and traced.
3. **Fed and staying deep,** as N of M, split by door column, with the fed/stay check.
4. **Store bites and store size,** and trip food (trip deliveries).
5. **Frame cost** (ascii worst frame).

**Then default-on needs Scott's bar:**
- 12 seeds to 300k at heap 30 and 90;
- clearly better overall, or the traced problem fixed;
- deaths traced;
- Deep trace's written yes;
- CI green.

## Asked of Deep trace

1. Does the rule address what both traces show, and is "cost K" right over "ground only"?
2. Should `out` (the soil way) be in, and is `mound` the same footing?
3. Is K = 4 with a sweep at 2 and 8 sound, given your 1.55 p90 on s1?
4. Anything that reads `dist` as steps that I should know before building?
