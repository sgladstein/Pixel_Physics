# Giving the ant a way home — the plan

*2026-09-20. Owner-directed, after reading the decision path end to end and
checking it against the survey. Supersedes nothing; it is the next step after
[`pheromone-trail-direction-2026-09-16.md`](pheromone-trail-direction-2026-09-16.md)
§7.46–§7.49, and it is the first document on this line to propose changing how
an ant **steers** rather than what it **smells**.*

---

## Context — why this, and why now

The forage loop is discovery → return → recruitment → granary. **The return
arm is the one that does not work**, and two consecutive repairs to the ant's
*sense* did not move it:

- **§7.46** found the homing plane was being erased faster than an ant could
  walk it and shipped `TRAIL_A_RHO = 0`. Round trips 0.88% → 14.84%, sign
  23/6/7. Real, and one in seven is still a failure.
- **§7.47** found six of eight headings sampled open sky or solid rock and
  reported a confident **−0.909** where the honest answer is "no information",
  and shipped a readability test. Freeze runs halved, more ants reached food,
  colonies survived — **round trips did not move**.
- **§7.49** then paired those figures within seed and found the sensor-level
  win was a colony-size artefact. What survived is sharper than what it
  replaced: **the one arm that demonstrably changes what the ant reads (the
  row projection: down-gradient share +18.3 points, 32/4; silence −18.5, 2/34)
  is the arm that makes its homing measurably worse** (up-gradient homeward
  yield 13/23 and 8/28).

If the reading were what set homeward drift, that could not happen. So the
question stopped being *"is the signal good enough"* and became *"what is the
signal connected to."*

**Reading the decision path answers it.** An ant makes two separate decisions
per tick, and the trail reaches only one of them.

---

## 1. What an ant here actually decides, per tick

Verified by reading `creature.rs`, `brain.rs` and `assets/species/ant.ron`
rather than from memory. An ant decides once per **6 frames**
(`ant.ron:28`), scaled per individual by the heritable `TRAIT_PACE` gene and
by leg-cell fraction.

### 1a. Whether to step — `P(move)`

`creature.rs:4484`, `let p_move = outputs[Move].clamp(0.0, 1.0);`. The output
is already `squash(Σ) = Σ/(1+|Σ|)` in (−1,1), so **the clamp means any
negative sum is exactly zero** — not a small probability, a literal freeze.

Seven direct terms (`ant.ron`): `Bias +2.0`, `Energy −1.75`,
`FoodAdjacent −1.16`, `Crowding −0.3`, `Stillness +1.5`, `Alarm −1.0`,
`KinNeed +1.25`. Plus four hidden units, which are the whole homing circuit:

```
h0 = squash(−45 + 45.5·CarryingFood + 6.0·PheroAAlong)   →  Move ×(+2.5)
h1 = squash(−45 + 45.5·CarryingFood − 6.0·PheroAAlong)   →  Move ×(−2.5)
h2 = squash(+0.5 − 45.5·CarryingFood + 6.0·PheroBAlong)  →  Move ×(+2.5)
h3 = squash(+0.5 − 45.5·CarryingFood − 6.0·PheroBAlong)  →  Move ×(−2.5)
```

A differential pair twice over, each gated by a ±45 bias on crop fill, each
entering `Move` with opposite signs — so a **shut pair contributes exactly
zero** and an open one contributes `2.5·(h0 − h1)`.

Worked, for a laden ant at half energy with nothing else firing:

| `PheroAAlong` | homing pair | Σ | **P(move)** |
|---|---|---|---|
| **+0.3** (facing home) | +3.155 | +4.28 | **0.81** |
| **0.0** (no reading) | 0.000 | +1.125 | **0.53** |
| **−0.3** (facing away) | −3.155 | −2.03 | **0.00** |

That is the entire homing mechanism, and it matches the measured buckets
(0.64 / 0.23 / 0.02).

### 1b. Which cell — and the trail is not in this at all

`step_chain:10239`. Three candidates, always: `[heading+1, heading,
heading−1]`.

```rust
let turn    = outputs[Turn];                        // ant: 0 unless it is hot
let persist = unit_scale(outputs[Persist], 2.0);    // unauthored → 1.0
let footing = unit_scale(outputs[Caution], 1.2);    // unauthored → 0.6
let base    = [turn.max(0.0), persist, (-turn).max(0.0)];
scores[i]   = base[i] + if has_foothold { footing } else { 0.0 };
```

