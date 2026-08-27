# Recruitment measured: grass clears the generational barrier, tree does not (2026-08-27)

**Status: measurement, 16 runs, paired.** Run to settle the one question the
three-way review (`plant-evolvability-three-reviews-2026-08-27.md`) found
gating every other question: **can any plant population in this engine reach
a second generation?** The review's unanimous finding was that none had. It
was measured on `tree`. `grass` ships as a pioneer and had never been
measured.

**Everything below is from one build, one harness, one sitting.** Logs are in
`Reports/data/recruitment-2026-08-27-{grass,tree}/`. **Read §6 before
generalising any of it** — the bed is flat, uniform and undisturbed by
construction.

**Determinism control, run after the fact and passed**: re-running seed 1
produced a **byte-identical** log (`7b499d6d102ab66fb351de766542c04a` both
times), so the eight distinct md5s below are eight distinct worlds rather
than eight samples of run-to-run noise. Environment: ephemeral container, 4
cores, headless, release profile (`lto = "thin"`, `codegen-units = 1`). **No
timing is quoted anywhere in this report** — every number here is a count,
which is why a shared box cannot reach it.

```
cargo build --release --example plant_probe     # set -o pipefail; BUILD_EXIT=0
./target/release/examples/plant_probe species=<sp> trees=16 frames=45000 worldseed=<1..8>
```

Anti-stale checks, both passed: all 16 logs have **distinct md5s**, and every
log's first line echoes its own `worldseed`. This is the defence the 3.5-hour
megastudy lacked when it produced 24 logs holding 3 distinct populations.

---

## 1. The result

| | seeds set | born | died | established | **max generation** |
|---|---|---|---|---|---|
| **grass** | 19–32 | 35–48 | 6–17 | 6–12 | **2 in 7 of 8 seeds** |
| **tree** | 143–196 | 159–212 | 109–148 | 15–17 | **1 in 8 of 8 seeds** |

**Trees set five to eight times more seeds than grass and never once produce
a third-generation-capable individual. Grass, on a fraction of the fecundity,
reaches generation 2 in seven of eight seeds.**

**Fecundity is not the bottleneck. Establishment is.** This is
`open-bugs-handoff.md`'s *"Mortality was necessary and is not sufficient"*
arriving from the other side — and with, for the first time, a species that
clears the bar to compare against.

## 2. The tree zero reproduces exactly, and it is narrower than it reads

Trees reach generation 1 **abundantly** — 34 to 54 gen-1 individuals per
seed. What they never do is *establish* one: tree `established` runs 15–17
against 16 founders, so the established set is essentially the founding
cohort and nothing else, in all eight seeds. Every one of those 34–54
inherited genomes is a seed or a seedling that never gets there.

That is exactly what `open-bugs-handoff.md:3126` means by *"inherited-genome
establishment is still 0"*, and it reproduces on this harness today. It is
**not** "trees never make a second generation of organisms" — they make
dozens. It is "no inherited tree genome ever reaches reproductive size."

> **Correction to this session's own earlier reading**, recorded because the
> distinction is exactly the kind that gets flattened in a summary: an
> interim note to the owner described trees as showing "zero establishment"
> in a way that implied no gen-1 organisms at all. There are many. The zero
> is about gen-1 plants *establishing*, and the difference decides where the
> fix goes — fecundity is already 5–8x what grass manages.

Grass's `established` (6–12) runs *below* its 16 founders, so founders die
**and** inherited genomes take their place. A generation-2 individual is
proof on its own: it requires a gen-1 plant to have reached `seed_maturity`
and reproduced.

## 3. What this changes

- **The generational loop exists in this engine.** The review's unanimous
  "nothing can evolve yet" is a statement about `tree`, not about the plant
  system. Any evolution experiment should run on `grass` or a
  grass-like life history, and the ~20-session recruitment programme the
  feasibility review scoped is **not** a prerequisite for a first radiation
  experiment.
- **The 4,095-organism ceiling is nowhere near binding.** Slot high-water was
  **24–36 of 4,095 (0.7–0.9%)** on grass and 58–72 on tree, with **0 births
  refused** in all 16 runs. Both the design note and the feasibility review
  treated this as a live constraint. It is not, at this scene scale.
