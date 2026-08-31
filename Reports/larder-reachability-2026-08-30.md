# Does the granary end of `store_in_body` exist? Censusing the nest pile

**Status:** measured pre-flight, 2026-08-30, on this lane's branch with
`origin/main` merged in at `2ed5c51` — carrying #142's economy rewrite
(`start_energy` 900 → 200, `birth_grant` a heritable slot, `birth_cost` =
`grant + body_energy × cells` = **1,040**), #154's per-cell metabolism, and
#167's beetle sight sense. One 4-core cloud container.

**Every figure has been taken five times, on five trees, in one session, and
the fifth one is the report.** `main` moved under this study four times
while it was being written. §8a has the five-way comparison and is the part
of this document most likely to be useful to somebody else. The short
version, because two of those merges changed what this report claims:

- **plant + worldgen** (53 commits): moved every number, changed no finding.
- **#142's economy**: changed two findings — the pile acquired a consumer at
  frame 3,000 instead of 10,500, and stopped plateauing.
- **#154's per-cell metabolism**: `idle_cost: 0.10` became
  `idle_cost_per_cell: 0.05`, which for a two-cell ant is *arithmetically
  identical* and touches no rule here. It moved the numbers anyway, and it
  broke one sentence of this report that had been written too tightly (§4a).
- **#167's sight sense**: moved the trajectory seed's deliveries from 625 to
  958 — a 53% change, out of a feature that gives a *beetle* eyes.

**The fifth re-measure was forced rather than chosen**, and the distinction
matters because §8a argues against exactly this. The branch crossed
`branchcheck`'s `BxF` bar (330 against 300), so merging `main` became a
condition of landing; having merged it, the branch carried code these
numbers had not been taken on. That is a different situation from chasing a
moving trunk, and it is the only one that warrants another run.

**On this line, "which tree" is not a footnote on the numbers, it is one of
the inputs** — and the corollary, learned the hard way in §4a, is that a
finding should be stated at the strength that survives a tree change. Every number here comes from
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
   at frame 18,000, a colony holds a median **10** free food cells within
   two of its nest against **1** for the same world with no colony in it,
   and 16 of 18 colonies hold something against 10 of 18 empty worlds.
   Paired within each seed the difference is **+6 cells, 13 seeds of 18 up
   against 2 down**, and at band 8 it is **+14 with 15 up against 1** — the
   first tree on which the wider band is also clean.
   The material says it more sharply than the count: a colony's band holds
   **leaf, moss, seed and corpse**, a colony-free one holds **litter and
   nothing else** — the background is what falls, the difference is what is
   carried and what dies there.
3. **It does not accumulate. It peaks at frame 1,600 and is then eaten to
   nothing.** On the trajectory seed the standing count reaches 11 at frame
   800 — with **120 of that run's 958 deliveries made** — and then falls
   away to single figures for the rest of the run. The other 838 deliveries
   do not merely fail to build a pile, they arrive at one that is shrinking.
   On the pre-#142 economy this curve was flat at 10–16 instead; the
   difference is entirely that ants now get hungry.
4. **And it is a flow, not a store.** Tracked on that same seed as a *set of
   positions* (`mode=turnover`): over 15,000 frames, **145 entries against
   143 exits** — they track each other to within 2% — while 915 deliveries
   were made, and `resident`, the count of positions occupied both at the
   first non-empty sample and now, ends at **0**. Essentially nothing that
   was in the first pile is still there, and essentially everything that
   arrived later has left again. A standing count of ten cannot tell a store
   of ten from ten in transit; this is ten in transit. **Do not quote this
   as an exact identity** — see §4's note: on the previous tree the same
   seed read 119/119 with a standing count of exactly 0, and that tidiness
   did not survive a refactor that changed no rule.
5. **The material keeps; the colony is the sink.** A hand-planted 40-cell
   pile in a **colony-free** world settles to 22–23 and holds there for
   18,000 frames on every one of 18 seeds — the litter half rots into soil
   on `decay.rs`'s moisture-gated schedule, the leaf half does not
   (`leaf.ron` has no `decays_into` at all). Put a colony on that same pile
   and the paired difference is **−14 cells, down on 15 seeds of 18**. So a
   granary can physically stand here, and does not stand *in the presence of
   ants*.
