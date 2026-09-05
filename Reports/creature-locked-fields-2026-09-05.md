# Which parts of a creature can evolve, and which are still the author's

**2026-09-05.** Status: **live** — the table is the working list, and the
three rows marked *needs a price first* are the queue.

## Why this exists

The owner's ruling, given on the sight gene an hour after it shipped:
**"anything should be able to evolve. don't lock."**

That ruling was applied to one field. It is a statement about all of them, and
nothing in the tree said which the others were. `sight_range` was not
identified as a lock by anybody looking for locks — it was found because a
plan proposed selecting for predator-avoidance and the animal turned out to be
blind. There are twenty-odd more fields on `CreatureDef` and no reason to
believe the next one gets found the same lucky way.

So: every field, classified. The classification has two axes and they are not
the same question.

- **Locked?** Is the value fixed for the whole species for ever, or can a
  lineage move it?
- **Priced?** Does moving it in the "more" direction cost the animal anything?

**Unlocking an unpriced field is worse than leaving it locked.** That is not a
style preference; it is the failure this repo has now paid for three times.
An unpriced lever ratchets to its maximum on the first generation and then
expresses nothing — `idle_cost_per_cell`'s own doc records it for body size,
`sight_fraction`'s records it for reach, and `CLAUDE.md` records the version
that took plant reproduction to **zero**. A gene on an unpriced field is not
variety, it is a constant with extra steps.

Hence the order: **price first, unlock second.** `sight_fraction` landed a
week before `TRAIT_SIGHT_RANGE` and said in its own doc that it existed so
that "the gene arrives into a world that already charges for it".

## The ruling that this document was first written the wrong way round

**Owner, on reading the sentence above: *"everything should be priced."***

The first draft of this report treated *unpriced* as a standing reason to
leave a field locked, and its queue accordingly read as four fields waiting on
a design decision that might never come. That is backwards. **An unpriced
field is a defect to be fixed, not a justification for a lock.** "Leave it
locked" is not one of the available resting states; the resting state is
*priced and open*, and everything not yet there is work.

This changes nothing about the ordering — a price still has to land before or
with its gene, for the ratchet reason above — and everything about what the
table is *for*. It is a work list with a known end, not a classification with
some rows permanently in the wrong column. The rows below marked *needs a
price first* are the remaining prices to author, in order.

**It also disposes of the two "should stay the author's" arguments further
down.** They survive only in the narrow form the ruling leaves them: a *price*
is a rule of the game rather than a property of an animal, so it is not a
locked trait at all — there is nothing there to unlock. Anything that is a
property of the animal gets priced and opened.

## The table

`CreatureDef`, every field, at 2026-09-05.

| field | locked? | priced? | verdict |
|---|---|---|---|
| `traits` slot 0 `gut_bias` | **no** | n/a — an axis, not a quantity | evolving |
| `traits` slot 1 `birth_grant` | **no** | yes, `birth_cost` | evolving |
| `traits` slot 2 `reproduce_at` | **no** | yes, the bar it multiplies | evolving |
| `traits` slot 3 `sight_range` | **no** | yes, `sight_fraction` | evolving |
| `traits` slot 4 `pace` | **no** | **yes, by construction** | evolving — this change |
| `tick_interval` | *was* | yes | **unlocked here**, via slot 4 |
| `sight_range` | *was* | yes | unlocked 2026-09-05 |
| `crop_capacity` | **yes** | **yes**, `carried_cells` charges a load | **ready — next** |
| `body` (cell count) | **yes** | **yes**, every cost is per cell | **ready**, but it is S8 and larger than a slot |
| `dig_force` | *was* | **now yes**, `force_fraction` | **priced here**; unlock next |
| `bite_force` | *was* | **now yes**, the same `force_fraction` | **priced here**, on the max of the two |
| `curvature_radius` | *was* | **now yes**, `curvature_fraction` | **priced here**; unlock next |
| `digest_rate` | **yes** | **no** | price to author |
| `sensor_offset` | **yes** | no — but see below | **safe unpriced**, uniquely |
| `body_energy` | **yes** | it is a price | see *the ones that should stay* |
| `mutation_rate` | **yes** | no | see *the ones that should stay* |
| `climbs_over_kin`, `eats_kin` | **yes** | n/a — booleans | need an axis before they need a price |
| `nest`, `shade_rule`, `instincts`, `hidden_*`, `recurrence` | **yes** | n/a | structure, not scalars |
| `start_energy`, `idle_cost_per_cell`, `move_cost_per_cell`, `dig_cost_in_moves`, `emit_cost_in_moves`, `spoil_weight_cells`, `exposure_cost_per_cell`, `synapse_fraction`, `sight_fraction` | **yes** | they *are* the prices | must stay the author's — see below |

