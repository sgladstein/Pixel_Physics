# Ecological LOD: how an ecology survives a world that is not simulated

**Answers `population-dynamics-research.md` §7b**, which flagged off-camera
catch-up as unresolved: *"a chunk that unloads with two prey and one predator and
reloads 10,000 frames later has no cheap correct answer."*

**Status: recommendation, not settled.** §1–§3 and §6 are architecture the rest of
the document depends on and are proposed as decisions. §4 is the direct answer to
§7b and is the one that needs owner sign-off before M10 sequencing can be trusted.
§8 is a verification protocol, not a measured set of bars — the numbers it does not
yet have are marked as such rather than invented.

Depends on: `population-dynamics-research.md` (§2 space, §3 atto-fox, §7a chunk
sleeping, §7c slot overflow), `worldgen-design.md` (§4 `worldgen(seed, coord,
world_age)`, §8 persistence taxonomy, §9 do not hardcode the depth split),
`emergent-world-architecture.md` (§9 field sleeping as the binding constraint).

---

## 0. Summary, stated first

1. **The mechanism that prevents ecological collapse lives precisely in the part
   of the world that cannot be afforded at full fidelity.** §2 of the population
   report establishes that space — refuges, dispersal lag, spatial decorrelation —
   is what makes coexistence possible at all. Chunk sleeping and M10 unloading are
   the machinery for *not simulating* space. These are not neighbouring concerns;
   they are the same concern.

2. **The resolution is ecological LOD: three tiers, each a lossy projection of the
   one below, with a defined lower and lift.** The lift already exists in design —
   `worldgen-design.md` §4's `worldgen(seed, coord, world_age)` is exactly the
   function that regenerates a chunk of a given age. This document adds the
   ecological state to it and names the tier boundaries.

3. **A small set of conserved currencies is what makes the tiers a simulation
   rather than a diorama.** Water, biomass and one nutrient, balanced at every
   tier at its own resolution. Conservation is the operational meaning of
   "internally consistent" and it is what makes the refine/coarsen seam invisible.

