# How other systems build trees, and what transfers

Survey against `Reports/tree-shape-problem-statement.md`. Read alongside
`Reports/tree-extension-biology.md` — the two were researched independently
and converge on the same mechanism, which is the strongest signal in either.

## 0. The finding, stated first

Every model surveyed agrees on one thing, and it is the exact inverse of
what this engine does:

> **Extension is never one-shot, because a tip never owns a budget. Every
> growth cycle the whole plant's income is recomputed at the base and
> re-divided among the surviving frontier. A bud receives an *allocation*,
> not a *reserve*.**

This dissolves §4's impasse directly. Bud break failed because it asked a
*local* question — "do I have surplus carbon and low crowding?" — and every
mature cell answered yes at once. **No model in the literature ever asks
that question.** They ask "what *share* of the plant's total income did I
get?", and a share cannot saturate everywhere, because shares sum to one.
`carbon` clamps to `RESOURCE_SCALE` everywhere; a share is competitive by
construction.

The second finding is about the blob:

> **In every model surveyed, girth is a derived quantity, not a process.**
> Diameter is recomputed each cycle from the number of tips supported. A
> plant that stops extending stops thickening — not by tuning, but because
> there is no term that could add width.

`SecondaryThicken` is a free-standing process with its own knob. **That is
the knob every attempt has turned**, and it is why tuning slides along a
mass axis with structure absent at both ends. Nothing in the literature has
that knob to turn.

---

## 1. Palubicki et al. 2009 — the closest prior art

