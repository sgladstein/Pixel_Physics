# Gate 0: ants breed, and the thing that was stopping them was satiety

**Status: built and landed.** An ant that is close to affording young now
finishes the meal instead of carrying it home, and takes the best mouthful
within reach rather than the first one the neighbour array happens to hit.
On the lab bed with food on the ground the shipped, unmodified ant reaches
**generation 13**; on the shipped worldgen world the same colony goes from
**0 births to 1** at the authored gut and from **0 to 6, generation 1**, at a
specialised one. What still blocks Gate 0 in the *unfed* lab bed is measured
here to the cell, and it is a foraging problem rather than an economy one.

Files: `src/sim/creature.rs`, `src/sim/world.rs`,
`examples/windfall_probe.rs`, `wiki/ants.md`.

---

## 0. Findings, in the order they change a decision

1. **The block was never the economy and it was never the gut. It was that a
   full ant stops eating.** `creature::act` fed an animal only below
   `hunger_fraction * start_energy` and made it carry everything after that,
   so the largest bank any animal could ever hold was **the satiety line plus
   one mouthful** — 1,060 against a birth bar of 1,041 and an authored bud
   threshold of 1,100. The colony was not short of food. It was *full*, and
   being full is what stopped it. §2.
2. **Given food, the shipped ant at the shipped neutral gut breeds to
   generation 13.** This is the positive control and it is the strongest
   result in the round: it says the whole birth economy —
   `birth_grant`, the body stamp, `reproduce_threshold` — is *solved* as
   authored, and nothing further is owed on the arithmetic. §4.
3. **"Cannot reach" is true, and now it has a number.** Ants climb 25–35 rows
   up the stems, organs stand as low as the soil line, and the new
   `best_offer` counter says that over 24,000 frames **no ant in the colony
   was ever offered anything better than a leaf**. It is not a steering
   failure at the food; there is almost no food. §3.
4. **The fruit → windfall pipeline is *set-and-hang*, not eaten-instantly, and
   the no-ant control is what separates them.** 13–29 windfalls reach the
   ground per 24,000 frames; mean standing stock is **0.15–0.57 cells** across
   a 512-wide bed and each one stands for **568–765 frames**. With the colony
   removed entirely the stock rises only 0.40 → 0.57, so ants are not the
   sink. Meanwhile 4–8 fruit stand ripe at any moment and
   `organ_ripening_blocked` runs 1,700–2,800: the plant sets fruit and then
   cannot pay the instalment that lets it fall. §3.
5. **The last link is the delivery loop, and it is the handoff.** In the lab
   bed the colony makes **1,651 pickups and 4 deliveries**, and the nest
   larder censuses at **0 food cells**. The ladder this change builds eats
   from the colony's stores, and in that bed the stores do not exist. Isolated
   with a controlled probe: scale the out-of-nest drop probability to zero and
   deliveries go 4 → 13, larder 0 → 3, births **0 → 2, generation 1**, with
   nothing else changed. §5.
6. **A specialised ancestral gut was built, measured, and reverted, because
   the owner has already ruled on it.** At `gut_bias = -1.0` the lab bed
   reaches generation 2 on two seeds with no scaffolding at all — and
   `a_starved_nestmates_corpse_is_still_dinner` carries the owner's verdict
   on review card `20260823T104411499Z-963f8d` in as many words, *"An omnivore
   should be viable"*, and is written to go red for exactly this. It did.
   Recorded in `dead-ends.md` with its numbers rather than retried. §6.

---

## 1. What was built

**`adjacent_food` returns the best mouthful in reach, not the first.** It
short-circuited on the first neighbour over `EAT_YIELD_THRESHOLD` in
`NEIGHBOURS_8` order, so an animal standing on a stem between a leaf and a
flower ate whichever the array reached first. That is not a preference, it is
an artifact of a loop — and it is expensive here in a way it would not be in
most engines, because what decides whether an ant can afford a child is
`hunger_fraction * start_energy + one mouthful`, so *which* mouthful is the
whole of the arithmetic.

**Provisioning: an animal short of a child's price keeps eating.** Two clauses,
and the second is the colony's own loop finally being load-bearing:

* **out on the route**, only when the mouthful in front of it reaches the bar
  by itself. Everything smaller still goes to the nest, so the delivery loop
  is untouched;