4. **The answer to §7b: freeze individuals, advance fields and the patch tier,
   quantize everything to integers.** Freezing individuals avoids the atto-fox
   (§3). Advancing the aggregate tier means the world lived while you were away
   rather than pausing, which is what §5e of the architecture report ("nothing
   reseeds") and its "the world currently runs down" finding both need.

5. **§7a's accidental refuge should be made deliberate rather than removed.** A
   sleeping chunk is an accidental refuge or an accidental extinction. Refuges are
   Huffaker's stabilizing mechanism. Make the coarse tier a *refuge tier* on
   purpose — slow, buffered, hard to drive extinct — and the performance
   architecture and the coexistence architecture become the same architecture.

6. **One hard rule keeps this from going hollow: detail flows up, never down.**
   The coarse tier may only hold state the fine tier produced, or worldgen defaults
   for territory never simulated. If a patch model *decides* composition and the
   cell simulation renders the verdict, the emergent physics is decoration and
   every world converges to the same table.

---

## 1. Why §7b is genuinely hard, stated precisely

The population report's §7b frames this as a correctness problem: what is the right
population after 10,000 unsimulated frames? That framing has no cheap answer, and
chasing it leads directly to the ODE whose atto-fox behaviour §3 warns against —
a predator population of 10⁻¹⁸ that recovers to carrying capacity.

The framing that does have an answer is different:

> The off-camera world does not owe the player a *correct* population. It owes a
> *plausible and conserved* one, and it owes the on-camera world a supply of
> colonists.

Those are much weaker requirements, and they are the ones the ecology actually
depends on. §9f of the population report — five prey must recover to carrying
capacity — is a statement about a reservoir, not about accuracy. A reservoir needs
to exist and be hard to empty. It does not need to be right.

This is also why the problem cannot be deferred behind M10. §2's finding is that
without spatial structure the ecology goes extinct at any starting population. The
off-camera world *is* the spatial structure. Getting its behaviour wrong does not
degrade the ecology at the edges — it removes the mechanism the ecology runs on.

---

## 2. Three tiers

| Tier | Holds | Ticks | Where |
|---|---|---|---|
| **Cell** | Full falling-sand fidelity — per-cell material, load, liquid fill, fields | Every frame | On-camera plus margin |
| **Individual** | Per-organism summary: species, age, size, health, stored resource, seed output, position | Coarse clock | Resident but off-camera |
| **Patch** | Per chunk-column: occupancy fraction per strategy, successional stage, soil depth, moisture and biomass totals | Very coarse, or lazily on read | Unloaded |

**Tier selection keys on active sites, never on distance or depth.**
`worldgen-design.md` §9 already establishes this for persistence and tick tiering,
and the argument transfers verbatim: today's "biologically active zone" is a
calibration of today's parameters, not an architectural fact, and the owner's
stated intent is to tune those freely. `World::active_site_count()`
(`src/sim/world.rs:837`) already reports the quantity. A distance rule would also
misclassify the interesting cases — a deep cave holding a nest is biologically
active and a barren surface chunk is not.

**Each tier must define both directions.**

- **Lower** (fine → coarse) runs at unload: aggregate cells into individuals,
  individuals into patch fractions and totals. This is a pure summary and must be
  the only writer of patch state for visited territory (§6).
- **Lift** (coarse → fine) runs at load: `worldgen(seed, coord, world_age)`
  conditioned on the stored patch summary. For never-visited territory the summary
  is absent and worldgen supplies it from `(seed, coord, age)` alone — which is
  what `worldgen-design.md` §4 already requires so that walking into fresh
  territory does not show a seam between "just generated" and "lived in".

The consequence worth noticing: **worldgen, succession and catch-up are one
function evaluated three ways.** `worldgen-design.md` §4 already says this for
terrain. Ecological LOD is the same claim extended to what lives on it, and it
means the catch-up path is not new machinery — it is worldgen with a non-empty
prior.

---

## 3. Conserved currencies

A tier boundary is honest if the quantities that matter cross it unchanged.
Proposed set, deliberately small:

| Currency | Cell tier | Individual tier | Patch tier |
|---|---|---|---|
| **Water** | `Liquid` fill + soil saturation (`aux`) | stored resource | column total |
| **Biomass** | organism cell count × material density | size | occupancy × stage |
| **Nutrient** (one, if the plant work needs it) | soil channel | stored resource | column total |

Rules:

- **Lower is exact.** Summing cells into a summary loses arrangement, not amount.
- **Lift is exact in total, free in arrangement.** Regenerating a chunk must place
  material summing to the stored totals; where it puts it is worldgen's business.
- **Advancement conserves or accounts.** Anything the patch tier removes must go
  somewhere named — evaporated, drained to a neighbour, respired — not simply
  decremented. The point is not bookkeeping purity; it is that a player who floods
  a valley and walks away must not find the water gone on return.

This is what makes a distant forest not-faked: it is the same accounting at coarser
grain. It is also the cheapest possible test of the whole architecture (§8), because
conservation is checkable exactly and needs no measured baseline.

Note the collision with existing behaviour, which must be resolved before this can
hold: the powder/liquid `aux` convention is inverted between materials (`aux == 0`
means *full* on a `Liquid` and *dry* on a `Powder`), and CLAUDE.md records that
getting it backwards manufactures water from nothing. A conservation check across a
tier boundary will find any such bug immediately, which is an argument for building
the check early rather than a reason to fear it.

---

## 4. The answer to §7b

**Freeze individuals. Advance fields and the patch tier. Quantize everything.**

| Off-camera state | Treatment | Why |
|---|---|---|
| Individual creatures and plants | **Frozen** at unload state | §3's atto-fox is a consequence of advancing *discrete* populations through a *continuous* model. Not advancing them cannot produce fractional survivors. |
| Derived fields (light, temperature) | **Discarded and regenerated** | Already the decision in `worldgen-design.md` §8 — they are functions of geometry and frame. |
| Accumulated fields (moisture deviation, pheromone) | **Persisted, then advanced** by the coarse model | §8 again. Pheromone decay off-camera is what stops a stale trail reappearing intact after a week. |
| Patch tier (composition, succession, soil) | **Advanced** on a coarse clock | This is what makes the world have lived while you were away. |

**Quantization is the load-bearing detail.** Every population-like quantity at every
tier is an integer — whole individuals, whole cells of biomass. A patch that decays
to 0.4 individuals rounds to zero and *stays* zero. This is the discreteness §3 asks
for, applied at the tier where the continuous model would otherwise sneak back in.
It is also what makes local extinction real and therefore recolonization meaningful.

**Determinism holds.** Catch-up stays a pure function of (state at unload, elapsed
time, seed), which `PLAN.md` requires. Freezing is trivially pure; the coarse
advance must draw from a stream seeded from `(seed, coord, unload_frame)` rather
than the per-chunk RNG — `Chunk::rng` is *stateful and shared with the CA sweep*,
so a catch-up draw taken from it depends on how much sweeping happened first,
which is not a pure function of the three inputs above.

> **CORRECTED 2026-08-28.** This sentence used to justify itself "for the reason
> §7d gives — `Chunk::rng` seeded from chunk coordinates makes
> position-correlated noise". That reason is wrong (see
> `population-dynamics-research.md` §7d's own correction and
> `src/sim/rng.rs:105-117`): the defect is **order coupling**, not position.
> The recommendation here is unaffected — a `(seed, coord, unload_frame)`
> stream is still the right call, and `rng::stream` is the mechanism for it —
> but it holds for purity, not for spatial correlation.

