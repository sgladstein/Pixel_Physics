# The colony's economy — why the box starves in a full larder, and the four ways out

*Design report, 2026-09-09. Written after §Z6's diagnosis
([`colony-starvation-separated-2026-09-08.md`](colony-starvation-separated-2026-09-08.md))
separated "cannot reach" from "overgrazes". This asks the next question —
**where does the energy actually go** — and prices the options against it.*

**Status: design of record for the creature line's energy economy. Nothing
here is built.** One of the four options is already an owner-vetoed dead end
and is included because **the price of that veto has now been measured for the
first time**, which is a fact the owner should have even though the ruling
stands.

---

## 0. The answers, stated once

| question | answer |
|---|---|
| Do the ants fail to *find* food? | **No.** They walk 58–71% of the bed's columns and climb to 22–23 rows. Food at the nest does not save them |
| Do they fail to *eat*? | **No.** They eat throughout, and eat *more* as they die — on each other |
| Then why do they starve? | **Foraging returns less than it costs.** Intake is 16% of burn; the founding grant covers the other 84% until it runs out |
| Why do they all die together? | Every founder is stamped with the identical `start_energy: 200`, so every founder reaches zero in the same 500-frame window |
| Is it a pricing problem? | **Mostly no.** The prices survive scrutiny; see §3 |
| Is it a mechanics problem? | **Yes — two absences**, and both are things real ants do: they do not share food, and they never rest |
| Is there a third way? | **Three.** Castes, litter, and founding the colony the way nature does |
| Can scouting-and-recruitment emerge rather than be coded? | **Yes, and two of its three parts are already built.** The trail is wired and castes shipped; what is missing is the ability to *do nothing*, which is authored against |

**The one-line diagnosis: the shipped ant is a generalist in a world made of
plants, it walks constantly, and it eats alone.**

---

## 1. The failure, measured

`labforage`, default lab bed, `RAYON_NUM_THREADS=1`, seed 1. Ants alive
against edible cells standing in the whole bed:

| frame | 3,000 | 3,500 | **4,000** | 4,500 | 6,000 |
|---|---|---|---|---|---|
| ants | 52 | 50 | **14** | 6 | 5 |
| edible cells | 213 | 253 | **341** | 341 | 462 |

**46 of 52 die inside one 500-frame window, at the moment the larder is
fuller than it has ever been and still filling.** The same shape appears on
seeds 2 and 3.

**The cliff is the founding grant, and the arithmetic predicts it rather than
fitting it.** `start_energy` is 200 J; measured burn is 0.0524 J/frame per
ant; 200 / 0.0524 = **3,817 frames**. Observed mass death: between 3,500 and
4,000. Every founder is placed at frame 0 with the same 200 J and burns it at
the same rate, so they empty together — a cohort, not a population.

**Two candidate explanations were tested and both failed.**

- **Not vertical reach.** An earlier reading of "highest ant head 11 rows"
  suggested food was above the ants. That figure is a *snapshot at the last
  sample*, taken when four ants were alive. The run maximum is **22–23 rows**
  on all three seeds, above the 16-row `aloft` band. Ants climb;
  `MaterialKind::Plant` is a foothold.
- **Not distance to food, either.** `handout=200` drops a food cell at the
  colony's own column every 200 frames. Same seed, same brain: the colony
  eats **70% more** (343 meals by frame 4,000 against 201) and **still
  collapses**, 52 → 21. Food at the doorstep does not buy past the cliff.

---

## 2. Where the energy goes

`labstats`, 5,000 frames, default bed, seed 1:

```
burn 10,796 J  =  metabolized 4,604 (43%)  +  moved 5,599 (52%)  +  synapse 593 (5%)
intake  1,710 J
```

**Locomotion is 52% of everything the colony spends, and more than three
times its entire food intake.** The colony recovers **16%** of what it burns.
Fifty-two grants of 200 J total 10,400 J, which is almost exactly the 9,086 J
deficit — the colony runs on start-up capital for its whole life.

