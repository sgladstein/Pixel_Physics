# The world has teeth, and they are graded (2026-08-29)

**Status: measurement, 5 arms x 18 seeds, each seed a mirrored pair.** Closes
the gap `plant-fate-operator-gate-2026-08-29.md` §6 names — *"the gate asks
whether a mutant lives, never whether it wins, and no experiment in this repo
yet asks the second question for plants"*.

It was built to answer a **false-negative** worry, raised by the owner: run an
evolution experiment in a world whose selective pressures are incomplete,
measure nothing, and record *"evolution does not work"* when the truth is
*"this world does not select"*. Those are separable, because a genome known to
be worse gives the environment something it must be able to punish.

**The answer is that this bed selects, and it discriminates among plants that
are alive rather than merely removing ones that are not.** It also says
plainly which pressure is missing.

```
./target/release/examples/selection_arena arm=<same|lethal|early|nobranch|norootbranch> seeds=18
```

## 0. The caveat, and its resolution: the numbers held

**Raised, then checked.** §1's arms ran 20,000 frames and the share does not
settle until around frame 50,000-75,000, so they were mid-transient readings
presented as equilibria — `CLAUDE.md`'s censused-before-it-settles trap, whose
remedy is that *the tell that works is the quantity holding still*, never a
frame budget that looks generous.

**Re-measured at 90,000 frames, the load-bearing number moves 0.5 points:**

| | 20,000 frames | 90,000 frames |
|---|---|---|
| `nobranch` median | 38.9% | **39.4%** |
| seeds where B lost | 18 of 18 | **11 of 11** |
| `nobranch` p | 0.0002 | 0.0039 |
| control median | 55.4% | 57.4% |
| control p | 0.19 | 0.13 |

A 4.5x change in run length moves the headline by half a point and leaves
every seed pointing the same way. **§1 stands as written.** The caveat was
correct to raise and did not bite — recorded because a check that clears is
evidence, and deleting it would leave the next reader to re-run it.

**One thing the re-run did expose: the settle detector is too strict.** It
flagged 6 of 12 control runs and 6 of 11 `nobranch` runs as still moving at
90,000 frames, while the answer barely changed. Its rule — under one
percentage point of drift across a quarter of the run — is tighter than the
share's own wobble around equilibrium, so it reports "unsettled" for runs that
are settled to any precision this harness can use. Treat a flag as *read the
trajectory*, not as *discard the number*; and note it fails in the safe
direction, which is why it was left as it is.

*(90,000-frame figures: 12 seeds control, 11 `nobranch`, against 18 apiece at
20,000. The narrower n is why `nobranch`'s p rises from 0.0002 to 0.0039 while
its median is flat — signed-rank saturates at n, so fewer seeds cannot reach
as small a floor. It is not a weaker effect.)*

## 0a. The original caveat, as written before it was checked

**The arms in §1 ran 20,000 frames, and the system does not settle until
around frame 50,000-75,000.** They are therefore mid-transient readings, not
equilibria.

Found by a 150,000-frame run whose purpose was something else entirely.
Arm B's share over its last eighteen samples:

```
58.4 58.2 58.1 57.9 56.1 55.7 55.4 55.9 55.5 55.9 55.8 55.6 55.7 55.6 55.5 55.7 55.6 55.6
```

It rises, flattens, and then holds ~55.6% for the whole second half. This is
`CLAUDE.md`'s censused-before-it-settles trap, whose remedy is stated there and
was not applied here: *the tell that works is that the quantity being censused
has stopped moving* -- never a frame budget that looks generous.

**What this does and does not put in doubt.** The *direction* is safe: 18 of 18
seeds is not going to reverse on a longer run, and the ladder's ordering
(`lethal` < `early` < `nobranch` < `norootbranch` ~ control) is a large,
consistent signal. The *magnitudes* are provisional -- 38.9% for `nobranch` is
a value on the way to an equilibrium, not the equilibrium. A re-run at 90,000
frames is under way and this section will be replaced by its numbers.

`selection_arena` now reports how many world-runs ended while the share was
still moving, so this cannot recur silently.

## 1. The ladder

`herb`, 8 founders, 20,000 frames, flat bed. Arm B's share of the final
biomass; 50% is no effect.

| arm | what changed | median | quartiles | seeds B lost | p |
|---|---|---|---|---|---|
| **control** (`same mirror=off`) | nothing | 55.4% | 46.6–59.2 | 6/18 | **0.19** |
| `lethal` | shoot's `Grew` child -> `Seed` | **0.0%** | 0.0–0.0 | 18/18 | 0.0002 |
| `early` | determinate at 2 metamers, not 8 | **7.1%** | 5.1–8.4 | 18/18 | 0.0002 |
| `nobranch` | bud matures instead of flushing | **38.9%** | 35.3–42.7 | 18/18 | 0.0002 |
| `norootbranch` | root laterals are dead ends | 49.7% | 48.2–52.8 | 10/18 | **0.86** |

