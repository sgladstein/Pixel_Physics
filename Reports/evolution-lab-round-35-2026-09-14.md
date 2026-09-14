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

**AND THE BOOKS IMMEDIATELY PRODUCED THE HEADLINE THEY WERE BUILT FOR: THE
COLONIES DO NOT FEED THEMSELVES.** Shipped bed, seed 1, two colonies of 8
founders, 20,000 frames. One colony took in **10,400 J of founding grant and
foraged 399 J**; both together foraged **485 J against 18,800 J granted —
2.6%**. They ate **84% and 95% corpse**. Walking is 27% of everything spent,
brains 8.3%, upkeep 46–48%.

**That is the mechanism under a standing lab observation, stated in joules for
the first time**: 70–90% of ant deaths in this bed are starvation, and the
reason is not that a colony fails to find its dead — **plant income is
approximately zero**. Every colony in the lab has been living on the founding
grant and then on its own corpses. It is the strongest argument yet that the
bed, not the animal, is what wants attention, and it was invisible until the
ledger had a per-colony face.

**One scene fact cost a measurement on the way**, and it is the
*scene-contradicts-the-code* trap again: the two colonies were founded **30
cells apart, not 120**. At 120 they never meet, every share stays inside one
colony, and **the cross-colony transfer the whole split was designed around is
untested while its equality assertion passes perfectly**. The guard bed now
founds at 85 and 115 and asserts the crossing happens.

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
- **A guard written with a *hungry* eater starved its own eater**, so the null
  read as *"strangers do not eat each other"*. Feeding is an urge, so the first
  version made the eater hungry — at a quarter bank it walked off looking for
  food and was dead inside the window, and the probe found no eater at all. The
  hunger wire makes a **full** ant rest, so two rich strangers stay adjacent
  long enough for the mouth to find flesh already touching it: **163 eats and
  54 cells taken, against zero**. The scene, not the mechanism, was the bug —
  and the obvious scene was the wrong one.
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

