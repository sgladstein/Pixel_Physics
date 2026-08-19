# The heritable genome: the slot map

**STATUS: SIGNED OFF — all four §9 calls made by the owner, 2026-08-18:
Map A; re-key; penetration in with its cost; seed strategy deferred with
its plumbing.** The §5 map as amended by those calls is FINAL and slots
are positional forever: the slot index selects which stored draw a trait
reads. It gets copied into `PLAN.md`'s settled decisions when the edit
lands (not before — `PLAN.md` is a contested file and holds no unlanded
promises). An implementation draft is parked on this branch —
`Reports/plant-genome-implementation-handoff.md` is the entry point for
whoever finishes it.

Audited against the committed tip of `plant-substrate-v2` (`16dcdc4`,
"Support from the anchors outward"). The water economy was **in flight,
uncommitted, in `.claude/worktrees/plant-v2`** while this was written
(snapshot taken 2026-08-18 00:48); every reference to it below names its
interface, not line numbers, and the landing session must re-verify those
names against what actually committed. This document was produced in a
separate worktree (`plant-genome` branch) — the audit is of committed code
plus that one snapshot.

**Update, 07:40 the same morning:** the water economy committed as
`8c19439` while this proposal was being written. The delta-check §8
requires is done and recorded in §10 — every hook held, no genome code
moved, the §4.3 seam is confirmed live in the committed code, and the only
remaining gate on the edit is §9.

---

## 1. What exists, verified by reading

Eleven loci. Every claim below was checked in the file, not carried over
from the brief — and two of the brief's claims needed correcting (§1c, §7).

### 1a. Six continuous slots — two dead everywhere, by measurement

`OrganismState::genotype_draws: [f32; GENOTYPE_TRAITS]` (organism.rs:892,
`GENOTYPE_TRAITS = 6` at :973). Drawn once at germination, uniform in
[-1, 1], one independent rng stream per `(world seed, position, slot)`
(plant.rs:361–368). Consumed as `1 + draw × variance` via
`plant::genotype()` (plant.rs:321), variance read live from the species'
`Behavior::Grow::genotype_variance`. Inherited offspring copy the parent's
draws with ±`MUTATION_SIGMA` (0.08) uniform jitter, clamped to [-1, 1]
(plant.rs:529–532).

