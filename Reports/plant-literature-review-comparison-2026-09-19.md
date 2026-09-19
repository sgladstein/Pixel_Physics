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

**Updated the same day — §7 re-evaluates the rejections, on the owner's
instruction not to trust them.** Two of the seven were reasoned by
analogy and never measured, one was measured by a test that could not
answer, one is overturned in its stated generality, and the largest
finding is about the economy rather than any one mechanism: the girth
term is 70% of every plant's bill and its level was set without the
quantity it most controls, so a full-size tree is seedless by
construction. Read §7's table first if you are here for that.

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
7. The rejections re-evaluated — same day, on the owner's instruction
   (7.1 maintenance · 7.2 reproduction from surplus · 7.3 root turnover ·
   7.4 nutrient return · 7.5 dormancy · 7.6 attractors · 7.7 temperature
   and season · 7.8 what changed and what was written back)

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

---

## 7. The rejections re-evaluated — same day, on the owner's instruction

> *"Re-evaluate anything that seems like a good idea and is supported by
> research but was rejected as a dead end or just declined. Don't trust
> past results. Past agents may have made mistakes and these are all very
> complex systems so it may have rejected it based on a bad test or a
> failure coming from an interconnected system."* — the owner, 2026-09-19

Seven items from §1 and §3 met that description: research-backed, and
either recorded as a dead end or declined here. Two auditors, each in its
own worktree, each briefed with the records and the method rules and told
to be as willing to write *stands* as *overturned*, took them. Every
number below is paired across 12 world seeds at fixed parallelism
(`RAYON_NUM_THREADS` pinned), from a binary rebuilt after every edit with
an identity control (the switches unset reproduce the shipped run
byte-for-byte), with a positive control showing the readout can move.
Timings are not quoted anywhere; the box was shared. I re-checked every
file:line the auditors leaned on in this checkout and corrected the two
claims that did not survive it (§7.4, §7.7).

| § | rejected as | verdict | the one line |
|---|---|---|---|
| 7.1 | maintenance ∝ biomass "bounds nothing" | **re-tested: confirmed**, recorded reason wrong | never measured before; flat-at-equal-bill loses a fifth of the seed 12/12 — but the girth term is 70% of the bill and holds the median plant in deficit |
| 7.2 | `seed_launch` on a tree | **stands**; the design under it is the finding | surplus funds growth, seed and buds from one number, so a full-size tree is seedless by construction; a 10% floor on income to seed **2.18x seed on 12/12** |
| 7.3 | fine-root turnover "worse at every rate" | **overturned in its stated generality** | the sweep carried a collapsed bed in every arm; on the shipped bed, not worse at any rate, +11% income 9/12 |
| 7.4 | litter returning nutrients "a documented double dead end" | **never measured; rejection unsafe** | both cited entries are moisture; a nutrient return was never built, and the store it would repay into cannot pump |
| 7.5 | reversible dormancy (tip retirement is "meristem senescence") | **unsafe test**, plus a confirmed measurement | the named acceptance test exercises a disjoint code path; a cut *suppresses* bud break 12/12; stumps cannot resprout, structurally |
| 7.6 | space-colonisation attractors, replaced by crowding | **never measured**; substitute live but not doing its job | zeroing crown shyness makes the fused run *smaller* (median 0.964, larger on 5/12) |
| 7.7 | temperature and season | season **stands, strengthened**; temperature **condition changed** | a tree lives two app-days, so no year fits; but the field writes a temperature nothing in `plant.rs` reads |

### 7.1 Maintenance respiration proportional to biomass

**What the record measured: nothing.** The dead end points at a
"Deferred, with reasons" bullet in a plan whose own framing sentence says
every single-lever change in the session that produced it was *wrong,
circular or scene-dependent*. No scene, seeds, frames or number exist to
be stale. Its premise — *income linear in leaf count* — has been false
since income became intercepted light (`plant.rs:9867`), and is still
repeated in three live places (`README` §The economy re-derived,
`plant.rs` `MAINTENANCE_PER_CELL` doc, the superlinearity guard). The
property the exponent defended, a tree that stops growing, was **retired
by owner decision** on 2026-08-22 (`open-bugs-handoff.md` §V). And the
flat:superlinear split was never swept — the re-derivation swept both
constants as one multiplier. Measured now: the girth term is **70% of the
stand's bill**.

