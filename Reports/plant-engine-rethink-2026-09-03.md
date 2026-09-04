# Re-thinking the plant engine: what the genome can reach, and what a plant actually inherits

*2026-09-03, an unattended overnight session answering
[`plant-engine-rethink-brief-2026-09-03.md`](plant-engine-rethink-brief-2026-09-03.md).
Two instruments, one mechanism, and one finding that reframes the rest.*

> *"As much as possible nothing should be hard coded, we don't want to design
> specific behavior but create a flexible system that will allow variety to
> evolve."* — the owner, and the sentence the brief exists to serve.

---

## 0. The short version

**Two questions were measured that nobody had measured, and the second is the
one that matters.**

**Q: what phenotypes can a lineage reach?** The continuous genome expresses as
`base * (1 + draw * variance)`, and both `base` and `variance` are authored per
species and never inherited — so the reachable set is a closed interval fixed
the moment somebody writes the `.ron`, and an authored **zero is a cage no
mutation can leave**. Measured across the seven species that grow: **3 of 70
(species x slot) cells are caged**, 60 live, and 7 are slot 9, which has no
consumer at all. §1.

**Q: how much of the difference between two plants is their genome?** Very
little of the part a person notices first. Broad-sense heritability of plant
**size** is **0.03 median over four reference genomes** (0.00 / 0.01 / 0.05 /
0.38), against a positive control — the widest genetic contrast the engine can
express — reaching **0.44–0.74** on the same descriptor. So the column is not
blind; the genetic variation the engine actually produces is just small against
its developmental noise. Composition is the heritable half (foliage share,
median 0.61), and **crown width is not reachable at all**: its positive control
reads 0.000 on all four. The owner's *"clones of the same plant end up growing
very different"* is not an impression, it is the dominant term. §2.

**That second result reframes the first.** Widening what a genome *can* express
buys nothing if selection cannot see what it *did* express. A cage matters less
than a signal-to-noise floor, and the floor is where the leverage is.

**Landed.** `organism::ParamGenome` — every scalar in a species' behaviour
table as a heritable, mutable per-individual override, founded empty and
therefore **inert on every founder**. And, on top of it,
`Behavior::Reproduce::seed_launch`: the dispersal channel the reseeding report
measures as absent, as a heritable distance founded at zero — worth **+38%
plants established well away from a founder column** at a reach of 12, on 3 of
3 seeds, and inert at the zero every species ships (§6.1). It replaces the authored number instead
of scaling it, so an authored zero is a starting point. It multiplies the
heritable surface from **70 slots to 804 addresses**, and it ships with its
mutation rate at **0.0** — the mechanism is measured, the *rate* is not, and
§5.4 says exactly what must be measured before raising it. Two instruments
land with it: `examples/genome_reach` and `examples/clone_variance`.

**Also answered, from the brief's own list.** Foliage share no longer reads
~5%: it is **43 / 42 / 41 / 37 / 31%** across tree / conifer / shrub / herb /
scrambler, so the first of `plant-appearance-design.md`'s two stated causes for
the invisible architectural levers **is stale**. `grass` reads **0** — it owns
no `CellType::Leaf` at all. §3.

**And a clonal growth form is reachable in the shipped game and does nothing.**
A `herb` lineage can already point a root's lateral at a shoot — the fate
genome acquiring a growth form no species file authors (§4a). Then 144 of those
launches, across eight worlds, produce **no second stem above the background
rate and no extra width at all**, on a stand 22% smaller for having tried
(§4b). Reachability is not the binding constraint; the engine gives an
underground shoot nothing to do.

**Three claims this session made and then withdrew**, all recorded because the
withdrawal is the useful part: that tree, conifer and shrub cannot evolve a
branching root system (§1.3 — slot 1 has a second consumer); a first
heritability table taken with a `ref=` knob that was never connected (§2.1);
and *"the organism id in the growth key is a per-individual random seed and
removing it is a one-line fix"* — the arm that would show that holds the id
fixed by construction, so what it measures is position alone (§2.3), and the
real repair is a larger one (§7 item 2).

---

## 1. What the genome can reach — `examples/genome_reach`

### 1.1 The arithmetic

`plant::genotype` is the whole of the continuous genome's expression:

```rust
pub fn genotype(world: &World, organism_id: u16, slot: usize, variance: f32) -> f32 {
    if variance <= 0.0 { return 1.0; }
    let draw = world.organism(organism_id).map_or(0.0, |s| s.genotype_draws[slot]);
    (1.0 + draw * variance).max(0.0)
}
```

and every consumer multiplies it into an authored species constant. The draw is
heritable and lives in `-1..=1`; `base` and `variance` are read from the species
file every time. So the set of phenotypes a lineage can ever reach on one slot
is

```
[ base * (1 - variance),  base * (1 + variance) ]   clamped at 0
```

and it is **not a property of the lineage**. Two ways it collapses to a point:
`variance == 0` (pinned — usually just slot ownership, since slots 1/5/8 are
read from the `RootTip` vector and the rest from the shoot's), and `base == 0`
— **caged**, because zero times any genome is zero.

### 1.2 The census

`genome_reach` with no arguments prints, per species and per slot, the authored
base its consumer actually reads, the authored variance from the vector that
consumer actually reads, the resulting interval, and a verdict. Pooled over the
seven species that grow:

| | cells |
|---|---|
| live — a real interval a lineage can move inside | **60** |
| **CAGED** — authored base is zero | **3** |
| PINNED — authored variance is zero | 0 |
| no consumer — drawn, inherited, mutated, read by nothing | 7 |

The three cages: **`herb`'s shoot `branch_chance`** (`[0.0, 0.0]`, against a
`genotype_variance` of 0.4 that is therefore multiplying nothing), and
**`grass`'s `plastochron` and `pipe_ratio`**, both authored `0`.

`herb.ron`'s own comment on that line reads *"**Near zero: one stalk.** Not
literally zero, because a plant that cannot branch at all has no way to recover
from losing its apex"* — and the value is literally zero. The intent and the
number disagree, and the multiplier form turns that slip into a permanent
property of the lineage.

**`moss` is not in the census and that is a finding rather than an omission —
and the sharp version of it is sharper than "no slots".** It declares one
behaviour, `Divide`, and no `Grow`, so it has no `genotype_variance` vector and
**none of the ten continuous slots is expressed for it**: its whole continuous
genome is drawn, inherited, mutated and read by nothing.