**`nobranch` is the load-bearing row.** `lethal` is close to tautological — a
plant whose tip builds a seed leaves no biomass *by construction* and would
die alone in an empty world. `nobranch` is a plant that grows, flowers, sets
seed and is visibly a herb, and it loses **11 points of the bed on every one
of 18 seeds**. That is selection between two living plants, which is the claim
that matters.

## 2. The control is what licenses the rest, and it had to be rebuilt once

**Mirrored, `arm=same` is vacuous.** It returned exactly 50.0% on every seed
and both metrics, which is the tidiness `CLAUDE.md` warns is evidence of an
artifact before it is evidence of anything. It is an algebraic identity: with
one genome in both arms, the mirrored pair is *the same simulation with the
labels swapped*, so pooling gives `A == B`. A control that cannot fail is not
a control.

Run **unmirrored** it can fail, and it passes: median 55.4%, but **z=1.31,
p=0.19**. So the bed's own asymmetry — position plus genotype draw — is
visible and not significant at this n.

**That asymmetry is large enough to have manufactured a finding.** With
identical genomes, single seeds read **34.2%** and **73.1%**. An unmirrored
one-arm experiment could have reported a 20-point "effect" from nothing, which
is why every real arm is a mirrored pair: each position is occupied by both
arms at the identical genotype draw, so position and draw cancel exactly
rather than approximately.

## 3. What the design cannot see, stated before anyone reads a null into it

From the control's own spread — **±9.3 share-points per seed with no true
effect present**:

| true effect | seeds for 80% power |
|---|---|
| 20 points | 8 |
| 7.5 points | **18** |
| 5 points | 40 |
| 2 points | 158 |
| 1 point | **620** |

**18 seeds resolve ~7.5 points and are blind below ~5.** A selection
coefficient of 1% — *strong* in population genetics, enough to fix an allele
in a few hundred generations — sits in the 620-seed column, and one seed is
two mirrored 20,000-frame runs.

So this design answers *"is the environment dead?"* and cannot answer *"can
selection act on the variation mutation actually produces?"* The second needs
a **different design, not more seeds**: a frequency trajectory over many
generations, which integrates a small per-generation effect instead of reading
one endpoint. `OrganismState::lineage` — now claimed by plants — is what that
needs.

**And the test saturates.** Once every seed points one way the signed-rank
floor at n=18 is p=0.0002, so `lethal` (median 0.0%) and `early` (7.1%) return
*identical* statistics. Past the floor the magnitude lives in the median.

## 4. The null that is a finding: roots are not under selection here

`norootbranch` reads **49.7%, 10 of 18, p=0.86** — no detectable effect,
against a `nobranch` handicap on the same plant that reads 11 points. Removing
the shoot's ability to branch costs it the bed; removing the root's costs
nothing measurable.

### 4a. The obvious explanation was tested and is **wrong**

The straightforward reading was the bed: `PlantScene::default()` is uniform
soil at field capacity across its whole width, so there is no water or
rooting-volume scarcity to compete over and a better root buys nothing.
`Relief::Varied` varies moisture linearly and rooting depth on a full sine
period, so it should switch that on.

**It does not.** Both branch arms re-run on the varied bed, 18 seeds each:

| arm | bed | median | seeds B lost | p |
|---|---|---|---|---|
| `nobranch` | flat | 38.9% | 18/18 | 0.0002 |
| `nobranch` | **varied** | 36.9% | 17/18 | 0.0003 |
| `norootbranch` | flat | 49.7% | 10/18 | 0.86 |
| `norootbranch` | **varied** | **50.6%** | 7/18 | **0.28** |

The shoot handicap is unmoved by the bed and the root handicap is still a
null. **The hypothesis this section was written to test is refuted**, and the
prediction is recorded rather than quietly replaced because it was made in
advance and it was wrong.

### 4b. What the evidence actually supports

The asymmetry tracks the **binding resource**, not the bed's variability.
`plant-throughput-herb-2026-08-29.md` measured herb's carbon binds as severe —
*"leaf construction refuses 45–48% of wanted cells and organ clusters 31–36%"*
— so these plants are carbon-limited, not water-limited. Selection sees what
the binding constraint sees: shoot architecture serves light capture and
therefore carbon, and is worth 11 points; root architecture serves water and
nutrients, which are not scarce, and is worth nothing measurable.

That reading predicts something specific and testable, which this report does
not test: making water genuinely limiting — a bed at or below the wilting
point, or a drought cycle — should bring the root arm to life. **Varying the
bed is not the same as making a resource scarce**, and only the second should
matter.

**Read the power table before reading any of this as "roots do not matter".**
A true effect of 3 points is invisible here on either bed.

## 5. Three vacuous arms, and what each cost

