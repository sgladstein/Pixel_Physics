# Stigmergy research: indirect coordination through the environment

Raw findings kept in full here, matching the format of `research/m16-plant-biology.md`
and `research/m18-creature-biology.md`. This is the source material to build
colony behaviour against — read this, not just a condensed summary, when
implementing.

**Why this file exists — and what it is *not*.** Stigmergy is the
deposit → diffuse → decay → follow loop, which is the general primitive behind
most "complex behaviour from simple rules" systems: ant trails, Physarum and
mycelial networks, erosion carving channels, desire paths, scent-based
predation, and `plant.rs`'s existing auxin `channel`. Ants are simply the
instance with sixty years of quantitative literature and tuned parameters
written down, which is why this file leans on them.

**Read the numbers here as parameters for the general primitive, not as an ant
feature.** Anything built from this should be built once and called by many
systems — see `emergent-world-architecture.md` §0 for the general framing.
`m18-creature-biology.md` §4 and §5 touch the edges of this (Wa-Tor, Physarum,
Braitenberg); the mechanism itself was not covered.

**Relationship to the architecture decision.** Stigmergy is the biological
statement of the operational rule in `emergent-world-architecture.md` §1: agents
interact only through the world, never directly. That rule was arrived at from
architecture; Grassé arrived at it from termites in 1959. The convergence is
worth trusting.

---

## 1. What stigmergy is

Pierre-Paul Grassé coined the term in 1959 to resolve what Theraulaz and
Bonabeau later called the *coordination paradox*: how do insects of very limited
individual intelligence, with no apparent communication, collectively build
structures of enormous complexity?

<cite index="2-1">Grassé's answer was that work performed by an agent leaves a trace in the environment that stimulates further work — by the same agent or any other. This mediation through the environment means tasks get executed in the right order with no planning, no central control, and no direct interaction between agents.</cite> The term is from Greek *stigma* (mark, sign) and *ergon* (work): the stimulating product of labour.

<cite index="2-1">His observation was of termites repairing a nest. Individuals initially wander more or less at random, carrying mud and depositing it haphazardly. Those first deposits then stimulate other termites to add mud in the same place, and the small heaps grow into columns that eventually meet as arches.</cite>

The load-bearing consequence: **the structure regulates its own construction.**
The worker does not direct the work; the work directs the worker. No individual
holds a plan.

### Two kinds, and the distinction matters for implementation

- **Sematectonic (quantitative) stigmergy** — agents modify the physical
  environment, and the resulting *structure* is the stimulus. Termite pillars.
  In engine terms: reading and writing **cells**.
- **Marker-based (qualitative) stigmergy** — agents deposit transient signals
  (pheromones) that do not change the structure but carry information. Ant
  trails. In engine terms: reading and writing **field channels**.

These are different mechanisms with different costs, and a colony uses both.
The engine already has the substrate for each: the CA grid and the field grid.

### The population threshold

Grassé's own experiments found that groups below roughly 50 workers consistently
failed to complete a nest — not from insufficient labour, but because too few
deposits meant the density threshold for the stimulating configuration was never
reached. **Stigmergy has a minimum viable population.** A handful of test agents
will produce nothing and look broken. Plan the first ant scene with dozens, not
three.

