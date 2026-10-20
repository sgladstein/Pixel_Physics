# The plant trainer — plan

*2026-10-19, **revision 2**, after the independent adversarial review (§12:
1 blocker, 13 major and 7 minor findings, all answered). Step 3 of 5 of the
plant-trainer line: brainstorm
([`plant-training-brainstorm-2026-10-19.md`](plant-training-brainstorm-2026-10-19.md))
→ engine review
([`plant-engine-trainability-review-2026-10-19.md`](plant-engine-trainability-review-2026-10-19.md))
→ **this plan** → adversarial review (§12) → build. The ant trainer's own
design notes, in the repo since PR 725
(`Reports/brain-training/design/general-brain-design-rev3.md` and its rung
notes), are the source for the controls borrowed here.*

**What it is for.** A tool that breeds plant **species files**: herbs first,
then any plant. It aims at three things the owner asked for: plants that
live well in the lab, the current herb looking better, and new plants that
look like nothing in the box today. It is the plant counterpart of the ant
trainer (`src/train/`, 2026-10-17), and it borrows that trainer's controls,
because those are what made its scores honest.

## 1. Owner rulings this plan is built on (2026-10-19)

1. **A trained plant is a species file**, loadable like any other.
2. **The trainer may change shape and look.** Scoring, measuring and
   optimising looks must be thought through, and should lead to better
   looks for current plants and new looks for new ones.
3. **Herb first.**
4. **A plant-only bed may shed overhead** to train faster. Low priority.
5. Sequence: brainstorm, engine review, this plan, an independent
   adversarial review of it, then implementation.

Standing rulings that bind it: *"ship everything on"* for behaviours, but
**a look ships default-off pending the owner's eye** (lab coordinator note).
The review queue is for visual judgements only. A wrong default is reported,
not tuned. **The shipped herb is never changed by the trainer.** A trained
herb is a new species beside it, as the stage-2 ant is a new ant beside the
frozen game ant. It replaces nothing until it beats the shipped plant on the
lab's own bars and the owner's eye.