**The biology sides with the girth term, not the flat one.** Maintenance
respiration scales with *living* mass — sapwood, not bole (Ryan 1990;
Ryan et al. 1995) — and under the pipe model sapwood volume goes as
foliage × height, which is superlinear in foliage. A flat per-cell charge
would price dead heartwood at the living rate. Where the engine departs
from the literature is the **monotone** `q_peak`: real sapwood converts to
heartwood as a crown recedes, so the respiring fraction falls; the doc's
"ratchet that eventually kills an adult" is the *respiration hypothesis*
for age-related decline, which the field has rejected on measurement
(Ryan, Binkley & Fownes 1992: stem respiration 61 → 79 g C m⁻² yr⁻¹ from a
40- to a 245-year stand while wood production fell 210 → 46; Tang et al.
2014).

**Re-test, equal bill.** `PIXEL_PHYSICS_MAINT_PER_NODE=0` with
`PIXEL_PHYSICS_MAINT_PER_CELL` raised to 5.1e-4 so the pooled stand bill
matches the control (194.9 against 187.1 over 12 seeds; 5.01e-4 against
5.08e-4 per cell). `plant_probe trees=8 frames=20000`, 12 paired seeds:

| | shipped | flat, equal bill | direction |
|---|---|---|---|
| seeds set, stand | 837 | 685 (0.80) | **12/12 down** |
| germinations | 138 | 115.5 (0.88) | 11/12 down |
| median plant's surplus | −0.006 | −0.024 | **12/12 more negative** |
| stand cells | 32,467 | 30,819 (0.96) | 9/12 down |
| largest plant | 5,582 | 5,646 (1.02) | 8/12 up |

At equal stand cost a flat rate taxes the median plant and rebates the
largest, and the stand loses a fifth of its seed. That is "impoverished
rather than shaped", measured for the first time. **Verdict: confirmed.**

**The positive control is the substantive finding.** `MAINT_PER_NODE=0`
with the flat term unchanged (bill strictly cheaper, 3 seeds): seeds set
611 / 896 / 850 → **1,187 / 1,381 / 1,550**; median plant surplus −0.005 /
−0.004 / −0.009 → **+0.40 / −0.003 / +0.97**. The girth term is what puts
the median established plant into permanent deficit, and the maintenance
*level* was calibrated on median bill-to-income (1.27–1.45 targeted,
1.24–1.65 measured today) with recruitment nowhere in the calibration —
which is the quantity §P2 later recorded as having moved the wrong way
(inherited-genome establishments 1 → 0, 2 → 0).

### 7.2 Reproduction from a surplus that is zero at the ceiling

**The `seed_launch` dead end stands.** Its measured fall in seeds borne,
0.71 / 0.56 / 0.53 / 0.56 over four seeds, matches the launch price
1 / 1.87 = 0.535: the output fell by exactly the price, which is what a
budget-limited account does. Four seeds is under the house minimum, but
the direction was 4/4, the arm was an env switch on one binary, and the
entry itself names the condition below.

**The condition is real and structural.** `plant.rs:9977` is one
expression, `surplus = (income − maintenance).max(0).min(stock)`, and it
funds the growth pool, the seed budget (`reproductive_share`) and bud
break (`supportable`). *Cannot afford another cell* and *has no seed
budget* are the same number, so a tree at its ceiling is seedless by
construction. Measured: the median established plant's surplus is
**negative on 12/12 seeds**; median seeds set per plant is 0 on several
seeds while the best plant sets 16–23% of the stand's total. This is the
opposite of the literature — fecundity rises with size (Greene & Johnson
1994), reproductive investment rises with net production and is ~1/8 of
it at peak (Hirayama 2004; Moore et al. 2023), large trees are the largest
seed producers — and `wiki/plants.md` said size buys offspring, which was
true when seed was set per mature cell and has not been since the fence
came out. **The wiki line is corrected in this branch.**

**Re-test: allocate from production rather than from the residual.**
`PIXEL_PHYSICS_REPRO_FLOOR=0.10` sets `reproductive_share = max(surplus ×
allocation, min(income × 0.10, stock))`, taken off the growth pool:

| | shipped | floor 0.10 | direction |
|---|---|---|---|
| seeds set, stand | 837 | 1,811 (**2.18**) | **12/12 up** |
| germinations | 138 | 178.5 (1.24) | 11/12 up |
| reproductive budget | 0.128 | 0.252 (2.5) | 11/12 up |
| established plants | 18.5 | 20 (1.06) | 8/12 up |
| inherited-genome establishments | 10.5 | 12 (1.12) | 8/12 up |
| stand cells | 32,467 | 34,690 (1.04) | 7/12 up |

Seed and germination move decisively; establishment and selection
throughput move up at 8/12, suggestive and not settled; **no stand-size
cost at this horizon.** Positive control at floor 1.0: seeds ~8x, budget
pinned at its cap, and germinations **304 / 304 / 305 across three
different worlds** — recruitment saturates at ~305 whatever the seed
supply, the demand-side fence `plant-equilibrium-costs` §13b found for
`seed_chance`. So the floor buys seed and the bank, not recruits, until
the fence moves.

**Next.** This is a decision, not a repair: `reproductive_allocation` is
authored 0.10–0.30 across eight species *against a residual*, so changing
the base changes what every one of those numbers means
(`why-changes-cost-so-much-2026-08-27.md`), and the draw is notional in
both arms (credited without debiting donor cells). 6–10 h, starting with a
blind A/B card, because "more seedlings, slightly smaller trees" is a
judge-by-eye trade. **And the cross-cutting fact:** `income − maintenance`
is the only currency, the girth term on a monotone `q_peak` is 70% of the
bill, and it sets the growth ceiling, zeroes the seed budget at that
ceiling, and is why a per-seed price like `seed_launch` halves output. The
constant most responsible for recruitment was never calibrated against it.

### 7.3 Fine-root turnover

**What the record measured.** `plant-soil-nutrient-plan-2026-09-05.md`
§2b-v: `soil=6`, `PIXEL_PHYSICS_ROOT_GATE=local`, 12 paired seeds, rates
0.002 / 0.01 / 0.05 against 0; income 0.189 → 0.104 / 0.070 / 0.123. **The
sweep trap is present and the entry half-names it**: the rider constant
across all four arms was the gate on a 6-row bed, which alone cost 88% of
income (1.588 → 0.189), and the root zone read **0.000 in all four arms**,
so there was nothing to rotate *to*. The isolating control was never run,
and the entry's own per-seed counts are 7/12, 3/12, 5/12.

**The condition has changed, measured.** The gate is on by default, the
tank is capped, roots are 4.7x, the nutrient recovers by time. On the
shipped 96-row bed the control arm's root-zone water reads 0.36–0.75,
median 0.50. `PIXEL_PHYSICS_ROOT_TURNOVER` already existed. Means per
established plant, 20,000 frames, 12 paired seeds:

| | turnover 0 | 0.002 | 0.05 |
|---|---|---|---|
| income | 0.864 | 0.931 (**1.107**, 9/12 up) | 0.952 (1.11, 7/12 up) |
| uptake | 8.05 | 9.54 (1.14, 9/12 up) | 9.41 (1.11, 7/12 up) |
| root soil contact | 71.7% | 74.9% (1.03, 9/12 up) | 91.3% (1.26, **12/12 up**) |
| root cells | 335 | 298 (neutral) | 297 (0.89, 7/12 down) |
| stand cells | 32,139 | 31,677 (0.97, 8/12 down) | 30,815 (0.93, 8/12 down) |
| established | 15.5 | 16.5 (neutral) | 14.5 (0.82, 8/12 down) |

Positive control at rate 0.5 (3 seeds): contact 69 / 84 / 69% → **89 / 93
/ 95%**, root cells −16 to −38%, stand −10 to −17% — the switch culls
exactly the roots that touch nothing, which is what the term was written
for. **Verdict: overturned in its stated generality.** "Worse at every
rate" is a property of the 6-row bed; on the bed the entry names, income
is not worse at any rate and the gentlest rate moves income, uptake and
contact together on the same 9/12 seeds. Not a win yet — 9/12 is
suggestive, and the high rate costs 7% of mass and establishment on 8/12 —
but the recorded rejection does not carry. Next: 24 seeds at 0.002 plus
0.005 and 0.01 to find the knee, and a card with the root overlay rather
than the stand, since roots are invisible on a contact sheet. 3–4 h.