**What this deliberately gives up:** an unloaded chunk's predator does not starve.
A chunk holding a doomed population preserves it. Both are wrong as ecology and
both are wanted as *design*, because they are the reservoir. That is the trade
being made, and it should be made with eyes open rather than discovered later.

---

## 5. Sleeping as a refuge tier

§7a says chunk sleeping silently creates "either a perfect refuge or a silent
extinction, depending on how the wake rules interact with creature scheduling",
that neither is intended, and that it must be decided explicitly.

**Decide it toward the refuge.** §2's finding is that this engine gets spatial
structure more cheaply than any purpose-built ecology simulator — Huffaker's
apparatus of oranges and petroleum-jelly barriers is an approximation of caves,
water and material-gated movement that this world already has. Sleeping adds the
one thing that apparatus also had and this engine otherwise lacks: **a rate
difference**. Huffaker did not only add space, he handicapped the predator's
dispersal.

The resulting shape:

- **The fine zone around the player is the disturbance zone.** Fast, high-variance,
  things burn and collapse and get dug out. This is where the drama is, and where
  the destruction engine already lives.
- **The coarse surround is the reservoir.** Slow, buffered, quantized, hard to
  drive extinct.
- **The tier boundary is itself the dispersal handicap.** Colonists cross it at the
  coarse tier's rate, not the fine tier's.

This is the part worth flagging to the roadmap: the player becomes a disturbance
agent whose effects are ecologically real rather than scripted, and recovery after
a dig or a fire is recolonization from a reservoir rather than a respawn timer.
That is a mechanic falling out of a performance constraint, which is usually the
sign the decomposition is right.

**Corollary that must not be skipped:** if sleeping is a refuge, then §7a's other
horn — creatures whose timers silently do not advance — is no longer a bug to fix
but a behaviour to *specify*. Frozen means frozen for hunger, age and breeding
alike. A creature that ages but does not eat off-camera is the worst of both.

---

## 6. The rule that keeps it honest

> **Detail flows up, never down.** Patch-tier state for visited territory is
> written only by lowering from the fine tiers. Worldgen writes it only where
> nothing has ever been simulated.

The failure this prevents is the one the owner named directly: a cool, complex,
internally consistent system that always produces the same consequences. If a patch
model holds authored composition and the cell simulation is a renderer for it, then
every world converges to the table, and all the physics is decoration. Convergence
would arrive by architecture rather than by tuning, and would be much harder to see.

**Operational test.** Deleting the fine-tier simulation must change patch-tier
outcomes in territory the player has visited. If patch state is indistinguishable
between a visited and an unvisited chunk of the same worldgen parameters, this rule
is being violated.

**The nice consequence:** territory never seen at fine resolution holds worldgen
defaults, and territory lived in diverges from them. The world becomes idiosyncratic
exactly where the player has spent time, and it carries the mark of that.

---

## 7. What this changes elsewhere

