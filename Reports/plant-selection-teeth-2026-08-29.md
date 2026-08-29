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

The straightforward reading is the bed: `PlantScene::default()` is **uniform
soil at field capacity across the whole width**, so there is no water or
rooting-volume scarcity for a root system to compete over, and a better root
buys nothing. `Relief::Varied` exists precisely to vary both, and the same arm
is running on it now — that comparison is the test of this reading and it is
not in this report.

**Read the power table before reading this as "roots do not matter".** A
true effect of 3 points would be invisible here.

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
