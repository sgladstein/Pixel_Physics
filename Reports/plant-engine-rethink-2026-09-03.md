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
therefore **inert on every founder**. It replaces the authored number instead
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

**`moss` is not in the census and that is a finding rather than an omission.**
It declares one behaviour, `Divide`, and no `Grow` — so it has no
`genotype_variance` vector at all and **none of the ten continuous slots is
expressed for it**. Its whole continuous genome is drawn, inherited, mutated
and read by nothing.

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

Result, `herb`, 8 founders, 16,000 frames, three world seeds: every one of the
eight live slots moved the world on every seed, and `branch` — the caged one —
was **byte-identical on every seed**. On `tree` all eight live slots moved and
so did slot 1, which is what produced §1.3.

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
- **Whether any of those shoots becomes a *plant* is not measured.** The
  counter says the event fires; it says nothing about whether the sucker
  establishes, and the census that would answer that is the one this section
  just showed cannot be trusted in this bed. That is the next instrument.

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

### 6.1 Dispersal

The brief says it should be in scope whatever else is, and this session did not
build it. The reason is that §2's result changes what dispersal is worth
measuring against, and the honest ordering puts it after the noise floor: a
heritable dispersal distance whose H2 is 0.1 is a channel selection cannot use.

Two things were established for whoever takes it. `dead-ends.md` has **nothing**
on seed dispersal — grepped for `dispers` and `launch`, nine and six hits, all
liquids, advection and gusts. And the one entry that does bind is a warning
about the second candidate in the brief's list: **a steady global wind term was
built, measured and reverted** — a uniform velocity in a bounded world pushes
air into the walls, so `field::is_converged` never returns true again and the
settled-field cost went from 0.0002 ms to a permanent 3.55 ms on every scene.
Gusts, which are bounded impulses that disperse, are the recorded replacement.
So "wind on light powders" has to ride gusts, which makes dispersal **bursty**
— intermittent by construction, which satisfies the ethos's first law (a
distribution, not a binary) and needs saying up front rather than discovering.

### 6.2 Speciation

`individual_as_species` refuses because it copies the parent *species'* fates
rather than the individual's. With `ParamGenome` landed, "promote this
individual to a species" now has a second thing to copy, and both are on
`OrganismState` and both round-trip through the shelf — so the promotion is
now a matter of copying three genomes into a `SpeciesDef` rather than of
inventing a representation. It is still not the harder half, which is that a
`SpeciesId` is copied to offspring unchanged and every consumer of it assumes
species are a fixed set.

### 6.3 Leaf-cluster shape, seasons, sex

Named in §4 and not started. Leaf-cluster shape is the one this session would
build next if it had another night, for the reason §4 gives: it is the only
item on the list that is ink rather than a label.

---

## 7. What to do next, in the order the evidence supports

1. **Post §2's result to the owner and get a verdict on the noise floor.** It
   is the finding that reorders everything else, and it is one number.
2. **Make development heritable, which is the real form of "attack the noise
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
3. **Find out whether a sucker becomes a plant.** §4a shows the *event* is
   already reachable — a `herb` lineage discovers a shoot off a root in the
   shipped game, and a root `plastochron` takes it from ~1 per run to 13–29 —
   but nothing measures whether any of those shoots establishes, and §4a is
   also the section that shows a static census of this bed cannot tell a sucker
   from a buried collar. The instrument wants to follow the launched shoot
   rather than census the bed.
4. **Leaf-cluster shape**, §4.
5. **Then the parameter-genome rate**, once generation depth is deep enough to
   test §5.4's prediction — and run `genome_reach -- drift=1` first, because
   the free-lever list is the thing to price, not the rate.
6. **Then dispersal**, §6.1.

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