* **at the nest**, for as long as the larder lasts.

The obvious rule — *keep eating while below the bar* — is the recorded
`hunger_fraction` dead end in a new costume: every ant is below the bar
essentially always, so nothing would ever be carried home again (deliveries
1,733 → 3 when that was tried directly). The bar an individual is measured
against is read through the same `reproduce_at` / `birth_cost_of` pair that
`try_bud` reads it through, so the feeding rule and the birth cannot disagree,
and it is `None` for a species that does not reproduce — which is what makes
the branch free for everything that does not breed.

**Three counters, in `CreatureStats`.** They exist because `eats` cannot
answer any of the three questions this round is about:

| counter | says |
|---|---|
| `best_offer` | the largest mouthful any animal was ever **offered** — the best cell in its own 8-neighbourhood, whether it took it or not |
| `best_bite` | the largest it ever **swallowed**, after the gut's matched filter |
| `peak_bank` | the highest bank ever held, sampled where every charge and credit passes, rather than censused over the survivors at the end |

The first two are the near and far sides of one call, which is the standing
house rule, and the pair is the whole diagnosis: **a best bite stuck at the
leaf value means *the good food was never within reach* if `best_offer` is
stuck there too, and *the animal walked away from it* if it is not.** Those
want opposite fixes. `peak_bank` exists because `richest bank`, as every
harness here reports it, is a census of the survivors — an animal that reached
the bar and then spent it back down, or reached it and died, is invisible in
that number, which against a birth question is exactly the wrong way round.

**`examples/windfall_probe.rs`**, the fruit → windfall counter the programme
asked for. See §7.

---

## 2. The satiety roof, measured rather than argued

The positive control came first, because *"never exceeds one standing cell"*
is three findings wearing one number and none of them could be tested without
putting food where it was wanted. `handout=` drops fresh windfall on the
colony's own doorstep and changes nothing else.

At `gut_bias = -1.0`, lab bed, 24,000 frames, **before** the provisioning rule:

```text
best mouthful ever offered   960      (a whole windfall)
best mouthful ever swallowed 960
peak bank ever held        1,060      = 100 satiety line + 960
birth cost                 1,040   bud threshold 1,100
births                         0
```

The ant reached the birth *cost* and stopped **40 points short of the
threshold**, having eaten the best thing in the world. It could not take a
second bite, because after the first it was no longer hungry. That is the
whole of the deadlock, and no amount of food fixes it: the roof is a rule,
not a shortage.

With the provisioning rule, the same arm:

```text
peak bank ever held        2,008
births                        54   deepest generation 6
```

---

## 3. Which of the three failure modes is true

The brief asks for **"cannot reach"** against **"reaches but the bank ceiling
blocks"** against **"reaches and breeds but dies first"**, with the
measurement that separates them. All three arms are 24,000 frames on the lab
bed, shipped species, seed 1 unless stated.

**The ceiling blocked, and it no longer does.** §2 is that arm, and it is
settled: the fix is landed and the positive control breeds.

**Cannot reach is true, and `best_offer` is what says so.** In every unfed
arm, at every gut, `best_offer == best_bite` exactly:

| arm | best offered | best swallowed | peak bank |
|---|---|---|---|
| shipped, seed 1 | 120 | 120 | 769 |
| shipped, seed 7 | 240 | 240 | 598 |
| `gut=-1.0` | 480 (a leaf) | 480 | 580 |

No ant ever declined a better mouthful, because no ant was ever shown one.
And it is **not** a height problem: organs stand from **0 to 49 rows** above
the soil, ants reach **25–35 rows** and 3–9 of them are up a stem at any
moment. There is simply almost nothing standing — mean 4.8 flower and 4.6
fruit cells across a 512-wide bed.

**Dies first is not the failure mode**, and the fed arm says so: 148 births
against 64 deaths, 136 ants alive at the end of the run.

### 3a. The fruit → windfall pipeline, and the control that reads it

| | shipped colony | **no ants at all** |
|---|---|---|
| windfalls created in 24,000 frames | 14 | 21 |
| mean standing windfall | 0.28 | **0.57** |
| mean time one stands | 568 frames | 653 frames |
| ripening refused for want of budget | 1,732 | 2,793 |
| standing flower / fruit, mean | 4.8 / 4.6 | 7.3 / 7.9 |

