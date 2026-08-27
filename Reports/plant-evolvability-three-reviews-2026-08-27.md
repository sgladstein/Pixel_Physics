# Three independent reviews of the plant-evolvability question (2026-08-27)

**Status: review synthesis. Disagreements are preserved, not averaged.** The
owner asked for a three-agent review of a serious design decision and said
explicitly that disagreement was an acceptable outcome. It is the outcome.

Three reviewers ran with **different mandates and different reading access**,
deliberately, because three agents on one brief produce three variations of
one review:

| | mandate | read the design note? |
|---|---|---|
| **A** | feasibility, cost, what kills it | yes, including both refutations |
| **B** | design the best evolution-based answer *independently* | **no** — firewalled |
| **C** | audit the evidence base; design the decisive experiment | **no** — firewalled |

All three were given `plant-evolvability-facts-2026-08-27.md` (claims-only
ground truth) and told to falsify it. B was asked to report if it felt
steered by it; it reported that it did not. C was asked to audit it as an
interested party's account, since the same session wrote it and the refuted
proposal.

**Every finding quoted below was re-verified in source by the coordinating
session before being recorded here.** Two of the three reviews produced
findings that overturn claims that session had made earlier the same day.

---

## 1. The unanimous finding, from two independent measurements

**Nothing can evolve in the plant system today, because there is no second
generation.** A and C reached this from different evidence, neither citing
the other:

- **A**, `open-bugs-handoff.md:3126`: *"Founders now die and **inherited-genome
  establishment is still 0 at both horizons.** Mortality was necessary and is
  not sufficient… the next session should stop re-deriving that hypothesis."*
  Zero at 28,800 and 45,000 frames, eight seeds, *after* `STARVATION_DEATH_TICKS`
  landed.
- **C**, `plant-genome-design.md:555-566`: `genome_drift species=tree
  founders=16 frames=200000`, seeds 1 and 7 — **maximum generation reached
  was 1, on both.** The standing population is essentially the founding
  cohort; the rest is seed bank. *"No frame budget alone reaches a
  multi-generation selection experiment."*

**Consequence, and it is the finding that reorders everything:** every
question about *what the genome should hold* is downstream of a question
nobody has paid for. A structural genome with no turnover is a mutation
operator that never fires twice. The 2026-08-26 design note's gates 1 and 2
were both aimed below this.

A also closed the escape hatch the note reached for. `plant-simulation-
research.md` §7d's "headless fast-forward with its own clock" is
`growth_slowdown`, and `clock.rs:42-64` records a paired 8-seed sweep in
which **the same number of organism ticks** at 4x slowdown produced a median
**0.61x** final cells with ~9x spread: *"a slowed subsystem is not the same
subsystem later."*

## 2. Where the reviews split

### A and B: architecture is spent; build elsewhere

**A** — feasible-but-expensive, and blocked upstream. Costs the programme at
~20-35 sessions of which the genome itself is ~6: *"the encoding is the small
part, and the design note spends its whole budget there."* Its ranked killer
is the generation problem above.

**B** — proposes an **organ budget** instead: a 6-way allocation simplex
(root / stem / leaf / storage / reproductive / attachment) that generalises
`allocate_to_frontier`'s existing 2-way root:shoot weight, plus heritable
tissue-per-organ from a clade's legal list, plus a jumping size-class allele.
Budget mutations are bounded and renormalised, so they are **always viable** —
which sidesteps §7a's nonviability objection rather than fighting it.

B's diagnosis, verified independently by the coordinator: **the genome points
at the walk, and the phenotype is set by the body.** `leaf_cluster` — which
sets foliage share — is consumed raw at `plant.rs:1509` and `:2438` and is
never genotype-scaled. The same holds for `shade_death`, `drought_death`,
`seed_cost`, `seed_chance`, `seed_maturity`, `seed_half_life`,
`remains_half_life` and all three material fields. **Composition and life
history are not heritable at all**, so the ~90% wood / ~5% leaf ratio that
`plant-appearance-design.md` says sets the silhouette is precisely what
evolution cannot currently touch.

