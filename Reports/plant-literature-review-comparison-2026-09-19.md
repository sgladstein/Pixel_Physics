# An outside literature review, read against this engine

**Review, 2026-09-19. `engine`.** The owner commissioned a survey of plant
and ecosystem simulation from someone who knew only that this is "a
plant/ecology simulation game" — *A Comprehensive Review of Plant Growth and
Ecological Computational Simulation*, fourteen sections from L-systems to
dynamic global vegetation models, ending in a five-layer architecture
recommendation and a staged build plan. This report reads it against what
is actually in the tree, and answers the question that was asked: **is
there anything in it we can learn from?**

**The short answer: the engine already holds, under other names, nearly
everything the review recommends for its first two stages, and several of
the review's textbook forms are recorded dead ends here.** What is worth
taking is small and specific: a handful of cheap *pattern checks* (§5.1), a
literature name and a mechanism for one open bug (§5.2), and two ideas to
read *when* two roadmap items are scoped rather than now (§5.3, §5.4). Nothing
in it argues for changing direction, and one thing in it — a calendar
season — has already been considered here and declined for a reason the
review could not know.

*Sibling of [`plant-simulation-research.md`](plant-simulation-research.md),
which surveyed the same literature from inside the project on 2026-08 and
whose recommendations have since mostly shipped. This one is the view from
outside, and its value is exactly that: it says what a competent stranger
would expect a plant engine to have, so the gaps it names are the gaps a
player would name.*

---

## Contents

0. What the review could not know, and why that matters for reading it
1. The mapping: every mechanism it recommends, against the tree
2. What it confirms — arrived at here independently, by measurement
3. Where its textbook form is a recorded dead end here
4. Where it does not transfer
5. What is worth taking, ranked
6. What to do with the document itself

---

## 0. What the review could not know

The review is competent and its citations check out against the ones this
project already holds (Prusinkiewicz, Palubicki, Runions, Wright's leaf
economics spectrum, Hallé–Oldeman, the pipe model, Grimm's pattern-oriented
modelling are all in `plant-simulation-research.md` and
`research/m16-plant-biology.md` already). Six facts about this project
change how almost every line of it should be read:

1. **The world is a two-dimensional slice one cell thick**, and a plant is
   made of the same cells as the rock. Every 3D law the review quotes — the
   pipe model's area conservation, Beer–Lambert over leaf area index,
   space-colonisation's spherical occupancy zones — has to be re-derived for
   a projection, and `Reports/dead-ends.md` carries 25 entries on the pipe
   model alone, most of them the cost of that re-derivation.
2. **Exactness is not a goal.** `CLAUDE.md`: *looks good and realistic, in
   motion, at play scale — without ruining performance*. The review's
   accuracy column ranks models by predictive fidelity to field data. That
   is the wrong axis here, and its cost column is the right one.
3. **There are no real units.** Species are `tree`, `conifer`, `shrub`,
   `creeper`, `grass`, `herb`, `scrambler`, `reed`, `moss` — fantasy
   archetypes in a world 512 cells wide with an eight-minute day. The
   review's §11 (TRY, BIEN, GlobAllomeTree, WorldClim) parameterises real
   taxa in real climates and has nothing to attach to.
4. **The ethos outranks correctness.** Every mechanic is judged on whether
   it *feels* satisfying and on whether the player has a verb for it
   (`Reports/design-philosophy.md` §0a). The review's §9.5 arrives at the
   same conclusion for games — *push mechanistic depth on the few variables
   the player observes and acts on* — but as advice about performance, not
   as the standard the work is judged by.
5. **Frame cost is a hard constraint and the CPU does the simulation.** GPU
   fields, ML surrogates and ray-traced light (review §10) are ruled out by
   settled decisions in `PLAN.md` (CPU simulation, same-build determinism).
6. **A literal biophysical model was considered and rejected**, explicitly
   so it would not be reinvented: `design-philosophy.md` §3 — *model the
   signals that drive the behaviour (light, gravity, wind, water), not the
   biochemistry that carries those signals in real plants.* The review's
   §4.1–4.2 (Farquhar–von Caemmerer–Berry photosynthesis, Ball–Berry and
   Medlyn stomata) are the biochemistry that decision declines, and the
   review itself says FvCB is only worth it *if the player interacts with
   gas-exchange mechanics*.

None of these is a criticism of the review. They are why a document that
reads as "here is what you should build" is, for this tree, mostly "here is
what you have built, and what it is called in the literature."

---

## 1. The mapping

One row per mechanism the review recommends, in the review's own order.
**Status** is one of: **present** (shipped and measured), **present as**
(shipped under another name or in a different form), **dead end** (built,
measured, withdrawn — the `dead-ends.md` line is given), **absent, declined**
(considered and not wanted), **absent, open** (wanted and not built), or
**n/a** (does not transfer). Anchors are `file:line` at this commit.

