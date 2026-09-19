# A forest that turns over: the plan for the most impactful plant changes

**Status: plan, proposed, not started. Seven owner calls in §8; nothing here
is authorised until they are answered.** Written 2026-09-19 from the outside
literature review read against the engine
([`plant-literature-review-comparison-2026-09-19.md`](plant-literature-review-comparison-2026-09-19.md))
and its same-day re-audit of seven rejections (that report's §7), at the
owner's request: *"write up a plan for the most impactful changes."*

Every measurement cited is paired over 12 world seeds unless it says
otherwise, and every cost is an estimate in hours of one session's work,
from the auditors where they priced it and from me where they did not.

---

## 0. The one-page version

**What a player sees today.** A wood grows up, closes, and stops. The big
trees are the ones setting the fewest seeds, because a plant at its full
size has nothing left over after paying to keep what it has. Seed piles
under the parent and mostly waits. No gap opens unless the player cuts or
burns one, and when he cuts a tree the stump sits there for ever. Nothing
about the ground remembers what grew on it, so the same species wins the
same spot indefinitely. The two woody species are one organism in two
colours. It is a diorama of a forest, well built, standing still.

**What this plan makes.** A wood that turns over: grown trees are the seed
producers, seed reaches ground it can grow on, gaps open on their own
(frost, roots giving out, age) and from the player's verbs, seedlings win
them, the ground remembers what stood there so the next winner is a
different plant, and a cut tree answers by coppicing. Succession as the
spectacle and disturbance as the player's verb — the sentence
`plant-evolution-design.md` §4 already wrote as the payoff, with the
mechanisms it was missing named and priced.

**Why now.** The re-audit found the single fact that ties it together:
**`income − maintenance` is the only currency, the girth term is 70% of
the bill, and its level was set on bill-to-income with recruitment nowhere
in the calibration.** That one number is the growth ceiling, the empty
seed budget at that ceiling, the reason a per-seed price like
`seed_launch` halves output, and the reason `open-bugs-handoff.md` §P2
records that *no selection claim can be made for trees*. Three of the
seven rejections re-audited were consequences of it. Fix the account and
the rest of the plan has something to work with; fix anything else first
and it is tuning a stand that cannot breed.

**The order, and the dependency that sets it.** Five phases, roughly
10–12 session-days of lane time plus review cards, in this order because
each is the precondition the next was measured to need:

| phase | in the world's words | the measured reason it comes here | cost |
|---|---|---|---|
| **0** | see it before touching it | every readout the plan is judged on already has its numbers printed somewhere and none of them is a curve | 1 day |
| **1** | a grown plant is productive | median established plant's surplus negative 12/12; a 10% floor of income to seed gives 2.18x seed on 12/12 | 3–4 days |
| **2** | seed reaches ground it can grow on | germination is the blocked stage, confirmed three times; recruitment saturates at ~305 whatever the seed supply | 2 days |
| **3** | gaps open and the ground remembers | root turnover +11% income on the shipped bed; nutrient return never built and cannot pump; frost has a writer and no plant reader | 2–3 days |
| **4** | a cut tree answers | tip retirement is permanent; a cut *suppresses* bud break 12/12; a stump cannot resprout, structurally | 1 day |
| **5** | one decisive silhouette test | the crowding channel is live and does not prevent fusion; exclusion is the one property attractors had that it lacks | ½ day |

**The rule every phase runs under.** A switch, default off; a paired
12-seed measurement with an identity control and a positive control; the
constants calibrated against the current behaviour named and re-derived
as part of the work, not after (`CLAUDE.md`, *a change that reallocates a
shared budget*); a review card before the mechanism is called done; the
wiki page in the same change. Three phases (1, 3c, 4) want a blind A/B
card before a line of mechanism, because their outcome is judge-by-eye.

---

## 1. The dependency graph

```
 Phase 0  readouts ─────────────────────────────────────────────┐
                                                                 │
 Phase 1  economy: seed from production; heartwood ─┐            │
                                                    ├─► Phase 2  recruitment: dispersal verb, safe sites
 (herb already has seed to spare — Phase 2 can      │            │
  start on herb while Phase 1 runs on tree)         │            ▼
                                                    └─► Phase 3  turnover & memory: root turnover, nutrient return, frost
                                                                 │
 Phase 4  resprout (independent; wants Phase 1's bill so a       │
          stump has something to spend) ◄────────────────────────┘
 Phase 5  exclusion test (independent; measurement-only, any time)
```

Two arrows are load-bearing. **Phase 2 without Phase 1 is inert for
trees**: the `seed_launch` dead end measured that a tree with 211 seeds in
60,000 frames cannot afford any per-seed price, and that entry stands.
**Phase 3 without Phase 2 opens gaps nothing reaches**: a frost that kills
back a canopy over a seed bank that is all under the parent produces a
scar, not a succession.

---

## 2. Phase 0 — see it before touching it (1 day)

The re-audit's largest finding, a seedless canopy, was visible in numbers
`plant_probe` and `labstats` already print, and nobody saw it because no
readout drew the curve. The review's §12 sanity checks are the forester's
first glance and none of them exists as a plot.

| readout | what it answers | where the numbers already are | positive control |
|---|---|---|---|
| **seeds set against plant size**, per established plant, one point per plant | does size buy offspring (it should, and today it does not) | `plant_probe` seeds-set column (landed in this branch) | `PIXEL_PHYSICS_REPRO_FLOOR=0.10` must tilt it |
| **share of established plants with zero seed** | how much of the stand is sterile | same | same |
| **self-thinning slope** — log mean cells per plant against log plants, over a run | is competition thinning the stand, and how hard | `labstats` plants and cells per tick | a sparse planting must read flat |
| **height against base thickness**, log-log | is the pipe model producing an allometry | `plant_probe` height and thickness | `pipe_ratio` moved must move the slope |
| **safe-site count** — cells where a seed could germinate *right now* (soil, lit above threshold, damp below threshold, unoccupied) | is recruitment site-limited, and by how much | new census over the world; the gate's own predicate | dig a gap and the count must rise |

Shapes, not values: a 2D slice has no reason to hit −3/2, and `CLAUDE.md`
says exactness is not the goal. Read the sign and the rough magnitude, and
read each against its own positive control before believing it about a
change. Also in this phase: **two baseline review cards** of the shipped
stand at 20,000 and 45,000 frames, so every later card has a *before*.

Cost: an afternoon per readout in an example that exists; a day for all
five and the cards. Files: `examples/plant_probe.rs`, `examples/labstats`,
one new census. No engine change, so it can run beside anything.

---

## 3. Phase 1 — a grown plant is productive (3–4 days)

### 3a. Seed comes out of what a plant makes, not out of what is left

**In the world's words.** A full-grown tree is the biggest seed producer
in the wood, and a plant that has stopped growing is spending on seed
instead. Today it is the reverse: the median established plant sets no
seed, the single best plant sets a fifth of the bed's total, and the
typical grown tree is sterile because its surplus is zero by construction
(`plant.rs:9977`, one expression funding growth, seed and bud break).

**Mechanism.** `reproductive_share = max(surplus × allocation, min(income
× floor, stock))`, taken off the growth pool — allocation from production
rather than from the residual, which is what the literature measures
(Greene & Johnson 1994; Hirayama 2004; ~1/8 of production at peak). The
shape is already built and measured as `PIXEL_PHYSICS_REPRO_FLOOR`
(landed default-off in this branch): at 0.10, **seeds set 2.18x on 12/12
seeds, germinations 1.24x on 11/12, reproductive budget 2.5x, no
stand-size cost at 20,000 frames**. Two things make it a design rather
than a switch flip: the draw is notional in both arms (the budget is
credited without debiting donor cells) and a shipped version must make it
physical; and `reproductive_allocation` is authored 0.10–0.30 across eight
species *against a residual*, so changing the base changes what every one
of those numbers means.

**Constants to re-derive, named up front.** `reproductive_allocation` in
all eight species files; `seed_maturity` (rethink §6.9 found the price
cannot yet fire — it may fire now); `REPRODUCTIVE_BUDGET_CAP`. Watch
`max_active_tips`, which the same surplus feeds through `supportable`.

**Demonstration.** Phase 0's seeds-against-size curve tilts positive;
share of sterile established plants falls; the seed bank rises without
the stand shrinking (12 seeds, stand cells within noise); and a **blind
A/B card** of the same bed at 45,000 frames, because *more seedlings,
slightly smaller trees* is a trade only the eye settles. Cost **6–10 h**
including the re-derivation.

### 3b. Heartwood: the bill falls when the crown does

**In the world's words.** A tree that has lost part of its crown stops
paying for the crown it used to have. Today it never does: maintenance is
charged on `q_peak`, a monotone high-water mark of foliage ever supported,
and the code's own doc names it *the ratchet that eventually kills an
adult*. The re-audit's biology reading is that the *superlinearity* is
right — respiration scales with living sapwood, which under the pipe model
is foliage × height (Ryan 1990, 1995) — and the *monotonicity* is the
respiration hypothesis for age-related decline that the field rejected on
measurement (Ryan, Binkley & Fownes 1992; Tang et al. 2014).

**Mechanism.** Let `q_peak` relax toward `q_now` at a species-scale rate —
sapwood converting to heartwood — so the girth term follows the crown down
with a lag rather than never. One constant, one line, and then
`MAINTENANCE_PER_NODE` re-derived against the changed basis, **against
recruitment this time** (a ×0.5 / ×1 / ×2 sweep at 12 seeds reading
established and inherited-genome establishments), because the re-audit
measured that the level was set on bill-to-income and the quantity it most
controls was never in the calibration.

**Constants.** `MAINTENANCE_PER_NODE`; the relaxation rate (per species,
or one engine constant first); `INCOME_PER_NODE` and `RESPROUT_DEFICIT_
FLOOR` are both calibrated against the bill and must be checked, not
assumed. `MAINTENANCE_PER_CELL` stays: flat-at-equal-bill lost a fifth of
the seed on 12/12 and that verdict stands.

**Demonstration.** Median established plant's surplus non-negative on most
seeds; a tree that has lost a limb regains a seed budget within N ticks;
inherited-genome establishments for `tree` **above zero** — §P2's zero is
the bar this phase exists to move. Cost **5–7 h** plus the **3–4 h**
sweep. Do 3a first: it is cheaper, already measured, and decisive; 3b
changes the bill 3a draws on.

**What this phase does not do.** It does not touch how a plant grows,
branches or looks. If the card says the stand reads worse, the numbers do
not overrule it; the ethos does.

---

## 4. Phase 2 — seed reaches ground it can grow on (2 days)

**The wall, measured three times.** `plant-equilibrium-costs` §13b drove
seed supply six-fold and germinations moved 24–25 → 25–29: *germination is
the blocked stage and seed supply is very nearly irrelevant to it*. The
re-audit's floor at 1.0 made ~8x the seed and germinations pinned at
**304 / 304 / 305 across three different worlds**. `plant-reseeding-
2026-09-03.md` measured the funnel on the lab's herb: the gate can only
open on two materials, seed piles under the parent in its own shade,
the colony eats the bank, and dispersal is worth about 1.4x. So Phase 1's
seed becomes a bigger bank and nothing else until this phase.

### 4a. The wind carries seed, and most of it lands close and a little of it goes far

**In the world's words.** A gust takes seed off a plant and a few of them
go a long way, so a stand spreads by throwing the occasional outlier into
a distant gap instead of widening as a block. That is the graded outcome
the ethos asks for, and it is what every field kernel looks like
(leptokurtic: 2Dt, log-normal). The shipped `seed_launch` draws its
distance **uniform** on `[−reach, reach]` (`plant.rs:3591`) and every
species authors 0.

**Mechanism.** The wind-dispersal verb on gusts, designed and priced in
rethink §6.5 (its prerequisite measured, unbuilt only because the session
ended), with the distance drawn from a fat-tailed kernel rather than a
uniform one — one line — and the walk still stopping at the first
obstruction so nothing teleports through a wall. `launch_price`'s
square-root shape stays; the re-audit confirmed a tree can afford it once
it has seed to spend, and herb already can.

**Constants.** The kernel's scale per species (or one engine constant
first); `seed_launch` in the species that should throw (herb, scrambler,
grass; then tree once Phase 1 lands). `wiki/plants.md`'s "a few go a long
way" becomes true and its line gets rewritten.