**Superseded in part, 2026-10-20.** P1 is on `main` (PR #731). The owner
then ruled that colour is a species' own (D1, with limits), reframed D2 as
"plants should develop like real ones: genes and surroundings matter, dice
little", and set the next goal as a diverse bed that thrives over a long run,
not looks. P2–P6 below are replaced by
[`plant-community-plan-2026-10-20.md`](plant-community-plan-2026-10-20.md).

**Two decisions for the owner**, asked now because each changes what the
trainer can reach, though P1 needs neither:

- **D1. May a species carry its own palette** (leaf, bark and flower
  colours), rather than choosing bands from the shared materials? Colour
  and material mix are what the project measured as setting a silhouette
  (`.claude/rules/plants-and-organisms.md`). Today a trainer can only pick
  among 8 shared bands.
- **D2. May a species own its developmental key** (whether its plants grow as
  one coherent form or a mosaic of per-cell dice)? Measured on herb, the key
  moves size heritability from 0.034 to 0.650 (`plant-engine-rethink` §6.8).
  Today it is a world setting, the same for every plant in both games.

## 2. Design in one paragraph

**The genome is the species file itself.** A run loads a parent species
(`herb.ron`). It mutates a declared set of its numbers and fate rules
(**gene groups**; the rest are frozen), under bounds read from where each
number is used. Each child gets its own name, passes a validator, is
registered through the shipped parser, and is grown alone in small **lab
beds**. Those beds are built and stepped by the lab's own code with nothing
else alive. Each child gets a **viability score in steps** (germinate,
establish, flower, set fruit, its seedlings establish) whose every
threshold is a constant of the run, never a gene. The trainer keeps the best
(*train* mode) or the best per look cell (*atlas* mode, quality-diversity).
Beside it every generation run the shipped species, a hold-out, a teeth
check and a selection-only null. Champions are written as species files to
a staging folder with pictures and a census. Moving one into
`assets/species/` is the owner's decision.

## 3. Where the code goes

| path | what | why there |
|---|---|---|
| `src/train_plant/mod.rs` | run loop, settings line, CSV and sheet output | a sibling of `src/train/`, not inside it: that module and `examples/train.rs` change daily on the brain lane |
| `src/train_plant/genome.rs` | the gene table, mutation, validator, species-file round trip | |
| `src/train_plant/arena.rs` | lab beds, layouts, the step loop | through `lab::scene::LabBox` and a shared `lab::tick_world`, **never a private copy** |
| `src/train_plant/score.rs` | viability steps, folding beds, paired differences | |
| `src/train_plant/looks.rs` | look descriptors, phase anchors, vetoes, the atlas grid | |
| `src/lab/mod.rs`, `src/lab/scene.rs` | two small library seams (§5.1) | |
| `examples/plantrain.rs` | the command line | the house pattern for instruments |
| `examples/herb_morphs.rs` | the review's probe (exists) | the quick "what can it reach" look |
| `training/plants/<run>/` | **staging** for champion species files, sheets, CSVs | never `assets/` directly |

## 4. The genome

**4.1 It is the `SpeciesDef`, written directly.** `ParamGenome` holds at most
8 overrides and is not the store. **The mutation operator is new.** It shares
nothing with the in-game one (one address, additive step) except that both
write the same fields. Each gene is an address, a kind, a **use-site range**,
a step rule and a group:

- **Behaviour numbers**: the 45 `ParamId`s per cell type and per branch order
  where tiered (`ByOrder`), written into the `Behavior` the way
  `ParamGenome::apply` does.
- **Species-level numbers**: the life-history half-lives, `rebloom_after`,
  `seed_gut_survival`, `seed_provision_fraction`, `stomatal_reserve`,
  palette band choices, the genotype `variance` arrays.
- **Fate genes**: `after_metamers`; the table's structural moves via
  `FateGenome`'s four operators and `to_table`.

**4.2 Bounds come from where each number is read, not from `clamp_param`.**
`clamp_param` bounds at 4× `param_scale`, and `param_scale` is the largest
value of the parameter over **every registered species**, a registered child
included. A trainer that read it would ratchet its own bounds (review F1).
So the gene table carries each gene's range as the consumer uses it: `stem_stiffness` is clamped to
[0, 1] where it is read, so its gene is [0, 1]. Any shared scale it needs is
**frozen per run from the shipped species only** and printed in the run's
settings line.

**Zero is a mode, not a value, for several genes** (review F2). `internode`
and `branch_angle` at 0 mean "unset, the legacy behaviour"; `juvenile_size` 0
switches the juvenile stage off; `leaf_spread` 0 is a bit-identical early
return. These become a **discrete mode gene** with a rare flip, plus a
continuous gene bounded away from zero while the mode is on. **Steering
weights are sign-locked**: a step may not cross zero, because crossing it
reverses a behaviour rather than nudging it.

**4.3 Mutation.** Each child changes **1–3 genes**. The design doc's prior art
gives 10–20 of ~2,800 weights; a ~120-gene species file wants a handful.
**Every step is arithmetic only, with no `exp`, `ln` or Gaussian draw**, so a
run replays bit for bit on Actions, on Modal and here. That was the ant
rig's own fix (design rev 3, *Learning*).

- A magnitude moves by a factor `1 + σu` for `u ≥ 0` or `1 / (1 + σ|u|)` for
  `u < 0`, `u` uniform on [-1, 1]. That is symmetric in log space to first
  order, and it is a fraction of the **parent's own value**. This avoids the
  `dead-ends.md` trap where a step scaled to the corpus is 175 against herb's
  60 on `seed_maturity`.
- A probability moves additively by `σu`, clamped to [0, 1].
- A count moves by ±1 (±2 at large σ).
- A zero-authored value with no mode meaning is reached by a draw from its
  use-site range.

**An offline trainer may carry a per-gene range table, which the in-game
mutator rightly may not.** `dead-ends.md` warns against hand-set ranges for
in-game heredity, because nothing there scores a child. Here every child is
scored, and the range is the consumer's own clamp, not a guess. Crossover is
off by default.

**4.4 Every child is renamed, validated and round-tripped** (review F4, F5):

- **A unique name per child** (`herb_t<run>_<gen>_<idx>`). `register_ron`
  replaces any species of the same name and checks nothing, so a child
  named "herb" would silently replace the shipped control. The arena asserts
  that the registry grew by one.
- **A validator**, since the parser has none: every named material exists;
  band ranges fit their palettes; the fate table has at most `MAX_FATES` (16)
  rules, and `FateGenome::from_table(def.fates).to_table()` equals
  `def.fates` exactly. `from_table` truncates silently past 16, so a file
  and its plants could otherwise disagree. Inserted fate rules go **after**
  the determinate `after_metamers` rule, never ahead of it, so a single
  insert cannot shadow it.
- **The round trip** (measured, §4.5): the staged file is the exact text that
  was registered and scored. A guard test re-registers a staged child and
  checks its bed is bit-identical to the scored one. It is **watched failing**
  on a deliberate re-serialisation fault before it is trusted.

**4.5 Measured before revision 1** (scratch harness, deleted): the shipped
herb parsed, written by `to_ron` and registered again grows a
**bit-identical bed** to the embedded herb (whole-grid hash after 12,000
frames, seeds 1–3). So does a copy **renamed** `herbcopy`. A species' name
and registry slot do not enter its growth, so every child can carry its own
name without that being a variable.

**4.6 Gene groups**, the plant version of the ant's banned verbs. A run names
which are free and freezes the rest:

| group | genes |
|---|---|
| `shape` | branch chance, tip cap, branch priming, branch angle, internode, plastochron, heading inertia, steering weights, crowding, tropism and sympody per order, acrotony, juvenile phase |
| `leaf` | leaf cluster, leaf spread, photosynthesis rate, shade and drought shedding |
| `flower` | `after_metamers`, organ cluster, ripen rates and costs, rebloom, nectar |
| `height` | turgor source, yield, per cell, taper; stem stiffness; pipe ratio |
| `root` | the root `Grow` block, absorb, penetration |
| `seed` | seed cost, allocation, maturity, launch, gut survival, provisioning, germination thresholds |
| `life` | the half-lives |
| `colour` | palette band choices |
| `fates` | structural fate moves |

**A gene whose consumer is off in the lab bed is frozen**, and the staged
file says so. Example: wind-driven terms, if a sealed box has no wind
(unverified, measured in P1).

## 5. The arena: the lab, from the first milestone

**5.1 Built by the lab's code, from P1** (review F7, F18). `LabBox::build_counted`
sets `plant_size_cadence = true`, `plant_bending = false` and
`growth_slowdown = 2`, where a default world has the opposite. A bed that is
not the lab's would train a different plant. Two library seams, each
defaulting to today and guarded:

- **`LabBox::founder_columns: Option<Vec<i32>>`**: `None` keeps today's
  `spread(founders)`. A single founder today always lands at the compartment
  midpoint, under a lamp whenever the lamp count is odd.
- **`LabBox::species_ron: Option<String>`**, registered **before** planting:
  `PlantScene`'s doc records a silent empty stand from the other order.
- **`lab::tick_world(world, spec, scenario)`** factored out of `Lab::tick`:
  `frame::step`, the scenario timeline, then `rain::tick`. `Lab::tick` and the
  trainer both call it, so the trainer cannot drift from the game. The
  observers that only feed the window (stats, UI, food and dig maps) stay in
  `Lab::tick`. A guard test hashes a bed stepped both ways.

**5.2 Bed size and length are measured, not assumed** (F18). Before any
budget, P1 measures with `planttrace` the shipped herb's **lab cycle**
(germination to its seedlings established) and the per-bed cost at 96
columns. Its length is that cycle with headroom, as a constant of the run.
Every speed setting (ruling 4: pace, soil depth, width, rain cadence) is
priced against the shipped herb's steps and descriptors and adopted only
inside the noise bar. **A cost that vanishes may be work that vanished.**

**5.3 Layouts are stratified by light, not mirrored** (F6). A lab bed is not
mirror-symmetric (soil texture, lamp texture, rain columns, sweep order).
The ant rig mirrors to cancel a left/right habit that plants do not have.
So a layout is **(world seed, founder offset from the nearest lamp ∈
{under, quarter, between})**. A generation uses K layouts covering all three
offsets. The score is the mean, with the minimum printed beside it.
**Before choosing mean or min**, P1 ranks the shipped herb and 16 probe
variants both ways and keeps min only if it changes the ranking towards
plants that hold up on every offset. **Measured (§13): it changes nothing,
so the rank is the mean.** The min is still printed. Training layouts are redrawn each
generation. **Two sealed sets**: a monitoring hold-out read every
generation, and a **final set read once**, at the end (F10).

**5.4 In-bed mutation is off while scoring**, as in the ant rig (the owner's
2026-10-17 ruling there: the only variation is the one the loop makes).
That is `World::fate_mutation_chance` and `World::param_mutation_chance` at
0. **Founder alleles are pinned** for the founder in training beds (F9):
`FOUNDER_VARIANT_CHANCE` gets a world dial, default 0.25 (today), and the
trainer sets 0, so the founder carries its species file's alleles exactly.
The per-birth jitter (`MUTATION_SIGMA`) and gene flips
(`DISCRETE_MUTATION_CHANCE` 0.03) still touch offspring. P1 measures whether
the seedlings step moves when they are forced to 0 in a scratch build, and
gives them dials if so. The lab check (§7.8) runs with the game's own
settings.

**5.5 Stand beds come in at P2** (`bed=stand`): the child sown among shipped
herbs and grass, scored by its lineage's share (`selection_arena`'s design,
lineage attribution). **The outdoor bed** (`PlantScene`) comes later, for
the outdoor game, and only after world seed 2's stunting there is traced
with `planttrace` (F21).

