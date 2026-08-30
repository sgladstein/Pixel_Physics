# Three ways past the body stamp, priced

> **CORRECTED 2026-08-30, after this landed. §5's recommendation — sow a
> fruiting plant — has been measured and it does not work, for the reason
> §5 itself names as its failure case.**
> `evolution-lab-gate-1-2026-08-30.md` §4.3 fitted the matched gut in a box
> that *does* flower and fruit: the margin goes from **−820 to +500** and
> the colony still produces **zero births over 48,000 frames**, because
> **every flower and attached fruit stands 22 to 40 rows up a stem**, and
> `windfall` — the ground form, the only one an ant walking the soil can
> reach — **never exceeds 1 cell** in a 90,000-frame run. So this is a
> *reach* problem, not an economy one, exactly as §5 said it would be if it
> failed. **The margin model in §4.1 still holds; what it does not contain
> is whether the best mouthful can be got at**, and §1.1's census counts a
> flower up a stem as standing food. Read §5 with that correction: the fruit
> route is not dead, but it is blocked behind fruit *falling*, which is a
> plant question. The fallback in §5's step 2 is untouched.

*Lane I of the creature program, 2026-08-30. A decision document: the three
routes named in `creature-reproduction-economics.md` §§3.1, 3.2 and 3.5, each
with a number against it, and a recommendation.*

**Read §1 and §5 if you read nothing else.** §1 is the correction that
changes which routes are still live; §5 is what to build.

---

## 0. The finding

An ant cannot afford a child because a birth has to *buy the child's body* —
`body_energy` for each of its two cells, **960** — against a bank that stops
filling at **100**. Three ways past that were named and none was priced.
Priced, four things come out, and the first is the one that reorders the
choice.

1. **Neither stamp route removes the stamp; both defer it** — route 1 leaves
   the newborn a second cell to buy, route 2 leaves the parent one to buy
   back — and the deferred instalment is **480 against a bank that caps at
   220**. At the shipped diet it is unaffordable for ever. **So route 3 is
   not an alternative to routes 1 and 2. It is the precondition for both**,
   because the specialised gut is the only thing in the current design that
   lifts the ceiling (220 → 580) past that 480.
2. **No single route breeds.** Measured over 12 pre-registered seeds at
   24,000 frames: route 3 alone **0 births**, route 1 alone **0 births**, the
   shipped ant **0 births** — and all three report `denied-no-space` of
   **zero**, so those are energy results and not space ones.
3. **Routes 1 and 3 together do breed**, and the colony grows rather than
   merely persisting. That is the only combination in this report that both
   works and is measurable.
4. **Route 3's escape hatch does not exist in this world, by construction.**
   §3.5 rests on a matched gut collecting a 960-point `fruit` or a
   1,440-point `flower`. **No fruit or flower cell stands in any sampled
   world at any frame** — and the reason is not the horizon: worldgen sows
   `creeper`, `shrub`, `conifer` and `tree`, and the only two species that
   bear fruit at all, `herb` and `scrambler`, **are never planted**. That
   makes "sow a fruiting plant" the cheapest lead in this document — and it
   is the recommendation.

**What decides a birth is one number: `ceiling − bar`.** Every arm with a
negative margin gave **exactly zero births over 12 seeds**; every arm with a
positive one bred. Two arms sharing no mechanism but the same +99 margin bred
at the same rate (96 and 92). So a proposal can be priced before it is built.

**The recommendation is to sow a fruiting plant first** — no creature code at
all, and the arm that models it breeds on **12 of 12 seeds**. Route 1 with
route 3's gut is the fallback if the fruit turns out to be out of foraging
range; route 2 stays last, because this report could not price it. §5 has the
ordering and what each step costs.

---

## 1. The correction: the routes were priced against a budget that no longer ships

An ant's bank has a hard ceiling. It stops eating above
`hunger_fraction * start_energy` and carries the mouthful home instead, so
the most it can ever hold is **the satiety line plus one mouthful**:

```
ceiling  =  hunger_fraction * start_energy  +  Y
bar      =  max(reproduce_threshold, g + body_energy * cells + 1)
```

where `g` is what the newborn is *given* (`birth_grant`) and `Y` is the best
mouthful this gut can digest. That model is not new — Lane A verified it
against measurement, predicting 165/195/220/270/570 for a measured
`richest bank` of 164/175/219/260/567 — and this report uses it only to
place the routes, never as a verdict. §3 measures.

**What has changed is one operand.** `creature-reproduction-economics.md`
§3 writes the satiety line as **450**, i.e. `0.5 * 900`. Between that report
and this one, E14 shipped and cut `start_energy` to **200**. `ant.ron`
today:

| | |
|---|---|
| `start_energy` | **200** (was 900) |
| `hunger_fraction` | 0.5 → satiety line **100** (was 450) |
| `body_energy` x 2 cells | **960**, unchanged |
| `birth_grant` trait −0.2 → fraction 0.4 | g = **80** (was 360) |
| `reproduce_threshold` | 1100 |
| `birth_cost` | 80 + 960 = 1040, floored bar **1100** |

**Every route's headroom fell by 350 and its stamp did not move.** That is
the whole of this correction, and it is enough to reverse two verdicts.

### 1.1 `Y` is not what the material table says it is

The other operand is `Y`, and it has a trap in it that this lane's harness
exists to remove. `creature_probe` prices the best mouthful over **the whole
material table**, which contains a `flower` worth 1,440 and a `fruit` worth
960. Its own comment says a bound read that way "can rule out and can never
rule in". Route 3 is exactly the case where that matters, because a matched
herbivore gut collects a flower's whole 1,440 and would clear every bar in
this document on it.

**Measured, censusing the cells actually standing in the world** (18 seeds,
before and after 24,000 frames, `examples/stamp_probe.rs`):

| gut | best mouthful on the whole table | best mouthful **standing in this world** |
|---|---|---|
| 0.0 (shipped) | 360 | **120** — a leaf, at quarter value |
| −1.0 (matched herbivore) | 1,440 | **480** — a leaf, at full value |

**No `fruit`, `flower` or `windfall` cell exists in any of these worlds, at
frame 600 or at frame 24,000.** The standing food is `leaf`, `moss`,
`litter` and ant flesh, every one of them face value 480. So the route-3
escape hatch — reproduction riding a fruit crop, which §3.5 calls "already
in the engine with nothing to build" — **is not in the world the ants are
standing in**, and the 1,440 that makes route 3 look comfortable is a number
about the material registry rather than about this game.

This is the difference between a ceiling of 1,540 and a ceiling of 580 for
the same animal, and it is the single number that decides route 3.

### 1.2 The arithmetic, all six arms

Satiety line 100 unless stated. "Short by" is `bar − ceiling`; a negative
number is headroom.

| arm | what it models | stamp | g | bar | Y | ceiling | short by |
|---|---|---|---|---|---|---|---|
| **S** | the shipped ant, untouched | 960 | 80 | 1,100 | 120 | 220 | **+880** |
| **R3** | route 3 alone, best case (`gut=−1`, `g=0`) | 960 | 0 | 961 | 480 | 580 | **+381** |
| **R3b** | route 3 at the *pre-E14* budget (`start_energy=900`) | 960 | 0 | 961 | 480 | 930 | **+31** |
| **R1** | route 1's stamp (born at one cell → 480), `g=0` | 480 | 0 | 481 | 120 | 220 | **+261** |
| **R13** | routes 1 and 3 together, at the *shipped* grant | 480 | 80 | 561 | 480 | 580 | **−19** |
| **R2** | route 2's stamp (moved, not bought → 0), shipped grant | 0 | 80 | 81 | 120 | 220 | **−139** |