**The ant authors no weight into `Turn`, `Persist`, `Caution`, `Tumble` or
`Impulse`** — checked across every species file: every ant-family genome has
exactly **one** direct `Turn` wire and **none** through a hidden unit.
`unit_scale` maps 0 → half of max, so those outputs are not off, they sit at
their midpoints, and for the shipped ant the candidate scores are constants:

| candidate | with foothold | without |
|---|---|---|
| straight ahead | **1.6** | 1.0 |
| left / right | 0.6 | 0.0 |

`choose_weighted` then picks with probability ∝ `(0.1 + s)²`. **Straight ahead
is ~2.4x more likely than a turn onto solid ground, and `TempAboveAmb −0.8` is
the only thing in the whole genome that can bias left against right.**

### 1c. When it does not step — `tumble`

On a failed move roll, at probability `unit_scale(0, 1.0) = 0.5`, `tumble`
rewrites `heading` **uniformly at random** among headings whose landing is
placeable and has a foothold (`creature.rs:12201`). Nothing weights it
homeward, because `home_bias` is 0.

### 1d. The shape this leaves

**A run-and-tumble bacterium.** The trail sets the *duration of runs* and
never their direction. Homing works only because the ant re-aims at random
until it happens to face up-gradient, then runs. That is why a laden ant nets
**+0.0054 cells/tick** against a 90-cell trip.

**Three couplings worth naming**, each found by reading rather than
predicted:

- **Gravity is gated on the move roll.** The fall lives *inside* `step_chain`
  (`:10220`), which is only reached once `draw.unit_f32() < p_move` has
  succeeded. An ant standing on nothing with `P(move) = 0` **hangs in the
  air** until a roll succeeds. A falling ant also returns `true`, so it counts
  as `moved` and **lays pheromone**.
- **There is no hunger gate.** Energy is charged *after* the move (`:4890`).
  Hunger reaches movement only through `Energy → Move −1.75`, which points the
  *wrong way* — a hungry ant has low energy, so the term is small, so it moves
  **more**. There is no "too weak to move" state at all.
- **`still_ticks` increments on a refused step too** (`:4654`). An ant frozen
  by a bad gradient is accumulating `Stillness`, which pushes `Move` back up.
  `STILL_SATURATION = 192` ticks is **1,152 frames**, and the input is the
  *square* of the fraction — a curve whose shape is load-bearing, because a
  linear ramp over 64 ticks took deliveries **4,908 → 54**.

---

## 2. What real ants do, from our own research

From [`ant-sim-literature-review-external-2026-09-19.md`](ant-sim-literature-review-external-2026-09-19.md)
and the repo's review of it,
[`ant-sim-research-review-2026-09-19.md`](ant-sim-research-review-2026-09-19.md):

1. **Osmotropotaxis.** Ants steer by comparing **left antenna against
   right**, with a sigmoidal / Weber-law response to the local pheromone
   *difference*. Perna et al. (2012) showed that individual response
   reproduces the Deneubourg choice function and generates realistic
   dendritic networks. Direction is modulated by the trail **every step**.
2. **Path integration.** Desert ants run a home vector (Wehner); the trail is
   a *contextual modulator*. The *Veromessor* result is the part that
   transfers: **a long vector dominates the trail, a short one defers to it.**
3. **Deneubourg's choice function**, `P_A = (k+A)ⁿ/[(k+A)ⁿ+(k+B)ⁿ]`, n≈2,
   k≈20 — where **A and B are the pheromone concentrations on the two
   branches**.

### The gap, as three facts

| | |
|---|---|
| **Direction is uncoupled from the trail.** | `Turn` carries one wire, and it is temperature. |
| **`Turn` is structurally inert on flat ground.** | `open-bugs-handoff.md` **§R4, OPEN**, with a clean reproduction: **byte-identical movement over 1,600 frames while the eye reported prey on 71% of its casts.** Both outer candidates lose at *every* `Turn` value — the downward diagonal fails `passable`, the upward one fails `body_has_foothold`. Its own words: *"A walking creature on level ground cannot be steered; it can only be scattered."* |
| **We ship Deneubourg's function with the pheromone removed from its arguments.** | `choose_weighted` is `(k+s)²` — the right function — but `s` is `persist + footing`. |

