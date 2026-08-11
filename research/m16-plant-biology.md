# M16 research: plant biology, for scientifically-grounded plant mechanisms

Raw findings from two research passes, kept in full here because `PLAN.md`'s
M16 section only carries a condensed synthesis. This is the source material
to build the actual M16 implementation against — read this, not just the
plan summary, when writing the code.

Two passes: an initial broad pass (water uptake, phototropism, moss/lichen
ecology, growth rates), and a deep-dive follow-up specifically on root
system architecture and hormonal signaling (requested explicitly because the
first pass's coverage of "root growth" and "signaling" was judged too thin
to build from).

---

## Pass 1: water uptake, phototropism, moss/lichen ecology, growth rates

### Root water uptake — simulatable mechanism

Real uptake is driven by a water-potential gradient (soil water potential is
higher/less negative than root-cell water potential), not active pumping —
osmosis is passive. Root cells actively pump mineral ions (K+, Ca2+, NO3-)
via ATP, which lowers internal water potential and *steepens* the gradient,
and root hairs work purely by maximizing surface-area contact with soil
water. Above ~1m, capillary action can't lift water — actual long-distance
transport is the **cohesion-tension mechanism**: transpiration at leaves
creates negative pressure (~-2 MPa) that pulls a continuous, cohesive water
column up through xylem "straws," with adhesion/cohesion/surface-tension
only handling capillary rise below ~1m.

Simulatable translation for a per-cell scheduler: give each root cell a
scalar "local water potential" (lower = more depleted/thirsty); adjacent
water cells satisfy it and the deficit propagates one step toward the
plant's base each tick (cheap stand-in for the pulled-column effect) rather
than requiring a full network solve. A root should also *prioritize*
adjacent water over non-adjacent (real root hairs only access water within
microns), and growth should bias toward cells with more neighbouring
water/lower local moisture-competition, mimicking real roots proliferating
in moist soil patches (hydrotropism/hydropatterning) rather than growing
uniformly.

