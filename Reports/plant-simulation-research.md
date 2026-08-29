# Plant growth, plant evolution, and plant biology: research, and what it means for this architecture

Written against the state of `master` at `838c557` (tree rewrite step 1+3). Assumes
`research/m16-plant-biology.md`, `Reports/organism-substrate-design.md` and
`Reports/tree-rewrite-design.md` as read — this deliberately does not re-derive
auxin canalization, MIZ1 antagonism, space colonization or the pipe model, all of
which those documents already cover with citations. What follows is the
literature those documents *don't* touch, and an assessment of how far the
current substrate can carry the plant work before something structural has to
change.

---

## 0. Summary, stated first

The pixel substrate is a genuinely strong fit for **environmental coupling** and
for **self-organizing, stigmergic developmental rules** — better than most
research plant models, which have to bolt on a light model and a soil model this
engine already has. It is a genuinely poor fit for **growth by expansion**, which
is what plants actually do, and that mismatch is already visible in the
playtest feedback about one-pixel trunks.

Three things will hit, roughly in this order:

1. **Cells can be added but not expanded or displaced.** This is accretion, not
   growth. It is the reason trunks don't thicken, internodes don't elongate, and
   leaves don't expand. Everything else on this list is smaller.
2. **`Cell::aux` is already full** (4 bits type + 8 resource + 4 canopy density =
   16/16), and the plant work needs at least three more scalars.
3. **Nothing in the substrate carries polarity**, and every real
   shape-generating mechanism in plant development is polar. Isotropic diffusion
   cannot canalize; it can only blur.

For **evolution** specifically, the architecture is unusually well-placed: the
`.ron` species file is already a genotype, `organism_tick` is already a
developmental program, and `structural.rs` already measures one of Niklas's four
fitness tasks. The gap is reproduction, a generational clock, and built-in
trade-offs.

---

## 1. The three model families, and where this engine sits

Prusinkiewicz & Runions, *Computational models of plant development and form*,
New Phytologist 193:549–569 (2012) is the map for this whole area and is worth
reading end to end. It divides the field along a line that matters directly here:

**(a) Lineage / rule-based models.** L-systems (Prusinkiewicz & Lindenmayer,
*The Algorithmic Beauty of Plants*, 1990). Form comes from a rewriting grammar
applied to a string of modules. Strengths: compact, expressive, captures the
modular repetition real plants show. Weakness: the plant does not perceive its
environment unless you explicitly plumb it in (open L-systems).

**(b) Self-organizing / interaction-based models.** Form emerges from
competition and local feedback rather than from a grammar. Space colonization
(Runions, Lane & Prusinkiewicz 2007), leaf venation as a canalization process
(Runions et al., *Modeling and visualization of leaf venation patterns*, ACM
TOG 24:702–711, 2005), phyllotaxis from PIN1/auxin feedback (Jönsson et al.
2006; Smith et al. 2006), and bud activation by auxin canalization
(Prusinkiewicz et al., PNAS 2009 — already cited in `m16-plant-biology.md`).
Palubicki et al., *Self-organizing tree models for image synthesis* (ACM TOG
28:58, 2009) is the synthesis: light competition plus an endogenous
resource-distribution bias, and diverse tree forms fall out of changing that
bias alone.

**(c) Functional–structural plant models (FSPM).** Architecture coupled to
physiology: photosynthesis produces carbon, carbon is allocated to organs, organ
growth changes architecture, architecture changes light capture. LIGNUM
(Perttunen et al., Ecol. Model. 108:189–198, 1998), GreenLab (see the 2024
state-of-the-art review in *Plant Phenomics*), L-PEACH (Allen, Prusinkiewicz &
DeJong, New Phytologist 166:869–880, 2005). The defining device is a **common
pool with source–sink allocation**: biomass produced anywhere is distributed to
organs by *relative sink strength*, not by proximity.

