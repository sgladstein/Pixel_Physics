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

## 5a. Which stage of the life cycle actually fails, and why

Decomposed on `tree` seed 1 (the run with the census time series). **The
blocked step is germination, not fecundity and not seedling survival.**

| stage | outcome | evidence |
|---|---|---|
| set seed | **works** | 155 seeds set in one run; 143-196 across seeds |
| land viably | works | 40 standing above the surface, **0 buried** (buried can never germinate) |
| **germinate** | **essentially never** | see the decay arithmetic below |
| establish | 1 of 155, at 23 cells / 9 rows | `established plants carrying an inherited genome: 1 of 17`, and **0 of 16** in the other seven seeds |
| reproduce | never | no established gen-2 plant in any tree run |

**The discriminator is the standing seed bank against a pure-decay
prediction.** `default_seed_half_life()` is **9,000 frames**
(`organism.rs:1192`) and `tree.ron` does not override it, so tree seeds
half every 9,000 frames — five half-lives over a 45,000-frame run.
`grass.ron:127` sets **18,000**. For seeds set uniformly across the run, the
expected standing count is `n x (1 - e^-Lt)/(Lt)`:

| | seeds set | half-life | expected standing under **decay alone** | observed |
|---|---|---|---|---|
| tree | 155 | 9,000 | **~43** | **40** |
| grass | 24 | 18,000 | ~11 | **6** |

**Tree's bank sits almost exactly where decay alone puts it** — so it is
being emptied by the half-life clock, essentially undrawn by germination,
and the 114 deaths are overwhelmingly seeds expiring in the bank rather than
seedlings starving. **Grass's bank sits well below its decay prediction** —
seeds are leaving it by germinating, which is what its 6-of-8 established
inherited genomes are made of.

*(Assumption stated: seed-set uniform over the run. The census series
supports it — standing seeds accumulate steadily 1, 5, 9, 15, 20 ... 40 —
but this is an estimate against a model, not a direct measurement of
germination events. A germination counter would settle it outright and does
not exist.)*

**Why germination fails is measured, not inferred.** `Behavior::Germinate`
is gated on `light_threshold` **and** `soil_water_threshold`
(`organism.rs:822`), and by frame 45,000 the founders have removed both:

| | tree | grass |
|---|---|---|
| canopy fusion | **84%** | **0%** |
| leaves below 0.5 noon-equivalent light | 14,183 of 22,648 (**63%**) | n/a |
| median leaf light | **0.14** of 4.0 | n/a |
| soil at or below wilting point | **55%** | **7%** |
| stomatal term (1.0 = demand met) | median **0.81** | **1.00** |
| cells shed to starvation, cumulative | **16,730** | 109 |

**And the founders do not leave.** 10-13 senescent against 16 founders over
45,000 frames, and the stand has stopped growing outright — **6
`GrowingTip` cells in the entire world** at frame 45,000, against 395
`DormantBud`. The founders are themselves running a deficit (bill/income
median **1.31**, max 3.08) and shedding cells to starvation, but they hold
their ground and their canopy.

So: **this is forest gap dynamics with the gaps missing.** A real seed bank
waits for a treefall. Here the canopy never opens, the germination gate never
clears, and the bank expires on its half-life clock. That is the precise
content of `open-bugs-handoff.md`'s *"mortality was necessary and is not
sufficient"*.

### 5b. An instrument bug found on the way, and it invalidates one column

`plant_probe`'s carbon book reported `bill / income median inf` for grass,
with `income min 0.000 median 0.000 max 0.000`. It is not a night-phase
sample: **grass owns no `CellType::Leaf` cells at all** — its census is 410
`MatureBody` and 6 `Seed`, photosynthesising on `grassblade` material
through `MatureBody`. The probe sums income over cells whose type is
`CellType::Leaf` (`plant_probe.rs:475`), so for any species without a leaf
stage it sums nothing and reports exactly zero.

**Consequence: the carbon book cannot measure grass, moss, or any future
leafless species, and reads a plausible `0.000` rather than failing.** No
grass carbon number in this report is used for anything. This is
`CLAUDE.md`'s *"ask what your number counts when nothing is wrong"* — the
figure is arithmetically correct and answers a different question than the
one asked.

## 5d. MEASURED: the germination counter, added 2026-08-27, confirms §5a

§5a reached its conclusion by arithmetic against a decay model and said so.
`World::germinations` now counts every germination at `plant::germinate`, the
single chokepoint, and settles it directly. Logs in
`Reports/data/germination-2026-08-27/`.

| run | seeds set | germinations | **minus 16 founders** | rate | established w/ inherited genome |
|---|---|---|---|---|---|
| tree 1 | 155 | 17 | **1** | 0.6% | 1 of 17 |
| tree 2 | 153 | 19 | **3** | 2.0% | 0 of 16 |
| tree 3 | 170 | 16 | **0** | 0% | 0 of 16 |
| grass 1 | 24 | 26 | **10** | 42% | 4 of 12 |
| grass 2 | 32 | 24 | **8** | 25% | 2 of 8 |
| grass 3 | 21 | 22 | **6** | 29% | 0 of 7 |

**Pooled: tree 4 germinations from 478 seeds (0.8%); grass 24 from 77 (31%).
A ~40x difference in germination rate, measured rather than inferred.**

**Read the counter with the founders subtracted.** The 16 founders are
planted as seeds and germinate like any other, so a raw "17 germinations"
looks like healthy recruitment until you take them out. This is the trap the
counter creates and it is why the table above has that column.

**Three independent quantities agree on tree seed 1**: the counter says 1,
the `inherited genome` line says 1 of 17, and the decay prediction said ~43
standing against 40 observed. §5a stands.

**One refinement it does add.** Germination is necessary and not sufficient,
for both species: tree 2 germinated 3 and established none, and grass 3
germinated 6 and established none. So there is a real
germinate-then-fail-to-establish component — it is simply dwarfed, on tree,
by the fact that almost nothing germinates at all.

## 5c. A confound in this report's own headline: grass cannot starve

**`plant.rs:4810` gates starvation death on `has_leaf_stage`:**

```rust
// ... until then a species with no `Leaf` stage cannot starve to death.
if has_economy && has_leaf_stage {
```

`grass.ron` declares no `Leaf` cell type at all (its header says so
explicitly, and the cell census confirms it: 410 `MatureBody`, 6 `Seed`).
**So grass is exempt, by construction, from the starvation mortality that
operates on trees.** Its measured `0 organisms senescent` is that exemption,
not a finding about grass vigour.

**What this qualifies:** the headline comparison in §1 has a
species-asymmetric mortality rule inside it. "Grass sustains a generational
loop and tree does not" is partly "grass is immune to the rule that kills
tree seedlings." Any later use of these numbers must carry that.

**What survives it intact:** the §5a germination diagnosis. The seed-bank-
versus-decay arithmetic compares each species' standing bank against *its
own* half-life, and starvation immunity does not enter it — a seed that never
germinates was never subject to the starvation rule either way. Tree's bank
matching pure decay, and grass's falling below it, is unaffected.

**Also filed, not fixed** (`open-bugs-handoff.md` §V2): the same
`has_leaf_stage` shape makes `OrganismState::income` structurally zero for
leafless species, which gates `break_buds`' `supportable` — so grass can
never flush a bud either. See §5b.

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
