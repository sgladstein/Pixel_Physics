# Following food home: where it goes after the door, and why the nest store stays empty (2026-10-10)

*Owner's ask (Scott, 2026-10-10 03:38): trace food from the moment it reaches
the door, find why the store stays near empty while larvae go hungry, decide
the store from the trace, then turn the stack on. Every claim is marked
**measured** (counted in a named run), **traced** (followed ant by ant) or
**inferred**.*

## 1. How it should work

Written before looking at the runs, from the biology and from the code
(`Reports/how-the-ant-works.md` §5, §9, §12).

| Step | Real ants | Our ant today (code-read) | What would show it working |
|---|---|---|---|
| 1. A forager brings food home | Most of a load reaches the nest; a forager eats little of it on the way | The crop is cargo and stomach at once: digestion runs every tick, so a carrier eats its load as it walks (§9) | Of the food bitten at the heap, the share digested by its carrier before the door, against the share put down or handed on at home |
| 2. It unloads at the door | Foragers hand food to receivers just inside, and go back out; they rarely go deep (Gordon; Tschinkel 2004 on depth by age -- *not checked here*) | A fed carrier puts cells down at the nest at about 0.25 a tick; the food-door rule keeps the door's own cells clear, so it goes down beside the door, on the mound, or handed through the crowd (§5 step 4) | Where trip cells go down: zone, how far along the way in, by whom |
| 3. Nest workers take it in | Receivers carry it to the brood or to store chambers | `NEST_STORE`'s `carry`: a fed nest worker with empty crop and jaws takes a cell in its jaws and walks it down to the store, at least 20 way steps in (§6d). `pick=20`: also from the doorstep | How many put-down cells are carried in, against swallowed where they lie, and by whom |
| 4. Larvae are fed | Nurses that live with the brood feed it from their crops, by the larvae's hunger signals (Cassill & Tschinkel 1995 -- *not checked here*) | Three routes, each by touch: a larva eats food lying beside it; a carrier touching it gives from its crop (`crop_feed`); the richest adult touching it gives from its bank (`nurse`). Nothing brings food or a nurse to a larva that lies away from both | Larva meals by source; hungry larvae's distance to the nearest food cell and to the store |
| 5. A surplus is banked | When intake beats use, food is stored (granaries, repletes); the store is drawn down when intake falls | The store is food lying beside the deep way. `keep`: a fed ant may not eat store food; `eat`: a hungry ant inside is pulled to it when it holds 8+ cells (within `smell=10` steps) | Store inflow (jaws loads arriving) against outflow (who eats store cells: hungry adults, larvae, nurses); the standing stock over time |