- **Rot is not parking slots.** `0 organisms senescent` in every grass run.

## 4. The architectural loci are frozen in practice, on both species

`plant_probe`'s morph histogram covers
`[foliage, angle, internode, sympodial, tropism, density]` among established
plants. Across **all 16 runs, both species**, positions 2 and 3 — branch
angle and internode — are `1` in **every single row**:

```
grass                        tree
[1, 1, 1, 0, 0, 2]           [0, 1, 1, 0, 0, 0]
[1, 1, 1, 0, 0, 1]           [0, 1, 1, 0, 0, 1]
[1, 1, 1, 0, 0, 0]           [0, 1, 1, 0, 0, 2]
[0, 1, 1, 0, 0, 2]           [1, 1, 1, 0, 0, 0]
[0, 1, 1, 0, 0, 1]           [1, 1, 1, 0, 0, 1]
[0, 1, 1, 0, 0, 0]           [1, 1, 1, 0, 0, 2]
     ^  ^        ^
     |  |        density — varies 0/1/2 (positionally founded)
     angle, internode — never move
```

Variation appears exactly where founding draws put it (foliage economy,
wood density) and nowhere else. This is `plant.rs:718-719` — the hardcoded
`= 1` — observed in the population rather than read in the source, and it
confirms review C's finding on live data.

**And it survives a working generational loop, which is the new part.** At
generation depth 1–2, with `DISCRETE_MUTATION_CHANCE = 0.03` per locus per
birth and a third of jumps re-drawing the same allele, the expected number of
visible architectural variants among established plants is of order 1–3
across all eight grass seeds. Zero observed is consistent with that. **With
numbers this small this is not a sharp test of the mutation rate** — what it
does establish is that mutation alone will not populate these axes at the
generation depths this engine reaches, which makes founding them from
positional draws the unblocking change rather than a tidiness.

## 5. What to do next

1. **Found `LOCUS_BRANCH_ANGLE` and `LOCUS_INTERNODE` from positional streams
   66/67**, exactly as `LOCUS_WOOD_DENSITY` uses 65 (`plant.rs:698-706`).
   Keyed streams, so no existing draw shifts. This is a prerequisite for any
   test of whether architecture reads, and §4 shows waiting for mutation will
   not substitute for it.
2. **Re-run the WP-C question with those loci actually varying.** The verdicts
   that retired `weeping` and `prostrate` moved `upward_weight` (separately
   measured inert) and `heading_inertia`; branch angle and internode have
   never been in front of the owner as a *population*, and
   `branch-angle-and-the-width-bound.md` carries the one measured positive.
3. **Run evolution experiments on grass, not tree.** Established here.
4. **Do not treat the slot ceiling or recruitment as blockers** without
   re-measuring at the scene scale in question.

## 6. What this does not show

- Nothing here says architecture *does* read to the owner. §4 says the
  question is still open because the axes have never varied; it is not
  evidence for the affirmative.
- **The scene is flat, uniform and undisturbed, and that bounds this
  hard.** `common::PlantScene` at `trees=16`: 1024x320 cells, ground line at
  y=200, **34 rows of soil at `SOIL_FIELD_CAPACITY` everywhere**, 16 plants
  evenly spaced ~60 columns apart, `start_frame=0`. Day/night (3,600 frames,
  so 45,000 is ~12.5 cycles), wind, gusts and rain are live. There is **no
  terrain, no slope, no moisture gradient, no water table at depth, no fire,
  no structural collapse, no creatures and no player**. It is also a test
  bed: the shipped world is 8192x2560.

  So this scene can answer *"can a lineage complete a life cycle"* — which is
  what it was run for — and **cannot speak to selection, niche
  differentiation or coexistence at all**. Spatial heterogeneity plus
  disturbance is exactly what review B and the Bornhofen work name as the
  mechanism, and this bed has neither. Gate 3 of the three-way review is
  untouched by anything here.
- One frame budget, one growth rate. `plant-evolvability-three-reviews` §6's method traps —
  especially the day/night and rain oscillators, and reading a descriptor
  before it has stopped moving — apply to anything built on this.
- Generation 2 is a low bar. It is the difference between "no selection is
  possible" and "selection is possible"; it is not a deep pedigree, and
  `MUTATION_SIGMA` remains untuned at 0.08.