**Sources:**
- [Stigmergy as a Universal Coordination Mechanism (Heylighen, Springer)](https://pespmc1.vub.ac.be/Papers/Stigmergy-Springer.pdf)
- [A Brief History of Stigmergy (Theraulaz & Bonabeau, Artificial Life)](https://www.researchgate.net/publication/12680033_A_Brief_History_of_Stigmergy)
- [Stigmergy overview (stigmergicsystems.com)](http://www.stigmergicsystems.com/stig_v1/stigrefs/article1.html)

---

## 2. Trail formation: the double-bridge experiment

The canonical demonstration, and the direct ancestor of Ant Colony Optimization.

<cite index="11-1">Deneubourg and colleagues connected an Argentine ant nest to a food source by two bridges. Ants deposit pheromone as they move, and at a choice point their decision is biased by pheromone concentration — the higher the concentration on a path, the more likely they are to take it.</cite>

Two results:

**Equal-length bridges** — <cite index="11-1">the colony converges on one bridge or the other, and across repeated trials each bridge wins about half the time.</cite> Symmetry breaking from random fluctuation, amplified by positive feedback. Nothing chooses; the choice emerges.

**Unequal-length bridges** (Goss et al. 1989) — <cite index="14-1">ants that happen to take the short bridge reach the food and return first, so the short bridge accumulates pheromone earlier, which biases subsequent ants toward it.</cite> **The colony finds the shorter path without any ant measuring anything.** Differential reinforcement rate does all the work.

### The choice function — this is what a worm's `min_by` should become

<cite index="19-1">In Deneubourg's model, an ant at a bifurcation chooses probabilistically as a function of the pheromone concentrations on the two branches, with a parameter *a* setting the degree of nonlinearity. A high *a* means even a slight concentration difference produces a near-deterministic choice.</cite>

For the engine: gradient-following should be a **probabilistic choice weighted
by a nonlinear function of channel concentration**, not `min_by`/`max_by`.
Deterministic selection kills the exploration that makes the whole mechanism
work. `creature.rs`'s current `min_by` over four neighbours is the deterministic
version and should be replaced when thermotaxis is fixed.

### A refinement worth knowing

<cite index="19-1">Perna et al. found that individual Argentine ants actually show a proportional (Weber's Law) response to pheromone, not the sigmoidal one the classical model assumes — but agent simulations with the Weber response still produced trails matching the literature, and the sigmoidal collective response can be derived analytically from the individual Weber response once directional noise around the ant's preferred heading is assumed.</cite>

**The noise is not a nuisance term — it is load-bearing.** A simple proportional
response plus movement noise reproduces the nonlinear collective behaviour. If
trails fail to form, adding noise is as likely to be the fix as tuning the
response curve.

**Sources:**
- [Ant colony optimization — Scholarpedia (Dorigo)](http://www.scholarpedia.org/article/Ant_colony_optimization)
- [Individual Rules for Trail Pattern Formation in Argentine Ants (Perna et al., PLoS Comput Biol)](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC3400603/)
- [Evolution of Ant Colony Optimization Algorithm — A Brief Literature Review (arXiv)](https://arxiv.org/pdf/1908.08007)

---

## 3. Evaporation, and a correction to the naive story

I previously told you "evaporation is the algorithm." That is *mostly* right and
worth refining, because the literature separates two things the phrase conflates.

### Evaporation is required for convergence

The ACO literature is unambiguous. <cite index="25-1">With ρ = 0 (no evaporation) the algorithm fails to converge at all; with excessive ρ it converges prematurely to sub-optimal solutions.</cite> <cite index="26-1">One systematic study found ρ = 0 gave poor results across the board, with general improvement appearing from ρ ≥ 0.2 — moderate-to-intense evaporation is needed to preserve the balance between exploration and deposition.</cite>

Concrete starting values from the TSP literature: <cite index="24-1">typical defaults are α = 1 (pheromone weight), β = 2–5 (heuristic weight), and ρ = 0.1–0.5 (evaporation rate), with larger β making ants greedier and converging faster at greater risk of premature convergence.</cite> One tuned ACS configuration reported α = 0.5, β = 5.0, ρ = 0.1, q₀ = 0.7.

`field.rs` already has `LIGHT_DECAY = 0.85` as machinery of exactly this shape
(a per-step multiplier, so ρ ≈ 0.15). That is inside the useful band and is a
reasonable first guess for pheromone. Expect to tune it — this will be the
parameter you spend the most time on.

### But flexibility comes from somewhere else

Here is the correction. **Evaporation drives path *selection*. It does not, by
itself, give a colony the flexibility to abandon a path once committed.**

<cite index="52-1">Models predict that ants stay on an established trail for longer than the pheromone's evaporation rate would suggest, because ants continue reinforcing the trail to the poor food source — colonies get "trapped" in suboptimal solutions. The ability to reallocate depends on the strength of the positive feedback, and a highly nonlinear system is *more* susceptible to being trapped, because two modes of exploitation can coexist; a weakly nonlinear system always finds the best source but commits to it less strongly.</cite>

That last clause is a direct design lever: **high nonlinearity buys sharp,
decisive trails at the cost of getting stuck. Low nonlinearity buys adaptability
at the cost of diffuse, less legible trails.** Pick knowingly.

And the resolution real ants use is not more evaporation:

<cite index="51-1">Grüter et al. showed both experimentally (in *Lasius niger*) and in simulation that negative feedback from crowding at feeding sites is what preserves flexibility. In a constant environment it prevents the usual symmetry-breaking bias toward one feeder; in a changing environment it lets a colony rapidly reallocate to a better patch discovered late. The model confirmed that switching to a superior source does not require pheromone decay at all.</cite>

**Implementation consequence: build a repulsion or crowding term, not just
attraction.** An ant should be less inclined to a cell that is already crowded,
or a food source already saturated. This is the single most likely fix for
trails that form and then ossify — more likely than turning up evaporation, and
the one a naive implementation will omit.

**Sources:**
- [Parameter Adaptation in Ant Colony Optimization (Stützle et al.)](https://lopez-ibanez.eu/doc/StuLopPelMau2010adaptiveACO.pdf)
- [Negative Feedback Enables Fast and Flexible Collective Decision-Making in Ants (Grüter et al., PLoS ONE)](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC3440389/)
- [Flexibility in collective decision-making by ant colonies (ScienceDirect)](https://www.sciencedirect.com/science/article/abs/pii/S0960077913000325)
- [Ant Colony Optimization for Density Functionals (arXiv, ρ sweep)](https://arxiv.org/pdf/2504.20317)

---

## 4. Construction — and a finding that lands directly on the moisture channel

**This is the most important section in this file for the current plan.**

> **STATUS 2026-09-02: the engine does not implement this section, and the
> reason is measurable rather than a tuning gap.** The literature below is
> unchanged and is still what the design is aimed at; what changed is a
> reading of what shipped. `creature::moisture_gradient` — the channel built
> to carry the evaporation-flux/curvature bias this section describes — was
> measured with `examples/field_sense_probe.rs` on a bed built to have
> curvature, and a convex crest reads **1.012x** a flat plateau at the same
> elevation (1.003x if the sampler is widened to ±24). It carries the
> vertical air/soil step, which every surface has, and is therefore a
> **depth** signal. Deposition does follow it, and pillars and galleries do
> appear — but not for the reason stated here, so *"deposition follows
> curvature"* is not a premise anything may be built on until a channel
> carrying surface shape exists. Full numbers on
> `creature::moisture_gradient`;
> `Reports/creature-genome-flexibility-2026-09-02.md` §5 carries the same
> status line.

The classical model of termite construction assumes a **cement pheromone**: a
chemical added to deposited material that stimulates further deposition nearby,
producing the positive feedback that grows pillars. Deneubourg's 1977
mathematical model, and the agent-based models that followed (Bonabeau et al.
1997; Khuong et al. 2011, 2016), all rest on it.

<cite index="31-1">Bonabeau et al. extended the pillar model with a convection air stream carrying pheromone, a net directional flux of individuals, a self-maintained trail, and a pheromonal template from the continuously-emitting queen — and showed pillars transform into walls, galleries or chambers under different conditions with no change whatever in the termites' behaviour.</cite> **Same rule, different environment, different architecture.** That is the emergent-world thesis stated as a result.

### The 2024 result: no cement pheromone needed

Facchini et al. (eLife 2024) ran *Coptotermes gestroi* on clay arenas seeded
with topographic cues and pellets **sterilised to remove any chemical marking**,
and tracked collection and deposition as separate events.

Findings, in order of usefulness to this engine:

1. <cite index="40-1">Pellet *collection* was distributed evenly across the arena, consistent with a mechanism unaffected by local topography and driven only by where termites happen to be. *Deposition* was concentrated at points of high surface curvature and at boundaries between substrate types.</cite> **Collection is uniform; deposition is targeted.** That asymmetry is the whole algorithm and is trivially cheap to implement.

2. <cite index="40-1">The single feature shared by every deposition region was that it was a local maximum in evaporation flux — and evaporation flux is provably proportional to local surface curvature.</cite>

3. <cite index="40-1">The authors conclude that surface curvature alone is sufficient to organise building activity, and that termites likely sense curvature *indirectly*, through substrate evaporation.</cite>

4. A salt-solution "chemical garden" control with no termites deposited salt in
   precisely the regions where termites had built — a physical visualisation of
   the evaporation field matching the construction pattern.

5. <cite index="40-1">Their model predicts, and their experiments show, that convex regions attract deposition while concave regions attract digging — which also reconciles an earlier study that had reported the opposite sign, because that study could not separate digging from building.</cite>

### Why this matters enormously here

**The moisture channel already planned for `emergent-world-architecture.md` §4
is, on this account, the construction channel.** No separate cement-pheromone
channel is needed for building. Deposition probability ∝ local moisture gradient
magnitude; digging probability ∝ the inverse.

Given the channel exists, an ant/termite becomes roughly:

```
if carrying:  deposit with probability ∝ |∇moisture|      (convex, drying → build)
else:         pick up with uniform probability wherever you are
              dig with probability ∝ -|∇moisture| or toward wetter (concave → excavate)
move:         gradient-following with noise
```

That is a handful of lines against an existing channel, and it produces pillars,
walls, and arches as consequences. Note also the self-sustaining loop the authors
propose: <cite index="40-1">where moisture is not externally replenished, the evaporation flux can be maintained by the humidity carried in recently-dropped pellets themselves.</cite> Wet deposited material re-seeds the field that attracts the next deposit — positive feedback, purely physical, no marker chemical.

**A caveat worth carrying.** The engine's existing plant systems will also read
this channel. Termite construction and root hydrotropism competing for the same
moisture field is not a bug — it is exactly the kind of unplanned interaction the
architecture is built for — but it means tuning one affects the other.

**Sources:**
- [Substrate evaporation drives collective construction in termites (Facchini et al., eLife 2024)](https://elifesciences.org/articles/86843)
- [A model for the emergence of pillars, walls and royal chambers in termite nests (Bonabeau et al., Phil Trans R Soc B)](https://royalsocietypublishing.org/doi/10.1098/rstb.1998.0310)
- [The role of logistic constraints in termite construction of chambers and tunnels (Ladley & Bullock)](https://www.sciencedirect.com/science/article/abs/pii/S0022519304006113)
- [Modelling the Role of Trail Pheromone in Collective Construction of Termite Royal Chambers (Hill & Bullock, ECAL 2015)](https://eprints.soton.ac.uk/376660/1/Hill_20and_20Bullock_20ECAL_202015.pdf)

---

## 5. Excavation — ant nests are dug, not built, and density does the shaping

For ants specifically (as opposed to mound-building termites), "building a nest"
mostly means **digging**. This has its own well-characterised mechanism, and it
is remarkably cheap.

Toffin et al. (PNAS 2009) ran 2D nest-digging experiments in deliberately
homogeneous conditions — no environmental heterogeneity to bias anything.

<cite index="60-1">A morphological transition occurs during excavation: the initial circular cavity evolves into a ramified, branching structure. The transition happens regardless of the number of ants, but more often with more workers, and a stochastic model shows density effects are central to it.</cite>

The mechanism, as summarised by later work:

<cite index="65-1">A large number of digging ants relative to nest area produces uniform digging, because ant density along the initially small nest perimeter is high. As the nest grows, average density falls to a critical value, at which point localized excavated buds appear through amplification. High worker density promotes uniform digging; low worker density promotes localized digging and new tunnels.</cite>

**This needs no new channel at all.** Ant density along a perimeter is already
implicit in where the agents are. A round chamber that spontaneously sprouts
tunnels once it outgrows its colony is emergent nest architecture from one
density-dependent rule — and it produces a *recognisable ant nest*, which is
precisely the stated goal.

**Sources:**
- [Shape transition during nest digging in ants (Toffin et al., PNAS 2009)](https://www.pnas.org/doi/10.1073/pnas.0902685106)
- [The Role of Colony Size on Tunnel Branching Morphogenesis in Ant Nests (PLoS ONE)](https://journals.plos.org/plosone/article?id=10.1371%2Fjournal.pone.0109436)

---

## 6. The Physarum agent model — closest algorithmic prior art, with real numbers

Jones' multi-agent Physarum model is the most directly transferable
implementation in this literature, because it is already a grid-plus-agents
design of exactly the shape this engine uses. `m18-creature-biology.md` §5 cites
it; here are the actual parameters.

<cite index="43-1">The model has an agent layer and a continuum trail-map layer that affect each other: agents deposit onto the trail map, and sense the trail map to determine movement. Each agent has a position, a heading, and three sensors — front-left, front, front-right.</cite>

The loop, in full:

<cite index="48-1">**Motor stage:** attempt to move forward in the current heading. If the move succeeds, deposit trail at the new location. If it fails, choose a new random orientation. **Sensory stage:** sample the trail map at the three sensors. If front is strongest, keep heading. If front is weakest, rotate randomly left or right by RA. Otherwise rotate toward whichever side sensor is stronger, by RA.</cite>

Then diffuse (a 3×3 mean filter) and decay the trail map.

### Parameters that actually matter

| Parameter | Typical value | Effect |
|---|---|---|
| SA — sensor angle | 22.5°–45° | angular spread of the three sensors |
| RA — rotation angle | 45° | how far an agent turns per decision |
| SO — sensor offset | 5–9 cells | how far ahead it senses |
| deposit | 5 units per successful move | trail strength |
| step | 1 cell | movement per tick |

Two findings are worth more than the numbers:

**The SA/RA relationship controls network topology.** <cite index="42-1">There is significant interplay between sensor angle and rotation angle. When they are equal, the emerging network contracts. If RA < SA, contraction increases. If RA > SA, spontaneous branching appears during network formation — because the larger rotation places the agent's sensors outside the trail it was following.</cite> **One inequality is the difference between a minimal network and a richly branching one.** That is a direct tuning knob for how ant trails look.

**Deposit only on successful movement.** <cite index="47-1">Particles deposit into the lattice only when a forward move actually succeeds; if the target site is occupied, the default behaviour is to abandon the move and pick a new random direction.</cite> Blocked agents do not reinforce. This is what prevents trails from building up in congested dead ends, and it is easy to omit by accident.

### And a hard constraint on field resolution

<cite index="48-1">A minimum sensor offset of 3 cells is required for strong local coupling and for complex patterns to emerge at all. Increasing SO gives thicker networks, faster adaptation, and coarser-grained results.</cite>

**This is direct quantitative evidence for the resolution problem in
`emergent-world-architecture.md` §6.** The mechanism needs to resolve
differences at a 3-to-9-cell sensor offset. At `FIELD_SCALE = 8`, a 3-cell
offset falls entirely inside one field cell and all three sensors return the
same number. Bilinear interpolation (§6a) recovers a usable gradient at that
offset; it does not recover trail *width*, since a one-cell trail smeared across
an 8-cell block stays smeared.

Conclusion: bilinear sampling is necessary and may be sufficient at SO ≈ 8.
Below that, pheromone needs its own finer grid. **Test this cheaply before
redesigning `FieldTile`** — seed a synthetic trail at `FIELD_SCALE = 8`, run a
gradient-follower with SO = 8, and see whether it tracks.

**Sources:**
- [Characteristics of Pattern Formation and Evolution in Approximations of Physarum Transport Networks (Jones, Artificial Life 2010)](https://royalsocietypublishing.org/doi/10.1098/rstb.1998.0310)
- [Programmable reconfiguration of Physarum machines (Jones, arXiv)](https://arxiv.org/pdf/0901.4556)
- [Routing Physarum with electrical flow/current (arXiv)](https://arxiv.org/pdf/1204.1752)
- [Formation and Optimisation of Vein Networks in Physarum (arXiv)](https://arxiv.org/pdf/2305.12244)
- [Mechanisms Inducing Parallel Computation in a Model of Physarum Transport Networks (Jones, arXiv)](https://arxiv.org/abs/1511.05869)
- [physarum — Sage Jenson (implementation notes)](https://cargocollective.com/sagejenson/physarum)

---

## 7. Summary: what this means for the build

1. **Stigmergy is two mechanisms, and this engine has substrate for both.**
   Marker-based (pheromone) → field channels. Sematectonic (structure as its own
   stimulus) → cells. A colony uses both; don't build only the first.

2. **Movement is probabilistic gradient-following with noise, never `min_by`.**
   Deterministic selection kills exploration and the mechanism collapses.
   Deneubourg's nonlinear choice function is the reference; a Weber-style
   proportional response plus directional noise provably reproduces it.

3. **Evaporation drives path selection; crowding drives flexibility.** ρ = 0
   never converges, ρ ≥ 0.2 works, typical range 0.1–0.5. `LIGHT_DECAY = 0.85`
   is already in-band. But **also build a negative-feedback/crowding term** —
   without it the colony ossifies on the first path found, and more evaporation
   will not fix it.

4. **The moisture channel is the construction channel.** Deposition ∝ evaporation
   flux ∝ surface curvature, with no cement pheromone required. Collection is
   uniform, deposition is targeted, and convex attracts building while concave
   attracts digging. This is a handful of lines on a channel already scheduled
   to be built.

5. **Nest shape comes from density, free.** High worker density per unit
   perimeter → uniform circular chamber; falling density → localized budding →
   branching tunnels. No new channel, no new state.

6. **Sensor offset ≥ 3 cells is a hard requirement**, which makes the §6
   resolution work a prerequisite rather than a nicety. Test at
   `FIELD_SCALE = 8` with bilinear sampling before considering a finer grid.

7. **Deposit only on successful movement.** Blocked agents must not reinforce,
   or congested dead ends accumulate trail.

8. **Population has a floor.** Below roughly 50 workers, real termite colonies
   fail to reach the density threshold and build nothing. A three-ant test scene
   will look broken when the code is correct. This also makes reseeding /
   reproduction (`emergent-world-architecture.md` §5e) a prerequisite for
   colonies rather than a nice-to-have.

### Where else this primitive applies

Every mechanism in this file is worth reading with a substitution in mind. The
literature says "ant" and "pheromone"; the engine should hear "agent" and
"channel":

| Mechanism here | Also describes |
|---|---|
| Trail reinforcement + evaporation (§2, §3) | Erosion channels, desire paths, Physarum tubes, `plant.rs`'s auxin `channel` |
| Nonlinear choice function (§2) | Any gradient-follower — roots, worms, future predators |
| Negative feedback / crowding (§3) | Resource competition of every kind, including roots depleting shared water |
| Deposition ∝ evaporation flux (§4) | Any deposition process keyed to a field gradient — sediment, frost, salt |
| Density-driven shape transition (§5) | Any process where local crowding flips uniform behaviour into localised budding |
| Three-sensor gradient sampling (§6) | The read half of every taxis in the engine |

If an implementation of any of these can only be called by ants, it was built at
the wrong level.

### The verification scene this implies

Per the protocol in `emergent-world-architecture.md` §10 — scene first, then the
weakest assertion that would fail if the mechanism were deleted:

**Scene:** a nest cell, two food sources at different distances, 50+ agents.
**Watch for:** a trail forming, then thinning onto the shorter path.
**Assertion:** after N frames, mean pheromone on the short path exceeds the long
path. That single comparison would fail if deposition, evaporation, or
gradient-following were broken, and it is the double-bridge experiment reduced
to one line of test code.

### Staging this in a side-view world — the orientation caveat

**Every diagram in this literature is drawn top-down, and this engine is a
side-view cross-section** (see `worldgen-design.md` §0). That mismatch has two
consequences worth knowing before building any of the above.

**The double bridge needs two competing routes, and a line only has one.** In a
plane, two paths between nest and food are trivially arranged. Along a
horizontal strip there is one path unless *terrain* provides the alternatives —
over a ridge versus around it, through a cave versus over the hill, up a cliff
face versus along the base. The scene above must be staged with vertical
geometry, not by placing two food sources at different distances on flat ground.
That is a real constraint on the test, not a cosmetic one.

**The ant vision is split across orientations, and side view gets the better
half.** Foraging is natively top-down — trails spreading across a floor. Nest
architecture is natively side view — every real nest cross-section, and every
one of Toffin's digging experiments (§5), is a vertical section. So this engine
gets chambers, tunnels, real depth, the density-driven branching transition, and
roots and water intersecting the nest; it gets a simplified foraging problem
with fewer competing paths. Expect trail formation to look less impressive than
the published figures, and do not read that as a bug.

**Population density may not transfer.** Grassé's ~50-worker threshold (§1) was
measured in an arena with area to spread across. A strip has less area per unit
of travel, so the density at which the stimulating configuration forms may
differ — in which direction is unknown. Treat 50 as an order-of-magnitude
starting point rather than a number to trust.

---

## 8. Open questions, deliberately not resolved here

- **How many pheromone channels?** Real colonies use several (trail, alarm,
  brood, nest-site). Each is another O(world) pass. Start with one and add only
  when a second consumer exists — the standing review question in
  `emergent-world-architecture.md` §12 applies.
- **Do ants need heading state?** The Physarum model gives each agent a heading
  and three forward sensors. The engine's `CreatureState` currently has no
  orientation. Three-sensor sensing is meaningfully better than four-neighbour
  sampling for trail *following*, but it is per-agent state, which cuts against
  thin agents. Worth deciding deliberately.
- **Is curvature computable cheaply from the moisture field?** The eLife result
  gives evaporation flux ∝ curvature, so `|∇moisture|` should stand in — but that
  equivalence was derived for a physically evaporating substrate, not a diffusing
  scalar on a coarse grid. Verify empirically before relying on it.