Three things to read off it before any measurement:

- **R3b is the brief's own number** — "clears the 961 bar to within 31" — and
  it is right *at a budget of 900*. At the shipped 200 the same route is
  short by 381, twelve times as far.
- **R1 was a route and E14 closed it.** §3.1's table gives route 1 a budget
  of "≤ 90" for `g` at the shipped gut; that is `570 − 480` at the old
  satiety line. At 100 the budget is **−260**: there is no grant, including
  nothing at all, that makes a one-cell newborn affordable to a neutral gut.
- **R13 clears by 19 out of 580 — three percent.** That is a knife-edge, and
  the ceiling model is known to be beatable: Lane A measured a bank of 616
  against a printed 540. R13 is therefore the one arm where the arithmetic
  genuinely cannot call it and the measurement has to.

---

## 2. What was measured, and what the proxies cannot see

**The two stamp routes need mechanism in `src/sim/creature.rs`, which this
lane does not own.** They are priced by proxy: `body_energy=` reproduces the
birth arithmetic each route implies, in-process, with no source change and no
asset edit. What each proxy does and does not cover, stated rather than
assumed:

- **Route 1's proxy is near-exact on the books.** The route halves the
  *cells* stamped at birth (2 → 1) at `body_energy` 480; the proxy halves
  `body_energy` (480 → 240) at 2 cells. The product — what a birth takes out
  of the world, 480 — is identical, and so is the total worth of the
  resulting animal's flesh. What it cannot see is the thing the route is
  really about: **a one-cell newborn's survival in the window before it grows
  its second cell**. The proxy's animals are born full-size. That question
  needs the mechanism and is named in §5 as the first thing to measure after
  building it.
- **Route 2's proxy is conservative.** Setting `body_energy=0` gives the
  stamp the route gives it, but the harness moves `ant` and `corpse`
  `food_energy` with it — deliberately, to hold the flesh-pricing invariant
  that closes the corpse pump (dead-ends §13l) rather than open a pump it
  would then measure. Real fission moves a cell that is still worth 480. So
  **the proxy deletes carrion from the menu**, which makes route 2 look
  *worse* than it is. A route that closes despite it, closes. It also cannot
  see what fission does to the *parent* — §5 names that too.
- **Route 3 needs no proxy at all.** `gut=` sets the diet gene directly and
  the harness reads it back off a live founder, so a run cannot silently
  measure the neutral gut. This is the route that needs no new mechanism, and
  it is measured as itself.

**The seeds are pre-registered on a criterion independent of every route.**
Placement is wildly seed-dependent — the harness reports how many of its 55
founders actually got onto the ground, and it ranges from **2** (seed 1) to
**51** (seed 21). A world with no ants in it reports `births 0` exactly like
an ant that cannot afford one. All 30 seeds were screened at `frames=1`,
before any arm ran, and the **18 that seat at least 30 founders** were taken:
2, 6, 8, 9, 10, 11, 12, 13, 14, 16, 18, 20, 21, 22, 23, 25, 26, 27. Six seeds
is not a sweep, and eighteen is what the house rule asks for.

**The positive control fires.** Lane A's gate control —
`start_energy=200 body_energy=20 threshold=241 hunger=0.9 terrain=world
frames=24000`, at seed 2711 — gives **births 8,982, live 3,283, generation
102, 19 lineages**. A null there would have voided everything below. It is a
control and not a proposal: `hunger=0.9` is in dead-end territory, and a
route that only worked there would not have worked.

---

## 3. The wall both stamp routes hit one step later

This is the finding that reframes the choice, and it is arithmetic rather
than measurement, so it is stated before the results.

**Neither stamp route removes the stamp. Both defer it.**

- Route 1 births a one-cell child for `g + 480`. That child is not an ant
  yet. To become one it must **buy its second cell, at `body_energy` = 480**,
  out of its own bank.
