# Plants that grow like plants, and a bed where different kinds coexist: plan (2026-10-20, revision 2)

**Status: plan, revised after the owner's review the same day; the open
questions are in §9. Nothing here is built yet.** It supersedes P2–P6 of
[`plant-trainer-plan-2026-10-19.md`](plant-trainer-plan-2026-10-19.md). That
plan's P1, the trainer core, is on `main` (PR #731) and is reused throughout.

**What changed in revision 2.** Revision 1 took the owner's examples (roots to
water, shoots to light) as the specification, and planned one rung: a whole bed
run long. The owner asked for:

- research on how plants actually grow, reflected in Part A (§3);
- an explanation of how an individual plant is scored (§5);
- a diverse bed of a *few varieties filling different ecological needs*,
  grounded in real ecology, not all our designed plants (§4);
- a ladder like the ants', where most rungs are small and quick enough to run
  for many generations (§6).

Revision 2 also uses the project's existing Google Cloud setup and its $100 a
month (§7).

## 0. In one paragraph

The goal is a lab bed where a few different kinds of plant, each filling its
own ecological role, live together and keep thriving over a long run. It is
reached by training each kind in small steps.

Two things stand in the way:

1. **Today's plants differ mostly by dice.** Clones of one herb a column
   apart range 134–387 cells. **Part A** rebuilds development the way real
   plants develop, so a plant's form comes from its genes acting on its
   surroundings, with dice adding only a little.
2. **A whole bed run long is too slow and too noisy to be a training step.**
   **Part C** borrows the ant ladder. Small arenas teach each stage of a
   plant's life in turn, from germinating in a pocket of soil to invading an
   established bed when rare. Whole long beds are kept for validation only.

**Part B** says what a "diverse bed" is, ecologically: which roles, which
trade-offs separate them, and the test that proves they coexist.

## 1. Rulings this plan is built on

**Owner, 2026-10-20:**

- **Development:** plants should resemble real plants, with genetics and
  environment both playing a role. *"If cloned plants all look very different
  even in almost identical conditions then nothing we are doing matters."* Part A
  ships on for every plant in both games, each step shown first as pictures in
  the chat (agreed).
- **Colour:** a species may carry its own colours. Each is built from one base
  colour with the existing 4-tone shading, kept within a natural range per plant
  part, and none ships without the owner's pick (agreed). Not needed until
  looks.
- **Goal:** *"a diverse bed … doesn't need all our designed plants. It should
  have a few varieties that fill different ecological needs."* Looks come
  later. *"Each training step should be a small step towards that goal."*
- **Ladder:** small and quick for many generations, the way the ant forager
  was built rung by rung.
- **Compute:** Google Cloud, through the launcher the ant lane built (PR 734),
  aiming at **$100 a month**.
- **Images:** they come in the chat, or in a report readable in the chat.

**Standing rulings:**