**(d) Cell-based tissue models** — the family that most resembles this engine's
substrate. VirtualLeaf (Merks, Guravage, Inzé & Beemster, *Plant Physiology*
155:656–666, 2011) and the Cellular Potts Model represent individual cells with
walls, turgor and diffusing chemicals. Note what they do that this engine does
not: **cells change shape and size**. VirtualLeaf's core is a balance between
turgor pressure driving expansion and wall tension restricting it, resolved by
Metropolis energy minimization. A cell divides when its *area* doubles.

This engine is currently a hybrid of (b) and a hand-rolled (c), running on a
substrate that superficially looks like (d) but has none of (d)'s deformability.
That hybrid is a defensible and interesting position — it is close to nothing
else in the literature — but it's worth being explicit that the substrate
resemblance to cell-based tissue models is skin-deep.

---

## 2. What the architecture is unusually good at

**Environmental coupling is already solved, and this is rare.** The hardest and
least glamorous part of any FSPM is the environment: light interception normally
needs a separate ray-tracer or a voxel shadow-propagation grid, soil water needs
a separate hydrology model, and mechanical failure usually needs a separate
solver or isn't modelled at all. This engine has a light field, a moisture
field, a temperature field, a pressure/velocity field, fire, and a working
structural-failure model, all sharing one substrate and one frame loop. Palubicki's
shadow-propagation voxel grid is, functionally, `rebuild_blocked` blocking on
`Solid | Plant` plus `LIGHT_DECAY` — that piece is done.

The practical consequence: mechanisms that are *hypotheses* in the literature
because the environment is stubbed out can be run here for real. A tree that
shades its own lower branches, gets struck by a burning neighbour, loses its
trunk base, and collapses under `max_unsupported_span` is not something GreenLab
or LIGNUM can express at all.

**Stigmergy suits the substrate.** Deposit–diffuse–decay–follow is exactly the
shape of a CA rule, and it is also the actual shape of canalization, venation,
and phyllotaxis. `Reports/stigmergy-research.md` is pointed in the right
direction here. This is the family of mechanisms to lean on.

**Damage, death and decay are already unified.** The ash → soil → regrowth loop
is the kind of thing plant models almost never close, because their plants exist
in a world with no chemistry.

---

## 3. Wall 1: accretion is not growth

This is the important one.

Real plant growth has two components: **cell division at meristems**, and
**irreversible turgor-driven cell expansion**, formalized by Lockhart, *An
analysis of irreversible plant cell elongation*, J. Theor. Biol. 8:264–275
(1965): a wall yields irreversibly when turgor exceeds a threshold *Y*, at a rate
set by wall extensibility. Expansion is the larger contributor to final size in
most tissues, and — critically — **expansion displaces everything distal to it**.
An elongating internode lifts the entire shoot above it. A thickening trunk
shoves bark outward. A growing leaf pushes air out of the way.

On this substrate, a `Plant` cell is immovable and exactly one pixel. Growth can
only happen by writing a new cell into an *empty* neighbour. That is accretion —
the growth mode of coral, lichen, crust fungi, and moss. It is not the growth
mode of a tree.

Concrete consequences, all of which are already latent in the code:

- **No internode elongation.** A tip cannot push its own stem upward; it can only
  extend into free space. The whole plant is therefore built tip-outward, and
  every cell's position is fixed the moment it is created.
- **`SecondaryThicken` can only fill empty neighbours.** Real secondary growth
  adds a cylinder of xylem *inside* existing tissue and displaces the bark. Here,
  a trunk surrounded by other wood cannot thicken at all — which means thickening
  works only at the canopy edge, backwards from reality.
- **No leaf expansion.** A `Leaf` cell is one pixel forever. Leaf area, the single
  most important trait in plant ecology, is not a variable.
- **No etiolation or shade-avoidance elongation.** A shaded real plant elongates
  dramatically to escape. That behaviour is unavailable.
- **No self-supporting form feedback.** Because nothing pushes, mechanical stress
  never arises from growth itself, only from removal.

The playtest note about "uniform one-pixel trunk thickness" and "tree growth
starting mid-air with no germination" is this wall, correctly diagnosed by eye
before the design caught up.

### Three ways out, in increasing order of ambition