- Route 2 moves a cell from parent to child, so the birth is nearly free —
  and leaves **the parent one cell short**. To be a two-cell ant again it
  must buy that cell back, at 480, out of its own bank.

In both cases the deferred instalment is **480 against a bank that stops
filling at 220** at the shipped gut. The stamp is not avoided; it is split
into two payments, and at the shipped diet **the second payment is
unaffordable for ever**.

| gut | bank ceiling | can it pay the deferred 480? |
|---|---|---|
| 0.0 (shipped) | 220 | **no, not ever** |
| −1.0 (matched herbivore) | 580 | yes, with 100 to spare |

So route 1 at the shipped gut does not produce ants that start small and grow
up. It produces **a species of permanent juveniles** — one-cell animals that
can never afford their second cell. Route 2 at the shipped gut produces the
same thing from the other end: a colony that halves itself once, cannot
regrow, and — since §3.2's own stopping rule is that a one-cell animal has no
cell to give — **stops reproducing at exactly twice the founder count**.

**Which makes route 3 not an alternative to routes 1 and 2 but the
precondition for both.** The specialised gut is the only thing in the current
design that lifts the bank ceiling over 480, and without it neither stamp
route survives its own second instalment. That is why the arm that works
below is the one that combines them.

### 3.1 And the growth verb does not exist

`creature-reproduction-economics.md` §3.1 costs route 1 as "`birth_cost` must
become a function of the *born* size rather than `def.body.len()`; a growth
step must charge at the moment it appends". **There is no growth step.**
Nothing in `src/sim/creature.rs` ever appends a body cell to a live organism:
the only `Grow` in the file is a comment about plants, and the only
`regrowing` is about a tree. A creature is placed at its full body plan by
`place_creature` and never changes size again.

So both stamp routes need **a new verb for creatures — growth — that the
engine does not have**, on top of the birth-cost change. That is not a
line-item in either §3.1 or §3.2, and it is the larger half of both. Route 3
needs no new mechanism at all, which was always its stated advantage; what
this report adds is that route 3 cannot be *used* on its own.

---

## 4. Measured

12 pre-registered seeds, 24,000 frames, `terrain=world`, one binary.

| arm | seeds that bred | births (median / max) | ants alive at the end (median) | seeds with any ant left | bank / bar (median) | `denied-no-space` (median) |
|---|---|---|---|---|---|---|
| **S** — the shipped ant, untouched | **0/12** | 0 / 0 | 15 | 12/12 | 0.200 | 0 |
| **R3** — route 3 alone — `gut=-1`, `g=0` | **0/12** | 0 / 0 | 12 | 10/12 | 0.571 | 0 |
| **R3b** — route 3 at the pre-E14 budget (`start_energy=900`) | **0/12** | 0 / 0 | 22 | 12/12 | 0.869 | 0 |
| **R3c** — route 3 with the headroom a fruit would give | **12/12** | 96 / 440 | 9 | 10/12 | 1.014 | 662 |
| **R1** — route 1's stamp alone (480), `g=0` | **0/12** | 0 / 0 | 14 | 10/12 | 0.444 | 0 |
| **R13** — routes 1+3 together, shipped grant | **11/12** | 22 / 79 | 22 | 10/12 | 0.958 | 31 |
| **R13b** — routes 1+3, `g=0` — margin 99, not 19 | **11/12** | 92 / 685 | 14 | 10/12 | 1.016 | 1318 |
| **R2** — route 2's stamp, no anti-freeloading margin | **12/12** | 946 / 2430 | 0 | 3/12 | 0.000 | 21458 |
| **R2b** — route 2's stamp, `threshold=200` | **11/12** | 1354 / 13614 | 228 | 7/12 | 6.021 | 44219 |

**"bank / bar" is the median over seeds of the richest ant's bank divided by
the bar it had to clear.** It is the continuous quantity behind the binary
`births` column, and it is the one to read when a route fails: it says *how
far* short, not merely that it fell short.

