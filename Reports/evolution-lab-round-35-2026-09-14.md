# Round 35 — you can watch a colony eat, and a stranger turns out to be food

*The owner's ask, 2026-09-14: **"We should explore better instruments,
visualizations, whatever for the player to understand the food economy of each
colony. I want to know what they are eating, where it is coming from, if/where
it is being stored or movement paths, general colony food stats/balances"**,
and beside it **"review/explore why we don't have different colonies fighting
or eating each other"**, then **"do some research on when and why different
types of ants, bugs and bigger creatures fight and how that might relate to our
game. Implementation."** Ruled at 04:43, overturning the coordinator's caution:
**"You can ship it on."** Four lanes, all Opus, run overnight while he slept.*

*Design of record: [`colony-food-economy-design-2026-09-14.md`](colony-food-economy-design-2026-09-14.md).
Landed #416, #417, #419, #420, and #409 from round 34's tail.*

---

## 1. The finding that reframes his second question (#417, lane D)

**A stranger is already food, and nobody had noticed.** `ant` material carries
`food_class: 1.0` and the shipped ant's gut sits at a neutral bias, so the
moment two colonies fall outside each other's tolerance **each is prey to the
other's ordinary mouth** — through `adjacent_food_counted`, with no `Attack`
weight involved anywhere. Total `eats` goes **54 → 750** on a 4,000-frame bed
when the only thing that changes is whether the colonies recognise each other,
and cross-colony kills (9) outnumber `Attack` kills (4) two to one.

**Turning rivalry on does not produce a war. It produces predation.** The
engine already models intraguild predation by default, and models interference
competition — fighting you do not eat — only through the unwired `Attack` verb.
That is a different mechanism from the one everybody, this coordinator
included, assumed the question was about.

## 2. Assessment before commitment (#417)

The fight had exactly one shape since 2026-09-06: **an encounter *was* a
bite.** Every assessment model in the contest literature says the opposite —
escalation falls as asymmetry rises, and most encounters never escalate — and
in ants the graded channel is the **assessment**, not the damage (Czaczkes 2024
on *Lasius niger*: aggression flat with relatedness, antennation and jerking
graded; *Myrmecocystus* tournaments, hundreds of ants displaying and almost
none fighting, the border sliding toward whichever colony is outnumbered).
Research: [`animal-conflict-research-2026-09-14.md`](animal-conflict-research-2026-09-14.md).

`src/sim/contest.rs` reads the odds first — the engine's own `(bite/armour)²`
**both ways round**, plus the local numerical asymmetry over the animals
touching this body — through a floored logistic. **A declined encounter is not
a non-event**: it displays, writing 40 into the alarm plane against the 240 a
wound writes, which is the register a border is actually made of and is also
what recruits.

| | contests | fights | displays | escalation |
|---|---|---|---|---|
| `assess=off` | 31.5 | 31.5 | 0 | **1.000** |
| shipped | 43 | 23 | 20.5 | **0.520** |

Four seeds, same binary, one argument apart. **All of contact used to be a
bite; half of it is now a withdrawal**, and the rate holds across a threefold
range of contact, so it is a property of the assessment rather than of how busy
the bed was. With one colony plated (`armour=4`) the plated colony wins:
survivors **15.5/15.5 → 11.5/18.5**. `COMMIT_FLOOR = 0.05` is this change's
compliance with the protection-is-not-an-exemption rule that killed four
support models: **no configuration makes an animal unattackable.**

## 3. Why colonies do not fight: one number, and it is not the one anybody named (#416, lane C)

Three things were proposed as the reason. **Only one is binding.**
`Behavior::scent_spread` defaults to 0, which puts every colony's click at the
species' authored point, **so two colonies are one family** and neither the
mouth nor the fight verb ever sees a stranger. Report:
[`why-colonies-do-not-fight-2026-09-14.md`](why-colonies-do-not-fight-2026-09-14.md).

**And the dial is a threshold, not a slope.** Lane D's §8.3 sizes what lane C
deliberately did not: the acceptance radius is `tolerance + 1` = 1.0 against an
offset drawn from `±spread`, so at `spread = 1` only **9.3% of ordered ant
pairs** read as non-kin and **one seed in four never produces an encounter at
all** — which in a single run reads as "the mechanism does not work". It
saturates by **2**. A dial whose top is 1 would ship a mechanism a third of
beds never show.

**The two lanes' stranger figures differ by denominator, not by fact**, and
neither lane reviewed the other: lane D's 0.093 is over *all* ordered ant pairs
(kin-heavy), lane C's `between%` is over cross-colony pairs only, which is why
it reads 100% or 0% and never in between. Pooled: **7 of 10 seeds separate at
`spread = 1`.** Reconciling two numbers by finding the denominator, rather than
by picking a winner, is the part worth copying.

**§Z23 filed and its repair rewritten mid-round.** `nearest_foe` targets any
living non-kin *organism*, and a plant is an organism, so an armed ant bites
herbs — found by lane D's arena **specificity** arm reporting 710 contests in a
bed with no strangers in it. #417 made the odds count animals only and left the
target rule alone, which **sharpens** the bug rather than closing it: a plant
now fails `is_animal`, so `commits` is unconditionally true and **a plant is
the one target in the world struck with no assessment at all.** Lane C's first
proposed repair — test that the target's species has a `creature` def — is
wrong for the reason #417 gives, that an animal cornered by something it cannot
digest must still be able to hit it. The repair moved upstream: gate
`cry_alarm`'s two feeding call sites on the victim being an animal.