| slot | trait | consumer | tree / shrub / conifer variance | status |
|---|---|---|---|---|
| 0 | shoot `branch_chance` | plant.rs:1074 | 0.5 / 0.5 / 0.4 | live, r on size ≈ +2.5x quintile spread |
| 1 | `upward_weight` | plant.rs:1076 | **0.0 / 0.0 / 0.0** | **dead — measured flat at ±40%, 1,024 genomes** |
| 2 | `plastochron` | plant.rs:1077 | 0.4 / 0.4 / 0.3 | live, strongest trait (3.9x) |
| 3 | `turgor_per_cell` | plant.rs:1095 | 0.18 / 0.18 / 0.15 | live, r(height) −0.72…−0.76, replicates |
| 4 | `pipe_ratio` | plant.rs:3044 (`SecondaryThicken`, variance borrowed from the shoot `Grow` — "one plant, one genotype", plant.rs:2851) | 0.7 / 0.7 / 0.5 | live, correctly signed on stem width |
| 5 | `light_weight` | plant.rs:1075 | **0.0 / 0.0 / 0.0** | **dead — measured flat at ±50%, and structurally: the per-column sky cast leaves almost no lateral light gradient to steer by (tree.ron's own note: "worth fixing at the field, not by widening this")** |

Slots 1 and 5 are zero in **all three** Grow species, so they are dead
everywhere, not just in `tree`. Moss uses `Divide`, carries no `Grow`, and
is untouched by anything in this document.

### 1b. Five discrete loci — one with no mechanical consequence

`OrganismState::alleles: [u8; DISCRETE_LOCI]` (organism.rs:966,
`DISCRETE_LOCI = 5` at :995). Inherited whole; each locus re-rolls
uniformly over its allele range with `DISCRETE_MUTATION_CHANCE = 0.03`
per seed (plant.rs:542–547). `LOCUS_ALLELES = [6, 3, 3, 2, 2]`.

| slot | locus | alleles | consumer | consequence |
|---|---|---|---|---|
| 0 | `LOCUS_FOLIAGE` | 6 | foliage palette band (plant.rs:552) | **colour only — the one locus with no mechanical consequence** |
| 1 | `LOCUS_BRANCH_ANGLE` | 3 — ×[0.4, 1.0, 1.6] | departure angle (plant.rs:1058) | ~28° / 70° / 112° on tree's trunk value |
| 2 | `LOCUS_INTERNODE` | 3 — ×[0.4, 1.0, 2.0] | straightness budget (plant.rs:1060) | twiggy ↔ long straight runs |
| 3 | `LOCUS_SYMPODIAL` | 2 | fork relabelling, order > 0 (plant.rs:1067) | axis-replacing forks |
| 4 | `LOCUS_TROPISM` | 2 | tier reference, order > 0 (plant.rs:1068) | plagiotropic tiers |
|   | *(not a locus)* `bark_band` | — | inherited outright, never mutates (plant.rs:553) | **a heritable-but-frozen channel — evolution cannot move bark at all** |

The brief listed the discrete loci in a different order than their slots;
the table above is the slot order and is the one that matters.

**A latent mutation bias worth fixing while we are here:** `LOCUS_FOLIAGE`
mutates uniformly over 6 alleles, but every current species declares
`foliage_bands.count = 2` and the consumer clamps
(`.min(count − 1)`, plant.rs:552) — so alleles 1–5 all render as the top
band. A mutation at this locus lands on the top band five times as often
as the bottom one. Nothing measured has depended on it yet; the re-key in
§5 removes it by construction.

### 1c. Roots — the gap is worse than the brief said

The brief said a root's `branch_chance` reads slot 0, the same draw as the
shoot's (plant.rs:1074 — verified, exactly as stated). But it undersold
the situation: `genotype_variance` is `#[serde(default)]`
(organism.rs:444), **no species' `RootTip` `Grow` declares one, and the
default is all zeroes — so every root multiplier is exactly 1.0. Roots do
not have shared genetic variation today; they have none at all.** Two
independent fixes are therefore needed, and the slot map must provide for
both: root parameters need *their own slots* (so root and shoot can
diverge within one individual), and the root `Grow` needs a variance
vector that is non-zero at those slots.

### 1d. What "measured genomes" actually means here

No genome is persisted anywhere. Draws regenerate deterministically from
`(world seed, germination coordinate)`; alleles exist only at runtime.
"Renumbering rewrites every genome ever measured" is about the *measurement
record* — the study tables whose columns are named by slot. The current
record is: two 1,024-genome studies (labels correct for their runs, and the
strongest evidence that slots 1/5 are dead), and one megastudy that is
already void (stale binary, 3 populations wearing 24 logs —
`genetic-variability-study.md` §1) and must be re-run regardless of what is
decided here.

---

## 2. The three tests, as applied

1. **Does it change what a cell does, or only what a cell is labelled?**
   Sympody, tropism and acrotony fired perfectly and moved nothing anyone
   could see, because they relabel cells and the silhouette was set by
   texture and colour. Every locus below names the *behavioural* site it
   reaches — a rate, a cost, a threshold, a strength — and, where the
   claim is visual, which pixels change.
2. **Is there a measurable outcome it trades against?** Named per locus,
   with the probe facility that measures it. A locus with no trade-off is
   worse than a free parameter: selection saturates it in one direction
   and it stops being variation at all.
3. **Continuous or discrete?** The rule this proposal follows: **strategy
   axes are discrete, quantitative gains are continuous.** Discrete loci
   are what make clusters (jump mutation at 0.03/seed lets a morph persist;
   drift would smear it — organism.rs:1027), and they are also what makes
   a *readable* colour: a band is a discrete object, so a band-selecting
   locus satisfies `plant-appearance-design.md` §7 by construction, where
   any continuous hue derivation converges on mud.

---

## 3. The water-economy interface this design hooks into

From the 00:48 snapshot of the in-flight work (names to re-verify at
landing):

- `OrganismState::water` — whole-plant stock; capacity =
  `WATER_SCALE × root_cells` (`water_capacity_of`). Root mass *is* the
  drought buffer.
- `OrganismState::water_status` — the stomatal term, fraction of this
  tick's transpirational demand met; multiplies every photosynthetic
  credit and every leaf's contribution to intercepted light (Liebig).
- `Behavior::Photosynthesize { transpiration, drought_death, .. }` —
  demand per foliage cell scaled by its light; graded shedding on thirst
  cubed.
- `Behavior::Absorb { rate }` — credits **water**, runs on `RootTip` and
  on mature root tissue, so uptake scales with root mass in contact with
  damp soil.
- `allocate_to_frontier` — functional balance: a root tip's share weight
  is `ROOT_BIAS_AT_FULL_WATER (0.5) + (1 − water_status)`.
- `break_root_tips` — root re-initiation from mature root tissue when
  `water_status < 0.95`, candidate scored by adjacent available water
  (hydrotropism as a placement decision).

This is what makes root and leaf loci selectable: before it, a plant with
no roots ran no deficit and there was nothing for these traits to buy.

---

## 4. The candidates, each against the three tests

### 4.1 Wood density — IN, discrete (3 alleles)

- **What a cell does:** two live consumers today, one visual.
  *Strength:* `organism_structural_tick` computes
  `effective_span = max_cantilever_reach − supported_load/LOAD_PER_SPAN_UNIT`
  (structural.rs:409–425); the density multiplier scales
  `max_cantilever_reach` per individual — dense wood holds a longer branch
  under more piled load before it snaps to deadwood. *Cost:* the shoot
  `Grow.cost` (and the thickening price — site to confirm in `thicken()`
  at landing) scales with density — dense wood grows slower per unit
  carbon. *Pixels:* `bark_band` derives from the allele (§6).
- **Trade:** growth rate against breakage. Cheap wood outgrows dense wood
  and loses more of itself to load and (goal 2, later) to storms and to
  the root-plate-vs-stem comparison, which needs exactly this quantity on
  the stem side.
- **Measurement:** paired — stand cells/height at 30k frames (probe,
  exists) against the loaded-branch span
  (`a_loaded_branch_breaks_at_a_shorter_span_than_a_bare_one`'s quantity,
  exists as a test scene). Deadwood conversion counts are visible in the
  probe's census.
- **Why discrete:** it is the pioneer-vs-dense *strategy* axis and the
  bark readout wants bands. Alleles ×[0.75, 1.0, 1.35] on both reach and
  cost, first pass, swept at landing. Fresh stands draw the allele
  positionally on stream 65 (the stream that used to pick the bark band
  directly — §6), so a first generation is a mixed stand and bark keeps
  its day-one variety.

### 4.2 Leaf construction economics — IN, by re-keying `LOCUS_FOLIAGE` (2 alleles)

- **What a cell does:** allele scales `Photosynthesize.rate` up with
  `transpiration` up (dark, expensive, acquisitive) or both down (pale,
  cheap, conservative). Consumers: the credit sites (`organism_tick` and
  `organism_upkeep` Photosynthesize arms) and the demand sum in
  `organism_upkeep` — all live in the water snapshot.
- **Trade:** carbon income against water demand, Liebig-mediated: dark
  leaves win where light is the binding constraint (shade, wet), pale
  leaves win where water is (bright, dry). Fully exercisable the day the
  water economy lands, and *only* then — this locus is why the genome
  session was sequenced after it.
- **Measurement:** paired wet/dry stands (deep soil bed vs thin soil over
  stone — the exact pair `roots-and-breakage-handoff.md` prescribes),
  reading the probe's water-balance block (stomatal term, uptake, demand —
  in the water session's probe additions) and foliage share (exists).
- **Why this locus and not a new one:** the allele *already* selects the
  foliage band (plant.rs:552) — the consumer exists and is the colour.
  Re-keying means the same allele now also carries the rate/transpiration
  pair, so the colour stops being a free gene and becomes the visible face
  of a real one — which is `plant-appearance-design.md` §7's end state,
  "a sick plant should look sick because it *is* sick", applied to
  strategy: a dark tree is dark because its leaves are expensive. **This
  is not a renumbering.** Allele values keep their colour meaning exactly;
  they gain a mechanical one. `LOCUS_ALLELES[0]` drops 6 → 2 to match the
  band count every species actually declares, which also removes the 5:1
  mutation bias of §1b. Fresh stands keep their positional band draw
  (plant.rs:413), so a first generation is a visible mixed-strategy stand
  from frame one.
- **Cost stated plainly:** pure-cosmetic heritable foliage colour ceases
  to exist. Within-band tonal variation (4 steps per band) and the future
  live-state modulation stay; *which band you are* becomes physiology. If
  the owner wants colour to stay a free gene, the alternative is a new
  `LOCUS_LEAF_ECONOMY` at discrete slot 5 with its own band mapping — one
  more locus, and foliage tone then answers to two masters, which §7
  argues against.

### 4.3 Stomatal closure point — IN, continuous — with one seam flagged

- **What a cell does:** a new species scalar (`stomatal_reserve`, the
  stock fraction below which stomata begin closing; 0 = today's
  behaviour), multiplied by this slot's draw. Consumed once per organism
  tick where demand settles against stock (`organism_upkeep`'s balance
  block in the snapshot): effective demand = demand × openness, openness
  ramping 0→1 as `stock/capacity` rises through the threshold.
- **Trade:** drought endurance against growth rate. A conservative
  individual hoards its buffer — earns less in a mild shortage, still has
  leaves after a long one. A profligate one earns flat-out until the tank
  is dry.
- **The seam (which quantity does the shedding rule read):** in the
  snapshot, `drought_death` keys on `(1 − water_status)³`. If closure
  lowers `water_status`, a conservative plant would *shed harder while
  protecting its stock* — the lever would select against itself and the
  trade inverts. Leaves die of desiccation, not of prudence.
  **Settled design (supersedes an earlier draft of this bullet that keyed
  shedding on raw `1 − stock/capacity`):** shedding keys on a new
  `water_desiccation` = the shortfall *with stomata fully open*
  (`1 − min(stock, demand)/demand`), while earning keys on the actual,
  closure-limited `water_status`. This is strictly better than the raw
  stock key because it changes **nothing** until a species opts in: with
  `stomatal_reserve = 0`, openness is 1, the two draws are the same
  number, and desiccation ≡ `1 − status` — the water session's
  `drought_death: 0.003` tuning is untouched by construction, where the
  raw-stock key would have re-opened it for every species (a plant living
  hand-to-mouth at full status would suddenly shed). The identity is the
  guard test.
- **Measurement:** the same wet/dry pair as 4.2, plus a drought-onset
  scene (soil drying out): foliage retention curve and final cells. The
  probe's water block and per-plant leaf counts already carry both.
- **Why continuous:** a threshold point on a 0–1 reserve is a
  quantitative gain with a smooth response; there is no morph boundary in
  it, and its visible face (pallor under stress) belongs to the live-state
  appearance work, not to a band.

### 4.4 Root branch chance — IN, continuous, own slot

- **What a cell does:** the root `Grow` arm reads its `branch_chance`
  multiplier from a root slot instead of slot 0 (the plant.rs:1074 edit —
  the dispatch already knows the cell type). Root `Grow` gets a variance
  vector non-zero at the root slots (§1c).
- **Trade:** uptake surface and water capacity (`water_capacity_of` reads
  `root_cells`) against carbon spent underground (root `Grow.cost` 0.25 a
  cell) that the canopy never sees — bounded above by the existing
  allometric cap (`MAX_ROOT_FRACTION`, plant.rs:1187).
- **Measurement:** `root_cells`/`shoot_cells` (probe prints them; the root
  readout landed in `ed28d16`), uptake/demand from the water block, and
  paired growth under wet vs dry.
- **Why continuous:** same character as shoot branching — a rate, already
  proven to spread outcomes as a continuous draw.

### 4.5 Root tropism gain — IN, continuous, own slot

- **What a cell does:** the root's `upward_weight` multiplier moves to a
  root slot. For a `RootTip` the reference this weights is already
  *moisture-or-down* — `moisture_pull` when the gradient is strong enough,
  `(0, 1)` otherwise (plant.rs:1266–1270) — so one slot genuinely is
  "how hard this root follows water and gravity versus wandering", the
  hydrotropic gain the night handoff queued, on a consumer that already
  exists.
- **Trade:** depth against spread. A high-gain root drives to the water
  table and the deep buffer; a low-gain one wanders laterally near the
  surface and catches rain first. Neither wins everywhere — that is the
  trade.
- **Measurement:** the root depth histogram (`ed28d16`'s readout — the one
  that overturned "roots hug the surface") paired across a
  surface-watered scene and a deep-table scene.
- **Why continuous:** a steering weight; smooth, no morph boundary.

### 4.6 Root:shoot allocation bias — IN, continuous, own slot

- **What a cell does:** multiplies the root weight in
  `allocate_to_frontier`'s functional balance (the
  `ROOT_BIAS_AT_FULL_WATER + (1 − status)` term in the snapshot) — a
  constitutively root-heavy individual versus a canopy-gambler, on top of
  the plastic response the economy already provides.