## Why `pace` was the one to take now

`tick_interval` is priced **and nobody had to author the price**, which is
true of no other locked field in the table. Every levy an animal pays —
`idle`, `synapse_tax`, `sight_tax`, exposure — is charged once per decision,
so halving the interval exactly doubles the cost of living per unit of world
time. `CreatureDef::scaled`'s doc had already written the identity down for a
different purpose: idle burn per frame is
`idle_cost_per_cell * cells / tick_interval`.

**And it is very nearly neutral at first order, which is the argument for it
rather than against.** A creature steps one cell per decision, so a quick
animal takes twice the steps and pays twice the bill: joules *per step* do not
move. What does not cancel is everything measured against **world** time —
food regrowing, a predator closing, a rival reaching the same leaf first, a
famine to be outlasted. The gradient on this slot is supplied by the bed, not
by the arithmetic.

That is the shape worth wanting. A gene whose sign is set by the environment
gives **different answers in different beds**, where a gene with a built-in
winner gives the same answer everywhere and stops being interesting on the
generation it saturates.

**Measured, one ant, no reproduction, 600 frames** — the positive control
that the arithmetic reaches the scheduler rather than only the function:

| allele | interval | turns taken | burned |
|---|---|---|---|
| `-1` | 12 | 50 | 5.0 J |
| `0` | 6 | 100 | 10.0 J |
| `+1` | 3 | 199 | 19.9 J |

Turns and joules move together to three figures, which is the claim that this
is not a free speed-up. The 199 is the window edge, not slack: 600 frames at
interval 3 puts the 200th slot on frame 600, one past the end of the run.

**The first version of that measurement read 1,261 / 4,334 / 13,717 turns — a
ratio of 3.4x where the arithmetic says 2x** — because breeding was left on
and budding is attempted once per decision, so a quick ant *breeds* quicker
and the counter was pace multiplied by a population. The excess was real and
was not the gene. Worth recording as `CLAUDE.md`'s tidiness rule running the
other way: here the *untidy* number was the tell, and the clean 2x exists only
because the confound was removed rather than corrected for.

### And in a live bed it decides whether the colony breeds at all

Eight seeds, the sealed lab bed, 9,000 frames, the whole colony set to one
allele. **The two obvious measures point in exactly opposite directions, with
no exceptions in either.**

| seed | animals left −1 / 0 / +1 | born −1 / 0 / +1 | died −1 / 0 / +1 |
|---|---|---|---|
| 1 | 24 / 13 / 10 | 0 / 0 / 2 | 28 / 39 / 44 |
| 2 | 24 / 17 / 16 | 0 / 0 / 3 | 28 / 35 / 39 |
| 3 | 22 / 5 / 11 | 0 / 0 / 1 | 30 / 47 / 42 |
| 4 | 27 / 21 / 18 | 0 / 0 / 5 | 25 / 31 / 39 |
| 5 | 23 / 14 / 8 | 0 / 0 / 2 | 29 / 38 / 46 |
| 6 | 22 / 13 / 9 | 0 / 0 / 3 | 30 / 39 / 46 |
| 7 | 34 / 25 / 13 | 0 / 1 / 5 | 18 / 28 / 44 |
| 8 | 26 / 9 / 11 | 0 / 0 / 2 | 26 / 43 / 43 |
| **total** | — | **0 / 1 / 23** | **214 / 300 / 343** |

**Slow colonies end larger on 8 of 8. Quick colonies are the only ones that
breed, on 8 of 8 — the slow arm reproduced zero times across all eight seeds,
and the shipped ant managed one.**

**The standing-population column is very nearly a tautology and is reported as
one.** This bed starves its colony from 52 animals down to a dozen, so
"animals alive at frame 9000" is a snapshot on a declining curve, and a
half-metabolism ant outliving a double-metabolism one is arithmetic rather
than selection. It measures survival time under starvation, which low
metabolism wins by construction. It is in the table because leaving it out
would hide the disagreement, not because it is evidence.

**The births column is the one that carries information**, and its mechanism
is already written down elsewhere: `wiki/ants.md` records that this colony
"eats its neighbourhood bare and then starves in a bed that is filling up with
food it cannot reach". A birth costs ~1,100 J banked. A quick ant covers twice
the ground per frame, so it is the only one that reaches the far plants and
gets rich enough — and it pays for the trip, dying 343 times against the slow
arm's 214.

So in the owner's own showcase bed, **the pace allele is the difference
between a colony that merely persists and one that reproduces**, and it is
also what kills them. That is the "gradient supplied by the bed" claim
appearing as a measurement rather than as an argument: neither arm is fitter
in the abstract, and which one is fitter here depends on a question about the
bed — whether the far food is worth the trip.

