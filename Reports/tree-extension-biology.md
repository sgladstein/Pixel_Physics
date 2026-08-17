# How a real tree keeps extending, and what actually stops it

Research pass against `Reports/tree-shape-problem-statement.md` §5. Judged
throughout by three tests: *is it local*, *does it bound without an
arbitrary cap*, *does it produce structure rather than mass*.

## The headline, in two sentences

**Extension sustains itself because every unit of extension deposits a
bud** — the meristem reservoir grows with the shoot system instead of being
a fixed starting allowance. **It is bounded because a shoot apex is limited
by a positional, geometric quantity — turgor at the apex, set by height —
not by a physiological one.**

That second half is the whole answer to §4. §4 established that every local
"am I idle" signal saturates together once growth stops: carbon fills every
cell to the cap, crowding decays everywhere within two ticks, conductance
relaxes to basal everywhere because there is no flux. **Geometry does not
do that.** When the tree stops growing the apex is still at the top and the
collar is still at the bottom, permanently. A height-based gate can never
fire on every cell at once, because height is never uniform.

---

## 1. Meristem senescence is not real, and we implemented it

A shoot apical meristem is indeterminate and effectively immortal; there is
no evidence for meristem senescence in trees. In Great Basin bristlecone
pine (>4,800 years) neither shoot length, stem-unit number nor stem-unit
length trends with tree age.

The decisive experiment: Bond et al. grafted shoot tips from old-growth
Douglas-fir onto 2-year-old seedling rootstock and got **a 10-fold increase
in stem elongation rate within two years**. Their conclusion — *"size, not
age, drives developmental changes in height growth… Reduced carbon
assimilation does not play an important role in height growth decline."*

**So `ORGANISM_STALE_LIMIT` retiring a tip permanently is not a
simplification of biology — it is the mechanism the literature specifically
falsifies.** A tip's growth potential is a function of *where it is*, not
of how long it has been alive. Any timer-based retirement models the
disproven thing.

## 2. Buds are a stock, deposited by extension

The unit of shoot construction is the **metamer**: internode + leaf +
axillary bud. Buds form in leaf axils, so **extension manufactures its own
future meristems, one per node.** Palubicki et al. give a bud exactly four
fates: produce a new metamer, produce a flower, remain dormant (retaining
the possibility of growing later), or abort.

Most never activate. The reservoir has a name — the **bud bank** — and the
literature's own analogy is apt: dormant buds are to reiteration what the
seed bank is to regeneration. Epicormic buds persist for decades under the
bark, pushed outward by secondary growth while maintaining a vascular **bud
trace** to the pith; a bud that fails to keep its trace ahead of the
cambium is buried and dies.

**What this says about `max_active_tips: 14`.** Fourteen *concurrently
active* apices is defensible. What is wrong is that 14 is also the
*lifetime* total. Biology separates these completely: the active set is
small and throttled by apical dominance, while the reservoir is large,
grows monotonically with the shoot system, and depletes only by activation
or death.

## 3. What actually bounds height

Four candidates, ranked by what the evidence supports.

**Meristem senescence — rejected** (§1).

**Carbon balance — real, but it bounds *branches*, not height.** Mature
trees are generally not carbon-limited; a review of 51 studies found no
evidence that hydraulic limitation of assimilation explains growth decline.
Carbon balance is the right mechanism for self-pruning and crown recession
(which `tree-architecture-research.md` §1 already has), but note its shape:
shaded branches are shed by *preferential allocation to branches that
assimilate most* — a **relative** comparison between branches. **Relative
comparisons are exactly the class §4 rules out**: when everything idles,
every branch's relative standing equalizes together. This is the negative
result the brief asked for — the carbon bound has §4's failure mode built
in and cannot be the primary bound.