| Review § | Mechanism | Status | Where, and how |
|---|---|---|---|
| 2.1 | L-systems | absent, declined | `PLAN.md` research: *trees should use space colonization, not L-systems*. Growth is a scored random walk (`wiki/plants.md` §How a stem is shaped), not a grammar. The growth *program* — what a tip becomes, what it puts out sideways, when an axis stops — is a heritable rule table (`plant.rs` fates), which is the one L-system-shaped thing here, and it mutates |
| 2.2 | Space colonisation / self-organising trees (Palubicki 2009) | present as | Buds compete for light (the light field) and for space (a deposit-diffuse-decay canopy-density channel), fate is set by a resource flow. Two of Palubicki's rules are in the code by name: whole-plant income is his `Q`, *intercepted light, not leaf count* (`plant.rs:9867`), and bud break is his `n = ⌊v⌋`, income divided by what one tip costs (`plant.rs:7756`, `break_buds` at `:9464`). The *attractor list* itself was removed with `TreeState` (`design-philosophy.md` §2; dead-ends 266, 795) and its kill-distance became the crowding channel. This engine *is* a self-organising tree model on a lattice |
| 2.3 | Borchert–Honda flux control | present as | Carbon moves cell-to-cell along per-face conductances that ratchet on measured flux — Sachs's canalisation made mechanical (`plant-substrate-v2-design.md` §7; `organism.rs` `VEIN_GAIN`, dead-ends 825). Surplus is then distributed to frontier cells (`plant.rs:9800` `allocate_to_frontier`) |
| 2.4 | Pipe model | present | `SecondaryThicken(pipe_ratio: 5.5)` (`tree.ron:551`), `thicken()` counting foliage *supplied* along an axis. 25 dead-end entries; §3 below |
| 2.4 | Allometric scaling (WBE 3/4) | n/a | Nothing scales by mass to a fractional power. Maintenance scales superlinearly in girth (§2), which is the same *shape* of idea arrived at from Takenaka rather than West–Brown–Enquist |
| 2.4 | Hallé–Oldeman architectural models | present as target | Named in `plant-simulation-research.md` §9 as the falsification test for the species parameter surface. The three flowering habits (`wiki/plants.md` §Plants that stop) are the first reached |
| 2.5 | Multiscale tree graph (MTG) | present as | Per-organism `Vec<OrganismCell>` sidecar with the cell holding a 12-bit slot index (`plant-substrate-v2-design.md` Decision 2) — one scale, not several, and the review's LOD reason for wanting several does not arise (§4) |
| 2.7 | SpeedTree | n/a | Rendering tool; nothing here renders geometry |
| 3 | FSPM common pool, source–sink | present as, partly | `surplus = income − maintenance` is pooled per plant and allocated to the frontier; seeds draw from it since the fence came out (`plant-equilibrium-costs-2026-08-27.md` §14, `reproductive_allocation: 0.10`, `tree.ron:526`). **Organs are still donors, not sinks** — the one half missing, and already named as the follow-on (dead-ends 979; §5.2 below) |
| 4.1 | FvCB photosynthesis | absent, declined | `design-philosophy.md` §3. What ships is the review's own cheap alternative: **light-use efficiency**, income linear in intercepted light — `intercepted / L_node × INCOME_PER_NODE × nutrient` (`plant.rs:10421`), with the day/night cycle divided out of every *decision* (`field::noon_equivalent_light`). There is no leaf-level saturating curve; saturation is emergent, from self-shading and the per-cell carbon cap (`RESOURCE_SCALE` 4.0) |
| 4.2 | Stomatal conductance | present as | Foliage spends water in proportion to light (`transpiration` and `rate` are one number since `plant-equilibrium-costs` §15a). Water stress is **two terms that must not be collapsed** (`plant.rs:1149–1164` `settle_water`): a *status* that multiplies every photosynthetic credit and shifts allocation, and a *desiccation* that drives drought shedding. The closure point is heritable — genome slot 7, authored as `stomatal_reserve: 0.2` (`tree.ron:118`). No conductance equation, but the thing Ball–Berry exists to decide — when to stop spending water for carbon — is a trait |
| 4.2 | Growth respiration | present | Construction charged at the decision, from the acting cell (`plant-equilibrium-costs` §10a) |
| 4.2 | Maintenance respiration ∝ biomass, Q10 ≈ 2 | **dead end** in the ∝-biomass form | Flat per-cell respiration *impoverished rather than shaped* (dead-ends 751): cost linear in mass against income linear in leaf count balances at any size. Shipped form: flat mass term **plus** `(q_peak / L_node)^1.5` on shoot tissue (`plant.rs:7348`, Takenaka's exponent). **No temperature term anywhere in plant code** — `plant.rs` does not read `temperature` at all |
| 4.3 | Allocation with priorities (roots first under drought) | present | Functional balance, in one line (`plant.rs:10007–10074`): the root frontier weighs `(0.5 + (1 − water_status) + anchor_stress) × slot 6` against a shoot tip's 1, and the pool `surplus − reproductive_share` splits by weight. Stresses **add**, not multiply, because with an even split *root cells fell from a median of 346 to 3 — water-limited income starves the very roots that would fix it*. Root-zone water is read at the tip, not the tank (`README` §The economy re-derived: root:shoot 7.5% → 22.4%) |
| 4.4 | Darcy / SPAC hydraulics, cohesion–tension | present as | The height ceiling *is* hydrostatic: `h_max = (turgor_source − turgor_yield) / turgor_per_cell` (`organism.rs:835`), and the aquatic report notes the waterline could set it. Water store is a capped bucket (`WATER_TANK_CONTACT_CAP` 32 contact cells) — the review's own "cheap engine proxy" |
| 4.4 | P50 embolism, irreversible drought damage | absent, open | This is the one physiological idea with a named home: `open-bugs-handoff.md` §V2 — *wood and root have no drought path at all*, and shedding a leaf reduces the very signal that shed it. §5.2 |
| 4.5 | Nutrient uptake, Michaelis–Menten | present | Soil richness on exactly that curve, arrived at by measurement (`README` §Soil nutrient status: *its shape is the finding*; `plant.rs:10297–10387`), pricing income and construction off **one** curve after having them on two cost a third of the stand. One scalar, not N and P — the deliberate position in `plant-equilibrium-costs` §10e. Renewal is a *time period* on the depleted cell; crediting it back where litter rots is a **double dead end** (`plant.rs:10202`: *built twice, reverted twice, once with a measured runaway … because it makes a pump*) |
| 4.6 | Phenology: GDD, chilling–forcing | absent, declined for now | There is no year. `DAY_NIGHT_PERIOD_FRAMES` 3600 is the only calendar; weather is a channel, not a season. Considered for crown shape and rejected — *the shape it is wanted for comes from shading and not from the calendar, and a season short enough to be visible in a tree that matures in 30,000 frames would read as flicker* (`organism.rs:893`). Rethink §6.4 lists *seasons and sex* as named, not started. §5.5 |
| 4.7 | Temperature response functions | absent | Nothing in the plant economy reads temperature. The field has an ambient temperature with a day/night swing and `noon_equivalent_temperature` (`field.rs:2067–2103`) ready for a consumer that does not yet exist |
| 4.8 | Auxin canalisation, apical dominance | present as | Canalised carbon transport (row 2.3). Plain diffusion as the dominance mechanism was withdrawn permanently (dead-ends 799). A literal auxin model is declined (`design-philosophy.md` §3) |
| 4.9 | Phototropism, gravitropism | present | Scored terms on the growth walk (`wiki/plants.md` §How a stem is shaped); roots follow soil water (hydrotropism) and turn down at the surface. `light_weight` was inert by construction until `phototropism_dir` became a real 2D gradient, and that repair reallocated the whole weighted sum (`CLAUDE.md` Method) |
| 4.9 | Biomechanics: bending, breaking, self-pruning, constant-stress tapering | present, partly | Soft tissue bends under wind (`README` §Bending status), a badly proportioned tree loses a limb (§Breaking status), crown lift by shade shedding is the self-pruning. Trunks do not sway or buckle; Eloy's constant-stress criterion is not used. `beam_probe` exists to ask whether a stress model would say anything |
| 4.10 | Mortality: carbon starvation | present | 200 consecutive ticks unable to pay the mass term of upkeep → senescent → rots at the species half-life (`README` §The economy re-derived) |
| 4.10 | Mortality: stochastic age-related | present, better than the textbook | `life_half_life: 60000` (`tree.ron:97`) is a **Weibull shape-2** hazard, rising with age (`plant.rs:4067–4093`). The flat exponential the review describes was rejected in the same doc comment: *a seedling exactly as likely to die today as a two-hundred-year oak*, and a flat rate that opens gaps culls the recruits meant to fill them |
| 4.10 | Mortality: stress-induced (drought, fire, wind) | present, partly | Fire yes; load and wind-break yes; drought only through foliage (§V2) |
| 5 | Beer–Lambert canopy extinction | present | Per-CA-column transmission, Beer–Lambert, averaged across the 16-cell field block (`field.rs:332–347`). The binary whole-block shade it replaced, and a count-columns-hit variant, are both dead ends (1229–1230). The review's clumping warning is real here: the coarser block was chosen *by eye* because a thin canopy in a taller block blocks less sky (`wiki/plants.md` §What a healthy stand looks like) |
| 5 | Voxel light grid / shadow propagation | present | `FieldTile` at 1/16 resolution carries light, moisture, temperature, pressure, velocity. `sky-light-design.md` adds seeded distance propagation for the *render* of dug spaces, having measured that the leaf channel answers the wrong question for a cave |
| 5 | Microclimate coupling (Ecoclimates) | present, in 2D | Vegetation writes canopy density and drinks soil water; evaporation humidifies air; humidity throttles evaporation; rain drips through canopy; weather is a channel the field reads. That is Ecoclimates' loop on a coarse 2D grid, at two rates (field per frame, organisms every 45) |
| 6.1 | Root system architecture (CRootBox tropisms, pipe radius) | present | Gravi-, hydro-, and a moisture-antagonism root walk (`research/m16-plant-biology.md` pass 2); roots displace soil, bind slopes, and are ground to walk on. `root-morphology-findings.md` says what the walk structurally cannot express |
| 6.2 | Richards vs bucket soil water | present as bucket-per-cell | Per-cell soil fill on a saturation / field-capacity / wilting-point curve (`plant-substrate-v2-design.md` §4) — finer than a bucket, cheaper than Richards |
| 6.3 | CENTURY / RothC multi-pool decomposition | absent, and not the fix | A per-material chain (`deadleaf → litter → soil` via `decays_into`) with **two rates per material, damp and dry** (`decay.rs:114–136`), and a 5% yield (`soil-accumulation-and-the-carbon-cycle.md` §3). New soil is left dry on purpose — both richer versions *manufacture water* (`decay.rs:182–195`). The review's own point — most litter carbon respires away — is already the reason for the 5%. The open defect is a **missing sink**, which more pools do not supply (§4 of that report: bioturbation first) |
| 6.4 | Mycorrhizae | absent | Not wanted until a third currency is (§10e trigger) |
| 7 | Crop models as equation sources | n/a | Field-scale yield in real units |
| 8.1 | Gap models | present as, emergent | Establishment under a canopy fails, a death opens light and the survivors visibly grow (`wiki/plants.md` §What trouble looks like). No patch abstraction — the cells are the patch |
| 8.2 | Perfect Plasticity Approximation | absent, open for M10 | §5.3. The one structural idea with a home on the roadmap that has no model yet |
| 8.3 | Zone of influence / field competition | present | Competition *is* the shared fields — the review's §10 "grid-based resource fields convert O(N²) to O(N)" is `design-philosophy.md` §0's *deposit → diffuse → decay → follow* |
| 8.4 | Dispersal kernels | present as, wrong shape | Seed is a falling powder that rolls; `seed_launch` throws it sideways through open cells and stops at the first obstruction, priced by `1 + k√reach` (rethink §6.6). **The kernel is uniform on `[−reach, reach]`** (`plant.rs:3591`), not the leptokurtic shape the review describes, and every shipped species has it at 0. Animal dispersal by ants is live. The wind-dispersal verb is designed and not built (rethink §6.5). §5.4 |
| 8.4 | Succession | absent, open, with a named trigger | `plant-equilibrium-costs` §10e: revisit a fertility loop when *the same species wins on the same ground indefinitely*. Two thirds of the loop exist and return nothing to the soil — and the obvious return half, crediting fertility where litter rots, is the pump in row 4.5 |
| 8.5 | Disturbance | present | Fire regimes through grass, load, wind-break, burial, the player's tools, colonies eating |
| 8.6 | Matrix / integral projection models | n/a | Population is individuals |
| 8.7 | Herbivory, seed predation, mutualism | present | Ants graze litter and the seed bank, carry seed, occasionally plant it (`wiki/plants.md` §The seed bank) |
| 8.10 | Cellular automata vegetation | present — it is the substrate | The review rates CA *low–medium* accuracy because it imagines discrete states. Every organism cell here carries continuous scalars in the sidecar; the CA is the spatial substrate, not the model |
| 10 | Multi-rate integration | present | CA every frame, field every frame with settle-skipping, organisms every 45 frames, seeds every 4, five independent world clocks (`README` §World speed). `PLANT_SIZE_CADENCE` (`plant.rs:8078`) then bands the organism tick by plant size, 1x–5x — off in the outdoor world, on in the lab and the held world. **And the cadence is not behaviour-preserving**, measured: it changed standing litter 6.9x between two games (`lab-vs-druid-plant-mechanics-2026-09-15.md`), and its own doc says *what it costs is not frame budget, it is plant time* — the review's *transitions must conserve mass* warning, already paid for |
| 10 | Simulation LOD conserving mass | designed, not built | `ecological-lod-design.md` §4: freeze individuals, advance fields and a patch tier, quantise to integers. A continuous population model was rejected as the atto-fox (dead-ends 1619) |
| 10 | Deterministic per-entity RNG streams | present | `rng::stream(organism_id, x, y, frame)` (`plant.rs` `seed_genotype` doc), same-build determinism required (`PLAN.md`) |
| 10 | ECS / data-oriented layout | present as | Cell-typed behaviours in species files dispatched over a sidecar; the shape without the framework |
| 11 | Trait databases | n/a, but see §2 | Díaz's six-trait spectrum is a check on the genome axes, not a data source |
| 12 | ODD protocol | absent, declined | The always-loaded context budget is already gated (`scripts/contextbudget.py --gate`); `wiki/` + `Reports/` is the ODD in effect |
| 12 | Pattern-oriented modelling, sanity checks | partly present | §5.1 — the one cheap thing worth building |
| 12 | Sensitivity analysis (Morris, Sobol) | absent | §5.6 — a method note |
| 13 | ML surrogates, digital twins, differentiable models | absent, declined | CPU, determinism, and no field data to twin |

---

## 2. What it confirms

Worth stating because each of these was arrived at here by measurement,
usually after the textbook version failed, and the review shows the shape
that survived is the shape the field settled on too.

- **Michaelis–Menten for nutrient uptake.** `README` §Soil nutrient status:
  a linear weight failed at every setting because one number set both how
  hard a healthy stand is taxed and whether a rootless plant can die; the
  saturating curve made "no soil is fatal" structural. The review names the
  same curve (Barber–Cushman) as the standard.
- **Light-use efficiency, not FvCB, for real time.** The review says LUE
  suffices unless the player plays gas exchange. Income here is linear in
  intercepted light, and the one refinement the engine needed was not a
  saturating curve but *dividing the day/night oscillator out of every
  decision* — a problem the review does not mention because its models run
  on daily means.
- **Source–sink from a surplus pool, reproduction after maintenance.**
  `plant-equilibrium-costs` §10b's option C — *charge the decision against
  the account it should have been drawing on* — is the review's §4.3
  recommendation word for word, and it shipped (§14).
- **The leaf economics spectrum and the wood-density trade as the heritable
  axes.** Foliage tone *is* the leaf's price, bark tone *is* the wood's
  density (`wiki/plants.md` §Colour is a readout). The review's §11 adds
  Díaz et al. 2016: three quarters of global trait variation sits on two
  dimensions built from **height, wood density, leaf area, leaf mass per
  area, leaf N and seed mass**. Map those onto the genome: the turgor
  ceiling (height), bark tone (wood density), `leaf_cluster` and
  `leaf_spread` (leaf area), foliage tone (LMA and leaf N together, since
  the engine has one leaf cost), seed provisioning (seed mass). In slot
  terms (`organism.rs:6511–6552`): turgor per cell is slot 3, the wood
  density and leaf rate/transpiration alleles are discrete loci, and the
  rest of the ten slots — branch chances, plastochron, pipe ratio, root
  tropism gain, root:shoot bias, stomatal closure — are the *architecture
  and physiology* the spectrum's two axes do not carry, which is where a
  slice-world's silhouette actually lives. **The engine's heritable axes
  contain the global spectrum's axes**, chosen without reference to it. That is the strongest single piece of evidence in the
  review that the genome design is pointed the right way, and it is a
  useful shortlist when authoring a new species: pick a point on the
  size axis and a point on the leaf-economics axis, and most of a real
  plant's strategy follows.
- **Age-dependent, not flat, mortality.** The review's gap models use
  *combined deterministic + stochastic* mortality; the engine's
  Weibull-shape-2 hazard is the deterministic half made stochastic in the
  right direction, and the review's *carbon-starvation / growth-efficiency
  rule* is the shipped death rule.
- **Grid resource fields as the O(N) competition trick, and multi-rate
  stepping.** Both are the engine's founding architecture
  (`emergent-world-architecture.md`), described in the review as the key
  scalability move of Ecoclimates and LANDIS-II.
- **Self-thinning happens without being written.** The held world's stand
  *peaks near 10,000 frames and then self-thins* (`README` §Held world
  status); `plant-species-authoring.md` §7 calls a regular-pitch thinned
  stand *a real forestry result, not a defect to fix*; the nutrient tax
  produced fewer, larger plants at harsh settings and not at shipped ones.
  The review lists the −3/2 self-thinning law as the first sanity check a
  developer should run, and §5.1 is about making that a readout.
- **Games should push depth only where the player looks.** Review §9.5's
  conclusion is the ethos, reached from performance rather than from feel.

---

## 3. Where its textbook form is a recorded dead end here

Listed so nobody imports one of these from the review as new. Each has the
`dead-ends.md` line or the source comment that carries the condition.

| Review recommends | What happened here | Record |
|---|---|---|
| Maintenance respiration proportional to living biomass | Flat per-cell cost against income linear in leaf count balances at any size — it impoverished the plant without bounding it. Superlinear in girth is what bounds size | dead-ends 751; `plant.rs:7319–7348` |
| Q10 temperature response on respiration | No consumer of temperature exists in plant code, and the one seasonal question asked (crown shape) was answered by shading. Adding a temperature term now would be *a channel with a reader and no writer* in reverse — a writer nobody has asked for | `organism.rs:867–897`; `plant-equilibrium-costs` §10e's channel rule |
| Apical dominance from resource flow | Emerges from plain diffusion — withdrawn entirely, *diffusion is an equalizing operator*. Needs the flux-reinforced conductance that shipped as Decision 6 | dead-ends 799–800 |
| Pipe model as *a cheap, robust rule for diameter* | Cheap in 3D. In a 2D slice, measuring cross-section from a cell's neighbours always reads 2 at a growing end and thickening runs away; counting the whole organism thickens everything equally; only *foliage supplied along an axis* works, and the constant then had to be re-derived after a traversal bug was fixed | dead-ends 705–706, 933–939; `CLAUDE.md` §Fixing a bug often exposes a constant |
| Space colonisation with explicit attractor points | Removed with `TreeState`; the attractor list was private per-organism state the philosophy minimises, and its space-filling role moved into a shared canopy-density channel | dead-ends 266, 795; `design-philosophy.md` §2 |
| Auxin transport, xylem/phloem, hormone signalling as such | Considered and rejected: *real hormone transport runs on timescales a game will always compress past relevance* | `design-philosophy.md` §3; dead-ends 687 |
| A flat stochastic mortality rate for turnover | Kills the recruits meant to fill the gaps it opens; `Hazard.chance` 0.02 added nothing to lineage depth and cost 11% of births | `plant.rs:4067`; dead-ends 1829 |
| Permanent meristem senescence (a tip that ages out) | Grafting experiments falsify it; the replacement is reversible dormancy | dead-ends 787–788 |
| Root:shoot ratio bounds as allocation | A ratio bound lets both halves grow without limit while staying in band | dead-ends 653 |
| Waterlogging as reduced uptake or slowed growth | Backwards — oxygen is the scarce resource; the grounded mechanism is root-tip necrosis on a duration gate | dead-ends 667, 737 |
| A continuous population model for off-camera cohorts | The atto-fox: a predator population of 10⁻¹⁸ that recovers. Off-camera individuals are frozen and every population quantity is an integer | dead-ends 1619; `ecological-lod-design.md` §4 |
| A threshold on light or temperature for a decision | Any threshold sampled at an arbitrary phase of the 20:1 day/night oscillator is a nightly extinction event — this is the repo's most-recurring method failure, and the review's phenology models (GDD, chilling units) would meet it on their first frame unless built as *accumulators*, which is the one shape that survives it | `CLAUDE.md` §A designed oscillator must be divided out |
| Throwing seed further as the fix for a stand that does not spread | *A tree is budget-limited, not distance-limited, and throwing its seed costs it seeds* | dead-ends 623–624 |
| Litter decomposition returns nutrients to the soil (CENTURY's loop, and the review's §6.3) | Crediting fertility or moisture where litter rots makes a pump — tree sheds, litter rots into resource, tree drinks it, sheds more — measured as a stand that never stops growing (1,718 → 2,652 cells and climbing). Built twice, reverted twice. Renewal is by time on the depleted cell instead | `plant.rs:10202`; `decay.rs:182–195`; `soil-accumulation` §4A |

---

## 4. Where it does not transfer

- **Everything in §11 (trait databases, climate reanalysis, FLUXNET
  calibration)**: no real taxa, no real units, no field data. What survives
  is one idea (§2, the six-trait spectrum) as an authoring shortlist.
- **§7 crop models**: yield in a field with a fixed planting density is the
  one thing this world never does.
- **§8.6 matrix and integral projection models**: the population is
  individuals on cells; nothing needs a stage-class abstraction.
- **§10's MTG multiscale graph for simulation LOD**: its purpose is to
  coarsen a plant to axis level for distant plants. Here the distant plants
  are simply not swept — the dirty-rect skip and the organism tick interval
  do that job for free, and `PLANT_SIZE_CADENCE` is the crude form of the
  LOD already, with its cost measured (the tick it bands *is* the economy).
- **§10's ECS, GPU and ML surrogates**: settled decisions rule them out.
- **The accuracy/cost table**: it is priced for 3D FSPMs at field scale.
  Its row for cellular automata assumes discrete cell states and calls the
  paradigm low-accuracy; this engine's cells carry continuous carbon, water,
  conductance and age, and the review's own §9.5 admits falling-sand games
  are the one place CA plants *grow along moisture/light gradients*. The
  engine is past where the review thinks the paradigm ends.
- **§12 ODD**: a documentation protocol for reproducing a published model.
  This project's problem is the opposite — too much always-loaded
  documentation, gated by `contextbudget --gate` — and its equivalent of
  ODD's *purpose, entities, process scheduling, submodels* already exists as
  `wiki/`, README's architecture section and the species files.

---

## 5. What is worth taking, ranked

Ranked by what it buys against what it costs, with the demonstration each
would need before it counts as done.

### 5.1 Pattern checks as instruments, not as gates

Review §12's seven *sanity checks a game developer can run* are the one
section that maps onto a real gap: the plant line has excellent
**instruments** (`instruments.md` §Plants lists eleven) and no readout for
the **stand-level patterns** a forester would look at first. Most of the
seven are already answerable or already true; the point is to make them
printable so a change can be read against them.

| Review check | State here | What would make it a readout |
|---|---|---|
| Self-thinning: density × mean mass tracks −3/2 (or −4/3) | Observed by eye and in two prose lines; never plotted | `labstats`/`plant_probe` already count plants and cells per tick. Print `log(mean cells per plant)` against `log(plants)` over a run and the slope. **Read the slope's sign and rough magnitude, not its third decimal** — a flat line says competition is not doing its job, a steep negative one says it is, and a 2D slice has no reason to hit −3/2 exactly |
| Succession: pioneers precede shade-tolerant | Absent by design until a fertility loop (§10e trigger) | Nothing to build; the trigger already names the symptom |
| LAI plateaus at realistic values | Foliage share is measured (`plant_probe`); it plateaus | Already a readout. The realistic *value* does not transfer to a slice |
| Height–diameter allometry | `plant_probe` prints height and thickness per plant; never regressed | One log-log fit per run. Cheap, and the pipe-model line has 25 dead ends that a standing allometry readout would have shortened |
| Sub-canopy light follows Beer–Lambert | True by construction (`field.rs:332`) | Nothing |
| Mass conservation: carbon in = growth + respiration + litter | **Fails by design and is known to**: *a plant fixes an abstract carbon resource out of light and builds a solid cell with it. Matter enters the world from nothing* (`soil-accumulation` §1) | Not a check to add; it is the open sink problem, already ranked |
| Response direction: drought and shade reduce growth | Guarded — `a_tree_denied_water_dies_and_a_watered_one_does_not`, the shade-death sweeps | Already a gate |

So the actionable residue is **two log-log readouts** — self-thinning
slope and height–thickness slope — printed by an instrument that already
has the numbers. Both fit `CLAUDE.md`'s rules better than a bar would:
they are *paired comparisons* by nature (before and after, same seeds),
they measure a *shape* rather than a value, and they have an obvious
positive control (turn competition off with a sparse planting and the
slope must go flat; `CLAUDE.md` §Ask what your number counts). Neither
should gate anything, because exactness is not the goal and the review's
exponents are 3D field values. Cost: an afternoon, in an example that
exists.

### 5.2 A name and a mechanism for §V2: hydraulic failure, graded

`open-bugs-handoff.md` §V2 is the plant economy's oldest open limit:
drought acts only on foliage, so a plant that has shed its leaves is no
longer being killed by drought but by its bill, *wood and root have no
drought path at all*, and shedding a leaf reduces the very signal that
shed it. The review's §4.4 and §4.10 give this the literature's name —
**hydraulic failure**, the second of the two debated tree-mortality
mechanisms beside carbon starvation, which the engine already has — and a
mechanism shaped for it: **loss of conductance under sustained water
deficit, irreversible past a threshold** (the P50 vulnerability curve).

In engine terms: the per-face conductances that canalisation already
maintains (`organism.rs` `VEIN_GAIN`) are the conducting tissue. A stem
cell whose plant has run a water deficit for a sustained stretch loses
conductance on its faces; below some fraction it does not recover. That
gives bare wood a drought path that is **graded** (conductance falls in
steps, the plant's reach shortens from the tips inward, which is the
die-back geometry already built) rather than a second binary death, and
it gives drought a *mark left behind* — dead tips on a living tree —
which the ethos asks of every event. It also has a ready instrument:
`plant_reach` already asks *how far does the vein model carry carbon
through a grown tree*.

This is a candidate, not a plan. Two things have to be true before it is
worth building: §V2 has to be the thing being picked up (it is ranked
there with follow-ons), and the deficit signal it keys on has to be one
that does not vanish when the leaves do — which is §V2's own first
finding, and the reason nothing simpler has closed it.

### 5.3 The Perfect Plasticity Approximation, for the patch tier — when M10 is scoped

`ecological-lod-design.md` §4 decides that an unloaded chunk freezes its
individuals and *advances a patch tier* on a coarse clock, quantised to
integers, and leaves the patch tier's plant model unspecified. The review's
§8.2 describes the PPA as *the single most valuable idea for scaling
forests accurately without simulating every crown geometrically*: assume
crowns fill canopy space perfectly, and stand dynamics reduce to one
number per patch — the canopy height `Z*` separating sun from shade
individuals — with cohorts above and below it.

Two things make it a better fit here than in 3D. In a side-on slice
**`Z*` is literally the skyline**, a height per column, and the "perfect
plasticity" assumption that crowns spread sideways to fill light is what
the shade-shedding walk already does. And PPA cohorts are **counts** of
individuals in height classes, so they satisfy the LOD design's
quantisation rule where the rejected ODE model did not. It would also
respect the design's one hard rule — *detail flows up, never down* — since
a `Z*` and cohort counts are things the fine tier produces and the coarse
tier only advances.

Read Strigul et al. 2008 and Purves et al. 2008 when the M10 catch-up
work is scoped, and not before: nothing on the current world needs it,
and the LOD design is explicit that §4 needs owner sign-off before M10
sequencing can be trusted.

### 5.4 Dispersal: the verb is queued, and the kernel is the wrong shape

Review §8.4 names the empirical fact about seed dispersal that every
field study agrees on: kernels are **leptokurtic** — most seed lands at
the parent's feet and a *few* go very far (2Dt, log-normal, exponential
tails) — and the tail is what lets a lineage reach a niche it is not
standing next to (`plant-evolution-design.md` §5d says the same).