It is also the only gene in the table a person can see without an overlay. The
owner's stated target is *"clear variety in behavior, different methods of
movement"*; a quick ant scurries and a slow one plods, at up to a **4x spread
across the population's extremes**, with no rendering work at all.

## The one field that is safe unlocked and unpriced

`sensor_offset`, and it is worth stating why because the reasoning does not
generalise. It is unpriced — reading a sensor 8 cells ahead costs exactly what
reading one 4 cells ahead costs — so by the rule above it should ratchet.

It cannot, because **its performance curve has an interior maximum**:
`pheromone::tests::trail_following_sweep` puts on-trail tracking at 0.817 at
offset 6, against 0.755 at 4, 0.743 at 8 and 0.727 at 10. There is nothing to
ratchet *to*. A lineage that drifts either way tracks worse.

**The general rule this exposes:** a lever needs a price when "more" is
monotonically better. Where the world already punishes both extremes, the
world is the price. `dig_force`, `bite_force`, `curvature_radius` and
`digest_rate` are all monotone — nothing in the engine makes a stronger jaw,
a wider disc or a faster gut *worse* — which is exactly why they are in the
queue and this is not.

## The ones that should stay the author's, and why that is not a lock

Two categories, and neither is the ruling being quietly re-litigated.

**The prices themselves** — `idle_cost_per_cell`, `dig_cost_in_moves`,
`sight_fraction` and the rest. A heritable price is a lineage that votes on
its own bill, and there is exactly one setting of that vote: zero. This is not
"locked so the author stays in charge"; it is that the field is the *rule of
the game* rather than a property of the animal. The animal-side quantity in
every one of those transactions — how big a body, how far an eye, how fast a
clock — is precisely what these slots unlock.

**`mutation_rate`**, for the same shape one level up: it governs how every
other gene explores, and a lineage that inherits its own mutation rate has a
gradient toward zero in any population that is currently doing well.
`Reports/dead-ends.md` should be checked before anyone builds it anyway.

`body_energy` is the interesting borderline. It is what a corpse is worth, so
a heritable version is prey evolving to be **unpalatable**, which is a real
and attractive mechanism — but it is also the divisor in `carried_cells` and a
term in `birth_cost`, so it is three couplings, not one. It belongs in a
change of its own with those re-derived, which is `CLAUDE.md`'s
*name the constants calibrated against the current behaviour* rule and not a
reason to refuse it.

## What the prices actually cost, measured

Three seeds, the sealed bed, 9,000 frames, all four prices at their authored
values. **One of these numbers contradicts the derivation that set it, and the
derivation was the thing that was loosely stated.**

| lever | share of burn | derived as |
|---|---|---|
| `curvature_fraction` | **0.13%** | 0.39% of an *idle* lifetime |
| `force_fraction` | **1.6%** | 0.05 of an *idle* lifetime |
| `exposure_cost_per_cell` | 0.00% | ships at 0 |
| `digest_fraction` | **4.5–4.6% of intake** | 0.05 of the meal |

**`S = 0.05 of an idle lifetime` is not 5% of what an animal spends, and the
two got conflated.** An idle lifetime prices standing still; a working ant
also *moves*, and `moved` (8,346 J) is larger than everything metabolic put
together (6,913 J). So a lever sized against idle alone lands at roughly a
third of that share of the real bill. The derivations are arithmetically right
and each says "of an idle lifetime" in its own comment — but read quickly they
invite "5% of the budget", which is wrong by 3x.

The remedy is not to re-tune: the sizes are defensible and the ratios between
them are what was being chosen. It is to **quote both**, which is what
`labstats`' *priced levers* line now does, and to size any future price
against **burn** if that is the share meant.

The digestive overhead is the exception and lands where it was aimed, because
it was derived against the meal rather than against a lifetime — a share of
throughput priced per unit of throughput.

**What this does not show.** These runs sit on a `main` that also landed
roots-on-by-default (+30% plant income), so the colony numbers here — 19–21
animals and 2 births at 9,000 frames against 13–17 and 0–1 earlier in the day
— are **not** attributable to the prices and are not claimed to be. Comparing
those would need the paired arms, which is the standing rule about a baseline
measured on a different tree.

## The queue

Under the ruling this is a work list with a known end — every row priced, every
row open — rather than a set of candidates.

1. **`curvature_fraction`** — **done, this change.** Charged per cell the disc
   reads, at *the same per-cell rate as `sight_fraction`*, because a disc read
   and a ray read are both one `World::get` and the price of looking at a cell
   cannot depend on which organ looked. The share then falls out of the work:
   the ant's r=2 disc is 24 cells against 616 for a r=32 cast, so feeling the
   ground is **0.39% of an idle lifetime** where a full sweep is 10%. It is
   quadratic in the radius — r=8 is 4.7%, r=16 is 17.6% — so a broad sense is
   expensive without any hand-written cap. `curvature_radius` unlocks next.
