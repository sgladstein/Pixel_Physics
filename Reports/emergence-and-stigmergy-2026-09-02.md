# Emergence and stigmergy

**Audience:** the coding agent working on Pixel Physics.
**Status:** research and synthesis. Nothing in `src/` changes as a result of
this report; it exists to answer a question `emergent-world-architecture.md`
and `stigmergy-research.md` both raise and neither settles: *stigmergy is one
worked instance of the general emergence primitive — so what is the general
category, what else in this engine belongs to it, and what does the specific
instance require that the general one doesn't?*

**Read this after, not instead of, the other two.** `stigmergy-research.md`
holds the citations and tuned parameters for the ant colony; read it, not a
summary of it, before touching `pheromone.rs` or `creature.rs`'s movement
code. `emergent-world-architecture.md` §0 and §7 hold the architectural
decision (agents interact only through the world) and the first framing of
stigmergy as "one worked instance of the primitive, not the destination."
This report widens that framing: it surveys what else in the shipped engine
is emergent by the same definition, sorts out which of those are stigmergic
and which are a different named phenomenon wearing the same
deposit-diffuse-decay-follow clothes, and records what building the
stigmergic case specifically taught the rest of the emergence programme.
§8 then turns the question forward: if the goal widens from "can this engine
exhibit stigmergy" to "can this engine exhibit emergence generally," what
features, design principles and research are still missing.

---

## 1. What "emergence" means here, and what it doesn't claim

The owner's own statement of the goal, quoted in
`emergent-world-architecture.md` §0: *"a world that feels alive on its own
with unplanned emergent behavior that I can explore and interact with...
complex behavior from simple rules."* That is a description of **weak
emergence** in the sense the complexity-science literature uses the term,
not the stronger, more contested claim philosophers argue about — and the
distinction is worth being precise about, because the two have different
engineering consequences.