The shipped `seed_launch` draws its distance **uniform on
`[−reach, reach]`** (`plant.rs:3591`). Its comment reasons that *most
seeds still land near the parent* because the distribution is symmetric;
with a uniform draw that is only true relative to the far end, and the
"a few go a long way" line in `wiki/plants.md` §Individuals of one species
differ too describes the obstruction truncation, not the draw. It has cost
nothing so far because every species authors 0, and the measured +38%
far-dispersal headline (rethink §6.1) was taken with the uniform kernel
and then retired by pricing anyway.

When the wind-dispersal verb is built (rethink §6.5 — designed, its
prerequisite measured, unbuilt only because the session ended), draw the
distance from a fat-tailed kernel rather than a uniform one. It is one
line, it is what the wiki already claims, and it is the difference between
a stand that spreads as a slowly widening block and one that throws an
occasional outlier into a distant gap — which is the graded outcome the
ethos asks for. The wiki line is owed a correction when the lever goes
live, not before.

### 5.5 A year — declined, with the condition under which to revisit

The review's §4.6 phenology is the largest thing it has that the engine
does not: dormancy, budburst, leaf fall on a calendar. Three facts decide
it for now. The seasonal question that was actually asked here — crown
lift — was answered by shading, correctly (`organism.rs:867`). A year at
the shipped eight-minute day is *hours* of play, so any season a player
would see in a tree that matures in 30,000 frames (~8 days) would be a
flicker. And the appearance end-state that wants autumn colour
(`plant-appearance-design.md` §7) wants it *derived from the temperature
channel*, not from a calendar.