B explicitly lists the structural / L-system genome under *what I would not
build*.

### C: "architecture is dead" is folklore, and has never been tested

C challenges the premise both A and B rest on, and the challenge **verifies**:

**Four of the six discrete loci are monomorphic in every stand this engine
has ever grown.** `plant.rs:718-719` hardcodes `LOCUS_BRANCH_ANGLE = 1` and
`LOCUS_INTERNODE = 1` for every founder; `LOCUS_SYMPODIAL` and
`LOCUS_TROPISM` are copied verbatim from the species file (`:736-737`). Only
wood density and leaf economy draw positionally (streams 64/65). The four
architectural loci can vary *only* through `DISCRETE_MUTATION_CHANCE` at
`set_seed` — i.e. only at generation ≥ 1 — i.e. only inside a seed bank that
never establishes (§1).

The founder code says why, and the reason is sound in itself: *"a freshly
planted stand is exactly the species as written and every morph is one
mutation away in either direction."* The unintended consequence is that the
morphs are one mutation away in a population that never reaches generation 1.

**So what did the "architecture is inert" verdicts actually test?** WP-C's
three probes moved `upward_weight` — separately measured **inert across 1,024
genomes** (`plant-species-authoring.md`) — and `heading_inertia` 0.05 vs 0.1.
The appearance phase's own diagnosis is that its levers were rendered through
a stand that was 90% wood drawing from one four-brown palette. Neither tested
`branch_angle` or `internode`, because those cannot vary.

**And the one architectural change with a measured positive is buried under a
stale header.** `branch-angle-and-the-width-bound.md:3` still reads *"built,
measured, working, and **NOT merged**"*. It **is** merged —
`organism.rs:587,607` and five species `.ron`s carry the fields. Its measured
result, 8 trees / 30,000 frames, on scale-free descriptors: crown profile
`[100, 80, 41, 0, 0]` → `[100, 95, 0, 0, 0]`, foliage centre 80 → 89,
*"trunk → limb → twig is legible… where before the plant was a tangle of
similar-looking strands"*, and — unlooked for — *"the conifer lean is largely
gone"*, an open bug with three dead theories behind it.

C's verdict: the evidence does not support a large architectural decision
now, **in either direction**. *"The 'architecture is inert' diagnosis is a
reasonable prior that has been laundered into a finding."*

## 3. What this does to the 2026-08-26 design note

The note is now **three times corrected**. §5a withdrew its first
recommendation; §5b withdrew `ByOrder`; this review round removes the ground
under the framing that produced both:

- The note's central dichotomy (parametric genome → blob; developmental
  genome → novelty) was already weakened by Niklas's six-variable parametric
  morphospace (§5b). C's finding weakens it further from the other side: the
  parametric genome this engine *has* was never given the chance to express
  four of its six discrete axes.
- **The coordinating session told the owner that "three independent sessions
  landed on 'not the structural genome'".** That overstated it. C is a fourth
  session saying the evidence for that conclusion is weaker than claimed, and
  C's own decisive experiment has an outcome ("coverage low") that points
  *back* at the structural level.

## 4. What all three support, or none opposes

**Two cheap corrections C established, neither of which needs a decision:**

1. **Give founders allele variance on branch angle and internode.**
   `plant.rs:718-719`. Drawing them from positional streams 66/67 exactly as
   density uses 65 shifts no existing draw (keyed streams, not sequential),
   makes a founding stand polymorphic on the two loci with the only measured
   positive behind them, and is a **prerequisite for any architecture test**.
2. **Fix `branch-angle-and-the-width-bound.md`'s status header.** It is
   suppressing the single counter-example in the record.

**The cheapest measurement on the board — A's, ~20 minutes, existing binary,
no code written:**

```
cargo build --release --examples     # set -o pipefail; the include_str! gotcha
cargo run --release --example plant_probe -- species=grass trees=16 frames=45000 worldseed=<1..8>
```

