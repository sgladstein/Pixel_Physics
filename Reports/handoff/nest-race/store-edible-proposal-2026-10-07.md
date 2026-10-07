# Proposal: the store counts only food a mouth can take (Nest race, 2026-10-07 ~21:15 UTC), for Deep trace to check

**What it is for.** The late nest deaths in the WAY_FOOT + backfill rerun (heap 90, seeds 3 and 4: 145 and 260 at
200-300k). Deep trace traced them (`store-arms/way-foot/residual-nest-deaths-deep-trace-2026-10-07.md`, measured):
337 of 396 starvers died with an inedible crumb within 10 cells and no edible store cell within 10. Those deaths count
against NEST_STORE, not WAY_FOOT.

**The cause, checked in the code** (`claude/nest-race-way-foot` 74a2f08a, `src/sim/creature.rs`):
- The store's food test is `loose_food` (14037): it is food if `food_value > 0` and it is not a corpse. That test is used
  by `fill_nest_store` (14084; it builds `store_cells` and seeds the `store` and `store_foot` fields), by `is_store_cell`
  (14163, at the bite) and, through `store_cells`, by `store_can_feed` (14359; at least `STORE_EAT_MIN` = 8 cells).
- The mouth's test is `adjacent_food_counted` (11429): `diet_yield(cell, gut) > EAT_YIELD_THRESHOLD` = 12 J
  (`diet_yield` = worth x `diet_quality`, 0.81 for this gut on crumbs).
- So a crumb worth 0-15 J is store food and not food. The store reads "can feed", which fires the eat pull and turns
  HUNGRY_OUT off (`store_feeds_here`), and the field leads the hungry to that crumb.

## The rule

**A new NEST_STORE part, `edible`: store food is food this colony's mouth can take.** Under `edible`, `loose_food`'s
store use becomes `loose_food(c) && diet_yield(c, colony gut) > EAT_YIELD_THRESHOLD`. That one predicate then feeds all
three readers: the count, the field's seeds and the bite-time check. The store, its smell and its "can feed" can then
never disagree with the mouth.

**Which gut.** The field is shared by the nest, so it needs one gut. I'd use the gut of the nest site's founding species
as authored (`gut_bias` from its `CreatureDef`). With mutation off, as in every test, that is every ant's gut exactly.
With mutation on, an ant whose gut has drifted could still disagree with the store at the margin. That is a known gap, and
it is left visible here rather than closed by a per-ant field. The alternative is a per-ant check: count the store cells
*this* ant can eat when it decides, plus a bite-time check per ant. It is exact under mutation, but it leaves the field's
seeds shared, so the field would still lead to cells the ant cannot eat.

**What does not change.**
- The doorstep pick (`door_food`) still takes any loose food. Carrying a poor crumb in is harmless, and stopping it would
  change what reaches the store.
- `STORE_EAT_MIN` stays 8. It now counts edible cells, so it means what its own doc says it means.
- The inedible crumbs stay where they lie. Nothing removes them, so they will build up in the lobe. Whether that clogs the
  lobe is the first cost to watch. A later part could have a nest worker carry them out, but not in this proposal.

**Local signal:** none new. It narrows the shared store map to what the mouth would already see.
**Biology:** none cited. Inferred: an ant's food stores are what the colony can eat, so the store and the mouth agree.

## Test

- **Base:** the 12-seed heap-90 stack (74a2f08a env with WAY_FOOT=on). Arms: base against base + `edible`, seeds 1-4,
  heap 90, 300k.
- **Judged in this order:**
  1. **The traced problem.** Nest starvers who died within 10 cells of an inedible store crumb and with no edible one
     within 10 cells, via Deep trace's `storecensus.py`. It should go to near zero.
  2. **Starved 20-300k and where,** as N of M with the baseline beside it.
  3. **Deep and fed ants** (scorecard ANTS DEEP / ANTS LIVING DEEP), store bites, and store size in edible cells.
  4. **The cost:** inedible cells in the lobe over time, and whether the eat pull or HUNGRY_OUT now fires differently
     while the store is genuinely empty.
- **Build:** off by default. Guard test: a store of 8 crumbs worth 5 J reads "cannot feed" under `edible` and "can feed"
  without it. I'll watch it fail with the part removed.

## Asked of Deep trace
1. Does `edible` address the traced cause in both cases (s4 empty store, s3 wrong crumb)?
2. One founding-species gut, or the per-ant check?
3. Should poor crumbs also leave the store, or is that a separate part?