**What we do get right**, and the review credits it: `PheroAAlong =
(ahead − here)/(ahead + here + guard)` **is** a Weber relative difference,
scale-free in ratio. Our sensing obeys the right law. We feed it into a timer
instead of a rudder.

### The honest defence, which is also a constraint

`ant-sim-research-review` §7, written before any ant existed: **every foraging
diagram in this literature is top-down, and this engine is a vertical
section.** A side-view trail has almost no bifurcations, and the side antennae
genuinely do read floor and air — that is the Jones/Physarum dead end, and it
is *why* the along-heading scalar replaced the laterals. It was not
carelessness; it was the projection.

But note what the substitution cost: **replacing a lateral reading with an
along reading converts a steering signal into a speed signal.** Two
independent routes — reading the code, and reading the literature — arrive at
the same sentence.

---

## 3. The mechanism that is already built, and switched off

`wiki/ants.md` already says a laden ant homes *"the way a bacterium does"*.
The review's verdict: **"which both sources call the wrong primitive. The
right one is built and switched off."**

`home_weighted_pick` (`creature.rs:12251`), commit `da4a461a`:

- `forage_anchor` is the world coordinate of the last nest touch, re-anchored
  at every contact, and **exact** — no integration error, no drift.
- Inside `tumble`, at probability `home_bias × crop_fill`, the re-roll picks
  the viable heading with the **largest dot product** with the home vector
  instead of a uniform one.
- Gated on `CreatureDef::home_bias`, `#[serde(default)]`. **Verified
  2026-09-20: no species file authors it**, so it ships at 0.0 and takes no
  RNG draw at all.
- **No lab dial, no wiki line, and no landed measurement.**

**Where it composes well with what is already there.** A laden ant facing away
from home computes `P(move) = 0` and always fails its roll; `tumble` then
fires at 50%; and the homeward pick fires inside that at `home_bias × fill`.
So the freeze the ratchet produces becomes *the opportunity to re-aim*. The
ratchet and the compass are complementary rather than competing.

---

## 4. The measurement

### 4a. What already existed, unlanded, and what is wrong with it

An 18-seed `home_bias` sweep was found on this box at `/tmp/claude-0/hb`,
run **2026-09-19 03:37–04:07**, never written down anywhere — which is why the
research review reported "no landed measurement I could find". The `hand` arm,
paired within seed:

| `home_bias` | homeward tumble % | `DELIVERED` med | `carry->nest` med | sign on `carry->nest` | `born` med | sign on `born` |
|---|---|---|---|---|---|---|
| **0 (shipped)** | **0.00** | 0 | 64.5 | — | 2.5 | — |
| 0.1 | 1.10 | 0 | 76.0 | 11/5/0 | 4.0 | 10/6/2 |
| 0.25 | 1.85 | 0 | 100.0 | 12/6/0 | 1.0 | 6/10/2 |
| 0.5 | 4.28 | 0 | 121.5 | **14/2/0** | 0.5 | 6/8/4 |
| **1.0** | **8.35** | **6.0** | **216.5** | **17/1/0** | 0.0 | **3/11/4** |

Three things to read from it:

- **The mechanism counter is a perfect positive control.** Homeward tumble %
  goes 0.00 → 1.10 → 1.85 → 4.28 → 8.35, **18/0/0 at every setting**. The
  lever fires, monotonically, dose-responsively. `CLAUDE.md` asks for an
  effect counter from the far side of an "it fired" counter; here both are
  present and both move.
- **`carry->nest` — laden ant-ticks inside the nest band — rises 3.4x**, at
  17/1/0. `DELIVERED` goes from a median of **0 to 6**, and the laden leg
  median from **0 to 1,675**: from "no laden ant ever completes a leg home" to
  one that does.
- **It costs founding.** `born` falls 2.5 → 0.0 at `home_bias = 1.0`, worse in
  11 of 18 seeds. At 0.1 it is *better* (10/6/2) with `carry->nest` still up.
  So there is a trade and the sweep has a shape, which is what a dose-response
  is for.

**Why it is a hypothesis and not a result:** it ran at **03:37** and
`TRAIL_A_RHO = 0` landed at **11:52 the same day**, eight hours later. Every
row above was measured on a tree whose homing plane was being erased faster
than an ant could walk it — and on the pre-§7.47 sensor. The compass may have
been *compensating* for a dead plane, in which case it wins less now; or the
two may compose, in which case it wins more. Nothing in that table can say.

