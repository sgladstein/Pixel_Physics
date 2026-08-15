# Tree architecture: why the canopy is a mass instead of a tree

Research pass prompted by a playtest reading that is sharper than the one
`PLAN.md` had been carrying. The recorded complaint was *"canopies merge
into a slab"*, filed under size, and the economy pass was chasing size.
The owner's correction:

> the biggest problem right now seems less that we grow forever but we grow
> into a mass instead of growing more up than out. If we maintain a tree
> shape, that would be less a problem. The tree is still overall small.

That reframing is load-bearing, and it is what this document is a response
to. **Size is downstream of shape.** A tree that keeps its shape can be
allowed to grow; a tree that fills in as a volume looks wrong at any size,
and bounding its size only produces a smaller blob.

A second correction, recorded in §0 because it retired this document's own
first framing: the problem is not that the crown is broad rather than
conical. An oak is broad *and* has a clear bole with its canopy up top.
What is missing is the bole and the hollow crown, not the taper.

It also retires a question the previous session spent real effort on. Bud
break was reverted for want of an absolute size bound (`PLAN.md`), and two
candidates were left open. §1 below supersedes that: the correct mechanism
bounds size **as a side effect of maintaining shape**, so it was never a
size problem.

---

## 0. The vocabulary — and a framing this document got wrong first

Botany names the axis precisely, which turns "looks wrong" into a
literature search:

- **Excurrent** — a single central stem with a conical crown (spruce, fir).
  The apical leader elongates more than the laterals below it.
- **Decurrent** — a broad crown with several co-equal scaffold branches
  (mature oak, maple). Laterals grow as fast as or faster than the leader.

**A first draft of this document blamed the engine's shape on being
decurrent, and that is wrong.** The owner's correction:

> You say our current design is Decurrent like oak trees but oak trees are
> still tall trunk with canopy at top, not a big mass like we are currently
> building

That is exactly right and it matters, because it separates two things the
first framing had welded together. A decurrent oak still has a **clear
bole** — a length of trunk carrying no branches at all — with the crown
starting well above the ground. Excurrent and decurrent differ in *crown
shape*; **both have a clear bole, and both have a crown that is a shell of
foliage over a branch skeleton rather than a solid volume.**

So the engine's trees are not decurrent oaks. They are missing two
properties that oaks and spruces share:

1. **No clear bole.** Growth starts at the ground and never stops
   happening there, so there is no length of bare trunk. A real bole exists
   because its branches *were there and are gone*.
2. **The crown is a solid volume, not a shell.** Real foliage is a surface
   over a skeleton; the interior is bare branch and air. Ours fills in.

Both of those are the *same* missing mechanism, and it is not apical
control — it is §1. Crown recession is what produces a bole, and interior
die-back is what makes a crown a shell. Excurrent-vs-decurrent is a real
axis and worth a species parameter later (§2), but it is **not** what the
complaint is about, and building apical control first would have been
answering the wrong question.

Two further distinctions the literature is careful about, kept because §4
depends on them:

- **Apical dominance** — the apex suppresses *the activation of buds* below
  it. The auxin mechanism `research/m16-plant-biology.md` §3-4 documents.
- **Apical control** — the apex suppresses *the elongation rate of laterals
  that already exist*. This is what decides excurrent vs decurrent.

---

## 0b. Everything here must be per-species, not per-tree

Stated by the owner, and it changes how each mechanism below should be
built rather than merely what order to build them in:

> We will want a variety of plants in the future, not just trees and not
> just one type. So we don't need to fully limit but be flexible

So the target is **not** "make the tree look like a tree". It is a
parameter surface on which a tree, a shrub, a vine, a grass and a moss are
all reachable points. `Reports/tree-rewrite-design.md` §9 already set this
precedent for `Grow` and demonstrated it across differing plant forms;
this document inherits it.

Concretely, for every mechanism below:

- **The knob is a `Behavior` field in the species `.ron`, not a `const` in
  `plant.rs`.** A global constant is a claim that every plant that will
  ever exist wants the same value.
- **Zero must be a legal, meaningful value**, and must mean "this species
  does not do this" rather than "misconfigured". Moss should not self-prune;
  a grass has no bole to recess; `MAX_ROOT_FRACTION` and `plastochron: 0`
  already set this precedent in the codebase.
