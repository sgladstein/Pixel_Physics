# Does the granary end of `store_in_body` exist? Censusing the nest pile

**Status:** measured pre-flight, 2026-08-30, on `56b6b97` — this lane's
branch with `origin/main` merged in, so 53 commits of worldgen and plant
work are included. One 4-core cloud container. **Every figure was re-taken
after that merge**: it moved them. The first sweep ran on `e7b72e7`, before
the merge, and reported a paired median of +7 cells where the merged tree
gives +5; the qualitative findings are identical and not one number is. Every number here comes from
`examples/larder_probe.rs`, built in the same session as it was run
(`cargo build --release --example larder_probe`, exit code read through
`PIPESTATUS`). Nothing in the engine was changed; this report proposes work
and does not do any.

`Reports/creature-reproduction-economics.md` §5.3 proposes `store_in_body`
as a heritable gene: an animal's surplus sits either in a **granary** beside
the nest or in its own **body** — capital against income breeding. The
owner's ruling, 2026-08-30, is that the two forks must be *"possible based
on our implementation. not that we should manually design two creatures that
do that."* So both ends of the gene's codomain have to be shown reachable
before the gene is written.

**This exists because a gene with one reachable end expresses nothing, and
this project has already paid for that once.** Plants' `light_weight` was
authored up to 0.6 while `phototropism_dir` could only return up-or-nothing;
fixing the codomain took reproduction to zero, because every constant had
been calibrated against the broken quantity
(`Reports/why-changes-cost-so-much-2026-08-27.md`). The replete end of
`store_in_body` is a number on an organism and cannot fail to exist. The
granary end needs cells in the world, and that is the half that can.

---

## 0. Findings, in the order they change a decision

**The one-sentence answer, and it is the uncomfortable one: the granary end
is an empty set.** Not because the world holds no pile — it holds a real
one, made of real cells — but because that pile is a *rolling* handful (a
median of 13 cells, and nothing at all on one seed in six) that no code path
can spend, and a `store_in_body` allele set to "granary" would therefore
express as *throw the surplus on the floor and never breed*.

1. **Nothing can pay a birth from the world, and this is a code fact, not a
   measurement.** `creature::try_bud` gates on `state.energy >= reproduce_at(def)`
   and charges `state.energy -= birth_cost(def)`. The parent's own bank is
   the only bank the birth path can see. A granary of ten thousand cells
   would fund exactly zero births. **The replete end is not one fork of a
   gene today; it is the only implemented mechanism**, and `store_in_body`
   is pinned at "high" by construction.
2. **A pile does exist and is not a rounding error.** Over 18 world seeds,
   at frame 18,000, a colony holds a median **11** free food cells within
   two of its nest against **3** for the same world with no colony in it,
   and 15 of 18 colonies hold something against 9 of 18 empty worlds.
   Paired within each seed the difference is **+5 cells, 14 seeds of 18 up
   against 3 down** — and at band 8 it is 11 up against 7, near a coin flip, so the
   effect lives exactly where a delivery lands and nowhere wider. The
   material says the same thing more sharply: a colony's band holds **leaf,
   moss and seed**, a colony-free one holds **litter and nothing else** —
   the background is what falls, the difference is what is carried.
3. **It does not accumulate, and it stops early.** On the trajectory seed
   the standing count reaches 11 by frame **1,600** and then moves between
   10 and 16 for the next 13,000 frames. **196 of that run's 1,313
   deliveries had happened by then**: 85% of the carrying a colony does over
   18,000 frames buys no pile at all.
4. **And it is a flow, not a store.** Tracked on that same seed as a *set of
   positions* (`mode=turnover`): 174 entries and 163 exits over 15,000 frames, and
   `resident` — positions occupied both at the first non-empty sample and
   now — is **zero from frame 200 onward**. The first pile forms by frame
   100 and is gone by frame 200. A standing ten cannot be told from ten in
   transit by a count, and this is ten in transit.