Sources:
- [Role of root hairs in water uptake (Oxford/JXB)](https://academic.oup.com/jxb/article/73/11/3330/6552107)
- [ABSORPTION OF WATER lecture notes](http://eagri.org/eagri50/PPHY261/lec04.pdf)
- [Transpiration stream (Wikipedia)](https://en.wikipedia.org/wiki/Transpiration_stream)
- [Trees suck: physics of transpiration (ScienceDirect)](https://www.sciencedirect.com/science/article/pii/S0079610724001135)

### Phototropism — mechanism and prior art combining it with space colonization

Biological mechanism: blue-light photoreceptors (phototropins) on the lit
side trigger lateral (not just polar) auxin transport via PIN-protein efflux
carriers, so auxin accumulates on the *shaded* side, which acidifies cell
walls there (via proton pumps), activates expansins, and makes shaded-side
cells elongate faster — bending the organ toward light. This is a
differential-growth-rate mechanism, not literal "growth toward light."

Directly relevant prior art: **Palubicki, Horel, Longay, Runions, Lane,
Měch & Prusinkiewicz, "Self-organizing tree models for image synthesis,"
SIGGRAPH/ACM TOG 2009** — the direct sequel to the Runions space-colonization
paper already committed to in the plan. Adds competition for light via a
**voxel-grid "shadow propagation"** model: each branch casts a shadow into a
coarse 3D grid, local light value per voxel drives growth-direction
weighting and resource/auxin-like allocation between competing buds, plus
internal signaling that starves shaded branches. This maps almost exactly
onto the engine's *existing* M13 light field rather than requiring a
separate light model.

Sources:
- [Self-organizing tree models for image synthesis (SIGGRAPH 2009)](https://algorithmicbotany.org/papers/selforg.sig2009.html)
- [ResearchGate mirror](https://www.researchgate.net/publication/216813636_Self-organizing_tree_models_for_image_synthesis)
- [Light Distribution Models for Tree Growth Simulation, CGF 2025 (Nauber)](https://onlinelibrary.wiley.com/doi/10.1111/cgf.15268) — recent survey of light-model variants for this algorithm family; abstract accessible, full text paywalled
- [Phototropism: Bending towards Enlightenment (PMC/NIH)](https://pmc.ncbi.nlm.nih.gov/articles/PMC1456868/)
- [phot1/PIN auxin mechanism (PLOS Biology)](https://journals.plos.org/plosbiology/article?id=10.1371%2Fjournal.pbio.1001076)

### Moss/lichen substrate and moisture ecology

Moss is an opportunist that follows moisture, not a magnet drawn to a
direction. It favours north-facing/shaded surfaces specifically because
shade reduces evaporation and preserves a damp microclimate, not because of
any directional attraction. Moss lacks a waterproof cuticle and desiccates
quickly in direct sun (moss/lichen are *poikilohydric*: they can't regulate
internal water and depend entirely on ambient moisture). Substrate
texture/porosity also matters — rougher/more porous surfaces hold more
moisture. Lichens similarly need consistently moist rock and are strongly
shaped by aspect, wind-driven rain exposure, and low solar radiation.

Simulatable rule: moss/lichen spread probability on a stone cell should be a
function of (local shade/light-grid value — lower is better, adjacent water
or humidity, temperature — lower favours less evaporation), not a fixed
"north side" or flat "damp stone" rule.

Sources:
- [Moisture Interactions Between Mosses and Stone Substrates (Taylor & Francis)](https://www.tandfonline.com/doi/full/10.1080/00393630.2021.1892430)
- [Ecology of Lichens on Rock Surfaces (ResearchGate PDF)](https://www.researchgate.net/profile/Richard-Armstrong-17/publication/377499686_Ecology_of_Lichens_on_Rock_Surfaces/links/65aa2fbef323f74ff1cc878c/Ecology-of-Lichens-on-Rock-Surfaces.pdf)
- [Lichen Habitat (US Forest Service)](https://www.fs.usda.gov/wildflowers/beauty/lichens/habitat.shtml)

### Relative growth rates (order of magnitude, for tuning constants)

- Moss: ~0.5-4 cm/year linear spread (some species 10-20 cm/yr once
  established).
- Lichen: dramatically slower and more variable — 0.5-8 mm/yr typical
  temperate crustose species; as low as 0.005 mm/yr in Antarctic extremes;
  up to 500 mm/yr for fast fruticose species in ideal conditions.
- Trees: tens to 100+ cm/yr of new shoot growth for fast growers.

Roughly **lichen << moss << grass << tree** by 1-2 orders of magnitude at
each step. Suggested tuning anchor: lichen : moss : tree growth ≈ 1 : 10 : 100.

Sources:
- [Moss growth rate (Biology Insights)](https://biologyinsights.com/what-is-the-typical-moss-growth-rate/)
- [Extremely low lichen growth rates, Antarctica (Springer)](https://link.springer.com/article/10.1007/s00300-011-1098-7)
- [Lichen growth and development (earthlife.net)](https://earthlife.net/lichens/growth)

### Closest unified prior art (pass 1)

The **Palubicki et al. 2009 self-organizing tree paper** is the single best
match for this engine — direct successor to the committed-to space
colonization paper, already fusing light competition (shadow-voxel grid),
internal resource/signal allocation, and branch competition into one
coherent model. The Nauber 2025 CGF paper is the most current survey of
light-model variants for the same algorithm family. On the practical side:
Noita's own devlogs describe grass/moss as simple cellular-automaton spread
rules (not physically modeled) — confirms the simple-per-cell-tick approach
is the standard game-dev shortcut, useful context for how far to push
accuracy vs. cost.

---

## Pass 2 (deep-dive): root architecture and hormonal signaling

Requested as a follow-up specifically because pass 1's treatment of root
*growth* (as opposed to just water uptake) and plant *signaling* (as
opposed to just the one phototropism mechanism) was judged too thin.

### 1. Gravitropism (statolith mechanism) and its conflict with hydrotropism

**Biology.** Root tip **columella cells** (in the root cap) contain
**amyloplasts** (starch-dense plastids, ~1.5 g/cm³ vs ~1.1 g/cm³ cytoplasm)
that sediment to the lowest cell face under gravity. This triggers
relocalization of **NGR proteins**, which redirect **PIN3/PIN7 auxin efflux
carriers** to the new lower membrane face, creating asymmetric auxin flow
toward the lower side of the root. Auxin accumulating on the lower flank
*inhibits* cell elongation there (opposite of shoots), so the upper flank
elongates faster and the root bends downward.

When water availability conflicts with gravity, **MIZ1** actively
suppresses the gravitropic PIN-polarization machinery, letting a
moisture-gradient-driven PIN response dominate instead — genuine
antagonism, not a simple weighted average. Eliminating gravitropism (via
auxin-transport inhibitors) restores hydrotropic sensitivity in *miz1*
mutants, confirming the mechanism.

Lateral roots additionally maintain a genetically fixed **gravitropic
setpoint angle (GSA)** via a *balance* of two opposing auxin fluxes in the
columella (a constant "antigravitropic" upward flux vs. an angle-dependent
downward PIN3/PIN7 flux, tuned by RCN1/PP2A phosphoregulation) — this is why
lateral roots grow obliquely rather than straight down, not because of a
separate rule.

**Simulation translation.** Give each root tip node a `gravity_bias` vector
(strong pull toward local "down") and a `water_bias` vector (toward higher
water-potential neighbour cells, from pass 1). Root state carries a
`miz_active` flag that flips on when the local moisture gradient exceeds a
threshold, temporarily zeroing/reducing `gravity_bias` so water wins;
otherwise gravity dominates. This is a genuine antagonism switch, not a
blend — matches the biology more precisely than averaging the two vectors
every tick would. Lateral roots spawn with a fixed target angle offset from
the parent's growth vector (e.g. 60-90 degrees) rather than a free
direction, reproducing GSA cheaply as a per-node constant instead of
continuous flux math.

No dedicated graphics/procedural-generation paper was found reproducing GSA
computationally — the biology above is directly encodable as the rule
described, but there's no prior art shortcut to lean on here.

Sources:
- [Molecular Mechanisms of Root Gravitropism (Current Biology review)](https://www.cell.com/current-biology/pdf/S0960-9822(17)30873-4.pdf)
- [Rapid translocation of NGR proteins during root gravitropism (PMC)](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC10942638/)
- [Auxin-mediated statolith production for root gravitropism (New Phytologist 2019)](https://nph.onlinelibrary.wiley.com/doi/10.1111/nph.15932)
- [Foraging for water by MIZ1-mediated antagonism between gravitropism and hydrotropism (PNAS 2025)](https://www.pnas.org/doi/10.1073/pnas.2427315122)
- [Hormonal interactions during root hydrotropism vs gravitropism (Takahashi et al. 2009)](https://rootbiome.tamu.edu/wp-content/uploads/sites/38/2015/06/2009-Takahashi-et-al-Hormonal-interact-during-root-tropism-hydro-vs-gravitropism-art3A10.10072Fs11103-008-9438-x.pdf)
- [Antigravitropic PIN polarization maintains non-vertical lateral root growth (Nature Plants 2023)](https://www.nature.com/articles/s41477-023-01478-x)
- [Auxin Controls Gravitropic Setpoint Angle (Current Biology 2013)](https://www.sciencedirect.com/science/article/pii/S0960982213007598)

### 2. Root system architecture / branching

**Biology.** Lateral roots originate only from **pericycle founder cells**,
primed by **oscillating auxin-response pulses** in the basal meristem (an
internal "clock" — Moreno-Risueno et al. 2010) that set periodic, roughly
evenly-spaced priming sites as the root tip grows past them; only later do
local auxin maxima trigger actual founder-cell division. Measured branching
angles cluster ~30-90 degrees (classed horizontal/inclined/vertical), with
real interlateral spacing on the order of mm-cm depending on species, and
root:shoot ratio shifts environmentally (e.g. phosphorus starvation ->
denser, more-branched roots + reduced shoot branching).

**Simulation translation.** Instead of a flat per-tick branch probability,
run a simple **oscillator counter** on each growing root tip: every N
growth-ticks, mark the current node as a "primed" site; a primed site
sprouts a lateral root only if the local resource/auxin signal exceeds a
threshold. This gives naturally regular spacing "for free" and lets
environment (water/soil chemistry) gate whether priming becomes an actual
branch, rather than a memoryless random roll each tick.

Sources:
- [Lateral root initiation: one step at a time (De Smet 2012, New Phytologist)](https://nph.onlinelibrary.wiley.com/doi/10.1111/j.1469-8137.2011.03996.x)
- [Periodic Lateral Root Priming (Plant Cell 2017)](https://academic.oup.com/plcell/article/29/3/432/6099004)
- [A Comparative Analysis of Quantitative Metrics of Root Architecture (Plant Phenomics 2021)](https://spj.science.org/doi/10.34133/2021/6953197)

### 3-4. Apical dominance and the auxin/cytokinin push-pull — the key shape-control lever

This is the single most important finding for making procedural tree shape
look right, and it has a direct, implementable graphics citation.

**Biology.** Auxin made at the shoot apex flows **basipetally** (via
basally-localized PIN carriers) down the stem and suppresses cytokinin
biosynthesis and auxin export from axillary buds, keeping them dormant.
Cytokinin (root-synthesized, transported up) does the opposite — it
promotes PIN accumulation and auxin export *out* of buds, activating them.
Decapitation removes the apical auxin source, allowing buds nearest the cut
to activate first — this is the real mechanism behind "cutting the top off
a plant makes it bush out," a widely-observed effect with a precise cause.

**Prusinkiewicz, Mündermann, Karwowski & Lane, "Control of bud activation by
an auxin transport switch," PNAS 106:17431-17436 (2009)** formalizes this as
**auxin canalization**: each bud is a competing auxin source trying to
establish a self-reinforcing transport channel into the main stream;
whichever source achieves canalization first suppresses the others, and the
switch is **hysteretic** (hard to reverse once established). This is a
genuine graphics/procedural-botany paper implementing exactly this
mechanism computationally — not just biology to adapt, but a directly
portable algorithm.

**Simulation translation.** Each active bud/node computes a scalar "auxin
channel strength" toward the trunk; update it via simple positive feedback
(`strength += flow`, flow capacity grows with strength, saturating) each
tick. A bud only sprouts a branch once its channel strength crosses a
threshold; a dominant apex's already-large channel strength suppresses new
channels below it (shared pooled auxin budget per parent segment). Cytokinin
is just a second scalar field diffusing from root nodes upward, added
positively to bud activation probability — implementing the auxin/cytokinin
ratio as two small per-node numbers, not a PDE. This is cheap enough for the
same per-cell scheduler everything else in M16 uses.

Canalization itself was originally formalized by Mitchison (1980/1981),
following Sachs' original hypothesis — Prusinkiewicz et al. 2009 is the
citation to actually implement from.

Sources:
- [Control of bud activation by an auxin transport switch (Prusinkiewicz et al., PNAS 2009)](https://www.pnas.org/doi/10.1073/pnas.0906696106)
- [Auxin canalization: From speculative models toward molecular players (Curr Opin Plant Biol 2022)](https://www.sciencedirect.com/science/article/pii/S1369526622000036)
- [Receptor kinase module targets PIN-dependent auxin transport during canalization (Science 2020)](https://www.science.org/doi/10.1126/science.aba3178)
- [Cytokinin Targets Auxin Transport to Promote Shoot Branching (Plant Physiology 2018)](https://pmc.ncbi.nlm.nih.gov/articles/PMC6001322/)

### 5. Systemic stress signaling (bonus — optional future mechanic)

**Biology.** Wounding/heat releases glutamate and ATP, triggering
**GLR3.3/GLR3.6-gated Ca2+ waves** and electrical signals that propagate
whole-plant via phloem/plasmodesmata within seconds-minutes, priming distal
tissue defenses; a parallel **ROS wave** mechanism specifically operates
under heat stress. Fire-adapted trees resprout epicormically from protected
buds using stored reserves.

**Simulation translation.** On cell damage/heat-spike (the engine already
has both, from M14/M15), push a short-lived "signal" value along the
plant's graph edges (parent/child links) at a fixed propagation speed per
tick, decaying with distance — a cheap flood-fill rather than any field
solve. Nodes receiving it could locally boost stress-response state (e.g.
prioritize reserve allocation, trigger dormant bud activation for
resprouting after fire damage). No procedural-graphics implementation of
this was found — it's biology-only, offered as an optional mechanic to
revisit if M16's core lands and there's appetite for more depth, not part
of the core build.

Sources:
- [Glutamate triggers long-distance, calcium-based plant defense signaling (Science 2018)](https://www.science.org/doi/10.1126/science.aat7744)
- [Electrical and calcium signaling in plant systemic defense (New Phytologist 2025)](https://nph.onlinelibrary.wiley.com/doi/10.1111/nph.70301)
- [Mesophyll cells mediate systemic ROS signaling during wounding or heat stress (bioRxiv)](https://www.biorxiv.org/content/10.1101/2021.02.02.429427.full.pdf)
- [How Do Plants Respond Biochemically to Fire? (Forests 2021)](https://doi.org/10.3390/f12010056)

---

## Summary: what this means for the M16 build

Priority order if implementing all of this is too much for one pass:

1. **Root growth = gravity_bias + water_bias with a MIZ1-style antagonism
   switch**, not just "grow toward water." This is cheap and directly
   replaces the pass-1 placeholder.
2. **Auxin canalization for branch/bud activation** (Prusinkiewicz 2009) is
   the highest-value addition — it's what makes procedural trees look like
   trees (one dominant leader, suppressed side branches) rather than
   symmetric bushes, and it's a citable, already-formalized algorithm, not
   an invention.
3. **Oscillator-based lateral root priming** instead of flat branch
   probability, for regularly-spaced root branching.
4. **Moisture/shade-driven moss spread** (function of light-field value +
   adjacent water/humidity), not a flat "damp stone" check.
5. Growth-rate ratios (~1:10:100 lichen:moss:tree) for tuning constants.
6. Systemic stress signaling: optional, revisit later if there's appetite.