- Features ship on unless a harm is measured.
- **An outcome is a distribution, not a binary** (`CLAUDE.md`'s ethos).
- *"A wrong default is reported, not tuned"*: the trainer is the tool, and the
  owner promotes what it finds.
- **The trainer never overwrites a shipped species.**
- **A bed is judged over a few hundred thousand frames, as an order statistic
  over seeds.**

## 2. Where things stand, measured

### 2.1 Development is mostly dice

| what | number | source |
|---|---|---|
| One herb genome, 8 spots one column apart, today's key | 134–387 cells, 29–67 rows (size CV 0.41) | `clone_variance shift=1`, 2026-10-20 |
| The same with the per-plant key | 128–142 cells (CV 0.04) | same |
| Herb size heritability, today / per-plant key | 0.034 / 0.650 | `plant-engine-rethink-2026-09-03.md` §6.8 |
| Tree clones a column apart, per-plant key | still diverge, 151–3,368 cells at 20,000 frames; the amplifier was never found | rethink §6.13 |
| Trainer children 1–3 genes from the herb | score heritability ≤ 0.21: selection cannot see small steps | trainer plan §13 |

**Where the dice are** (code survey of `src/sim/plant.rs`, 2026-10-20). Four
rolls change a plant for good:

1. **The taper roll** (`:5863`): four misses retire an axis, so a branch's
   length is a lottery.
2. **The step sample** (`:6328`): a vertical tip steps diagonally 59% of the
   time.
3. **Which side a leaf goes** (`:6806`, `:6915`).
4. **Bud kill**: thickening kills each neighbouring bud with chance 0.5
   (`:14520`).

All four are keyed on world position and the clock (`:2278`), or under the
per-plant key on **age**. So a plant one tick behind re-rolls everything
after: the likely amplifier §6.13 never found (INFERRED).

**What the plant senses today:**

- **Light:** income per leaf (graded), and an up-or-nothing steer. Measured
  2026-10-19: light steering as shipped, off, or ×4 gives the same plant to
  the cell through frame 8,000.
- **Water:** roots switch to the wetter side past a threshold, and drought
  shifts growth to roots.
- **Crowding:** steers a tip.
- **Missing:** shade leaves, shade avoidance, branching that responds to
  light or sugar, root foraging, and a sideways lean to light.

### 2.2 The community today

- **The bed of record ("mix7")**: herb, scrambler, shrub, creeper, grass,
  conifer and tree, 450,000 frames, 12 seeds. About 4 of the 7 kinds hold
  100+ leaf cells per seed.
- **Grass is the ratchet.** It once took 21% of the low plants' columns every
  50,000 frames and gave none back.
- **Low plants lose to seed heaps and burial** (76 of 89 covered sprouts
  died). Every seed-thinning lever tried grew more grass.
- **Herbs** (annual) are still declining at 450,000 frames. With rain off they
  die out, and a 49.7M-frame playtest ended as grass alone.
- **Ants** live mainly on dead leaf (60% of their energy) and move seed (188
  seeds carried home in one playtest).

### 2.3 What a bed costs

Measured 2026-10-20 on a 4-vCPU machine, played bed (512×320):

| run | time |
|---|---|
| 10,000 frames, no ants, 1 thread | 6.0 s |
| the same, 4 threads | 7.4 s |
| 10,000 frames, with ants, 4 threads | 9.0 s |
| 60,000 frames, no ants, 4 threads | 62 s (cost rises as the bed fills) |

**One bed per core beats one bed on four threads.** So training runs many
small beds side by side, one per core. A full 450,000-frame bed is about 8–12
core-minutes. P1's single-plant bed (96 wide, 40,000 frames) is about 12
core-seconds.

## 3. Part A: development the way plants develop

### 3.1 How real plants grow: what the engine should copy

| how a real plant does it | what it means for the engine | engine today |
|---|---|---|
| **Built from repeated units.** A shoot tip lays down one unit (node, internode, leaf, bud) at a steady rate, and **leaves are placed by a rule, not a coin**. In phyllotaxis, each new leaf goes where the inhibition from earlier leaves is weakest. The result is canalised: noise produces a few specific defects, not a different plant ([Mirabet et al. 2012](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC3280957/); [geometric canalisation](https://hal.archives-ouvertes.fr/hal-03014239)) | the unit rate stays (plastochron); **leaf position becomes a phyllotaxis rule** (alternate, opposite, spiral) with a small jitter | leaf side is a coin |
| **Branching is controlled, not random.** A bud stays dormant while the growing tip above it dominates (apical dominance, via auxin). It breaks when that signal weakens with distance, or when sugar is plentiful (sugar opposes the auxin signal). Shade suppresses it ([Bertheloot et al. 2020](https://pmc.ncbi.nlm.nih.gov/articles/PMC10504579/); [bud outgrowth review](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC10610460/)) | a bud breaks when (distance from the active tip × local sugar × light) crosses a threshold the genes set. Thickening no longer kills buds by coin | bud kill is a coin; bud break is an argmax of light |
| **Shade avoidance or shade tolerance: two strategies.** A plant that detects neighbours (red-to-far-red drops from 1.2–1.5 in the open to 0.2–0.4 under a canopy, before real shading) lengthens its internodes, branches less, angles its leaves up and flowers earlier. A shade-tolerant plant instead makes bigger, thinner, cheaper leaves and lives on little ([Wang et al. 2020](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC7174782/)) | a **neighbour signal** per tip: the light already shadowed by other plants, plus crowding (the engine has no light quality; leaf shadow and crowding are the honest proxies). **Two genes set the response**: how much to elongate and stop branching (avoid), and how much bigger, thinner and cheaper leaves get (tolerate) | crowding only steers; nothing else |
| **Sun leaves and shade leaves.** Shade leaves are larger and thinner (high leaf area per mass), worse in bright light, better in dim. All leaves sit on one fast–slow economics spectrum: cheap, short-lived, high-rate leaves against costly, long-lived ones ([Wright et al. 2004](https://research.vu.nl/en/publications/the-worldwide-leaf-economics-spectrum/); [Chen et al. 2020](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC7300261/)) | leaf cluster size and leaf cost respond to the light a leaf sits in. **Leaf lifespan against leaf rate becomes a priced gene axis.** Income must count per leaf, not per node at the genome's cluster size (`plant.rs:8160–8210`), or a bigger cluster in shade earns nothing | leaf size fixed per species; leaf cost and lifespan partly separate |
| **Phototropism** bends shoots towards the brighter side, graded | the sideways-light patch already built (`plant-phototropism-lateral-2026-08-27.md`), opted into per species, with the trainer re-finding each species' light weight | up or nothing |
| **Roots:** gravity sets the default direction and moisture bends roots, graded. Roots **proliferate in rich or wet patches** (precision) or **explore widely** (scale). The supposed trade-off between the two is contested across species, but both behaviours are real ([Kembel & Cahill 2005](https://www.journals.uchicago.edu/doi/reader/10.1086/431287)) | graded hydrotropism. **Root branching keyed on local water and nutrients** (precision). A tap-against-fibrous gene sets rooting depth and spread | an on/off switch past a gradient of 0.05; branching a coin |
| **Allocation follows the limiting resource.** More root when water or nutrients are short, more shoot and leaf when light is short. The direction is well supported; the "optimal" version is debated ([OPT review](https://revistas.unal.edu.co/index.php/actabiol/article/view/27158); [Robinson 2022](https://abdn.pure.elsevier.com/files/281226451/Robinson_PCE_OPT-ing_Out_Root_VoR.pdf)) | root:shoot responds to light as it already does to water | water only |
| **Life history and timing:** annual against perennial, flowering on size, age and shade, and seed dormancy as a hedge (a little *designed* variability is real: [plant noise review](https://pmc.ncbi.nlm.nih.gov/articles/PMC4845803)) | kept, with timing as genes. **Germination spread is the one place deliberate dice belong** | mostly there |

**The idea that ties these together is the reaction norm.** A gene does not
fix a trait. It fixes how the trait responds to the surroundings: leaf size
against light, internode length against neighbours, root share against
water. That is exactly *"environment plays a role and genetics play a role"*.
Two plants with the same genes grown in different light differ for a reason,
and differ the same way every time. Two plants with different genes in the
same light differ because of the genes. **In the engine, each plastic trait
gets a base value and a sensitivity, both genes.**

### 3.2 The target, and the instrument (A0)

There are three numbers, with starting bars that M0 replaces with measured
ones:

| number | how | today (herb) | first bar |
|---|---|---|---|
| **Dice** | clones in identical surroundings, different spots and different developmental seeds | size CV 0.26–0.41 | ≤ 0.10 |
| **Surroundings** | clones across set gradients (bright ↔ dim, wet ↔ dry, alone ↔ with a neighbour): shift ÷ dice spread | unmeasured; sideways light provably zero | ≥ 3, in the direction §3.1 says |
| **Genes** | different genomes, same surroundings: heritability | 0.03 | ≥ 0.5 at the trainer's step size |

The instrument is `examples/clone_variance.rs` extended:

- **arms:** a seed arm, an environment arm and a one-line summary;
- **a picture strip per arm**, sent to the chat;
- **positive controls:** a gene known to change size must read heritable, and
  the seed arm must read non-zero today;
- **species:** herb, shrub and grass.

### 3.3 The changes, in order, one PR each

Each change gets the three numbers before and after, a picture in the chat,
and a frame-cost check.

1. **A1: separate the plants' dice from the weather's.** The leaf-litter
   shade and the rain mister appear to share one generator
   (`plant.rs:13623`, `lab/rain.rs:332`; INFERRED, confirmed first).
2. **A2: key dice on developmental position** (the plant's seed, axis, node),
   not on world position and the clock. A plant one tick behind then rolls the
   same numbers when it gets there.
3. **A3: canalise the four lasting rolls**, using §3.1's rules:
   - leaves by a phyllotaxis rule, with a small jitter;
   - buds by apical dominance, sugar and light, not a coin;
   - the taper roll becomes a carried growth fraction;
   - the tip's step becomes the best-scoring neighbour with error diffusion,
     plus a small temperature;
   - a margin on near-tie choices.
4. **A4: the key that ships.** After A2–A3 shrink the dice, each plant gets
   its own seed at germination: individual, but the dice are too small to
   swamp genes or light. The pictures decide.
5. **A5: reaction norms**, each a base gene and a sensitivity gene, each with
   an environment-arm strip:
   1. shade response: avoid against tolerate (§3.1 row 3);
   2. sun and shade leaves, and the leaf-economics axis;
   3. graded hydrotropism, and root branching in wet or rich soil;
   4. root:shoot balance following light;
   5. sideways light, per species (U7).

   Each response comes with its cost, or a free response becomes uniform.
   For example, elongating in shade costs stem strength and seed set, and
   tolerance means a slower top speed.

After Part A, **today's species are re-fitted by the trainer, not by hand.**
Their constants were tuned against dice. That re-fit is also the first real
training job.

## 4. Part B: a diverse bed, ecologically

### 4.1 What lets different plants live together

Real coexistence needs **each kind to be limited more by itself than by the
others**: niche differences big enough to outweigh fitness differences
([Chesson's coexistence theory](https://en.wikipedia.org/wiki/Coexistence_theory);
[Adler et al. 2007](https://faculty.washington.edu/jhrl/Adleretal07.pdf)).
**The practical test is mutual invasibility: each kind, introduced rare into a
bed of the others, must increase.** If every kind can, they coexist. If one
cannot, it is being excluded, however things look at frame 450,000. This test
is also the cheapest community measurement there is (§6, rung 6).

Niches come from **trade-offs**: being good at one thing must cost another.

| the axis | the trade-off | how it shows | in the lab? |
|---|---|---|---|
| **Light, vertically** | grow fast in bright light ↔ survive in shade (the growth–survival trade-off; rankings reverse between gaps and understory) ([Baraloto et al. 2005](https://deepblue.lib.umich.edu/bitstream/2027.42/117031/1/ecy20058692461.pdf)) | a tall light-taker over a shade-tolerant understory | **yes**: ceiling lamps make a vertical light gradient under every canopy |
| **Time and disturbance** | a fast annual that seeds a lot ↔ a slow perennial that holds ground (Grime's ruderal ↔ competitor; Tilman's colonisation ↔ competition) ([CSR overview](https://aj.tubitak.gov.tr/botany/issues/bot-10-34-6/bot-34-6-6-0912-270.pdf)) | annuals fill the gaps perennials leave when they die, or where ants dig and graze | **yes**: ants dig and eat; plants age and die (grass since 2026-10-19) |
| **Spreading** | clonal, vegetative spread (holds ground, few seeds) ↔ seed dispersal (reaches new gaps) | a tillering ground cover against seeders | **yes**: grass tillers; the rest seed |
| **Rooting depth** | shallow fibrous roots that take rain first ↔ deep roots that reach stored water (Walter's two-layer idea for grasses and woody plants) | grass shallow, shrub deep | **unknown**: the bed starts at field capacity and the mister wets the top. **M0 measures the soil-water profile over a long run**; if water is never short at depth this niche does not exist here |
| **Being eaten** | defended and slow ↔ palatable and fast | the ants' choices shift the balance | partly: ants eat mostly dead leaf |

### 4.2 The roles: four, not nine

**Proposed** (the owner may cut or add one; §9). Each existing species is a
starting point, not the answer:

| role | ecological need it fills | trade-off that keeps it from winning everywhere | starts from |
|---|---|---|---|
| **Canopy perennial** (competitor) | takes the light at the top, holds ground for a long time | slow to establish, needs bright light, few seeds, pays for wood | shrub |
| **Shade-tolerant understory** (stress-tolerator for light) | lives under the canopy on dim light | slow, low top speed, loses in the open to the fast | creeper (or moss) |
| **Annual colonist** (ruderal) | fills gaps fast after death and disturbance; keeps a seed bank | short life, loses any long contest, needs open ground | herb |
| **Clonal ground cover** | spreads sideways, holds open, bright ground near the surface | shallow-rooted, poor in shade, few seeds | grass |

**What "diverse" means, in numbers:**

- all four roles are present and **reproducing** in the last 100,000 frames of
  a long bed;
- **each one can invade the other three from rare**;
- no role is trending to zero;
- the bed is not one carpet with tokens (an evenness term).

All of this is read as the 25th percentile over 12 seeds, so a bed that is
diverse on half its seeds does not count. The score is graded, never pass or
fail.

## 5. How an individual plant is scored

**The unit of selection is a genome**: one species-file variant. **An
individual** is one plant grown from it.

**An individual's score is how far it gets through its own life, graded**, as
P1 already does. Each step is worth more than everything below it, with a
fraction for progress inside a step:

| step | the fitness component (real life-history terms) | graded by |
|---|---|---|
| 1 | germinated | — |
| 2 | **established**: rooted, with its first leaves | shoot cells to the establishment size |
| 3 | **grew**: relative growth rate in its first stretch of life | growth rate against the shipped species in the same station |
| 4 | **survived** to maturity | the fraction of the run it lived |
| 5 | **reproduced**: flowered and set seed | seed count up to a cap |
| 6 | **recruited**: its seedlings established | seedlings up to a cap |

A cap on each count stops the cheap climbs P1 found, such as a dwarf that
flowers at once, or a flood of tiny seeds.

**A genome's score combines its individuals.**

- **Stations.** One plant is noisy (§2.1), so every genome is grown as
  several individuals across a set of **stations**: small arenas of fixed
  conditions (bright, dim under a canopy, dry topsoil, beside a neighbour).
  Its score is the mean over its station set, and that set depends on its
  role. An understory candidate is scored mostly in dim stations, and a
  colonist mostly in open, disturbed ones. That is how a genome is judged
  across its niche, with the reaction norm included.
- **Community rungs.** There the score is **population-level**: the growth
  rate of the genome's own line when it is rare in a bed of the other roles.
  That is the ecological definition of fitness in a community, and it is what
  mutual invasibility tests.
- **Others do not pay for it.** A step also has to keep every other role
  above its floor. A candidate that wins by wiping out another role is
  rejected, however well its own line does.

**Diversity is never scored on a single genome.** It is the outcome of each
role thriving in its own niche, together with mutual invasibility, and it is
checked at the top of the ladder. This keeps each step attributable: one
genome, one score, in conditions anyone can rerun.

## 6. Part C: the ladder

### 6.1 What the ant ladder did

The ant forager was not trained by letting a colony loose and counting food.
It was trained:

1. on a few ants following a pheromone trail, scored on distance;
2. then following the trail to food;
3. then on picking food up;
4. then on carrying it closer to the nest;
5. then on carrying it home and dropping it there.

Each rung was small and quick, and taught one stage. The next rung started
from the last rung's champion.

**A plant's life has stages in the same way**, and each can be put in a small
arena: germinate, establish, grow, survive, reproduce, recruit, hold ground
against a neighbour, invade a bed. The stages are cumulative, as the ant's
were. A rung's score is the stepped individual score (§5) up to that stage,
and the run is cut off once the stage is decided.

### 6.2 The rungs

Costs are estimated from §2.3, assuming the arena's cost scales with its area
and length (to be measured in M0). "Gens/hour" is for 12 genomes × 8 stations
on one 16-core machine.

| rung | arena | what is scored | length | est. cost per bed | est. gens/hour |
|---|---|---|---|---|---|
| **1 Establish** | a pocket of soil 24 wide, one seed, a station's light and water | steps 1–2: germinated, rooted, first leaves | 3,000 frames | ~0.3 core-s | hundreds |
| **2 Grow** | the same pocket, 48 wide | step 3: growth rate over the juvenile stage, per station; the reaction norm is learned here | 10,000 | ~1.5 core-s | ~100 |
| **3 Complete a life** | P1's bed (96 wide), alone | steps 4–6 | 40,000 | ~12 core-s | ~15 |
| **4 Hold ground** | 64 wide, the candidate beside one fixed plant of each other role in turn | own steps, plus the neighbour must also complete its life | 40,000 | ~10 core-s | ~15 |
| **5 Recruit into a stand** | a pre-grown stand of the other roles (grown once, copied per candidate), a few candidate seeds in a gap | the candidate's seedlings established, and residents above their floors | 30,000 from the copy | ~8 core-s | ~20 |
| **6 Invade from rare** | the same pre-grown bed of the other three roles, at steady state, copied per candidate | **invasion growth rate**: did the candidate's line increase from rare over the window? Residents above their floors | 80,000 from the copy | ~25 core-s | ~6 |
| **7 Long bed** (validation only) | the four-role bed, full size, 12 seeds | §4.2's score | 450,000 | ~10 core-min | — |

**What makes the top rungs affordable** is rungs 5–6, which grow the resident
bed once and copy it for every candidate. `World` already derives `Clone`,
and the lab already starts batch runs from a copied world
(`batch::Start::Copy`, `lab/mod.rs:1078`). Only the window after the copy is
paid per candidate, not the hundreds of thousands of frames it took the bed
to settle. That is the plant version of a short ant rung.

**Rules for the ladder, from the ant lane:**

- **Each rung must predict the next.** Before a rung is used as a filter, its
  ranking of a set of candidates is checked against the next rung's ranking
  (rank correlation, as P1 checked mean against min). A rung that does not
  predict the next one is teaching something else, and is redesigned.
- **Earlier rungs stay as a regression suite.** A step accepted on rung 5
  must not lose rung 3.
- **The champion is picked on a validation set** that is neither the
  monitoring hold-out nor the final set.
- **Controls every round:**
  - a crippled-species known answer, beside random selection at equal
    compute (P1's acceptance task, one rung up);
  - determinism;
  - a **cheat watch**: a gene pinned at its range, or a role winning by a
    lever with no cost, is reported as an engine pricing gap and never tuned
    around (`plant-equilibrium-costs-2026-08-27.md`).
- **The owner sees each round's champion** as a picture or a GIF in the chat,
  with the numbers.

### 6.3 Order of training

1. **Re-fit the four role species to the Part A engine on rungs 1–3**, one at
   a time, each in its own station set. This is the first training job, and it
   is where small steps first become visible to selection, once the heritability
   gate is re-measured after Part A.
2. **Rung 4 for each pair of roles**, worst first. Each step is one species
   file.
3. **Rungs 5–6 round-robin**: train whichever role currently fails to invade,
   until all four can.
4. **Rung 7** with and without ants, then the owner's look at the GIF. Then
   promotion (§8, M5).

## 7. Compute: Google Cloud, $100 a month

**What exists** (PR 734, `Reports/brain-training/cloud-compute/gcp/`):

- `gcp_train_job.py` starts a **spot VM** that builds the repo at a pinned
  commit and runs a training job. It writes a snapshot to the results bucket
  every 5 minutes, restarts after a preemption from the bucket copy, and
  **deletes itself after its hours plus one**.
- The key comes in as `GCP_SA_KEY_B64` (never in a file, log or chat).
- Project `pixel-physics-512114`, us-central1, with a quota of **64 spot vCPUs
  shared with the ant lane**.
- Measured on ant jobs:
  - `c4d-highcpu-32` (AMD, 32 vCPU) did 100 generations in about 1.5 hours for
    about $0.65;
  - a `-16` is 0.6× the speed at half the price;
  - the Intel `c4` is 11% slower;
  - the build is a fixed ~95 s.

**What plants need from it:**

- **The launcher runs the ant trainer**, so it gains a choice of binary
  (`plantrain`) and a plant job file. A plant job must also run one bed per
  core, which is a change to how `plantrain` schedules work. It currently
  parallelises inside a generation; that suits rungs 1–3 as they are.
- **Rungs 1–4 are short**, so a preempted VM costs minutes. Rungs 5–6 resume
  from the saved generation, as the ant trainer does.
- **`c4d-highcpu-16` is the default for plant jobs.** Plant work is many
  independent beds, which favours more machines over bigger ones.

**Budget.** The $100 a month is assumed to be shared with the ant lane, with
plants planning for about **$40 a month**, to be confirmed (§9). At spot
prices that is roughly **180 machine-hours of a 16-vCPU VM**, about 2,000
bed-core-hours. A rung-1–3 round of four roles costs well under $1. A
rung-6 round is a few dollars. A single rung-7 validation (12 seeds × 4
roles, with and without ants) is about 4 core-hours, about $0.25.
**Training is cheap; the expensive mistake would be running whole long beds
as training steps, which this ladder avoids.**

**What the next session needs:**

- the key in its environment;
- `pip install google-cloud-compute google-auth`;
- network access to Google's APIs;
- the PR 734 notes.

This session cannot launch anything.

## 8. Milestones

| | what | done when |
|---|---|---|
| **M0** | **Baselines and feasibility, no engine change:** the development report (§3.2) for herb, shrub and grass; mix7 and a four-role bed, 12 seeds × 450,000 frames, with and without ants; **the soil-water profile over a long run** (§4.1); the cost of a small arena by size and length; a copied bed starting the same as its original | pictures and numbers in the chat; §3.2's bars set; §6.2's costs measured; the rooting-depth niche confirmed or dropped |
| **M1** | **Part A**, A1–A5, one PR each | each meets its bar, with strips shown |
| **M2** | rungs 1–3 as `plantrain` modes, with station sets; the rung-predicts-rung check; the four roles re-fitted to the new engine | each re-fitted role completes its life in its own stations at least as well as the shipped species did on the old engine |
| **M3** | the launcher runs plant jobs (new session, with the key) | an M2 job reproduces byte for byte on Google Cloud |
| **M4** | rungs 4–6, the crippled-species known answer, and training rounds | all four roles invade each other from rare |
| **M5** | rung-7 validation with and without ants; the owner's GIF; promotion of the files the owner picks; README, wiki and freshness notes | the owner's pick |

M0 and M1 run here, with no Google Cloud needed.

## 9. Open questions for the owner

1. **The four roles** (§4.2): canopy perennial, shade-tolerant understory,
   annual colonist, clonal ground cover. Cut one, add one (a climber, or a
   deep-rooted drought plant), or keep them?
2. **The budget split:** of the $100 a month, is about $40 for plants right,
   given the ant lane uses the same account and quota?
3. **If M0 finds water is never short at depth:** drop the rooting-depth
   niche, or change the lab's watering so water has a profile? The second is a
   change to the game, not to the plants, and is yours to make.

## 10. Risks

- **Coexistence needs trade-offs the engine may not price.** A free lever
  makes every role converge on it, and *"a free lever made heritable produces
  uniformity, not diversity"* (`dead-ends.md`). Answered by the cheat watch,
  by giving every Part A response a cost, and by the mutual-invasibility test,
  which fails loudly when one role dominates.
- **A rung that teaches the wrong thing.** Answered by the rung-predicts-rung
  check before each rung is used.
- **Small arenas are not the bed.** A pocket of soil has edges a real bed does
  not. Answered by the same check, and by rung 7.
- **Part A moves every plant in both games.** One PR per change, pictures
  first, M0 baselines to compare against.

## 11. Sources

Plant biology and ecology, all found through searches on 2026-10-20 (abstracts
and summaries; full texts not read):

- **Shade avoidance:** Wang et al. 2020, *Front. Plant Sci.*, [PMC7174782](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC7174782/).
- **Phyllotaxis noise and canalisation:** Mirabet et al. 2012, [PMC3280957](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC3280957/); [HAL 03014239](https://hal.archives-ouvertes.fr/hal-03014239); the developmental noise review, [PMC4845803](https://pmc.ncbi.nlm.nih.gov/articles/PMC4845803).
- **Branching:** strigolactones and bud outgrowth, [PMC10504579](https://pmc.ncbi.nlm.nih.gov/articles/PMC10504579/); [PMC10610460](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC10610460/).
- **Leaf economics:** Wright et al. 2004, [VU](https://research.vu.nl/en/publications/the-worldwide-leaf-economics-spectrum/); sun and shade leaves, Chen et al. 2020, [PMC7300261](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC7300261/).
- **Root foraging:** Kembel & Cahill 2005, [Am. Nat.](https://www.journals.uchicago.edu/doi/reader/10.1086/431287).
- **Allocation:** [OPT review](https://revistas.unal.edu.co/index.php/actabiol/article/view/27158); Robinson 2022, [PCE](https://abdn.pure.elsevier.com/files/281226451/Robinson_PCE_OPT-ing_Out_Root_VoR.pdf).
- **Coexistence:** [Chesson's coexistence theory](https://en.wikipedia.org/wiki/Coexistence_theory); Adler et al. 2007, [PDF](https://faculty.washington.edu/jhrl/Adleretal07.pdf).
- **Growth–survival trade-off:** Baraloto et al. 2005, [Ecology](https://deepblue.lib.umich.edu/bitstream/2027.42/117031/1/ecy20058692461.pdf).
- **CSR strategies:** [overview](https://aj.tubitak.gov.tr/botany/issues/bot-10-34-6/bot-34-6-6-0912-270.pdf).
- **From memory, not re-read:** Tilman's colonisation–competition trade-off (1994) and Walter's two-layer rooting hypothesis.

This repository:

- `Reports/plant-engine-rethink-2026-09-03.md` §2.3, §6.8, §6.13 and §7;
  `Reports/plant-trainer-plan-2026-10-19.md` §13;
  `Reports/plant-phototropism-lateral-2026-08-27.md`;
  `Reports/plant-equilibrium-costs-2026-08-27.md`.
- PR 734: `Reports/brain-training/cloud-compute/gcp/` and
  `Reports/handoff/brain-lane-handoff.md`.
- The code survey of `plant.rs` (line numbers in §2.1) and the records survey:
  research agents, 2026-10-20.