But `Divide` writes `with_organism_id(organism_id)` — **the same organism** —
so a moss patch is one individual spreading, and it never bears anything. Every
heredity channel in this engine hangs off `plant::bear_seed_at`: the continuous
jitter, the discrete allele jump, the fate mutation, and now the parameter
mutation. **So no heredity of any kind reaches moss. It is outside evolution
entirely**, and the parameter genome does not change that — it would give moss
three addresses (`Divide`'s cost and its two chances) and there is no event at
which one could ever mutate.

That is not a defect to fix tonight; it is a shipped species that the whole
evolution programme silently excludes, and nothing in the record said so.

### 1.3 The correction: a slot can have two consumers, and one does

The first version of this census returned **one** base per slot and reported
slot 1 CAGED on `tree`, `conifer` and `shrub`, whose roots author
`branch_chance: [0.0]` against a slot-1 variance of 0.5. That reads as *three
of seven species can never evolve a branching root system*, and it is wrong.

The dynamic arm (§1.4) caught it: widening slot 1 on `tree` **moved the world
on every seed**. The reason is written in the species files in as many words —
`tree.ron`'s root carries a comment reading *"Superseded by `branch_priming`
below"*, and `plant.rs:3346` reads slot 1 a second time to divide the root's
priming interval (*"the branching oscillator"*). The authored zero is a retired
mechanism, not a cage.

**The general form, and the reason this is a section rather than a quiet fix:
a reachability census taken by reading one call site is a census of that call
site.** When the arithmetic table and the widening arm disagree, the arm is the
one to believe. `bases_of` now returns every consumer of a slot and marks it
caged only when all of them are zero.

### 1.4 The dynamic half, and why it is a hash

The static table says what the arithmetic permits and cannot say whether a
permitted interval *does* anything. `genome_reach -- grow=1` settles that: it
widens one slot's `genotype_variance` to 1.0 and hashes the whole grid against
a baseline of the same scene. Widening a variance consumes no draw —
`seed_genotype` fills the vector whatever the widths are — so the two arms
share an RNG stream and any difference at all is that slot expressing itself.

**A hash rather than a phenotype summary, deliberately.** Every summary this
repo reaches for over plants is a lossy projection, and `CLAUDE.md` records six
occasions where a number was arithmetically correct and about a different
question. `hash unchanged` is not a projection: it says the two worlds are the
same world, cell for cell. It is also asymmetric in exactly the right
direction — outcomes here are chaotic in the seed, so a *changed* hash is weak
evidence of importance and an *unchanged* one is conclusive evidence of nothing.

Two controls print on every run and are asserted. The **positive** control is
that a slot the static table calls live must move the world; without it, a
harness that patched the wrong species, or patched after the founders were
planted, reports every slot dead and looks like a decisive finding. The
**negative** control is slot 9, which has no consumer and must not move.

Result over three species x three world seeds, 8 founders, 16,000 frames —
**every live slot moved the world and every caged slot was byte-identical, on
every seed**:

| species | live slots that moved | caged slots the world confirms dead |
|---|---|---|
| `herb` | 8 of 8, on 3 of 3 seeds | `branch` |
| `tree` | 9 of 9, on 3 of 3 seeds | none — and this is what produced §1.3 |
| `grass` | 6 of 7 (`alloc` did not) | `plast`, `pipe` |

`grass`'s one exception is the scene rather than the slot: it establishes ~8
organisms holding 123 cells in this bed, so the allocation bias never has a
regime to express in. `grass` is a species whose numbers in this bed are mostly
about the bed.

**And the same run at a shorter horizon is a trap worth recording.** At 4,000
frames and 4 founders, five of the eight live slots read `identical`
(`turgor`, `pipe`, `roottr`, `alloc`, `penetr`) — because the stand had not yet
thickened, rooted to depth, or entered the regimes those slots govern. A null
in this instrument at a short horizon is a statement about the scene. The
run that is worth quoting is the one where the positive control comes back
complete.

### 1.5 The bigger number is not the cage

Three caged cells is a small defect. The larger one is what the continuous
genome **never addresses at all**. Counted over the seven species that grow, a
plant is specified by roughly **85 named fields** in its `.ron` — materials,
palette bands, half-lives, and every scalar on `Grow`, `Photosynthesize`,
`Absorb`, `Reproduce`, `SecondaryThicken`, `Ripen`, `Germinate` and `BudBreak`.
Against that: ten continuous draws and six discrete loci.

`genome_reach` counts the addressable surface directly. Every cell type x
parameter x branch order a species actually has:

| | addresses |
|---|---|
| tree / conifer / shrub / creeper | 114 each |
| grass | 112 |
| herb / scrambler | 118 each |
| **pooled** | **804** |
| continuous genome, for comparison | 70 (60 live) |

The clearest single case is **root `plastochron`**, authored `0` in **every
shipped species** and addressed by no genome slot at all — slot 2 reads the
*shoot's*. A `plastochron` of zero means "this axis has no nodes", so no plant
in this engine may put a node underground, in any lineage, ever. A node
underground is a rhizome, a runner, or a sucker.
`plant-reseeding-2026-09-03.md` names clonal spread as *"one authored number
away and no species has taken it"*; it is more than that — no species **can**
take it by evolving, and clonal spread is one of the five things the owner
named as wanted.

### 1.6 And what is outside heredity entirely

A seed copies its parent's **species id unchanged**, so everything on
`SpeciesDef` is fixed for a lineage for ever: the six materials it is made of,
the four palette bands, `seed_half_life`, `remains_half_life`. A lineage
cannot change what it is made of — which is also why it cannot change how its
seeds fall, since `friction_angle` is a property of the material.
`individual_as_species` exists and refuses, because it copies the parent
*species'* fates rather than the individual's, so **speciation is impossible by
construction**. That is unchanged by this session and is §6.2.

---

## 2. How much of a plant is its genome — `examples/clone_variance`

The owner, mid-session: *"within the current engine clones of the same plant
end up growing/looking very different from one another which makes it much
harder to identify when growth patterns do change."*

**The scatter is in the record and has never been treated as a defect.**
`plant-heritability-survey-design-2026-08-27.md` §3 states it as a method
consequence — *31 to 153 cells and 90 / 438 / 1,435 root cells for identical
genomes, so a median is not a shape* — i.e. "do not quote a stand median". The
consequence nobody had drawn is the sharper one: **if developmental scatter is
larger than genetic difference, selection cannot see the genome either.** A
population would then re-roll its variety every generation instead of
inheriting it.

### 2.1 The instrument, and the arm that was vacuous first

Three arms in one bed, paired on world seed:

- **`pop`** — the shipped stand, every founder its own genome.
- **`clone`** — every founder carrying founder `ref`'s genome, written through
  the new `World::set_organism_genotype` (which also sets `inherited`, or
  `seed_genotype` redraws at germination and the arm silently becomes the
  control).
- **`spread`** — the estimator's positive control and mandatory: half the
  founders at every continuous draw `-1` with allele 0 at every locus, half at
  `+1` with the top allele. The widest contrast the engine can express. If no
  descriptor separates *that* from a clone stand, the descriptors are blind and
  every number above them is void.

`H2 = 1 - Var(clone)/Var(pop)`, with the variances pooled **within** each seed
and then averaged — a variance pooled *across* seeds carries the between-world
difference in both arms and drags every ratio toward 1.

**The first version of this arm was vacuous, and the tell was the standing
one.** `ref=0`, `ref=1` and `ref=5` produced **byte-identical output**.
`PlantScene::build` plants through `World::plant_tree_species`, which allocates
the organism and writes the cell and **never calls `plant::seed_genotype`** —
only `World::plant_tree` does. So at frame 0 every founder holds
`genotype_draws = [0.0; 10]`, the species mean, and they are already clones of
each other: the arm was cloning the mean genome and reporting it as a clone of
a sampled individual. The fix is one line (found a real genome at the reference
founder's own coordinate before copying it), and it moved the answer: the
broken arm read clone-arm CV on height at 0.376, the fixed one at 0.213.

### 2.2 The result

`herb`, 16 founders, 12,000 frames, four world seeds pooled per row, and the
whole table repeated for four different **reference genomes** — because one
genome is one sample, and a founder sitting near a threshold in the economy
makes every clone of it sit there too.

`H2 = 1 - Var(clone)/Var(pop)`:

| ref genome | cells | height | width | slenderness | foliage share | root share | foliage centre |
|---|---|---|---|---|---|---|---|
| 0 | **0.013** | 0.199 | 0.161 | 0.155 | 0.551 | 0.309 | 0.255 |
| 1 | **0.054** | 0.470 | 0.226 | 0.253 | 0.673 | 0.409 | 0.403 |
| 2 | **0.000** | 0.534 | 0.315 | 0.163 | 0.313 | 0.082 | 0.378 |
| 3 | 0.376 | 0.575 | 0.227 | 0.427 | 0.669 | 0.630 | 0.278 |
| **median** | **0.034** | 0.502 | 0.227 | 0.208 | 0.610 | 0.359 | 0.328 |
| *positive control* | *0.44 – 0.74* | *0.62 – 0.80* | ***0.000 x4*** | *0.14 – 0.42* | *0.62 – 0.82* | *0.13 – 0.65* | *0.56 – 0.65* |

Three things to read out of it.

**Plant size is the least heritable thing the engine produces within a
species** — median `H2 = 0.03`, and three of the four reference genomes read
0.00–0.05. The last row is what makes that a statement about the *world*
rather than about the descriptor: the widest genetic contrast the engine can
express moves cell count clearly (0.44–0.74), so `cells` is not a blind
column. **The natural genetic variation this engine produces is simply small
against its developmental noise**, which is exactly what `base * (1 +
draw*variance)` predicts — a band of +/-15% to +/-70% around one species mean.

**Which inverts the standing complaint.** The owner's verdict three separate
times has been *"the biggest differences are still size and colour"*. That is
true **between** species, because the species files differ in their turgor
ceilings and their palettes. **Within** a species it is backwards.

**And crown width is not reachable at all.** `width`'s positive control reads
**0.000 on all four reference genomes** — pushing the genome to both of its
extremes does not widen the spread of crown width beyond what a stand of
clones already shows. That matters more than it looks:
`plant-reseeding-2026-09-03.md` §1 identifies crown width as *"the one indirect
lever"* the genome has on seed dispersal, since a wider crown rains seed over a
wider footprint. It is measured here as a lever the genome cannot pull.

**On `tree` the same measurement is too weak to quote and that is worth
saying rather than omitting.** A 20,000-frame run establishes 23 plants, and
four of the seven variance ratios come out above 1.0 — including one at 6.7 —
which at that n is sampling noise rather than negative heritability. `tree`
reaches generation 1 in a run of this length (`FATE_MUTATION_CHANCE`'s own doc
records that its whole 0–0.30 ladder leaves a tree stand bit-identical for the
same reason), so the woody species need a horizon this harness has not been run
at. Only the `herb` table above is a result.

**How much the reference genome matters is itself the finding behind the
finding.** `ref=3` reads 0.376 on cells where the other three read ~0.03, which
means the *environmental* variance a genome experiences depends on the genome —
some genomes are developmentally stable and some are not. Any future claim of
the form "this change made plants more consistent" has to be made against
several reference genomes or it is a claim about one of them.

### 2.3 Where the scatter comes from

`plant.rs`'s growth draws come from `rng::stream(organism_id, cx, cy, frame)`.
Two things follow that a player would not guess: **a plant one column over is a
different plant**, because the cell coordinate is in the key, and **a plant that
germinated into organism slot 7 rather than 6 is a different plant again**,
because the id is too.

`clone_variance -- shift=1` measures the first of those on its own. One
founder, alone, with a single reference genome written onto every run, in a bed
made two columns wider each time so the plant stands one column further right —
no neighbours, no competition, no genetic difference, **and the same organism
id every run**, since each run builds a fresh world and plants one thing.
Twelve positions, `herb`, 12,000 frames:

| | range over twelve positions | coefficient of variation |
|---|---|---|
| cells | **83 → 181** | 0.280 |
| height | **27 → 63 rows** | 0.260 |
| width | 10 → 31 | 0.358 |
| **foliage share** | 0.387 → 0.495 | **0.075** |
| root share | 0.081 → 0.278 | 0.319 |
| foliage centre | 0.536 → 0.719 | 0.090 |

**Twelve genetically identical plants, alone in identical beds, come out
anywhere between 83 and 181 cells and between 27 and 63 rows tall.** That is
the floor under every plant comparison this project has published. And it
carries §2.2's split in miniature, from a completely different direction:
**size varies by 28% and composition by 7.5%** — the same four-fold gap between
what position does to a plant's size and what it does to its proportions.

Review card `20260903T060947894Z-4646f4` renders those twelve side by side.

**The organism-id term is a *hypothesis* here and not a result, and an earlier
draft of this report had it the other way round.** The arm above holds the id
fixed by construction, so it attributes the whole 0.28 to position; whether the
id adds anything on top is unmeasured, and the arm that would measure it —
same coordinate, different slot — needs a way to advance the organism counter
without putting another plant in the bed. Since position alone already produces
that spread, the id is not the lever it looked like.

### 2.4 What this means for everything else on the plant line

Three consequences, in descending order of how much they should change what
gets built next.

1. **An architectural lever can fire, be counted, and still be invisible —
   for a reason that has nothing to do with what it does.** If H2 on the
   descriptor it moves is 0.2, then four fifths of what the eye sees on a
   contact sheet is noise. `plant-appearance-design.md` diagnosed the
   sympody/tropism/acrotony round as *a lever that relabels a cell cannot move
   a silhouette that texture and colour set*; this is a second, independent
   mechanism for the same outcome, and it was never on the list.
2. **A selection experiment in this bed is fighting the same floor.**
   `selection_arena`'s own finding is that a null there is a statement about
   the world; this says how big the statement has to be before it can be
   heard.
3. **Halving developmental variance does the same thing to the signal-to-noise
   ratio as doubling genetic variance**, and nobody has tried the first. §7
   item 2 says what that would take, and it is not the one-line change it
   looked like before §2.3 was measured properly.

---

## 3. The stale verdict, re-tested

The brief asks for one five-minute check: does foliage share still read ~5%?
`plant_probe`, 6 founders, 20,000 frames:

| species | foliage share (% of cells that are leaf, median) |
|---|---|
| creeper | **50** |
| tree | **43** |
| conifer | **42** |
| shrub | **41** |
| herb | **37** |
| scrambler | **31** |
| grass | **0** |

`plant-appearance-design.md`'s diagnosis rests on two stated causes: *every
species was ~90% wood and ~5% leaf*, and *every plant drew from one four-brown
palette and one four-green one*. **The first is stale by an order of
magnitude.** `LOCUS_LEAF_ECONOMY` became a real gene carrying its own foliage
bands, bark bands derive from wood density, and `cell_scale` doubled the
world's resolution; composition moved with them.

**But the second half of the same table is the new problem.** Six of the seven
species now sit between 31% and 50% leaf — they agree on composition to within
a factor of 1.6, where they used to agree at ~5%. Composition has stopped being
the reason the levers were invisible and has not started being a *difference*
between species. And `grass` at **0** is its own finding: it owns no
`CellType::Leaf`, so its entire carbon readout and every composition descriptor
is structurally zero for it — which is worth knowing before anyone measures
`grass` against anything.

---

## 4. What actually makes plants different — and what this engine can say

*The owner's other question: what features make plants in the real world unique
from one another, and interesting.*

Ranked by what a person notices at this pixel scale, with what the engine can
express beside it.

| what separates real plants | can the engine say it? |
|---|---|
| **Silhouette / architecture** — spire, vase, dome, fan, rosette, mat, climber, weeper. Hallé's 23 models are enumerated by categorical choices, not scalars | **Partly.** Sympody, tropism and acrotony are live heritable loci and all three measured invisible |
| **The leaf** — size, shape (needle / blade / lobed / compound), arrangement (opposite / alternate / whorled), density, gloss, evergreen or deciduous | **Barely.** A leaf is **one cell**. `leaf_cluster` sets how many cells a node places and they have no shape. There is no phyllotaxy |
| **Scale and proportion** — height:width, trunk taper, internode length ("leggy" against "compact") | **Yes** — and §2.2 says proportion is the *most* heritable thing here while size is the least |
| **Bark and stem** — smooth, fissured, peeling, white / red / green | **Partly** — bands derive from wood density |
| **Reproductive display** — flower size, colour, count, and *where they sit*: terminal spike, axillary, catkin, umbel | **Yes, newly** — organs ship, and `organ_cluster` sizes the head |
| **Habit relative to ground** — upright, prostrate, climbing, rosette; and **whether it reads as one individual or a colony** | **Upright/prostrate yes. Colony no** — §1.5: no plant may put a node underground |
| **Seasonality** — deciduousness, autumn colour, flowering season | **No.** The clock has a day, not a year |
| **Age and damage character** — a broken leader, a lean, a burl, epicormic sprouting | **Partly** |

**The one that is both cheap and unbuilt is leaf-cluster shape.** A leaf is one
cell and cannot have a shape; a *cluster* of 8 to 20 cells can, and `herb`
already places 8 and `tree` places several. Today `leaf_cluster` is a count and
the cells go wherever the placement rule puts them. Giving a cluster a *form* —
a blob, a line along the axis, a fan, a whorl at the node — is the difference
between an oak and a pine at a glance, and unlike sympody or tropism it is
**ink rather than a label**: it moves which pixels are green and how they are
massed, which is the axis `plant-appearance-design.md` found was never
parameterised.

Second cheapest, and it is now one mutation away rather than one edit away:
**clonal spread**. A `RootTip` fate can already set `child: GrowingTip` and put
a shoot up from underground; what stops it is that every root authors
`plastochron: [0]`, so no node ever fires. §1.5.

---

## 4a. Clonal spread is already reachable, and nobody knew

§1.5 says root `plastochron` is `0` in every shipped species and no genome
slot addresses it, so no plant may put a node underground. That is true and it
is not the whole story, and finding out took three passes.

**The experiment.** `genome_reach -- rhizome=1` registers a runtime variant of
`herb` — root `plastochron: [3]` plus a `RootTip` fate whose `lateral` is a
`GrowingTip` — beside the shipped species, and runs both. No shipped file is
touched.

**Two censuses failed before a counter worked, and both failures are the
useful part.** Counting shoot tissue below the ground line reads **1 / 3 / 2 in
the *unmodified* species** over three world seeds, against 2 / 2 / 2 treated —
a clean null, from a discriminator that does not discriminate: a plant whose
collar is buried by a cell of moving soil looks exactly like a shoot that came
up from below. Restricting to four or more rows down does not fix it either;
the control reaches **nine rows** on one seed. `CLAUDE.md`'s *"did it fire at
all" needs a counter, not a picture*, and this is the third occurrence in the
register.

`World::root_shoots_launched` is the counter — a shoot launched off a
`RootTip`, at the one site where a lateral is created. It costs one comparison
on a path that already branches on `is_organ`.

| `herb`, 4 founders, 20,000 frames | ws 1 | ws 2 | ws 3 |
|---|---|---|---|
| control, shipped species | **0** | **4** | **0** |
| control, `FATE_MUTATION_CHANCE=0` | 0 | 0 | 0 |
| root node + lateral shoot | **15** | **13** | **29** |

**The middle row is the finding.** The control is *not* zero, and turning the
fate mutation off is what shows why: no root ever reaches a `Node` fate, but
`FateOp::Retarget` can point the root's **`Grew`** rule's `lateral` at a
`GrowingTip` — and then every root growth step launches a shoot. **So a `herb`
lineage can already discover a clonal growth form nobody authored, in the
shipped game, today.** Rarely — four events in one run of three — but not
never, and that is the owner's *"a flexible system that will allow variety to
evolve"* working rather than being proposed.

Three things follow.

- **The fate genome is doing more than its own reports claim.** It is
  documented as able to acquire a rule a species never had; this is a measured
  case of it acquiring a *growth form* nobody has ever authored in any species
  file, and it was found by accident while controlling something else.
- **The species-file lever is a multiplier on it, not the enabler.** A root
  `plastochron` takes the same event from ~1 per run to 13–29, and
  `ParamGenome` is what makes that number reachable by mutation rather than
  only by an author.
- **Whether any of those shoots becomes a *plant* was not measured when this
  section was written. It is now, and the answer is no.** §4b.

## 4b. The sucker is launched and nothing comes of it

The instrument §4a asked for, built and run. **`emergent_clumps`** counts each
organism's 8-connected **above-ground** clumps of shoot tissue and calls a
clump a *stem* only if some cell of it is 8-adjacent to a cell of the same
organism at or below the ground line. One plant, one collar, is one stem
however deeply that collar is buried — which is what makes this immune to the
burial trap that killed §4a's first two discriminators. A sucker that surfaced
somewhere new is a *second* stem: joined to its parent only underground,
through root tissue the walk does not cross.

`herb`, 4 founders, 20,000 frames, root `plastochron: [2]` on the treated arm,
eight world seeds:

| | shoots launched off a root | plants standing | second stems | mean width |
|---|---|---|---|---|
| control, shipped species | **4** | 358 | **3** | 12.84 |
| root node + lateral shoot | **144** | 278 | **5** | 12.04 |

**144 launches produce nothing.** Five second stems against the shipped
species' three is not a difference at this count — and the control settles it
without any arithmetic, because **all three of its second stems occur on seeds
where the launch counter reads zero**. Whatever produces a second above-ground
clump in this bed, it is not a sucker; a plant's base can be split by a grain
of surface soil, and that is the background this measures against.

**Width is the number that closes the one loophole**, and it was added for
exactly that. A sucker surfacing *inside* its parent's crown is adjacent to the
crown's own tissue, merges into that component, and cannot raise the stem
count. Width cannot be fooled that way: anything that surfaces where the crown
does not already reach makes the plant wider. The treated arm is **narrower**,
12.04 against 12.84. So the shoots are not merging into the crown either.
Nothing surfaced.

**And the growth form is not free.** The treated stand carries 278 standing
plants against 358 (**-22%**) and 26,241 organism cells against 37,499
(**-30%**). A root that stops to build a node every two steps is a root that
did not grow, and the lateral it builds returns nothing.

**Both controls ran on every one of the sixteen runs.** The positive control
grafts a column of the plant's own shoot tissue out of one of its own
below-ground cells — a sucker built by hand — and the stem count **must** move
by exactly one; it did, 16 times out of 16. Its first two versions did not work
and both failures are the useful part:

- **An unanchored stamp is not a control.** Stamping a clump into open air
  proved only that the census can count a component, which the census
  specifically declines as debris. *A control the mechanism under test is
  designed to reject proves nothing about the mechanism.*
- **A control that hunts for a clear site is skipped exactly where it matters.**
  Requiring a column already free of tissue found no site at all on the densest
  bed of nine — 47 plants, no root with five clear rows above it — and printed
  `None`, which reads identically to a census with nothing to find. It now
  *clears* three columns and re-takes its own baseline afterwards, so whatever
  the clear removed is in both terms of the difference and cancels. The denser
  the world, the more certain the old version failed, which is precisely
  backwards.

**What this changes.** §4a stands as written about the *event*: a `herb`
lineage can discover a shoot off a root in the shipped game, and that is the
fate genome acquiring a growth form nobody authored. What it cannot claim, and
an earlier draft of this report implied, is that the lineage thereby discovers
**clonal spread**. The form is reachable and inert — reachability is not the
binding constraint, and this is the same shape as `CLAUDE.md`'s channel with a
reader and no writer: the genome can say *put a shoot here* and the engine
gives that shoot nothing to do.

**Why it dies is not determined, and the reason it is not determined is worth
recording**: `CellType::MatureBody` is shared between root and shoot, and
`plant::organ_material` gives a distinct material only to `Flower` and `Fruit`
— so once a below-ground shoot matures it is indistinguishable, by cell type
*and* by material, from the root it came off. A sucker can be followed only
while it is still a `GrowingTip`. Any future work on underground shoots needs
that distinction to exist before it can be measured.

## 5. What was built: `organism::ParamGenome`

### 5.1 The shape

An individual carries a small set of **overrides** on its species' behaviour
table: `(CellType, ParamId, tier, f32)`, capped at `MAX_PARAM_OVERRIDES = 8`.
Founders carry none. A bred seed inherits its parent's set whole and takes at
most one point mutation.

It is deliberately the **`FateGenome`'s shape applied to the numbers**. That is
the engine's existing existence proof — the only channel through which a
lineage can acquire something its species never had — and copying its shape
means copying its guarantees: founded from the species file, inherited whole,
point-mutated, empty means "use the species file exactly".

**An override replaces the authored value rather than scaling it**, which is
the whole point. A multiplier cannot leave zero; a replacement starts *at* the
authored value and walks. `an_authored_zero` is guarded by
`a_parameter_mutation_can_leave_an_authored_zero`, which was watched going red
with the additive step swapped back to a multiplicative one.

**The tier is carried rather than flattened**, and that was a correction to the
first design. Several `Grow` fields are `ByOrder` — `tree`'s
`branch_chance: [0.03, 0.12, 0.2, 0.25]` is an order-graded profile — and an
override that replaced the whole profile with one number would make every
mutation to a tiered field *also* a decision to throw the grading away. Two
changes wearing one operator, and it would have made the mutation look far more
destructive than it is.

### 5.2 Where the units come from, and why that is not a hardcode

A mutation needs a step size, and a step size needs a scale. Taking it from the
species being mutated reproduces the cage one level up: every shipped root
authors `plastochron: [0]`, so a root-local scale is zero and no mutation could
ever move it.

`SpeciesRegistry::param_scale` takes it **from the corpus** — the largest
magnitude any species in the registry authors for that parameter, across every
cell type and tier. The shoot's `plastochron: 14` is the engine's own statement
of what a plastochron is worth, so a mutation on the root's node spacing is
drawn on the same scale as the shoot's. That is what makes rhizomes reachable,
and it is guarded by `the_corpus_scale_crosses_cell_types`.

Bounds come from a **kind**, not a table: `Probability` is `[0,1]`, `Weight` is
`+/- PARAM_REACH x scale` (`scrambler` authors `acrotony: -1.4`, so weights must
be two-sided), `Magnitude` is `[0, reach]`, `Divisor` has a floor because
`turgor_per_cell` divides into the height ceiling and a zero there is an
unbounded plant, and `Count` rounds. **The line between a bound and a cage is
that a kind cannot collapse to a point**, which is asserted for every parameter
by `no_parameter_kind_collapses_to_a_point`. The only two free numbers in the
whole mechanism are `PARAM_REACH = 4.0` — how far outside the authored corpus a
lineage may go — and `PARAM_DIVISOR_FLOOR = 1/64`.

### 5.3 What it cost, and what it did not

**One patch point, and finding it is why the addressable set could be the whole
table rather than a handful of fields.** `plant::organism_tick` copies each
`Behavior` out of the registry into a fixed dispatch buffer once per organism
cell per tick; patching that copy reaches every consumer at once. There are two
such fills (the frontier pass and the mature-tissue pass) and both apply. An
empty genome costs a length check, which is every founder.

**The dozen sites that read a behaviour *outside* those buffers all had to be
routed too**, and that is a correctness constraint rather than tidiness: a site
reading `world.species.get(id).behaviors(ct)` directly would give one plant two
different values for one number — its own on the frontier and its species' in
whatever pass read around the buffer. `plant::individual_behavior` is the one
way in, and its doc says so.

**The specimen shelf keeps it.** A jar stored `draws`, `alleles` and `fates`;
without `params` it would hand the player back a plant that is not the one they
kept, silently, because the released specimen still looks like its species.
`#[serde(default)]`, so every jar written before this field existed still loads.

**The full lib suite is 1338 passed / 0 failed** with the mechanism in and its
rate at zero, which is the guard that matters: at rate 0 the engine is
bit-identical to before it existed. Founders carry no overrides, the mutation
roll comes from a keyed substream so the caller's `Rng` position does not move
(the failure `set_seed_leaves_the_callers_rng_position_alone` exists for), and
an empty genome applies nothing.

### 5.4 Why the rate ships at zero

Not caution about the code. A specific, named, measurable risk.

`plant-heritability-survey-design-2026-08-27.md` §2 states it: **a free lever
made heritable produces uniformity, not diversity** — a quantity with a benefit
and no counterweight has exactly one optimum, which a working economy finds and
holds every plant at, and the visible result is one morphology everywhere. Its
§4a then inventories nine parameters as free *today*: `turgor_source`,
`turgor_yield`, `plastochron`, `heading_inertia`, `juvenile_size`,
`juvenile_plastochron`, `seed_maturity`, the `Photosynthesize` rate,
`branch_angle` on the scoring side. This mechanism makes all forty-three
heritable at once, free ones included.

**So the honest thing is to measure it, and the measurement does not fit in one
night.** `genome_reach -- drift=1` is the instrument — it censuses the standing
population's override tables and reports how many sit at their `clamp_param`
bound, which is the free-lever signature. Run on `herb`, 8 founders, at rate
0.3:

| | 20,000 frames | 60,000 frames |
|---|---|---|
| live organisms | 1,615 | 1,664 |
| carrying at least one override | 782 | 825 |
| rolls / applied | 2,921 / 681 | 11,745 / 2,792 |
| overrides per individual | {0: 833, 1: 626, 2: 146, 3: 10} | {0: 839, 1: 657, 2: 151, **3: 17**} |
| addresses at their bound | ~0, and `juvenile_size` | ~0, and `juvenile_size` |

**The prediction is neither confirmed nor refuted, and the reason is the
interesting part: the pedigree is too shallow to test it.** Quadrupling the run
length moved the deepest individual from 3 overrides to 3, and the population's
depth histogram barely moved. `herb`'s mean generation in this bed is ~2.3
(`FATE_MUTATION_CHANCE`'s own doc records the same arithmetic), so a lineage
cannot accumulate the *coordinated set* of overrides a degenerate optimum would
need. The one address that does pile up at a bound is `juvenile_size`, driven
to zero — a lineage discarding its juvenile stage, which is exactly the free
lever the inventory predicted.

**Turning it on does not break the stand, which is a different question and is
also answered.** `herb`, 8 founders, 30,000 frames, three world seeds, paired
at rate 0 against rate 0.3 — the same binary, the rate from the environment:

| world seed | established, rate 0 | rate 0.3 | seeds set, rate 0 | rate 0.3 |
|---|---|---|---|---|
| 1 | 103 | **110** | 6,438 | 6,204 |
| 2 | 105 | 100 | 7,024 | **7,551** |
| 3 | 87 | 77 | 6,337 | 5,290 |
| **pooled** | 295 | 287 (−2.7%) | 19,799 | 19,045 (−3.8%) |

Two seeds down, one up, both within the spread this bed produces from world
seed alone. **This A/B is also the night's second stale-binary catch**: run
before `cargo build --release --examples`, the two rates produced six
byte-identical censuses, because `plant_probe` had never been rebuilt after the
mechanism landed and the arms were the same binary. The tell was the standing
one.

**What that means for the rate.** Turning it on today would not produce the
degeneracy, because nothing in this bed lives long enough. It would also not
produce much *evolution*, for the same reason. Both change the day M10
streaming or a deeper lab run raises generation depth — which is precisely
when this needs to have been measured. Shipping the dial at 0 with the
instrument beside it is the position that survives either answer.

One thing the drift table already shows that is worth reading now: at 60,000
frames a lineage carries `RootTip/stem_stiffness` at **-0.0495** against an
authored `0.0`, and `RootTip/crowding_weight` at a median **-5.15** against an
authored `0.0`. Those are lineages expressing a *sign* their species never
authored — a root that seeks crowding rather than avoiding it. That is the
mechanism doing the thing it was built for, on a parameter no genome slot has
ever addressed.

---

## 6. What was not built, and why

### 6.1 Dispersal — built after all, and inert until somebody turns it on

The brief says dispersal should be in scope whatever else is, and the first
draft of this report deferred it. It is built, because §1 turned out to make it
nearly free.

**`Behavior::Reproduce::seed_launch`** — how far a plant flings a seed
sideways, in cells. **Zero in every shipped species**, and the guard
`a_zero_seed_launch_moves_nothing` asserts both halves of that: the throw
function returns its input unchanged at a non-positive reach, *and* no species
file authors one. So the outdoor game is untouched and every stand measured
before it existed grows the same plant.

**What makes it worth building now rather than earlier is that it is an
ordinary `ParamId`**, so `ParamGenome` reaches it and a lineage can leave the
authored zero. That is exactly the cage §1.1 describes, and this is the first
new channel built on top of the fix: *a heritable dispersal distance, founded
at zero.* Under the multiplicative genome it could not have existed — a slot
multiplying an authored `0.0` is a slot that does nothing for ever, which is
why `plant-reseeding-2026-09-03.md` §1's own proposal had to include *"give the
species files a non-zero authored base so the multiplier has something to
scale"*. It no longer does.

**It is a distance, not a destination**, and that is the answer to the same
report's objection that the cheap form *"reads as magic"*. `plant::set_seed`
draws a displacement uniform on `[-reach, reach]` and **walks toward it one
cell at a time through open cells, stopping at the first thing in the way** —
so a seed is flung rather than teleported, it cannot cross a wall, and a plant
in a crevice disperses no further than the crevice.
`a_flung_seed_cannot_cross_a_wall` is the guard, with its own positive control
(remove the walls and the same throws must travel). A symmetric draw is also
the ethos's first law: most seed still lands near the parent and a few go a
long way.

**What it buys**, `herb`, 8 founders, no ants, 27,000 frames, three world
seeds, on the evenly-lit bench:

| reach | columns of 512 held | plants ≥16 cells from a founder column | established |
|---|---|---|---|
| **0** (shipped) | 398 / 454 / 396 | 66 / 69 / 61 | 105 / 106 / 93 |
| **4** | 457 / 479 / 415 | 64 / 82 / 72 | 106 / 115 / 107 |
| **12** | 500 / 465 / 481 | **91 / 92 / 88** | 133 / 110 / 116 |

Pooled, reach 12 against 0: coverage **+16%**, established **+18%**, and the
number this is actually about — plants that reached ground more than fifteen
columns from anything anyone planted — **+38%, up on 3 of 3 seeds**. Reach 4
is up on 2 of 3 and is inside the spread.

> **The +38% is withdrawn: it was the free-lunch half of an unpriced lever.**
> §6.6 gives `seed_launch` the cost this section should have given it, and
> re-measures the same statistic on the same bed. Priced, the far-dispersal
> column reads **71 / 71 / 56 against 66 / 69 / 61 — flat**. What survives
> pricing is coverage (+12.5%) and establishment (+11%), both up on 3 of 3;
> distance does not. Read the table above as *what an unpriced launch buys*,
> which is not what ships.

For scale, `plant-reseeding-2026-09-03.md` §6.1 moved that same statistic
**4.1x** by lighting the bench evenly. So dispersal is real, it is worth having,
and it is still not the headline — which is what that report's `scatter=1`
positive control said before the mechanism existed.

**And the limitation the same measurement exposes, in the mechanism this
session built rather than in the one it inherited.** No species authors a
launch, so `param_scale` falls back to 1.0 and `clamp_param` bounds this
`Magnitude` at `PARAM_REACH * 1.0` = **4 cells** — and the table above puts
reach 4 *inside the spread* while reach 12 is the +38%. **So the channel is
heritable and its evolvable range is currently below its useful range.** A
lineage can discover a seed throw and cannot discover a useful one.

That is a property of the corpus-scale design, not of seeds, and it generalises:
**a brand-new parameter that no species has ever authored is evolvable only
within four units of zero, in whatever units it happens to have.** The remedy
is authoring rather than patching — one species with a non-zero value raises
the corpus scale and widens the range for every lineage at once, so the owner's
dial and what evolution can reach move together. Writing a `4.0` into a table
for this one parameter would buy the same range and give the mechanism back the
hardcode it exists to remove.

**Cost.** At the shipped zero it is one float comparison per seed borne;
`launch_offset` returns before drawing anything. The per-frame figures in the
sweep (4.33 / 3.82 / 3.74 ms) are not a cost comparison and should not be read
as one — the arms grow different amounts of plant, which is `CLAUDE.md`'s *a
cost that appears may be biomass that appeared* seen from the other side.

**And the harness seam it needed is worth more than the arm.**
`SpeciesRegistry::set_param` writes one authored parameter by `ParamId` into
the live registry. Every parameter sweep in this repo has until now had to edit
a `.ron` and rebuild, which is the `include_str!` trap `CLAUDE.md` records
producing *whole invalid sweeps*: identical output across settings because the
prebuilt binary never read the file. A sweep that goes through `set_param`
cannot go stale. `reseed_probe -- launch=N` is the first user, and it refuses
outright if the write matches nothing.

**What is still missing**, and the brief's own list is right about it: this is
the *cheapest* of the three mechanisms, not the most satisfying. It gives the
plant a dispersal trait; it does not give the **player a verb**. Wind on light
powders would, and `dead-ends.md` carries the warning that matters for it — a
steady global wind was built, measured and reverted (settled-field cost 0.0002
→ 3.55 ms on every scene, because a uniform velocity in a bounded world pushes
air into the walls and `field::is_converged` never returns true again). Gusts
are the recorded replacement, so wind-borne seed would be **bursty** by
construction. That is a good property and it needs saying up front rather than
discovering.

### 6.2 Speciation

`individual_as_species` refuses because it copies the parent *species'* fates
rather than the individual's. With `ParamGenome` landed, "promote this
individual to a species" now has a second thing to copy, and both are on
`OrganismState` and both round-trip through the shelf — so the promotion is
now a matter of copying three genomes into a `SpeciesDef` rather than of
inventing a representation. It is still not the harder half, which is that a
`SpeciesId` is copied to offspring unchanged and every consumer of it assumes
species are a fixed set.

### 6.3 Leaf-cluster shape — built, and the check that came before it

§4 called this the cheapest unbuilt lever. It is built:
`Behavior::Grow::leaf_spread`.

**The check first, because this line has a record.**
`plant-appearance-design.md` describes three levers that fired, were counted,
and were invisible, and `CLAUDE.md` turns that into a rule: *ask which pixels a
lever moves, before ranking it by silhouette*. So before writing anything, what
does `leaf_cluster` actually do?

`plant.rs` places the first leaf behind the apex and then grows the rest as a
**breadth-first walk off it that picks one open 8-neighbour at random** at every
step. So a cluster is a contiguous blob of up to `leaf_cluster` cells — and its
**shape is drawn fresh from the RNG at every node of every plant**. Nothing has
ever controlled it.

That answers the question in the right direction. Giving the walk a form
changes **which cells are green**, at every node, with the same number of cells
placed. It relabels nothing, which is precisely what the three invisible levers
did.

**And it has a second payoff that only §2 makes visible.** Cluster shape was
one of the per-position draws behind the heritability result: foliage
arrangement was noise. As a `ParamId` it is heritable, so the axis moves out of
the noise column and into the genome — which is the only lever on this list
that does that.

`leaf_spread` is the probability that each step of the walk takes the
best-aligned candidate rather than a uniform one, so the outcome is graded
rather than a switch between two shapes. Measured on `herb`, 10 founders,
14,000 frames, one bed:

| | leaf cells | clusters of 3+ | mean elongation |
|---|---|---|---|
| `leaf_spread: 0` (shipped) | 3,203 | 229 | **0.634** |
| `leaf_spread: 1` | 3,389 | 226 | **0.769** |

Elongation **+21%** with leaf-cell count within 6% — the arrangement moved and
the amount did not. `a_spread_leaf_cluster_is_longer_than_a_blob` is the guard,
and its own first run is worth recording: at 1,200 frames the scene grew **one**
cluster of three or more cells, so the harness could not answer its question.
Fixed in the scene (6,000 frames) rather than in the bar.

**Whether it is *visible* is not settled here and must not be**, because that is
the exact claim this line keeps getting wrong. It is a blind A/B in the review
queue (`20260903T120950045Z-4f14bd`), asked so that *"these look the same"* is
an available answer — and if that is the answer, the lever should be retired
rather than kept as a knob nobody can see.

**Two properties worth carrying.** It is a `ParamKind::Probability`, so it
**sidesteps §6.1's empty-corpus limitation** outright — a probability's bound is
`[0, 1]` whatever the corpus says, so unlike `seed_launch` it is fully reachable
from the day it lands. That is the kind system doing its job, and it is an
argument for preferring a probability-shaped parameter when a new channel has a
choice. And the **zero guard is not an optimisation**: the spread draw sits
inside the growth walk itself, so one extra `chance` at zero would shift that
stream and make every plant in both games a different plant, silently.
`a_zero_leaf_spread_takes_no_draw` asserts both halves — zero is bit-identical,
and 1.0 is not.

### 6.4 Seasons and sex

Named in §4 and not started.

---

### 6.5 The dispersal verb — measured, designed, not built

**Everything below is the answer to *can this step demonstrate itself*, asked
before building anything** (`CLAUDE.md`). The phase that preceded this report
spent itself on three architectural levers that all fired perfectly and moved
nothing anyone could see, so the guard is to check the channel a proposed
mechanism would read *before* writing the mechanism.

**Is there wind where a seed falls?** `wind_probe -- velocity=4000` censuses
`field_at(x, y).vx` over the rows above **each column's own surface**, so the
terrain decides where the band is rather than a fixed row. On `rolling`, three
seeds, against the gas rule's own `WIND_BIAS_THRESHOLD` (0.01) rather than a
new number:

| | over threshold | max \|vx\| | p50 | p99 |
|---|---|---|---|---|
| GALE seed 1 | 919/1920 (**47.9%**) | 1.756 | 0.0046 | 1.683 |
| GALE seed 3 | 686/1920 (**35.7%**) | 1.697 | 0.0000 | 1.571 |
| GALE seed 7 | 947/1920 (**49.3%**) | 2.867 | 0.0059 | 2.824 |
| CLEAR — control | 0/1920 (**0.0%**) | 0.000 | 0.000 | 0.000 |

Three readings, and the third is the one that shapes the design:

- **The wind is real down there.** A third to a half of the cells a seed passes
  through are windy enough for the threshold its consumer would use.
- **It never reaches full strength.** Nothing hits `WIND_BIAS_FULL_SPEED`
  (4.0), so a mechanism scaled to that ramp runs permanently in its lower half.
  Either scale to the measured range or accept a weak effect deliberately.
- **It is concentrated, not ambient** — median ~0.005 against a p99 of 1.5–2.8.
  A channel that is quiet almost everywhere and strong in a few places
  disperses seeds *in a few places*, which is what dispersal wants and is a
  better answer than a uniform breeze would have been.

The `Pin::Clear` arm reads **exactly 0.0000** on every seed and is not
decoration: `field::SETTLE_EPSILON_VELOCITY` means still air settles *near*
rather than at zero — which is why `WIND_BIAS_THRESHOLD` exists at all — so
without a calm arm on the same terrain for the same frames, the GALE row cannot
be told from the solver's own residue.

**And the lever is the move ordering, not the coin.** This is the part that
would have been got wrong. `update_powder` picks `(first, second)` with a fair
coin, and that coin decides only the *diagonal*; the straight-down move is
tried first and always succeeds in open air. **So biasing the coin would be a
no-op for a seed in free fall** — the exact *which pixels does this lever move*
trap, and `update_gas`'s own `MAX_LEAN_CHANCE` comment says as much about its
own mechanism: *"without this the whole mechanism is a no-op in open air"*.

So the shape, when someone picks this up:

- A wind-borne powder tries the **downwind diagonal before straight down**,
  mirroring `wind_biased_order`'s `lean` exactly.
- Opted in by a `#[serde(default)]` field on `MaterialDef`, mirrored into the
  runtime `Material` and tested at `update_powder`'s dispatch site where the
  `Cell` is already in hand — `CLAUDE.md`'s *guard hot-path work at the call
  site*, and `clings_to_wood` is the precedent to copy. Every other powder in
  the world then pays one `Vec` index.
- **No RNG draw at all below threshold**, so a calm world stays bit-identical
  and that is the negative control: a world hash unchanged under `Pin::Clear`
  proves the change cannot touch a still day.
- **No `PREVAILING_DRIFT` for powders.** A constant would slide every seed one
  way for ever, and would destroy that bit-identical guarantee.

**Not attempted here.** It reaches every forest in the outdoor game, and the
frame-cost question — whether a wind-borne powder keeps chunks awake — has not
been measured.

## 6.8 The developmental seed, built as one dial

§2.3 measured the problem: growth draws come from
`rng::stream(organism_id, cell_x, cell_y, frame)`, so **a plant one column over
is a different plant**, and twelve genetically identical plants alone in
identical beds come out between 83 and 181 cells and 27 and 63 rows tall.

**One dial rather than two designs**, which is the owner's correction and it
matters: per-cell draws key on the plant's own frame *in every arm*, and the
arms differ only in how coarsely `dev_seed` carries the germination
coordinate. So an A/B differs by exactly one number and the answer can land
between the endpoints.

| `shared_development` | `DevelopmentalKey` | what it is |
|---|---|---|
| 0 | `World` | today: `(organism_id, cell_x, cell_y, frame)` |
| 1 | `Plant { coarseness: 0 }` | position dropped — a lineage has **one inherited form** |
| 2 | `Plant { coarseness: 1 }` | folded at full resolution — every plant its own **coherent** form |
| 3+ | `Plant { coarseness: k }` | plants within `k` columns share a form |

**Coarsening is applied to the germination coordinate once, before hashing,
never inside the per-cell key.** The latter is the block-nearest coarse-field
trap `CLAUDE.md` records hitting four times on three lines and never once
catching in a test. This is instead `seed_genotype`'s own idiom — position
captured once, per plant, at germination — applied to development rather than
to the genome draw.

### The acceptance test

`clone_variance -- shift=1`: one founder, alone, moved one column at a time,
with the reference genome **and reference lineage seed** written onto it.

| | CV cells | CV height | CV width |
|---|---|---|---|
| shipped | **0.280** | 0.260 | 0.358 |
| `dev=0` | **0.074** | 0.143 | 0.121 |
| `dev=1` | 0.374 | 0.252 | 0.542 |

Adjacent columns give 91 and 92 cells at `dev=0` where the shipped key gave 153
and 173. The residual 0.074 is environmental response — the bed widens by two
columns per step — which is the variation worth keeping.

**`dev=1` reads *higher* than the shipped key, and that is a finding rather
than a fault.** Per-cell noise partially cancels across a plant; per-plant
noise does not. Folding position once makes each plant coherent **and** makes
plants differ from one another more.

### Heritability, `herb`, 16 founders, 12,000 frames, four world seeds pooled

`H2 = 1 - Var(clone)/Var(pop)`, per reference genome:

| descriptor | shipped (median) | `dev=0` (median) | `dev=1` (median) |
|---|---|---|---|
| **cells** | **0.034** | **0.650** | 0.323 |
| height | 0.502 | 0.599 | 0.363 |
| **width** | **0.227** | **0.658** | 0.354 |
| slenderness | 0.208 | 0.729 | 0.532 |
| foliage share | 0.610 | 0.483 | 0.227 |
| root share | 0.359 | 0.175 | 0.354 |
| foliage centre | 0.328 | 0.465 | 0.145 |

Three readings:

- **Plant size becomes heritable**, 0.034 → 0.650. It was the least heritable
  thing the engine produced within a species.
- **So does crown width**, 0.227 → 0.658 — and §2.2 measured width's *positive
  control* at 0.000 on all four reference genomes, i.e. as a lever the genome
  could not pull at all. `plant-reseeding-2026-09-03.md` §1 calls crown width
  the genome's *"one indirect lever"* on seed dispersal.
- **Composition does not improve and may fall** — foliage share 0.610 → 0.483.
  That is coherent rather than contradictory: developmental noise was masking
  *size and shape*, not *composition*, because composition is an allocation
  ratio the genome sets directly while shape is where the per-cell dice landed.

**`dev=1` moves heritability substantially, which was predicted not to.** The
review's expectation was that folding position once would leave H2 untouched,
since germination position still fully selects which form a plant gets. It
reads ~0.32 on `cells` against the shipped 0.034 — less than `dev=0`, but not
nothing. The rendered comparison is still the primary instrument at that end;
the prediction about the number was simply wrong.

### And a withdrawal, caught by a control that was already there

A first `dev=0` table was measured and is **void**. The estimator's sensitivity
control read **0.000 on every descriptor**, where the shipped key reads
0.44–0.82 — the row whose whole job is to say *these numbers mean nothing*.

A founder has no lineage seed until something draws one, because
`PlantScene::build` never calls `seed_genotype` (§2.1). So the Spread arm read
`0` off every ungerminated founder and wrote `0` onto all sixteen, making the
**widest-genetic-contrast arm developmentally uniform** and collapsing
`Var(spread)`, the denominator of every H2 in the table. The Clone arm already
had the fix and a comment saying why; its sibling did not.

**It is the `ref=` failure repeating in the same file**, and the difference
worth carrying is that a *control* caught it rather than a byte-identical
output. With the fix the control returns to 0.612 on `cells`.

---

## 6.6 Pricing `seed_launch`, and what the price took away

**§6.1 built a tenth free lever while §5.4 was stating the law against exactly
that**, and neither the report nor its author noticed. Caught in review, and
the owner reached it independently: *"this seems like a lever that is only a
benefit. If there is no trade off then everything will evolve towards a single
maximum."*

`launch_price(reach)` scales what the **parent** is charged:

```
charge = seed_cost * (1 + 0.25 * sqrt(reach))
```

No new economy. `seed_cost` already sets how many seeds a plant can afford
(`budget / cost`), so scaling it by reach *is* the dispersal/fecundity
trade-off — throw far and set fewer. It is `TRANSPIRATION_PER_RATE`'s pattern,
a price **derived** from the lever so tuning cannot decouple them, rather than
a second authored number somebody can set to zero.

`herb`, 8 founders, 27,000 frames, three world seeds:

| | seeds set | columns held | plants ≥16 cols out | established |
|---|---|---|---|---|
| launch 0 | 3,203 / 4,237 / 2,947 | 398 / 454 / 396 | 66 / 69 / 61 | 105 / 106 / 93 |
| launch 12 | 2,115 / 2,573 / 1,731 | 483 / 480 / 442 | 71 / 71 / 56 | 110 / 116 / 110 |

Pooled: **seeds −38%** (down 3 of 3), **coverage +12.5%** (up 3 of 3),
**established +11%** (up 3 of 3).

**And the price ate the headline, which is the finding.** §6.1 published +38%
on far-dispersal for an unpriced reach of 12. Priced, that column is flat. So
the number this report led its dispersal section with was measuring a free
lunch; what survives is coverage and establishment, not distance. The lever is
still worth having in this bed — it is now a trade rather than a dominant
strategy, which is what the law asks for.

**Three traps, each of which breaks it silently:**

- **Square root, not linear.** `REPRODUCTIVE_BUDGET_CAP` is `RESOURCE_SCALE`
  (4.0), and a charge above it makes `budget >= charge` unsatisfiable —
  **permanent, silent sterility**, which is the failure
  `a_lit_sward_funds_a_reproductive_budget` already documents for `grass`. A
  concave price also matches the mechanism, since `launch_offset` walks and
  stops at the first obstruction, so realised distance grows more slowly than
  requested reach.
- **`seed_cost` is also the child's endowment.** The parent pays `charge`, the
  child is handed the unscaled `seed_cost` — otherwise a far-flung seed
  germinates *richer* and the cost pays itself back.
- **Exactly 1.0 at reach 0**, which every shipped species authors, so both
  games are untouched.

The fruit/windfall path reaches `bear_seed_at` directly and stays unpriced: a
fruit lets go where it hangs.

**§5.4's free-lever inventory was stale and is corrected here.** It listed
nine; three are no longer free. `Photosynthesize.rate` is priced by
`TRANSPIRATION_PER_RATE`, built expressly to kill that free lever, and
`plastochron` / `juvenile_plastochron` are partly priced now that leaves cost
carbon at `LEAF_CONSTRUCTION_MULTIPLE` and maintenance for ever. **Genuinely
free today: `turgor_source`, `turgor_yield`, `heading_inertia`,
`seed_maturity`, and `branch_angle`** — whose wide path takes its own
candidate set at uniform score and so bypasses the light and crowding filters
entirely, which is the bypass rather than a weighting.

---

## 6.7 Generations per hour, and the clock that can measure it

**The instrument had to come first, because every existing readout answers a
different question.** `examples/selection_arena.rs` records it in its own
output: over 150,000 frames the population's mean generation rose to ~2.9 and
then **fell back** — 2.88, 2.85, 2.77, 2.73, 2.63, 2.60 — and it prints
`*** THE GENERATION AXIS IS SATURATED ***` when the span is under 3.0. Nothing
is wrong with the world when that happens. **Mean generation is taken over
*living* organisms, and at steady state deaths balance births, so it
equilibrates rather than accumulating.** Every generation readout in this repo
is a max or a mean over the living, so all of them do this, and "did this
change make lineages deeper?" was unanswerable.

`World::deepest_generation` is a high-water mark updated at `bear_seed_at`,
the one place a generation is ever created. `World::generation_clock` returns
it with cumulative births and the standing count.

**It caught its own artifact on the first run.** `tree`'s deepest *living*
generation is 1 and its deepest *ever* is 2; `scrambler` reads 4 living
against 5 ever. The deep lineage died, and both numbers would have been
reported as the depth.

`plant_probe`, 8 founders, 30,000 frames, world seed 1:

| arm | deepest ever | frames per generation | births | standing |
|---|---|---|---|---|
| `tree` — the default bed | 2 | 15,000 | 968 | 283 |
| `herb` | 5 | 6,000 | 6,474 | 2,381 |
| `scrambler` | 5 | 6,000 | 5,443 | 1,491 |
| `herb`, `seed_maturity` 20 | **8** | **3,750** | 6,815 | 2,523 |
| `herb`, maturity 20 + hazard 0.02 | 8 | 3,750 | 5,987 | 2,100 |
| `herb`, hazard 0.02 | 5 | 6,000 | 5,765 | 1,949 |

**Two levers, and together they are 4x.** Species is 2.5x — and the default
bed is `tree`, so anything measured on `PlantScene::default()` has been paying
that. `seed_maturity` 60 → 20 is a further 1.6x and costs nothing: births and
standing biomass are both *up* slightly. `lab::params` already called it *"the
single biggest lever on whether a generation ever turns over"*.

**Hazard is a clean negative and is worth recording as one.** Age-neutral
mortality adds **nothing** to depth (5 against 5, and 8 against 8 on top of
the maturity change) while costing 11% of the births and 18% of the standing
plants. That follows from its own design — it is deliberately independent of
age, size and genotype, so it removes recruits as readily as adults — but the
intuition that "more death means more turnover means deeper lineages" is
wrong here, and it would have been an easy thing to assume.

**What is not built, and is the owner's call.** `examples/genome_drift.rs`
records the deep cause: *nothing in the engine kills a healthy adult*, so once
a stand closes the founding cohort **is** the population. There is no age
gate, no lifespan, and reproduction reads no age anywhere. Engine-side adult
mortality would reach every forest in the outdoor game, so it is a decision
rather than an unattended change — but the bed-side levers above are worth
4x before it is needed.

---

## 7. What to do next, in the order the evidence supports

1. **Post §2's result to the owner and get a verdict on the noise floor.** It
   is the finding that reorders everything else, and it is one number.
2. ~~**Make development heritable**~~ — **built, §6.8, and the acceptance test
   passed.** `DevelopmentalKey` on the world, one dial, default unchanged. What
   is left is the owner's verdict on which end of the dial ships.

   *The original entry follows, because its caveats still hold.*

   **Make development heritable, which is the real form of "attack the noise
   floor".** §2.3 measures the cause: growth is drawn from
   `rng::stream(organism_id, cx, cy, frame)`, so a plant's whole development is
   a function of **where in the world it is standing**. Two clones therefore
   cannot develop alike, ever, however identical their genomes and their
   surroundings — which is why a clone stand is as varied as a mixed one.

   The change that would fix it is to key the stream on the plant's **own**
   frame rather than the world's: `(developmental_seed, x - collar_x,
   y - collar_y, frame - germination_frame)`, with `developmental_seed`
   **inherited** like any other gene. Then two plants carrying the same genome
   and the same seed grow the same shape wherever they stand, differing only by
   what the environment actually does to them — and a lineage's characteristic
   form becomes something selection can act on instead of something re-rolled
   every generation. That is the whole of *"a flexible system that will allow
   variety to evolve"* applied to the half of the variety that is currently not
   inherited at all.

   **It is not a one-line change and it should not be attempted unattended**:
   it moves every plant in both games, it needs `collar_x`/`germination_frame`
   on `OrganismState`, and it needs the review card
   (`20260903T060947894Z-4646f4`) answered first — some of the present scatter
   is what makes a stand read as alive rather than stamped, and flattening it
   is a design decision rather than a repair.
3. ~~**Find out whether a sucker becomes a plant.**~~ — **done, §4b, and the
   answer is no.** 144 launches across eight worlds produce five second stems
   against the shipped species' three, and all three of *those* occur on seeds
   with zero launches. The stand is 22% smaller for it. What is left is a
   decision rather than a measurement: **either give an underground shoot a
   way to reach the surface, or stop treating clonal spread as reachable.** The
   first needs `MatureBody` to distinguish root from shoot before anything
   about it can be measured (§4b's last paragraph).
4. ~~**Leaf-cluster shape**~~ — **built, §6.3, and out for review.** What is
   left on it is the owner's verdict: if the two stands read the same, retire
   the lever rather than keep a knob nobody can see.
5. **Then the parameter-genome rate.** Two of its three prerequisites are now
   done: the free-lever list is corrected and `seed_launch` is priced (§6.6),
   and generation depth is 4x cheaper to reach than it was (§6.7). What is
   still owed is pricing the four remaining free levers — `turgor_source`,
   `turgor_yield`, `heading_inertia`, `seed_maturity` — and `seed_maturity` is
   the one to do first, because §6.7 makes it the generation lever and
   `plant.rs` says in as many words that it *"makes precocious reproduction
   unreachable rather than expensive"*. Pricing it and using it are the same
   work.
6. **Then the dispersal *verb*.** §6.1 built the trait; what it does not build
   is something the player turns on. Wind on light powders is the candidate,
   and `dead-ends.md`'s reverted steady-wind entry says it has to ride gusts.
   **The prerequisite is now measured and the design is settled — see §6.5.**
   It was not started, and the reason is that the session ran out rather than
   that anything blocks it.

---

## 8. Method notes worth carrying

- **A reachability census taken by reading one call site is a census of that
  call site.** §1.3. The arithmetic table and the widening arm disagreed and
  the arm was right.
- **A null in a widening arm at a short horizon is a statement about the
  scene.** §1.4. Five of eight live slots read `identical` at 4,000 frames and
  all eight moved at 16,000.
- **Identical output across settings is still the tell, and it fired twice in
  one night.** Once for a `ref=` argument that was selecting among genomes that
  were all the species mean (§2.1), and once for a `plant_probe` binary that
  had never been rebuilt after the mechanism landed, so a rate A/B compared a
  binary to itself and produced six byte-identical censuses. `cargo build
  --release --examples` is not optional after a lib change, and this is the
  fifth recorded occurrence of that exact failure.
- **A variance pooled across seeds is not the variance the question asks for.**
  §2.1: pool within seed and average, or the between-world difference sits in
  both arms and drags every ratio toward 1.