**Weak emergence**: a macro-level pattern or behaviour is a genuine
consequence of the micro-level rules and their interactions, discoverable
only by letting the system run (simulation, or the equivalent), not derivable
in closed form or predictable by inspecting a single rule in isolation. It is
not mysterious and it is not irreducible — it is exactly as reducible as the
code that produces it — but it is not *authored* either; nobody writes down
the flock's shape, only the three rules each bird follows
([Wikipedia, *Emergence*](https://en.wikipedia.org/wiki/Emergence)).

**Strong emergence**, by contrast, is the philosophically loaded claim that
some macro-level phenomena exert causal power that cannot in principle be
reduced to or predicted from the micro level, however completely it is
simulated ([Chalmers, *Strong and Weak
Emergence*](https://consc.net/papers/emergence.pdf)). Mark Bedau's line on
it is worth keeping in mind whenever a design conversation starts reaching
for "the world just decided to do this": strong emergence is "logically
possible" but "uncomfortable, like magic." Nothing this engine does needs
that claim, and nothing in it should be defended by it — every emergent
behaviour here is, by construction, a consequence of code that can be read,
and the fact that nobody *planned* the outcome is a statement about the
design process, not about metaphysics.

This lines up exactly with a rule already settled in `design-philosophy.md`
§2b, arrived at independently for a different reason (avoiding hardcoded
outcomes) and worth naming as the same thing: *"you may not hardcode 'a
trunk widens at height 10' or any other authored shape... the test for any
new rule: is the resulting shape a side effect of a mechanism, or is it
curve-fit to look a particular way?"* That is the operational definition of
weak emergence this engine actually enforces, stated in a design review
rather than a complexity-theory paper, and it predates this report by
several months. Nobody needed the vocabulary to reach the rule; the rule came
from watching a curve-fit trunk width look wrong on screen.

**The generative primitive** — `emergent-world-architecture.md` §0's own
framing, worth restating because everything below classifies mechanisms
against it — is a four-step loop: *deposit → diffuse → decay → follow.*
Something writes a scalar into shared state, the scalar spreads, the scalar
fades, something reads the gradient and acts, which writes more. Ants,
`plant.rs`'s auxin channel, erosion, desire paths, and predator scent trails
are all named there as the same loop wearing different vocabulary. That
claim is correct at the level of *code shape*. It is not correct at the
level of *which named phenomenon in the literature the resulting behaviour
belongs to* — and that gap is most of what this report is about.

## 2. Stigmergy, briefly — the instance the project actually researched

Full treatment, citations, and tuned parameters are in
`stigmergy-research.md`; this is the two paragraphs needed to place it
against the wider category.

Pierre-Paul Grassé coined *stigmergy* in 1959 watching termites repair a
nest: an agent's work leaves a trace in the environment, the trace stimulates
further work — by the same agent or any other — and the structure ends up
directing its own construction with no individual holding a plan
(`stigmergy-research.md` §1). Bonabeau, Dorigo and Theraulaz later placed it
alongside *self-organization* as one of the two central concepts underpinning
swarm intelligence generally
([Springer, *The biological principles of swarm
intelligence*](https://link.springer.com/article/10.1007/s11721-007-0004-y);
[ScienceDirect, *Stigmergy*](https://www.sciencedirect.com/topics/engineering/stigmergy)
frames it the same way) — a mechanism, not a single algorithm, that mediates
coordination between agents through environment modification rather than
direct signalling.

What makes stigmergy a *named, specific* thing rather than just "emergence"
is three properties the general primitive doesn't require on its own:

1. **Discrete, countable acting agents.** The environment field is not
   self-interacting; something with a position and a decision walks through
   it, deposits into it, and reads it back.
2. **Coordination is the phenomenon being explained.** The question stigmergy
   answers is specifically "how do many independent, unplanned individuals
   divide labour or converge on a shared outcome with no communication
   channel between them" — not just "how does a pattern form."
3. **The loop runs through an agent's *choice*.** Deposit and decay can run
   on their own; *follow* is a decision an agent makes based on what it
   senses, which is why Deneubourg's probabilistic choice function and its
   exploration noise (`stigmergy-research.md` §2, and `creature.rs`'s
   `choose_weighted`) are load-bearing for stigmergy specifically in a way
   they are not for, say, a diffusing temperature field.

Every mechanism in this engine with agents choosing where to go based on a
channel they and others like them wrote is a stigmergic system. Not every
emergent mechanism in this engine has that shape — which is the survey in
§4.

## 3. The relationship, stated precisely

**Stigmergy is a subset of emergence, not a synonym for it.** Every
stigmergic system here is emergent (the trail shape, the shortest-path
selection, the nest's branching are none of them authored). The converse is
false: reaction-diffusion pattern formation, self-organized criticality,
forced oscillation, and at least one purely algorithmic artifact this engine
has already shipped are all emergent — none of them are stigmergy, because
none of them route through a population of discrete agents making a choice.

One case worth naming because it collapses the distinction from an
unexpected direction: **`plant.rs`'s auxin `channel` is stigmergy that
predates the word being applied to it**, and `emergent-world-architecture.md`
§0 says so directly — "reinforced on successful growth, decayed at dead ends
... a stigmergic system operating inside a single organism. It was built
without the word for it." The "agents" there are a single tree's own growing
tips; the "environment" they coordinate through is a private per-tree
structure (`TreeState.attractors`, not a shared world channel — flagged as a
violation of the shared-channel rule in `emergent-world-architecture.md`
§5a). That is the same coordination logic Grassé described in termites,
operating one level down in scale, inside one organism rather than between
many. It is good evidence the primitive generalizes the way the architecture
report claims — and it is also evidence that "stigmergic" is a property of
the *coordination shape*, independent of whether the agents are separate
creatures, tips of one plant, or (per the `emergent-world-architecture.md`
§0 table) grains of sediment in an erosion channel.

**The table in §0 of the architecture report is correct about code shape and
silent about taxonomy**, and that silence is worth filling in, because two
different fields of research independently discovered the same four-step
loop for different reasons and gave the phenomena that arise from it
different names depending on what was doing the depositing:

- **Ants, termites, slime moulds** (biology / swarm intelligence): the field
  that named it *stigmergy*, because the depositing party is an animal
  making decisions.
- **Vegetation banding, coral patterns, animal coat markings** (mathematical
  biology): the same activator/inhibitor shape, discovered by Turing in 1952
  and independently rediscovered for semi-arid vegetation — where the
  literature calls it **reaction-diffusion pattern formation** or a **Turing
  pattern**, not stigmergy, because there is no discrete choosing agent: a
  plant does not decide to inhibit its neighbour, it just is one, and the
  inhibition is a passive consequence of resource depletion
  ([Springer, *An Analysis of Vegetation Stripe Formation in Semi-Arid
  Landscapes*](https://link.springer.com/article/10.1007/s00285-005-0319-5)).
- **Sandpiles, earthquakes, forest fires** (statistical physics): the same
  shape again — local threshold, cascading redistribution, no external
  tuning to a critical point — named **self-organized criticality** by Bak,
  Tang and Wiesenfeld, and the literature's own canonical *second* example
  alongside the sandpile lands closest to home here: the **forest-fire
  model** ([Drossel & Schwabl, *Self-organized critical forest-fire
  model*](https://link.aps.org/doi/10.1103/PhysRevLett.69.1629);
  [Wikipedia, *Forest-fire
  model*](https://en.wikipedia.org/wiki/Forest-fire_model)).

`emergent-world-architecture.md` calls fire's unburnt→burning→ash loop an
**excitable medium** (§0) and separately notes the matter-graph was a DAG
with no path back from ash (§5f, since closed — see §4 below). Read against
the literature above, this engine's fire is close kin to the
Drossel-Schwabl forest-fire automaton — tree/empty/burning states on a grid,
a burn that propagates to flammable neighbours and dies out behind itself —
with one structural difference worth being honest about: Drossel-Schwabl's
criticality requires three well-separated timescales (fire spreads
effectively instantly; regrowth is much slower than that but much faster
than random ignition), and it is exactly that separation that produces the
model's headline result, a scale-free (power-law) distribution of fire
sizes. Nobody has tuned this engine's growth rate against its fire-spread
rate with that separation in mind, and nobody has measured whether burn-size
distributions here are scale-free, merely graded, or something else
entirely. CLAUDE.md's own ethos law (an outcome is a distribution, not
binary) asks for graded, which is a much weaker and already-satisfied claim.
**Do not read this paragraph as evidence the engine has self-organized
criticality; it is a claim that the nearest named academic model of what
this engine's fire loop already resembles is one bearing that name, and that
the model's own literature says what would have to be checked before
claiming the stronger result.**

## 4. A survey: what's shipped, and which category it belongs to

| Mechanism | File | Emergent | Stigmergic | If not stigmergy, what |
|---|---|:-:|:-:|---|
| Pheromone trail formation, shortest-path selection | `pheromone.rs`, `creature.rs::choose_weighted` | yes | **yes — textbook** | — |
| Nest tamping / tunnel lining, spoil placement | `creature.rs` (`packs_into`, pellet-carry) | yes | **yes — sematectonic** | — |
| Density-driven nest-shape transition (round chamber → branching tunnels) | proposed, `stigmergy-research.md` §5 | claimed | claimed, **unverified** | see §7 |
| Moisture-curvature construction (deposition ∝ evaporation flux) | `creature::moisture_gradient` | intended | **refuted as built** — samples a depth signal, not curvature (`creature-genome-flexibility-2026-09-02.md` §5) | see §7 |
| `plant.rs` auxin `channel` (space colonization) | `plant.rs` | yes | **yes — pre-dates the term**, scoped to one organism | — |
| Root water uptake ↔ moisture-gradient steering | `world.rs::deplete_moisture`, `plant.rs` | yes | **yes**, agents are competing root systems | — |
| Light occlusion by canopy / plant shading | `field.rs::rebuild_blocked` | yes | borderline — passive geometry, no agent "chooses" to shade | closer to reaction-diffusion facilitation |
| Tiger-bush vegetation banding (light + moisture as activator/inhibitor pair) | ingredients shipped, pattern unverified | claimed | **no**, if it forms | reaction-diffusion / Turing pattern |
| Fire → heat → ignition → more fire | `fire.rs` | yes | **no** — no discrete agent choosing where to spread | excitable medium / forest-fire automaton |
| Day/night oscillator driving light, growth, evaporation | `sky.rs`, `field.rs::apply_sky` | yes | no | forced/entrained global oscillation |
| Ash → soil → reseeding matter cycle | `decay.rs` | yes | no | closed-loop material cycle |
| Structural "count-to-infinity" runaway on an unsupported island | `structural.rs` | yes, and explicitly unplanned | **no** — no agents, no deposit, one global relaxation | distributed-algorithm artifact (distance-vector routing analogue) |
| Fracture-size distribution (graded breakage, not binary) | `rigid.rs`, `load.rs` | yes | no | avalanche/cascade statistics (self-organized-criticality-adjacent) |

Three rows deserve more than the table cell.

**Light occlusion is the interesting boundary case.** A plant occluding
light for the plant beneath it satisfies "environment modification that
changes another agent's local conditions," which is stigmergy's letter. But
nothing *decides* to cast a shadow — it is a fixed geometric consequence of
standing where you already grew, with no analogue of an ant's choice to
deposit pheromone here rather than there. The ecology literature's own
framing (activator autocatalysis at short range, inhibition at long range,
both passive) is the better fit, and it is worth keeping the line there: an
agent's active choice to deposit is what stigmergy researchers actually
study and tune (§2's evaporation/crowding literature is about *choices*, not
geometry), and blurring it away loses the reason the extra vocabulary
exists.

**The structural "count-to-infinity" runaway is the cleanest example in this
engine of emergence with no stigmergy in it at all**, and it is worth
keeping on hand as the counter-case whenever "emergent" and "stigmergic"
start being used interchangeably in conversation. README (~line 1949)
records it as "one genuinely interesting emergent property, not deliberately
designed in": a `Solid` structure with no path to any anchor relaxes its
distance value upward without bound once disturbed, because cells with only
each other to reference have no true zero to converge toward — the
identical failure mode known from distance-vector network routing, arrived
at independently by a support-distance relaxation algorithm that has nothing
to do with agents, pheromone, or biology. It is unplanned, it is a genuine
consequence of the algorithm rather than an authored outcome, and it
satisfies the weak-emergence definition in §1 completely. It has no deposit,
no decay, no agent choosing to follow a gradient — just a fixed-point
iteration without a well-defined fixed point. Filing it as "stigmergy" would
be wrong; filing it as "not emergent because it's just a bug" would be the
opposite mistake — CLAUDE.md's own instinct to *keep* interesting unplanned
behaviour rather than defensively assume it's an error is the right one
here, since it reproduces the target outcome (an unsupported structure
fails) for a genuine structural reason.

**The fracture-size distribution is the ethos law wearing statistical-physics
clothes — and this engine has already run the actual experiment on the
sibling system, with a result worth carrying over rather than re-guessing.**
CLAUDE.md's first law — *"an outcome is a distribution, not a binary... real
breakage is a few blocks, more cobbles, a lot of grit"* — was arrived at from
playtesting, not from reading Bak, and a graded size distribution from local
threshold failure cascading through a chunked medium is superficially the
shape self-organized criticality studies. It is tempting to reach for
Bak-Tang-Wiesenfeld's sandpile as the model to aim fracture cascades at. **Do
not — `granular-mechanics-research.md` (Report A of four) already asked this
exact question for the closest analogous system in this engine, granular
avalanches, and the literature answer was no.** BTW toppling was in `PLAN.md`
as pending work for sand avalanches and that report recommends deleting it,
because the experimental record on real sandpiles rejects scale-free
avalanches: rotating-drum work finds "a sharply peaked avalanche size
distribution, not a power law," and even the closest confirmed case (the
Oslo rice-pile experiment) only produces power-law avalanches for
sufficiently elongated grains, giving a stretched-exponential distribution
otherwise. The mechanisms named for the mismatch — dilatancy and
velocity-weakening friction — are physical properties of real grains, not
artifacts of measurement, and that report's own two-angle model (θ_ms/θ_r
hysteresis) is offered as "a more accurate [model], not a cheaper substitute"
for the graded-but-not-scale-free avalanches real sand actually produces,
with its own acceptance test (`§8c`, "avalanche size distribution is peaked,
not power-law") already specified.

The transferable lesson is not "fracture won't be critical either" — nobody
has run the equivalent experiment on rock — it is **the instrument and the
scepticism**: `granular-mechanics-research.md` §8c's peaked-vs-power-law
histogram check is exactly what fracture-size measurement needs before
anyone claims a distributional shape for it, and CLAUDE.md's own repeated
warning about tidy-looking results applies doubly to reaching for a famous
model (SOC) because the qualitative picture (graded, cascading, threshold-
driven) matches, when the one part of that model's story that's actually
checkable — the exponent — is usually the part that turns out wrong for
real driven-dissipative granular and fracture systems alike. If fracture-size
measurement is ever built, borrow the instrument from §8c before reaching for
new theory.

## 5. What stigmergy specifically requires, that the general loop doesn't

This is the part worth carrying into any future creature or plant work: the
extra structure §2 named is not free, and three of its costs already forced
real engine changes that a non-stigmergic emergent channel never would have.

**A minimum population, not just a minimum field resolution.** Grassé's own
termite experiments found groups below roughly fifty workers never reach the
density of deposits needed to trigger the stimulating configuration at all —
`stigmergy-research.md` §1 calls this "stigmergy has a minimum viable
population," and §8 flags that the number itself may not transfer to a
side-view strip. Nothing in this requirement applies to reaction-diffusion
patterning or to fire spread: a two-channel activator/inhibitor field can
show a pattern from a single perturbed cell given enough diffusion steps,
and fire needs exactly one ignition point. Stigmergy's threshold is about
*discrete choosing agents* reinforcing faster than the channel decays, which
only exists once there are enough of them; it is a fact about population
dynamics layered on top of the field mechanics, not a property of the field
itself.

**Evaporation is necessary but not sufficient — flexibility needs a second,
different term.** `stigmergy-research.md` §3, and `dead-ends.md`'s P-12
entry on `BrainInput::Crowding`, record a finding with no analogue elsewhere
in this engine's emergent mechanisms: a stigmergic system can get *trapped*
exploiting a known-inferior path, because the same positive feedback that
finds the short bridge in Deneubourg's double-bridge experiment also locks
onto whichever path happened to be found first, and **more evaporation does
not fix this** — a separate negative-feedback (crowding) term is required.
Fire has no equivalent failure mode (a fire that commits to spreading one
direction isn't "wrong," it's just physics), and neither does the day/night
oscillator or ash→soil decay. Ossification-by-positive-feedback is
specifically a hazard of systems where *choice* under recruitment is the
mechanism, which narrows it to the stigmergic and reaction-diffusion-with-
recruitment families, not to emergent systems generally.

**Resolution: the general fix (bilinear sampling) was not the specific fix
(a dedicated plane).** `emergent-world-architecture.md` §6 diagnosed a
single underlying defect — block-nearest field reads defeat any
gradient-follower whose sensors sit inside one `FIELD_SCALE=8` block — and
it first showed up outside stigmergy entirely, in the worm's four-neighbour
thermotaxis (`dead-ends.md`'s `field_at_bilinear` entry). Bilinear
interpolation fixed that case completely. It did **not** fix pheromone:
`emergent-world-architecture.md` §6b predicted exactly this in advance
("interpolating a smear yields a smooth smear"), and `pheromone.rs`'s own
module doc records the experiment that confirmed it — a Jones follower
reading a would-be sixth `FieldCell` channel at `FIELD_SCALE=8` could find a
smeared trail but not travel it (0.052 along-trail progress against a
random walk's 0.262), and two trails four cells apart produced a
bit-identical field, 0.0000 difference across the seventeen rows spanning
them, at any sensor offset. No amount of reading between block centres
recovers information the coarse write destroyed. That is why pheromone lives
in its own CA-resolution double-buffered plane (`pheromone.rs`) rather than
as a sixth `FieldCell` channel, at a real cost (~84 MB standing allocation
at the shipped world size, `pheromone.rs`'s own sizing comment) that no
other channel in the engine pays. **The general resolution problem and the
specific stigmergy requirement diagnosed each other**: the worm case proved
the mismatch existed and that bilinear sampling was the right general
answer; the pheromone case proved bilinear sampling has a ceiling, and that
trail-width information specifically needs finer storage, not just smarter
reads. Neither finding would have surfaced from the other mechanism alone.

## 6. What the specific case taught the general programme

Four lessons that were found by building the stigmergic instance and are now
standing checks for *any* future emergent channel, stigmergic or not — worth
stating explicitly as payback for the extra research effort §2's literature
review cost:

- **A channel needs both a writer and a reader before it means anything, and
  neither compiling nor testing green proves both exist.** First found on
  the light field (dead for a whole milestone, `emergent-world-architecture.md`
  §2), it recurred inside the stigmergy work itself when the lateral
  pheromone sensor pair (`PheroALateral`/`PheroBLateral`) was wired in but
  measured at exactly 0.000 for a side-view surface-walker (`dead-ends.md`,
  `BrainInput::PheroAAlong` entry) — a reader with nothing useful to read
  from its own geometry, not from a missing writer. Same failure family,
  different cause; both are now covered by
  `emergent-world-architecture.md` §12's standing review question 2.
- **"Did it fire at all" needs a counter, not a picture — and the pheromone
  plane's sleep mechanism is the shipped instance of the rule.**
  `PheromoneStats::tiles_processed` and the
  `a_settled_plane_processes_zero_tiles` test are the direct application of
  `CLAUDE.md`'s Method-section rule to this specific channel: a settled
  plane must report zero tiles processed, not merely look quiet in a
  render. This is the same tile-sleep law field-sleeping established
  generally (`emergent-world-architecture.md` §9) — stigmergy didn't invent
  the pattern, it inherited it and gave it its own counter.
- **Quantized decay must be forced to strictly decrease, or it fixes a point
  above zero.** `pheromone.rs::build_decay_lut`'s doc records the
  discovery — a plain multiplicative decay on a `u8` can map a small value
  to itself forever — and names the sibling bug it generalizes from: the
  4-bit canopy-density scalar hit the *identical* fixed point independently
  (0.800 → 0.533 → 0.267 → 0.267, a permanent floor). `dead-ends.md` records
  this as one rule now, holding for "any decay on quantized storage," found
  because the two systems were built close enough in time for the pattern
  to be visible.
- **Deterministic selection (`min_by`/`argmax`) kills the mechanism it's
  installed in, silently.** Named first from the ACO literature
  (`stigmergy-research.md` §2, Deneubourg's nonlinear choice function), it
  was then found as a live bug in the worm's thermotaxis (`min_by` always
  fleeing west on a tie) and is now `creature.rs::choose_weighted`'s own
  standing law — "never replace the squared-weight sample with an argmax" —
  applied to every gradient-following decision in the creature code, not
  only pheromone-following. The research pass built for one mechanism
  became the reason a second, unrelated mechanism's bug was recognized on
  sight instead of re-discovered from scratch.

## 7. Open ground: claimed, wired, and actually observed are three different states

In the order they matter most to whoever picks this up next:

**Tiger-bush banding is now mechanically possible and has not been looked
at.** `emergent-world-architecture.md` §0 named this as the highest-value
"nearly free" generator months ago — local facilitation (a plant retains
moisture nearby) and long-range inhibition (a plant depletes moisture and
casts shade over a wider radius) are the two ingredients, and both have
since shipped: `world.rs::deplete_moisture` lets one root's drinking show up
in a neighbour's gradient read (README's "Root water uptake" section), and
canopy shading via `field::rebuild_blocked` has been live since the light
writer landed. **Nobody has rendered the scene.** This is exactly the
situation `CLAUDE.md`'s Method section warns about under "check that a
planned step can demonstrate itself" — the prerequisite writes are real, the
literature's mechanism is well-established for real semi-arid vegetation,
and the actual banding pattern in this engine is an unverified prediction,
not a measured result. A cheap next step: a large, uniformly-seeded strip of
comparably-spaced plants over a long run, watched (not metriced first) for
banding, per `emergent-world-architecture.md` §10's own verification
protocol.

**The density-driven nest-shape transition is asserted from the literature,
not from a rendered nest.** `stigmergy-research.md` §5 (Toffin et al., PNAS
2009) is confident this needs no new channel — ant density along a tunnel
perimeter is already implicit in agent positions — and `wiki/ants.md`
(current as of 2026-09-02) confirms tunnels now stay dug, are tamped, and
produce "narrow galleries... branching back from the face." It does **not**
confirm the specific claim the research file makes: that the transition
from a uniform circular chamber to localized branching tunnels is
*density-dependent* in the way Toffin's model predicts, as opposed to simply
"ants dig where they happen to be." That distinction matters, because
density-dependence is the actual mechanism being claimed credit for, and "a
nest has tunnels" is consistent with several other explanations too.

**The moisture-curvature construction channel is a documented refutation,
not a gap in documentation.** Worth restating because `stigmergy-research.md`
§4 is still the design's own strongest section by citation count, and a
reader skimming only that file would come away believing sematectonic
construction (deposition following surface curvature via evaporation flux,
Facchini et al. 2024) shipped. It didn't:
`creature-genome-flexibility-2026-09-02.md` §5 measured
`creature::moisture_gradient` directly and found a convex crest reads
1.012x a flat plateau at the same elevation — the channel is sensing the
vertical air/soil boundary every surface has, not curvature, and widening
the sensor span moves the ratio toward 1.0, the wrong direction for the
claim. Deposition does follow the channel that exists; it is not yet
following the thing the theory says it should be following. This is
recorded in both files now (the status block was added to
`stigmergy-research.md` itself), which is the right fix — a superseded
design claim stays discoverable from wherever a reader lands, rather than
correct in one file and stale in the one people are more likely to open
first.

## 8. Beyond stigmergy: what a broader emergence programme is still missing

Everything above classifies what exists. This section answers the different
question the owner asked directly: **if the goal widens from "a simulation
that can exhibit stigmergy" to "a simulation that exhibits emergence
generally," what's not here yet** — as features, as design principles, and
as research to do before building. Ordered by how directly it extends
substrate already in place, cheapest first.

### 8a. Facilitative stigmergy — the engine only has the competitive half

Every inter-organism indirect interaction shipped so far is **inhibitory**:
a root depletes moisture and a neighbour's gradient read steers away
(§4 above); a canopy shades the ground and suppresses what grows there. Both
are real stigmergy, and both are competition. Nothing in this engine lets
one organism's presence indirectly *help* another's through the environment —
the only facilitation on record is `plant.rs`'s auxin channel, and that is
scoped to one organism's own tips, not between organisms.

Real ecosystems run on both halves at once, and the ecology literature is
explicit that they interact rather than simply add: nurse-plant facilitation
in harsh environments, and — the concrete, well-studied case most directly
portable here — mycorrhizal fungal networks physically connecting separate
trees' root systems and moving carbon, water and nitrogen between them,
demonstrably including from established "mother trees" toward seedlings
([Simard et al.'s isotope-tracing work, summarized in *Mycorrhizal Networks
Facilitate Tree Communication, Learning, and
Memory*](https://www.researchgate.net/publication/324710824_Mycorrhizal_Networks_Facilitate_Tree_Communication_Learning_and_Memory)).
Mechanically this is the *same* substrate the moisture channel already
provides — a shared field two agents both read and write — with the sign
flipped and the topology changed from "everyone competes for the same pool"
to "connected individuals pool into a shared one." It is a cheap addition
architecturally (one more write path on an existing channel, or a
root-graft adjacency relation between connected `Plant` cells reusing the
transport mechanics §3 of `design-philosophy.md` already commits to
building) and it closes a real conceptual gap: right now this engine's
plants can only ever be worse off for a neighbour's existence, which is not
how real plant communities work and forecloses an entire class of emergent
outcome — stands that self-organize into cooperative clusters, not just
competitively spaced ones.

### 8b. Division of labour by response threshold — a different stigmergy consumer than trail-following

Everything the ant colony does is either foraging, digging, or (implicitly)
resting — one animal, one behavioural repertoire, gated by its own hunger
and what it happens to be standing next to. Nothing in `creature.rs` or
`ant.ron` gives individual ants *different* standing propensities to
specialize into roles, and nothing in the research corpus (`stigmergy-
research.md`, `dead-ends.md`, `creature-direction.md`) covers it — this is a
genuine gap, not a documented-and-shelved idea.

The relevant model is Bonabeau, Theraulaz and Deneubourg's **fixed
response-threshold model**: each worker carries its own threshold for each
task, task-associated stimulus (brood needing food, tunnel needing
clearing, refuse needing removal) rises when unaddressed and falls when
worked, and an individual engages once the stimulus crosses its personal
threshold — with **no communication or central assignment at all**
([Royal Society, *Quantitative study of the fixed threshold
model*](https://royalsocietypublishing.org/rspb/article/263/1376/1565/69462/Quantitative-study-of-the-fixed-threshold-model);
[Springer, *Fixed response thresholds and the regulation of division of
labor*](https://link.springer.com/article/10.1006/bulm.1998.0041)). This is
a distinct emergence mechanism from trail-following stigmergy, worth
naming as its own category precisely because it is easy to mistake for the
same thing: it produces *specialization* (a colony with distinct forager
and nest-maintenance sub-populations, without anyone deciding who does
what) rather than *coordination toward a location* (the trail's job). It
fits this engine's substrate almost exactly as-is — the genome already
carries heritable per-instinct weights (`ant.ron`, the brain-weight
positional genome `dead-ends.md` documents at length), so a per-ant
threshold is one more heritable scalar, and the "stimulus" side is exactly
the kind of scalar this engine already knows how to accumulate and decay
(a starvation-pressure or refuse-density counter, shaped like the pheromone
planes but read by the colony's *own* task-selection rather than by
movement). Building it would be the first emergent phenomenon in this
engine that is about **who does what**, as opposed to every existing one,
which is about **where things go**.

### 8c. Predator–prey population dynamics — researched in depth, not yet built correctly

This is the most mature unbuilt item on this list, because the research
already exists at a depth matching `stigmergy-research.md`'s own:
`population-dynamics-research.md` (Report D of four) is a dedicated,
citation-backed study of exactly this question, and it should be read in
full before anyone builds a second predator species, not summarized from
here. Its headline finding is a hard constraint the current beetle
mechanism already violates in the way the report predicts: **the default
outcome of an unstructured two-species predator-prey system is total
extinction, not a stable oscillation** — Gause could not keep it from
happening at any starting population, and Huffaker only achieved persistence
by deliberately building spatial heterogeneity *and* handicapping the
predator's dispersal relative to the prey's. `dead-ends.md`'s own "beetles"
entry records the current state matches the *other* failure mode entirely:
beetles have no pheromone instincts and run-and-tumble at random, so
`beetles:0` and `beetles:9` runs are bit-identical over 6,000 frames — no
predation pressure exists to go extinct *from*, which is a different and
more basic gap than the population-dynamics report is warning against, and
worth closing first.

The report's design consequences are concrete and specific enough to build
against directly: prey must disperse *better* than predators, as a designed
asymmetry rather than a tuned equality; barriers and patchiness matter as
much as raw heterogeneity, and there is an interior optimum ("patchy but not
too patchy") rather than a monotonic dial; and a three-species cyclic-
dominance structure is more robust to the instability than any two-species
chain, worth considering directly rather than as a stretch goal. Two of its
warnings connect straight back to this report's own §5: mobility has a sharp
critical threshold above which coexistence collapses (the same
population-has-a-phase-transition shape as stigmergy's fifty-worker floor,
arrived at independently in a different subsystem), and this engine's
individual-based, non-continuous population representation means it has no
"atto-fox" safety net — a species here that dips to zero is gone permanently,
with no fractional-individual reservoir to regrow from, which raises the
stakes on getting the spatial-structure design right before shipping a
second predator, not after.

### 8d. Ontogenetic plasticity — a third generator of adaptive complexity, currently entirely absent

This engine's creature genome already gives one axis of adaptation:
**phylogenetic** — trait values that change across generations by mutation
and selection, `creature-genome-flexibility-2026-09-02.md`'s whole subject.
It also has, per this report's own §2–§4, an **ecological** axis — behaviour
that adapts within a single lifetime by reading and reacting to a shared
world channel (a trail gets stronger, a moisture gradient gets steeper, and
an individual's *next* action changes accordingly, with no change to the
animal itself). What's missing is the third axis every real nervous system
adds on top of both: **an individual's own decision function changing
within its own lifetime as a function of its own experience** — the
textbook case being Hebbian synaptic plasticity, but the general principle
is any rule that lets `creature.rs`'s brain weights move during a single
ant's life rather than only across `genome_from_wiring`'s generational
mutation step.

This is worth naming as a distinct category rather than folding it into
"more genome work," because it changes what kind of unplanned behaviour is
possible. Evolution discovers strategies across a population over many
generations; stigmergy lets a population coordinate through a channel none
of them privately remember; neither lets *one individual* get better at its
own environment over its own lifespan, which is the specific thing real
foragers do (associative learning that shifts an ant's own trail-following
threshold based on its own history of reward, not the colony's). Building it
does not require new substrate so much as relaxing an existing constraint:
`ant.ron`'s brain weights are currently authored once per genome and fixed
for an individual's life; the smallest version of this is a single
self-recurrent weight per relevant input, updated by a fixed local rule
after each reward event, which is the same "authored hidden unit" pattern
`creature-genome-flexibility-2026-09-02.md` §4 already uses for the odometer
idea, aimed at a different target (weights that move within a life, not
just a memory that accumulates within one).

### 8e. Direct multi-agent interaction — a foreclosed category worth naming as a deliberate trade-off

`emergent-world-architecture.md` §12's standing review question 5 is
unconditional: *"Do two agents talk to each other anywhere in this? They
must not... everything goes through the world."* That rule is exactly right
for stigmergy, and it is worth being honest that it also rules out, by
construction, one of the two or three most famous classes of emergent
behaviour in the entire field: **Reynolds-style flocking**, where each
boid's heading is a weighted average of separation, alignment and cohesion
computed directly against its *nearby neighbours' own state* — not against
any environment channel
([Wikipedia, *Boids*](https://en.wikipedia.org/wiki/Boids); the 1987 SIGGRAPH
original). A flock, a school of fish, and a herd's collective evasion are
not stigmergic in the textbook sense at all — no deposit, no decay, no
environment mediation — they are direct social-force averaging, and this
engine's own architectural rule currently forbids building them as
literature describes them.

This is not necessarily wrong — the rule was adopted for good reasons this
report's own §5 documents (determinism, O(population) blow-up, the
private-state-arena failure mode) — but it should be a decision made with
the cost visible, not a gap nobody noticed. The honest research question, if
coordinated group *movement* (as opposed to construction or foraging) is
ever wanted: can a field-mediated approximation get close enough? A
"crowd density and mean local heading" field that agents both write (their
own position and current heading) and read (nearby agents' averaged heading,
sampled the way pheromone already is) would keep the letter of the
architectural rule while approximating alignment and cohesion; separation
already exists for free as ordinary CA collision. Whether that approximation
actually flocks, or whether the field's spatial coarseness (§5's resolution
findings apply here too — flocking needs neighbour-scale resolution, same as
trail width) destroys the effect the way it nearly destroyed pheromone, is
an open, cheap-to-test question and not yet asked anywhere in this codebase.

### 8f. Percolation and threshold connectivity — already found once, not yet generalized

Unlike the items above, this one does not need to be discovered — it already
was, independently, outside any emergence-framed discussion.
`grassfire-and-the-desert-2026-08-23.md` measured fire spread through a
sward and named it explicitly: *"fire spread here is a percolation — a
sward either carries a fire the width of the world or stops it inside a
hundred cells, with very little between."* The measured numbers are a
textbook percolation transition — a sward at 0.30 wetness burns 80%, the
same sward at 0.40 burns 22%, and one specific configuration was found
"sitting on the percolation threshold and flipping" between 99.9% and 27.6%
across small wetness changes. Nobody needed percolation theory to build
this; the CA rule (spread to a flammable neighbour or don't) produces the
phase transition for free, which is itself a small illustration of this
report's §1 point that weak emergence needs no imported theory to occur,
only to be *recognised*.

What hasn't happened is generalizing the recognition. The same
threshold-connectivity shape — a giant connected component appearing
suddenly once local occupation crosses a critical density, with almost
nothing in between — is a candidate mechanism for several things this
engine currently generates by authored noise fields instead: whether a cave
network reaches the surface, whether an aquifer's water table connects to a
river, whether rot or an infestation spreads through a stand of trees planted
at a given density, whether an ore vein or root system forms one connected
mass or many isolated pockets. None of these need new theory to try — the
CA substrate that produced the fire-spread transition for free is the same
substrate worldgen and decay already run on — but nobody has asked, for any
of them, "is this a percolation, and if so, where is its threshold and how
sharp is it," which `grassfire-and-the-desert-2026-08-23.md`'s own method
(measure the outcome across a fine sweep of the density-like parameter,
looking for the same-configuration flip) is a ready-made instrument for.

### 8g. The instrumentation gap: no general answer to "is this actually the pattern," only "did it fire"

CLAUDE.md's Method section and this report's §6 both establish, at length,
that this engine is disciplined about **counters** — "did it fire" is
answered rigorously everywhere (`PheromoneStats::tiles_processed`,
`FailureCounts::crumbled`, the structural distance census). It has no
equivalent discipline yet for **shape** — "is the spatial or size
distribution actually the thing the theory predicts," as opposed to "did
something happen and does it look right by eye." Three places in this
report alone need an instrument that does not exist:

- §7's tiger-bush banding needs a way to measure whether a canopy-density or
  moisture field actually has a **characteristic spatial wavelength** —
  the standard tool is a 2D spatial autocorrelation or radially-averaged
  power spectrum, looking for a peak at a nonzero spatial frequency, which
  is categorically different from "an ascii render looks stripy."
- §4/§8c's fracture and cascade sizes need the **peaked-vs-power-law
  histogram test** `granular-mechanics-research.md` §8c already specified
  for avalanches, reused rather than reinvented for rock.
- §8f's candidate percolation systems need the same **fine-sweep-for-a-flip**
  method `grassfire-and-the-desert-2026-08-23.md` already used, generalized
  into a reusable harness rather than a one-off measurement.

All three are cheap relative to the mechanisms they would verify, and none
of them are metrics in the sense CLAUDE.md warns against (a single number
that answers the wrong question) — they are closer to the order-statistic
and control-scene discipline `emergent-world-architecture.md` §10 already
asks for, extended from "did it happen" to "does it have the shape the
literature says it should." Building even one of them — the tiger-bush
autocorrelation check is the cheapest, since both writer channels already
exist and only the scene and the measurement are missing — would convert
this report's single largest "claimed but not observed" item into either a
real result or a real, specific dead end.

---

## Sources (general emergence and complexity literature; ant/termite/ACO
sources are in `stigmergy-research.md` and not repeated here)

- [Emergence — Wikipedia](https://en.wikipedia.org/wiki/Emergence)
- [Chalmers, *Strong and Weak Emergence*](https://consc.net/papers/emergence.pdf)
- [The biological principles of swarm intelligence — *Swarm Intelligence*
  (Springer)](https://link.springer.com/article/10.1007/s11721-007-0004-y)
- [Stigmergy — ScienceDirect Topics
  overview](https://www.sciencedirect.com/topics/engineering/stigmergy)
- [An Analysis of Vegetation Stripe Formation in Semi-Arid Landscapes —
  *Journal of Mathematical
  Biology*](https://link.springer.com/article/10.1007/s00285-005-0319-5)
- [Self-organized critical forest-fire model (Drossel & Schwabl) — *Phys.
  Rev. Lett.* 69,
  1629](https://link.aps.org/doi/10.1103/PhysRevLett.69.1629)
- [Forest-fire model — Wikipedia](https://en.wikipedia.org/wiki/Forest-fire_model)
- [The BAK-TANG-WIESENFELD Sandpile — SocSim
  documentation](https://socsim.readthedocs.io/en/latest/BTW.html)
- [Quantitative study of the fixed threshold model for the regulation of
  division of labour in insect societies — *Proc. R. Soc.
  B*](https://royalsocietypublishing.org/rspb/article/263/1376/1565/69462/Quantitative-study-of-the-fixed-threshold-model)
- [Fixed response thresholds and the regulation of division of labor in
  insect societies — *Bull. Math.
  Biol.*](https://link.springer.com/article/10.1006/bulm.1998.0041)
- [Boids — Wikipedia](https://en.wikipedia.org/wiki/Boids)
- [Mycorrhizal Networks Facilitate Tree Communication, Learning, and
  Memory](https://www.researchgate.net/publication/324710824_Mycorrhizal_Networks_Facilitate_Tree_Communication_Learning_and_Memory)
