# Does the granary end of `store_in_body` exist? Censusing the nest pile

**Status:** measured pre-flight, 2026-08-30, on this lane's branch with
`origin/main` merged in at `99f16a7` — **including PR #142, which rewrites
the ant's economy** (`start_energy` 900 → 200, `birth_grant` as a heritable
slot, `birth_cost` now `grant + body_energy × cells` = **1,040**). One
4-core cloud container.

**Every figure has been taken three times, and the third one is the report.**
The first sweep ran on `e7b72e7`; merging 53 commits of plant and worldgen
work moved every number; merging #142 on top of that moved them again *and
changed two findings qualitatively*, because an ant that starts at 200
rather than 900 crosses its hunger threshold about three times sooner and
then eats the pile. §8 has the three-way comparison. The lesson is cheap to
state and was expensive to learn twice: **on this line, "which tree" is not
a footnote on the numbers, it is one of the inputs.** Every number here comes from
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
one, made of real cells — but because that pile is a rolling handful (a
median of 9 cells, empty on one seed in six, and on the trajectory seed
eaten to **zero** by frame 15,000) that no code path can spend. A
`store_in_body` allele set to "granary" would express as *throw the surplus
on the floor and watch the colony eat it as income anyway*.

1. **Nothing can pay a birth from the world, and this is a code fact, not a
   measurement.** `creature::try_bud` gates on `state.energy >= reproduce_at(def)`
   and charges `state.energy -= birth_cost(def)`. The parent's own bank is
   the only bank the birth path can see, and `adjacent_nest` is read by a
   brain input, the drop branch and a visit counter — never by anything that
   looks at what is *in* the nest neighbourhood. A granary of ten thousand
   cells would fund exactly zero births. **The replete end is not one fork
   of a gene today; it is the only implemented mechanism**, and
   `store_in_body` is pinned at "high" by construction.
   **PR #142 did not change this and makes it matter more.** It split
   `birth_cost` into `birth_grant(def, traits) + body_energy × cells` and
   made the grant heritable — so the *replete* end now has a real gene on it
   (`TRAIT_BIRTH_GRANT`, slot 1, authored −0.2) while the granary end still
   has no reader at all. The asymmetry the owner's ruling is about got
   wider, not narrower.
2. **A pile does exist and is not a rounding error.** Over 18 world seeds,
   at frame 18,000, a colony holds a median **9** free food cells within two
   of its nest against **3** for the same world with no colony in it, and 17
   of 18 colonies hold something against 9 of 18 empty worlds. Paired within
   each seed the difference is **+3 cells, 13 seeds of 18 up against 4
   down**. The material says it more sharply than the count: a colony's band
   holds **leaf, moss, seed and corpse**, a colony-free one holds **litter
   and nothing else** — the background is what falls, the difference is what
   is carried and what dies there.
3. **It does not accumulate. It peaks at frame 1,600 and is then eaten to
   nothing.** On the trajectory seed the standing count reaches 11 at frame
   1,600 — with **196 of that run's 607 deliveries made** — and then falls:
   8, 6, 2, 1, **0 at frame 15,000 and still 0 at 18,000**. The other 411
   deliveries do not merely fail to build a pile, they arrive at one that is
   shrinking. On the pre-#142 economy this curve was flat at 10–16 instead;
   the difference is entirely that ants now get hungry.
4. **And it is a flow, not a store — now provably, because the flow closed.**
   Tracked on that same seed as a *set of positions* (`mode=turnover`):
   **119 entries and 119 exits** over 15,000 frames against 600 deliveries.
   The two are equal, and the standing count at the end is **0**: every cell
   that ever entered this larder has left it. `resident` — positions
   occupied both at the first non-empty sample and now — is **zero from
   frame 200 onward**. A standing ten cannot be told from ten in transit by
   a count; here the transit finished.
5. **The material keeps; the colony is the sink.** A hand-planted 40-cell
   pile in a **colony-free** world settles to 22–23 and holds there for
   18,000 frames on every one of 18 seeds — the litter half rots into soil
   on `decay.rs`'s moisture-gated schedule, the leaf half does not
   (`leaf.ron` has no `decays_into` at all). Put a colony on that same pile
   and the paired difference is **−14 cells, down on 15 seeds of 18**. So a
   granary can physically stand here, and does not stand *in the presence of
   ants*.