- **Trade:** the plainest one in plant ecology — canopy now against water
  security later. Interacts with, and is bounded by, `MAX_ROOT_FRACTION`.
- **Measurement:** root:shoot ratio distribution across a stand (probe),
  and survival/foliage in the drought-onset scene against growth in the
  wet one.
- **Why continuous:** an investment fraction; the interesting population
  outcome (bimodal strategies) should be allowed to *emerge* from
  selection on a smooth axis rather than be imposed by allele design —
  and if it never does, that is a finding about the landscape, visible in
  the megastudy histograms.

### 4.7 Root penetration force — IN, continuous — conditional on its cost

- **What a cell does:** multiplies `penetration_force` (root `Grow`,
  tree at 1.2) in `growable()` (plant.rs:81–94) against
  `Material::penetration_resistance` — soil 0.8, sand 1.4, gravel 3.5.
  The axis is qualitative and legible: today's tree roots soil only; a
  high draw opens sand (dune-rooting morphs), a very high one gravel.
- **The condition:** as a bare threshold this fails test 2 — more
  penetration costs nothing, selection saturates it high, and it stops
  varying. The companion (part of the same edit): root `Grow.cost` scales
  with the entered material's resistance (soil ~1×, sand ~1.75×, gravel
  ~4×, i.e. `resistance / soil's 0.8`). Then hard ground is *expensive*
  ground, and penetration is a real strategy with a real bill. If the
  owner prefers not to touch the root cost model yet, this slot should be
  **deferred, not allocated dead** — appending it later costs nothing.