The condition to revisit: **when the weather channel drives cold spells
long enough to read as seasons.** `weather::Pin` already grades nothing →
wind → cold → wet → violent, with `Frost` and `Blizzard` above real
freezing thresholds, and `weather.rs:1674` already describes frost that
kills plants as an intent (a held-world spell) that nothing implements; a
cold air mass that lingered would be a winter the world *had*, not one the
calendar decreed, which is the emergent route
`plant-evolution-design.md` §5c prefers for turnover. If that is ever
built, the review's two structural lessons hold: build phenology as an
**accumulator** (degree-days and chilling units are integrals, which is
the oscillator-proof shape), and gate a *rate*, never a threshold on a
sampled value.

### 5.6 A method note: elementary-effects screening for inert levers

The plant line has found inert levers one at a time, each expensively:
`light_weight` inert across 1,024 genomes; three architectural levers
that fired thousands of times and moved nothing; the *free-lever list*
that needed a hand audit (`plant-equilibrium-costs` §3); five of eight
genome slots reading byte-identical at a short horizon. Review §12 names
the standard tool for this — **Morris elementary-effects screening**, a
one-at-a-time sweep over every parameter that ranks them by how much the
output moves. `genome_reach --grow=1` is half of it (one slot widened to
its bound); Morris does all of them in one design and ranks them.