**(i) Accept it and change the target organism.** Accretive growth is real
biology — it is what moss, lichen, crustose algae, fungal mycelium, coral and
some vines actually do. A world whose flora is accretive is scientifically
defensible and would look unlike any other game. Trees become the odd case, not
the flagship. Cheapest, and honestly the most likely to produce something novel.

**(ii) A displacement primitive.** Give organism cells the ability to push: a
`Plant` cell that grows displaces the column of cells ahead of it by one, the way
a piston does. The machinery mostly exists — `move_cell` plus a bounded scan —
and the cost is bounded by the column length, which `MAX_REACH` already caps at
32. Elongation, thickening and leaf expansion all fall out of one primitive.
Risks: it interacts with `structural.rs` (a pushed structure's anchor distances
all change), with the parallel sweep's write-disjointness proof (a push crossing
a chunk boundary is a multi-cell write), and with the `FLAG_MOVED` sweep
semantics. Not small, but it is the single change that unlocks the most.

**(iii) Sub-cell fill fraction — and this engine already invented it.** The
liquid rewrite put a continuous fill amount in `Cell::aux` on a `LIQUID_FULL =
1000` scale, precisely because discrete occupancy could not express a continuous
quantity. The same trick is Lockhart's equation: give an organism cell a
*turgor/extension* scalar that accumulates while resource is available, and
promote it to a whole new cell when it saturates. That gives sub-pixel growth
rate, makes `wall extensibility` and `yield threshold` into real evolvable
parameters, and needs no displacement. It does not solve thickening or leaf
area, but it makes growth *rate* continuous and physically grounded, which is
most of what tuning currently fakes.

(ii) and (iii) compose well: (iii) for rate, (ii) for displacement.

---

## 4. Wall 2: `Cell::aux` is full, and the packing is already too coarse

Current layout: bits 0–3 cell type, bits 4–11 resource (8 bits), bits 12–15
canopy density (4 bits). That is all 16 bits, on top of a separate 16-bit
`organism_id`, inside a 12-byte `Cell` the codebase has already declined to
widen a third time.

What the plant work still needs somewhere: a second currency (every FSPM needs at
minimum carbon *and* water/nitrogen — they have different sources and different
sinks, and collapsing them to one scalar removes the trade-off that makes
allocation interesting), organ age (leaf lifespan is the central axis of the leaf
economics spectrum; see §6), a polarity direction (§5), a turgor/extension
scalar if §3(iii) lands, and a phyllotaxis phase if that's ever attempted.

Four bits for canopy density is also already too coarse for its purpose: 16
levels, read through a scoring function that compares eight neighbours, will
frequently produce ties and quantization artefacts in exactly the situation where
the signal is supposed to discriminate.

**Recommendation.** Stop packing into `Cell`. Organism-owned cells are a tiny
fraction of any world — a mature forest is maybe 1–2% of cells — so a sidecar
table (`HashMap<(i32, i32), OrganismCell>`, or per-organism `Vec` of cells with a
position index) costs almost nothing in aggregate and removes the ceiling
permanently. Invariant 3 (all cell access goes through `get`/`set`) is exactly
the seam that makes this a contained change. `Cell::organism_id` stays as the
"is this organism tissue, and whose" tag; everything else moves out. This also
makes the `TransportChannel` scope cut unnecessary, and gives
`OrganismState` — currently holding only `species` — somewhere useful to live.

---

## 5. Wall 3: nothing carries polarity, so diffusion can never canalize

`organism::diffuse_resource` averages a cell's value against its four
same-organism neighbours. That is isotropic diffusion. It is the correct model
for a passive solute, and the wrong model for essentially every shape-generating
process in a plant.

Plant development is polar throughout: auxin moves basipetally through
directional PIN1 efflux carriers; xylem moves water up and phloem moves
photosynthate down, in separate directional tissues; a gravitropic setpoint angle
is a property of a branch, not a point. Sachs's canalization hypothesis — the
basis of the auxin mechanism already cited in `m16-plant-biology.md` — is
explicitly a **positive feedback between flux and conductivity**: a path that
carries more flux becomes better at carrying flux, which is what turns a diffuse
field into a discrete channel. That is how veins form, how vascular strands form,
how one leader dominates its siblings.