**§Z23 is open with a repair already designed** (gate `cry_alarm`'s two feeding
call sites on the victim being an animal — *not* a test that the target has a
`creature` def, which #417 argues against and correctly), a contained first job
for whoever picks up the creature line.

**What is left of the ship-it-on ruling is the economy, not the switch.** Lane
C shipped it: `assets/species/ant.ron` authors `scent_spread: 2.0` (#423). The
constants that default reallocates — the birth bar, `colony_ants`, and the
starvation balance, all calibrated on a bed where **no ant is food** — are
named in the commit and deliberately **not** re-derived inside a one-line asset
change. That is the correct boundary and it is round 36's job.

## 8. Shipping it on, and the instrument bug that nearly set the wrong value

*Added after the round's record was first written, when lane C landed the
switch (#423).*

**The value is 2.0, gated on an order statistic** because the per-seed outcome
is binary and a median hides the beds where the switch does nothing. 12 seeds ×
24,000 frames, counting seeds with a cross-colony killing:

| `scent_spread` | seeds with a kill | kills | median founding gap |
|---|---|---|---|
| 0 | **0 of 12** | 0 | — |
| 1 | 9 of 12 | 86 | 1.62 |
| **2** | **11 of 12** | 105 | 2.28 |

**What the dial decides is how many seeds clear the recognition radius at
founding, and nothing else** — `is_living_kin` is a *boolean*, so 1 → 2 leaves
the kill count identical on 10 of 12 seeds and the whole gain is the two seeds
that cross. **It saturates because the signature is clamped**: `apply_colony_
scent` clamps each slot to `[-1, 1]`, so past ~1 the dial folds draws onto the
corners of that cube instead of pushing colonies apart. One seed is
byte-identical to the unswitched bed at *every* value — **one bed in twelve
cannot be separated by any setting. Structural, not tuning.**

**The cost, paired off(0) against shipped(2) over the same 12 seeds:** deaths
**+99 median**, up on 11 of 12 and down on none; starvation share **−4.3 points
median**, down on 10 of 12 — killing displaces starving; population and births
**both unmoved, medians exactly 0**. The bed carries it.

**And the table above is the second one, because the first was measured through
a broken instrument.** `rivalry.rs`'s `spread=` override *added* its offset to
whatever scent an animal already carried. While the default was 0 that was
identical to re-deriving, so every measurement taken before the switch went
live is sound — but the moment `ant.ron` authored a live value, every arm was
measuring `authored + requested`, and **`spread=0` was not an off arm at all**:
it left the authored offset standing while claiming to have removed it. The
confounded sweep said *"1.0 and 2.0 are equivalent and 2.0 has the worse
tail"*, and the branch briefly shipped **1.0** on that reading.

**The control that caught it is the one this repo demands of every other knob**
— run the bed from the *authored* value and from the *runtime override* at one
seed and require them byte-identical. They disagreed on the founding gap alone
(1.817 against 2.289) **while every outcome column matched** — matched only
because both sat past a threshold, so the doubled offset changed no decision
and would have gone on changing none until some arm sat near the boundary.
**A confound that is invisible in every column you are looking at is still
there.**

**A second finding from the same work, and it is a rule rather than a number:
the founding draw is not stable across engine changes.** One seed's founding
gap at an unchanged `spread=1` moved **0.907 → 2.170** across the round-35
merge. The offsets are a pure hash of `(world seed, colony label, slot)`, so
what moved was *which labels get claimed*, upstream of this field entirely.
**Tune this dial on the threshold argument, which is structural; never on a
table of particular seeds' gaps, which is not.**

**And the echo earned its place immediately.** The harness prints
`scent_spread as authored` every run, and it caught the lane editing the
comment to 2.0 while leaving the value at 1.0. *A default nobody can see the
value of is a default nobody can tell has moved.*

**The first thing the live default actually broke was a harness, and the
diagnosis was half right.** CI went red on `cargo run --release --example
ascii` — the one gate the lane's local set was missing, and exactly where a
behavioural default change was always going to land. **Three scenes place their
animals in a `world.plant_ant(..)` loop**, and `plant_ant` routes through
`Origin::Founder { colony: None }`, **which claims a fresh label per call**. So
each scene held **55–60 one-ant colonies that only looked like a colony**:
inert while every label smelled identical, mutual strangers the moment
`ant.ron` authored a live value. `examples/ascii.rs` **had already ruled on
this once**, in its own moisture scene — *"One colony, not fifty-five … they
stopped foraging and started eating each other"* — so the fix is that precedent
applied to the three loops that never got it.

| scene | before | after |
|---|---|---|
| excavation (the red one) | digs 62, roofed void **0** | digs 354, roofed void 42 |
| foraging | 12 of 15 alive at 12k frames, 703 deliveries | 15 of 15, 849 |
| double bridge | 1,076 deliveries | **13** |

**Two of those do not say what they first look like.** In the excavation scene
`deaths` barely moved — **52 → 55**. It was not that more ants died; they spent
the run *fighting instead of digging*, so what moved is the digging. And the
bridge's collapse is **the old number being the artifact**: sixty one-ant
colonies each satisfied *"a laden animal reaches its colony's nest"* trivially,
where one colony of sixty has to make the trip. `deaths` is 59 either way. **13
is a real and rather low round-trip count over that bridge** — a finding about
the bridge, stated out loud rather than quietly installed as a new baseline.

**And the reading this is not, which the coordinator proposed and the lane
refuted.** The coordinator's relay suggested this might mean *a player
sprinkling ants one at a time now gets a massacre*, and said it would be a
finding for the owner if true. **It is not true**: `plant_ant` is not reachable
from the game — the sandbox's ant key goes through `World::found_colony`, which
claims one label for the whole colony, and the only non-test caller of
`plant_ant` sits inside a `#[test]`. **A harness placement artifact, not a
player-facing consequence of the default.** Checked because it was asked for,
and reported as negative.

**Two things it left for the owner**, neither a lane's to decide: at a live
dial, **jarring an ant and releasing it beside its own nest makes it a stranger
its nestmates will eat** — that follows from the documented release rule and
was invisible while the dial was 0. And **only the common ant is on**; the held
world's other five foundable stocks still found as one family with themselves.