5. **Persistence is not the blocker, and that is the good news.** A
   hand-planted 40-cell pile in a colony-free world settles to 22–23 and
   holds there for 18,000 frames. The **litter half rots into soil** on
   `decay.rs`'s moisture-gated schedule and the **leaf half does not**
   (`leaf.ron` has no `decays_into` at all). So a granary *can* stand here;
   it just has to be made of the right material.
6. **The colony's net effect on the world's food is dispersal, not
   concentration.** 20,506 deliveries across 18 colonies, against 157,788
   pickups and 156,434 drops: **87% of what an ant puts down, it puts down
   away from the nest.** An ant is a conveyor that happens to pass its own
   nest, and free food ends up spread over the map rather than banked at
   home — §2.2 has the world-wide count beside the banded one.
7. **The larder's peak is worth about half of one child.** Averaged over 18
   seeds, a colony's tight band peaks at **2,427 digestible = 1.30 births**
   against `birth_cost` 1,860 — but the colony-free control peaks at
   **1,420 = 0.76 births** on ambient litter alone, so the part the colony
   put there is **0.54 of a child**. At the settled frame it is 0.72 births
   against 0.29, a colony-attributable **0.43**. Quoting face value would
   have said 5.2 births: `food_value` is what a mouthful is worth to anybody,
   `diet_yield` is what this gut extracts, and at the ant's generalist
   `gut_bias: 0.0` against a plant food's `food_class: -1.0` the filter
   keeps a quarter. **Four-x, and in the flattering direction.**

**What would have to exist for the granary end to be real** is §6. It is
three changes, and only the first is large.

---

## 1. The instrument, and what its numbers count

`examples/larder_probe.rs`. The scene is the one `predation_probe` and
`creature_space::run_one` build — a 512x160 wetland, a nest strip cut into
the surface at x∈[16,90), two trees, 52 ants, 2,400 frames of warmup before
anything walks — reproduced rather than invented, because the 532 deliveries
that prompted this question were logged on it and a census taken on a
different world than the claim it is checking checks nothing.