The question has two halves, and the table says where each could break:
**the store stays empty** if little reaches step 3 (food is swallowed where it
is put down, or never put down), or if what arrives is eaten at once
(step 5's outflow). **Larvae go hungry while food comes in** if the food that
does come in lies where no larva touches it, or the ants that carry it never
touch a larva (step 4).

## 2. Setup

Main `e594d970` plus the food log (this branch). `deeptrace foodlog=1`
records every food cell from the moment it moves (`food.csv.gz`), digestion
by place (`digest.csv`), and the floor's food and every larva every 1,000
frames (`foodcells.csv`, `larvae.csv`). Recording changes nothing (seed 1,
heap 90, the stack, 20k frames: `stats.csv` and `colony.csv` byte-identical).

Arms: **main** (no switch set), **stack** (`NEEDS_FIRST=on,backfill
CARRY_HOME=on DOOR_COLUMN=on LAY_BAR=body
NEST_STORE=on,pick=20,jaws,sky,meal,smell=10,edible WAY_FOOT=on`),
**no store** (the stack without `NEST_STORE`), **line** (the stack plus
`RECRUIT=on SPOIL_SIDE=trail`, the owner's playtest line). Beds: **steady food**
(`steady_income`, 40 cells every 1,000 frames 30 columns east; the owner's
main test) and **heap 90** (`nest_goal`, endless food 90 columns east).
Seeds 1-4, 200k frames, mutation off, evolved founder, one thread a run.

Window for every number below: frames 100-200k. Per-seed rows come from
`summary.py`, the larva rows from `larvaetrace.py`, the flow rows from
`foodflow.py` (all in this folder).

## 3. What the trace found

### 3a. The store is not empty in these runs (measured)

| Bed, arm | Store cells standing (mean of 1,000-frame censuses), seeds 1-4 | Loads carried into it |
|---|---|---|
| steady, stack | 98, 68, 48, 110 | 555-719 |
| steady, line | 23, 38, 22, 54 | 273-441 |
| heap 90, stack | 135, 119, 213, 189 | 863-1,264 |
| heap 90, line | 18, 95, 18, 77 | 886-1,166 |
| main, no store (both beds) | 0-3 | 0-4 |

The near-empty store the question started from was the finite-food boom and
bust (store 1-6 cells while the colony starved, `PLAN-2026-10-07.md`). With
food coming in, the store fills. Where its food comes from (steady, seeds
2-4): nest workers take it into their jaws from the mound top (461-663 cells),
the mound tunnels (153-188) and the door (82-131), all of it food a carrier
had put down there.

### 3b. Larvae still starve, and the ones that do lie deep, out of touch (traced, every larva)

Each larva's life followed from egg to `starved` or `pupated`
(`larvaetrace.py`; its count of starved rows matched `stats.csv` within 1-3
on every run checked).

| Run (seed 1) | Starved | Where they lay | Nearest store cell (median) | Meals in a starved life (median) | Since last meal (median frames) | Pupated deep (way 20+) of all pupated |
|---|---|---|---|---|---|---|
| steady, main | 177 | deep 139, way 10-19 27 | no store | 44 | 38,281 | 10 of 735 |
| steady, no store | 233 | deep 216 | (35) | 31 | 16,707 | 63 of 775 |
| steady, stack | 80 | **deep 80 of 80** | 14 | **6** | 6,949 | **467 of 808** |
| heap 90, main | 193 | deep 144 | no store | 26 | 45,721 | 5 of 696 |
| heap 90, stack | 473 | **deep 463 of 473** | 14 | 42 | 5,216 | 236 of 1,689 |

- Starved larvae lie deep on every arm: 75-100% of them at way 20+.
  Larvae that pupate lie mostly at the door, where traffic passes, except
  on the stack, where the store feeds the deep larvae lying beside it
  (steady stack: 467 of 808 pupated deep, eating 297 J a larva off the floor
  against 4 J for the starved ones).
- A larva eats only by touch: food beside it, a carrier's crop, the bank of an
  adult touching it, or a brain's share (`how-the-ant-works.md` §12).
  **Nothing brings food or a nurse to a larva.** So a starving deep larva
  14 cells from a full store starves. On steady stack, the starved larvae
  had a median 6 meals in their life. They were not fed and then dropped.
  Almost nothing ever reached them. (Traced, from the meal rows.)
- On the stack, larvae lie deeper: 77-80% of larva-censuses at way 20+,
  against 45-53% on main (steady, seeds 2-4). Why they end up deep is
  **untraced**: laying site, brood carried, or the nest's shape.

### 3c. The store moves the steady-food deaths from larvae to foragers (measured; the link inferred)

| Steady food, seeds 1-4 | Ants (mean) | Adults starved: surface / mound / nest | Larvae starved | per egg | Eggs |
|---|---|---|---|---|---|
| main | 296, 285, 278, 268 | 15/14/1, 2/10/4, 26/14/2, 4/25/6 | 172, 155, 138, 193 | 0.15-0.22 | 868-930 |
| no store | 274, 303, 286, 285 | 24/0/0, 41/5/0, 46/3/2, 15/4/0 | 231, 204, 179, 203 | 0.18-0.21 | 1,002-1,077 |
| stack | 269, 269, 273, 276 | **113/5/0, 88/31/9, 73/14/1, 62/2/0** | 77, 73, 4, 86 | **0.01-0.10** | 782-870 |
| line | 268, 253, 276, 256 | 22/5/7, 206/7/0, 26/11/51, 21/40/0 | 63, 39, 0, 120 | 0.00-0.12 | 811-972 |

The bed delivers 40 cells every 1,000 frames and every arm eats all of it
(3.6-3.8 MJ bitten at the heap), so the food caps the colony. **What the store
changes is mostly who dies.** Without it, about 200 larvae starve per 100k
frames and 20-50 adults. With it, under 90 larvae starve but 64-128 adults do.
Fewer die in total (92-201 against 222-255), but each adult that starves
has eaten a whole larval life first. The colony is a little smaller with the
store on every seed (269-276 against 274-303, by 5-34 ants). That agrees
with the leave-one-out (`stack-leave-one-out-2026-10-08/`), which found no
store best on every seed of this bed.

The adult deaths, followed (steady stack seed 1, 118 starved, every one):
117 were foragers, not nest workers. Median age was 23.6k, and 85 had bitten
the heap. They starved a median 4.0k frames after their last swallow. Their
last swallow was on the mound top (52), at the heap (30), in the mound
tunnels (24), or in the nest (11). Only 9 last ate a store cell. They die in
bursts: 95 of the 113 surface deaths fell in 130-150k. That is just after the
colony peaked at 349 adults, against 286 at 110k. At 138k, 79 ants stood more
than 60 columns east of the door, beyond the heap. Their median energy was
166 J and falling to 65. Of the 118, 67 starved 120-280 columns east of the
door, past the heap. On the no-store arm, the starvers die west of the door
instead (all 24 on seed 1). The population swings by 60-130 adults on every
arm, main included (seeds 1-4, 10k samples), so the store does not obviously
make the cycle bigger.

Read together (**inferred**): the store's food is the food foragers used to
graze at home. Nest workers carry it in from the mound top and the door, and
`keep` stops a fed ant from eating it. Instead it feeds deep larvae that would
have starved. More of them become adults, the colony overshoots the fixed
income, and at each peak the extra mouths starve where the crowd is, at and
beyond the heap. The per-ant numbers fit this. Carriers ate 120-121 J per
1,000 frames on the stack against 130-138 without the store, and nest workers'
median energy was 205 J against 244-256 (steady, seeds 1-4). What sends the
starving crowd east rather than west is **untraced**.

On heap 90, where food never runs short, the trade does not arise. Adults
starved 0-5 a seed on the stack, larvae starved 0.15-0.20 per egg against
0.17-0.24 without the store, and the colony was 693-720 against 554-591
without the store (4 of 4 seeds) and 252-336 on main.

### 3d. A cost the store carries into digging (the late-tunnels lane's finding, not re-measured here)

The store's band of crumbs, together with `NEEDS_FIRST`'s `job` part (which
works only while the storeroom carries), makes diggers set soil down beside
stored crumbs. Of a room's cuts, 55-64% are set down within two cells. That
lane names this as a main reason the nest stays one room
(`/mnt/project-files/late-tunnels/`). Turning the store on turns that on too.

### 3e. The store is what keeps ants in the nest (measured)

Adult rows of `colony.csv` (one per ant every 1,000 frames) in the dug nest
(`nest`, below the founding ground; the mound's tunnels are `mound_in`),
and the share of nest time spent in stays over 5,000 frames:

| Seeds 1-4 | In the dug nest | Mound tunnels | Nest time in stays over 5k frames |
|---|---|---|---|
| steady, main | 1.2-1.5% | 30-34% | 0% |
| steady, no store | 3.5-5.3% | 32-42% | 1.1-2.1% |
| steady, stack | **21-34%** | 13-23% | **33-49%** |
| heap 90, main | 0.9-1.5% | 35-40% | 0% |
| heap 90, no store | 5.1-5.4% | 32-38% | 6.9-8.3% |
| heap 90, stack | **11-12%** | 35-37% | **22-25%** |

The store is the only food below ground, and with it a quarter to a third of
the colony lives in the dug nest. Without it, almost none does. The late-tunnels
lane found the same at heap 90 over 8 nests (store off 3-6% of rows, leaving
mostly on `hungry out` carrying nothing), and with the store off the nest is a
narrow room down the door column. It measured 843-1,546 cells against
3,236-4,272, and nest digging nearly stopped (300-550 cuts over 200-300k
against 15,000-36,000; `/mnt/project-files/late-tunnels/`).

## 4. What to do with the store

**Turn it on with the stack.** This revises a first reading that said off.
That reading weighed colony size and deaths, and had not yet counted where
the ants live (§3e). The owner's first goal is ants living in the nest, and
the store is the only thing measured that does it: 21-34% of adults in the
dug nest on steady food against 3.5-5.3% without it. It also makes the
colony a quarter bigger on endless food (693-720 against 554-591, heap 90)
and halves larval starvation on steady food.

What it costs, all on steady food, 4 of 4 seeds: the colony is 5-34 ants
smaller, and 3-4x as many adults starve (64-128 against 19-51). The deaths
move from larvae to foragers (§3c). Its crumbs also make diggers set soil down
where they cut (§3d). The late-tunnels lane is testing the store with
`NEEDS_FIRST`'s `job` part off, to see whether the churn goes and the ants
stay. If they do, that is the version to ship.

**What the trace says to build next**, as separate proposals:

1. **Food that reaches deep larvae.** Every starved larva lay deep and was
   almost never fed. A store only helps larvae lying beside it. The fix is
   local: a hungry larva's need reaches a passing nurse or carrier, or brood
   is carried to the food (Cassill & Tschinkel 1995 on larval hunger
   signals, *not checked here*). Why larvae lie deep is still open.
2. **A store that does not take the foragers' food.** The store fills from
   the mound top and the door, which is exactly where foragers eat between
   trips. A store that banks only a surplus (say, only food left lying for a
   while, or only while foragers are fed) might keep the nest life without
   the starved foragers. Untested.
3. **Feeding priority under shortage.** Real colonies under shortage cut
   brood first and keep workers (*not checked here*). With the store, ours
   does the reverse on steady food.

The 12-seed check against main (heap 90, heap 30, steady; main, stack, stack
without the store) is §7.

## 5. Not traced

- How larvae come to lie deep on the stack (§3b).
- Why steady-food starvers die east of the heap on the stack and west of the
  door without the store (§3c).
- Why the stack lays fewer eggs on steady food (782-870 against 1,002-1,077
  without the store).

## 6. Reproduce

```
# a run (steady food, the stack); the switches are in section 2
RAYON_NUM_THREADS=1 PIXEL_PHYSICS_MUTATION=off <the stack's switches> \
  cargo run --release --example deeptrace -- scenario=steady_income food=0 founder=evolved \
  ants=0 census=1 set=ant.digest_hunger_weight=0 seed=1 frames=200000 out=OUT foodlog=1
python3 Reports/follow-food-home-2026-10-10/summary.py OUT ...
python3 Reports/follow-food-home-2026-10-10/larvaetrace.py OUT
python3 Reports/follow-food-home-2026-10-10/foodflow.py OUT_A OUT_B ...
```
