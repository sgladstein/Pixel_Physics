# Plant evolvability: where this stands, and the drift to avoid (2026-08-27)

**Status: handoff. Read this before continuing the plant-evolvability line.**
Written at the end of a long session, at the owner's request, so that the
conclusions reached in conversation are on disk rather than in a transcript.

Its most important content is **§3, a correction the owner caught** — the
session drifted from *"build the missing machinery"* toward *"characterise
the machinery we have"*, and those are different projects.

## 1. The documents, in reading order

| file | what it is |
|---|---|
| `plant-morphology-reach-2026-08-23.md` | the original: what a sunflower/tomato/vine needs. **Still stands.** |
| `plant-morphology-evolvability-2026-08-26.md` | can those forms *evolve*? **Twice corrected — read §5a and §5b, which withdraw two of its own proposals.** |
| `plant-evolvability-facts-2026-08-27.md` | claims-only verified ground truth, no recommendations |
| `plant-evolvability-three-reviews-2026-08-27.md` | three independent reviews, **disagreements preserved** |
| `plant-recruitment-measurement-2026-08-27.md` | 16 paired runs. Grass vs tree recruitment; §5a the germination diagnosis; §5b an instrument bug; **§5c a confound in its own headline** |
| `Reports/data/recruitment-2026-08-27-{grass,tree}/` | the raw logs, 8 seeds each |

## 2. What is actually established

1. **The reach report's diagnosis holds.** New archetypes need organ cell
   types, determinate axes and a climbing tropism. Nothing measured since
   has touched that, and no amount of genome work substitutes for it.
2. **Two proposals for *what should be heritable* were refuted** (§5a, §5b
   of the evolvability note): priced loci on one ancestor is still
   parametric; `ByOrder` is a fixed-arity array of four numbers and is not a
   production set. **What should replace them is open.**
3. **Generation depth is the live constraint for trees and the blocked stage
   is germination**, established by a seed-bank-versus-decay comparison, not
   by inference from mortality.
4. **Four of six discrete loci are frozen** — `plant.rs:718-719` hardcodes
   branch angle and internode to `1` in every founder — confirmed in 16 live
   runs where those positions never move while foliage and density do.
5. **The evidence that "architecture does not read" is weaker than it looks**
   (review C): the probes that produced it moved `upward_weight`, separately
   measured inert, and `heading_inertia`. It is a reasonable prior that has
   been repeated until it reads as a finding.

## 3. THE DRIFT, and the correction — owner-caught, 2026-08-27

> "I thought our original conclusion was we didn't yet have the machinery
> available to create the morphology differences we're looking for and we had
> to create that first, but now it seems like we're testing morphology
> differences first as a feasibility assessment. what am I missing?"

**Nothing. The session drifted and the objection is correct.**

Two different questions got conflated:

| | question | answers the goal? |
|---|---|---|
| **the goal** | can we build machinery that produces genuinely different plant forms? | yes — this is the project |
| **the census** | does the genome space *as it exists today* contain visibly different forms? | **no** |

The morphospace census (review C's design) is a good experiment for the
second question and **it is not on the critical path to the first.** Even its
best possible outcome — "the current space is rich" — does not produce a
flower, a fruit, or a determinate axis, because those cell types and
behaviours do not exist. The reach report's bill of materials is unchanged by
any census result.

**Why the drift happened, recorded so it is not repeated:** review C's
challenge is genuinely interesting — the "architecture is inert" verdict was
reached with the architectural loci frozen, so it could not have come out any
other way — and an interesting open question pulled the session toward
settling it. But it is *one reviewer against four measurements*, and settling
it is a side quest.

**The honest value of the census, stated at its real size:**
- it hedges against review C being right, which would mean the existing axes
  have untapped reach and are worth pushing;
- its positive control (grass vs tree must separate) **builds and validates
  the descriptor set that judging the new machinery will need anyway.**

That is worth something. It is not "the decisive experiment", which is how
the session had begun describing it, and it should not displace the
machinery work.

## 4. If the census is built anyway — the parts that matter

- **Sample uniformly over the full legal range, never perturb `tree.ron`.**
  Perturbation samples the neighbourhood and would rediscover the
  blob-around-the-ancestor problem while looking like a test of the space.
- **Founder allele variance is NOT a prerequisite for it.** `plant.rs:718-719`
  runs in `seed_genotype` at *germination*; a harness that stamps
  `genotype_draws` and `alleles` onto organisms **after** they germinate
  bypasses the freeze entirely. (This corrects advice given earlier in the
  same session. The founder fix is needed for live evolution, not for
  sampling.)
- **One genome per world, K plants each**; the unit of comparison is the
  genome, its descriptor the median over its own K plants with the IQR
  beside it. This is what defeats the within-stand spread (root cells
  90/438/1435) that made every previous stand-versus-stand morphology
  comparison unreadable.
- **Scale-free descriptors only; size excluded from the grid** and printed as
  a diagnostic. Include size and the result is guaranteed positive and
  worthless.
- **Positive control**: grass vs tree must separate, or the instrument is
  blind and nothing it reports counts. **Negative control**: an all-identical
  `tree.ron` arm, whose within-arm spread *is* the noise floor.
- **Acceptance is a blind owner card** asking his own question verbatim —
  *"are these different plants, or one plant in several sizes?"* — not the
  coverage number.

## 5. If disturbance is added to the bed — one trap

The test bed is flat, uniform and undisturbed, so it cannot speak to
selection at all (`plant-recruitment-measurement` §6). Adding a gap process
is installing real ecology rather than faking it — the engine already has
fire, collapse and the player, and `PlantScene` simply has none of them.

**But do not cull by age.** Age-biased removal is itself a selective force
favouring fast reproducers, so it would *manufacture* the ruderal-strategy
result the experiment is hoping to observe. Use a **neutral random hazard**:
each plant, each interval, a fixed probability independent of age, size and
genotype. Age-structured mortality is legitimate later, once the space is
known to be rich.

## 6. Open owner calls, carried forward

1. What replaces the withdrawn §6.2 — i.e. what *should* be heritable?
2. Which clades, and in what order? (Bryophyte is substantially `moss.ron`.)
3. Does the niche table keep naming species? Radiation and authored
   per-niche sowing are in direct tension.