**The recorded failure this instrument is shaped against**: *a census counted
every `Solid` in the world rather than the platform under test*
(`CLAUDE.md`, one of six instances of this repo's worst-recurring failure).
A count of every food cell in the world is not a count of the pile. So:

- Cells are banded by **Chebyshev distance to the nearest nest cell**,
  multi-source BFS run once per scene. **2 is the tight band and it is not a
  guess**: `act` drops into an empty 8-neighbour of a head that is itself an
  8-neighbour of a nest cell, so every delivery lands at distance ≤ 2 by
  construction.
- The world-wide figure is printed **beside** the banded one and never
  instead of it.
- "Food" is the engine's own predicate — `diet_yield(cell, ant gut) >
  EAT_YIELD_THRESHOLD`, the same test `adjacent_food` runs — not a material
  whitelist, so a picture of the pile cannot disagree with what an animal
  would take from it.
- **Free** (`organism_id() == 0`) is split from **owned**. Carrion and
  dropped mouthfuls belong to nobody; living tissue belongs to somebody
  (`creature.rs`'s `is_living_kin` makes the same distinction). A living
  leaf on a tree is standing crop, not a granary.

### 1.1 Both halves of the control, and the specificity half is the one that matters here

`mode=control`, all four assertions live in the binary:

| arm | ≤2 | ≤4 | ≤8 | ≤16 | world | worth ≤4 | digestible ≤4 |
|---|---|---|---|---|---|---|---|
| bare | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 40 cells planted **at the nest** | 40 | 40 | 40 | 40 | 40 | 19,200 | 4,800 |
| the same 40 planted **at the far end** | 0 | 0 | 0 | 0 | 40 | 0 | 0 |

**Sensitivity**: a planted pile moves the banded census. **Specificity**: the
same pile ~363 cells away moves the world column and *not* the banded one —
so the instrument is counting the larder and not the world, tested rather
than intended. Five of the six wrong numbers in `CLAUDE.md`'s recorded
session had the first check and needed the second.

The third row is also the cheapest possible demonstration that the two
questions have different answers: **40 against 0 on the same world.**

### 1.2 Face value against digestible, which the control caught before the census ran

19,200 and 4,800 for the same forty cells. `food_value` returns
`food_energy` (480 for every food in this scene); `diet_yield` multiplies it
by `(1 - |gut_bias - food_class|/2)²`, which at the ant's authored
`gut_bias: 0.0` against `food_class: -1.0` is 0.25. **Every "the larder is
worth X" figure in this report is the digestible one.** The face-value
column is kept in the output because the two differing by 4x is itself
worth seeing.

### 1.3 The oscillator

Every arm shares its world seed and its frame indices with the other three,
so the day/night and water cycles are common-mode and an arm-to-arm
difference read at one frame is not a phase difference. The absolute
trajectory is printed *as* a trajectory for the same reason: any single
reading off it carries that frame's phase. The rendering modes pin daylight
for the identical reason — the first contact sheet came back with half its
tiles at night, the oscillator aliasing into the picture exactly as it
aliases into a number.

### 1.4 Early and settled

`CLAUDE.md`: a census taken long after an event measures the system's
*response* rather than the event (369 / 42,825 / 67,100 cells at 5 / 50 /
1,300 frames after one blast). The probe therefore samples at 50, 100, 200,
400, 800 and 1,600 frames on top of its regular cadence. It matters here —
§2.1 has the trajectory.

---

## 2. Does the pile exist?

Four arms — `ants` x `planted`, 2x2 — over 18 world seeds, 18,000 frames.
**Six seeds is not a sweep** (`CLAUDE.md`, measured: 1.64x over six and
1.08x over the next twelve, pooling to a per-seed median of zero), so
everything below is an order statistic and nothing is a mean.

Free food cells at frame 18,000, over 18 seeds:

| arm | quantity | min | p10 | med | p90 | max | peak (med) | seeds > 0 |
|---|---|---|---|---|---|---|---|---|
| colony | within 2 of nest | 0 | 0 | **11** | 20 | 43 | 19 | **15/18** |
| no ants | within 2 of nest | 0 | 0 | **3** | 12 | 17 | 8 | 9/18 |
| colony | within 8 of nest | 0 | 3 | **29** | 55 | 138 | 43 | 17/18 |
| no ants | within 8 of nest | 0 | 0 | **5** | 60 | 139 | 31 | 12/18 |
| planted, no ants | within 2 | 21 | 22 | 23 | 34 | 45 | 40 | 18/18 |
| planted + colony | within 2 | 6 | 9 | 22 | 29 | 50 | 41 | 18/18 |

**Read the spread before the medians.** The colony's tight band runs 0 to 43
across seeds and is **empty on 3 seeds of 18**; nothing here is tidy, which
is what an outcome in this engine is supposed to look like (`CLAUDE.md`: a
clean first result is evidence of an artifact). The single seed this probe
was first run on read 3 at the same frame — near the bottom of the
distribution — and would have understated the pile by nearly 4x had it been
quoted alone.

**The paired difference is the number to carry**, taken within each seed so
that terrain, the water cycle and the day cycle all cancel:

| comparison | p10 | med | p90 | seeds up / down |
|---|---|---|---|---|
| colony − no ants, cells within 2 of nest | −4 | **+5** | +16 | **14 up / 3 down** |
| colony − no ants, cells within 8 of nest | −23 | +6 | +53 | 11 up / 7 down |
| planted+colony − planted-no-ants, within 2 | −16 | **−6** | +5 | 4 up / 13 down |

**Read the third column with the fifth.** The colony's effect is real in the
**tight** band — a median of +5 cells with 14 of 18 seeds up and 3 down —
and at band 8 it is **11 up against 7 down**, close enough to a coin flip
that the median there should not be quoted alone. That is the right shape
rather than a disappointment: a delivery
lands within 2 of a nest cell *by construction*, so an effect that lives
there and dies by band 8 is the delivery mechanism showing itself and
nothing else.

**And this table is the reason the probe was rewritten mid-session.** The
first version of this line differenced the two arms' medians and printed
**+9 and +19** under the heading "paired, per-seed" — on the pre-merge tree,
where the genuinely paired figures were **+7 and +7**. A difference of
medians is not a paired statistic; on a distribution this wide it is not
even close to one, and it overstated the effect by about a third in both
bands.

**And the material is the sharper evidence than the count.** Summed over 18
seeds, the free cells within 2 of the nest at frame 18,000:

| arm | what the band holds |
|---|---|
| colony | litter 53, **leaf 25, moss 46, seed 78** |
| no ants | **litter 82** — and nothing else |

A colony-free nest strip collects litter, because litter is what falls. A
colony's nest strip collects moss and seed as well, and neither of those
arrives by falling: they were carried. **This is the cleanest single piece
of evidence that the pile is delivered rather than ambient**, and it needed
no statistics at all.

### 2.1 Early and settled, which are different questions

`CLAUDE.md` asks for a measurement close to the event as well as a settled
one, because a late census can be reading the system's *response* rather
than the event. The probe therefore samples at 50, 100, 200, 400, 800 and
1,600 frames on top of its cadence, and here the early half is where
everything happens:

| frame | 50 | 100 | 200 | 400 | 800 | 1,600 | 3,000 | 9,000 | 15,000 |
|---|---|---|---|---|---|---|---|---|---|
| cells within 2 of nest | 3 | 3 | **0** | 5 | 7 | 11 | 10 | 14 | 11 |
| deliveries so far | 8 | 17 | 23 | 45 | 99 | 196 | 332 | 929 | 1,305 |

The pile is at its steady state by frame 1,600 and 85% of the deliveries
come after that. **A run of 6,000 frames and a run of 18,000 measure the
same pile**, which is worth knowing before anyone spends an hour on a longer
one — and the dip to zero at frame 200 is the flow in §4 seen from the
standing side.

### 2.2 The number a careless census would have quoted instead

The world-wide free-food count is in the same table, deliberately:

| arm | min | p10 | med | p90 | max |
|---|---|---|---|---|---|
| colony, free cells **world-wide** | 139 | 209 | **396** | 547 | 807 |
| no ants, free cells **world-wide** | 152 | 177 | **325** | 495 | 839 |

That column is what "census the food near the ant colony" returns when the
band is left off: **396 against 11**, a factor of 36 on the median. And it
is not merely bigger, it is **blunter**: between a world with a colony and
one without, the banded median moves 11 → 3 (**3.7x**) and the world-wide
median moves 396 → 325 (**1.2x**). Quoting the world column would have said
the larder was thirty-six times its true size *and* been three times less
able to tell whether a colony was there at all. That is the recorded failure
— *a census counted every `Solid` in the world rather than the platform
under test* — with both of its costs made explicit.

---

## 3. Does it persist?

**Measured without depending on delivery at all**, which is the point of the
planted arms: if ants never accumulate a pile, that arm still says whether a
granary *could* stand here. 40 cells — one `litter` and one `leaf` in each of 20 columns of the nest
strip, planted after the warmup in a colony-free world:

| frame | 0 | 200 | 400 | 800 | 1,600 | 3,000 | 6,000 | 12,000 | 18,000 |
|---|---|---|---|---|---|---|---|---|---|
| cells within 2 of the nest | 40 | 40 | 31 | 28 | 26 | 23 | 23 | 23 | 22 |

Nothing at all happens for the first 200 frames, it settles by 3,000, and
then it does not move for another 15,000. Over 18 seeds the settled band
reads **median 23, min 21, max 45, nonzero on every seed** — the tightest
distribution anywhere in this report, and the one arm whose outcome is not
chaotic.

**The material breakdown says which half went.** Summed over 18 seeds at
frame 18,000 the planted band holds `leaf 405, litter 70` — against 360
leaves planted and 360 litter, and against a background of `litter 82` in
the arm where nothing was planted at all. So essentially **every planted
litter cell is gone and essentially every planted leaf cell is still
there.** `litter.ron` carries `decays_into: "soil"` at
`decay_chance_damp: 0.5`; `leaf.ron` carries no `decays_into` and
`corpse.ron` carries none either.

So **persistence is not what stops a granary**, and the shape of the loss is
worth keeping: a pile of litter drains to nothing, a pile of leaf or meat
keeps indefinitely. §5.3's own caveat — *"corpses have no `decays_into` in
the current asset table, so meat keeps for ever while plant litter rots.
That is backwards from reality"* — is confirmed as stated.

---

## 4. Is it a store, or a flow?

**A standing count of ten cannot tell a granary of ten from ten cells
permanently in transit**, and the gene question turns on which it is: the
first is stock `store_in_body` could trade against, the second is a queue.
So `mode=turnover` tracks the tight band as a *set of positions* and counts
entries and exits between samples. `deliveries` is the near side of the
verb; an entry is the far side (`CLAUDE.md`: pair every "it fired" counter
with an effect counter from the far side of the call).

One seed, 15,000 frames, sampled every 250:

| frame | cells | entries | exits | resident | deliveries | eats |
|---|---|---|---|---|---|---|
| 1,500 | 11 | 34 | 23 | **0** | 185 | 0 |
| 6,000 | 12 | 109 | 97 | **0** | 694 | 0 |
| 12,000 | 16 | 158 | 142 | **0** | 1,183 | 5 |
| 15,000 | 11 | 174 | 163 | **0** | 1,305 | 27 |

`resident` is the count of positions occupied both at the first non-empty
sample and now. It is zero everywhere. Sampled every 100 frames instead, the
first pile — three cells at frame 100 — is **gone by frame 200**.

Mean residence works out at roughly 11 cells ÷ (163 exits / 15,000 frames) ≈
**1,010 frames per cell**. Long enough to see in a picture, far too short to
be a store, and nothing in it is the same food twice.

**174 entries against 1,305 deliveries** is the other half of the same
sentence. It is a lower bound — a delivery picked back up inside one
250-frame sampling interval is invisible — which only makes the ratio worse.

---

## 5. Is it ever eaten, and by what?

Yes, and by the colony that built it, but late and not much.

- `eats` is **0 until about frame 10,500** and reaches 79 by 18,000. An ant
  only *swallows* when `energy < start_energy * hunger_fraction` = 450, and
  starting at 900 it takes roughly 10,000 frames of `idle_cost` and
  `move_cost` to get there. For most of a run the pile has no consumer at
  all.
- The colony still removes cells from a pile it did not build: over 18
  seeds the planted band settles at a median **22** with a colony present
  against **23** without, in runs where `eats` totalled 1,259 across all 18
  colonies (≈70 each, and 0 for the first 10,000 frames of every one) and
  `deaths` was **0**. Paired within each seed the difference is a median of
  **−6 cells, down on 13 seeds of 18 and up on 4** — modest, and pointing
  the same way on nearly three quarters of the worlds, which the unpaired
  medians (22 against 23) would have hidden entirely. The removals are therefore
  **pickups**, not meals.
- That is `act`'s own order. The eat/pick-up branch runs **before** the drop
  branch and is gated only on `carrying.is_none()`, so a sated ant standing
  beside its colony's own store picks a cell up rather than leaving it —
  and, still at the nest, may put it down again on a later tick, scoring a
  second delivery. **Nothing marks a cell as stored.** `ant.ron`'s
  `nest_memory` comment already records the visible form of this loop:
  *"arriving, picking food up and then milling on the spot"*.

Summed over 18 colonies: `pickups` **157,788**, `drops` **156,434**,
`deliveries` **20,506**, and **1 death in 18 colonies over 18,000 frames**. Essentially every pickup is followed by a drop, and
**87% of those drops happen away from the nest** — an ant is a conveyor that
happens to pass its own nest, not a stockpiler.

---

## 6. What would have to exist for the granary end to be reachable

Three things. Only the first is real work, and the third may not be wanted.

1. **A birth has to be payable from the world.** This is the whole of it.
   `try_bud` charges `state.energy` and there is no second term; until a
   parent can convert nest-adjacent stock into a child, "granary" is not a
   strategy that can be scored, and no amount of pile makes it one. That is
   a change to the birth path, which is Lane A's file and not this report's
   to design — but note what it buys: it is *also* the mechanism §3.3 of the
   economics report (mass provisioning from the nest store) needs, so one
   change serves both.
2. **A stored cell has to be distinguishable from a mouthful.** With the
   pickup branch ahead of the drop branch and no stored bit, a colony cannot
   hold a pile larger than its own carrying rate: what is put down is picked
   back up. A flag on the cell, or a rule that an ant adjacent to its nest
   does not pick up, closes it. The measurement that says this is the sink —
   rather than rot or predation — is §5's planted-pile pair: the colony
   removes a median 6 cells from a pile it did not build, in runs where
   `eats` is 0 for the first 10,000 frames and `deaths` is 0 throughout.
3. **And the store has to be made of something that keeps.** Persistence
   already works for `leaf` and `corpse` and already fails for `litter`
   (§3), so this one is free today and would stop being free the moment
   `decays_into` is added to corpses — which §5.3 says is the realistic
   thing to do. Whoever owns `decay.rs` should know that a larder mechanic
   would be downstream of that change.

**What is *not* needed**: the pile does not need to be bigger for the gene
to have two ends. A granary worth one child that persists and cannot be
re-taken is a fully expressible strategy. It is not size that makes the
codomain degenerate, it is that nothing reads it.

---

## 7. What this means for the gene, stated as a decision

**Do not write `store_in_body` yet.** Written today it would be a slot whose
low end is silently lethal — a lineage that banks in the granary never
reaches `reproduce_threshold`, because the granary is unreadable — and the
selection that followed would be a measurement of that bug rather than of
capital-versus-income breeding. That is the `light_weight` failure exactly:
an authored weight with a codomain that cannot express it.

**The order that works** is (1) make a birth payable from a nest-adjacent
store, (2) stop stored cells being re-taken, (3) re-derive whatever was
calibrated against the current behaviour, and only then (4) write the gene.
Step 3 is not optional and is the expensive one: `hunger_fraction`,
`reproduce_threshold` and `drop_urge` are all currently balanced against a
world in which the pile is inert, and `CLAUDE.md`'s standing rule is that a
correct mechanism at inherited constants is a regression.

**What §5.3's trade-off table survives.** Its *loss* row — a herbivore
lineage cannot digest its own dead, because the matched filter at −1.0 draws
nothing from a corpse — is measured-true here in the arithmetic (§1.2), and
its *mobility* row is true by construction. The table is good. It is the
premise above it, that both columns are reachable states of the world, that
does not hold today.

---

## 8. Provenance, and how to reproduce every number

```
cargo build --release --examples          # set -o pipefail; read PIPESTATUS[0]
./target/release/examples/larder_probe mode=control
./target/release/examples/larder_probe mode=turnover frames=15000 every=250
./target/release/examples/larder_probe seeds=18 frames=18000 every=3000
./target/release/examples/larder_probe mode=pair frames=15000     # the review card
```

The harness echoes its own parameters on its first line and panics on an
unknown argument, so a log that does not name its seed was written by a
binary that never had one.

**Two limits worth stating rather than burying.** The scene's nest strip is
74 columns wide; `World::found_colony` — what the `Y` key gives a player —
paints 53. The banded census reports the whole nest neighbourhood (≤16 held
21–25 cells at peak against 10–14 at ≤2), so the pile is not merely spread
thin by a wide nest, but the shipped colony was not the scene measured. And
`eats` is only just waking at 18,000 frames; a run long enough for the
colony to start starving in earnest would measure a *consumption* regime
this report does not cover.

A blind A/B was posted to the owner's review queue
(`20260830T014759506Z-618977`, board `creatures`): the colony's nest against
a colony-free one at the same frame, asking which has taken 1,449
deliveries. It was rendered **before** the `main` merge, so its counts are
the pre-merge ones; what it asks — whether a person can see a store — is
unaffected by a 2-cell shift in the median. The verdict is not in this document; whatever it says, it
addresses whether the pile is *visible*, not whether it is *spendable*, and
§0.1 does not depend on it.