Symmetric averaging has no flux, therefore no feedback, therefore no channels. It
will produce a smooth gradient and nothing else, no matter how long it runs.
The `Grow` doc comment's honesty about canalization being "translated honestly,
not oversold" is right, and this is the mechanical reason why.

Worth noting that this is the *same failure mode* as the crowding bug in the
current `Grow` implementation, where the self-avoidance term reads canopy density
from a cell it has just proven is empty and therefore always gets 0.0. In both
cases a mechanism named after a directional, self-reinforcing process is
implemented as a symmetric or inert one. It's a pattern worth watching for.

**Recommendation.** Three or four bits of per-cell polarity (8 directions, plus
"none") and a flux-following update rule — move resource preferentially along
polarity, and rotate polarity toward the direction that carried the most flux
last tick. That single addition converts `diffuse_resource` from a blur into
canalization, and it is what would give trunk/branch hierarchy, apical dominance,
and vein-like structure as emergent outcomes rather than as tuned weights. It
also makes Palubicki's extended Borchert–Honda resource distribution
implementable: basipetal flux accumulation followed by acropetal allocation, which
is the mechanism that produces both apical dominance *and* canopy filling in the
2009 model.

---

## 6. Wall 4: local diffusion vs. the common pool

FSPM's central device is a common pool: total assimilate is divided among organs
by relative sink strength, regardless of where the organ sits. GreenLab's whole
calibration methodology rests on this.

Local diffusion is the philosophically correct choice for this project (nothing
should know about the whole organism), but it has a consequence worth accepting
deliberately: **allocation becomes distance-dependent for numerical rather than
biological reasons**. A tall tree's roots are many diffusion steps from its
leaves; a short tree's are few. At `DIFFUSION_RATE` per tick, resource reaching
a distant sink falls off roughly geometrically with path length. Tall plants will
starve their extremities not because that is what tall plants do, but because the
solver converges slowly. This will look like a biological result and won't be one.

Two mitigations, neither requiring abandoning the local rule:

- **Run diffusion to convergence, not one step per frame.** Growth ticks are
  every 20–45 frames; the resource field could relax many times between them.
  Cheap, and turns "slow gradient" into "near-equilibrium gradient with real
  distance-dependent losses" — which is actually the pipe model's own claim.
- **Let `OrganismState` hold whole-plant totals** (total leaf count, total
  resource, age). This is not the hardcoding `design-philosophy.md` forbids: the
  pipe model, allometry, and reproduction thresholds are all genuinely
  whole-organism properties, and there is no local rule that computes "am I big
  enough to flower." Keep the *decisions* local; let a small number of *totals*
  be global.

---

## 7. Evolution: this is the strongest part of the fit

### 7a. The genotype already exists

`assets/species/*.ron` is a genotype. `organism_tick` is a developmental program.
The grown plant is the phenotype. That separation is the thing most hobby
evolution projects never achieve, and here it fell out of the "species as data,
parallel to materials" decision. It means mutation is `.ron` field perturbation,
inheritance is file copying, and speciation is file divergence — no new
machinery.

Two levels of genome, and they should be built in that order:

- **Parameter vector.** Perturb the numeric fields of `Behavior` variants
  (`cost`, `branch_chance`, weights, `rate`, `pipe_ratio`). Safe, always
  produces a viable organism, and is what Niklas's and Bornhofen's models
  actually vary.
- **Structural.** Which `Behavior`s each `CellType` carries, and what the cell
  types transition into. Richer, and where genuinely novel body plans would come
  from, but most mutations produce nonviable organisms. Ochoa, *On Genetic
  Algorithms and Lindenmayer Systems* (PPSN 1998) is the reference on making
  structural mutation of a developmental grammar behave.

### 7b. Niklas's model is a near-exact fit, and is small

Karl Niklas's simulated adaptive walks are the canonical plant-evolution
simulation, and are far more tractable than they sound:

- Niklas & Kerchner (1984), Niklas (1988): a morphospace of early vascular land
  plant form defined by **six variables** — two branching probabilities, two
  rotation angles, two bifurcation angles.
- Niklas, *Morphological Evolution Through Complex Domains of Fitness*, PNAS
  91:6772–6779 (1994); *Evolutionary walks through a land plant morphospace*,
  J. Exp. Bot. 50:39–52 (1999); *Computer models of early land plant evolution*,
  Annu. Rev. Earth Planet. Sci. 32:47–66 (2004).
- Structure: a morphospace, a fitness function scoring each morphology on
  **light interception, mechanical stability, water conservation, and
  reproductive success**, and an adaptive walk that hill-climbs to neighbouring
  more-fit morphologies.

This engine can already measure three of the four natively:

| Niklas task | Engine measurement |
|---|---|
| Light interception | sum of light-field reads at `Leaf` cells over lifetime |
| Mechanical stability | `structural.rs` — a plant that exceeds `max_unsupported_span` breaks |
| Water conservation | moisture field + `Absorb` uptake vs. loss |
| Reproduction | **missing** — no seed dispersal mechanic exists |

The single most valuable result to take from Niklas: **multi-task fitness
landscapes have many near-equal optima; single-task landscapes have one.**
Optimizing for one thing collapses the population onto one morphology.
Optimizing for three or four conflicting things is what produces a *diverse*
flora, and it is also what Niklas concluded actually happened in the Devonian.
If the goal is an interesting evolving ecosystem rather than one winning plant,
selecting on at least three conflicting tasks is not a nice-to-have; it is the
mechanism.

### 7c. Trade-offs have to be built in, or evolution is boring

Without a cost attached to every benefit, selection maximizes everything and the
population converges to one maximal plant. Real plants can't do that, and the
constraints are well quantified:

- **Leaf economics spectrum** (Wright et al., *The worldwide leaf economics
  spectrum*, Nature 428:821–827, 2004; 2,548 species, 175 sites). One dominant
  axis from fast return to slow return: low leaf mass per area, high
  photosynthetic rate, short leaf lifespan at one end; high LMA, low rate, long
  lifespan and greater physical toughness at the other. Roughly three-quarters of
  interspecific variation in carbon-fixation traits sits on this one spectrum.
  In engine terms: `Photosynthesize { rate }` should be *inversely* coupled to
  leaf durability and lifespan, not independent of it.
- **Wood density vs. growth rate.** Denser wood correlates with slower growth and
  greater mechanical strength and survival. The engine already has both sides:
  `density` in the material file and `max_unsupported_span` in `structural.rs`.
  Coupling them is one line of design and gives a real evolvable trade-off.
- **Grime's CSR strategies** (competitor / stress-tolerator / ruderal). Bornhofen,
  Barot & Lattaud, *The evolution of CSR life-history strategies in a plant model
  with explicit physiology and architecture*, Ecol. Model. (2011), got all three
  to **emerge** — not be coded — from a physiological + architectural model given
  an environment with heterogeneous resource availability and varying disturbance
  frequency. Disturbance is the half most simulations lack and this engine has in
  abundance: fire, explosions, structural collapse, the player's own brush.

Bornhofen & Lattaud, *Competition and evolution in virtual plant communities*,
Natural Computing 8:349–385 (2009) is the closest existing precedent overall —
L-system morphology plus a transport–resistance physiology, in a simplified 3D
ecosystem, run over many generations, with mutation on both the L-system and a
genetic parameter set. Worth reading in full before designing the evolution
layer; it is essentially the target system, minus the physics.

### 7d. Two engine-specific problems evolution will surface

**Time scale.** Growth ticks of 20–45 frames mean a plant matures in seconds of
wall clock. Evolution needs thousands of generations. This needs a headless
fast-forward mode with its own clock — `examples/ascii.rs` is already the right
seed for it, and it already runs in CI.