[Self-organizing tree models for image synthesis](https://algorithmicbotany.org/papers/selforg.sig2009.html),
*ACM TOG* 28(3). From an authored **seedling**, each growth cycle runs:

1. calculate local environment of buds → per bud a scalar `Q` (light) and
   vector `V` (optimal direction)
2. determine bud fate — the two-pass allocation
3. append new shoots
4. shed branches
5. update branch width

### The extended Borchert–Honda allocation

**Pass 1, basipetal (tips → base).** `Q` accumulates downward; at each
branch point the internode stores `Q_m` (continuing axis) and `Q_l`
(lateral). At the base, `v_base = α · Q_base` — **the only place total
income enters**, and it is bounded by intercepted light.

**Pass 2, acropetal (base → tips).** Resource `v` splits at each branch:

```
v_m = v · λQ_m / (λQ_m + (1−λ)Q_l)
v_l = v · (1−λ)Q_l / (λQ_m + (1−λ)Q_l)
```

`λ > 0.5` biases the main axis (**excurrent**); `λ < 0.5` biases the
lateral (**decurrent**). Their Fig. 7 sweeps λ over 0.46–0.54 and that
±0.04 band spans the entire excurrent↔decurrent range, because the bias
applies multiplicatively at *every* branch point and compounds with depth.

This is `tree-architecture-research.md` §2's `β_x`, but in the correct
form: **a multiplicative split of a conserved flow**, not a conductance
nudge. Biasing conductance changes which face gets more, but nothing
enforces that the two shares sum to the parent's supply — so it cannot
compound.

### The terminator, and the single most important line

```
n = ⌊v⌋        metamers this bud produces
l = v / n      length of each internode
```

**The floor is the terminator.** A bud allocated `v = 0.9` produces *zero*
metamers and does not grow — but does not die, is not retired, and costs
nothing. Next cycle its share is recomputed; if a neighbour is shed or
shaded out, its share rises above 1 and it resumes. **Soft, reversible,
competitive dormancy** — the direct answer to §5.

Contrast: a tip that fails for `ORGANISM_STALE_LIMIT` (4) ticks retires
**permanently**. Palubicki's buds fail constantly and retire never.

### Shedding — and a warning that applies directly to us

Shedding follows Takenaka: total light gathered by a branch versus branch
size in internodes; below a threshold it is a liability and is shed. This
is what produces tall boles. **The paper's own caveat is a warning for
this codebase:**

> "The above method is more suitable for models that rely on shadow
> propagation rather than space colonization. In the latter case, the
> binary nature of the environmental input (Q = 1 or Q = 0) would cause
> branches to be shed immediately after they stop growing."

`tree-architecture-research.md` §1 wants self-pruning and §7c makes the
crowding channel organism-blind (a binary-ish proximity veto). **Combining
a graded shed rule with a binary space signal is a known failure mode** —
branches vanish the instant they stop extending. If self-pruning lands, the
light signal feeding it must be graded.

### Diameter

`d^n = d₁^n + d₂^n`, `n ∈ [2,3]`, basipetal. And critically:

> "Importantly, branch width is **not** decreased when leaves and branches
> are shed or pruned. The model thus requires a **memory of past leaves and
> branches**."

Girth is a **monotone high-water mark of past frontier**, not a function of
current mass. That is one per-cell scalar under a max-accumulate, and it is
the correct shape for `SecondaryThicken` to become.

### Can the allocation be local? Yes, and cheaply

**It is not a whole-tree traversal per cell — it is two extra linear sweeps
inside a pass that already does one.** `organism::transport` already
builds, once per organism per tick, a determinism-sorted cell vector plus a
flat 4-neighbour adjacency table, explicitly justified in-comment because
adjacency cannot change inside the loop.

Add: (1) one BFS from the base over that table for a rooted parent ordering
— O(N), and note a thickened trunk is a 2D *blob*, so the BFS yields a
spanning tree, made deterministic by the existing sort; (2) a basipetal
accumulate in reverse order; (3) an acropetal distribute forward. Three
extra O(N) passes per organism per 45 frames, over arrays already
materialised.

`OrganismState` already sets the precedent and sanctions it: `root_cells`/
`shoot_cells` are refreshed "once per organism tick while it is already
walking the cell list", with the doc noting "allometry is a genuine
whole-plant property, and no local rule can compute 'am I mostly root'."
Vigour allocation is the same category.

**Cheapest fallback if even that is too much:** keep the basipetal pass (it
gives both `Q_base` and pipe-model girth) and replace the acropetal one
with a per-organism broadcast — `v_i = v_total · Q_i / Q_base`. This loses
λ, so it loses the excurrent/decurrent axis, but **keeps the property that
matters**: allocation is a competitive share that cannot saturate, and
`⌊v⌋` still gives reversible dormancy.

---

## 2. Space colonization (Runions, Lane & Prusinkiewicz 2007)

Attraction points fill a crown **envelope**. Each point influences the
nearest node within radius of influence `d_i`; a node with a non-empty
influencing set grows one step of length `D` along the normalised sum of
directions to its points; points within **kill distance** `d_k` of any node
are deleted permanently.

**The result that matters most here**, quoted directly:

> "A comparison of Figures 5 and 6 also shows that **narrower trees have a
> clearly delineated trunk, whereas in widely spread trees even the main
> limbs are highly ramified.** This correlation between the overall form of
> the trees and their branching habits is an emergent property of the
> algorithm, and captures the defining properties of excurrent and
> decurrent tree forms."

**This is the whip/blob axis, named and explained.** They are the two ends
of the *envelope aspect ratio*, and thickening cannot escape them because
thickening does not touch the envelope. **The engine has no envelope at
all.**

They also raise attractor density *near the envelope surface*, reproducing
the real rise in branch density at the crown surface — the mechanism behind
"a crown is a shell, not a volume".

### As a CA field — the natural fit, with one trap

| Space colonization | CA equivalent |
|---|---|
| attraction point set | per-cell `vacancy` scalar on empty cells |
| **envelope** | **the light field** — attractors exist in unshaded air |
| radius of influence `d_i` | the tip's read window |
| growth direction | vacancy-weighted direction sum |
| kill distance `d_k` | occupancy/crowding veto in a radius |
| termination | field locally exhausted |

Using **light as the envelope** satisfies "bounded without an arbitrary
cap": the crown can only occupy lit air, so the envelope is emergent, and a
neighbour's shade deletes your attractors — delivering §7c's crown shyness
from the same rule, with the same organism-blindness, since light does not
know whose leaf blocked it.

**The trap, and it is real:** `canopy_density` is *deposit–diffuse–decay*.
An attractor field must be a **consumable stock with memory and no decay**;
the entire self-limiting property is that markers are removed
*permanently*. Reusing `canopy_density` because it is the field already
there would silently delete the bounding mechanism.

**Cheapest approximation:** do not store consumption at all. "Consumed" and
"occupied-or-adjacent-to-my-own-material" are nearly the same set, and the
second is already computed. Make crowding a **hard veto within a kill
radius** rather than a score weight, and make the tip climb the light
gradient rather than scoring discrete candidates.

---

## 3. L-systems — least transferable, because the engine already is one

A context-sensitive L-system with length-preserving productions **is**
formally a cellular automaton; the equivalence is established. The
`Behavior` dispatch over `CellType` is the 2D generalisation. Open
L-systems' `?E(x)` communication module — plant emits position and vigour,
environment replies with a scalar, apex fate follows — is what `Grow`
reading world fields already does. **Nothing to port.**

Three things do transfer:

**Branch order as a per-cell integer with per-order species parameters.**
Classical tree L-systems are parameterised by arrays indexed by branch
order: direction deviation, spacing between branch points, and the
threshold below which an apex dies. **This is the highest value-per-line
item in the survey. Fourteen tips draw a whip because they are fourteen
copies of the same rule.** Give a tip an order — inherited on straight
extension, incremented on lateral branching — and index species parameters
by it, and one mechanism produces trunk/limb/twig differentiation, taper,
and per-order angles. The room exists: `pack_cell_type` uses bits 0–3 and
the doc states bits 4–15 are free. Fully local. Lands exactly on §0b's
per-species requirement — a short array is a shrub, a long one a tree.

**The vigour decrement.** Takenaka carries `VD = 0.95` — apex vigour × 0.95
per step, plus a fixed length reduction relative to the mother segment at
each branch. One `f32`, fully local, produces geometric taper. Honestly a
soft version of an arbitrary cap, but nearly free and the structural payoff
(self-similar taper) is real.

**The authored axiom.** **Every model surveyed starts from an authored
seedling, not a single cell.** Palubicki's input is a "seedling structure";
Runions begins with "one or several tree nodes"; Takenaka's axiom is an
internode, a leaf and an apex already assembled. The engine starts from one
`Seed`. Making the axiom 3–5 cells of trunk plus a terminal bud and a
couple of dormant laterals guarantees a bole from frame zero, for one
function.

---

## 4. 2D games — nobody simulates this, and that is the finding

| Game | How trees are made |
|---|---|
| **Terraria** | Instant and authored. Sapling waits a random delay then swaps to a complete tree — "the growth is not gradual." Needs 9–20 tiles of vertical clearance and two clear tiles either side to place branches. Treetops are pre-drawn sprites. |
| **Starbound** | Stem asset × foliage asset, chosen independently — trunk and crown are two orthogonal authored axes combined combinatorially. |
| **Dwarf Fortress** | A typed part taxonomy: roots, trunk sections (1×1, 2×2, 3×3), thick branches, branches, twigs, with "growths" layered by species and season. |
| **RimWorld** | One sprite, scaled by growth %. No structure at all. |
| **Noita** | The direct architectural peer — every pixel simulated — and vegetation is **authored bitmap "pixel scenes"** up to 512×512, stamped by the generator. The physics acts on them afterwards; it does not grow them. |

Noita is worth dwelling on: an engine built on exactly this premise chose
to stamp authored bitmaps and let the sim take over.

**What a 2D pixel tree needs to READ as a tree** — all five converge, and
this should be the acceptance test:

1. **A clear vertical trunk of near-constant width carrying nothing low
   down.** Terraria enforces it structurally: branch sprites need two clear
   tiles either side, so a trunk in a tight space has no branches.
2. **A crown that is a distinct mass, offset upward and separated from the
   trunk.** Starbound's stem/foliage split and DF's trunk/growths split are
   *literally two different asset classes*. The separation is enforced by
   construction in every case, never emergent.
3. **Foliage as area, wood as line.** Trunk a handful of pixels, crown
   hundreds of pixels of leaf. Our measured **48:1 wood:leaf** is inverted
   from all of them.

The transferable *mechanism* is small: **Terraria's clearance
precondition** — space colonization's `Q` in its crudest form, an upward
raycast. Our light field is already an upward-clearance integral, so it is
nearly free.

The uncomfortable transferable *lesson* is that these games guarantee the
silhouette by making the mechanism **unable to produce anything else**. A
trunk that structurally cannot carry low branches, and a crown that can
only exist above a height threshold, are cheaper and more reliable than
tuning a system that *could* produce a blob into usually not doing so.

---

## 5. What actually stops growth

| Mechanism | Where | How it bounds | Localizable? |
|---|---|---|---|
| **Competitive share + `⌊v⌋` floor** | Palubicki | income `αQ_base` bounded by intercepted light; shares sum to it; below 1.0 a bud makes zero metamers but stays alive and reversible | **Yes** — 2 extra O(N) passes |
| **Attractor exhaustion** | Runions | halts when the field is locally consumed | **Yes** — needs a non-decaying consumable |
| **Superlinear maintenance** | Takenaka | `BM · size^BE`, **BM = 0.32, BE = 1.5**; income ~ surface (sublinear), cost ~ size^1.5 — they cross | **Yes**, given pipe-model girth |
| **Shed-on-liability** | Takenaka | light/internodes below threshold → shed; produces the bole | **Yes**, but needs a *graded* light signal |
| **Bud bank depletion** | biology | stock is finite, laid down at shoot formation, net-declining | **Yes** — fully local |
| **Derivation count** | L-systems | explicit iteration cap | the arbitrary cap §5 forbids |

**The two that best satisfy "bounded without an arbitrary cap":**

**(a) The `⌊v⌋` floor.** Build the design around this. It **bounds growth
without ever killing anything**: extension continues for life, at a rate
set by how income divides among the frontier, and self-throttles because
adding tips divides the same income more ways. Below 1, extension pauses.
The plant is bounded, alive, responsive, and **resumes locally the moment a
competitor is removed** — which is the property that makes destruction feel
like it matters. Cut a limb and neighbouring dormant buds visibly restart.

**(b) Superlinear maintenance — and this corrects
`tree-architecture-research.md` §1.** **A flat per-cell upkeep does not
bound anything.** Cost linear in mass against income roughly linear in leaf
count can balance at any size forever. Takenaka's exponent `BE = 1.5` is
what makes the curves cross. The full constant set:

```
LP 8     photosynthate production        PB 0.8   needed to branch
LM 2     leaf maintenance                PG 0.4   needed to grow
BM 0.32  branch maintenance coefficient  VD 0.95  apex vigor decrement
BE 1.5   branch maintenance EXPONENT     Nmin 25  shedding threshold
LS 5     leaf lifespan
```

A leaf earns 8 and costs 2 — a large, *constant* margin — while structural
cost is superlinear. That is the whole economy.

And Takenaka's own paper reports the mechanism works: *"If the growth was
unlimited, the number of terminal branch segments would double every year.
Due to the competition for light, the number of terminal segments observed
in an actual simulation increases more slowly."* **Extension continues
indefinitely; the *rate* is bounded.** The engine is the opposite — rate
unbounded per tip, *duration* bounded.

**Cheapest superlinearity:** charge upkeep proportional to girth. Under the
pipe model girth already *is* the accumulated count of supported tips, so
`upkeep ∝ d^1.5` is free once the basipetal pass exists.

---

## 6. Recommended composition

Items 1–4 are one design, not four:

1. **Branch order** (`u4` in free `aux` bits, per-order species arrays).
   Structural differentiation immediately, independent of everything else,
   near-zero cost. **Ship first — testable alone.**
2. **Bud bank** (`DormantBud` cell type, created only by extension). Fixes
   "nothing ever creates a new tip", runaway-proof by construction.
   `CellType` uses 5 of 16 variants, so this is free.
3. **Basipetal pass** over the existing `transport` topology — yields
   `Q_base`, pipe-model girth as a monotone high-water mark, and branch
   size for upkeep, from one sweep. **This is where `SecondaryThicken`
   stops being a process and becomes a derived quantity, which is what
   kills the blob.**
4. **Acropetal pass with λ and `n = ⌊v⌋`** — the sustain-and-bound
   mechanism and the excurrent/decurrent knob. Subsumes (2)'s release rule
   and makes dormancy reversible.

Then, lower priority: light-as-attractor-envelope for crown shape and
shyness, and superlinear upkeep as an absolute bound — with the
graded-signal caveat if self-pruning lands alongside.

**Two things to measure that existing metrics will not catch:**

- **Whether `v` per bud is actually competitive.** The failure mode is that
  `Q_base` grows in proportion to bud count, every share stays above 1
  forever, and it degenerates into unbounded extension. Log the
  *distribution* of `v`, not the total — a healthy plant shows most buds
  below 1.0 and a few above.
- **Whether dormancy is reversible.** Cut a limb at frame 10,000 and
  confirm neighbouring dormant buds restart. If they do not, the mechanism
  has become `ORGANISM_STALE_LIMIT` with extra steps — and per `CLAUDE.md`'s
  ethos note, that visible resumption after damage *is* the payoff for
  choosing this over a cap.

## Sources

- [Self-organizing tree models for image synthesis — Pałubicki et al., *ACM TOG* 28(3), 2009](https://algorithmicbotany.org/papers/selforg.sig2009.html) ([PDF](https://algorithmicbotany.org/papers/selforg.sig2009.small.pdf))
- [Modeling Trees with a Space Colonization Algorithm — Runions, Lane & Prusinkiewicz, EGWNP 2007](https://algorithmicbotany.org/papers/colonization.egwnp2007.large.pdf)
- [Visual Models of Plants Interacting with Their Environment — Měch & Prusinkiewicz, SIGGRAPH 96](https://algorithmicbotany.org/papers/enviro.sig96.pdf) — open L-systems and the Takenaka carbon economy constants
- [Epicormic buds in trees — Meier, Saunders & Michler, *Tree Physiology* 32, 2012](https://doi.org/10.1093/treephys/tps040)
- [Representation of some cellular automata by means of equivalent L systems — Alfonseca & Ortega](http://arantxa.ii.uam.es/~alfonsec/docs/artint/compi200.pdf)
- [Trees — Terraria Wiki](https://terraria.wiki.gg/wiki/Trees) · [Starbound root tables](https://starbounder.org/Modding:Lua/Tables/Root) · [DF2014:Tree](https://dwarffortresswiki.org/index.php/DF2014:Tree) · [RimWorld plant rendering](https://rimworldwiki.com/wiki/Modding_Tutorials/Plant_Rendering) · [Noita custom environments](https://noita.wiki.gg/wiki/Modding:_Making_a_custom_environment)
