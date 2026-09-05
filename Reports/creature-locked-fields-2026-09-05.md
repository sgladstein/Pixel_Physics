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
| `dig_force` | **yes** | **no** | needs a price first |
| `bite_force` | **yes** | **no** | needs a price first |
| `curvature_radius` | **yes** | **no** | needs a price first |
| `digest_rate` | **yes** | **no** | needs a price first |
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

## The queue

1. **`crop_capacity`** — locked, already priced, and its own doc names it as
   "the codomain of a future capacity gene". The cheapest remaining row.
2. **`body` cell count (S8)** — priced since per-cell metabolism landed. Not a
   trait slot: the body is a structure, so this is a real change rather than a
   fifth tuple element.
3. **A price for strength**, then `dig_force` and `bite_force` behind it. The
   verb price exists (`dig_cost_in_moves`) but is flat in force, so today a
   stronger jaw is free.
4. **A price for the curvature disc**, then `curvature_radius`. The shape is
   already solved next door — `sight_fraction` charges per cell *read*, and
   the disc's read count is `(2r+1)^2`, known before the read happens.
5. **A price for digestion**, then `digest_rate`. Least clear of the four:
   what a faster gut should cost is a design question, not a lookup.

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
