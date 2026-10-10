# LAY_BAR=body: test plan (Nest race, 2026-10-07 03:00), for Deep trace to agree or amend

Scott's standing go (02:56): "If nest race and deep trace agree on a plan feel free to move forward without my approval."

**Built:** claude/nest-race-store-stack 70cba8a4. It carries two settings:
- `PIXEL_PHYSICS_LAY_BAR=body`, off by default. With it off, the run matches 9a4914af map for map and stat for stat through 20k (seed 1, store on). With it on, seed 1 lays 288 eggs by 20k, against 332.
- `deeptrace foodgap=`, Laying's patch. `foodgap=90` puts the heap at x 346.

**Inputs:**
- `laying-bar-proposal-2026-10-07.md`, with the corrections section.
- Laying's review, `laying-bar-proposal-review-2026-10-07.md`, including §5: the constants tuned under the old rule, and that the planted bed needs no food-paid eggs.

## Arms
All arms keep NEEDS_FIRST, CARRY_HOME and DOOR_COLUMN on. Every arm runs at foodgap 30 (unset) and at foodgap 90, seeds 1-4, to 150k.

| Arm | Store | LAY_BAR | Status |
|---|---|---|---|
| off | off | off | heap 30 exists (`off-s*`); heap 90 running now (`g90off`) |
| smell | NEST_STORE=on,pick=20,jaws,sky,meal,smell=10 | off | heap 30 exists (`skysmell-s*`); heap 90 running now (`g90smell`) |
| off+body | off | body | new |
| smell+body | as smell | body | new |

That is 16 new runs, 4 at a time.

## What to read, in this order
1. **Did the switch do its job?**
   - Eggs by the layer's body (Laying's `bodyonly.py` and `paidby.py`). Under `body`, eggs from layers left under 200 J should be about 0.
   - Eggs laid at the heap (`heaplay.py`).
2. **Births:**
   - Births by 20k, 50k and 150k against the same arm without `body` (Laying's founding gap).
   - Colony size over time against the heap refill (216 cells per 1k frames).
   - Whether the heap went bare (`heapground.py` from arm2c/tools, not heapcount).
3. **Food fate:** door pile, store loads picked up and delivered, store size.
4. **Ants in the nest:** deep ants as N of M, door column vs off it, fed and staying.
5. **Starvation and where:** far west, deep, encased (encased read only with starvewhere_deep.py or the PR 652 repo copy; the old tool called every ant below row 175 encased). Larvae starved.
6. **Trip food,** heap 30 vs heap 90.

## Calls I would make
- **`body` works** if poor-layer eggs fall to near 0 and the store-on boom (seed 3) does not happen, without the colony falling below the store-off arm at the same heap distance.
- **If births stall:** check the constants Laying named (REPRODUCE_AT -0.14, egg and pupa frames 250) before calling the bar wrong.
- **Any death cluster over ~50** gets traced ant by ant before any next change is proposed.
- **Next steps:** nothing goes on by default and nothing merges without Scott. Any next change goes through a proposal file and a review, as before.

## Agreed with Deep trace (03:00), with these amendments (now part of the plan)
1. **The store-off share of food-paid eggs is not 2-4% on every seed.** It is 3% / 36% / 10% / 19% at 40-150k (seed 2 reaches 84% per 10k late), all at the top of the shaft, with no die-off. So read off+body against off paired by seed, seed 2 first, at 20k/50k/150k.
2. **The shared feature of every die-off is laying AT THE HEAP**, not store laying. "Any storeroom out-breeds its food" is withdrawn. That heap contact is the trigger is inferred.
3. **Class each death cluster as crowd or famine** (food within 5 cells inside; encased only by the fixed tool) before scoring it. Skymeal seed 3 was mostly crowd.
4. **For any arm that dies:** Laying's layerdeaths.py, plus starvers' birth frames against heap-laying start.
5. **Where food-paid layers stood:** at the heap, the top of the shaft (0-9 rows), or store depth. Deep trace's laywhere.py, in sky-meal/tools/.
6. **Trip food** = trip deliveries and forage_returns, not forage_trips. Compare against skyoff too, since `sky` alone cut trips home with food on seed 1 (untraced; Deep trace is running a focal skyoff s1).
7. **Probe fix applied** (8298e59f, creature.rs part only) in 95d65cd6. Body runs use that build. Stats are unchanged.
8. **Default-on needs Scott's bar** (12 seeds to 300k at 30 and 90, traced, CI green). This 4-seed test is not that.

Running: g90off / g90smell (body off), then b30off, b30smell, b90off, b90smell (LAY_BAR=body), seeds 1-4 to 150k.