- **Prefer a mechanism that produces several forms from one rule** over one
  that produces the right tree. Self-pruning at a high light threshold
  gives a tall clear bole; at a low threshold, a shrub that keeps its lower
  branches; at zero, a moss mat that keeps everything. That is one rule and
  three plants, which is the shape `design-philosophy.md` §2b asks for.

This is also a reason to prefer §1 over §4 beyond effect size: a carbon
balance is meaningful for *any* photosynthesising organism, whereas an
auxin leader channel presupposes something with a leader.

**Solve trees first, and build outward from that — agreed, with one
discipline attached.** The owner:

> I image in the future having trees, vines, bushes, all sorts of plants,
> maybe alien ones with no parallel, all built on this platform, but I want
> to solve trees first and build off that unless you think that doesn't
> make sense

It makes sense, and for a stronger reason than expedience. A parameter
surface cannot be designed from nothing: you need at least one point on it
that demonstrably works before you know which axes are real. Generalising
first produces knobs that turn out to be the wrong knobs, and this
codebase has the receipt — `TransportChannel` was specified as a
per-species behaviour, cut before implementation for want of a caller, and
the shared `DIFFUSION_RATE` that replaced it has been sufficient through
three subsequent passes.

The tree is also the *hardest* point on the surface, not a soft one. It is
the only plant form that needs a bole, a crown, secondary thickening, a
root:shoot balance and a transport hierarchy simultaneously. Anything that
supports a tree very likely supports a shrub by relaxing parameters; the
reverse is not true.

The discipline that makes "build off that" actually work costs nothing
now: **put each number on the species as it is introduced, even while only
one species uses it.** Not a speculative parameter for a plant that does
not exist — the specific number this tree needs, placed where a second
species could differ. Extracting constants later is a refactor across every
call site; putting them in `.ron` on the way in is a line of RON. The
"alien with no parallel" case is served by the same discipline rather than
by trying to anticipate it: what makes an unfamiliar form reachable is that
each rule is a number rather than an assumption.

---

## 1. The missing mechanism: maintenance respiration and self-pruning

**This is the finding. Everything else in this document is smaller.**

Real trees shed their own branches. A branch low in the crown, or densely
shaded, receives too little light for photosynthesis to cover its own
**maintenance respiration** — it crosses from net carbon *source* to net
carbon *sink* — and the tree abscises it. This is not damage or disease; it
is routine, it has a name (**cladoptosis**, or natural pruning), and it is
what *defines the base and depth of a crown*.

The threshold is a carbon balance, not a light level: a branch is shed when
it costs more to keep than it returns. Shade-tolerant species self-prune at
lower light precisely because their light compensation point is lower.

**The engine has neither half of this.** A `MatureBody` cell costs
*nothing* to keep — there is no maintenance term anywhere — and nothing is
ever shed for being unproductive. So:

- Interior tissue accumulates forever. Nothing clears the inside of the
  crown, which is precisely the "fills in as a mass" reading.
- The crown base never rises. Real trees look like trees substantially
  because their lower branches are *gone*.
- Growth is unbounded, because new tissue is free to keep.

**And it bounds size for free**, which is the part that supersedes the bud
break impasse. With a maintenance cost, a tree grows until new growth can
no longer pay for itself. That is not an invented cap, a ratio, or a tuned
constant — it is a carbon balance, and it is the actual reason real trees
have a mature size. The previous session went looking for an absolute size
bound and found that ratio bounds do not provide one; this is the bound,
approached from the correct end.

### 1a. Why this is cheap here, and why polarity makes it cheap

The obvious objection is that shedding needs branch membership — "which
cells belong to this limb" — which the engine does not track and which
would be a traversal per cell.

It does not, because **canalization already ranks cells by how well
supplied they are.** A cell's `carbon_conductance` is high exactly when
real flux has been passing through it. Under a maintenance cost, when
supply is scarce, the poorly-connected cells starve *first* — and the
poorly-connected cells are the shaded interior ones, because they have no
downstream demand drawing through them.

So the rule is local and needs no traversal:

> a cell that cannot pay its upkeep for several consecutive ticks dies.

Branch-level shedding then falls out on its own. When a cell dies, whatever
was above it loses its supply path, starves in turn, and follows — which is
a limb being shed, one cell at a time, without anything ever computing what
a limb is. `structural.rs` already turns disconnected plant structure into
falling debris, so a shed branch *falls*, which is both correct and the
kind of thing `CLAUDE.md` says should have visible consequence.

### 1b. What to watch for

The failure mode is a cascade that takes the whole tree: upkeep set too
high starves the trunk, the trunk dies, everything above it follows. The
guard is that a trunk cell is by construction the best-connected cell in
the plant, so it starves last — but this needs measuring, not assuming, and
the ensemble already reports establishment rate, which would collapse
first and visibly.

---

## 2. The branch-point split: one number decides excurrent vs decurrent

Streit et al. (2024, *in silico Plants*) parametrize tree architecture and
find that the single parameter controlling crown form is **`β_x`, the xylem
flow split**: at a branching point it sets how water and nutrients divide
between *the continuing main axis* and *the lateral branch*.

- `β_x > 0.5` — the continuing axis takes more, the leader outgrows the
  laterals, **excurrent**.
- `β_x <= 0.5` — laterals take an equal or greater share, **decurrent**.

**The engine already has this split and currently sets it to 0.5 by
omission.** `organism::transport` divides carbon at a branch point strictly
by the two faces' conductances, and both faces start at `CONDUCTANCE_MIN`
and canalize purely on measured flux. Nothing biases the straight-on face
over the turning one, so a branch point is a fair coin — which is exactly
`β_x = 0.5`, exactly decurrent, and exactly what is on screen.

The change is small and lands in machinery that already exists: bias the
conductance update by how well a face aligns with the *incoming* supply
direction. `supply_direction` already computes that vector for `Grow`. A
face continuing the axis gets a higher basal insertion or a boosted
response; a face turning off it gets less. `β_x` becomes one named
constant with a citation and a measured effect.

This is also the honest place to note that §7h's worked example is
unaffected: it concerns two *symmetric* tips, where alignment is equal and
the bias cancels.

---

## 3. Gravitropism is set far too low for an upright tree

The same paper decomposes shoot orientation into a **photogravitropic
set-point vector** built from three sensitivities: **gravitysense**
(preferred direction relative to gravity), **lightsense** (response toward
light), and **proprioception** (how strongly a branch keeps its existing
orientation). High gravitysense gives upright forms (poplar); moderate
values give spreading ones (oak).

`tree.ron`'s `GrowingTip` currently runs:

```
continuation_weight: 0.7    // proprioception -- the strongest term
light_weight:       0.4     // lightsense
upward_weight:      0.1     // gravitysense
crowding_weight:    0.5
```

Gravitysense is the **weakest** term in the blend, at a seventh of
proprioception and a quarter of lightsense — and the `RootTip` below it
uses `0.6` for the same term, six times higher. A shoot that barely prefers
up will not build a vertical leader, and phototropism cannot substitute:
light comes from above but is *lateral* at a crown edge, so a light-driven
tip grows outward into the open, which is the observed behaviour.

This is a `.ron` tuning question rather than a mechanism gap, and it is
cheap to sweep — but it should be swept *after* §1, because self-pruning
changes what the light field looks like and therefore what the right
balance is.

---

## 4. Apical dominance proper — the seam is already drawn

`research/m16-plant-biology.md` §3-4 calls auxin canalization *"the single
most important finding for making procedural tree shape look right"* and
cites Prusinkiewicz et al. (2009, PNAS 106:17431-17436) as a directly
portable algorithm: each bud is a competing auxin source, whichever
canalizes into the main stream first suppresses the others, and the switch
is hysteretic.

`plant-substrate-v2-design.md` §7i already drew the seam for this and
costed it: *"a second `[f32; 4]` and a second scalar on `OrganismCell`,
sourced at `GrowingTip` and sunk at the base, running the **identical**
update rule in the opposite polarity."*

That is now genuinely cheap, because the update rule it wants to reuse is
built, tested and tuned — `transport`'s pairwise exchange and the Hill
conductance update run unchanged in the opposite direction. The work is a
second channel, not a second mechanism.