## 4. The colony's books (#419, lane B)

Twelve pages in the lab and not one about food. `World::energy_ledger` already
existed and already balanced; what was missing was that it is **world-wide, has
no face, and has no location**. The FOOD page splits it per colony: food in per
sample (per window, **not** a running total, which can only climb and so cannot
show a colony that has stopped eating), a **diet band** splitting the same
joules by what they came out of with each line in that material's own palette
colour, and per colony FORAGED / MEAT / FED, UPKEEP / WALK / BRAIN, EMPTY /
CARRYING.

**Joules, never cells** — priced by `diet_yield` at the call that credits the
animal, so the page cannot disagree with the eat verb. **Both distribution rows
are distributions on purpose**: a colony does not thrive or starve, it empties
out, and a mean forager hides "a fifth do all of it" exactly as round 33's
pooled idle rate hid "a fifth are frozen". How many colonies fit is **measured,
not authored** — the first version capped by eye and drew the last block under
the bar.

## 5. The food road and the harvest map (#420, lane A)

`F7` cycles OFF / FOOD ROAD / HARVEST MAP / ROAD + HARVEST. The road is **per
cell** and decaying, split into carrying (saturated red-orange) and
empty-handed (cool blue) — per cell against the brief, for an arithmetic
reason: a road is one cell wide and at any coarse tile size it is a blob that
cannot say which way the food went. The harvest map is per 8-cell tile, per
colony, in that colony's hue, weighted by **face value taken** rather than by
bites, because a mouthful of flower is three of leaf.

**The two food numbers disagree on purpose**, and both lanes now say so: lane
B's books price a mouthful by the eater's gut, lane A's map prices what left
the world at that tile. Shading a patch by who happened to eat it would draw
one stand of leaf two brightnesses.

**What it costs, and the settled row is the real price:**

| bed | off | ROAD + HARVEST | delta |
|---|---|---|---|
| running | 3.93 → 5.62 ms | 5.60 → 8.39 ms | **+41% and +49%** |
| **settled** | 1.95 → 3.21 ms | 3.67 → 6.41 ms | **+88% and +100%** |

Both channels decay every tick, so **they defeat the dirty-rect render skip by
construction** — and a settled world is exactly where that skip does its work.
Off by default and free then. Two arms, two runs, because the container moved
between them: **the ratio transfers and the milliseconds do not.**

**The harness made both A/B errors this repo already names, and caught them.**
Alternating draw by draw reported the overlay **1.7 ms faster than having it
off**, because only the first draw of each pair followed a tick — the arms were
not exchangeable. And interleaving the off arm **wiped the map being priced**
down to 2 road cells, because switching the channel off drops it by design: *a
cost that vanished because the work vanished.* It now warms each arm, swaps the
order every round, and **prints the map size it timed**.

## 6. What the round cost in coordination

- **Two branches filed the same bug letter on the same day.**
  `claude/absorb-destroys-plants` filed §Z22 at 04:48, lane C at 05:47; both
  were green and both passed `bugindex.py --check`, which reads one working
  tree and **structurally cannot see a letter taken on an unlanded branch**.
  `--branches` found it in seconds. First filed wins. **The renumber was made
  by the coordinator, not the lane** — two messages to it had gone astray and
  three merges were queued behind it.
- **A new `dead-ends.md` entry needs a `screened.tsv` verdict**, and only
  `docscheck` says so. An entry added after the triage screen ran is absent
  from `candidates.tsv`, and absence there reads as *screened and closed* — so
  an unjudged entry is indistinguishable from a judged-and-dead one. clippy,
  the suite and `bugindex` were all green through it.
- **There is no delivery signal for a poke, at all.** Round 33's remedy — read
  `last_run` on the trigger and `updated_at` on the session — **is also
  wrong**: `last_run` is absent for pokes that did arrive, and `updated_at`
  moves for the lane's own work. The only check that works is the **branch
  head**. Say a message was *sent*, never that a lane was *contacted*, and put
  anything load-bearing in the repo as well as in the poke.
- **The rung-3 zoom question was re-asked in plain words** (#409, card
  `20260914T084458895Z-ae1b01`). He had answered *"keep rung 3 as a soft stop"*
  and then said he did not know what a soft stop was. Rendered and looked at
  before re-posting: **rung 3 does not read as soft, it reads as blocky** — so
  the original question was wrong about the screen as well as unreadable.

## 7. What round 36 inherits

**The owner's #1 — performance at high creature counts — is the lead**, and
round 34 left it a sharp one: there is no knee, and **about half of what an ant
costs is not in the creature pass at all** but in the CA sweep over the 29.4
cells it dirties per frame
([`evolution-lab-knee-2026-09-14.md`](evolution-lab-knee-2026-09-14.md)).

**One thing is unfinished and it is not a lane's fault.** The owner ruled *"you
can ship it on"* and **nothing in this round implements it**: #416 measures
rivalry and changes no default, and lane C's own reading remains *expose it,
default off*. Shipping it on is now a choice of **value** rather than of
boolean — the dial saturates by 2 and a top of 1 shows nothing in a third of
beds — so it wants the constants it reallocates named and re-derived, and a
seed sweep gating an order statistic. **Six seeds is not a sweep.**

**§Z23 is open with a repair already designed** (gate `cry_alarm`'s two feeding
call sites on the victim being an animal), which is a contained first job for
whoever picks up the creature line.
