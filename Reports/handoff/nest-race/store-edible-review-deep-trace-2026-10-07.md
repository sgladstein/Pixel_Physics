# Review: `edible`, the store counts only food a mouth can take (Deep trace, 2026-10-07 ~21:20 UTC)

Review of `store-edible-proposal-2026-10-07.md`, read against `claude/nest-race-way-foot` 74a2f08a.

**Verdict: yes, build it off, with one change.** Take the gut from the colony's ants, not from the species file. Two
additions to the test follow below. Measured unless marked *inferred*.

## 1. Does `edible` address the traced cause, s4 (empty store) and s3 (wrong crumb)?

**Yes, both.** The proposal's predicate matches the mouth exactly for every cell the store can hold.

- **The mouth applies one test to a loose cell.** For a cell next to the head, owned by nobody and not a corpse,
  `adjacent_food_counted` (creature.rs:11450) checks `diet_yield(cell, gut.bias) > EAT_YIELD_THRESHOLD`, and nothing else
  bites:
  - `deterred_by_defence` and the armour ratio only touch a plant with a defence and armoured flesh;
  - kin and nectar do not apply to crumbs or provisions.
  - So `loose_food && diet_yield > 12`, with the same gut, is the mouth's own test.
- **s4, the empty store.** At 229.5-262k the store held 0-6 edible cells.
  - Under `edible`, `store_can_feed` reads false, so the eat pull is off. `store_feeds_here` is then false and HUNGRY_OUT
    can send the hungry out.
  - That is the intended change. What happens next is the first risk; see §4b.
- **s3, the wrong crumb.** The field is seeded only beside edible cells, so it leads to food the mouth can take: 13-27
  cells away at the burst, instead of a stub 1-3 cells away.
  - At the burst only 1-5 of the 14-19 edible cells had an ant beside them, so crowding there should not be the next
    wall (*inferred*).
- **The smell gate shrinks with the field.** `smell=10` is counted from the store field (`store_smelt`), so fewer hungry
  ants are "within smell" and more of them get the plain walk out. That is expected, and it is part of what to watch.
- **When the store has no edible cell, the carry still has somewhere to go.** `store_inward` falls back to "deeper
  until a room at `depth`". So a load carried in while the store reads empty founds a new store wherever that room is.
  It may sit away from the stub-filled lobe. That is a behaviour change to look at in the pictures, not a hole.

## 2. One gut or a per-ant check: not the species file's gut

**The species as authored is the wrong gut.**

- `assets/species/ant.ron` authors a generalist: `gut_bias` 0 (`src/lab/scene.rs:1056`).
- The lab's founder is the evolved ant, whose trait is `TRAIT_GUT_BIAS` -0.8 (`LAB_ANT_TRAITS`, `scene.rs:1177`).
- The mouth reads the expressed trait: `gut_of`, creature.rs:10774, returns `traits[TRAIT_GUT_BIAS]`.

On crumbs (`food_class` -1), with `diet_quality` = (1 - |bias - class| / 2)^2:

| gut | quality | a crumb counts above |
|---|---|---|
| the ants' own, -0.8 | 0.81 | 14.8 J |
| as authored, 0 | 0.25 | 48 J |

So with the authored gut, **even with mutation off, the store would disagree with every ant's mouth**. A crumb worth
15-48 J would be food to every lab ant but not store food.

- That errs on the strict side, so the store reads emptier than it is and some hungry ants walk out past food they
  could eat.
- It is the same kind of defect the part exists to remove, just pointing the other way.

**Use the expressed gut of the colony's own ants.** Either:

- the founder's expressed traits, kept at founding, or
- any live member's gut at each rebuild.

With mutation off those are every ant's gut, exactly.

**Per-ant: not needed while mutation is off.** Leave it as the labelled gap the proposal names. If it is wanted when
mutation comes back, the cheap version keeps the shared field and adds a per-ant gate: can *this* ant eat 8 or more of
the store's cells? That gate is the part that holds an ant in, and so the part that kills.

**The guard should be able to catch the wrong gut.** As written, 8 crumbs at 5 J read "cannot feed" under both guts,
so the guard cannot tell the authored gut from the right one.

- **Add a case at 20-40 J with the evolved founder.** It must read "can feed" under `edible`, and it fails if the
  authored gut is used.
- **Put the authored gut back once and watch it go red.**
- **Add a specificity case:** 8 crumbs at 500 J read "can feed" with and without the part.

## 3. Should poor crumbs also leave the store? (answered 22:40)

**Yes, but as a separate part, and the data points at where stubs are made rather than at carrying them out.**

Measured with probe v5, s3 and s4, 150-285k. The probe (`store-arms/way-foot/tools/probe-v5-foodmoves-74a2f08a.patch`, `foodmoves=1`) logs
every food cell put down or picked up in the nest region. It books a put-down only when the putter's own decision row
says a cell went down (`DecisionRow::drop` Placed or Delivered). Stats and death lines are byte-identical to the
unpatched runs.

**The store makes its own stubs, from its own meals.**

| | s4 | s3 |
|---|---|---|
| stubs put down onto store cells | 189 | 184 |
| by nest workers | 180 | 175 |
| the last cell of a home meal (`lunch`) | 155 | 134 |