- **`emergent-world-architecture.md` §9 is upstream of everything here.** Field
  sleeping (issue #4) decides whether roughly one more channel is affordable or
  four to five, against a ~11.5 ms worst frame in a 16.6 ms budget. Each channel is
  an independent niche axis, and the number of independent niche axes is what
  decides whether the world supports several strategies or exactly one winner.
  **Field sleeping is not a perf chore; it is the diversity budget**, and it gates
  the ecology work as much as it gates M10.
- **§7c must be fixed before any of this ships.** A refuge tier makes populations
  persist that would otherwise die, which makes cumulative births larger, and
  `push_creature` allocates `u16` slots from a `Vec` that never shrinks behind a
  `debug_assert` that CI never runs. The free-list is a prerequisite, not a
  follow-up.
- **`worldgen-design.md` §8's table gains rows.** Individual and patch state are
  new persisted categories. Patch state should follow moisture's precedent and
  store *deviation from the worldgen baseline*, so undisturbed territory stores
  zeros and compresses to nothing.
- **`PLAN.md` M10 sequencing.** The doc already puts streaming last, after
  age-parameterized generation and catch-up. This document does not reorder that;
  it says what the catch-up step must contain.

---

## 8. Verification

Following the convention that a scene judges quality and an assertion catches total
failure, and the rule that outcomes here have enormous spread so comparisons must be
**paired** rather than measured against a remembered number.

**8a. The catch-up test is naturally paired, and that is its main virtue.** Run one
world continuously for N frames. Run a second, same seed, in which a region unloads
early and reloads at N. Compare the two.

| Must match | Tolerance |
|---|---|
| Conserved totals (§3) in the region | Exact, or a stated and justified epsilon |
| Species present / absent | Exact |
| Successional stage | ±1 stage |

| Free to differ | |
|---|---|
| Exact cell arrangement | Entirely — lift is free in arrangement (§2) |
| Individual identities and positions | Entirely |

**8b. Bars that need measurement, recorded as gaps rather than invented.** The
population report's §9a (both species alive at 100,000 frames in ≥80% of 20 seeds)
is the headline the ecology must eventually hit. This document cannot set the
catch-up equivalents — how far a caught-up region may drift, how coarse the patch
clock may be — because nothing has been measured. **They are to be set from a
measured baseline with headroom, and left visibly unset until then.**

**8c. Sweep the procedure, gate an order statistic.** This is a guard over a
procedural system, so a fixed scene cannot cover it: sweep seeds and gate p90 or max
across them, never a single seed. The failure mode being guarded against — a region
that loses or manufactures material across a tier boundary — is exactly the shape
that CLAUDE.md records as having passed all eight acceptance scenes twice while
eating fifty times more world than the bug it fixed.

**8d. Count the event, do not infer it from a picture.** "The chunk caught up" is a
discrete event and needs a counter next to any image. A reloaded region that looks
plausible proves nothing about whether the coarse advance ran at all — CLAUDE.md
records a collapse that looked like working chunks while the body count read zero
for the whole run.

**Deletion tests**, in the style of the population report's §10:

| Mechanism | Test that must fail without it |
|---|---|
| Conservation across the tier boundary | 8a — totals drift on reload |
| Quantization | A patch driven near zero recovers from a fractional remnant |
| Refuge tier | §9f — prey reduced to five individuals never recovers |
| Coarse-tier advance | A region reloaded after 100k frames is indistinguishable from one reloaded immediately |
| Detail-flows-up (§6) | A visited and an unvisited chunk of identical worldgen parameters hold identical patch state |

---

## 9. What is testable today, with only trees

The plant model is currently trees and moss, which is not enough to test coexistence
— that needs three or four functionally distinct strategies. But **the architecture
in §2–§4 does not need species diversity at all**, and is much cheaper to establish
before five species exist than to retrofit afterwards.

Testable now:

- **The tier seam.** Lower a forested region to a summary, advance, lift, and check
  §3's conservation exactly. One species is sufficient.
- **The paired catch-up comparison (8a).** Also single-species.
- **Succession as `worldgen(seed, coord, age)`.** Generating a 400-day-old stand
  directly and comparing it against a stand grown for 400 days is the same paired
  shape, and it is the check that `worldgen-design.md` §4's central claim actually
  holds.

Needs the plant expansion first: everything about diversity, §9a persistence, and
the mobility threshold.

**One recommendation for that expansion, from §6's logic.** Add strategies as points
spanning a trade-off space, not as appearances. Fast/cheap/short-lived,
slow/expensive/tall, stress-tolerant — Grime's CSR triangle is a serviceable
template and its virtue here is that it is explicitly three strategies with no best
one. Adding "a fern, a bush, a grass" as three looks with the same underlying
economics produces one winner and two also-rans, which is the monoculture outcome
arrived at a different way.

---

## 10. Open questions

- **The coarse clock's rate is unset**, and it is the main tuning knob for how alive
  the off-camera world feels versus what it costs. Needs measurement (8b).
- **Whether the individual tier is needed at all**, or whether resident-but-off-camera
  can stay at cell fidelity given field sleeping. Cheaper to answer after issue #4
  lands than to guess now.
- **Cross-slice ecology.** `worldgen-design.md` §0 reserves a slice identifier and
  keeps slice topology open. Whether a reservoir spans slices, or each slice has its
  own, is undecided and affects how large the reservoir actually is.
- **Interaction with the M10 persistence taxonomy for pheromone**, the one channel
  with no alternative to persisting in full. If the coarse tier advances it, the
  stored form may be able to shrink.