6. **The colony's net effect on the world's food is dispersal, not
   concentration.** 16,632 deliveries across 18 colonies, against 138,583
   pickups and 136,399 drops: **88% of what an ant puts down, it puts down
   away from the nest.** An ant is a conveyor that happens to pass its own
   nest, and free food ends up spread over the map rather than banked at
   home — §2.2 has the world-wide count beside the banded one.
7. **The larder's peak is worth about two thirds of one child.** Averaged
   over 18 seeds, a colony's tight band peaks at **2,153 digestible = 2.07
   births** against #142's `birth_cost` of 1,040 — but the colony-free
   control peaks at **1,420 = 1.37 births** on ambient litter alone, so the
   part the colony put there is **0.70 of a child**. Note which way #142
   moved this: the pile got *smaller* and the priced figure got *larger*,
   because a birth got cheaper faster than the larder shrank. Quoting face
   value would have said 8.3 births: `food_value` is what a mouthful is worth to anybody,
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
| colony | within 2 of nest | 0 | 1 | **9** | 23 | 28 | 13 | **17/18** |
| no ants | within 2 of nest | 0 | 0 | **3** | 12 | 17 | 8 | 9/18 |
| colony | within 8 of nest | 2 | 5 | **19** | 70 | 189 | 33 | 18/18 |
| no ants | within 8 of nest | 0 | 0 | **5** | 60 | 139 | 31 | 12/18 |
| planted, no ants | within 2 | 21 | 22 | 23 | 34 | 45 | 40 | 18/18 |
| planted + colony | within 2 | 2 | 4 | 14 | 48 | 53 | 41 | 18/18 |

**Read the spread before the medians.** The colony's tight band runs 0 to 28
across seeds and is **empty on 1 seed of 18**; nothing here is tidy, which
is what an outcome in this engine is supposed to look like (`CLAUDE.md`: a
clean first result is evidence of an artifact). The single seed the
trajectory is drawn from reads **0** at this frame — the very bottom — and
quoting it alone would have said the larder does not exist at all, which is
a stronger claim than the data supports.

**The paired difference is the number to carry**, taken within each seed so
that terrain, the water cycle and the day cycle all cancel:

| comparison | p10 | med | p90 | seeds up / down |
|---|---|---|---|---|
| colony − no ants, cells within 2 of nest | −3 | **+3** | +19 | **13 up / 4 down** |
| colony − no ants, cells within 8 of nest | −33 | +8 | +70 | 12 up / 6 down |
| planted+colony − planted-no-ants, within 2 | −19 | **−14** | +12 | 3 up / **15 down** |

**Read the third column with the fifth.** The colony's effect on its own
band is real but small — a median of +3 cells with 13 of 18 seeds up and 4
down. **The strongest row in the table is the third one**, and it is a
subtraction: put a colony on a granary somebody else built and it takes 14
cells off it, on 15 seeds of 18. A colony is measurably better at emptying a
larder than at filling one. On the pre-#142 economy that row read −6 on 13
seeds; ants that get hungry three times sooner eat more of it.

The band-8 row is worth a caution rather than a quote: a delivery
lands within 2 of a nest cell *by construction*, so an effect that lives
there and dies by band 8 is the delivery mechanism showing itself and
nothing else.

**And this table is the reason the probe was rewritten mid-session.** The
first version of this line differenced the two arms' medians and printed
**+9 and +19** under the heading "paired, per-seed" — on the first tree,
where the genuinely paired figures were **+7 and +7**. A difference of
medians is not a paired statistic; on a distribution this wide it is not
even close to one, and it overstated the effect by about a third in both
bands.

**And the material is the sharper evidence than the count.** Summed over 18
seeds, the free cells within 2 of the nest at frame 18,000:

| arm | what the band holds |
|---|---|
| colony | litter 67, **leaf 19, moss 39, seed 36, corpse 12** |
| no ants | **litter 82** — and nothing else |

A colony-free nest strip collects litter, because litter is what falls. A
colony's nest strip collects moss and seed as well, and neither of those
arrives by falling: they were carried. **This is the cleanest single piece
of evidence that the pile is delivered rather than ambient**, and it needed
no statistics at all.

The `corpse 12` is new since #142 and is not food anybody brought home: it
is dead ants. The colony arm logs **134 deaths across 18 seeds** where the
pre-#142 economy logged **1**, so part of what now sits by the nest is the
colony itself.

### 2.1 Early and settled, which are different questions

`CLAUDE.md` asks for a measurement close to the event as well as a settled
one, because a late census can be reading the system's *response* rather
than the event. The probe therefore samples at 50, 100, 200, 400, 800 and
1,600 frames on top of its cadence, and here the early half is where
everything happens:

| frame | 50 | 100 | 200 | 400 | 800 | 1,600 | 3,000 | 6,000 | 9,000 | 12,000 | 15,000 | 18,000 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| cells within 2 of nest | 3 | 3 | **0** | 5 | 7 | **11** | 8 | 6 | 2 | 1 | **0** | **0** |
| deliveries so far | 8 | 17 | 23 | 45 | 99 | 196 | 324 | 442 | 537 | 575 | 600 | 607 |
| `eats` so far | 0 | 0 | 0 | 0 | 0 | 0 | 5 | 52 | 71 | 114 | 137 | 176 |
| ants alive | 52 | 52 | 52 | 52 | 52 | 52 | 52 | 50 | 48 | 48 | 46 | 46 |

**The peak is at frame 1,600 and everything after it is decline.** 196 of
607 deliveries had been made by the peak; the remaining 411 arrive at a pile
that is shrinking, and by frame 15,000 there is nothing left. The `eats`
row is the mechanism — it is flat at 0 until the colony's first ants cross
their hunger threshold around frame 3,000, and from there it climbs
monotonically while the pile falls.

**A short run and a long run no longer measure the same thing**, which the
pre-#142 economy's flat curve did. Anything reading this larder must say
which frame it read.

### 2.2 The number a careless census would have quoted instead

The world-wide free-food count is in the same table, deliberately:

| arm | min | p10 | med | p90 | max |
|---|---|---|---|---|---|
| colony, free cells **world-wide** | 227 | 252 | **335** | 851 | 936 |
| no ants, free cells **world-wide** | 152 | 177 | **325** | 495 | 839 |

That column is what "census the food near the ant colony" returns when the
band is left off: **335 against 9**, a factor of 37 on the median. And it is
not merely bigger, it is **blunter**: between a world with a colony and one
without, the banded median moves 9 → 3 (**3x**) and the world-wide median
moves 335 → 325 — **a 3% difference, which is nothing at all**. Quoting the
world column would have said the larder was thirty-seven times its true size
*and* been unable to tell whether a colony was in the world. That is the recorded failure
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

**Now put a colony on that same pile.** The identical planting, in the
identical world, with 52 ants added:

| frame | 0 | 400 | 800 | 1,600 | 3,000 | 4,000 | 5,000 | 6,000 |
|---|---|---|---|---|---|---|---|---|
| no colony | 40 | 31 | 28 | 26 | 23 | 22 | 23 | 23 |
| with a colony | 40 | 29 | 27 | 27 | 22 | 20 | **13** | **12** |
| `eats` so far, colony arm | 0 | 0 | 0 | 0 | 6 | 17 | 51 | 52 |

The two arms track each other until about frame 3,000 — which is where the
colony's first ants get hungry — and then separate. Over 18 seeds and 18,000
frames the paired difference is **−14 cells, down on 15 seeds of 18**.

**This is the sentence the whole section exists for: the material keeps, and
the colony is the sink.** A granary can physically stand at this nest. It
cannot stand next to ants, because nothing tells an ant that a cell is
stored rather than found.

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
| 4,500 | 2 | 72 | 70 | **0** | 399 | 40 |
| 10,500 | 2 | 109 | 107 | **0** | 569 | 106 |
| 15,000 | **0** | **119** | **119** | **0** | 600 | 137 |

**Entries and exits end equal, at 119 each, and the standing count ends at
zero.** Everything that ever entered this larder has left it. That is as
categorical as a flow measurement gets, and a standing count could not have
produced it: at frame 1,500 the count says "eleven cells", which is
indistinguishable from a granary of eleven.

`resident` — positions occupied both at the first non-empty sample and now —
is zero everywhere. Sampled every 100 frames instead, the first pile (three
cells at frame 100) is **gone by frame 200**.

**119 entries against 600 deliveries** is the other half of the same
sentence: four deliveries in five never occupy a position the larder did not
already have. It is a lower bound — a delivery picked back up inside one
250-frame sampling interval is invisible — which only makes the ratio worse.

---

## 5. Is it ever eaten, and by what?

**Yes, and this is the answer PR #142 changed.** On the previous economy the
pile had essentially no consumer: `eats` stayed at 0 for the first 10,000
frames because an ant started at 900 and only swallows below
`start_energy × hunger_fraction`. At `start_energy: 200` that threshold is
100, an ant reaches it about three times sooner, and the colony turns from a
conveyor into a consumer.