Four things to take off that table.

**The zeros are energy, not room.** Every arm that failed to breed reports
`denied-no-space` of **0** — not a small number, zero. `births_denied_no_space`
counts a birth that was attempted and refused for want of a cell to put the
child in, so a zero means **no ant in twelve worlds ever reached its
threshold at all**. Lane A's grant sweep ran that counter at 159k–563k and
read the column; this one reads it too, and it says the opposite thing. The
arms that *do* breed prove the counter is live in this harness: R2 runs it to
27,429.

**The negatives cannot be blind, because the same binary produces the
positives.** R3 and R13b differ in exactly one knob — `body_energy`, 480
against 240 — and nothing else: same gut, same grant, same threshold rule,
same seeds. One breeds and one does not. That is the positive control the
house rule asks for, and it is inside the experiment rather than beside it:
the stamp is isolated as the binding term by a single-variable comparison
rather than by argument.

**Route 3 is not close, and it is not close at either budget.** Alone at the
shipped budget it banks a median **0.571** of its bar. At the pre-E14 budget
the earlier arithmetic says it should reach 0.968 and it measures **0.869** —
so the ceiling model is optimistic here, and even the version of route 3 that
looked like a near miss on paper is short by more than the paper says. Twelve
seeds, no births at either.

**Route 2's proxy breeds and the colony dies.** 11 of 11 seeds produce
births — a median of 826, up to 2,430 — and only 3 of 11 have a single ant
left standing at the end. That is not fission failing; it is the arm being
mis-specified, and the mis-specification is instructive. With
`reproduce_threshold` at its floor the bar is 81 against a grant of 80, so a
newborn needs **one** unit of energy to breed again. Economics §1.3's
condition (a) — the threshold must exceed what a newborn is *given* — is
violated by one point, and what comes out is a birth-death treadmill with
`denied-no-space` at 27,429: a space-limited explosion, then nothing. **R2b
is the same route with a real margin** (threshold 200 against a grant of 80,
so an ant must earn a whole leaf above its endowment before spending it). It
breeds on 11 of 12 seeds and holds a median of **228 ants** against the
shipped 15 — and `denied-no-space` runs to **44,219**, so that scene is
space-limited and its births column says nothing about energy.


### 4.1 The margin governs the outcome, not the mechanism

Put each arm's **margin** — `ceiling − bar`, how much headroom the arithmetic
gives it — beside what it did:

| arm | margin | seeds that bred | births (median) |
|---|---|---|---|
| S | −880 | 0/12 | 0 |
| R3 | −381 | 0/12 | 0 |
| R1 | −261 | 0/12 | 0 |
| R3b | −31 | 0/12 | 0 |
| R13 | **+19** | 11/12 | 22 |
| R13b | **+99** | 11/12 | 92 |
| R3c | **+99** | **12/12** | 96 |

**Every negative margin gives exactly zero births and every positive one
breeds, with no exceptions and no near misses in between** — including R3b at
−31, which is the closest any failing arm comes and still never once fires.
And the two arms at **+99 land on top of each other**: 92 and 96 births,
`bank / bar` of 1.016 and 1.014, from routes that share no mechanism at all —
R13b halves the stamp, R3c raises the budget. Their per-seed spreads are wide
and different (R13b 0–685, R3c 44–440), so this is agreement in aggregate
rather than a suspiciously clean coincidence.

**So what decides whether an ant can breed is one number**, and neither the
route nor the gene it goes through changes the answer. That is worth more
than any single route's verdict: it means a proposal can be *priced before it
is built*, by computing `hunger_fraction * start_energy + Y − birth_cost` and
asking whether it is positive.