Recorded because all three read as *clean 50/50s* — which is exactly what "the
world has no teeth" looks like, and the more comforting misreading.

- **`lateral: None` is not "no lateral".** `plant.rs`'s Grow arm is
  `fate.and_then(|f| f.lateral).unwrap_or(cell_type)`, so clearing the field
  falls back to *the growing cell's own type* — on a `GrowingTip`, exactly the
  `Some(GrowingTip)` already there. The field says what a lateral **is**, never
  whether there is one.
- **A `herb` shoot never places a lateral at all, while its root does.**
  Editing the shoot's lateral type is byte-identical to the control on every
  seed; editing the root's is not. The two halves of one plant reach branching
  by different routes, and a shoot branches only when a bud flushes — which is
  why `nobranch` now poisons `(DormantBud, Flush)`.
- **A genome that changed is not a plant that changed.** The first guard
  checked only that the edit moved the genome, which is the gate report's
  *silent* class wearing a new hat.

The harness now detects the class itself: under the mirror a silent arm gives
A and B *exactly* equal integer tallies, so it reports **"arm B changed the
genome and did NOT change the plant"** instead of a clean 50/50, and it
refuses to start if the arm edit matched no rule at all.

## 5a. The trajectory readout, and the axis that is not a clock

§3 says the fix for the resolution floor is a different design: fit the slope
of `logit(share)` against generation, since detection then needs
`g * Ne > 4/s^2` and improves with **run length** rather than seed count. That
readout is implemented. **It does not work, and the reason is worth more than
the readout was.**

**First: a 20,000-frame run is all transient.** `dump=1` prints the raw curve,
and looking at it settles in one run what three layers of inference had not:

```
gen 0.00  55.1%   gen 0.81  62.4%   <- rises
gen 1.09  52.1%   gen 1.55  47.6%   <- then falls
gen 2.00  50.0%
```

A hump, not a line. Every sample sits inside establishment, so a fitted slope
measures the transient — which is exactly what its **+0.446** intercept was
saying, since both arms start equal and the honest intercept is 0.

**Second, and fatal: the generation axis saturates.** A 150,000-frame run —
7.5x longer — does not reach 7.5x the generations. It reaches **2.9** and then
stops:

| frame | ~1.5k | ~15k | ~45k | ~75k | ~110k | ~150k |
|---|---|---|---|---|---|---|
| mean generation | 1.04 | 1.79 | 2.88 | 2.84 | 2.75 | 2.60 |

Mean generation is taken over **living** organisms, so at steady state the
deaths of old plants balance the births of new ones and it **equilibrates**.
It is a property of the population's age structure, not elapsed time.

So `s * g` cannot grow however long the run, the `g * Ne > 4/s^2` argument is
unreachable on this axis, and **a longer run buys nothing** — which is the
opposite of what §3 promised. The harness now detects a span under 3
generations and says so rather than fitting a slope against an axis that does
not move.

**Third, and it supersedes the second: the share equilibrates too.** Even a
correct clock would not rescue this, because there is no ongoing signal to
integrate -- log-odds stops moving because the *system* settles, not because
the axis does. The arms reach a stable coexistence rather than one displacing
the other, which is what every arm in §1 already showed and nobody read that
way: `nobranch` sits at 38.9% and `early` at 7.1%, neither heading for zero.

**So selection here is equilibrium-seeking, not directional**, and an endpoint
share is the right observable after all -- provided it is read *at* the
equilibrium (§0), which these runs were not. The cumulative-clock fix below is
recorded because it was the obvious next move and it is wrong; do not build it.

**~~The design is not dead; its clock is wrong.~~** ~~What is needed is a
*cumulative* generation count~~ — deepest generation reached, or cumulative
births over standing population — both of which do keep rising
(`plant-throughput-herb-2026-08-29.md` measured deepest established generation
5, 7 and 3 across seeds). That is the next build, and it is not validated
here.

**What survives from the calibration attempt:** the control half, `s = +0.054`
(z=0.00, p=1.0000) — there is no transient or saturation to confuse when there
is no effect. The positive half does not survive and `nobranch`'s `-0.34` must
not be quoted as a selection coefficient.

## 6. What is not established

- **One species, one bed geometry, one frame budget.** Everything here is
  `herb` at 20,000 frames on a flat field-capacity bed.
- **`lethal` is near-tautological** and should not be quoted as evidence the
  environment selects; `nobranch` is the row that carries that claim.
- **Fitness here is share of standing biomass at one instant**, not lifetime
  reproductive success. Seeds set is printed beside it and the two agree in
  direction on every arm, which is a weak check and not a validation.
- **The varied-bed comparison in §4 is running, not reported.**
- **Nothing here is about mutation.** Every arm is a hand-authored handicap of
  known direction. Whether the mutations the engine actually makes produce
  effects in this range is the next question, and §3 says this design cannot
  answer it.