Per-ant, the gap has a clean shape. An ant burns **0.25 J/tick** active
against an authored `upkeep` of 0.100/tick, because `move_cost_per_cell`
(0.125) is **2.5x** `idle_cost_per_cell` (0.05) and the shipped brain moves on
about two ticks in three (`(Bias, Move, 2.0)` → `squash(2.0) ≈ 0.667`). One
120 J mouthful buys **480 ticks** of that. An ant therefore needs one cell per
~2,880 frames and gets one per ~18,000 — **short by about 6x.**

`stamp_probe` prices the ideal case, and the ideal is the important number:

```
best mouthful standing in this world 120 J | upkeep 0.100/tick
best net +0.725/tick -- a child every 1434 ticks of uninterrupted feeding
```

**1,434 ticks is 8,600 frames — which is exactly the generation time §4f
quotes.** So the generation clock and the foraging economy are not two
problems. *A generation takes 8,600 frames because that is how long
uninterrupted feeding takes to pay for one child*, and no ant feeds
uninterrupted.

---

## 3. Option A — pricing. Mostly no.

Each price was checked against what it is modelling.

| price | value | verdict |
|---|---|---|
| `move_cost_per_cell` | 0.125 (2.5x idle) | **Defensible.** Walking insects run 2–10x resting metabolism. If anything ours is cheap. The problem is not the price of a step, it is that the ant never stops taking them |
| `digest` | 3.30 J/tick, ~36 ticks a cell | **Not binding.** Digestion overhead is 5% of intake, measured. A cell converts slowly but the ant is not waiting on its stomach, it is waiting on its legs |
| `start_energy` | 200 J | **Tuning, and it would hide the fault.** Raising it moves the cliff later without changing its shape: every founder still holds the same number, so they still empty together |
| `food_energy` | 480 J | Inflates both sides; changes nothing about the ratio |

**No price here is obviously wrong, and the house rule cuts against touching
them anyway**: a term in a weighted sum is not an independent knob, and this
economy has been re-derived once already at real cost
(`why-changes-cost-so-much-2026-08-27.md`). The one number that *is* mismatched
is not a price at all — it is a starting gene, and §6 is about it.

---

## 4. Option B — mechanics. Two absences, both of them things real ants do.

### 4a. Trophallaxis — the colony does not share food

**Fifty-two ants in this box are fifty-two separate economies that never
exchange a joule.** `grep trophallaxis src/` returns nothing.

In a real colony this is backwards. An ant's crop is called the **social
stomach**: a forager fills it, walks home, and regurgitates to nestmates,
who pass it on again. Individual ants do not feed themselves — the colony
feeds itself, and a forager may hand on most of what she carries. It is the
central logistical fact of ant life.

**What it fixes here, precisely.** The synchronised cliff exists because no
ant can be carried by a luckier sister. Every ant must independently solve
foraging or die, and they all fail on the same schedule. Pooling converts
that into one draining reserve: the colony thins as income falls short
instead of falling off a shelf. **It turns a binary into a distribution**,
which is the ethos's first law, and it does it without changing a single
price.

It is also cheap and *visible* — two kin adjacent, an energy transfer, ants
meeting head to head. The engine already has the pieces: `is_living_kin` for
who counts, and `Crop` for what is carried.

**Not in `dead-ends.md`** — grepped for `trophallaxis`, `food sharing`,
`share food`: zero hits. Nearest neighbour is a **`TRAIT_STORE_IN_BODY`
granary-versus-replete gene, specced and not built** on 2026-08-31 as
redundant against another lever; that is storage, not transfer, and the entry
does not reach this.

### 4b. Rest — the ant has no idle state, and it is authored that way

*Owner, 2026-09-09, from play: "They tend to just dig out the world when they
have nothing else to do."* That observation has an exact cause, and it is not
emergent.

**Both of the ant's main verbs run on unconditional biases:**

| authored weight | with no other input | means |
|---|---|---|
| `(Bias, Move, 2.0)` | `squash(2.0) = 0.667` | moves on **67%** of ticks, always |
| `(Bias, Dig, 0.4)` | `squash(0.4) = 0.286` | digs on **29%** of ticks, always |

There is no *"when they have nothing else to do"* — there is no state in which
an ant has nothing to do. It walks and it digs at a fixed rate for its whole
life, and `creature.rs`'s own comment concedes the second one is "an
*unconditional* drive", added because ants never dug otherwise. That is a
constant compensating for a different fault, which `CLAUDE.md` names as its
own recurring shape.