**Per-chunk RNG becomes a confound.** ~~`Chunk::rng` is seeded from the chunk's
coordinate. That was the right call for the parallel sweep, but it means the same
genome planted in two different places draws a different sequence — position
becomes a hidden inherited variable.~~ For a fitness comparison that's noise
correlated with location, which is exactly the kind of thing that produces a
spurious "evolutionary" result. A per-organism RNG stream, seeded from the
organism id, would keep determinism and remove the confound.

> **CORRECTED 2026-08-28 — this paragraph names the wrong culprit, and
> `src/sim/rng.rs:105-117` has said so in source for some time without the
> correction reaching this file.** Organisms never touch `Chunk::rng`; it is
> reached only by the CA sweep, through `CellSurface::rng()`. The real
> mechanism was **order coupling** — `world.rng`'s sequence depending on how
> many draws every other organism made first — and **the recommendation was
> right and has shipped**: `rng::stream(organism_id, x, y, frame)`
> (`plant.rs:1388`) is the per-organism stream this asks for.
>
> Position deliberately stays *in* that key, so two identical genomes planted
> in different places still grow differently; what changed is that the
> difference comes from where they are rather than from what else happens to
> exist. The residual confound binds **founders only** — see
> `plant-evolution-design.md` §1c and
> `plant-evolvability-facts-2026-08-27.md` §6.

---

## 8. Ordered recommendations

1. **Fix the two inert mechanisms first** (crowding reads an always-empty cell;
   `pack_aux` clobbers bits 12–15). Both are cheap, and the whole self-avoidance
   design is currently dead code.
2. **Add polarity.** Highest ratio of emergent behaviour to implementation cost
   of anything on this list. It is what makes canalization real, and it is a
   prerequisite for Borchert–Honda allocation.
3. **Move organism state out of `Cell::aux` into a sidecar.** Do it before adding
   more scalars, not after; the packing ceiling is already reached.
4. **Decide the growth-mode question explicitly** — accretive flora (§3(i)),
   displacement primitive (§3(ii)), or sub-cell extension (§3(iii)). This is a
   design decision that should be made deliberately and written down, because
   every subsequent plant mechanic inherits it. Choosing (i) is a legitimate
   answer and would be a distinctive one.
5. **Add reproduction.** It is the missing fourth Niklas task, it closes the M16
   verify criterion properly, and nothing else about evolution can start without
   it.
6. **Couple the trade-offs** (§7c) before running any selection.
7. **Build the headless generational harness** before the evolution rules, not
   after.
8. **Finish or delete the old `TreeState` path.** Two tree implementations
   coexisting, one unreachable and one referencing a `tree.ron` that doesn't
   exist, will make every subsequent finding ambiguous.

---

## 9. Reading list, ranked

1. Prusinkiewicz & Runions (2012), *Computational models of plant development
   and form*, New Phytologist 193:549–569. The map. Free full text.
2. Niklas (2004), *Computer models of early land plant evolution*, Annu. Rev.
   Earth Planet. Sci. 32:47–66. The evolution model, and the multi-task result.
3. Bornhofen & Lattaud (2009), *Competition and evolution in virtual plant
   communities*, Natural Computing 8:349–385. Closest existing precedent.
4. Palubicki et al. (2009), *Self-organizing tree models for image synthesis*,
   ACM TOG 28:58. The self-organizing tree model to target.
5. Merks et al. (2011), *VirtualLeaf*, Plant Physiology 155:656–666. What a
   cell-based plant model looks like when cells can deform.
6. Wright et al. (2004), *The worldwide leaf economics spectrum*, Nature
   428:821–827. The trade-off axis.
7. Lockhart (1965), *An analysis of irreversible plant cell elongation*,
   J. Theor. Biol. 8:264–275. Short, and directly implementable as §3(iii).
8. Hallé, Oldeman & Tomlinson (1978), *Tropical Trees and Forests: An
   Architectural Analysis*. The 23 architectural models — a ready-made target
   list for what the species-file parameter space should be able to express, and
   a good falsification test: if the parameter surface can't reach Rauh's,
   Leeuwenberg's and Massart's models, it isn't general yet.