**But it is deliberately last in this document's order.** It controls bud
*activation*, and §0 is explicit that activation is not what decides
excurrent vs decurrent — elongation rate is. Building auxin first would be
implementing the more famous mechanism rather than the one the complaint is
about.

---

## 5. Recommended order, and why this one

1. **Maintenance respiration and self-pruning** (§1). Largest effect,
   addresses the actual complaint, and dissolves the size-bound problem
   that blocked bud break rather than solving it separately.
2. **Sweep gravitysense** (§3). One `.ron` number, cheap, but only
   meaningful once §1 has changed the light field.
3. **The branch-point split `β_x`** (§2). Small, reuses the conductance
   machinery, and is the literature's single named control for crown form.
4. **Auxin channel / apical dominance** (§4). The largest build, the most
   famous mechanism, and the least directly aimed at what is wrong.
5. **Bud break, revisited** (`PLAN.md`). Re-land as *disturbance*-triggered
   once §1 exists — the wound case is what §2e actually wanted, is
   self-limiting by construction, and no longer needs the size bound that
   blocked it.

Each of 1-3 is measurable on the harness that already exists: the ensemble
reports establishment rate, stem thickness above the base, and the
differentiation split, and `filmstrip scene=forest` shows the silhouette.
**Judge these on the picture and on stem thickness above the base — not on
`rows >1 cell wide`**, which `PLAN.md` records is dominated by the basal
slab and has already misled this project twice.

---

## Sources