Measured, the two together are the colony's budget: **moved 5,599 J (52% of
burn), dig 670 J (6.2%)** — against a total food intake of 1,710 J.

Real colonies are the opposite. A large fraction of workers are inactive at
any moment, and that inactivity is adaptive: reserve labour, and energy
conservation. An ant that sits still burns 0.10/tick against 0.25 active —
**more than doubling its own reserve for nothing**.

**And rest is the precondition for the pattern the owner actually wants.**

> *Owner: "It would be more interesting if the colony sends out a few scouts,
> then they find food, leave a trail and the rest of the colony follows. Not
> that I want to specifically hard code that, but it would be interesting."*

That is the real ant pattern, and **it does not need to be authored — it needs
the three things it is made of to be available at once.** Two already are:

1. **Following a trail.** Already built and already wired. `ant.ron`'s hidden
   layer carries `(PheroAAlong, …)` and `(PheroBAlong, …)` into units driving
   `Move`, gated on `Carrying`: home scent when laden, food scent when empty.
2. **Division of labour.** Already built. The plasticity dial and heritable
   developmental weights let a parent hand each child a number that becomes a
   different body and a different behaviour — shipped on, at the owner's
   instruction.
3. **Doing nothing.** **Missing.** Not merely unselected — *authored against*,
   by the two unconditional biases above.

So scouting-and-recruitment is not a mechanism to build. It is what a lineage
should *find*, once staying home is cheaper than wandering and the trail is
worth following. Today a stay-at-home ant is impossible to be, so the
strategy has no foothold to start from, and the colony that would have
discovered it is dead by frame 4,000.

**The change this argues for is not "make ants rest". It is to make rest
reachable** — put the two drives on something an ant can be *about* (hunger,
crowding, the trail) rather than on `Bias`, so that a weight of zero means a
resting ant instead of a broken one, and let selection do the rest. That is
the difference between hardcoding the behaviour and hardcoding its
*possibility*.

**This is the cheapest experiment on the list** and it needs no new mechanism:
run the arena with the biases moved off `Bias`, and read the cliff and the
dig count.

---

## 5. Option C — the three out-of-the-box routes

### 5a. Let the colony specialise internally, instead of choosing one gut

**This is the recommendation, and §6 is why.** The generalist gut is what
starves the colony *and* what makes the ant-versus-beetle predation the owner
called the most fun they had had in the game. Choosing either pole loses
something real.

Real colonies do not choose. They divide the work — harvesters, soldiers,
nurses — so the *colony* is an omnivore while its *members* are specialists.
**The machinery for this already shipped**: the plasticity dial and heritable
developmental weights from the castes work, on by default at the owner's
instruction. A line that puts plant-gut foragers alongside flesh-gut
defenders is expressible today and nothing has asked it to.

### 5b. Let the plants feed the floor

`shed_to_litter` exists and fires from plant death and rot. In a real
woodland the **litter layer** is where the ants are: they do not climb every
tree, they harvest what falls. Continuous shedding — not only on death —
would put food at ant level near each plant's base, turning "walk to the far
half of the bed" into "walk to the nearest tree", and it would make the two
kingdoms interact, which is the point of the lab.

Cheapest form: measure first. Census the floor band over a long run and see
how much litter arrives without any change at all.

### 5c. Found the colony the way nature does — one queen, not 52 workers

The deepest of the three. A real colony is founded by **one** mated queen who
seals herself in a chamber and metabolises her own wing muscles to raise the
first brood. She does not forage. The first workers emerge into a colony of a
handful, whose needs are correspondingly tiny, and the colony's size tracks
its income from then on.

**The synchronised cliff cannot happen in nature**, because a real colony's
members are born at different times with different reserves. It happens here
only because the box drops 52 strangers with identical wallets into bare soil
and starts the clock. `grep queen src/` returns nothing.

This is a lifecycle and scenario change rather than an afternoon, and it is
the one that would make the box's opening state *mean* something.

---

## 6. The gut, and the verdict that already exists

**Measured 2026-09-09, `stamp_probe`, one world seed (2583), founders' gut
gene the only difference:**