The zero-establishment result was measured on **trees** (`seed_maturity: 600`,
`seed_chance: 4e-05`). **`grass` ships at `seed_maturity: 10` / `8e-05`
(`grass.ron:290`), was authored as a pioneer, and its establishment has never
been measured.** `plant_probe` already prints lineage turnover, slot
high-water and births refused. Both outcomes are decisive: if grass
establishes, the generational loop exists and the programme is the ~20-session
build; if it reads zero too, **every evolution claim at any horizon this
engine can run is about founders**, and the next work is recruitment, not
encoding.

**The decisive experiment, C's, for the question after that:** a **plant
morphospace census** (`examples/plant_space.rs`, ~250 lines, no engine
change), modelled on `creature_space.rs`, whose own doc states the logic:
*"If random genomes all behave identically, selection has nothing to act on…
better to learn that in an hour than after building queens, eggs and
inheritance."* Sample N genomes uniformly over the **full legal range**
rather than perturbing `tree.ron`; one genome per world, K plants per genome,
median-plus-IQR per genome as the unit; five **scale-free** descriptors with
**size excluded from the grid and printed as a diagnostic**; positive control
= grass vs tree must separate or the instrument is blind and nothing is
published; negative control = an all-identical `tree.ron` arm whose within-arm
spread *is* the noise floor. Acceptance is a blind owner card asking his own
question verbatim: *"are these different plants, or one plant in several
sizes?"*

## 5. Corrections to the fact sheet, from the reviews

- **The 4-bit `CellType` budget is not the pinch (A).** `pack_cell_type`
  decodes at one site (`organism.rs:2530`) and bits 4-15 of `aux` are free;
  widening is two lines. The real constraint is **209 `CellType::` references
  in `src/`** hardcoding what each variant *means*.
- **The variable-length determinism gate is already solved (A, and B
  independently).** `APPENDED_JITTER_SALT` (`plant.rs:826`) draws from
  `rng::stream(world_seed ^ SALT, …)` and consumes zero draws from the
  caller's `Rng`, so genome width cannot shift any other stream. §5b listed
  this as an open gate; it is not.
- **The world is 8192x2560** (`app.rs:67-68`), not the 512x320 `CLAUDE.md`
  still names. 4,095 organism slots over 8,192 columns is one plant per two
  columns — the ceiling is live.
- **The epiphyte ban is stale (B).** The germination guard was deleted and
  replaced by economics (`plant.rs:2611-2640`); no `wiki/` page carries the
  rule. The climbing vine is less blocked than the reach report assumed.
- **`light_weight` is not a dead lever (C).** It is inert *because the light
  field is nearly uniform above the canopy* — a flagged input bug, not a
  verdict on the lever.
- **`divergence` does not already answer "does a locus move morphology" (C).**
  `examples/divergence.rs:82-88`: `enum Axis { Moisture }`, one arm, and it
  varies an *environmental* setting. Its metrics are two aspect ratios of
  magnitudes. The fact sheet §10 overclaimed; the scale-free descriptors live
  in `plant_probe.rs:519,532,533`.
- **§7's verdict register is second-hand (C).** It quotes report summaries
  rather than the review queue, which breaks the fact sheet's own stated
  standard. The queue holds three later cards (2026-08-23), including the one
  that **originated this whole question** and carries the owner's own causal
  hypothesis: *"I think the issue is in the base design of the random walk
  growth."* C notes these strengthen the fact sheet's direction on net — but
  the `grass` 4/5 figure traces only to report text and no such card was
  found.

## 6. Method traps flagged for whoever runs the experiments

Stale binary / `include_str!` (the 3.5-hour megastudy failure —
`cargo build --release --examples` with `pipefail`, and the harness must echo
its own parameters *and* a hash of the genome it stamped); position-keyed RNG
making location heritable (P-21) — stamp genomes **after** germination, since
`plant.rs:5686` draws at the resting coordinate and would overwrite an
earlier stamp; size eating the answer if it enters the grid; designed
oscillators (day/night 3,600 frames, first rain at 14,400 — every arm shares
`start_frame` and runs a 3,600 multiple); the `canopy_top` ceiling void
(discard clipped samples and **report how many**, since clipped plants
converge and read as "the space is flat"); and no sort "optimisation" on
`allocate_to_frontier` while any of this runs.