**Hydraulic / turgor limitation — this is the local, non-saturating one.**
Water potential at the apex falls with height for two reasons, one purely
geometric: gravity costs **0.01 MPa per metre**, unconditionally, and
longer xylem paths add resistance. Koch et al. measured the height gradient
in redwood (tallest sampled 112.7 m) and extrapolated a maximum of
**122–130 m**. Potkay et al. turn it into a usable growth law: Lockhart
expansion, growth ∝ `max(P − Γ, 0)`, apex potential `ψ_root − ρgH` minus
path resistance; with `Γ = 0.75 MPa` they predict a 45 m ceiling for Scots
pine, matching observation.

**The bound is derived, not imposed:**
`H_max = (ψ_source − ψ_osmotic − Γ) / (ρg + path term)`. Three species
numbers give a ceiling — exactly what §5 asks for instead of "stop at N
cells" — and it is per-species for free: low `ψ_source` / high `Γ` gives a
shrub, a very low ceiling gives a moss mat, a high one gives a redwood,
from one rule.

**Mechanical abrasion — real, secondary.** Over 50% of lateral branches of
*Fagus*, *Carpinus* and *Tilia* broke from abrasion at least once in six
years. Worth knowing only because it means crown-shyness gaps are
*over-determined* in reality, so implementing them strongly is not
unrealistic.

**The hydraulic bound bounds height, not width.** Lateral extent still
needs light, crown shyness and self-pruning.

## 4. Growth is episodic, and that supplies the right semantics

Trees do not extend continuously. A **growth unit** is one uninterrupted
elongation episode; an **annual shoot** is everything a bud produces in a
year. In *fixed growth* (pine, oak) next year's entire shoot is preformed
in this year's bud; in *free growth* (willow, poplar) buds keep initiating
while conditions allow. The rhythm is **endogenous, not merely climatic** —
pedunculate oak flushes on 18–22 day cycles under constant conditions.

A rhythm is not a bound. It matters for two other things:

1. It gives the **correct semantics for a stalled tip**: a real apex that
   stops does not die, it *sets a terminal bud and rests*. That is one
   state change from what the engine does, and it is the difference between
   "growth is over forever" and "growth resumes next flush".
2. It **quantises extension**, so a re-armed tip extends one bounded growth
   unit rather than running.

A phase counter in the per-organism upkeep pass is legal under §5 — it is a
clock, not an idle signal, so it cannot saturate.

## 5. Reiteration is how a sapling architecture becomes an oak

The **architectural unit** is the species-specific developmental sequence a
young plant expresses — and it is *finite*. **Reiteration** is the mature
plant repeating that unit; it "seems to be a move backwards within the
plant's developmental sequence… the plant expresses again the juvenile
growth pattern", and a mature tree shows the traces of successive waves of
it.

The magnitude is documented: in old-growth Douglas-fir, epicormic branches
are **14.6–47.5% of live branches per tree**, and foliage on 450-year-old
trees is maintained by continuous epicormic shoot production.

**This is where "structure, not mass" comes from.** Borchert & Honda's
result is that branch number cannot keep increasing geometrically per order
if viable leaf area per terminal branch is to be maintained, so an axis's
contribution decays with branching order. **A tree without reiteration
therefore converges to a finite sapling — a whip, which is exactly what the
engine produces.** Reiteration adds size by stamping another copy of the
finite unit, so growth adds *architecture* rather than volume. That is
structurally the opposite of thickening.

---

## Recommendations, ranked by effect ÷ cost

### R1. Buds are a stock deposited *by extension*, each firing at most once

Every `plastochron` cells, a growing tip marks the cell it vacates as a
**dormant bud** — one flag, set at construction, and `plastochron` already
exists as a `Behavior` field. A dormant bud leaves that state only once:
flush, abort, or die.

**This is the structural answer to §4's runaway, and it is not a cap.**
Budding potential becomes proportional to *extension already performed*,
held in a **depleting stock**, rather than to volume. **Thickening deposits
no buds — so a blob, by construction, generates no new growth potential.**
That asymmetry between the two mechanisms is precisely what the problem
statement says is missing, and it comes from biology rather than tuning.

