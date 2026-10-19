# The plant trainer — plan

*2026-10-19. Step 3 of 5 of the plant-trainer line: brainstorm
([`plant-training-brainstorm-2026-10-19.md`](plant-training-brainstorm-2026-10-19.md))
→ engine review
([`plant-engine-trainability-review-2026-10-19.md`](plant-engine-trainability-review-2026-10-19.md))
→ **this plan** → adversarial review (§12, appended to this file) → build.*

**What it is for.** A tool that breeds plant **species files**: herbs first,
then any plant, aimed at three things the owner asked for. These are plants
that live well in the lab, the current herb looking better, and new plants
that look like nothing in the box today. It is the plant counterpart of the
ant trainer (`src/train/`, 2026-10-17), and it borrows that trainer's
controls wholesale, because those controls are what made its scores honest.

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
not tuned.

## 2. Design in one paragraph

**The genome is the species file itself.** A run loads a parent species
(`herb.ron`). It mutates a declared set of its numbers and fate rules
(**gene groups**; the rest are frozen). Each child is registered through the
shipped parser (`register_ron`) and grown in small **lab beds** with nothing
else alive. Each child gets a **viability score in steps**: germinate,
establish, flower, set seed, seedlings establish. The trainer keeps either
the best (*train* mode) or the best **per look cell** (*atlas* mode,
quality-diversity), with the shipped species, a hold-out set and a
teeth check alongside every generation. Champions are written as species
files to a staging folder with a picture and a census. Moving one into
`assets/species/` is the owner's decision.

## 3. Where the code goes