**Two arms sit outside that model and must not be read against it.** R2 and
R2b set `body_energy=0`, which does not merely zero the stamp — it changes
what a body is *worth*, and a corpse's worth is `(body_energy * cells +
leftover) / cells`, so with the stamp term gone a corpse is made entirely of
the dead ant's unspent bank. R2b's richest ant banks **6.0x its bar**, over
five times the ceiling the model prints for it. Lane A already recorded that
this ceiling is not a hard bound (616 measured against 540 printed, a 14%
overshoot); **5.5x is a different order of thing and this report does not
explain it.** It is flagged rather than smoothed over, and it is the second
reason — after the free regrowth in §2 — that **no number in the R2 rows is
evidence about fission.**


### 4.2 And whichever route wins, the player cannot see it

Rendered at the end of the run, seed 8, the two arms are a colony that
**bred 79 times and grew from 38 ants to 106** and one that never managed a
birth. Cropped to a 128x80 band on the nest and upscaled six times — the
tightest this harness can frame — **the two pictures are, to this author,
indistinguishable, and no ant is findable in either.**

That is not the crop missing the colony, and the number is what says so:
counted inside the rendered window, the breeding arm has **186 creature
cells in shot** and the shipped one **66**. They are there. They cannot be
seen. An ant is one or two cells of dark on dark soil, which is
`creature-appearance-design.md`'s standing finding — extent is the only
lever, and route 1 makes newborns *smaller*.

**This matters more than which route wins.** The ethos says an event that
produces no visible consequence is not finished regardless of what the
simulation believes. A reproduction economy whose entire visible output is a
population count that nobody can count is exactly that, and it is a cost
every one of these routes carries equally — so it does not change the
ranking in §5, but it does mean **none of them is done when the births
column moves.** Posted to the owner's review queue as
`20260830T062042271Z-8bcaa0` (board `creatures`) rather than settled here,
because "can you see a difference" is not a question this lane can answer
for him.

---

## 5. What to build

**Cheapest first, and the cheapest is not one of the three routes.**

### Step 1 — sow a plant that fruits. No creature code at all.

> **Measured since, and it fails here: see the correction at the head of
> this report.** The step is right that it needs no creature code and wrong
> that sowing is enough — a flower 22 to 40 rows up a stem is standing food
> this report's own census counts and an ant on the ground cannot reach.
> What it actually needs is fruit *falling*, which is `Behavior::Ripen`'s
> fruit-to-windfall path and a plant lane's work, not a creature one.

Route 3's own escape hatch is a `fruit` worth 960 and a `flower` worth 1,440.
Both exist, both are already grown by `plant.rs`, and **the world never
contains one**, because `LIFE_SPECIES` in `src/worldgen/passes.rs` sows
`creeper`, `shrub`, `conifer` and `tree` — and the only two species that bear
fruit, `herb` and `scrambler`, are not in that list. So the reason no ant has
ever reached a flower is not that ants forage badly. It is that there are
none.

Give a matched gut a 960-point fruit and its ceiling is `100 + 960 = 1060`
against a 961 bar — **margin +99**, which is exactly the margin R3c measures,
and R3c breeds on **12 of 12 seeds**, the only arm in this report that never
fails, at `denied-no-space` of 662 rather than tens of thousands. Route 3
then works *alone*, which is what it always promised and could not deliver on
an empty larder.

This is also the version the owner's own objection permits. The refusal on
record is to **a richer uniform floor** — *"I don't want ants sitting in one
spot eating fallen leaves"* — and a fruit crop is the opposite of uniform: it
ripens, it falls as `windfall` to where the ants walk, and it is gone.
Reproduction riding a fruit crop is masting, and it is the ecologically real
version of this mechanic.

**What is not established, and must be measured before this is called done:**
R3c delivers its margin by raising `start_energy`, not by putting a fruit in
the world. The two give the same *ceiling*; they do not give the same
*problem*, because a fruit has to be **found**. Ants are on record as poor at
finding food far away. So the experiment is: add `herb` to `LIFE_SPECIES`,
census fruit cells with `stamp_probe` (it already prints standing food per
material), and re-run R3. **If fruit cells stand in the world and R3 still
reads 0 births, the fruit is out of foraging range and this step is dead** —
which is a foraging problem, not an economy one, and a different lane's.

### Step 2, if step 1 fails — route 1, and route 3 with it, as one change

Not route 1 *or* route 3: **both, together**. Route 1 alone is 0/12 and route
3 alone is 0/12; together they are 11/12. The reason is §3 — route 1 leaves
the newborn a second cell to buy at 480, and only the specialised gut lifts
the bank ceiling (220 → 580) past that.

Build it at **`birth_grant` near zero** rather than at the shipped 80. R13
(grant 80, margin 19) breeds a median of 22; R13b (grant 0, margin 99) breeds
**92**, four times as many, because 19 points of headroom is less than one
mutation step of `trait_variance` (0.15 on the gut axis) and half the
children fall back out of solvency at birth. The trade is real and visible in
the data — R13b's colonies lose more ants (deaths 123 against 38) and hold a
smaller standing population (14 against 22) — which is the offspring
number-versus-quality trade-off arriving on its own, and it is a **graded**
outcome rather than a binary, which is what the ethos asks for.

Its costs, none of which §3.1 lists:

- **A growth verb for creatures, which does not exist.** Nothing in
  `creature.rs` appends a body cell to a live organism. This is the larger
  half of the work.
- **The ancestral gut has to be moved by hand** in `ant.ron`, from 0.0 toward
  −1.0. Selection will not find it: the positive control ends at a mean gut of
  −0.66 from a neutral ancestor, so the pressure is real, but at gut 0 nothing
  breeds and there is no differential reproduction to select through.
- **A one-cell newborn is at the bottom of the findable range**
  (`creature-appearance-design.md`), so the species will read as a scatter of
  dots until its members grow.

### Step 3, last — route 2

Not because fission is wrong. Because **this report could not price it**, and
says so: the `body_energy=0` proxy makes the parent's regrowth free, which is
precisely the constraint §3 identifies as binding, and it moves the corpse
economy so far that R2b's ants bank 6x their bar against a ceiling the model
cannot account for. Everything route 2 needs — the growth verb, the
specialised gut — route 1 needs too, and §3.2's own recommendation is to
build it second. That ordering survives this pricing unchanged.

### What not to do

- **Do not tune `reproduce_threshold` downward.** `reproduce_at` floors it at
  `birth_cost + 1`, so the edit does nothing and reads exactly like the change
  having been made.
- **Do not lower `body_energy`.** It is pinned to the flesh-pricing invariant
  against `ant`, `chitin_*` and `corpse`, and breaking it re-opens the corpse
  pump (dead-ends §13l).
- **Do not raise `leaf`, `litter` or `moss`.** That is the uniform richer
  floor the owner refused, and it is a different thing from a fruit crop.
- **Do not read the material table's ceiling as reachable.** It quotes a
  flower that no world contains.
- **Do not expect `start_energy` to help.** Cutting it lowers the ceiling by
  `0.5 dS` and the bar by only `grant_fraction * dS`; raising it is what R3c
  does, and R3c is a *stand-in* for a fruit rather than a proposal to double
  the budget.

---

## 6. What this does not measure, named rather than assumed

Four things, and the first two are the ones a reader should not let this
report quietly stand in for.

- **A one-cell newborn's survival in the window before it grows.** The brief
  asks it directly and the proxy cannot answer it: R1's and R13's animals are
  born full-size. What §3 establishes instead is that at the shipped gut
  there *is* no such window, because the second cell is never affordable.
  Once route 3's gut and a growth verb are both in, this is the first thing
  to measure, and `deaths` against `live` is the readout.
- **What fission does to the parent.** The `body_energy=0` proxy prices the
  birth and makes the regrowth free, which is precisely the constraint §3
  identifies as binding. So **R2 and R2b measure route 2's easy half only.**
  No number in this report is evidence about a fissioned parent, and the one
  place the report reasons about it (§3) is arithmetic.
- **Whether selection would find the specialised gut on its own.** The
  positive control ends at a mean gut of **−0.66** after 102 generations from
  a neutral ancestor, so selection clearly does push that way once
  reproduction works. It cannot bootstrap: at gut 0 nothing breeds, so there
  is no differential reproduction to select through. The ancestral value has
  to be moved by hand to start it. That is a one-line change to `ant.ron`'s
  `traits` and this lane does not own the file.
- **Whether the best mouthful can be *reached*.** §1.1 corrects
  `creature_probe` by pricing the food standing in this world rather than the
  whole material table — and "standing in the world" is still not "reachable
  by an ant on the ground". `stamp_probe` counts a flower 22 to 40 rows up a
  stem exactly as it counts a leaf on the floor. That gap is what
  `evolution-lab-gate-1-2026-08-30.md` §4.3 walked into, and it is the
  instrument's known limit rather than a surprise.
- **Frame cost of any of this.** Not measured. A breeding population is not a
  55-ant colony, and Lane A's figure (+1.3 ms for 1,781 ants) is the number
  to re-take rather than to carry forward.

---

## 7. Provenance

Everything here is `examples/stamp_probe.rs` at 24,000 frames on
`terrain=world`. **PR #142 was open, and conflicted against `main`, for the
whole of the measurement** — its `mergeable_state` was `dirty` and
`origin/main` carried no `birth_grant` at all — so the sweep was run on
`origin/main` with `origin/claude/creature-lane-a-birth-grant` merged in,
which is the tree the brief intends. #142 landed while this report was being
written, and `main` has since been merged in; **no number here was re-taken
against it**, and nothing in those commits touches `ant.ron` or the birth
path. The lane note records the original state.

### 7.1 Re-checked on the tree that landed under it

`main` moved twice while this was being written, and the second move was
**PR #154, which re-prices metabolism per body cell** — `idle_cost: 0.10`
and `move_cost: 0.25` became `idle_cost_per_cell: 0.05` and
`move_cost_per_cell: 0.125`. For the two-cell body every arm here uses, that
is the same number (`0.05 x 2 = 0.10`), so the cost path should be
untouched — but a diff is not evidence, and a measurement taken on a tree
nobody else has does not transfer.

So the two decisive arms were re-run on the merged tree, same seeds:

| arm, seed | before the merge | after |
|---|---|---|
| shipped, 8 | 0 births, 38 alive, bank/bar 0.211 | **0 births, 38 alive, 0.205** |
| shipped, 10 | 0 births, 27 alive, 0.208 | **0 births, 27 alive, 0.203** |
| routes 1+3, 8 | 79 births, 106 alive, 0.974 | **71 births, 100 alive, 0.973** |
| routes 1+3, 10 | 59 births, 71 alive, 0.982 | **61 births, 83 alive, 0.979** |

**Not bit-identical**, so something in those commits does reach these runs —
which is why this was checked rather than argued. But the movement (79 -> 71,
59 -> 61) sits well inside the per-seed spread the sweep already reports for
this arm (0 to 79 across twelve seeds), the shipped arm is unchanged to the
ant, and every verdict in §4 holds. The full sweep was **not** re-taken.

Seeds are the 18 of the first 30 that seat at least 30 of their 55 founders,
screened at `frames=1` before any arm was run. Arms differ from the shipped
species only in the knobs named in their row, applied in-process through
`set_creature` rather than by editing `ant.ron` — assets are `include_str!`ed
and a sweep that edits one and re-runs a prebuilt binary produces
bit-identical runs.

The binary was rebuilt with `cargo build --release --example stamp_probe`
under `set -o pipefail` before the sweep, and not rebuilt during it.