| founder gut | mouthful | net while feeding | ticks per child | births | live | deepest generation |
|---|---|---|---|---|---|---|
| **0.0 (shipped)** | 120 J | +0.725/tick | 1,434 | 15 | 18 | **4** |
| **−1.0 (plant specialist)** | 480 J | +3.200/tick | 325 | **2,076** | **262** | **19** |

**Do not read this as a proposal. It is already a dead end, and the owner
gave the ruling.** `dead-ends.md` carries `gut_bias: -1.0` as the shipped
ancestral value, *"built, measured and reverted the same day, 2026-08-30 — it
works, and it reverses a verdict the owner has already given"*. The verdict is
on review card `20260823T104411499Z-963f8d`: **"An omnivore should be
viable."** A `-1.0` gut rates flesh at zero, so `adjacent_food` stops seeing
carrion entirely and `a_starved_nestmates_corpse_is_still_dinner` goes red.
Carrion returns at about −0.68, and **no position that keeps carrion also
breeds on two cells**. The gene would also sit on the end of its own axis, so
drift could only move it inward and half of every child's draw would clamp
back onto the wall.

**What is new is the price of the ruling, and it has moved a long way.** The
2026-08-30 measurement put a `-1.0` gut at generation 2 over 60,000 frames,
against a bank ceiling of 580 and a bar of 961 — *short by 381*. Today the
same gene reaches **generation 19** with a richest bank of **1,916 against a
bar of 1,040** — it clears. The economy has been re-derived underneath that
dead end (the satiety roof and everything after it), so **its numbers are
stale even though its reasoning is not**. `CLAUDE.md` asks for exactly this:
re-test a do-not-retry entry once something changes its condition.

**So the honest statement to put in front of the owner is not "specialise the
gut". It is: omnivory costs about fifteen generations of depth in this bed,
and it did not cost that when you ruled on it.** Whether that changes the
ruling is the owner's call and nobody else's — and §5a is the route that
keeps the ruling *and* recovers most of the value, because a colony of
specialists is still an omnivore.

**Caveat on the numbers, stated because it matters**: this pair was measured
on `stamp_probe`'s **world** terrain, not the `LabBox`. The comparison is
paired and both arms share a bed, so the *ratio* stands; the absolute figures
are not the lab bed's. No bed harness takes a `gut=` argument today, which is
the first gap §8 lists.

---

## 7. Recommendation, and the order

1. **Rest (§4b) — first, and the owner has asked for it.** No new mechanism:
   move `Move` and `Dig` off their unconditional biases so that *doing
   nothing* is a state an ant can be in, then read the cliff and the dig
   count. It is also the precondition for scouting-and-recruitment emerging
   rather than being authored, which is the shape the owner wants.
2. **Trophallaxis (§4a) — the main build.** Highest ratio of realism to
   effort, it is the direct cure for the synchronised cliff, it makes the
   colony a colony rather than 52 solitary insects, and it is visible on
   screen.
3. **Expose the gut (§6), do not tune it.** Put the measured price of omnivory
   on the page, and let castes (§5a) carry the specialisation.
4. **Litter (§5b) — measure before building.** Census the floor band first.
5. **The queen (§5c) — the long answer**, and the one that makes the box's
   opening state honest.

**What none of these should be judged by is a test.** Every one of them
changes what the box looks like over a session, and that is a review-card
question.

---

## 8. What is not measured

- **No bed harness takes a `gut=` argument**, so §6's ratio has never been run
  on the `LabBox`. That is the first thing to close.
- **The handout taper is inconclusive.** Four rates × 3 seeds at 100,000
  frames gave no dose-response, but §Z6's own result lives at 300,000 — the
  taper was read at the wrong horizon, which is a gap and not a refutation.
- **Whether hauling home works at all.** `d<16` reads **0 at every sample of
  every run**: no edible cell is ever within 16 columns of the nest, while
  `(AtNest, Drop, 1.0889)` says ants should be dropping cargo there. Either
  they rarely make it home, or dropped food does not stay edible. Unexplained,
  and cheap to check.
- **Rest has never been raced.** The claim that burn falls toward upkeep is
  arithmetic, not measurement.
- **§Z6 stays OPEN.** Nothing here closes it; this report is about the
  economy underneath it.