Separate `max_active_tips` (concurrency throttle, fine at ~14) from
lifetime tip count (should be unbounded and supply-driven). Conflating them
is the actual defect.

**R1b, bundled and nearly free: `SecondaryThicken` kills buds it covers**,
with a per-species survival chance. Literal biology — a bud that cannot
keep its trace ahead of the cambium is buried. It gives real bud mortality,
makes old wood progressively barer, and therefore **produces a clear bole
for free**, which `tree-architecture-research.md` §0 identifies as missing.

### R2. Retirement becomes dormancy; a flush clock re-arms it

Replace "stale for `ORGANISM_STALE_LIMIT` → `MatureBody` forever" with
"stale → set terminal bud → dormant". A phase counter in the upkeep pass
marks flush events; on a flush, dormant buds become *eligible* (still
subject to R3). This deletes the disproven senescence model and ends
"growth is over forever". **R2 alone restores the runaway — it must ship
with R3.**

### R3. The bound: a turgor gate on *height*, not on resource state

Gate tip extension and bud release on `max(P − Γ, 0)` where
`P ≈ ψ_source − k·h`, and `h` is the cell's height above the organism's
root collar.

- **Local**: cache `base_y` once per organism in the upkeep pass; every
  cell then computes `h = base_y − y` with no traversal.
- **Non-saturating**: height does not equalize when growth stops. This is
  the only surveyed signal with that property.
- **Derived bound**: `H_max = (ψ_source − ψ_osm − Γ) / k`.
- **Per-species free**: `Γ` and `ψ_source` give moss, shrub, tree, giant
  from one rule.

**Trap, stated explicitly:** modulate with supply, but **gate on height**.
A gate written against carbon or conductance inherits §4's saturation
exactly.

**What is not local, and the cheapest approximation.** The path-length half
needs distance along the vascular path, which is a traversal. Prefer
**height alone** — it captures the gravitational term exactly, and for a
broadly upright plant height is monotone in path length anyway.

### R4. Vigour and reiteration reset — the mechanism that makes structure

Carry a small `vigour` float on each tip, scaling per-flush extension and
branch chance, decremented on lateral branching (morphogenetic drift).
Geometric decay makes each axis's contribution finite, so the architectural
unit is bounded — a correct sapling.

**Reiteration**: a bud that flushes under high local light near the crown
surface **resets vigour to the juvenile value**, producing a fresh
orthotropic axis — another copy of the unit.

**Why this converges, and why it is the 2D answer.** High light exists only
on the crown *surface*; self-shading and the organism-blind
`canopy_density` make the interior dark. So reiteration is gated on a
**perimeter**, which in 2D scales as `R`, while volume scales as `R²`. **A
surface-gated rule therefore loses to any volume-gated rule as the plant
grows, and converges.** This is the exact inverse of the reverted bud
break, which was volume-gated and therefore had to run away.
`tree-architecture-research.md` §7a treats 2D's `R` vs `R²` as a liability;
for a surface-gated growth rule it is an **asset**.

### R5. A per-flush activation budget, and later the auxin channel

After R1/R2 a flush makes many buds eligible at once; if all fire you get
fuzz rather than limbs. A per-organism per-flush budget is the throttle;
apical dominance (the already-scoped auxin channel) is the principled
version that decides *which* buds win.

**Note the reframing.** §4 rejected a per-organism cap because it turned
exponential growth into linear growth that still filled the world. That was
correct when the cap had to supply the *size* bound. With R3 supplying
that, a budget does a different job — **shape** — and is legitimate. Caps
were never wrong per se; they were wrong as bounds.

### R6. `sylleptic_fraction` as a species float

Sylleptic branching develops continuously from the parent apex, with no
rest and no complete bud; proleptic develops after a rest, from a formed
bud, at a bud-scale scar. **The engine's current `branch_chance`-at-the-tip
*is* syllepsis — branch now, from the apex, no reservoir — which is exactly
why there is no reservoir.** Prolepsis is the mode with the stock/delay
semantics the problem needs. Keep both as a species float: `1.0` for a
vine or poplar, `0.0` for a rhythmic oak.