### 4b. Re-run on the current tree — and it is bigger, not smaller

**2026-09-20, 36 seeds, gap 90, `arms=hand`, `refill=400`,
`RAYON_NUM_THREADS=2`, on `fcea4b88`** — i.e. after `TRAIL_A_RHO = 0`, after
the §7.47 sensor repair, and after `main` merged in. `hand` arm, paired within
seed. **`homebias=0.0` cannot be passed** — the harness asserts a rider must
differ from the file — so the baseline arm is the one that omits it, which is
also the proof the rider is wired.

> **Read this table, not the one that stood here first.** The original §4b
> headline was *"`DELIVERED` median 0 → 28"*, and `DELIVERED` is not a
> provisioning measure. `trailfollow`'s own doc disqualifies it:
> `CreatureStats::deliveries` increments on **any** drop while `at_nest`,
> whatever was dropped and wherever it came from, and the harness proves it —
> at gap 300 `near on` is **0 in all six seeds** (not one ant ever reached the
> food) while `deliv on` reads 0, 0, 6, 4, 13, 10. **A change that drives
> laden ants to the nest inflates that counter by construction.** Corrected
> 2026-09-20 within the hour, against the ledger columns below. `CLAUDE.md`'s
> worst-recurring failure, walked into while writing a plan that cites it.

| | **0 (shipped)** | **0.5** | **1.0** |
|---|---|---|---|
| homeward tumble % | **0.00** | 5.6 — **36/0/0** | 10.5 — **36/0/0** |
| **`carry->nest`** (signed cells home) | 123.0 | 226.0 — 27/9/0 | 254.5 — **34/2/0** |
| **`came back`** (round trips) | 1.0 | 2.0 — **21/9/6** | 2.0 — **22/7/7** |
| `trips laden` | 1.0 | 1.0 — 18/8/10 | 1.0 — 19/9/8 |
| `reached food` | 12.5 | 11.0 — 13/20/3 | 10.0 — 11/24/1 |
| **`ate J`** (larder intake) | **8,367** | 7,098 — **15/21/0** | 5,586 — 15/20/1 |
| `carry@nest` | 39,689 | 43,239 — 18/18/0 | 42,501 — 16/20/0 |
| `born` | 3.5 | 2.0 — 12/20/4 | 0.0 — **5/23/8** |
| `starved` | 14.0 | 15.0 — 16/18/2 | 15.0 — 13/18/5 |

**1. The navigation claim holds, and it is large.** `carry->nest` is *signed
cells, positive homeward* — a real displacement measure, not a proximity
count — and it **doubles**, 123 → 226 at 27/9/0 and → 254.5 at **34/2/0**
(p < 0.0001). The mechanism counter beside it reads 0.00 on the baseline and
36/0/0 with the lever. **The ant genuinely gets home better.**

**2. Round trips rise — the exact column §7.47 was null on.** `came back`
median 1 → 2, sign **21/9/6** (p ≈ 0.04) and **22/7/7** (p ≈ 0.009); totals
58 → 80 → 95. This is the first thing measured on this line that moves that
number since `TRAIL_A_RHO = 0`.