- [Parametrization of biological assumptions to simulate growth of tree branching architectures (Streit et al., *in silico Plants*, 2024)](https://pmc.ncbi.nlm.nih.gov/articles/PMC11128038/) — `β_x` xylem-flow split, photogravitropic set-point vector, shedding rate `σ_s`
- [Self-pruning in tree crowns is influenced by functional strategies and neighbourhood interactions (Kothari et al., *Functional Ecology*, 2025)](https://besjournals.onlinelibrary.wiley.com/doi/full/10.1111/1365-2435.70116) — self-pruning as a carbon-balance threshold
- [Uniform versus Asymmetric Shading Mediates Crown Recession in Conifers](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC4138101/) — crown recession driven by shading
- [Managing Tree Crowns for Quality — Crown Recession, Branch Shedding, and Silvicultural Pruning](https://digitalcommons.library.umaine.edu/silviculture/27/) — crown base defined by self-pruning
- [Apical dominance and apical control in multiple flushing (Cline & Harrington, USDA PNW)](https://www.fs.usda.gov/pnw/pubs/journals/pnw_2007_cline001.pdf) — apical dominance vs apical control
- [Tree Anatomy 101 (Iowa State University Extension)](https://naturalresources.extension.iastate.edu/forestry/tree_biology/101.html) — excurrent/decurrent and leader-vs-lateral elongation
- [Control of bud activation by an auxin transport switch (Prusinkiewicz et al., PNAS 2009)](https://www.pnas.org/doi/10.1073/pnas.0906696106) — already cited in `plant.rs`; the algorithm §4 would port

---

## 7. The 2D problem, and why it changes what "fix the shape" means

Raised by the owner, and it is the most important framing in this document
because it invalidates a category of fix rather than a particular one:

> we are working in a 2d environment. Real trees grow in 3d. In our world
> it is way easier for two branches or even separate trees to grow into
> each other/merge. How do we avoid this in a satisfying (internally
> consistent) manner.

### 7a. The density argument, stated numerically

A crown of radius `R` holds volume `~R³` in three dimensions and area
`~R²` in two. Put the same `N` branch tips in both and the 2D crown is
**`R` times denser**. Every branching intuition anyone has — including
every parameter in `tree.ron`, and every model in the literature §2 cites —
is calibrated against a world where a new branch can go *around* its
neighbours in the third dimension. In 2D there is nowhere to go around.

So a branching rule that produces an airy 3D crown produces a solid 2D
one, and no amount of tuning a 3D-shaped rule escapes that. **A 2D tree
needs materially fewer branches than its 3D counterpart, not the same
branches restrained.**

### 7b. The reframing: a real crown *is* a solid mass — of leaves

The complaint is that the canopy fills in. But a real tree viewed from
outside is *already* opaque: you cannot see its branches, only foliage.
Our 2D view is much closer to that silhouette than to a slice through the
crown, so **a filled crown is not the error.** What is filling it is.

Measured (`PLAN.md`): wood 12,039 against 253 leaves, a ratio of **48:1**,
and climbing. Invert that and the same silhouette reads correctly — a
sparse woody skeleton carrying a mass of foliage is what a tree *is*.

This changes the target. The previous framing — "stop the crown filling" —
was aimed at the wrong quantity, and self-pruning (§1) was proposed to
serve it. The corrected target is **shift the composition**: bound wood
hard by the pipe model, and let leaves be abundant. Self-pruning still has
a job (crown recession, the clear bole) but it is no longer the headline.

### 7c. Crown shyness — one real mechanism, both merge problems

Real forests solve exactly the merging problem the owner describes, and it
has a name: **crown shyness**, the river-like gaps adjacent trees leave
between their canopies. Three mechanisms are proposed and probably all
operate: mechanical abrasion of tips in wind, pest/disease avoidance, and
**shade-avoidance signalling** — phytochromes detecting the red/far-red
ratio of light *reflected off neighbouring foliage*, and growth shifting
away from it.

The third is directly implementable here, and it is worth noticing why it
is the right shape of answer rather than merely an available one:
**far-red reflectance does not care whose leaves it came off.** A tip
detects *foliage nearby*, not *foliage belonging to another organism*. So a
single rule prevents a tree merging with itself **and** with its
neighbour — which is precisely the pair of failures 2D makes likely.

The engine already has the field this needs. `canopy_density` is a
deposit-diffuse-decay proximity signal read by `Grow` as a crowding
penalty — a stigmergic stand-in for exactly this. But
`candidate_crowding` filters it to `n.organism_id() == organism_id`, and
there is a test asserting that it does
(`candidate_crowding_ignores_a_different_organisms_density`). **That filter
is the opposite of crown shyness**, and it was a reasonable call when the
channel was framed as *self*-avoidance; under the far-red reading it is
wrong, because a phytochrome cannot ask who a leaf belongs to.

Making the channel organism-blind is a small change with a citation, and
it converts an existing self-avoidance mechanism into stand-level spacing
for free.

### 7d. What this means for the order

Revised from §5, which was written before the 2D framing:

1. **Make `candidate_crowding` organism-blind** (§7c). Smallest change,
   real citation, and it is the direct answer to the owner's question.
   Also gives the crowding term something to do at stand scale, which it
   has never had.
2. **Invert the wood:leaf ratio** (§7b). `thicken()` is producing 48x more
   wood than foliage and also *consumes* leaves as it goes. Bound wood
   harder and stop it eating the crown it is supposed to serve.
3. **Reduce branching density for 2D** (§7a). `branch_chance` and
   `max_active_tips` were never chosen against a 2D area budget.
4. Self-pruning (§1), now for crown recession and the clear bole rather
   than as the fix for filling.
5. Everything else in §5, unchanged.

**And a caution about reading the pictures**, recorded because it has
already cost a wrong call in this document: a time series that gets
*less* blobby is not the same as a tree. An intermediate frame with a
visible stem and a spreading top was described here as "a genuine tree —
vertical trunk, branching crown" and the owner's correction was that it is
"better but still blobby". Judge against a clear bole and a foliage crown,
not against the previous frame.

## Sources (§7)

- [Crown shyness — Natural History Museum](https://www.nhm.ac.uk/discover/crown-shyness-are-trees-social-distancing.html)
- [Crown shyness: why some trees avoid touching leaves (IFLScience)](https://www.iflscience.com/crown-shyness-why-some-trees-avoid-touching-leaves-creating-a-fractured-canopy-59993)
- [The shade avoidance syndrome in Arabidopsis: phytochrome A and B differentiate vegetation proximity from canopy shade](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC4204825/) — the far-red proximity signal, distinct from shading
- [Crown shyness in various tree species (IJSDR)](https://www.ijsdr.org/papers/IJSDR1812056.pdf)