## 6. Scoring

**6.1 Viability, in steps, against constants of the run** (F12). Each step is
worth more than everything below it.

| step | read from (founder by `organism_id`, by a cell census, never world totals) |
|---|---|
| germinated | the founder exists past its seed stage |
| established | the founder reaches `N_est` shoot cells |
| flowered | a founder cell is a `Flower` |
| fruit dropped | fruit attributed to the founder (a per-organism count added to `drop_organ`) |
| seedlings established | founder offspring reaching `N_est` cells, **capped at `K_seed`** |

`N_est`, `K_seed`, the frames per step and the size floor below are
**measured on the shipped herb, written into the run header, and never read
from the child's genes**. The fraction of progress inside the next step is
measured against those constants: shoot cells against `N_est`, metamers
against the *shipped* `after_metamers`, seedlings against `K_seed`.

**A second number the plant cannot trade away**, the ant rig's rule. The
founder's size at its first flower must reach the shipped herb's 25th
percentile, or the flowering step and those above it are halved. Otherwise
the cheapest climb is a dwarf that flowers at once (`after_metamers` 1–3),
which the probe already found in one gene.

**A precocity probe runs before generation 0**: `after_metamers` swept 1–12
through the score. If the score peaks below the shipped 8 with the size
floor on, the floor is wrong, and the run refuses to start.

**6.2 Vetoes**, zeroing a bed's look credit, printed as counts:

- root cells above the soil line (W6);
- a disconnected **mass** of plant cells larger than k cells or x% of the
  plant. Not any cell: open bug **T** strands a cell in about 1 plant in 24,
  and a cell-level veto would select against starving plants (F16).
  Positive control: a constructed floating clump;
- a leafless standing stump (0-z);
- a one-cell whip (taller than K × its width with fewer than M cells);
- a stick figure (wood over 85% of visible cells, leaf under 5%,
  `plant-appearance-design.md` §2.1).

Every threshold is set so every shipped species passes on every monitoring
layout. Each veto is watched firing on a constructed case.

**6.3 Look descriptors, at phase anchors, at fixed light** (F13). Herb is an
annual, so its look is a life-phase oscillator. Every descriptor is taken at
**two anchors**, the frame of peak above-ground cells and the frame of first
flower, and the anchor frames are recorded. Colour is read from the shipped
`Renderer` with the light pinned. Descriptors come from **what is drawn**:
the founder's cells by material, and pixels for colour, never cell labels.