**Demonstration.** Columns occupied and plants established well away from
anything anyone planted (the reseeding report's own metrics), 12 seeds,
herb first; a card of the bed after a windy spell. Cost **1 day** for the
verb, since the design exists.

### 4b. Know whether recruitment is site-limited, then decide the gate

**Mechanism.** Phase 0's safe-site census against the ~305 germinations
recruitment pins at. If the count matches, recruitment is site-limited and
that is *correct*: safe sites are the ecology's own fence, and the lever is
Phase 3's gaps and 4a's reach, not the gate. If the count is far above
305, something else fences germination — the seed-on-seed-pile case, the
shade under the parent, the two-material rule — and that is a one-day
diagnosis with the census as the instrument. **Do not lower the
germination thresholds to buy recruits**: `plant-water-scarcity` measured
that every species' `soil_water_threshold` already sits above the
availability at which its own uptake limits, so the gate is not slack.

Cost **½–1 day**, measurement-led.

---

## 5. Phase 3 — gaps open, and the ground remembers (2–3 days)

`plant-evolution-design.md` §5c: *evolution needs generations, and a closed
canopy has none* — and it recommends the emergent route to turnover over
an authored lifespan. Three mechanisms, each with a measured or
never-measured record behind it, and none is a calendar.

### 5a. Roots give out where the soil is spent (3–4 h)

**In the world's words.** A root that touches nothing dies back and the
plant puts its growth where the water is; a root system rotates instead of
thickening in place.

**Record and re-test.** Rejected as *worse at every rate* on a 6-row bed
that carried the collapsed root gate in every arm — the sweep trap. On the
shipped 96-row bed with the shipped gate, `PIXEL_PHYSICS_ROOT_TURNOVER
=0.002` gives **income 1.107 (9/12 up), uptake 1.14 (9/12), contact 1.03
(9/12)** — three quantities moving together — and no rate is worse; the
positive control at 0.5 culls exactly the roots that touch nothing.

**Mechanism.** The switch exists (`plant.rs` `ROOT_TURNOVER_PER_TICK`,
inert at 0.0). Landing it is a decision on the rate: 24 seeds at 0.002
to turn 9/12 into a number worth trusting, plus 0.005 and 0.01 to find
the knee, and a **root-overlay card** — roots are invisible on a contact
sheet, so the stand is the wrong picture to post.

**Constants.** The rate; `MAX_ROOT_FRACTION` (0.5) is the cap the rotation
lives under and should be re-read once roots stop accumulating.

### 5b. The ground remembers what grew on it (2 h build, 1 h runs, then a decision)

**In the world's words.** Where a plant stood and died, the soil it drew
down gets back what it took as the plant rots — so ground has a history,
poor ground favours one kind of plant and its litter makes ground a
different kind wins next. That is succession, and `plant-equilibrium-costs`
§10e names it as the one thing carbon and water *structurally cannot
produce*: neither can record what grew somewhere.

**Record.** Cited as *a documented double dead end* and never built: both
cited entries are about **moisture**, `decay.rs` does not contain the word
nutrient, no return writer has ever existed, and the pump the moisture
versions measured is not expressible on a deficit-bounded store (a
conservative return cannot create nutrient, and at full soil both
consumers read 1.0, so its whole biomass range is the ablation's 0.843 →
1.0). What it buys is **soil memory**, not biomass, so biomass is the
wrong readout.

**Mechanism.** `OrganismState::nutrient_drawn`, a debit captured from the
return value `plant.rs:975` discards, repaid per cell at `rot_remains` into
the soil under the rotting cell through a `Chunk::return_nutrient` that
mirrors `draw_nutrient`; behind `PIXEL_PHYSICS_NUTRIENT_RETURN`, default
off. ~80–100 lines.

**Demonstration.** Spatial variance of `soil_nutrient_fraction` across the
bed, and a seedling's establishment on previously-occupied ground against
fresh ground, 12 seeds, two horizons; positive control a freely crediting
switch that must pin the deficit at zero everywhere; negative control
return off. Then the §10e trigger, read honestly: does the same species
still win the same ground indefinitely? If yes with the return on, the
loop is closed and succession needs the *other* half — different species
preferring different richness — which is a species-authoring pass, not an
engine change.

**What this is not.** It is not the litter-pile fix. `soil-accumulation-
and-the-carbon-cycle.md` §4 ranks bioturbation first for *that* problem,
and this plan leaves it there. The return is a ledger, not a cell.

### 5c. Frost kills back what is exposed (3 h, then a card)

**In the world's words.** A hard cold spell scorches foliage — a fraction
of it per tick, worst on exposed crowns, so a frost leaves a wood
browned rather than dead, opens gaps at the edges and on the ridges, and
the seedlings that follow are the story. And it is a verb: the held
world's spell list already reads *call frost to kill back what is winning*
(`weather.rs` near :1674).

**Record.** The season was declined and the re-audit strengthened that:
at the shipped eight-minute day a tree matures in one app-day and has a
two-day life half-life, so no year fits inside it. But **temperature is a
different question**: `field::noon_equivalent_temperature` exists,
de-oscillates exactly, already has consumers in creatures and
evaporation, `weather.rs` writes cell temperature and ships `Pin::Frost`
and `Pin::Blizzard` — and `plant.rs` reads temperature **zero times**. A
writer with no plant reader.

**Mechanism.** A graded shed pressure on foliage below a
`noon_equivalent_temperature` threshold, driven by the weather channel
and never a calendar, behind `PIXEL_PHYSICS_FROST`; the same shape as
`shade_death` and `drought_death` (a rate, cubed, not a threshold on a
sampled value — the oscillator rule). Exposure can reuse `weather::
exposure`, which bending already reads. Additive mortality: it reallocates
no budget. The one registry trap is a `DEATH_CAUSE_LIST` row enrolling in
every census that enumerates it — grep first.

**Constants.** The threshold and the rate, per species later (frost
tolerance is the first thing that would make `tree` and `conifer` differ
*functionally*; today they share every economic number and differ only in
`leaf_cluster` and `seed_maturity`). One engine pair first.

**Demonstration.** Under a pinned `Frost`, foliage lost is graded over
ticks and by exposure rather than a shelf swept off (12 seeds, the
distribution of per-plant loss, not its mean); under `Clear` nothing
moves; a **card** of a wood the morning after, because *browned, not dead*
is the whole point and only the eye can say it.

---

## 6. Phase 4 — a cut tree answers (1 day)

**In the world's words.** Top a tree and the stump throws shoots; cut a
limb and the buds below it wake. Coppicing is what real trees do and it
is the graded aftermath the felling verb is missing — `wiki/plants.md`
already says *a topped tree sits there indefinitely rather than
resprouting*, and the ethos says a verb whose consequence stops at a stump
is not finished.

**Record.** The dead end condemned permanent tip retirement (*meristem
senescence is not real*) and named its replacement, reversible dormancy;
the replacement was built for buds that were never tips. `break_buds`
selects only `DormantBud`, a retired tip becomes `MatureBody`
(`plant.rs:6559`), and no path leads back — the acceptance test as named
passes on a plant whose retired tips are untouched. Run anyway: a
mid-crown cut **suppresses** bud break, median 0.864, fewer on 12/12,
because `supportable = ⌊(noon_income − maintenance)/step_cost⌋` falls with
the foliage; a stump has no foliage, so `supportable` is 0 and it never
resprouts, structurally. The owner's 2026-09-12 blind A/B of
`PIXEL_PHYSICS_RESPROUT` came back *looks identical* — on mid-crown cuts,
never on stumps.

**Mechanism, in two steps.**
1. **Cheapest first (20 min of compute):** re-run the blind A/B of the
   existing `PIXEL_PHYSICS_RESPROUT` **on stumps**, the case it was never
   judged on and the one where a plant has rootstock carbon and no way to
   spend it. If that reads as coppicing, the rest of this phase is a
   default change.
2. **The real path (4–6 h):** a route from `MatureBody` back to
   `DormantBud` on a live stem, applied only to *staleness* retirement
   and never to starvation, so a plant that ran out of room can wake what
   it retired when room returns. Named risk: species files gate
   `StructuralAnchor` and `SecondaryThicken` on `MatureBody`, so this
   reallocates which cells thicken and `pipe_ratio` must be re-derived
   against the changed set.

**Constants.** `RESPROUT_DEFICIT_FLOOR` (set from a six-seed control the
12-seed control crosses — re-derive at 12); `ORGANISM_STALE_LIMIT`;
`pipe_ratio` per species if step 2 lands. `buds_flushed` is a world-wide
counter and reading it per plant is a confident wrong answer — the
instrument for this phase reads the cut plant's own shoot cells.

**Demonstration.** A stump puts up shoots on most seeds within a stated
budget of ticks; a mid-crown cut no longer *suppresses* bud break, or
suppresses it gradedly; a card of a topped tree a few thousand frames on.
Wants Phase 1's bill, so a stump has a surplus to spend, but does not
block on it.

---

## 7. Phase 5 — one decisive silhouette test (½ day)

**The question.** Crown fusion — the *one green mass* complaint that has
outlived three architectural levers — was attributed to the loss of space
colonisation's attractors, which were removed on philosophy and never
A/B'd. The re-audit ran the zero arm the record lacked: with crowding
weight 0 against the shipped 30, the thickest fused run is **median 0.964
and larger on only 5/12** — the channel is live and does not prevent the
fusion it exists to prevent.

**Mechanism.** Attractors differ from crowding in exactly one property,
**exclusion**: a consumed attractor cannot be entered twice, while two
tips read the same density and both may enter. Make the existing channel
exclusive within one tick — deposit at the *chosen* candidate before the
next tip scores, inside the candidate loop at `plant.rs:5347–5366` — no
new state, no attractor list. ~2 h, read on the same paired harness with
stand mass held or normalised (the re-audit's confound), plus a card.

**Why it is worth half a day either way.** If the fused run moves, it is
the cheapest silhouette win in months and the attractor question reopens
on evidence. If it does not, fusion is set by something neither mechanism
touches — which is the conclusion `plant-appearance-design.md` §2.1
reached for the three levers — and the attractor question closes for good.
Measurement-only until it moves; it can run in a worktree beside any
other phase.

---

## 8. Owner calls — numbered, and what each one unblocks

1. **Reproduction priced from production, not from the residual (3a).**
   Yes re-derives eight species' allocation numbers and makes the draw
   physical; no leaves grown trees sterile and Phases 2, 4 and the
   evolution line's tree claims where they are. *The plan's keystone.*
2. **Heartwood: let the bill follow the crown down (3b), with the level
   re-derived against recruitment.** Yes commits to the sweep; no keeps the
   ratchet the code's own doc calls the thing that kills an adult.
3. **Root turnover on by default at the knee (5a).** A rate decision after
   24 seeds and a root-overlay card.
4. **Nutrient return as the succession mechanism (5b), measured on soil
   memory.** Yes is 3 h to a measured answer; the decision after it is
   whether species should differ in what richness they want.
5. **Frost as the first thing plants read from temperature (5c), from the
   weather channel and never a calendar.** Yes gives the held world its
   spell and the outdoor world an emergent gap-maker; it also decides that
   `tree` and `conifer` will diverge on frost tolerance rather than on a
   deciduous/evergreen calendar.
6. **Resprout (Phase 4): the stump A/B first, then the `MatureBody →
   DormantBud` path if it reads.** No leaves the felling verb ending in a
   stump.
7. **The exclusion test (Phase 5).** Half a day, measurement-only, closes
   or reopens the attractor question.

And one that is not a call but a warning the plan runs under: **`plant.rs`
is one file and every phase but 0 and 5c edits it.** `plant-
implementation-split-2026-08-23.md` §1 measured that two sessions in it
concurrently recreate the 2026-08-22 merge fallout. So Phases 1 → 2 → 3a
→ 3b → 4 are **one serialised lane**; Phase 0 (examples only), 5c
(organism tick and weather) and Phase 5 (a worktree measurement) can run
beside it. Two lanes, not five.

---

## 9. What this plan deliberately does not do

- **No calendar year.** Declined and strengthened; a tree is an annual on
  the world's own clock. Frost comes from weather.
- **No FvCB photosynthesis, stomatal conductance model, or hormone
  biochemistry.** `design-philosophy.md` §3; light-use efficiency is the
  right model for real time and it is what ships.
- **No rebuilt attractor list.** Phase 5 tests the one property that
  mattered; a private per-organism attractor list stays out.
- **No flat maintenance respiration.** Re-tested at equal bill and it
  loses a fifth of the seed on 12/12.
- **No lowering of germination thresholds to buy recruits.** The gate is
  measured tight, not slack; Phases 2 and 3 buy recruits by reach and gaps.
- **No multi-pool decomposition, trait databases, ODD, GPU or ML
  surrogates.** The comparison report's §4.
- **Not the litter pile.** A different problem with its own ranked menu
  (bioturbation first); 5b is a ledger and does not add or remove a cell.
- **Not hydraulic failure for §V2 or the PPA for M10.** Both are named
  with a home in the comparison report (§5.2, §5.3) and wait on those
  items being picked up.

---

## 10. How to know it worked, as a whole

Not a bar — the ethos says only play settles it — but the readouts Phase
0 builds should, after Phases 1–3, say five things they cannot say today,
paired over 12 seeds against the baseline cards:

1. seeds set rises with plant size, and the share of sterile established
   plants falls;
2. inherited-genome establishments for `tree` are above zero;
3. plants establish well away from anything planted, and the far tail is
   occasionally very far;
4. after a pinned frost, the stand loses a graded fraction of foliage and
   the next generation's establishment concentrates in the gaps;
5. the spatial variance of soil richness is non-zero and rising, and a
   seedling on old ground does measurably differently from one on fresh.

And one card, the same bed at 45,000 frames before and after, blind. If
the owner picks the *before*, the numbers do not win the argument.

*Freshness: written 2026-09-19 against the state of this branch after its
merge of `main`; every anchor is at that commit. Costs are estimates.
Nothing in this file is authorised; §8 is the ask.*