Read the middle column against the right one. **Removing every ant from the
bed roughly doubles the standing stock and leaves it under one cell**, so
"they are eaten the moment they land" is false — a windfall stands for the
better part of ten seconds of play and there is still never one there,
because only twenty are ever made.

The refusal counter says where they are lost. `organ_ripening_blocked` fires
once per tick that a ripe organ cannot pay its instalment, so 1,700–2,800 with
4–8 fruit standing is not 1,700 lost fruit — it is a handful of fruit
standing ripe and waiting for thousands of frames each. **The herb sets fruit
and then cannot afford to let it go.** That is `herb.ron`'s reproductive
economy and it is left for the plant line rather than tuned from here; the
counter is the handoff.

---

## 4. The positive control, and what it licenses

`CLAUDE.md`'s remedy for a null is to construct the case whose answer you
already know. `handout=30` drops one windfall near the colony every 30 frames
and touches nothing else.

| | unfed | **fed** |
|---|---|---|
| births | 0 | **148** |
| deepest generation | 0 | **13** |
| ants alive at the end | 15 | 136 |
| plants standing | 177 | 37 |
| peak bank | 769 | 1,601 |
| nest larder at the end | 0 cells | 19 cells |
| deliveries | 4 | 191 |

**This is the shipped ant, at the shipped neutral gut, with the shipped
`reproduce_threshold`.** So the birth economy is not owed anything further:
every route the birth-grant report listed as necessary — a smaller stamp,
fission, a specialised gut — turns out to be unnecessary once the animal is
allowed to finish its meal. What Gate 0 is waiting on is food arriving where
the ants are.

Two things worth carrying out of the fed arm. The largest mouthful anyone took
was **812**, which is not a windfall (240 at this gut) — it is a **corpse**,
carrying a dead ant's leftover bank in `aux`. So a breeding colony
*bootstraps*: the first ant to bank big and die leaves the richest food in the
world. And the bed visibly pays for it, 177 plants down to 37, which is the
grazer-versus-crop pressure the lab wants and is on the owner's review queue
as `20260830T184356556Z-19553e` rather than assumed.

---

## 5. What is still in the way, isolated

The colony forages and does not bring anything home.

```text
lab bed, 24,000 frames, shipped ant
  pickups 1,651   drops 1,586   deliveries 4
  food cells standing within reach of a nest cell: 0
```

1,586 of 1,590 drops happen away from the nest. The ladder this change builds
eats from the stores, and there are none.

**Isolated with a controlled probe rather than inferred.** `act`'s
out-of-nest drop probability is `drop_urge * moisture_gradient(x, y)`; scaled
to zero, on the same bed and seed:

| | shipped | route-drop scaled to 0 |
|---|---|---|
| pickups | 1,651 | 115 |
| deliveries | 4 | **13** |
| nest larder at the end | 0 | **3** |
| births | 0 | **2** |
| deepest generation | 0 | **1** |

So the drop rule is the term, and the probe was removed rather than landed —
`moisture_gradient` is a **magnitude**, so it is large at *any* moisture
boundary, and a soil bed under air is one continuous boundary that every
forager walks along. Whether that is the right reading of the stigmergy model
is a design question for whoever owns foraging, not a knob to turn from here.
Filed as a bug with this table.

---

## 6. The gut, built and reverted

At `gut_bias = -1.0` the ancestral ant clears Gate 0 outright, and the
arithmetic predicts the value rather than fitting it: two adjacent larder
cells reach the bar when `100 + 960q >= 1041`, i.e. `q >= 0.980`, i.e.
`gut <= -0.96`. Measured, 60,000 frames, two seeds:

| gut | seed 1 | seed 7 |
|---|---|---|
| 0.0 | 0 births, generation 0 | 0 births, generation 0 |
| −0.9 | 2 births, generation 1 | 7 births, generation 1 |
| −1.0 | 7 births, **generation 2** | 14 births, **generation 2** |

−0.9 misses for a structural reason rather than a tuning one: it needs
*three* adjacent larder cells instead of two.