| descriptor | measures |
|---|---|
| height, width, size | magnitude |
| leaf, wood, flower and fruit shares | composition: the one lever that has read |
| hull fill (cells over convex hull area) | open or dense |
| crown profile (width by height decile; top-heaviness) | stalk, vase, ball, mat |
| branch endpoints per 100 cells | twiggy or sparse |
| leaf clumping (mean size of leaf-cell clusters) | speckle or foliage masses |
| stem thickness (mean run width of wood) | stick or trunk |
| left-right asymmetry | lopsided or balanced |
| ink against the background (`creature_look`'s measure) | legibility at play zoom |
| mean colour per material class | colour, at the pinned light |
| within-genome spread across layouts | recognisability |

**Each descriptor is validated twice before use.** A positive control is a
variant built to move it, and it must move it. A **phase control** is the
shipped herb captured ±2,000 frames from its anchor, and the descriptor must
move less than its bar. Reading the same bed twice is a determinism test,
not a validation, and is not counted as one.

**6.4 How looks are optimised: no beauty number** (F9, F14, F17). An
optimiser fed a proxy for beauty finds the proxy's weak point first. That
is the ant trainer's three-generation cheating applied to looks.

- **The heritability gate comes first** (F9, the review's blocker). At small
  steps, herb size is 0.00–0.05 heritable, so choosing among small variants is
  choosing among noise. P1 measures, in the lab bed with founder alleles
  pinned, the share of each descriptor's spread that is between genomes,
  for σ ∈ {0.05, 0.1, 0.2, 0.4, 0.9} (`clone_variance`'s method). **The
  smallest σ at which a descriptor is heritable above a bar is the smallest
  σ any look round may use.** No candidate reaches the owner unless its
  descriptor distance from the shipped herb exceeds **twice the shipped
  herb's own within-genome spread**, across layouts.
- **"The current herb, better" is therefore not a small-σ round.** It is the
  shipped herb's numbers held, plus the levers that move pixels at a
  heritable size: first the **colour** group (bands within today's palettes,
  its pixel effect measured), then the un-walled looks (§8) as they land, then
  `shape`/`leaf`/`flower` at the gated σ.
- **Atlas (quality-diversity, MAP-Elites): the default for new looks.** A grid
  over two or three validated descriptors. Each cell keeps the most viable
  plant that passes every veto and is recognisable. Output is a sheet of
  distinct living plants. The design doc proposes the same method for ant
  colonies, as "a game feature in its own right".
- **Owner rounds go through blind A/B cards** (F14). A card carries one
  `choice`, so a round is **4–6 blind A/B cards** (`review.py ab --blind`).
  Each is a candidate against the shipped herb, one image of three layouts
  at a readable size, with descriptor and viability numbers in `meta`.
  Alternatively it is one gallery card asking for **one** pick. A pick seeds
  the next generation; the owner's verdicts are A/B verdicts, counted as
  such. **Owner time:** about a minute a card, at most one round a day unless
  the owner asks for more.
- **Target mode**, distance to an exemplar the owner names, is used only when
  the owner can say what they want.
- **A learned taste model is deferred and gated**: at least 50 A/B verdicts,
  and it must predict held-out verdicts better than chance by a stated
  margin before it may steer anything.

## 7. Controls, every run

1. **The shipped species**, on the same layouts every generation.
2. **The monitoring hold-out**, every generation (`every=1`, the owner's rule
   for the ant rig): champion, shipped species, and the champion **blind**.
   The blind control is redesigned for the lab (F15). Under a lamp, light is
   nearly up, so zeroing the light weight mostly re-normalises into the up
   weight. It is run with the lamp moved off-axis (`LabBox::move_lamp`), and
   P1 first checks that the shipped herb leans towards it. Otherwise it is
   dropped. **Dropped (P1, §13): the herb's lean does not depend on
   `light_weight`, so a blind herb is the same herb.** It comes back with U7.
3. **The champion is staged by training score**, never by the hold-out, and
   the **final set** is read once at the end (F10).
4. **Teeth, before generation 0**: `lethal` (the shoot's `Grew` child becomes
   a seed) and `nobranch` must score below the shipped species on the
   monitoring hold-out. Otherwise the run refuses to start.
5. **The bar is a paired difference** (F10): per layout, champion minus
   shipped, with a sign test and an interval. The champion is the best of P
   noisy children (about +2 sd by chance at P = 32), so a single-layout sd is
   not the bar.
6. **A selection-only null**: the same run, at equal compute, with random
   selection. A trainer that does not beat it has learned nothing.
7. **Determinism** (byte for byte), **the round trip** (§4.4), and **the
   settings line** in every log (the ant rig's rule: which genome and which
   bed produced a number is never inferred later).
8. **The lab check before promotion**: the champion in the lab's
   `played_bed` (`latecensus`, plant-only, then with the colony) over **12
   seeds**, against the shipped herb, read as an order statistic. **A
   trainer win is not a game win**: the ant's 8× forager kept a colony half
   the game ant's size.

**7.1 Known-answer tasks, as the trainer's positive controls** (F8). Each is
run **beside random search at equal compute**. If random search solves it as
fast, it is not a control.

- **Sabotage and recover**: halve the shipped herb's photosynthesis rate;
  with `leaf` free the trainer must climb back inside the shipped herb's
  noise bar.
- **Height alone**: with only `height` free, starting from herb's
  pre-fix turgor (0.32, `herb.ron`'s own record: 9 of 32 axes flowered), it
  must recover a flowering herb near the hand answer (0.46).
- **Flowering alone**: with only `flower` free, from the same start, it must
  lower `after_metamers`. That is the one-gene route, and it is reported as
  such, not as a pass of the first task.
- **Germination below the authored floor** in a dry lab bed
  (`plant-water-scarcity` §3).

## 8. Un-walling: engine changes that unlock looks

The review's §6: the trainer can arrange today's parts, not change them. The
test for what moves into the species file is the ant design's *physics vs
policy* line, translated: **could a real plant have grown otherwise,
heritably?** Gravity and water transport stay physics. Leaf arrangement,
head shape, root share and the tip cap are policy, and a species should own
them.

Each item becomes **a species-file field whose default reproduces today
bit-identically** (the default branch takes no extra draw, the `leaf_spread`
pattern), guarded by the stand hashes and a before/after test. It also gets
**a price** where it buys anything, since a free lever produces uniformity.
**Before any is built, a hand-edited mock is rendered at play zoom and goes
to the owner as a blind A/B card**, so a lever that moves no pixel the owner
can see is never built (F17).

| # | field | today (hardcoded) | proposed | price |
|---|---|---|---|---|
| U0 | colour bands | shared palettes, 8 bands | the `colour` gene group, no engine change | none |
| U5 | species-owned palette | shared material palettes | **D1** | none (colour is not physics) |
| U3 | `max_root_fraction` | 0.5 for every species, binding on herbs | per species, default 0.5; **first run as the ant's stage-1 "free one rule" experiment**, where freeing the dig company veto alone was worth +35% | roots pay maintenance already |
| U1 | `phyllotaxis` | first leaf a random empty cell behind the tip | `random` (today), `alternate`, `opposite`, `whorled` | none |
| U2 | `leaf_shape` | breadth-first spray; `leaf_spread` blob ↔ line | `spray` (today), `fan`, `tuft`, `blade` | none |
| U4 | `organ_shape` | random blob | `blob` (today), `spike`, `umbel`, `disc` | organ costs already apply |
| U6 | species-owned developmental key | world setting | **D2** | none |
| U7 | `phototropism` | a tip reads light as straight up or nothing (`organism::phototropism_dir`), so no plant can lean towards a lamp beside it: measured, §13 | `up` (today), `gradient`: the banked patch of `plant-phototropism-lateral-2026-08-27.md`, opted into per species | its own: a stem steered sideways reaches seed maturity later. That is what withdrew the patch engine-wide (tree reproduction to zero at authored `light_weight`); per species, with the trainer re-finding `light_weight` under it, it is the re-derivation that report says the fix needs |

Order: U0 inside P3 (no engine change). Then U5 if D1 says yes, U3, and U1,
U2, U4, U7 in the order their mocks win. U7's mock is the patch itself on
one herb beside an off-axis lamp; it needs no hand edit.

## 9. Speed and compute

**Measured so far**: a one-herb outdoor bed of 96 columns for 20,000 frames
takes 2.7 s on one thread here, and twelve mixed variants 9.3 s on four.
**The lab bed is not measured, and its cycle is at least twice as long**
(growth slowed 2×, plus a second generation to establish). P1 measures both
before any budget is stated.

- **Compute**, from the ant trainer's measured notes
  (`Reports/brain-training/cloud-compute/cost-research-2026-10-18.md`):
  GitHub Actions is free with 6-hour jobs, but the "4-core" runner is 2
  physical cores. Modal Functions at 16 cores ran a 100-generation ant job
  in about 21 minutes for about $0.28, under a hard spend cap, from these
  sessions. The plant trainer gets a job type on the same workflow pattern
  once a run fits; **the owner is told before any paid run.**
- **Racing** (score children on 2 layouts, promote the top half to all K) is
  **deferred** (F11). It is adopted only after the rank correlation between
  2-layout and K-layout scores is measured over ≥ 3 runs × ≥ 5 generations
  per σ. The chance that a true top-quarter child is cut must be stated and
  under a bar, and the first-stage layouts are shared across all children
  of a generation.
- **The cheap bed** (ruling 4), §5.2: each setting priced before use.

## 10. Milestones

Each milestone is a PR. Each says its measurement in the commit message and
does not land broken.

**P1: the core, in the lab, with the measurements every later choice rests
on.**
- Library seams (§5.1) with their guards: `founder_columns`, `species_ron`,
  `lab::tick_world`, and the founder-variant dial.
- `train_plant`: genome (gene table with use-site ranges, mode genes, sign
  locks, renaming, the validator, arithmetic mutation), arena (lab bed,
  stratified layouts, two sealed sets), viability steps against run
  constants, the loop with controls 1–7, CSV, a per-generation sheet, staged
  champions. `examples/plantrain.rs`.
- **Measurements, before any training run is believed**: the shipped herb's
  lab cycle and per-bed cost; `N_est`, `K_seed` and the size floor from the
  shipped herb; the precocity probe; heritability against σ, the gate in
  §6.4; mean against min on the probe's variants; the blind control's lean
  test; whether a sealed box has wind.
- **Accepted when**: the round trip and every seam guard pass and were
  watched failing; teeth pass; the paired bar and the selection-only null
  print; the run is deterministic; **the sabotage task recovers and beats
  random search at equal compute**; the champion beside the shipped herb
  goes to the owner as a blind A/B card.

**P2: scale and stands.** Stand beds; the cheap-bed settings priced (ruling
4); racing measured; the plant job type on Actions; the other known-answer
tasks; the lab check (§7.8) run end to end on a P1 champion.

**P3: looks, without engine change.** Descriptors (each with its positive
and phase controls), vetoes (each watched firing, each passed by every
shipped species), the atlas, the colour group (U0), owner rounds by blind
A/B.
- **Accepted when**: an atlas of at least 12 distinct viable herbs, each
  beyond twice the shipped herb's spread, reaches the owner. A plant the
  owner picks is shown again from its descendants, and **still looks like
  itself**: its descriptors stay inside its own spread.

**P4: un-walling**, mock first, one PR each, in the order of §8.

**P5: promotion.** The owner's chosen herbs become species files in
`assets/species/`, with an `include_str!` line in `organism.rs`'s `EMBEDDED`.
The organism and plant lib test modules that sweep the registry are run (the
species sweeps are lib tests; `tests/` holds only `determinism.rs` and
`worldgen.rs`). **The per-`ParamId` corpus scale is printed before and
after**: adding a species with a larger value widens in-game mutation reach
for every lineage (F1). That is inert while `PARAM_MUTATION_CHANCE` ships at
0, but it is a game-wide change the day it does not. Each new species gets
a `README` status line and a `wiki/plants.md` section with a dated freshness
note. The lab check is a precondition.

**P6, conditional: a plant "mind"** (brainstorm rung G), only if P3–P4 show
the parameters are the limit.

## 11. Risks, and what each is answered by

| risk | answer |
|---|---|
| The score is cheated (dwarf early flowering, tiny seeds, a lowered threshold) | steps against run constants; the size floor; the capped seedling step; the precocity probe; vetoes; teeth |
| Owner picks are noise | the heritability gate; candidates beyond twice the shipped herb's spread; picks shown again from descendants |
| The look proxy is optimised instead of the look | no beauty number; atlas plus blind A/B; a learned taste only after it predicts held-out verdicts |
| The trainer drifts from the game | the lab's own builder and a shared `tick_world`; the round trip; the lab check over 12 seeds |
| Bounds ratchet, or a promotion widens every lineage's reach | use-site ranges, the scale frozen per run, printed before and after promotion |
| A child replaces the control | unique names, the registry-size assertion, a guard test |
| Trained alone, fails in a stand | stand beds from P2; the lab check |
| The hold-out becomes training data | staged by training score; a final set read once |
| The lab bed is too slow | measured first; the cheap bed, racing and Actions only after measurement |
| Un-walling changes every plant in both games | defaults bit-identical and guarded; looks ship default-off; mocks first |
| Free levers converge | prices on new fields; atlas rather than single champion |
| The brain lane's files conflict | a separate module and binary; the lab seams are small and guarded |

## 12. Adversarial review (2026-10-19) and responses

An independent reviewer (an Opus agent, briefed with the rulings and the
review's measurements, ~50 tool calls, no edits) read revision 1 against
the code. **Its claims were spot-checked against the code before they were
accepted**: `param_scale` reads every registered species
(`organism.rs`'s `param_scale`); `LabBox::build_counted` sets
`plant_size_cadence`, `plant_bending = false` and `growth_slowdown = 2`;
a single founder lands at `spread(1)`, the midpoint. All three hold.

| # | severity | finding | response |
|---|---|---|---|
| F9 | **BLOCKER** | "The current herb, better" at small σ is selection on noise: herb size heritability is 0.00–0.05 at small steps, and each founder also draws its own alleles (`FOUNDER_VARIANT_CHANCE` 0.25) | **Accepted.** The heritability gate (§6.4), founder alleles pinned in training (§5.4), candidates only beyond twice the shipped spread, and the round redefined: colour and un-walled levers first |
| F1 | major | `clamp_param`'s bound reads the whole registry, children included: a ratchet, and promotion widens every lineage's reach | **Accepted.** Use-site ranges, scale frozen per run (§4.2); printed before and after promotion (P5) |
| F2 | major | Zero is a mode for `internode`, `branch_angle`, `juvenile_size`, `leaf_spread`; additive steps can flip a weight's sign | **Accepted.** Mode genes and sign locks (§4.2) |
| F4 | major | `register_ron` replaces a same-named species and validates nothing | **Accepted.** Unique names, registry-size assertion, the trainer's own validator (§4.4) |
| F5 | major | `from_table` truncates past 16 rules; an insert can shadow the determinate rule | **Accepted.** Cap with refusal, exact table round trip, inserts after it (§4.4) |
| F6 | major | No founder-column field; one founder sits under a lamp; a lab bed is not mirror-symmetric | **Accepted.** `founder_columns` seam; layouts stratified by light offset; mean with min printed, min kept only if measured to help (§5.3) |
| F7 | major | The outdoor bed runs different plant physics from the lab, so P1 would be redone | **Accepted.** The lab from P1 (§5.1); outdoor deferred |
| F8 | major | The known-answer task has a one-gene solution (`after_metamers`) | **Accepted.** Split into height-only and flower-only, sabotage-and-recover as the acceptance task, all beside random search (§7.1) |
| F10 | major | The hold-out read every generation becomes training data; the noise bar should be a paired difference with the winner's curse | **Accepted.** Staged by training score, a final set read once, paired bar, selection-only null (§7) |
| F11 | major | Racing on 2 layouts is near-random at small σ; "checked once" is no check | **Accepted.** Deferred until priced (§9) |
| F12 | major | Progress fractions undefined and gameable; dwarf precocity; seedling counts; world-wide counters | **Accepted.** Run constants, a size floor, a capped top step, per-organism census, the precocity probe (§6.1) |
| F13 | major | An annual's look is a life-phase oscillator; colour depends on capture light | **Accepted.** Phase anchors, pinned light, phase controls (§6.3) |
| F14 | major | A card carries one choice; 64 sub-images do not read on a phone | **Accepted.** Blind A/B cards, one image of three layouts; owner time stated (§6.4) |
| F16 | major | A cell-level connectivity veto collides with open bug T | **Accepted.** Mass-based veto, bug T cited, positive control (§6.2) |
| F17 | major | Colour is the measured look ceiling and U5 was deferred; U1/U4 may move no pixel | **Accepted in part.** D1 asked now; colour bands first (U0); mocks before U1/U2/U4. **Not accepted** that placement levers are hopeless: the probe's distinct silhouettes came from size, branching and composition moving together, which the earlier failures (narrow allele steps on trees) never tried. The mocks settle it |
| F18 | major | The step loop omitted the scenario timeline: a private copy; the lab cycle and cost are unmeasured | **Accepted.** `lab::tick_world` shared with `Lab::tick`; measured before budgets (§5.1, §5.2) |
| F3 | minor | The mutation operator is new, not the in-game one | **Accepted** (§4.1) |
| F15 | minor | The blind control is uninformative under a vertical lamp | **Accepted.** Lamp off-axis, lean test first (§7.2) |
| F19 | minor | Additive steps on the corpus scale dwarf herb's own values (`dead-ends.md`) | **Accepted.** Relative steps on the parent's own value (§4.3) |
| F20 | minor | The registry sweeps are lib tests, not `tests/*.rs` | **Accepted** (P5) |
| F21 | minor | The outdoor seed-2 stunting is untraced | **Accepted.** Traced before any outdoor use (§5.5) |

**What the review could not settle**, and P1 measures: whether a sealed box
has wind; the lab herb's cycle and cost; whether U1–U4 can default
bit-identically (plausible by the `leaf_spread` pattern, checked when each is
built).

**Also folded in from PR 725's notes** (not review findings):

- arithmetic-only mutation, for replay across machines;
- the physics-vs-policy test for un-walling;
- the root share as a "free one rule" experiment;
- the measured compute options;
- "every log keeps its settings line".

## 13. Build log (P1)

*Appended as P1 lands; measurements, not plans.*

**2026-10-19, commit `3c7ad5e9`: the core.** Library seams (`LabBox::founder_at`,
`LabBox::species_ron`, `lab::tick_world`, `World::founder_variant_chance`), all
defaulting to today: `lab::` 299 tests and `tests/determinism.rs` 4 pass.
`src/train_plant/` (genome, arena, score, run loop) and `examples/plantrain.rs`.
Two corrections to §5.4 found while building: the per-birth genotype jitter
already has a world dial (`World::mutation_sigma`), so only gene flips
(`DISCRETE_MUTATION_CHANCE`) lack one; and the lab's batch runner
(`lab::batch`) steps the world without `rain::tick`, so a batch run of a box
with the mister on never rains. Recorded, not changed.

**The round-trip guard, watched failing.** A renamed herb written by `to_ron`
and registered again grows a whole-bed hash identical to the shipped herb's.
Its first version compared the founder's census and stayed green under a 1%
change to leaf photosynthesis: blind. Rewritten to hash the bed, it stayed
green under that fault and under one more leaf per node (8 → 9), and went red
under plastochron 5 → 4. **A parameter can be flat in a young bed**: carbon
caps a node's leaf spray below 8, so 9 changes nothing, and a sweep of
`leaf_cluster` 24 grew like the shipped 8.

**The shipped herb in the lab** (`mode=calibrate`, 24 layouts × 40,000
frames, 71 s on four threads, about 3 s a bed): germinated 24/24; flowered
17/24, by light position 7/8 under a lamp, 6/8 a quarter out, 4/8 between
lamps; first flower p50 at frame 6,750, at 58–103 shoot cells; 0–23
seedlings established. Run constants set from it with headroom:
`size_floor` 55 (under the smallest that flowered), `k_seed` 24 (over the
largest). At the first guess, `k_seed` 4, 14 of 24 shipped beds sat on the
cap.

**The precocity probe** (§6.1; `mode=sweep gene=after_metamers`, 12
layouts): the score peaks at the shipped 8 (4.139), with 7 close (4.061).
Earlier flowering scores lower: 1 → 1.117, 3 → 2.652, 5 → 3.413. **Passes:
the score does not pay a dwarf for flowering early.** Two side findings:
`after_metamers` 8 reproduced the shipped herb's row exactly (the round trip
at scale); and herbs that never reach their flowering node (10 or 12) still
establish 14–15 seedlings through the mature-stem seed route. **In the lab,
flowering buys a herb almost no fitness**, so the score's flowering step
pays for the herb's identity, not its survival.

**Teeth** (smoke run, 3 hold-out layouts): shipped 3.861, `seedbound` 1.100,
`starved` 2.191. The sabotaged start for the acceptance task (both
photosynthesis rates halved, `training/plants/starts/herb-halfphoto.ron`)
scores **1.160 ± 0.354 below the shipped herb, behind on 11 of 12 layouts**:
a gap the trainer has to close.

**The acceptance task: sabotage and recover, passed on one seed per arm**
(records in [`plant-trainer-2026-10-19/recover/`](plant-trainer-2026-10-19/recover/)).
Start: the herb with both photosynthesis rates halved; `groups=leaf`, pop 12,
10 generations, 6 training layouts, a hold-out of 8, a final set of 12 read
once, σ 0.4, run seed 11; the null identical but `select=random`; both on
two threads side by side, about 100 minutes each.

| arm (final set, 12 layouts) | score | minus the shipped herb |
|---|---|---|
| shipped herb | 3.882 | — |
| sabotaged start | 3.334 | −0.548 |
| **selection** | **3.799** | **−0.083 ± 0.293** (ahead 5, behind 4) |
| random-selection null | 3.418 | −0.463 ± 0.610 (ahead 2, behind 7) |

The champion **rediscovered the sabotage**: shoot photosynthesis
0.275 → 0.438 (shipped 0.55), leaf 0.225 → 0.494 (shipped 0.45), and settled on 5
leaves a node instead of 8. Its hold-out was level with the shipped herb from
generation 1 on (paired −0.209 to +0.232, generations 1–9). The null never
got there: −0.250 to −0.393 on every generation, and in generation 2 its best
*training* score (4.090) beat the shipped herb's (4.014) while its hold-out
trailed by 0.380. That is the best-of-many luck the paired bar and the hold-out
exist to catch, seen happening. **One seed per arm is a pass, not a measure
of the trainer**; P2 repeats it on three. Picture of all four on three beds
at frames 8,000 and 40,000 to the owner as a blind A/B card (shipped
against rebuilt), `20261019T053404354Z-52e16d`.

**The heritability gate, measured** (§6.4, the review's blocker; `plantrain
mode=heritability`, 10 children of the herb per row, each grown on the same 6
lab layouts, 40,000 frames). The share of the spread between children that is
their genome, `1 − mean within-child variance / total variance`:

| children | shoot size | height | size at first flower | score |
|---|---|---|---|---|
| 1–3 genes, σ 0.05 / 0.1 / 0.2 | 0.06 / 0.02 / 0.01 | 0.03 / 0.02 / 0.00 | 0.09 / 0.02 / 0.00 | 0.08 / 0.05 / 0.00 |
| 1–3 genes, σ 0.4 / 0.9 | 0.20 / 0.01 | 0.11 / 0.06 | 0.23 / 0.09 | 0.21 / 0.08 |
| 8 genes, σ 0.4, mixed light / all under a lamp | 0.03 / 0.04 | 0.03 / 0.07 | 0.02 / 0.24 | 0.03 / 0.25 |
| **8 genes, σ 0.9, mixed light / all under a lamp** | **0.69 / 0.70** | 0.34 / 0.41 | 0.46 / 0.61 | **0.37 / 0.56** |

**The review's blocker is real and now has a number.** A child one round of
1–3 genes away from the herb is invisible to selection at any step size: the
bed decides what it grows into. Children **8 genes at σ 0.9** apart are
visible (size 0.69–0.70), and fixing the light position helps second
(score 0.37 → 0.56). So the rule for look rounds is: candidates at least that
far from the shipped herb, compared within a light position. The sabotage run
still worked with 1–3-gene children because selection accumulates over
generations on one strong gene (photosynthesis); a look round cannot lean on
that.

**Gene liveness** (`plantrain mode=liveness`: each gene pushed to both ends
of its range, 2 layouts × 20,000 frames, bed hash against the herb's own):
**56 of 74 genes move the bed; 18 are inert** in a herb's lab bed. Three were
table bugs, leaf and head fields on a root's `Grow` block, now excluded. The
rest say something about the bed or the herb: **`wind_weight` is inert, so the
sealed box has no wind** (§5's open question, answered); `drought_death` is
inert under the mister; `branch_angle` at every tier, the tier-3 values and
`juvenile_branch` are inert because the herb never branches from its tip, and
they come alive only once another gene makes it branch. Those stay in the
table, since epistasis is the point, but a run spends a draw on them for
nothing until then.

**Determinism guard, and a red build fixed.** `a_run_is_a_function_of_its_settings`:
two tiny runs with the same settings write byte-identical `progress.csv`,
`holdout.csv` and champion. CI was red from `3c7ad5e9` to `5a46cb86`:
`examples/lab_resolution.rs` builds its `LabBox` exhaustively on purpose,
and the new seams were missing from it. My scan of `LabBox` literals before
adding fields read a range inside that literal as a struct update; the
compiler, `cargo clippy --all-targets`, is the scan to trust.

**P1 status (2026-10-19).**

- **Done:**
  - the seams;
  - the trainer core with every control in §7 except the blind arm;
  - the run constants measured;
  - the precocity probe passed;
  - the acceptance task passed on one seed per arm and posted to the owner;
  - the heritability gate measured;
  - gene liveness, which answered whether the box has wind.
- **Left for P1:**
  - the blind control, redesigned around a lamp moved off-axis (§7.2) and
    first checked for whether a herb leans toward an off-axis lamp at all;
  - mean against min of a genome's layouts, measured on the probe's variants
    before choosing (§5.3);
  - the acceptance task on two more seeds.
- **Carried to P2:**
  - stand beds;
  - a plant job type on the Actions workflow. A 10-generation run of 12
    genomes takes about 100 minutes on two threads here.

**The blind control's lean test: dropped, by construction and by
measurement.** `organism::phototropism_dir` returns `(0,-1)` or `(0,0)`, so
light steering can only pull up and the blind arm has nothing to remove. The
lateral repair was built and withdrawn on 2026-08-27
(`plant-phototropism-lateral-2026-08-27.md`). Measured anyway
(`plantrain mode=lean`, `measure/lean-bed17.log`, 72 s): herb, 4 world
seeds, founders 4 and 8 columns either side of the lamp nearest the middle
(lamps 17 apart), arms start, blind (`light_weight` 0 at every tier) and
heavy (x4):

- **Through frame 8,000 the three arms are the same plant to the cell**: the
  same shoot and the same lean in every row.
- After that they differ by under half a column (frame 24,000, offset 8,
  crown lean +3.77 to +4.58, against 2 sd/sqrt(n) of 6.6).
- **There is a lean towards a lamp 4 columns away, and it is not steering**:
  crown +1.89 columns at frame 8,000, the same sign from both sides
  (+2.64, +1.14), and the same in the blind arm. The lit side of the plant
  earns more and grows more.
- At 8 columns, half way to the next lamp, the two sides read opposite
  signs (-4.3, +12.1): a rightward habit of the bed, not a lean. It is the
  reason the test mirrors.

So the blind arm is the same herb and is dropped (§7.2). A plant that turns
to its light is now **U7** in §8. The withdrawn patch failed because every
authored `light_weight` was calibrated against a lever that could only say
"up". Re-finding those values under new physics is what this trainer is
for.

**Mean against min: the mean.** `plantrain mode=ranking`
(`measure/ranking-16x12.log`, 1,004 s): the herb and 16 children (σ 0.9,
up to 8 genes from shape, leaf, flower and height, the step the heritability
gate says is visible), scored on two disjoint sets of 12 layouts. Each
statistic picks its top four on set A, and the picks are judged on set B.

- **The min ranks nothing.** 16 of 17 genomes have a worst bed of exactly
  2.999, and that tidiness is the tell. It is the cap of the step below
  flowering: established, past the 55-cell size floor, and never flowered,
  the dim bed between two lamps. With every min tied, ranking by min falls
  through to its tie-break and picks the same four genomes as the mean
  (4 of 4 in common, identical on B).
- **The mean predicts a fresh set moderately**: rank correlation from mean
  on A to mean on B +0.50, and to the worst light position's mean on B
  +0.40. Min on A predicts neither (-0.15, -0.20). Its +0.64 against min on
  B is two genomes off the tie and nothing else.
- **Three children scored above the shipped herb on set B**: c02 4.458,
  c15 4.371 and c12 4.339, against 4.069. c15 also led set A (4.102 against
  3.911). That is 3 of 16 random 8-gene steps, not tested against the paired
  bar. It suggests there is room to climb, and proves nothing about any one
  of them.

Not measured: ranking by the worst light position's mean, which is graded
where the min is not. It needs only the per-bed scores this run already
computes. It is the alternative to try if a trained champion turns out to
hold up only under a lamp.

**The acceptance task on three seeds: selection beats random search on
all three.** The same settings as seed 11 (herb, photosynthesis halved, the
`leaf` group free, 12 genomes x 10 generations, 6 training layouts, 8
hold-out, a final set of 12 read once), at seeds 12 and 13, both arms. Each
run took 35–41 minutes on four threads. Files: `recover/*-s12-*`,
`recover/*-s13-*`. The selection and random arms at one seed share the same
final layouts, which is why their shipped rows match.

| seed | shipped | start vs shipped | selection vs shipped | random vs shipped | selection − random |
|---|---|---|---|---|---|
| 11 | 3.882 | −0.548 | −0.083 ± 0.293 | −0.463 ± 0.610 | +0.380 |
| 12 | 4.552 | −0.997 | −0.007 ± 0.119 | −1.002 ± 0.323 | +0.995 |
| 13 | 3.854 | −0.467 | −0.203 ± 0.528 | −0.528 ± 0.345 | +0.325 |

- **Selection recovered 85%, 99% and 57% of the sabotage, and random 16%,
  0% and −13%.** Selection's interval holds zero on every seed; random's
  excludes it on seeds 12 and 13. The plan's bar, *recovers and beats random search
  at equal compute*, is met: 3 of 3 by sign, 2 of 3 clearly.
- **The champions do not all take the same route back.**
  - Seed 11 put the rate back on both leaves and the shoot.
  - Seed 12 overshot on the shoot (tip photosynthesis 0.275 → 0.580, the
    shipped herb's is 0.55) and let leaf photosynthesis fall to 0.134.
  - Seed 13 raised the shoot only and cut the leaf cluster 8 → 5.

  The score cannot tell green stem from leaf. Whether the owner can is a
  question for P3's colour and leaf-shape work.
- **Seed 13 shows where the trainer is weak: which champion it hands
  back.** The champion is the last generation's best on 6 fresh layouts. At
  12 genomes that is a best-of-12 on a noisy sample. On the monitoring
  hold-out, generation 4's champion read +0.67 paired and generation 9's, the
  one returned, −0.10. The hold-out must not choose it (F10), so this goes to
  P2:
  - **more layouts per generation** (racing, §9);
  - or **the staged champions re-scored on a validation set** that is
    neither the hold-out nor the final set, with the best taken. The staged
    files now exist (`champion-gNNN.ron`).

**P1 is complete.** Every item in §10's P1 acceptance is met:
- the round trip and every seam guard pass, and were watched failing;
- teeth pass;
- the paired bar and the selection-only null print;
- runs are deterministic;
- the sabotage task recovers and beats random search on three seeds;
- the champion beside the shipped herb is with the owner as a blind A/B
  card.

The blind control was dropped by its own precondition (see the lean test
above). Ranking is by mean.

**Independent code review before merge (2026-10-19).** No problem in the
shared game code: the seams are default-off, `tick_world` is exactly the old
`Lab::tick`, and saved boxes round-trip byte-identically. Two real bugs in
the gene ranges, both now fixed and guarded by
`every_shipped_plant_starts_inside_its_own_ranges`, which was watched red
with both faults put back (9 values outside their ranges):

- **A relative range was built from `|v|`.** Shrub's and scrambler's
  negative `acrotony` (−1.6, −1.4) would have been clamped positive on the
  first step, turning a basitonous habit acrotonous. Ranges now keep the
  sign.
- **`juvenile_branch` was capped at 1.** It is a multiplier, and every
  shipped plant but herb authors 1.2–1.5. Its range is now [0, 4].

Neither touches the P1 results on herb: herb authors no negative relative
gene, and the sabotage runs left the shape group fixed. The heritability
and ranking runs did free it with the old cap, under which herb's
`juvenile_branch` (1.0) could only fall: one gene of about 40, not rerun.

Also fixed from the review, all in `plantrain`:
- `lean` prints stops in frame order;
- `heritability` scores with the run's own `k_seed`, `size_floor` and
  `n_est`, which it had silently ignored;
- `sweep` takes the start file itself as its baseline, so a trained file
  works;
- `ranking` refuses fewer than 3 layouts.

A run refuses a seed of 1000 or more, more than 100 layouts, and other
settings under which its three layout sets would overlap or be empty. The
determinism guard now runs its second run on one thread, as its doc
claimed.