Two of `CLAUDE.md`'s rules bound it. The noise floor is enormous — *two
plants of one species differ in size almost entirely for reasons that are
not their genes* — so the screening has to read **shape descriptors**
(`clone_variance`'s H² set), not size, and it needs seeds enough that a
per-seed median is meaningful, which the six-seed lesson says is more than
six. And every arm must prove its edit touched only its target (the
`crowding_weight` `sed` trap). Priced honestly it is a day, and it is
the one method suggestion in the review that would have shortened a past
phase.

---

## 6. What to do with the document itself

- **Keep it beside `plant-simulation-research.md`**, not in place of it.
  That report is the inside view and its recommendations shipped; this one
  is the outside view and its recommendations mostly *had* shipped. The two
  together say the project is where the field is, under different names.
- **Do not act on §14's staged plan.** Its Stage 1 (grid, gap-model plants,
  Beer–Lambert, bucket water; *move on when self-thinning, succession and
  LAI match*) is behind the engine except for the succession check, whose
  trigger is already recorded. Its Stage 2 (self-organising architecture,
  pipe model, source–sink) is the shipped plant line. Stage 3 (vegetation
  ↔ microclimate) is the founding architecture. Stage 4 (GPU, ML) is
  declined.
- **The one owner call it raises** is §5.5's: whether a season is ever
  wanted, and if so as weather rather than as a calendar. Nothing here
  needs that answered now.

*Freshness: every anchor is at the commit this report was written on;
`dead-ends.md` line numbers drift as the register grows, so grep the
mechanism rather than the line. No behaviour changed in this branch; no
wiki page is affected.*