6. **The colony's net effect on the world's food is dispersal, not
   concentration.** 17,305 deliveries across 18 colonies, against 140,202
   pickups and 137,945 drops: **87% of what an ant puts down, it puts down
   away from the nest.** An ant is a conveyor that happens to pass its own
   nest, and free food ends up spread over the map rather than banked at
   home — §2.2 has the world-wide count beside the banded one.
7. **The larder's peak is worth about two thirds of one child.** Averaged
   over 18 seeds, a colony's tight band peaks at **2,293 digestible = 2.21
   births** against #142's `birth_cost` of 1,040 — but the colony-free
   control peaks at **1,080 = 1.04 births** on ambient litter alone, so the
   part the colony put there is **1.17 children**. Note which way #142 moved
   this: the pile got *smaller* and the priced figure got *larger*, because
   a birth got cheaper faster than the larder shrank. Quoting face value
   would have said 9.4 births: `food_value` is what a mouthful is worth to anybody,
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
| colony | within 2 of nest | 0 | 2 | **10** | 20 | 34 | 23 | **16/18** |
| no ants | within 2 of nest | 0 | 0 | **1** | 10 | 21 | 5 | 10/18 |
| colony | within 8 of nest | 0 | 2 | **34** | 97 | 138 | 60 | 16/18 |
| no ants | within 8 of nest | 0 | 0 | **2** | 36 | 142 | 19 | 12/18 |
| planted, no ants | within 2 | 21 | 21 | 25 | 28 | 49 | 40 | 18/18 |
| planted + colony | within 2 | 2 | 3 | 20 | 41 | 44 | 41 | 18/18 |

**Read the spread before the medians.** The colony's tight band runs 0 to 34
across seeds and is **empty on 2 seeds of 18**; nothing here is tidy, which
is what an outcome in this engine is supposed to look like (`CLAUDE.md`: a
clean first result is evidence of an artifact). The single seed the
trajectory is drawn from reads **0** at this frame — the very bottom — and
quoting it alone would have said the larder does not exist at all, which is
a stronger claim than the data supports.

**The paired difference is the number to carry**, taken within each seed so
that terrain, the water cycle and the day cycle all cancel:

| comparison | p10 | med | p90 | seeds up / down |
|---|---|---|---|---|
| colony − no ants, cells within 2 of nest | +0 | **+6** | +19 | **13 up / 2 down** |
| colony − no ants, cells within 8 of nest | +0 | **+14** | +49 | **15 up / 1 down** |
| planted+colony − planted-no-ants, within 2 | −19 | **−10** | +16 | 4 up / **13 down** |

**Read the third column with the fifth.** The colony's effect on its own
band is real — a median of +6 cells with 13 of 18 seeds up and 2 down — and
for the first time on any tree **the wider band is the cleaner of the two**:
+14 with 15 seeds up and a single one down. **And the third row is a
subtraction**: put a colony on a granary somebody else built and it takes a
median 10 cells off it, on 13 seeds of 18. A colony fills its own larder and
empties one it finds, and the second effect is as large as the first.

**These rows have swapped strength across trees.** The tight band has read
+7, +5, +3, +9 and now +6; the wide band has been anywhere from a coin flip
(10 up / 8 down) to 15 up / 1 down. Every sign has held on all five trees;
no magnitude has, and no ordering between the two bands should be quoted as
if it were stable. The band-8 row: a delivery
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
| colony | litter 108, **leaf 13, moss 23, seed 40, corpse 14** |
| no ants | **litter 67** — and nothing else |

A colony-free nest strip collects litter, because litter is what falls. A
colony's nest strip collects moss and seed as well, and neither of those
arrives by falling: they were carried. **This is the cleanest single piece
of evidence that the pile is delivered rather than ambient**, and it needed
no statistics at all.