**It was reverted on the spot.** A corpse is `food_class: +1`, so at −1.0 the
matched filter pays it exactly 0 and the scavenging half of the diet
disappears — and `a_starved_nestmates_corpse_is_still_dinner` says, in its own
comment, that the shipped neutral gut is the owner's verdict on review card
`20260823T104411499Z-963f8d` (*"An omnivore should be viable"*) and that *"if
a future retune narrows the ant off the flesh end, this failing is the
point."* It went red. That is a guard doing its job, and the finding it
protects outranks a convenient number.

---

## 7. `windfall_probe`, and what it generalises to

`cargo run --release --example windfall_probe -- frames=24000`

* **Production and standing stock are counted separately and divided.**
  `World::fruit_dropped` is every windfall ever created; the census is what is
  on the floor now. Little's law closes them: `mean standing / production
  rate` is the **mean time a windfall stands**, so a stock of one cell is
  readable as "one is made every 400 frames and lasts 300" or as "four hundred
  are made and each lasts one frame". Neither number alone can say which, and
  they are opposite worlds. The trick is not about fruit and works for any
  stock whose creation is counted.
* **The census is by organism, not by grid sweep** — every cell it cares about
  is organism-owned — so sampling often enough for Little's law costs a few
  hundred lookups rather than 163,840 per sample.
* **Windfall is counted by *height*, not merely by count.** One lodged in the
  canopy is food an ant cannot reach and is otherwise the same cell in the
  same census as one in the leaf litter.
* **`handout=` is the positive control**, and §4 is the reason the binary
  exists in this shape at all.
* `png=` writes the bed at the end of the run, because "a colony that breeds
  eats the stand" is judge-by-eye.

**One trap it inherits and answers**: the gut is written to the species and
the colonies are founded *afterwards*, because `place_creature` copies the
trait at placement — a species-level write after the founders are standing
reaches nobody and the run silently measures the neutral gut.

---

## 8. Guards, and that they can fail

Both new tests were put back-to-front deliberately, per `CLAUDE.md`: the fault
each is named for was reintroduced and each went red.

* `an_animal_is_offered_the_best_mouthful_in_reach_not_the_first` — a leaf at
  `NEIGHBOURS_8[0]` and a flower at `NEIGHBOURS_8[7]`, with a third arm that
  says the scan still finds the leaf when the leaf is the only thing standing.
  Restoring first-match: **red**.
* `an_ant_at_the_nest_eats_past_satiety_to_pay_for_a_child` — three arms on
  one scene: at the nest the bank must climb past the old roof; away from the
  nest with a leaf-sized mouthful the roof must hold; and a species with
  `reproduce_threshold: 0` must not provision at all. The roof is computed
  from the def rather than hardcoded, so an economy retune moves the bar with
  the mechanism. Removing the provisioning branch: **red**.

`cargo test --lib`: 1,175 passed, 0 failed, 55 ignored.
`cargo clippy --all-targets --release --locked -- -D warnings`: clean.

---

## 9. Provenance

Everything above is one machine, one session, 2026-08-30, with the release
examples rebuilt between every code change — species files are `include_str!`'d
and a stale binary produces bit-identical "runs". The outdoor arm is a **paired
A/B against `origin/main` built in its own worktree**, so the two sides differ
only by this change:

| `stamp_probe terrain=world frames=24000`, seed 2583 | `main` | this branch |
|---|---|---|
| births at the shipped gut | 0 | **1** |
| births at `gut=-1.0` | 0 | **6** |
| deepest generation at `gut=-1.0` | 0 | **1** |

No timing is quoted. The change adds seven `World::get` and seven
`diet_yield` per creature tick in the worst case (`adjacent_food` no longer
short-circuits) and one `adjacent_nest` on the provisioning branch; a creature
ticks every `tick_interval` frames, not every frame, and four agents shared
this box, which `Reports/measurement-under-contention.md` says makes any clock
here untrustworthy. If it is wanted, it wants a quiet machine and
`examples/ascii`.

Reads on: `Reports/creature-birth-grant-2026-08-30.md` (whose §2 arithmetic is
confirmed exactly and whose §6 conclusion — that closing the gap needs the
stamp term — is superseded: it needs the satiety rule instead);
`Reports/evolution-lab-gate-1-2026-08-30.md`;
`Reports/lanes/evolution-lab-coordinator.md`.