### 7.4 Litter returning nutrients to the soil

**Never measured.** The "documented double dead end" the nutrient code
cites (`plant.rs` recovery-period doc) is two entries about **moisture**:
copying a neighbour's soil moisture into new soil, reverted on reasoning
because the donor keeps its own; and deriving it from the field's ambient
humidity, the one that was measured (1,718 → 2,652 cells and climbing —
one guard test's cell count, no seeds, no paired arm). `decay.rs` contains
zero occurrences of *nutrient*; `git log -S` over every plausible
identifier for a return writer returns nothing; the store's API is
draw-only (`World::draw_soil_nutrient`, `Chunk::draw_nutrient`), and the
draw's return value is discarded at `plant.rs:975`. Soil nutrient shipped
2026-09-06; both moisture reverts predate it. **The rejection was carried
across by analogy.**

**And the pump is not expressible on this store.** The moisture runaway
had two mechanisms: duplication from a donor that keeps its stock, and
reading an unconserved channel. The nutrient store is a *deficit* buffer —
`available = initial − buf`, draws add to `buf` — so a conservative return
is `buf −= returned`, floored at zero, and total nutrient is bounded by
Σ`initial` by construction. The plant holds no nutrient stock at all, and
both consumers (`nutrient_income_multiplier` ≤ 1, `nutrient_construction_
multiplier` ≥ 1) read **1.0 at full soil** — the best state is *no
penalty*, never a bonus. So the whole dynamic range of any nutrient return
on standing biomass is the shipped ablation's 0.843 → 1.0, an arm that
already exists. **What a return would buy is soil memory, not biomass**:
ground that remembers what grew on it, which is the succession loop
`plant-equilibrium-costs` §10e names as the thing carbon and water
structurally cannot produce, and which `soil-accumulation` §4A deferred
only because *a fertility channel that is not water* did not exist. It
shipped ten days later with a reader and a depleting writer and no return
writer.

One figure the auditor quoted is stale and is corrected here: the
where-a-dead-plant-goes ledger's *33% locked in deadwood for ever* predates
`deadwood.ron`'s `decays_into: "litter"`; deadwood rots now. The 9%-to-soil
figure is about cells and does not bear on a ledger-style return anyway.

**Next.** `OrganismState` gains a `nutrient_drawn` debit, captured from the
discarded return at `plant.rs:975`, repaid per cell at `rot_remains` into
the soil under the rotting cell through a `Chunk::return_nutrient` that
mirrors `draw_nutrient`; behind `PIXEL_PHYSICS_NUTRIENT_RETURN`, default
off. ~80–100 lines, 1.5–2 h. Then measure the right quantity: the spatial
variance of `soil_nutrient_fraction` and a seedling's advantage on
previously occupied ground against fresh ground, 12 paired seeds, two
horizons; positive control a freely crediting switch (the deficit must pin
at zero everywhere), negative control return off. ~1 h of runs. The doc
comment at the citation is corrected in this branch so the next reader is
not sent away by it.

### 7.5 Reversible bud dormancy against permanent tip retirement

**The acceptance test as named cannot answer the question.** A stale tip
is rewritten to `MatureBody` (`plant.rs:6559–6566`); `break_buds`
(`:9464`) selects only `CellType::DormantBud` (`:9531`), which the `Node`
fate creates in every species file. The two paths are disjoint, so *cut a
limb and watch neighbouring buds restart* passes on a plant whose retired
tips are untouched. **Permanent tip retirement is live**, and the dead
end's own replacement — reversible dormancy — was built for buds that were
never tips. A closely related measurement was taken on 2026-09-12
(`plants:124`, `PIXEL_PHYSICS_RESPROUT`): blind A/B, owner verdict *looks
identical*, and `max_active_tips` binds first.

**Run as written anyway**, `plant_severance`, 4 trees, 16,000 frames, cut
at 8,000, both arms in one process, 12 seeds — post-cut bud flushes, cut /
control: .864 .945 .474 .693 .941 .433 .776 .788 .930 .986 .979 .528,
**median 0.864, fewer on 12/12**. A cut *suppresses* bud break, because
`supportable = ⌊(noon_income − maintenance) / step_cost⌋` (`:9589–9591`)
falls with the foliage. Sensitivity: `RESPROUT=150` moves the cut arm 5,823
→ 6,085 flushes and the median to 0.907. A **stump never resprouts, and
that is structural**: no foliage, `intercepted = 0`, `supportable = 0`,
and the stump arm's tracked plants go 3,195 → 0 shoot cells while
mid-crown cuts recover (seed 3: 1,157 → 2,386). Two traps for the next
reader: `buds_flushed` is a **world-wide** counter, and in the stump arm it
read 32–118 while the tracked plants had zero shoot — it was counting
seedlings elsewhere; and `RESPROUT_DEFICIT_FLOOR = 1500` was set from a
six-seed control maximum that the 12-seed control still crosses.

**Next.** Not more frontier — that was measured. The smallest real change
is a path from `MatureBody` back to `DormantBud` on a live stem, applied
only to *staleness* retirement and not to starvation. Named risk: species
files gate `StructuralAnchor` and `SecondaryThicken` on `MatureBody`, so it
reallocates which cells thicken and `pipe_ratio` needs re-deriving. ~2 h
to build, 4–6 h with the re-derivation. Cheapest first: re-run the
owner's blind A/B of `PIXEL_PHYSICS_RESPROUT` **on stumps**, the case it
was never judged on and the one where a plant has rootstock carbon and no
way to spend it. ~20 min of compute.

### 7.6 Space-colonisation attractors, replaced by the crowding channel

**Never measured.** The removal is a design-review exchange
(`tree-rewrite-design.md` §2, `design-philosophy.md` §2) whose strongest
claim is stated as *very plausibly*; no A/B, seeds or frames. The
replacement is live: both inert-mechanism bugs are fixed
(`candidate_crowding` reads the candidate's occupied neighbours, with
regression tests; canopy density is an `f32` on the sidecar, not four bits
of `aux`), and `tree.ron` carries a four-point sweep of `crowding_weight`
(6 → run 76, 12 → 64, 20 → 52, 30 → 61) **with no zero arm and no seed
replication**.

**The missing zero arm, run.** Shoot `crowding_weight` 30.0 against 0.0
(the root's deliberate 0.0 untouched — the `sed` trap), two fixed binaries
confirmed different by `cmp`, `plant_probe trees=8 frames=30000`, 12
seeds. Thickest fused run, off / on: .941 .714 .622 .556 1.141 1.371 .910
.964 1.237 .707 1.125 1.029 — **median 0.964, larger without crowding on
only 5/12**; median cells 0.935. The channel is live (cells move 0.76x–
1.10x), but **zeroing it does not produce the fusion it exists to
prevent**: the direction is a coin flip and the median points the wrong
way. Confound, stated: the stand is ~6.5% smaller with crowding off and a
smaller stand has a smaller run, so run is not separated from mass at
n=12. At the shipped weight the fused run is 56–99 cells on a bed whose
spacing is 57, i.e. crowns span the whole spacing — the standing defect
`plant-appearance-design.md` §5a measures at 41–95.

**Next: test exclusion, not attractors.** Attractors differ from crowding
in one property — a consumed attractor cannot be entered twice, while two
tips read the same density and both may enter. Make the existing channel
exclusive within one tick (deposit at the *chosen* candidate before the
next tip scores, inside the loop at `plant.rs:5347–5366`), no new state, ~2
h, read on the same paired harness with mass held or normalised. If the
fused run does not move, fusion is set by something neither mechanism
touches, which is the conclusion `plant-appearance-design.md` §2.1 reached
for the three architectural levers, and that would settle it.

### 7.7 Temperature and season

**Season stands, and the arithmetic makes it stronger than the recorded
reason.** `DAY_NIGHT_PERIOD_FRAMES` is 3,600 and the app ships
`day_minutes: 8` (`assets/clock.ron:18`), so one app day is 28,800 frames;
`tree.ron` `life_half_life` 60,000 and maturity ~30,000 make **a tree that
matures in one app day and has a two-day life half-life** (8.3 and 16.7
days in a harness at baseline). A year in which a tree saw four cycles
would be 15,000 frames — *shorter than the day*. "Flicker" was the wrong
word; the real objection is a scale collision: on the world's own calendar
a tree is an annual, and no year with a recognisable number of days fits
inside its life. The eight-minute day landed 2026-08-23, a week *after*
the rejection, and moved the condition against seasons. (I have not
re-derived §5.5's revisit condition; it survives, and it is further off.)

**Temperature is a different question and its condition changed.** The
rejection bundled the two. `plant.rs` reads temperature **zero times**
while `field::noon_equivalent_temperature` (`field.rs:2067–2103`) exists,
de-oscillates exactly, and already has consumers in `creature.rs` and
`evaporation.rs`; `weather.rs` writes cell temperature and ships
`Pin::Frost` and `Pin::Blizzard`; the held world's spell list already
reads *call frost to kill back what is winning*. **A writer with no plant
reader** is the inverse of this repo's usual missing-end failure and the
cheap end to close. Three things it would buy, none needing a calendar:
frost as an emergent mortality and disturbance source; a Q10 on
decomposition, which is a lever on the litter sink; and the possibility
of `tree` and `conifer` differing *functionally* — verified from the
species files, they do not today: same `life_half_life`, same
`Photosynthesize` rates and shedding pressures, same `Absorb`, same
`Reproduce` bar `seed_maturity` 600 against 700; they differ in
`leaf_cluster` (10 against 6), which §5a calls a pure appearance knob.
**The two woody species are one organism in two costumes.**

**Cost, separated.** Q10 on *respiration* is not scoped: maintenance
enters `surplus` and sets `supportable`, so `INCOME_PER_NODE`,
`max_active_tips`, `seed_maturity` and `RESPROUT_DEFICIT_FLOOR` are all
calibrated against it — the reallocation trap exactly. Q10 on
*decomposition* touches only `litter.ron`'s yield and `remains_half_life`.
Frost damage reallocates nothing (additive mortality), and is tick- and
event-scheduled, not sweep-scale; the one registry trap is a new
`DEATH_CAUSE_LIST` row enrolling in every census that enumerates it.
**Cheapest first experiment: graded frost damage** — a fraction of foliage
per tick below a `noon_equivalent_temperature` threshold, driven by
`Pin::Frost` rather than a calendar, behind `PIXEL_PHYSICS_FROST`, ~3 h,
plus paired seeds to confirm it is graded and a review card because it is
judged by eye. The deciduous/evergreen trade is step two and should wait
until the two species differ in something.

### 7.8 What this changes, and what was written back

- **§5's ranking moves.** The largest thing in this document is no longer
  a pattern readout; it is §7.2 — the economy has one currency, the girth
  term is 70% of it, and the level of that term was set without the
  quantity it most controls. The pattern readouts (§5.1) are now the
  cheap way to *see* that: a self-thinning slope and a seeds-per-plant
  against size curve would have shown a seedless canopy years ago in the
  world's own time.
- **Two rejections were reasoned by analogy and never measured** (§7.4,
  §7.6); one was measured by a test that could not answer (§7.5); one was
  never measured and confirms on re-test for a reason other than the one
  recorded (§7.1); one is overturned in its generality (§7.3); one stands
  for a stronger reason than recorded (§7.7 season); one stands as
  written and is one switch away from its own condition (§7.2).
- **Written back in this branch**, as `CLAUDE.md` asks for a met
  condition: the five `dead-ends.md` entries (maintenance, `seed_launch`,
  meristem senescence, attractor removal, root turnover) each carry a
  dated re-evaluation line; the nutrient doc comment in `plant.rs` says
  what its "double dead end" actually cites; `wiki/plants.md`'s "size buys
  offspring" is corrected with a freshness note; and the six switches the
  economy audit was taken on land as instruments, default off, cached in
  `OnceLock`s so they cost nothing per cell, with `plant_probe` echoing
  them and printing surplus, reproductive budget, seeds set and root-zone
  water beside its size columns (`instruments.md`).
- **Not done here, deliberately**: none of the priced next steps. Each is
  a decision with a reallocation cost, and three of them (§7.2, §7.5,
  §7.7) want a review card before a line of mechanism.

*Freshness: every anchor is at the commit this report was written on;
`dead-ends.md` line numbers drift as the register grows, so grep the
mechanism rather than the line. §7's switches change no behaviour when
unset (identity control byte-identical). `wiki/plants.md` is corrected, not
changed, by this branch.*