The `corpse 14` is new since #142 and is not food anybody brought home: it
is dead ants. The colony arm logs **110 deaths across 18 seeds** where the
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
| cells within 2 of nest | 3 | 3 | **0** | 5 | 7 | **11** | 8 | 4 | **0** | 1 | 4 | **0** |
| deliveries so far | 8 | 17 | 23 | 45 | 99 | 200 | 330 | 468 | 563 | 621 | 625 | 635 |
| `eats` so far | 0 | 0 | 0 | 0 | 0 | 0 | 5 | 55 | 75 | 112 | 136 | 165 |
| ants alive | 52 | 52 | 52 | 52 | 52 | 52 | 52 | 51 | 47 | 45 | 40 | 40 |

**The peak is at frame 1,600 and everything after it is decline.** 200 of
635 deliveries had been made by the peak; the remaining 435 arrive at a pile
that is shrinking, and from frame 9,000 the count is bouncing around zero.
The `eats` row is the mechanism — flat at 0 until the colony's first ants
cross their hunger threshold around frame 3,000, then climbing monotonically
while the pile falls and the colony loses a quarter of its ants.

**A short run and a long run no longer measure the same thing**, which the
pre-#142 economy's flat curve did. Anything reading this larder must say
which frame it read — and, given the 0 / 1 / 4 / 0 wobble at the tail, must
not read a single late frame at all.

### 2.2 The number a careless census would have quoted instead

The world-wide free-food count is in the same table, deliberately:

| arm | min | p10 | med | p90 | max |
|---|---|---|---|---|---|
| colony, free cells **world-wide** | 273 | 287 | **381** | 579 | 849 |
| no ants, free cells **world-wide** | 120 | 157 | **277** | 578 | 667 |

That column is what "census the food near the ant colony" returns when the
band is left off: **381 against 10**, a factor of 38 on the median. And it
is not merely bigger, it is **blunter**: between a world with a colony and
one without, the banded median moves 10 → 1 (**10x**) and the world-wide
median moves 381 → 277 (**1.4x**). Quoting the world column would have said
the larder was thirty-eight times its true size *and* been seven times less
able to tell whether a colony was in the world at all. That is the recorded failure
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
| cells within 2 of the nest | 40 | 41 | 32 | 25 | 23 | 22 | 23 | 23 | 22 |

Nothing at all happens for the first 200 frames, it settles by 1,600, and
then it does not move for another 16,000. Over 18 seeds the settled band
reads **median 25, min 21, max 49, nonzero on every seed** — the tightest
distribution anywhere in this report, and the one arm whose outcome is not
chaotic. It is also the one figure that has barely moved across all five
trees (23, 23, 23, 24, 25), which is what a result that does not depend on
creature behaviour should look like.

**The material breakdown says which half went.** Summed over 18 seeds at
frame 18,000 the planted band holds `leaf 404, litter 56` — against 360
leaves planted and 360 litter, and against a background of `litter 67` in
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

| frame | 0 | 400 | 800 | 1,600 | 3,000 | 6,000 | 12,000 | 18,000 |
|---|---|---|---|---|---|---|---|---|
| no colony | 40 | 32 | 25 | 23 | 22 | 23 | 23 | 22 |
| with a colony | 40 | 36 | 30 | 26 | 27 | **17** | **7** | **6** |
| `eats` so far, colony arm | 0 | 0 | 0 | 0 | 3 | 53 | 113 | 168 |

The two arms track each other to within a few cells until about frame
3,000 — which is where the colony's first ants get hungry — and then
separate, ending 22 against 6. Over 18 seeds the paired difference is
**−10 cells, down on 13 seeds of 18**.

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
| 1,500 | 13 | 42 | 29 | 2 | 205 | 0 |
| 4,500 | 7 | 80 | 73 | **0** | 562 | 40 |
| 10,500 | 2 | 121 | 119 | 1 | 820 | 110 |
| 15,000 | 2 | **145** | **143** | **0** | 915 | 141 |

**Entries and exits track each other the whole way — 145 against 143 at the
end, within 2% — while the standing count falls to a handful.** `resident`,
the count of positions occupied both at the first non-empty sample and now,
never exceeds 2 and ends at 0. Essentially nothing that was in
the first pile is still there and essentially everything that arrived later
has left again. A standing count could not have shown this: at frame 1,500
it says "eleven cells", which is indistinguishable from a granary of eleven.