- **Calibration bound:** tree's 1.2 sits 1.5× above soil's 0.8; variance
  must keep the low tail above 0.8/1.2 ≈ 0.67 of the species value
  (width ≤ ~0.3) unless "a draw that cannot root even in soil" is wanted
  as a lethal. It is not proposed as one.
- **Measurement:** root cell counts by substrate under a mixed
  soil/sand/gravel bank scene (a filmstrip scene to add at landing), and
  the depth histogram.

### 4.8 Seed strategy — DEFERRED by the owner's call; its plumbing lands now

**Decision (2026-08-18, after debate):** no slot. Three things overturned
the doc's original "in": the engine has **no dispersal axis** (a seed's
physics are identical whatever the allele, so the succession story
shrinks to recruitment density near parents); the benefit side —
*does endowment actually move establishment?* — is the **only empirically
unknown trade among the candidates**, and assigning a permanent slot to
an unmeasured trade is exactly how slots 1 and 5 died the first time; and
it lands on the least-finished subsystem (seeds never decay — the
immortal seed bank and its u16 id ceiling — so a 2× fecundity allele
doubles a known leak). Appending a discrete locus later renumbers
nothing, so deferral is structurally cheap.

**What lands now:** the provisioning plumbing —
`OrganismState::endowment`, set from `Reproduce.seed_cost` at `set_seed`
and written to the seedling as its starting carbon at germination (the
cost used to vanish at the deduction site). That makes the response
curve measurable at species level: sweep `seed_cost`, rebuilt per point,
establishment fraction per seed as the outcome. If the curve has real
slope, `LOCUS_SEED_STRATEGY` appends in a later widening with allele
values set from measurement — ideally alongside seed decay, which fixes
the leak and hands the locus its second lever.

The original case, kept for that later session:

- **What a cell does:** allele scales `Reproduce.seed_cost` up while
  scaling `seed_chance` down (few big seeds ↔ many small ones; middle =
  authored). Consumer live at plant.rs:3015–3026. **Companion needed for
  the benefit side:** today `seed_cost` is deducted from the parent
  (plant.rs:3024) and *nothing reaches the child* — a big seed buys
  nothing. The mechanism: the paid cost is written onto the seed cell as
  carbon at `set_seed`, and germination keeps it as the seedling's
  starting stake — the seed *is* its provisions. A staked seedling clears
  the first-`Grow`-check-reads-zero hazard that tree.ron's own tuning
  history documents, so provisioning connects directly to establishment.
- **Trade:** offspring count against per-offspring establishment odds —
  measurable *now* because establishment failure is a real, measured
  phenomenon (5/16 tree, 4/16 conifer leafless in the megastudy).
- **Measurement:** seeds standing + established count per genotype
  (probe prints both), at stand density where establishment actually
  fails.
- **Fire/decay coupling, deferred honestly:** `seed.ron` has
  `flammability: 0.5`, but flammability is per-material, not per-cell —
  a per-allele fire resistance has no cheap hook. Seed *decay* does not
  exist at all (the seed-bank leak is an open item). Neither blocks the
  size↔number axis; both can join this locus later without renumbering.
- **Why discrete:** r/K seeding is a strategy, and three alleles give the
  authored middle plus both extremes as persistent morphs.

### 4.9 Rejected / not candidates

- **Widening sympody/tropism** — already loci; they relabel cells, and
  their lesson is the reason for test 1.
- **A juvenility locus** — the establishment fix landed as species data
  (`juvenile_size/plastochron/branch`, plant.rs:1085–1090) after the
  megastudy named it; genetic variation on top of it is speculative and
  has no measured outcome it would trade against yet.
- **Seed decay resistance** — no decay mechanism to hook (§4.8).
- **Anything reading temperature** — the channel oscillates and nothing
  divides it out yet (`CLAUDE.md`); a locus on it would alias the clock.

---

## 5. The two maps — the decision that needs the owner

Both maps deliver the same fifteen live loci. They differ only in what
happens to the two dead continuous slots, and the in-code doctrine is on
opposite sides from the audit brief:

- `organism.rs:422` (the variance field's own doc): *"Retire a dead trait
  by setting its width to 0.0, not by removing its slot."* Slot 1 is named
  there as exactly that case.
- The brief: *"This is a re-mapping question, not an append."*

**Map A — re-purpose (recommended).** Slots 1 and 5 take root meanings;
`GENOTYPE_TRAITS` 6 → 9.

The argument: a slot that never expressed rewrites no measured phenotype.
Slots 1 and 5 were flat lines in every study that touched them — their
draws never moved an outcome, so giving those draws a new meaning
invalidates no result; the only casualty is two column labels in logs that
are already superseded (the megastudy must re-run regardless, §1d). What
re-purposing buys, permanently: no dead columns in every genotype table,
every variance vector, and every future study of a genome that is about to
nearly double in width. The doctrine comment gets amended in the same
edit: *a slot dead by measurement in every species may be re-purposed
once, with the measurement record re-baselined; a live slot, never.*
`light_weight` and `upward_weight` become pure species data (their
genotype multiplications were already no-ops via zero variance; the lines
go). If the light field ever gains lateral structure, phototropic gain
returns as a *new appended slot* — cheap, and honest about it being a new
measurement.

**Map B — append (doctrine-conservative).** Slots 1 and 5 stay dead at
width 0.0; root traits take 6–10; `GENOTYPE_TRAITS` 6 → 11. Nothing ever
changes meaning; every future table carries two inert columns and the
genome is two slots wider for the same information.

### Map A, in full (the recommendation)

**Continuous — `GENOTYPE_TRAITS = 9`.** Draw semantics unchanged
(uniform [-1, 1], ±0.08/generation). Variance first-pass values below are
starting points for the landing sweeps, not tuned claims.

| slot | trait | consumer | tree variance (first pass) |
|---|---|---|---|
| 0 | shoot branch chance | plant.rs:1074 (unchanged) | 0.5 |
| 1 | **root branch chance** | root `Grow` arm, re-pointed | 0.5 |
| 2 | shoot plastochron | unchanged | 0.4 |
| 3 | turgor per cell | unchanged | 0.18 |
| 4 | pipe ratio | unchanged | 0.7 |
| 5 | **root tropism gain** | root `upward_weight` site, re-pointed | 0.4 |
| 6 | **root:shoot allocation bias** | `allocate_to_frontier` root weight | 0.4 |
| 7 | **stomatal closure point** | demand-settle in `organism_upkeep` | 0.5 |
| 8 | **root penetration force** | `growable()` via root `Grow` | 0.25 (bounded — §4.7) |

Slots 0, 2, 3, 4 keep their meanings exactly, so the one solid replicated
result (turgor, r(height) ≈ −0.75) and the other live regressions remain
comparable across the re-map. Variance for slots 6–8 lives on the shoot
`Grow` vector per the `pipe_variance` precedent ("one plant, one
genotype"); the root `Grow` vector carries 1 and 5 and is finally
non-zero.

**Discrete — `DISCRETE_LOCI = 6`** (seed strategy deferred, §4.8).

| slot | locus | alleles | consequence |
|---|---|---|---|
| 0 | **`LOCUS_LEAF_ECONOMY`** (re-keyed `LOCUS_FOLIAGE`) | 2 — dark/acquisitive, pale/conservative | `Photosynthesize.rate` ×[1.2, 0.85], `transpiration` ×[1.5, 0.7] (first pass); foliage band = allele, exactly the existing consumer |
| 1 | `LOCUS_BRANCH_ANGLE` | 3 | unchanged |
| 2 | `LOCUS_INTERNODE` | 3 | unchanged |
| 3 | `LOCUS_SYMPODIAL` | 2 | unchanged |
| 4 | `LOCUS_TROPISM` | 2 | unchanged |
| 5 | **`LOCUS_WOOD_DENSITY`** | 3 — ×[0.75, 1.0, 1.35] | `max_cantilever_reach` ×, wood carbon cost ×; bark band derives (§6) |

`LOCUS_ALLELES` becomes `[2, 3, 3, 2, 2, 3]`. Fresh stands start: economy
**and density** from positional draws (streams 64/65 — density takes over
the stream that used to pick the bark band directly, so day-one stands
keep their bark variety and both strategy axes are mixed from frame one);
angle and internode at allele 1 = species as authored. This supersedes an
earlier line here that had density starting at allele 1 — the positional
start is what §6 always said, and it is what preserves the day-one look.

**The count, and the answer to the brief's question:** fifteen loci —
nine continuous, six discrete — every one with a live behavioural
consumer and a named measurement. Under Map B it is the same fifteen live
loci wearing seventeen slots.

---

## 6. Appearance: the trait-derived half, under §7's constraint

The constraint holds by construction: **the trait picks the band; nothing
continuous touches hue.**

- Foliage: band = leaf-economy allele via the consumer that already exists
  (plant.rs:552). Dark band = acquisitive, pale = conservative, per
  species range (every species declares two bands; the mapping is the
  identity it already is).
- Bark: `bark_band` stops being a free-inherited frozen field and derives
  from the density allele, clamped into the species' declared bark range
  exactly as foliage clamps today. First generation draws density
  positionally (as bark does today), so day-one stands keep their bark
  variety. The exact allele→band aesthetics (which band reads "dense" per
  species) is a landing-session sheet judgment, not a design commitment.
- Within-band stays reserved for state: the four tonal steps per band, and
  the *live-state* modulation (drought pallor, bark darkening with age) —
  which, contrary to the brief, **has not landed anywhere** (verified: no
  such code at `16dcdc4`, none in the water snapshot). When it lands it
  modulates within the band the genome chose, and the two halves compose
  without either repainting the other.

Pixels test, stated: this section moves colour bytes on every wood and
foliage cell as a function of two mechanical alleles — the exact channel
(texture and colour) that the sympody phase proved is what silhouettes are
made of.

---

## 7. Corrections to the brief, for the record

- "The live-state half already landed" — it has not (§6). The genome
  session found no pallor/darkening code in the committed tree or the
  water snapshot.
- Roots do not merely *share* the shoot's draws; they have zero variation,
  because the root `Grow`'s variance vector defaults to zeroes (§1c).
- The discrete loci were listed out of slot order; `LOCUS_FOLIAGE` is
  slot 0 (§1b).
- The water economy was uncommitted and in flight during this audit, not
  landed; this proposal's §3/§4 hooks are against its snapshot interface.

---

## 8. The one edit, and how it is verified

**Sequencing:** the edit lands only after (a) the owner signs off on a map
and (b) the water economy commits — four loci hook its interface. **(b) is
satisfied: `8c19439`, delta-checked in §10.** The `drought_death` re-key of
§4.3 is coordinated with that code, not around it.

Touches, in one commit: `organism.rs` (constants, allele tables, locus
renames, the doctrine amendment), `plant.rs` (root slot re-pointing in the
`Grow` arm, allocation/stomatal/penetration/provisioning consumers, band
derivations), three species `.ron` (variance vectors — root and shoot —
and the new species scalars), `examples/plant_probe.rs` (genotype table
columns renamed to the new map, plus an allele-frequency line per discrete
locus so morph dynamics are visible in logs), tests (each new locus gets
its paired-comparison harness), `PLAN.md` (the final map, and that slots
are positional forever), and `wiki/plants.md` (written by the water commit
`8c19439`; heritable morphs and colour-as-readout are player-visible, so
the page updates in the same change, per convention).

**Verification, per the brief plus this repo's own rules:**

1. `cargo test --lib` and `cargo clippy --all-targets -- -D warnings`.
2. **A paired comparison per locus** — each named in §4; a locus that
   moves nothing does not keep its slot, and an exactly-zero delta means
   *first* suspect the condition is degenerate (the granular-divisor
   lesson), not that the lever is dead.
3. **Look first:** one filmstrip sheet per discrete locus at fixed allele
   (all-dark vs all-pale, all-dense vs all-light), judged by eye before
   any regression is trusted — and the band counters printed beside the
   sheets, because a recolour is invisible at contact-sheet zoom.
4. **Rebuild before the megastudy** (`cargo build --release --examples`),
   confirm the probe's first line echoes `worldseed=`, then re-run
   `scripts/megastudy.sh` — the old regressions are against the old map
   and the old (void) study; the re-run is the first real 8-seed study.
   Gate cross-species claims on the shape descriptors, per
   `genetic-variability-study.md` §6.
5. The re-mapped slots' regressions get fresh columns; nothing is compared
   across the re-map except slots 0/2/3/4, whose meanings did not move.

---

### 8a. Landing measurements — partial, and one row stops the rest

Measured in the `plant-genome` worktree at the completion session, all on
one machine in one session (`CLAUDE.md`: never against a remembered
number). **This section is incomplete on purpose** — the first §4 row
turned up a finding that belongs to the owner before the remaining rows
are worth running, and it is written up at the bottom.

**State of the edit.** The parked draft's one known break (a missing
`use super::organism;` in `structural.rs`) was the only compile error in
it; nothing else behind it needed touching. `cargo test --lib` is green at
485 tests, `cargo clippy --all-targets -- -D warnings` is clean. Six of
the handoff's seven guard tests are in; the seventh is the one held below.
`organism_upkeep`'s settle arithmetic was extracted to a pure
`plant::settle_water` with no behaviour change, which is what let §4.3's
seam be asserted directly.

**Stand-level sanity bar, set fresh this session** (the handoff asks for
this before anything else is believed). Standard probe, 8 trees / 30,000
frames / `worldseed=1`, release binary rebuilt first and its first line
confirmed echoing `worldseed=1`:

```text
                       draft as parked    after the review fixes below
established                 8 of 8                   8 of 8
organism cells              22,309                   27,048
  leaf                       6,940                    8,455
  mature                    14,593                   17,662
root cells per plant   median 158 max 472      median 318 max 826
seeds set                       65                       64
```

A breeding stand in the gross range the water session left either way, so
the re-map did not cost growth.

**The right-hand column is the review's doing, and it is the measurement
that says those findings were real.** An independent review over the diff
(repo convention before a significant commit) found that the density
multiplier had reached `Grow`'s own spending gate and *none* of the three
sites that budget in units of that same cost — so for the four plants of
eight that draw the dense allele, `break_root_tips` staked every
re-initiated root tip below its own first `Grow` check, `break_buds`
floored every flush below its first step, and the income-over-price tip
cap was denominated in the wrong currency in both directions at once.
Fixing all three (`organism::wood_density`, one accessor, applied at every
site) **doubled median root mass, 158 to 318, and added 21% to the
stand**. This is `CLAUDE.md`'s "when a fix changes what a number *means*,
re-deriving the constants that read it is part of the fix" arriving on
schedule.

Two smaller findings from the same review landed with it: the
no-finite-span sentinel in `structural::organism_structural_tick` was
being tested against the *scaled* span, so `leaf.ron`'s `u16::MAX` opt-out
(65535 x 0.75 = 49151, not the sentinel) silently stopped applying to
every pioneer-allele plant's foliage — latent, since 49151 still dwarfs
any real support distance, but the leaf span was latent too right up until
checks started firing and took a stand from 31,731 cells to 7,171. And the
founding leaf-economy allele is now clamped to the *locus'* range rather
than the palette's, a no-op at every shipped species and a guard against a
future 4-band one founding alleles mutation could never produce. One
finding was **not** acted on: `wiki/plants.md` needs the three new
player-visible mechanics, and it is a §2-held file until the water
session's review is relayed.

**Both new readouts are live on day one**, which is what §6 promised and
what a sheet cannot show at contact-sheet zoom:

```text
palette bands in use: wood band 0  8,328 cells / band 1  5,600 cells
                      leaf band 2  2,077 cells / band 3  4,863 cells
alleles: economy 2/6/0  angle 0/8/0  internode 0/8/0
         sympody 8/0/0  tropism 8/0/0  density 3/1/4
```

Density splits 3/1/4 across the stand from the stream-65 positional draw
and bark follows it into two bands — the day-one bark variety §5 said the
stream reuse would preserve is preserved, and it now means something.

**Slots 5 and 6, paired.** Density pinned to the authored allele
throughout, so the coherence fixes above are no-ops here and these numbers
are identical before and after them (re-run to check, not assumed). One
tree, genome frozen with `inherited`, one
slot moved to ±1.0, 12,000 frames, in a 61x30 walled soil bed chosen so
the *scene* cannot be what bounds the root system
(`plant::tests::print_root_branch_slot_pairing`, `#[ignore]`d, holds the
reproduction):

| slot | draw −1 | draw 0 | draw +1 |
|---|---|---|---|
| 5 root tropism gain | 444 / 2,453 | 352 / 2,440 | 362 / 2,353 |
| 6 allocation bias | 305 / 2,621 | 352 / 2,440 | 440 / 2,478 |

(root cells / shoot cells.) **Slot 6 holds §4.6's claim outright** — root
count monotone in the draw and the root:shoot ratio with it, 0.116 /
0.144 / 0.178, while the shoot moves the other way or not at all. Slot 5
moves the stand but not monotonically in *count*, which is not a
contradiction: §4.5's claim is about the depth histogram, and a low-gain
root wandering laterally laying more shallow cells is that claim. Its
proper readout (the `ed28d16` histogram, paired across a
surface-watered and a deep-table scene) is **not yet run**.

**Slot 1 (root branch chance) is exactly zero at every draw, and the
condition is degenerate — this is the row that stops the rest.**

The draw reads back correctly at the consumer (multiplier 0.5 / 1.0 / 1.5
against the root vector's 0.5 width), and the outcome is *bit-identical*:
352 root cells and 2,440 shoot cells at −1.0, 0.0 and +1.0 alike, with
identical trajectories from frame 2,000 on. Per `CLAUDE.md`, an
exactly-zero delta means suspecting the condition before the lever, so
the branch gate was instrumented (counters since removed — diagnostics do
not belong in `organism_tick`). Over one 12,000-frame run:

```text
root growth steps reaching the branch gate   351
  of those, holding `resource >= cost`         2
  of those, the 0.04 branch roll firing        0
carbon a root tip holds at the gate: mean 0.053, max 1.72, against cost 0.25
```

A root tip finishes its primary step holding about **a fifth** of what a
second step costs, so `branch_chance` — whatever the genome multiplies it
by — sits behind a gate the root economy clears twice in twelve thousand
frames, and at `branch_chance: 0.04` those two chances then produced no
branch at all. It is **starvation, not dead code**: the same counters
with slot 6 driven to +1.0 (the lever that funds roots) read 53
affordances and one firing.

So the §4.4 measurement as designed cannot come out non-zero at the
authored economy, and the handoff's guard test
`root_and_shoot_branching_read_different_slots` has a false premise —
`root_cells` cannot order with slot 1's draw when slot 1's consumer never
executes. **It is not written**, and the reproduction above is kept in
its place. What to do about it is a §9-shaped call and is not decided
here: §8's own rule ("a locus that moves nothing does not keep its slot")
points one way, and the fact that the block is a *cost gate on a fully
plumbed consumer* rather than a dead trait points another — this is the
same shape as the granular-capacity divisor, where the lever was fine and
the condition it was tested through was degenerate.

Worth stating plainly for whoever picks it up: this gate is **not** the
re-map's doing. Before it, the root's `branch_chance` was multiplied by
slot 0 through the RootTip's own all-zero variance vector, i.e. by exactly
1.0, against the same `resource >= cost` gate and the same 0.25 cost. The
re-map gave the trait a slot; it did not change what stands between the
trait and the world.

**Not yet measured** — every remaining §4 row, held with the above:
slot 5's depth histogram, slot 8 penetration (needs the sand-bank scene
§4.7 calls for), `stomatal_reserve` 0.2 → 0.0 on a drying scene against a
wet one, the leaf-economy wet/dry crossover, the wood-density
growth-against-load pair, the `seed_cost` endowment response curve, and
the two pinned filmstrip sheets with their band counters.

---

## 9. The owner's four calls — ANSWERED, 2026-08-18

1. **Map A** (re-purpose, 9 continuous). As recommended.
2. **Re-key `LOCUS_FOLIAGE` to leaf economics** — colour becomes a
   readout; the cosmetic-only colour gene ceases to exist. As
   recommended.
3. **Penetration force in**, with the resistance-scaled root cost that
   makes it selectable. As recommended.
4. **Seed strategy DEFERRED, plumbing in** — the recommendation was
   reversed in debate on three grounds recorded in §4.8: no dispersal
   axis, an unmeasured benefit curve (dead-slot risk), and the
   half-built seed-fate subsystem. The endowment plumbing lands so the
   curve becomes measurable; the locus appends later, from measurement.

The §5 tables as amended are the final map, and go into `PLAN.md` marked
positional forever when the edit lands.

---

## 10. Addendum: delta-check against the committed water economy

`plant-substrate-v2` moved to `8c19439` ("Water becomes a real currency,
and the epiphyte guard is deleted") a few hours after the audit, so §8's
delta-check is already discharged. Verified in the committed tree, not
the snapshot:

| §3 interface name | committed at |
|---|---|
| `WATER_SCALE`; `water_capacity_of` ∝ `root_cells` | organism.rs:1408; plant.rs:424 |
| `water_status` gating every credit | plant.rs:3331 (`rate × light × status`) |
| `ROOT_BIAS_AT_FULL_WATER` (0.5); functional-balance root weight | plant.rs:2423; :3069 |
| `break_root_tips`; `ROOT_REINITIATION_STATUS` (0.95) | plant.rs:2707; :2430 |
| `Photosynthesize { transpiration, drought_death }` | all three Grow species (e.g. conifer.ron:86, :151), `Absorb(rate: 1.5)` on mature root tissue |

The commit contains **no genome changes** — no `LOCUS_*`, `genotype_*` or
`DISCRETE_*` edits — so nothing in §5's map moves. Three findings stand
exactly as written: the §4.3 seam is live as committed (drought shedding
keys `thirst = 1 − water_status`, plant.rs:3324, so the stomatal-closure
locus still requires the status→stock-fraction re-key before it can
exist); there is still no pallor/darkening code anywhere (§7's correction
holds); and the water commit wrote `wiki/plants.md`, closing the gap §8
originally named.

**The only remaining gate on the edit is §9.** Line references in §1–§4
remain pinned to `16dcdc4`, as the header states; this section's are
against `8c19439`.