- **At the grant.** The putters' energy was a median 1.02 grants.
- **Already small.** The cell they put down was a small crumb when it was bitten (median 26-28 J), and it went back down
  at 8-10 J.
- **This is the `meal` part's cycle, running down.** A hungry ant bites a store crumb, holds it until its grant, and puts
  back what is left.
  - Of 16,000-20,000 home-meal put-downs onto store cells, 1,700-1,800 went back at 16-100 J and 134-155 under the
    mouth's line.
  - Each pass leaves the crumb smaller. Once it is under ~15 J no mouth takes it again, so it stays.
- **They are made fastest when the store is busiest:** 27-62 per 10k at 200-230k on s4 and 21-46 on s3. That is the
  window where the store's inedible share rose.

**Stubs above the ground line come from somewhere else, and they stay out of the store.**

- **Who.** 220 (s4) and 162 (s3) were put down by well-fed ants (median 5 grants). Each put down a whole cell chewed
  almost to nothing: bitten at 820-960 J, put down at 6-7 J.
- **Diggers move stubs as soil, but not between the mound and the store.**
  - Of 1,000-1,100 stubs dug from store cells, 804-865 went back onto store cells.
  - No carry from above ground into the store was seen (`store-arms/way-foot/tools/stubjaws.py`).
- **The doorstep pick carried no stub as a store load:** 0 of 1,500-1,700 stub jaws carries.
- **Death spills make few:** 19-20 per run had a worth that matched the dead ant's crop.

**Most of what the census shows as "new" stubs are old ones uncovered.** Over 8,000 appearances per run had no ant to
name. In 98-99% of them, a stub of the same worth had vanished within 8 cells in the previous 1,000 frames: it had been
hidden under a standing ant (`store-arms/way-foot/tools/stubreveal.py`).

*A caveat on my own first log, which lacked the decision-row check.* It booked ~4,700 stub "put-downs" per run. 92% of
them were uncovered stubs next to an ant that had only finished digesting a cell. The numbers above come from the second
log.

**What follows** (*inferred*):

- **Under `edible`** the store ignores stubs, so they stop holding the hungry and stop leading them astray.
- **They keep accumulating.** The meal cycle keeps making them at the store, ~1.4 per 1,000 frames on average and 3-6 at
  the busy time, and nothing removes them.
- **Two ways to deal with them, both separate from `edible`:**
  - Stop a remainder under the mouth's line from being put down: the ant finishes it, ~10 J and a few frames of
    digestion. That would stop the store's stubs at their source, with no new walking.
  - A carry-out part, which would add trips to move ~200 cells per run.
- **The choice and the proposal are yours.** `edible` should go first, because it is the part that removes the deaths.

## 4. The test: two corrections

**a. Judged item 1 would read zero by construction on the `edible` arm.** `storecensus.py` counts inedible *store* cells
(`is_store_cell`). Under `edible` a stub is no longer a store cell, so "starvers near an inedible store crumb" reads zero
whatever the ants do.

Use `store-arms/way-foot/tools/storecensus2.py` instead. It counts:

- an inedible crumb anywhere in the nest region, store or not;
- the nearest cell that is edible and a store cell.

That count means the same thing under both arms. The baseline on the same runs, 200-285k:

| | inedible crumb within 10 and no edible store cell within 10 | no edible store cell within 10 | median distance to edible store / to inedible crumb |
|---|---|---|---|
| s3 | 126 of 137 | 129 of 137 | 24 / 3 |
| s4 | 209 of 249 | 209 of 249 | 22 / 1 |

It needs `foodevery=500` on the runs (the census patch, measuring only), and `nd_rows.csv` from `store-arms/way-foot/tools/nestdeaths.py`.

**b. Add the release check.** Under `edible`, a store that held hungry ants on stubs now lets them all go at once
whenever it drops below 8 edible cells. The release is a hard switch, as in `store-eat-edge-2026-10-07` on s1. The deaths
can move from "held at a fake store" to "failed to climb out".

- **The good sign:** in today's on-arm runs, every hungry ant that left the nest during its spell was fed (234 of 234
  on s3, 174 of 174 on s4).
- **The open question:** on s4 the release will be about 250 ants at once.

Read `store-arms/way-foot/tools/spellfunnel.py` on both arms: lean spells that began in the nest, stayed in or left, then fed or starved.
Then check the starvers who left: where they died, and whether they were climbing.

**c. Count the cost by position, not by the store flag.** For "inedible cells in the lobe over time", the census's
`store` column changes meaning under `edible`. Count `visible == 0` crumbs below the ground line, or near the store's
way cells, in both arms.

## 5. What I am not saying

- That `edible` will lower the death count. The traced cause goes away, but the next limit on s4 is an edible store that
  ran dry for 30k frames. That is only roughly traced.
  - From the same log, most of what reaches the store is recycled meals: home-meal put-downs of 400-870 kJ per 10k.
  - New food arriving there (crops filled away, plus store carries) is only ~100-180 kJ per 10k.
  - From 150k on, what was eaten out of the store exceeded the new food. The edible store fell from ~43 kJ at 150k to
    ~1 kJ at 230k on s4.
  - These flows are approximate. They do not balance the census exactly, because food cells walked into or out of store
    cells are not counted.
- Anything about the off arm. Its s3 deaths are not separated from "a store too small for the crowd".