Sampled every 100 frames instead, the first pile (three cells at frame 100)
is **gone by frame 200**.

### 4a. A warning about how tidy this looked one tree ago

On the previous tree the same seed, same probe, same 15,000 frames read
**119 entries, 119 exits, standing count 0, resident 0** — exactly equal and
exactly zero, and the report quoted it as *"everything that ever entered has
left"*, a categorical claim. Merging a `main` that turned `idle_cost: 0.10`
into `idle_cost_per_cell: 0.05` — arithmetically identical for a two-cell
ant, and touching no rule this probe measures — moved it to 109/105 with a
standing 4; the tree after that, which gave a *beetle* eyes, moved it again
to 145/143 with a standing 2.

The finding survives and the phrasing did not, which is the lesson.
`CLAUDE.md` says a clean first result is evidence of an artifact rather than
of a strong effect, and it applies to a *coincidence* as much as to a
number: in a chaotic system an exact identity between two large counts is
something a single run happened to do, not something the mechanism
guarantees. **The claim to carry is "entries track exits and nothing
persists", which held on every tree; the claim to drop is any version with
the word "exactly" in it.**

**145 entries against 915 deliveries** is the other half of the same
sentence: five deliveries in six never occupy a position the larder did not
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
| `eats` | 1,276 | **2,984** |
| `deaths` | 1 | **110** |
| `deliveries` | 20,506 | 17,305 |
| paired effect on a *planted* granary | −6 (13/18 down) | **−10 (13/18 down)** |

Three things follow, and the second is the mechanism.

- **The colony eats its own larder to nothing.** On the trajectory seed the
  pile peaks at 13 cells at frame 800 and spends the back half of the run
  bouncing around zero, while `eats` climbs 0 → 4 → 55 → 171 and the colony
  loses 12 of its 52 ants. Deliveries keep arriving the whole time.
- **But eating is not the only sink, and the planted arms separate them.**
  The colony takes a median 10 cells off a pile it did not build, and it
  starts doing so *before* the ants are hungry — the two planted arms are
  still within a few cells of each other at frame 1,600, when `eats` is 0.
  What removes cells then is **pickups**. `act`'s eat/pick-up branch runs
  *before* the drop branch and is gated only on `carrying.is_none()`, so a
  sated ant standing beside its colony's own store picks a cell up rather
  than leaving it — and, still at the nest, may put it down again on a later
  tick, scoring a second delivery. **Nothing marks a cell as stored.**
  `ant.ron`'s `nest_memory` comment already records the visible form of the
  loop: *"arriving, picking food up and then milling on the spot"*.
- **And some of what is by the nest is the colony's own dead.** 110 deaths
  across 18 seeds, and `corpse 14` in the material census, where before
  there was one death and no corpses.

Summed over 18 colonies: `pickups` **140,202**, `drops` **137,945**,
`deliveries` **17,305**. Essentially every pickup is followed by a drop, and
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
   within a few cells of each other at frame 1,600, while `eats` is still 0,
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

### 8a. The same study on five trees, which is the reusable part

`main` took four creature-affecting merges in the twelve hours this took, so
the whole study was re-run five times. Same probe, same 18 seeds, same
18,000 frames:

| | `e7b72e7` | `+ plant/wg` | `+ #142` | `+ #154` | `+ #167` |
|---|---|---|---|---|---|
| paired median, within 2 of nest | +7 | +5 | +3 | +9 | **+6** |
| seeds up / down on that | 14/4 | 14/3 | 13/4 | 15/2 | **13/2** |
| paired median, within 8 | +7 | +7 | +8 | +14 | **+14** |
| seeds up / down on that | — | 11/7 | 12/6 | 14/3 | **15/1** |
| colony's settled band, median | 13 | 11 | 9 | 11 | **10** |
| turnover entries / exits | 195/185 | 174/163 | 119/119 | 109/105 | **145/143** |
| standing count at frame 15,000 | 10 | 11 | 0 | 4 | **2** |
| paired effect on a planted granary | — | −6 (13d) | −14 (15d) | −10 (11d) | **−10 (13d)** |
| `birth_cost` | 1,860 | 1,860 | 1,040 | 1,040 | **1,040** |
| peak larder, priced | 1.37 br | 1.30 | 2.07 | 2.35 | **2.21** |
| `deaths` over 18 colonies | 0 | 1 | 134 | 144 | **110** |
| planted pile, colony-free, settled | 23 | 23 | 23 | 24 | **25** |