2. **`crop_capacity`** — locked, already priced by `carried_cells`, and its own
   doc names it as "the codomain of a future capacity gene". No price to
   author; it is an unlock.
3. **`force_fraction`** — **done, this change.** A per-tick fraction of
   `start_energy` per unit of force, on the **max** of `dig_force` and
   `bite_force` and never their sum: `bite_force` defaults to `dig_force`, so
   summing would silently bill every species authoring only the one field
   twice over, with both asset lines still reading correctly.

   **A standing cost, not a per-swing one, and that is the design rather than
   a detail.** `dig_cost_in_moves` already charges for *using* the jaw. This
   charges for *having* it — otherwise an animal carries mandibles it never
   opens for free, and any lineage that does not happen to dig faces no
   gradient at all. Derived at each species' own constants, S = 0.05 of an
   idle lifetime: 2.5e-5 for the ant (0.005 J a tick against an idle 0.10),
   1.5625e-5 for the beetle. Half the brain's share and half a full eye
   sweep's — a jaw is expensive tissue but it is not a brain, and the number
   also has to leave the shipped colony's economy where it was.

   **What it makes possible, and the overclaim that had to be withdrawn.**
   The beetle authors `dig_force: 0.3` against soil's `penetration_resistance`
   of 0.8, so it cannot cut ground at all — which is why adding beetles to a
   bed has never made shelter pay. Priced and heritable, the force can evolve
   past 0.8, so burrowing becomes **affordable and physically possible**.

   **It does not make a beetle a burrower, and this report said it did until
   the guard for it failed.** `beetle.ron` wires `Dig` exactly once, as
   `(FoodAdjacent, Dig, 2.0)` — the verb is its *bite*. With no food adjacent
   nothing drives the output, so jaw strength is irrelevant: measured, a
   beetle with the strongest possible jaw in a bank of soil digs **zero
   cells at every allele**. The drive would have to evolve separately, into
   the free `(Bias, Dig)` slot.

   This report's own companion states the rule that would have caught it
   before the claim was made — *audit the animal before the environment: can
   it perceive the thing, can it express the response* — and it was written
   down one document away and still skipped. Worth recording as the recurrence
   rather than quietly fixing: **a price and a gene remove a constraint; they
   do not supply a motive.**
4. **A price for digestion**, then `digest_rate`. The least settled of the
   four, and the reason is worth stating: a fast gut is strictly better today
   (energy sooner, and less weight carried, since `carried_cells` charges for
   what is still in the crop). So the price wants to be a *digestive
   overhead* — a fraction of what is processed, lost to processing it — which
   makes the trade real in both directions: quick and wasteful against slow
   and efficient, with the crop's weight as the counterweight.
5. **`body` cell count (S8)** — priced since per-cell metabolism landed. Not a
   trait slot: a body is a structure, so this is a real change rather than a
   tuple element.
6. **`body_energy`** — what a corpse is worth, so a heritable version is prey
   evolving to be **unpalatable**. Real and attractive, and it is three
   couplings rather than one: it is also the divisor in `carried_cells` and a
   term in `birth_cost`. It gets a change of its own with those re-derived,
   which is `CLAUDE.md`'s *name the constants calibrated against the current
   behaviour* rule and not a reason to refuse it.
7. **`climbs_over_kin`, `eats_kin`** — booleans. They need an *axis* before
   they need a price; a gene on a bit is a switch, not a trait.

**`mutation_rate` is the one genuinely hard case**, and it is not being quietly
exempted. It governs how every other gene explores, so a lineage that inherits
its own rate has a gradient toward zero in any population currently doing well
— the trait would reliably evolve away the capacity to evolve. That is a real
design problem rather than a missing price, and it is the one row that should
be argued out before it is built. `Reports/dead-ends.md` should be checked
first either way.

## What this change also fixed, which was a lock of a different kind

Two of the four trait slots that existed this morning — `reproduce_at` and
`sight_range` — **were not reachable from the lab at all**. Each had shipped
with the parameters page registering the two older slots beside it by hand and
no row of its own, and the inspector's `GENOME` block listed the same two. So
the owner could neither set them nor see what an animal had inherited on them.

Nobody skips a slot on purpose. It is what a registration written one call at
a time does, twice in a row. Both now read one `TRAIT_ROWS` table, so a new
slot is a row in one place or a compile error in the other.

That is worth stating as its own finding: **a field can be unlocked in the
engine and still locked to the person playing the game**, and the second kind
is invisible from the source because everything about it looks finished.