**3. And the colony is not better fed. This is the finding.** `ate J` — the
larder intake from `ColonyBooks::diet()`, the measure the harness says the
ledger cannot fool — goes **8,367 → 7,098 → 5,586**, *worse* in 21 and 20 of
36 seeds. `carry@nest` is a dead coin flip at 18/18/0, and that is the exact
counter `dead-ends.md` `[other:128]` names as the return leg's test. `reached
food` falls (13/20/3, 11/24/1) — unsurprising, the ants are going home instead
of searching.

> **So the return arm improves, and the intake it costs is the bill this was
> predicted to carry.** The research review wrote it before the run: *"trips
> up, intake down on a hungry bed, **until a granary makes a delivery worth
> something**"* (§3 item 4). An ant carrying food home instead of eating it at
> the larder, into a nest `larder_probe` measures at *ten cells in transit,
> resident 0*, **must** show intake down — that is the mechanic working and
> the world not yet catching it.
>
> **Which is why the granary is Stage 6 and not Stage 0.** A granary built
> before this is a reader with no writer, the failure `dead-ends.md` records
> three times. `home_bias` is the writer. A prediction landing is also
> evidence the model is right: this is the second time on this line that the
> record called a result in advance, and `CLAUDE.md`'s tidiness rule cuts in
> favour of a messy predicted outcome over a clean unexpected one.

**4. The founding cost is real at 1.0 and marginal at 0.5.** `born` 3.5 → 2.0
(12/20/4, p ≈ 0.18) and → 0.0 (5/23/8, p ≈ 0.0005).

**5. The compass composes with the plane fix rather than substituting.** §4a's
open question. Pre-repair at 1.0, `carry->nest` went 64.5 → 216.5; on the
current tree the baseline itself has doubled to 123 and the compass still
doubles it again. Had it merely been compensating for a dead plane its
advantage would have shrunk. **Caveat: the two trees also differ by the §7.47
sensor repair and by `main`, so "the plane fix doubled the baseline" is an
attribution this run cannot make** — only the composition claim is supported.

**6. The control §7.47 owed is paid, and on the right column.** That section
read "round trips do not move" off a table whose `came back` has a per-seed
median of **one** — consistent with *no effect* and with *an effect the bed
cannot resolve*. Same bed, same harness, same 36 seeds, `came back` here moves
**21/9/6**. So the bed can resolve a real effect on that column, and §7.47's
and §7.49's nulls are about the change rather than about the bar.

**What this run does not resolve:** one bed (`refill=400`), one gap (90), and
only `{0, 0.5, 1.0}`. 0.5 and 1.0 were each tested against baseline and
**never against each other**, so "0.5 beats 1.0" is not established.

---

## 5. The plan

### Stage 1 — Ship `home_bias` on, at a value the fine grid picks

Ship it **on**, under the standing ruling the review names: *ship new
behaviours on; a default that looks wrong is to register and report, never to
tune*. **§4b already clears the decision in principle — `DELIVERED` 0 → 28 —
and leaves only the value open.**

- **Fill in the grid** `{0.1, 0.25, 0.75}` against the `{0, 0.5, 1.0}` already
  run, 36 seeds, gap 90, `arms=hand`, `RAYON_NUM_THREADS` pinned. The prior
  sweep hints 0.1 may take the return leg with **no** founding cost, which
  would make the trade disappear rather than be accepted.
- **Repeat on the second bed** (`refill=2000`). §7.46's trip-versus-intake
  trade showed up only there, and §4b is one bed.
- **Outcome** `DELIVERED` and `carry->nest`, paired within seed, with `born`
  and `alive` in the same table.
- **The bar, pre-registered: `came back` and `carry->nest` up, `born` the cost
  to minimise.** The value that buys the most return leg for the least
  founding cost.
- **`ate J` is NOT a bar on this stage, and an earlier revision of this
  document wrongly made it one.** That version required intake not to fall,
  which asks the return leg to pay for a granary that does not exist —
  inverting the loop's own order. Ants that carry food home instead of eating
  it at the larder, into a nest measured at *"ten cells in transit, resident
  0"*, **must** show intake down; that is the mechanic working and the world
  not yet catching it, and §5 is where it gets caught. Record `ate J`, do not
  gate on it.
- **Never read `DELIVERED`** for this or anything else on this line. Read
  `ate J`, `came back` and `carry->nest`. The first version of this plan led
  on `DELIVERED` and was wrong within the hour — **that withdrawal stands and
  is unrelated to the bar above**: one was a counter measuring the wrong
  thing, the other a correct counter used as the wrong gate.
- Add the **lab dial** and the **`wiki/ants.md` line**, both of which the
  review flags as missing, and a **README status section** before calling it
  done.

### Stage 1b — Confirm where the food goes (one run, does not block)

§4b's finding is that **an ant that walks home is not an ant that feeds the
colony**: `carry->nest` doubles at 34/2/0 while `ate J` falls at 15/21/0 and
`carry@nest` is a coin flip.

**The answer is predicted, so this is a confirmation and not a hunt.** The
research review wrote it down before the run: *"trips up, intake down on a
hungry bed, until a granary makes a delivery worth something"* (§3 item 4).
The run below checks the predicted cause and gives §5 its baseline; nothing
waits on it. Three candidates, in the order they are likely:

1. **The returning ants are not carrying larder food.** `Carrying` is
   `crop_fill.max(spoil ? 1.0 : 0.0)` and `home_bias` is keyed on *crop fill*
   specifically to avoid marching an ant home for dig tailings — but the
   `trips laden` totals (28 → 44 → 54) against `came back` (58 → 80 → 95) say
   under half of returns are laden at all. **Census what a returning ant
   holds**, by material, at the moment it crosses the nest band.
2. **They carry it home and it does not book.** `ate J` comes from
   `ColonyBooks::diet()`; a drop that lands outside the band, or a crop that
   empties into a corpse, books nowhere. Pair the drop counter with the diet
   band per seed.
3. **The search time lost costs more than the return gains.** `reached food`
   falls 13/20/3. If most of the intake was always eaten *at* the larder
   rather than carried, a change that pulls ants off the food is a net loss
   however well they navigate — and that would make this a **scene** finding,
   not a navigation one.

The discriminator is free and is the whole of this stage: split `ate J` by
**where it was eaten**, at the larder against inside the nest band. The
harness already books by material; this needs the position.

### Stage 2 — Weight by vector *length*, not only by crop fill

The *Veromessor* result: a long vector dominates the trail, a short one defers
to it. `home_bias × fill` weights by **how full the ant is, never by how far
out it is** — the review names this gap explicitly and says `HomeDistance` was
the master's term for it.

One extra factor in an expression that already computes `len`
(`creature.rs:12270`). Cheap, and it is the difference between "a laden ant
always beelines" and "an ant far from home beelines, an ant near home
forages".

### Stage 3 — Soften the argmax

`home_weighted_pick` ends in `max_by` over the dot product — **a hard
argmax**. Two reasons that is wrong here, and they are the repo's own:

- `dead-ends.md` records replacing `choose_weighted` with an argmax as a named
  dead end: ***"the noise is load-bearing."***
- `CLAUDE.md`'s first law: **an outcome is a distribution, not a binary.** At
  `home_bias × fill = 1.0` a laden ant turns exactly homeward on every tumble.
  That is a perfect compass, and `nest-design` §2 says the error and the
  search *"are what make a real return look like searching rather than
  teleporting."*

Replace the `max_by` with `choose_weighted` over the dot products — the same
function the forward candidates already use, with the same exploration floor.
**This is the stage that decides whether the fix looks right**, as opposed to
measuring right, and it gets a review card.

### Stage 4 — §R4: the steering that does not work

Stages 1–3 give the ant a direction from an *exact internal vector*. Real ants
get it from the *world*, through osmotropotaxis — and the output that would
carry it, `Turn`, is inert on flat ground.

The bug says the fix is not obvious and wants its own design pass, and names
why each easy version fails: letting an unfooted diagonal win puts creatures
back to walking off ledges (`falls 16,451 against moves 22,138`), and letting
the heading rotate without a step is a movement-model change.

**What this plan commits to is the cheap thing the bug itself asks for**: a
counter for how often a `Turn` request is discarded because the side it asked
for scored zero. Size the problem before designing the fix — and note §R4's
own finding that the ability to steer is *a property of the ground*, so the
counter must be read on flat and generated terrain separately.

**One observation to check while doing it, not yet measured.** `footing_ahead`
(`:10317`) is meant to stop an ant marching into the sky; its bar is
`footing × 0.5 = 0.3`, and straight-ahead scores `persist = 1.0` with *no*
foothold. For the shipped ant that early-out may therefore fire only when
straight-ahead is impassable, never on lack of footing — a constant silently
defeating a rule. The footing preference still works through the scoring
(2.4x) and the fall path catches the consequence, so this is a question, not a
claim.

### Stage 5 — Make the per-tick trace standing

The owner's stated method, 2026-09-20: *"select some ants and actually record
every decision they make at every tick and see why they're making the choices
they're making — we should run this every time."*

`trailfollow`'s `trace` already emits the focal CSV and the per-seed TRACE
block. What it does not do is make that the **default** readout of this line.
Make the decision columns first-class — `p_move`, the seven direct terms and
the four hidden contributions *separately*, `along`, `sensor_kind`,
`footing_ahead`, the chosen candidate and why, `tumble` fired, homeward
fired — so a row says not just what the ant did but which term decided it.

This is also the defence against the failure §7.49 records: a per-tick trace
has no denominator to get wrong.

---

### Stage 6 — Then the granary, and not before it

**The loop is discovery → return → recruitment → granary, and the granary is
the last stage for a reason that is this repo's own rule rather than a
preference.** *A channel needs a writer and a reader, and the compiler checks
neither* — a field written and never read is dead weight; one **read and never
written is worse, because every consumer of it is dead code that looks
alive.** `dead-ends.md` records this project hitting that three times: light
with no writer, canopy density with an always-zero reader, pressure with no
liquid consumer.

A granary built today is **a reader with no writer**. `larder_probe` measured
the nest at *ten cells in transit, resident 0* — nothing arrives and stays,
because until stages 1–3 nothing reliably arrives at all. Build the store
first and it is a fourth entry in that list; build the carrier first and the
store has something to catch on its first frame.

So this stage **inverts §4b's cost into the thing that pays for itself**: the
intake `home_bias` spends on transport is intake the colony gets back once a
delivery is worth something. That is why §4b's `ate J` column is recorded and
not gated on.

Out of scope here beyond naming it — the nest needs a store a delivery lands
in, that persists, that the colony can draw on when hungry, and that a player
can see, rob and lose. `nest-biology-2026-09-19.md` finding 7 is the standing
argument for one, and `ant-sim-research-review-2026-09-19.md` §2.3 has the
biology (repletes are the crop; seed caching is half there). **It gets its own
plan, and it now has a writer feeding it.**


## 6. Risks, and the re-derivation budget

**1. `Move`'s terms were calibrated against a uniform tumble.** `CLAUDE.md`:
*a term in a weighted sum is not an independent knob* — before starting a
change that reallocates a shared budget, name the constants calibrated against
current behaviour and budget re-deriving them. They are: `Bias 2.0`,
`Energy −1.75`, `FoodAdjacent −1.16`, `Stillness 1.5`, and `STILL_SATURATION
192`. Changing what `tumble` *does* changes what a failed roll is worth, which
is exactly the sum those were fitted to. **The `born` column in the prior
sweep is this effect already visible.**

**2. Exactness against the ethos.** `forage_anchor` has no error. An exact
anchor is a binary — you know the way or you do not — and the owner's first
law is that an outcome is a distribution. Stage 3 is the answer; if it is not
enough, an error term on the anchor is the next lever, and `nest-design` §2
prices it.

**3. Founding may fall, and that may be correct.** A colony that provisions
instead of founding is a different colony, not a worse one. It must be
*seen* rather than assumed — hence both beds and both cost columns.

**4. Blast radius.** `home_bias` is a `CreatureDef` field, not a genome slot,
so **`mutation_rate` and the genome manifest do not move** — the expensive
half of a brain-input change is not incurred. World-hash gates (`ascii`,
`acceptance.sh`, `tests/determinism.rs`) will move by construction, because
ant behaviour changes; re-baseline, do not widen. The only test that names
`home_bias` is a round-trip assertion in `species_export.rs:340`.

---

## 7. Verification

1. `cargo build --release --examples` with `set -o pipefail` **first** — a
   stale example binary has bitten this line four times.
2. **The positive control, every run:** homeward tumble % must read **0.00**
   on the baseline arm and rise monotonically with `home_bias`. A sweep where
   it does not is measuring a disconnected knob.
3. Paired within seed via `scripts/tracepair.py`, never pooled (§7.49), on
   both beds, with `born` and `alive` in the same table.
4. A **review card** for Stage 3 — does the return read as searching or as
   teleporting — with `DELIVERED` in the card's `meta`, per the house rule
   that only a number says whether it fired.
5. Full gates: `cargo test` (not `--lib`), `clippy`, `ascii` worst-frame,
   `acceptance.sh`, `worldgencheck.sh`, `docscheck.sh`.
6. `python3 scripts/deadendindex.py --touching` before opening anything —
   `home_weighted_pick` and `Turn` both have register entries.

---

## 8. What this rests on

- `src/sim/creature.rs`, `src/sim/brain.rs`, `assets/species/*.ron`, read
  2026-09-19/20 for §1 and §3.
- [`ant-sim-literature-review-external-2026-09-19.md`](ant-sim-literature-review-external-2026-09-19.md)
  and [`ant-sim-research-review-2026-09-19.md`](ant-sim-research-review-2026-09-19.md)
  §0, §2.5, §2.11, §7 for §2.
- [`open-bugs-handoff.md`](open-bugs-handoff.md) §R4 for the steering bug.
- [`pheromone-trail-direction-2026-09-16.md`](pheromone-trail-direction-2026-09-16.md)
  §7.46–§7.49 for the measurements this follows.
- The 18-seed sweep at `/tmp/claude-0/hb`, archived under `Reports/data/`
  with its pre-`TRAIL_A_RHO` caveat attached.