What moved, same probe, same seeds, same 18,000 frames:

| summed over 18 colonies | before #142 | after |
|---|---|---|
| `eats` | 1,276 | **2,898** |
| `deaths` | 1 | **134** |
| `deliveries` | 20,506 | 16,632 |
| colony's settled band, median | 11 | 9 |
| paired effect on a *planted* granary | −6 (13/18 down) | **−14 (15/18 down)** |

Three things follow, and the second is the mechanism.

- **The colony eats its own larder to nothing.** On the trajectory seed the
  pile peaks at 11 cells at frame 1,600 and is at 0 by 15,000, while `eats`
  climbs 0 → 5 → 52 → 176. Deliveries keep arriving the whole time.
- **But eating is not the only sink, and the planted arms separate them.**
  The colony takes a median 14 cells off a pile it did not build, and it
  starts doing so *before* the ants are hungry — the two planted arms are
  still within 1–2 cells of each other at frame 1,600, when `eats` is 0.
  What removes cells then is **pickups**. `act`'s eat/pick-up branch runs
  *before* the drop branch and is gated only on `carrying.is_none()`, so a
  sated ant standing beside its colony's own store picks a cell up rather
  than leaving it — and, still at the nest, may put it down again on a later
  tick, scoring a second delivery. **Nothing marks a cell as stored.**
  `ant.ron`'s `nest_memory` comment already records the visible form of the
  loop: *"arriving, picking food up and then milling on the spot"*.
- **And some of what is by the nest is the colony's own dead.** 134 deaths
  across 18 seeds, and `corpse 12` in the material census, where before
  there was one death and no corpses.

Summed over 18 colonies: `pickups` **138,583**, `drops` **136,399**,
`deliveries` **16,632**. Essentially every pickup is followed by a drop, and
**88% of those drops happen away from the nest** — an ant is a conveyor that
happens to pass its own nest, not a stockpiler.

---

## 6. What would have to exist for the granary end to be reachable

Three things. Only the first is real work, and the third may not be wanted.

1. **A birth has to be payable from the world.** This is the whole of it.
   `try_bud` charges `state.energy` and there is no second term; until a
   parent can convert nest-adjacent stock into a child, "granary" is not a
   strategy that can be scored, and no amount of pile makes it one. That is
   a change to the birth path, which is Lane A's file and not this report's
   to design — but note what it buys. It is *also* the mechanism §3.3 of the
   economics report (mass provisioning from the nest store) needs, and it is
   a route past the thing **Lane A measured as the actual blocker on
   breeding: the body stamp, `body_energy × cells` = 960 of a 1,040
   `birth_cost`.** #142 made the *grant* half heritable and cheap; the stamp
   half is 92% of the price and comes out of one animal's bank. A birth part-
   paid from a nest store is one of the few ways that 960 stops having to be
   saved up by a single ant. So this is not only the gene's prerequisite —
   it is on the critical path of the breeding problem itself.
2. **A stored cell has to be distinguishable from a mouthful.** With the
   pickup branch ahead of the drop branch and no stored bit, a colony cannot
   hold a pile larger than its own carrying rate: what is put down is picked
   back up. A flag on the cell, or a rule that an ant adjacent to its nest
   does not pick up, closes it. The measurement that separates this sink
   from eating is §5's planted-pile pair *read early*: the two arms are
   within 1–2 cells of each other at frame 1,600, while `eats` is still 0,
   and only diverge once the colony gets hungry around frame 3,000. What
   removes cells before that can only be pickups.
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
correct mechanism at inherited constants is a regression. **#142 is the
worked example, from this week**: it moved `start_energy` and every number
in this report moved with it, two of them enough to change a finding.

**And the gene is now visibly lopsided in the source, not just in the
world.** #142 gave the replete end a real heritable slot —
`TRAIT_BIRTH_GRANT`, authored −0.2, mutating on every birth. The granary end
still has no reader. Adding `store_in_body` beside `birth_grant` would put
two alleles in one genome where one of them is connected to the simulation
and the other is not, which is the harder version of the bug to find later.

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
deliveries. **It was rendered on the first tree**, before either merge, so
its counts are the pre-#142 ones — a card asking whether a person can pick
the colony's nest out of a pair is not sensitive to a few cells either way,
but the numbers under it are not this report's. The verdict is not in this
document; whatever it says, it addresses whether the pile is *visible*, not
whether it is *spendable*, and §0.1 does not depend on it.