| path | what | why there |
|---|---|---|
| `src/train_plant/mod.rs` | run loop, settings, CSV and sheet output | a sibling of `src/train/`, not inside it: `src/train/mod.rs` and `examples/train.rs` change daily on the brain lane, and a plant module in them would conflict on every merge |
| `src/train_plant/genome.rs` | the gene table, mutation, crossover, species-file round trip | |
| `src/train_plant/arena.rs` | bed kinds (lab, outdoor), layouts, pinned sky, the step loop | calls `LabBox` / `PlantScene` and `frame::step`, **never a private copy of either** (`lab/scene.rs`'s own rule) |
| `src/train_plant/score.rs` | viability steps, folding beds | |
| `src/train_plant/looks.rs` | look descriptors, vetoes, the atlas grid | |
| `examples/plantrain.rs` | the command line | the house pattern for instruments |
| `examples/herb_morphs.rs` | the review's probe (exists) | kept as the quick "what can it reach" look |
| `training/plants/<run>/` | **staging** for champion species files, sheets, CSVs | never `assets/` directly |

`PlantScene` lives in `examples/common/` and the library cannot see it.
The outdoor bed therefore moves into the library as a small builder, or
stays harness-side behind a trait. The build step decides which. That is
one of two library seams; the other is the lab bed, already in the library
(`lab::scene::LabBox`).

## 4. The genome

**4.1 What is mutable.** Each gene is an address, a kind, bounds and a step:

- **`ParamId` genes**: any of the 45 behaviour parameters, per cell type and
  per branch order where the field is per-order (`ByOrder`). Written through
  `ParamGenome::apply` and bounded by `clamp_param(param, v,
  param_scale)`. These are the exact functions the in-game heritable
  mutation uses, so a trained value is one the engine already accepts.
- **Species-level genes**: `annual_half_life`, `life_half_life`,
  `seed_half_life`, `remains_half_life`, `rebloom_after`,
  `seed_gut_survival`, `seed_provision_fraction`, `stomatal_reserve`, palette
  band choices (`first`, `count` within the material's band range), and the
  genotype `variance` arrays. Each gets explicit bounds, written in the gene
  table with the reason.
- **Fate genes**: `after_metamers`, plus structural moves on the table using
  `FateGenome`'s four operators (`retarget`, `recondition`, `insert`,
  `delete`) and `to_table`. They are weighted by the viability record:
  `child` mutations on a frontier type killed 5 of 6 plants, while
  `becomes`/`lateral` took 34 without a failure
  (`plant-fate-viability-2026-08-28.md`).

**4.2 Gene groups**, the plant version of the ant's banned verbs. A run
names which groups are free and freezes the rest:

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

**4.3 Mutation.** Each child changes **1–3 genes** (the ant rig's lesson:
small, rare connection changes held a mind lean). Magnitudes move by a log
factor `exp(N(0, σ))`. Probabilities and weights move additively by `σ ×
param_scale`. Counts move by integer steps. **A zero-authored value is
reached by a range draw**, never by a factor that cannot leave zero (the
`ParamGenome` doc's own correction). σ is per run (`sigma=`). The atlas
mode uses a larger σ. Uniform crossover between two parents is off by
default.

**4.4 The round trip is guarded.** A child is serialised with
`species_export::to_ron`, registered with `register_ron`, and the file
written to staging is **that text**. A test checks two things. First, the
shipped herb run through the round trip grows bit-identically to the
embedded one (hash of the bed after N frames). Second, a mutated child's
staged file, re-registered, grows bit-identically to the in-memory child
that was scored. *What was scored is what ships.*

## 5. The arena

**5.1 The lab bed is the default** (review §4.3: outdoors, the shipped herb
never flowers). It is built with `LabBox { colonies: 0, founders: 1, species:
<child>, width, seed, rain, plant_pace, .. }`. It is stepped with the lab's
tick: `frame::step`, then `lab::rain::tick`, as `Lab::tick` does. The
build step decides between giving `LabBox` a `species_ron` field (the
`PlantScene` pattern, whose doc records why registration must come before
planting) and registering on the world before a founder-less build. **The
first is the safer, because the doc records a silent empty stand from the
second ordering.**

**5.2 Bed size and length are measured, not assumed.** Starting points: 96
wide, the shipped soil depth, the shipped rain and pace. Length is set so the
shipped herb completes one full cycle (flower, fruit, seed, seedlings
established), read from its own trace. Everything that might buy speed is a
setting, priced against the shipped bed on the shipped herb's census before
it becomes a default (ruling 4): plant pace `Full` against `Half`, soil
depth, width, rain cadence. **A cost that vanishes may be work that
vanished** (`CLAUDE.md`). Each speed setting must leave the shipped herb's
viability steps and look descriptors inside the noise bar, or it is not
used.

**5.3 Layouts.** A layout is a world seed and a founder column relative to
the lamps, in **mirrored pairs**: the same bed with the founder mirrored
across the nearest lamp, scored on the worse side, as the ant rig does.
Training layouts are redrawn each generation. **Hold-out layouts are a
sealed set**, fixed for the run and never trained on.

**5.4 In-bed mutation is off while scoring**, as in the ant rig (owner
ruling of 2026-10-17 there: the only variation is the one the loop makes).
For plants that is two world dials, `World::fate_mutation_chance` and
`World::param_mutation_chance`, set to 0 by the arena. **Two heredity
channels have no off switch**: the per-birth genotype jitter
(`MUTATION_SIGMA`) and gene flips (`DISCRETE_MUTATION_CHANCE` 0.03). They
touch only the founder's offspring, so they reach the score only through
the *seedlings established* step. P1 measures whether that step moves when
they are forced to 0 in a scratch build; if it does, they get world dials
too, defaulting to today. The lab check (§7.7) runs with the game's own
settings, mutation included.

**5.5 The outdoor bed** stays available (`bed=outdoor`, `PlantScene` with
the sky pinned per layout). It serves the outdoor game and the known-answer
control (§7.2). It is never the default for scoring a lab plant.

**5.6 Stand beds come in at the second rung.** A plant that is good alone
can fail among others. `bed=stand` sows the child among shipped herbs and
grass, and scores the child lineage's share, `selection_arena`'s design
(mirrored assignment, lineage attribution).

## 6. Scoring

**6.1 Viability, in steps.** Each step is worth more than everything below
it, so selection always has something to climb. The ant's foraging score
had no slope until it was built this way.

| step | read from |
|---|---|
| germinated | the founder exists past its seed stage |
| established | the founder reaches N shoot cells (N measured on the shipped herb) |
| flowered | a shoot tip retired as `flowered` (`GrowWhy::Flowered`), or organ cells built |
| fruit dropped | `fruit_dropped` events by this lineage |
| seedlings established | the founder's offspring that reached N cells by the end |

The score per bed is the highest step reached plus a fraction of progress
inside the next. Beds fold to one number per pair by the worse side, then
average. **Seeds set never score on their own**: a plant would win by making
many tiny seeds that never germinate.

**6.2 Vetoes**, cheap checks that zero a bed's look credit and are printed
as counts. Each is a defect this repo has already met: root cells above the
soil line (W6); plant cells not connected to the founder's anchor; a
leafless standing stump (0-z); a one-cell whip (taller than K × its width
with fewer than M cells); a stick figure (wood above 85% of visible cells
with leaf under 5%, `plant-appearance-design.md` §2.1). The thresholds are
set from the shipped species: every shipped plant must pass every veto on
every hold-out bed, or the veto is wrong.

**6.3 Look descriptors.** They are computed from **what is drawn**: the
founder's cells by material, plus the shipped `Renderer`'s pixels for colour.
Never from cell labels; the house rule says a relabelled cell moves no
pixel. Each descriptor is validated before it is trusted. A control
variant built to move it must move it, and the shipped herb on the same bed
twice must read identically.

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
| within-genome spread across beds | recognisability: does it look like itself |

**6.4 How looks are optimised: three modes, never a beauty number.** An
optimiser fed a proxy for beauty finds the proxy's weak point first. That
is the ant trainer's three-generation cheating applied to looks. So no
mode turns descriptors into "good":

- **Atlas (quality-diversity, MAP-Elites): the default for new looks.** A
  grid over two or three descriptors (say crown profile × leaf clumping ×
  size). Each cell keeps the **most viable** plant that falls in it,
  provided it passes every veto and is recognisable (within-genome spread
  under a bar measured on the shipped species). Output is a sheet of
  distinct living plants. The trainer is asked for **variety at a viability
  floor**, so it cannot converge on one ugly winner. The owner picks from
  it.
- **Owner-steered rounds: the only score that is the bar.** Every round the
  review queue gets a gallery card of 9–16 tiles (each a column: the plant
  alone on three beds, plus a stand). The owner's picks become parents and
  are pinned in the atlas. Posting is fire-and-forget: the trainer keeps
  filling the atlas between verdicts, and a verdict that lands seeds the
  next generation. **"Better looks for the current herb"** is a round
  started from the shipped herb with a small σ and the `shape`, `leaf` and
  `flower` groups free.
- **Target**: distance to an exemplar the owner names ("like variant 5, but
  flowering"). Used only when the owner can say what they want.

**A learned taste model is deferred and gated.** A small preference model
over the descriptors, fitted to the owner's picks, may later steer the
atlas between rounds. It needs at least ~50 pairwise verdicts. It must
predict held-out verdicts better than chance, with the gap stated, before
it may steer anything.

## 7. Controls, every run (ported from the ant rig)

1. **The shipped species**, scored on the same layouts every generation.
2. **The hold-out**, every generation (`every=1`, the owner's rule for the
   ant rig): champion, shipped species, and the champion **blind**. Blind
   means light, wind and water steering zeroed. If blind scores the same,
   the plant has a fixed habit rather than a response to its bed.
3. **Teeth, before generation 0**: `lethal` (the shoot's `Grew` child
   becomes a seed) and `nobranch` must score **below** the shipped species
   on the hold-out. Otherwise the run refuses to start: a score that cannot
   tell a dead plant from a live one is blind.
4. **The noise bar**: the shipped species alone on every hold-out layout,
   giving mean, sd, standard error and the rows. A "gain" inside it is
   printed as such.
5. **Determinism**: same arguments, same run, byte for byte.
6. **The round trip** (§4.4).
7. **The lab check before promotion**: a champion goes into the lab's
   `played_bed` scenario (`latecensus`, plant-only first, then with the
   colony) over **12 seeds**, read as an order statistic against the
   shipped herb. **A trainer win is not a game win**: the ant's 8×
   forager kept half the colony in the lab.

**7.1 Known-answer tasks**, the trainer's own positive controls. If it cannot
rediscover these, it is broken:

- **Make the outdoor herb flower** (review §4.3): `bed=outdoor`,
  `groups=height,flower`, score capped at the flowered step. The hand fix
  was turgor 0.32 → 0.46. The trainer must reach *a* flowering herb, on all
  hold-out layouts, within 20 generations on three seeds.
- **Germination below the authored floor** in a dry bed
  (`plant-water-scarcity` §3). The trainer must lower `soil_water_threshold`
  and gain establishment.

## 8. Un-walling: engine changes that unlock looks

The review's §6: the trainer can arrange today's parts, not change them.
Each item below becomes a **species-file field whose default reproduces
today bit-identically** (guarded by the stand hashes and a dedicated
before/after test). It also gets **a price** where it buys anything, since
a free lever produces uniformity. Only then is it added to the trainer's
gene table. **Each one goes to the owner as a blind A/B card before it is
used**, because they are looks.

| # | field | today (hardcoded) | proposed values | price |
|---|---|---|---|---|
| U1 | `phyllotaxis` | first leaf a random empty cell behind the tip | `random` (today), `alternate`, `opposite`, `whorled` | none (placement only), but it must be shown to move pixels |
| U2 | `leaf_shape` | breadth-first spray, `leaf_spread` blob ↔ line | `spray` (today), `fan`, `tuft`, `blade` | none |
| U3 | `max_root_fraction` | 0.5 for every species, binding on herbs (review §4.4) | per species, default 0.5 | roots already pay maintenance |
| U4 | `organ_shape` | random blob | `blob` (today), `spike`, `umbel`, `disc` | per-cell organ costs already apply |
| U5 | species-owned palette | shared material palettes, 8 bands | **owner's call**: a species may carry its own leaf, flower or bark palette | none (colour is not physics; the doc on `PaletteBands` says so) |
| U6 | species-owned developmental key | world setting | **owner's call** (review §7.5) | none |

U1–U4 are look changes that ship **default-off**: the default *is* today.
A trained species opts in through its own file. That is the standing
ruling on looks.

## 9. Speed and compute

Measured so far: a one-herb outdoor bed of 96 columns for 20,000 frames
takes **2.7 s** on one thread here, and twelve such beds of mixed variants
take 9.3 s on four (about 3 s a bed, since a bushy variant costs more than
the shipped stalk). The lab bed is not measured yet. Budget at those
numbers: 32 genomes × 8 layouts × 3 s ≈ 13 min a generation on one thread,
about 3.5 min on four, so 40 generations ≈ 2.3 h. Ways down, each measured
before use:

- **Racing**: a child is scored on 2 layouts, and only the top half
  continues to all 8. Checked once against full scoring: the same top
  quarter on N generations.
- **Shorter beds** once the trace shows when the score stops moving.
- **The cheap bed** (ruling 4), §5.2.
- **GitHub Actions**, as the ant trainer uses (`.github/train-jobs/`): a
  `plantrain` job type once a run fits the 6-hour limit.

## 10. Milestones

Each milestone is a PR. Each says its measurement in the commit message
and does not land broken.

**P1: the trainer core, on the outdoor bed.** No engine change at all.
- `train_plant` genome (ParamId and species-level genes, gene groups,
  mutation), arena (outdoor bed, pinned sky, mirrored layouts), viability
  steps, the run loop with controls 1–6, CSV, a per-generation sheet, staged
  champion files. `examples/plantrain.rs`.
- **Accepted when**: the round-trip test passes and was watched failing (a
  deliberate re-serialisation fault); teeth pass; the noise bar prints; the
  run is deterministic; **the known-answer task passes** (the outdoor herb
  learns to flower on three seeds); the sheet of the champion beside the
  shipped herb goes to the owner as a card.

**P2: the lab bed.** `LabBox` arena, made the default. Its cost measured
whole-frame. The cheap-bed settings priced one by one (ruling 4). The noise
bar re-taken in the lab. The second known-answer task.
- **Accepted when**: the shipped herb completes a cycle in the arena; each
  speed setting adopted is inside the noise bar; the lab check (§7.7) runs
  on a P1-style champion.

**P3: looks, without engine change.** Descriptors (each with its control),
vetoes (each passing on every shipped species), the atlas, owner-steered
rounds through the review queue. The first owner round is "the current herb,
better" (small σ from the shipped herb). The second is "new herbs" (large
σ, `fates` free).
- **Accepted when**: an atlas sheet of at least 12 distinct viable herbs
  reaches the owner; the owner's picks are fed back as parents in a later
  generation, and the picked plant's descendants are shown back.

**P4: un-walling**, in owner-approved order, one PR each (U3 first, since it
is the smallest and unblocks root habit; then U1, U2, U4). Each: field,
default bit-identical, guard, blind A/B card, then the gene.

**P5: promotion.** The owner's chosen herbs become species files in
`assets/species/`, with an `include_str!` line in `organism.rs`'s
`EMBEDDED` and the registry tests in `tests/*.rs` run, because adding to a
swept set enrols the species in every rule over that set. Each gets a
`README` status line and a `wiki/plants.md` section with a dated freshness
note. The lab check (§7.7) is a precondition.

**P6, conditional: a plant "mind"** (brainstorm rung G), only if P3–P4 show
the parameters are the limit.

## 11. Risks, and what each is answered by

| risk | answer |
|---|---|
| The score is cheated (tiny seeds, seeds thrown off the bed, a plant too short to be shaded) | stepped viability with no seed-count term; vetoes; worse-side folding; teeth; hold-out |
| The look proxy is optimised instead of the look | no beauty number; atlas plus owner picks; a learned taste only after it predicts held-out verdicts |
| Trained alone, fails in a stand | stand beds from P2; lab check over 12 seeds before promotion |
| The lab bed is too slow to train | racing, shorter beds, the cheap bed, Actions; all measured |
| A trained species file means something different in the game (bed regime, developmental key, pace) | the arena is the lab's own builder and tick; the round-trip test; the lab check |
| Un-walling changes every plant in both games | every field defaults to today bit-identically, guarded; looks ship default-off |
| Free levers converge (uniformity) | prices on new fields; atlas rather than single champion |
| Noise swamps selection (herb size heritability 0.00–0.05 at small steps) | mirrored beds, the noise bar, larger σ where the probe showed the genome explains 48–76% |
| The brain lane's files conflict | separate module and binary; no edits to `src/train/` or `examples/train.rs` |

## 12. Adversarial review

*(Appended after the independent review; findings and responses.)*