**A different lesson from each merge.**

- **plant + worldgen** (53 commits, none in `creature.rs`) moved every number
  and changed no finding.
- **#142's economy** (`start_energy` 900 → 200) changed two findings. "The
  pile has no consumer for the first 10,000 frames" became false; "the
  standing count plateaus" became "it is eaten away".
- **#154's per-cell metabolism** changed nothing anybody would predict —
  `idle_cost: 0.10` became `idle_cost_per_cell: 0.05` against a two-cell
  body, and `0.05f32 * 2` is bit-identical to `0.10f32`. It moved the paired
  median from +3 to +9 anyway. The cause is `creature.rs:1019`: metabolism is
  charged on `chain.len()`, the animal's **current** cell count, not the
  authored `body.len()`, so a damaged ant pays less to live. **It also broke
  a sentence** (§4a).
- **#167's sight sense** gives a *beetle* eyes and moved the ant's
  deliveries by 53% (625 → 958 on the trajectory seed). There are nine
  beetles and fifty-two ants in this scene; a change to what the beetles do
  reshuffles every ant's world within a few thousand frames.

**The transferable part is the bottom row against the top one.** The
colony-free planted pile reads 23, 23, 23, 24, 25 across all five trees — a
measurement of *materials and decay*, which no merge touched. Everything
above it swings by factors of two to three: measurements of *creature
behaviour*, which every merge touched. Before quoting a creature number, ask
which of those two kinds it is.

**Which sign is safe, and which is not.** Every sign held on all five trees
— the colony's band beats the colony-free control, the planted granary loses
cells to a colony, entries track exits. No magnitude held. And one *ordering*
did not: the tight band was the cleaner of the two bands on four trees and
the wider band is cleaner on the fifth, so even "the effect lives where a
delivery lands" is a claim about a tree rather than about the mechanism.

**The honest limit of a study run against a moving trunk.** Five re-measures
is not diligence, it is a treadmill, and the fifth was **forced rather than
chosen**: `branchcheck` put the branch at `BxF` 330 against a bar of 300, so
merging `main` became a condition of landing, and once merged the branch
carried code these numbers had not been taken on. That is the only condition
under which another run is worth it. Otherwise the answer is to state each
finding at the strength that survives a tree change — which is what §0 does
— and to name the tree, which is what the status line does.

*And the control that should have moved nothing, did not.* Before the
paired statistic was fixed, the first sweep was run twice on `e7b72e7` from
a rebuilt binary whose only change was what it prints, and every arm-level
order statistic came back identical digit for digit. That is the converse of
the stale-binary tell, and a determinism check nobody had to build.

**Two limits worth stating rather than burying.** The scene's nest strip is
74 columns wide; `World::found_colony` — what the `Y` key gives a player —
paints 53. The banded census reports the whole nest neighbourhood (≤16 held
21–25 cells at peak against 10–14 at ≤2), so the pile is not merely spread
thin by a wide nest, but the shipped colony was not the scene measured. And
the run ends while the colony is still losing ants — 12 of 52 gone by frame
18,000 on the trajectory seed, and still falling — so what happens to a
larder in a colony that has finished collapsing is outside this report.

A blind A/B was posted to the owner's review queue
(`20260830T014759506Z-618977`, board `creatures`): the colony's nest against
a colony-free one at the same frame, asking which has taken 1,449
deliveries. **It was rendered on the first of the five trees**, so its
counts are the pre-#142 ones — a card asking whether a person can pick
the colony's nest out of a pair is not sensitive to a few cells either way,
but the numbers under it are not this report's. The verdict is not in this
document; whatever it says, it addresses whether the pile is *visible*, not
whether it is *spendable*, and §0.1 does not depend on it.
