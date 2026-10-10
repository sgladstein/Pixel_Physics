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

Main `43522586` plus the food log (this branch). `deeptrace foodlog=1`
records every food cell from the moment it moves (`food.csv.gz`), digestion
by place (`digest.csv`), and the floor's food and every larva every 1,000
frames (`foodcells.csv`, `larvae.csv`). Recording changes nothing (seed 1,
heap 90, the stack, 20k frames: `stats.csv` and `colony.csv` byte-identical).

Arms: **main** (no switch set), **stack** (`NEEDS_FIRST=on,backfill
CARRY_HOME=on DOOR_COLUMN=on LAY_BAR=body
NEST_STORE=on,pick=20,jaws,sky,meal,smell=10,edible WAY_FOOT=on`),
**no store** (the stack without `NEST_STORE`). Beds: **steady food**
(`steady_income`, 40 cells every 1,000 frames 30 columns east; the owner's
main test) and **heap 90** (`nest_goal`, endless food 90 columns east).
Seeds 1-4, 200k frames, mutation off, evolved founder, one thread a run.