---

## Explicit negative results

- **There is no local "am I idle" bound in biology either.** Real bounds
  are of two kinds: **positional/geometric** (height, path length,
  buried-by-cambium) and **market-relative** (a branch's carbon balance
  *compared to competitors*). The second kind has §4's saturation problem
  intrinsically — which is why the reverted bud break failed, and why it
  would fail again in any resource-state formulation. **Build the bound out
  of geometry; use resources only to modulate.**
- **Meristem senescence is not real.** Any timer-based tip retirement
  implements a mechanism grafting experiments falsify.
- **Episodic growth is not a bound**, only a rhythm. It fixes "forever" and
  supplies no ceiling.
- **Whole-tree carbon accounting is not local.** The existing
  canalization-based stand-in (starve the poorly-connected first) is the
  right cheap approximation and should not be replaced with a global one.
- **The hydraulic bound bounds height only.** Width still needs light,
  crown shyness and self-pruning.

## Sources

- [Bond et al., grafting old-growth Douglas-fir tips (Tree Physiology 27:441)](https://academic.oup.com/treephys/article/27/3/441/1670235) — size, not age
- [Mencuccini et al., Ecology Letters 2005](https://onlinelibrary.wiley.com/doi/abs/10.1111/j.1461-0248.2005.00819.x) — no meristem senescence
- [Koch et al., The limits to tree height (Nature 428:851)](https://www.nature.com/articles/nature02417) — 122–130 m redwood ceiling
- [Potkay et al., turgor-driven growth law (Tree Physiology 42:229)](https://academic.oup.com/treephys/article/42/2/229/6325580) — `max(P − Γ, 0)`
- [Ryan, Phillips & Bond, hydraulic limitation reviewed (PCE 2006)](https://onlinelibrary.wiley.com/doi/10.1111/j.1365-3040.2005.01478.x)
- [Palubicki et al., Self-organizing tree models (SIGGRAPH 2009)](https://algorithmicbotany.org/papers/selforg.sig2009.html) — the four bud fates
- [Meier, Saunders & Michler, epicormic bud banks (Tree Physiology 32:565)](https://academic.oup.com/treephys/article/32/5/565/1735443)
- [Ishii & Ford, epicormic branches in old-growth Douglas-fir (Can. J. Bot. 2001)](https://cdnsciencepub.com/doi/10.1139/b00-158) — 14.6–47.5% of live branches
- [Barthélémy & Caraglio, plant architecture / physiological age (Annals of Botany 99:375)](https://academic.oup.com/aob/article-abstract/99/3/375/2464324)
- [CIRAD/UVED, Reiteration](https://greenlab.cirad.fr/GLUVED/html/P1_Prelim/Bota/Bota_unit_006.html) and [branching delays](https://greenlab.cirad.fr/GLUVED/html/P1_Prelim/Bota/Bota_typo_010.html)
- [Borchert & Honda, theoretical models of branch formation](https://pmc.ncbi.nlm.nih.gov/articles/PMC7082385/)
- [Mechanical abrasion bounds lateral crown expansion (FEM 2015)](https://www.sciencedirect.com/science/article/abs/pii/S0378112715001486)
- [Endogenous flush rhythm in pedunculate oak](https://pmc.ncbi.nlm.nih.gov/articles/PMC4765786)
- [Iowa State, Tree Anatomy 101](https://naturalresources.extension.iastate.edu/forestry/tree_biology/101.html) — fixed vs free growth

**Sourcing caveat:** several primary PDFs (Barthélémy & Caraglio 2007,
Meier et al. 2012, Hallé 1986) were unreachable — PMC captcha walls and
scanned-image PDFs — and were worked from abstracts plus secondary
summaries. Flagged rather than presented as full-text reads.
