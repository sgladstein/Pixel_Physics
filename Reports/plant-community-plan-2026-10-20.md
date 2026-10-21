# Plants that grow like plants, and a bed where different kinds coexist: plan (2026-10-20, revision 4)

**Status: plan, revision 5. M1 has started. Revision 4 answered the
independent review after M0 (§12: 2 blockers, 9 major, 4 minor, all accepted).
Revision 5 records the owner's two rulings (§9), and two considerations from
the owner: the environment must give plants something to respond to (§3.4),
and training cost grows as the ladder climbs (§13).** It supersedes P2–P6 of
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

**Revision 3** records the owner's answers and treats the 2D cross-section as
a constraint up front (§2.4). The answers: keep the four roles, $40 a month,
and the lab's watering may be changed.

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
- **Answers to revision 2:**
  - keep the four roles (§4.2);
  - plants get **$40 a month** of the $100;
  - **the lab's watering may be changed** if the bed needs a water profile
    for a rooting-depth niche;
  - *"we are simulating this in a 2d cross section world … consider this
    limitation up front"* (§2.4).

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
  died). Every seed-thinning lever tried grew more grass. **The soil rising
  over them is fixed** (PR 716: only roots turn back into soil). **The seed
  piles are not** (owner, 2026-10-20).
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

### 2.4 The world is a cross-section: what that changes, up front

The lab is a 2D slice. A real plant stands among neighbours on every side and
grows into depth that cannot be drawn here. Each real-world way out below is
missing in the slice, and each one changes the plan:

| in a real stand | in the slice | what it does to this plan | how the plan handles it |
|---|---|---|---|
| **Neighbours stand all round, with gaps between them.** Light, seed and roots pass through the gaps | **Ground is a line.** Two neighbours can box a plant in completely, and one plant can close a gap | Competition is harsher and more all-or-nothing, exclusion is faster, and a rare invader needs a gap in a line | Density is judged per 100 columns, never against real-world densities. The invasion rung (6) sows into the gaps the resident bed actually has, and M0 counts them |
| **Crowns interleave in depth.** A crown has volume, about R³ | **A crown has area, about R².** The same branching is R times denser, and neighbouring crowns collide instead of interleaving | Branches meet other plants far more often than in life | **Crown shyness** already exists: a tip steers away from any plant's foliage (`plant.rs:1021`, written for exactly this). M0 measures how often a tip stops because *another* plant is in the way. The census's `boxed` reason does not yet say whose cells boxed it, so M0 adds that split |
| **Saplings grow up through gaps** in the understory, and roots of neighbours intermingle | **A tip cannot pass another plant's cell.** It must find a gap in the slice | The canopy role may be unable to get up through the understory, and neighbouring root systems wall each other off | **A6** (§3.3): thin, soft structures pass where a 3D plant would. A growing tip may push through another plant's leaf, shedding it at a cost, and a root tip may pass another's fine root. Built only if M0's count says blocking is common, and chosen by pictures |
| **Light reaches the understory** through gaps in depth and at angles. A closed forest floor gets about 1–5% of open light, open woodland 5–20% (from memory, not re-read) | **Light passes foliage only through the field's per-column transmission** (Beer–Lambert, `field.rs:325–346`) | **The shade-tolerant role exists only if a canopy passes some light.** Too opaque and nothing lives under it; too clear and there is no understory niche | M0 measures the light under one canopy plant against open light. The leaf's transmission is **calibrated to the slice**, standing for the gaps in depth that cannot be drawn. That is a calibration of the world, not a tuning of any species |
| **Seeds and litter spread over an area.** In a real stand most seed is eaten, rots, or is washed or carried off and buried within a season | **Seeds and litter stack in one column.** Seed heaps reach 13–24 deep in M0's default bed and 45–59 in the planted-balance bed, and they bury sprouts (76 of 89 covered sprouts died). **The owner, 2026-10-20: "Huge piles of seed still build up"**: the soil rising over the low plants was fixed (PR 716, roots-only), but the seed piles were not | **The piles are a defect in their own right**: on screen, and because they bury sprouts. They are also partly something the slice invents. `SEED_HEAP_ROT` (heaped seed rots fast) halved the heaps and added no species, so a heap fix is **judged by pile depth and seed lying above the soil**, not by species count; the dead-end's re-test condition is met (`dead-ends.md`, written back 2026-10-20) | **A6, as a fix, not only for realism**: seed past a set depth per column leaves the slice (joins the bank as a number); seed that lands settles into the soil (the parked `SEED_SETTLE` rule, re-measured); litter thins. Each judged on pile depth, seed above the soil and sprout burial, with a picture of the bed at its peak pile |
| **Biomass grows with height × crown area**, and costs scale with volume | **Biomass grows with height × width** | Tall and wide cost differently from life, so real-world trade-off numbers do not transfer | Trade-offs are priced from measurements in the engine (the cheat watch and the ladder's checks), never copied from real values |

**The rule this sets for every part of the plan:** read the slice as a thin
transect through a 3D stand. Any rule by which one plant affects another
(shading, blocking, crowding, burying) answers *"what would happen in the
depth we cannot see?"* and is calibrated to that, not to what the slice
literally shows.

What the slice already shows correctly stays as it is. A filled crown
silhouette is right in 2D (`dead-ends.md`, tree-architecture §7b), and so is
the look of a cross-section.

**Where it bites on the ladder.** Rungs 1–3 grow one plant alone, so the slice
costs almost nothing there. **Rungs 4–6, a neighbour, a stand and an invasion,
are where it decides outcomes.** So the calibrations above (light under a
canopy, passage, stacking) are measured in M0 and settled in A6 before those
rungs are built.

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
   (`plant.rs:13623`, `lab/rain.rs:332`; CONFIRMED and fixed 2026-10-20, §8.1 M1 step 1).
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
6. **A6: the slice's rules** (§2.4). A6 is built from M0's counts, each piece
   only if its count shows the problem is real:
   - **passage:** a growing tip may push through another plant's leaf (the
     leaf is shed, and the tip pays for it), and a root tip may pass another
     plant's fine root;
   - **seed piles** (owner, 2026-10-20: *"huge piles of seed still build
     up"*): seed past a set depth per column leaves the slice (joins the bank
     as a number), landed seed settles into the soil, and litter thins. Judged
     on pile depth, seed lying above the soil and sprouts buried, never on
     species count (the 2026-10-17 heap-rot switch showed that measure cannot
     see a heap fix);
   - **light through foliage:** the transmission is calibrated so the light
     under a canopy plant falls in a realistic band.

After Part A, **today's species are re-fitted by the trainer, not by hand.**
Their constants were tuned against dice. That re-fit is also the first real
training job.

### 3.4 The environment has to carry the signals

**Owner, 2026-10-20:** *"We need to make sure that our environment provides
the signals for our plants to respond to. If our soil bed is fully uniform
... then there is no signal for the plant to respond in interesting ways."*

**Today it is fully uniform.** `LabBox::build_counted` fills every soil cell
with one material, at field capacity, under a flat surface
(`src/lab/scene.rs:939–942`). Nutrients start equal everywhere. The only
variation is the cosmetic shade byte. Apart from lamp position and
neighbours, a root has nothing to choose between, which is the other half of
why plants differ by dice rather than by place.

**Real ground varies at every scale**, and each variation is something a
plant answers:

| signal | where it comes from in real ground | what a plant does with it | in the lab |
|---|---|---|---|
| **Texture and density** | sand, loam and clay in patches and layers: topsoil over a denser subsoil | roots push through loose soil and avoid hard; sandy patches drain and dry, clay holds water | one material everywhere. `packedsoil` and `gravel` exist as materials |
| **Relief** | hollows and rises | water and seed collect in hollows; rises dry out | flat |
| **Moisture** | relief, texture, depth, and wet and dry spells | roots follow water; drought shifts growth to roots | uniform at field capacity, then floods (§Z38) |
| **Nutrients** | litter rotting in place, old root channels, patches | roots proliferate in rich patches | uniform at the start; litter adds locally |
| **Light** | gaps in the canopy, lamp spacing | shade avoidance or tolerance; leaves to the light | lamps evenly spaced; canopy shade only |
| **Disturbance** | animals digging, falls, deaths | gaps for colonists | ants, plant deaths |

**So M1 adds an environment step (E), right after the watering**, and builds
it the same way as everything else:

- **Patches of soil texture and density, and gentle relief.** Smooth noise
  from the world seed, so one seed is one ground, and different seeds give
  different grounds. Uses existing materials where they fit (`packedsoil`
  for a dense subsoil, a loose loam).
- **Starting nutrients in patches.**
- **Wet and dry spells** (the watering step) acting on that ground, so
  moisture varies by place, depth and time.
- **Measured as signal, not looks:** for each signal, the spread across the
  bed (per-column moisture, nutrients, density, surface light), and **a clone
  test**. The same genome planted on a rich patch and a poor one, a wet
  hollow and a dry rise, must differ, in the direction a real plant would and
  by more than its dice. That is §3.2's "surroundings" number, and it is what
  makes it non-zero.
- **It changes how the lab looks**, so the owner sees it before it ships, as
  a lab bed option first and the default after the owner's go.

**The 2D rule still applies** (§2.4). A patch in the slice is a patch in the
3D ground the slice cuts through, so patches are wide enough for a plant to
feel, not single-cell noise. A coarse-field read would turn single-cell noise
into nothing anyway (`.claude/rules/src-sim-cells.md`).

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
| **Rooting depth** | shallow fibrous roots that take rain first ↔ deep roots that reach stored water (Walter's two-layer idea for grasses and woody plants) | grass shallow, shrub deep | **unknown**: the bed starts at field capacity and the mister wets the top. **M0 measures the soil-water profile over a long run.** If water never runs short at depth, the lab's watering is changed so it has a profile (owner, 2026-10-20). For example, a light mister over a deeper store that recharges slowly. Measured before and after, and shown |
| **Being eaten** | defended and slow ↔ palatable and fast | the ants' choices shift the balance | partly: ants eat mostly dead leaf |

### 4.2 The roles: four, not nine

**Proposed** (the owner may cut or add one; §9). Each existing species is a
starting point, not the answer:

| role | ecological need it fills | trade-off that keeps it from winning everywhere | starts from |
|---|---|---|---|
| **Canopy perennial** (competitor) | takes the light at the top, holds ground for a long time | slow to establish, needs bright light, few seeds, pays for wood | tree or conifer (revised after the review: they persist on 10/12 beds and shade the soil to 0.31) |
| **Shade-tolerant understory** (stress-tolerator for light) | lives under the canopy on dim light | slow, low top speed, loses in the open to the fast | shrub, once A3 fixes its stall and A5 gives shade tolerance a price (revised; was creeper) |
| **Annual colonist** (ruderal) | fills gaps fast after death and disturbance; keeps a seed bank | short life, loses any long contest, needs open ground | herb |
| **Clonal ground cover** | spreads sideways, holds open, bright ground near the surface | shallow-rooted, poor in shade, few seeds | creeper, a ground runner (revised; grass spreads only by seed and sets the most of it) |

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

**Budget.** The $100 a month is shared with the ant lane; **plants get $40 a
month** (owner, 2026-10-20). At spot
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
| **M0** | **Baselines and feasibility, no engine change:** the development report (§3.2) for herb, shrub and grass; mix7 and a four-role bed, 12 seeds × 450,000 frames, with and without ants; **the soil-water profile over a long run** (§4.1); **the slice's counts** (§2.4: tips stopped by another plant, split from those stopped by their own plant; light under one canopy plant against open light; seed and litter depth per column; gaps per 100 columns); the cost of a small arena by size and length; a copied bed starting the same as its original | pictures and numbers in the chat; §3.2's bars set; §6.2's costs measured; the watering decision made from the water profile; A6's pieces chosen by the slice's counts |
| **M1** | **Part A**, A1–A6, one PR each, and the lab's watering if M0 calls for it | each meets its bar, with strips shown |
| **M2** | rungs 1–3 as `plantrain` modes, with station sets; the rung-predicts-rung check; the four roles re-fitted to the new engine | each re-fitted role completes its life in its own stations at least as well as the shipped species did on the old engine |
| **M3** | the launcher runs plant jobs (new session, with the key) | an M2 job reproduces byte for byte on Google Cloud |
| **M4** | rungs 4–6, the crippled-species known answer, and training rounds | all four roles invade each other from rare |
| **M5** | rung-7 validation with and without ants; the owner's GIF; promotion of the files the owner picks; README, wiki and freshness notes | the owner's pick |

M0 and M1 run here, with no Google Cloud needed.

**M0 is done (2026-10-20):** `plant-community-m0-2026-10-20.md`. After it and
the review (§12), **M1 runs in this order**:

1. **A1 first: confirm or fix the shared generator.** The leaf-litter shade
   and the rain mister may draw from one stream. If they do, changing the
   watering reshuffles every plant's dice, and no before-and-after
   comparison can be attributed. **Done, 2026-10-20.** Confirmed, and wider
   than inferred: the mister's drop positions, the shade the brush gives
   each drop (drawn only for a drop that lands, so where plants stand moved
   the stream), every litter and pip cell's rot roll in `decay::tick`, and
   the litter shade all drew from `World::rng`. A plant's own growth dice
   were already keyed (`rng::stream`). **Measured** (`plant_m0 mode=nudge`,
   roles4, load failure off): one discarded draw at frame 1,000, nothing
   else changed. Before: 4 cells differ that frame, 20,899 cells in 504 of
   512 columns by 60,000, herbs 7 against 32. After keying all four on
   (seed, place, frame): identical for all 60,000 frames; the control copy
   without the nudge identical in both builds. Guard:
   `a_draw_from_the_shared_world_stream_does_not_reach_the_bed`, red with
   the mister or the decay roll put back. **It changes every bed's dice
   once**, as any reshuffle does, and that exposed one real bug:
   `shed_cut_off_tissue` read a plant whose only drinking root was shed
   earlier in the same tick as a plant cut in two, and shed it at the cap
   tick after tick (a seedling, traced). Exempt now, as a plant with no
   drinking root already was; test
   `a_plant_whose_only_drinking_root_went_this_tick_is_not_read_as_cut`.
2. **Watering**: wet and dry spells replace the steady mister (§Z38), judged
   on ≥4 seeds by:
   - standing water;
   - the soil-water profile per column under each species' roots;
   - a shallow-rooted and a deep-rooted plant that actually differ in water
     status.

   **Done, 2026-10-20.** The flood's root cause was not the rate: the box
   conserves water and the mister makes it, so a box with no sink can only
   fill. **A floor drain** (`LabBox::drain`, on by default in every lab
   bed) lets the bottom soil row give up what it holds above field
   capacity. **Spells** (`LabBox::spells`) are 20,000 wet frames at
   `Steady` and 40,000 dry, vented, which is the same mean rain as `Light`.
   Spells are off in the played box, where the owner ruled drought is the
   player's to cause, and **on in the training world**. Four seeds, 480,000
   frames, load failure off:

   | at 480,000 | mister, no drain | mister + drain | drain + spells |
   |---|---|---|---|
   | standing water, per seed | 7,187-8,692, rising | 900-1,650, level | 311-1,121 after dry, 1,670-2,067 after wet |
   | soil water, top -> floor | 0.95 -> 0.97 | 0.63 -> 0.62 | 0.62 -> 0.62 |
   | water at grass roots (shallow), after dry / wet | 0.77 / 0.78 | 0.58 / 0.59 | **0.52 / 0.61** |
   | at shrub roots (deep) | 0.83 / 0.82 | 0.81 / 0.81 | 0.80 / 0.81 |
   | grass plants | 622-890 | 1,576-1,709 | 896-1,276 |
   | herb plants | 0-7 | 0-4 | 0-1 |

   **What it shows.**
   - Only the spells make a shallow and a deep root live through different
     weather.
   - The drain alone doubles grass, and with it the seed mounds (A6's job).
   - Herb fails in every arm, so its failure is not the water.
   - After the first cycles the top band barely dries in a dry spell
     (0.62-0.64). The signal is at the shallow roots, not in the bulk soil.
     A longer dry spell would deepen it, and the training world can set one.
   - **Still open:** pools stand over dense root masses through a whole dry
     spell, because soil water moves only between soil cells and a root mat
     is a floor to it. Real water runs down beside roots. This is a
     candidate for step E.
   - Frame cost of the drain, paired: within +/-3% both ways.
   - Pictures: `Reports/plant-community-2026-10-20/m1/watering-s1.png`.
3. **E: the environment carries signals** (§3.4): texture and density
   patches, gentle relief, nutrient patches, shown to the owner as a lab bed
   option. Judged by the spread of each signal and by clones that differ by
   place more than by dice.

   **Built, 2026-10-21: `Ground::Patchy`, a lab bed option, off by default.**
   - **Texture with no new materials.** Soil already ships three colour
     families: loam, a dark rich wetland soil and a pale dry one. A cell's
     family is now its texture (`material::SoilTexture`). It is data on the
     cell, it moves with the cell, and it is drawn already, so about 170
     name-keyed `id_of("soil")` rules need nothing new.
   - **What each texture does** (`World::soil_texture`):

     | texture | water held (field capacity) | drainage | nutrient share | rooting cost |
     |---|---|---|---|---|
     | sand | 470 | 2.5x | 0.4 | 1.0 |
     | loam | 620 | 1.0 | 0.75 | 1.0 |
     | rich clay-loam | 740 | 0.4 | 1.0 | 1.3 |

     Water evens out by wetness relative to each cell's own capacity, not by
     amount (`update::loam_level`).
   - **Water runs down beside roots and stones** (`World::soil_drains_around`).
   - **The bed lays the textures out** in patches tens of cells across, with
     hollows up to 8 deep whose top rows are rich.
   - `Ground::Sandy/Loam/Rich` fill a bed with one texture, as the clone
     test's arms.
   - Off, nothing changes: a uniform bed reproduces the step-2 run row for
     row.

   **Measured** (4 seeds, 480,000 frames, drain and spells on):
   - **Moisture signal.** Spread of soil water across columns: patchy
     **0.09-0.12 at every 10-row band**, all run. Uniform: 0.13-0.20 in the
     top band, 0.06-0.08 in the next, 0.00-0.03 below that.
   - **Nutrient signal.** The nutrient share's spread is 0.20-0.22, against
     0.08-0.12 on uniform ground. The plant's response curve saturates
     (Km 0.05), so the share bites mainly where soil is spent. The patchy
     bed's mean share is 0.69 against 0.94, worth about 2% of availability.
   - **Water at shallow (grass) roots** after a dry spell: 0.46 on patchy
     ground against 0.52 on uniform.
   - **Deepest roots:** shrub 9.1 rows against 5.8; herb 7.9 against 0.
   - **Standing water** after a wet spell: 1,095-1,750 against 1,670-2,067.
     The pool over the shrub's root mat is gone.
   - **Herb** persists and recruits on 1 of 4 seeds (7 plants), against 0-1
     plants on uniform ground. Other kinds are within seed spread.
   - **Clone test** (`plant_m0 mode=plastic axis=ground`; one founder under a
     lamp, 8 sets of dice, 60,000 frames):
     - On rich ground, compared with sand, every one of the four species
       builds a smaller, shallower root system: roots -1.6, -0.6, -1.1 and
       -1.0 spreads for creeper, grass, herb and shrub. That is optimal
       partitioning's direction.
     - The noise floor (loam against loam on other dice) reaches 1.3
       spreads, so **the signal is real but no bigger than the dice yet**.
       Shrinking the dice is A2 and A3's job.
   - **Frame cost**, paired on one thread on a quiet machine: **+7.2%**
     (2,253 -> 2,415 us/tick). Texture is 5.5% and drain-around 1.6%. The
     cost is the signal itself: more water moving.
   - **A first version** evened water out by amount across texture edges.
     Clay fed sand and sand drained it, for ever: soil-water changes went
     988 -> 2,521 a tick. Fixed before the numbers above; guarded by
     `sand_and_clay_each_at_field_capacity_exchange_nothing`, red with the
     old comparison put back.
   - **Owner's call:** patchy ground in the played lab. It changes how the
     soil looks, for +7% frame cost. The training world uses it either way.
   - Data and picture: `Reports/plant-community-2026-10-20/m1/ground/`,
     `.../m1/ground-s1.png`.
4. **Re-baseline the community** with in-bed heredity truly off. That means
   `mutation_off_for_measuring` plus founder variants at 0, printed in the
   log header (M0 left per-birth jitter and founder variants on; §12, M7).
   Also count contact split by seed, own kind and other kind, and light under
   each species' own leaf.

   **Done, 2026-10-21.** This is the training world: the four roles,
   patchy ground, the floor drain, spells (20,000 wet / 40,000 dry), load
   failure off, and all four heredity switches off, printed in each log's
   header. 4 seeds, 480,000 frames, `plant_m0` (`heredity=on` restores
   M0's bed). **This is the baseline A6, A3, A5 and A2 are compared
   against, paired by seed.**

   **A grass sward with a few creepers and shrubs.**
   - grass about 1,000 plants (959-1,157), still recruiting;
   - creeper 5-21 plants;
   - shrub 1-3;
   - herb dead on all four seeds;
   - 2-3 kinds with 100+ leaf cells.

   **The heaps:** 5,468-8,170 waiting seed above the soil, the deepest
   30-37 cells.

   **Contact, split (review M6).** 81-84% of growing tips touch another
   plant. **But that is a waiting seed (57-63% of tips) or the plant's own
   kind (53%); only 1.5-7% touch another species.** What crowds a tip in
   the slice is heaped seed and kin, not competition between kinds, so
   passage (A6's other piece) is not called for by this count.

   **Light at each species' own leaves**, as a fraction of a lamp's full
   light (raw: the lab has no daily cycle): creeper 0.53, grass 0.47, herb
   0.47, shrub 0.43.

   Data: `Reports/plant-community-2026-10-20/m1/rebaseline/`.
5. **A6, where a seed may germinate, and the piles**:

   **Done, 2026-10-21: heaps capped at 2 deep, on by default in every lab
   bed** (`LabBox::seed_pile_depth`, `World::seed_pile_depth`,
   `plant::seed_pile_below`, `DeathCause::Spread`).
   - **"Only on soil" was already the rule.** The germination gate wants a
     water-holding cell directly under the seed. So A6 came down to the heaps.
   - **The heaps are all living seed.** On the re-baseline, every seed cell
     above the soil is a waiting seed organism (9,009 on one bed). A waiting
     seed resting on a heap at least 2 deep is now taken to have landed
     beside the slice, and leaves it.
   - **Paired against the re-baseline** (4 seeds, 480,000 frames, training
     world):

     | | re-baseline | depth 2 | depth 4 |
     |---|---|---|---|
     | waiting seed above the soil | 5,468-8,170 | **486-934** | 1,032-1,841 |
     | deepest heap | 30-37 | 7-8 | 9-11 |
     | kinds with 100+ leaf, per seed | 2,3,3,2 | 3,4,3,2 | 2,3,4,2 |
     | grass plants | 959-1,157 | 325-562 (bigger) | 422-793 |
     | herb plants | 0-1 | 0-26 (one seed recruits) | 1-20 |
     | tips touching seed / blocked upward | 57-63% / 38-45% | 42-48% / 19-29% | 44-55% / 25-38% |

   - **Species-mix guard:** grass's share of leaf is 67-90% at depth 2,
     against about 92% before. There's no ratchet.
   - **Depth 2 ships:** the complaint is the heaps, and 2 halves them
     against 4. Herb's extra seed at 4 is within what 4 beds can tell apart.
   - **The bank "that ants can still reach"** is not built: seed that leaves
     the slice is gone, counted (`seeds_spread`, 141,000-308,000 a run). Seed
     on the ground and the buried bank are untouched.
   - **Outdoors** the rule is off until measured there.
   - Picture: `Reports/plant-community-2026-10-20/m1/a6-s1.png`.
   - a seed sprouts only in contact with soil, as a real seed does;
   - seed past a set depth leaves the slice into a bank that ants can still
     reach and that the census counts;
   - a species-mix guard, because removing the pile may bring back the grass
     ratchet.

   Runs with load failure off (owner, 2026-10-20).
6. **A3, bud break reads stored carbon** (the shrub trace), and **lifespans
   for shrub and creeper** (owner, 2026-10-20).

   **Done for A3 and the shrub, 2026-10-21; the creeper is held for the
   owner.**
   - **A3** (`World::bud_break_reserve`, on by default in every lab bed): a
     plant with no growing tip and four steps' carbon in store breaks one
     bud. Before, buds were paid for by this tick's income alone, so a
     plant that lost every tip could never earn one back.
   - **Not the rejected "stock in the numerator"** (`dead-ends.md`: the
     stand fused into one canopy). This never sizes the crown from the
     store; it gives a tipless plant one tip.
   - **Clone test** (lone shrub, 16 dice per ground, 60,000 frames): size
     233+-419 -> 777+-404 cells on loam, coefficient of variation 1.8 ->
     0.5 on every ground. The shrub stops being hit or miss. With the dice
     shrunk, rich against sandy ground moves shrub roots by -2.05 spreads
     against a 0.34 noise floor. **That is §3.4's clone test passing for
     roots.**
   - **Community** (training world, heaps at 2, 4 seeds, 480,000 frames),
     against heaps alone:

     | | heaps only | + A3 | + A3 + lifespans (shrub 45k, creeper 40k) |
     |---|---|---|---|
     | kinds with 100+ leaf, per seed | 3,4,3,2 | **4,4,4,4** | 2,3,3,2 |
     | herb plants (recruits, last 100k) | 0-26 (0-20) | 8-18 (3-9) | 0-35 (0-39) |
     | shrub plants | 0-7 | 4-18 | 2-13 |
     | creeper plants | 12-24 | 8-30 | 0-18 |
     | grass leaf | 1,643-2,930 | 1,297-2,531 | 1,805-3,017 |

     **A3 is the first bed where all four roles last the run on every
     seed.** There is no runaway: shrub leaf stays at or under 1,030, and
     grass falls.
   - **Lifespans.** The shrub's ships at 45,000 plant-time frames
     (`shrub.ron`); it holds 2-13 plants and recruits on 2 seeds.
   - **The creeper's is held.** At 40,000 it dies out or nearly so on 3 of
     4 seeds. A creeper organism is its whole clone, so a lifespan kills
     the clone, where a real clone renews through new ramets. **Owner's
     call:** a much longer creeper lifespan, or ramets that root and become
     their own plants first.
   - Picture: `Reports/plant-community-2026-10-20/m1/a3-s1.png`. Data:
     `.../m1/a3/`.
7. **A5.1–A5.2: shade avoidance against tolerance, sun and shade leaves.**
   These make an understory possible at all, and are priced.
8. **A2, development keying.**

**Rungs 4–6 and M4 wait** for three things:

- a resident bed that holds steady without the candidate;
- role-specific scoring (§5, revised in §12);
- a repeatability and heritability gate on the community rungs.

The light arm of the development instrument is redesigned with a real shade.

### 8.1 M0, item by item, each with its control

Every number below is checked against a case known to be fine
(specificity) and a case known to be broken (sensitivity), as `CLAUDE.md`
requires. A measurement whose control fails is not reported.

| item | measures | instrument | known-fine case | known-broken case |
|---|---|---|---|---|
| **M0.1 dice** | size, height and width spread of clones: spot arm (today's key and the per-plant key), and a **seed arm** (one spot, the per-plant key, N developmental seeds) | `clone_variance shift=1`, plus a new `seeds=` arm | the same seed in the same spot twice reads exactly zero | the seed arm reads non-zero under the per-plant key: different seeds must give different plants |
| **M0.2 surroundings** | each trait's shift from bright to dim (under a lamp ↔ between lamps) and wet to dry (mister on ↔ off), divided by the dice spread; herb, shrub, grass | `examples/plant_m0.rs mode=plastic` | two arms differing only in world seed read inside the noise | bright against dim must move size (P1: herb flowered 7/8 under a lamp, 4/8 between) |
| **M0.3 genes** | heritability of size, width and leaf share | `clone_variance`'s existing pop, clone and spread arms; P1's `plantrain mode=heritability` for herb | — | the spread arm, the two most distant genomes, must read high |
| **M0.4 community** | per species: plants alive (not waiting seed), leaf cells, seedlings in the last 100,000 frames, leaf-share trend over the second half; species with 100+ leaf; evenness. mix7 and the four-role bed, 12 seeds without ants and 8 with, 450,000 frames | `examples/plant_m0.rs mode=community` | two runs of one seed are byte-identical | a species crippled at sowing (photosynthesis ×0.1) must read as lost |
| **M0.5 water** | soil water by 10-row band below the surface, over time | same harness | — | a bed with the mister off reads drier than one on Light |
| **M0.6 the slice** | (a) growing tips with no free neighbour, split by whose cells box them (own or another plant's); (b) light at the soil surface under plant cover against open columns; (c) seed and litter depth per column; (d) gaps (3+ bare columns) per 100 columns | same harness | (a) a lone plant reads zero "boxed by another" | (a) two plants sown two columns apart read non-zero; (b) a column under solid rock reads near zero |
| **M0.7 arena cost** | core-seconds per bed by width and length, one thread | `mode=cost` | — | cost must scale with area × frames, or the §6.2 table is wrong |
| **M0.8 copying a bed** | a grown bed, copied, then stepped on: identical to the original; then a new species registered into the copy and sown: it germinates | `mode=copy` | copy and original are hash-equal after 5,000 more frames | — |

## 9. Questions

**Answered (2026-10-20):**

- the four roles are kept;
- $40 a month for plants;
- the watering may change;
- the 2D cross-section is handled up front (§2.4);
- Part A ships on for every plant.

**Answered, 2026-10-20, after the review:**

- **Plant load failure is off** in the world the community is trained in, as
  the owner plays. Rung 7 is still validated under both settings.
- **Shrub and creeper get lifespans**, measured and shown before they ship.

**As asked** (kept for the record):

1. **Plant load failure, in the world the community is trained in** (B2).
   With it on (the lab's default), grass seedlings that sprout on the seed
   pile are felled and grass dies out. With it off (your setting), grass
   carpets the bed and its pile never clears.
   - **Recommendation: off**, the way you play. Felling a seedling because
     it sprouted on loose seed is not something a real plant suffers, and the
     pile problem is visible there, so a fix can be judged where it shows.
   - Rung 7 is validated under both either way.
2. **Shrub and creeper never die of old age** (B1); they have no lifespan.
   Without one, the residents of a bed can't be displaced and there's no
   room for the colonist role.
   - **Recommendation:** give both a lifespan, the way grass got one on
     2026-10-19, with the value set by measurement and shown to you before
     it ships. It changes two shipped species in both games, so it's yours to
     decide.

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
- **The slice makes plant-to-plant effects harsher than life** (§2.4). Two
  neighbours box a plant in, a canopy can block all light, and seed stacks.
  Answered by measuring each effect in M0, and by A6 calibrating it as the
  depth we cannot see, before rungs 4–6 rely on it.
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


## 12. Independent review after M0 (2026-10-20) and responses

One adversarial reviewer read this plan and the M0 report in full and checked
the cited code. Every finding below was checked against the code before
being accepted, and each one is accepted. The changes are already in M1's
order above and in §9.

| # | severity | finding | response |
|---|---|---|---|
| B1 | BLOCKER | The invasion test cannot mean anything against today's residents. They decline (median 0 kinds still producing new plants), cannot be displaced (shrub and creeper never die of age: no `life_half_life`, checked), or are under water. Grass's ~60,000-frame lifespan is shorter than the 80,000-frame window. | **Accepted.** Rung 6 is gated on a resident-only copy over the same window showing no trend, and the invader is scored against that paired control. The line is counted in established plants, never organisms (seeds are organisms). ≥4 resident beds, so no single gap layout is fitted. Perennial lifespans for shrub and creeper go to the owner (§9). |
| B2 | BLOCKER | The ladder's world is undefined on a switch that decides the outcome: load failure on means grass is extinct, off means grass carpets the bed with a pile that never clears. | **Accepted.** Owner ruling (§9). Training runs in the chosen setting, and rung 7 is validated under both. |
| M3 | MAJOR | A score ordered so that recruitment outranks everything, over 3,000–40,000-frame windows, pushes every role towards the fast annual. Rung 2 rewards growth rate and has no neighbour, so shade avoidance cannot be learned there. A rung-predicts-rung rule would punish the very trade-offs the roles exist for. | **Accepted.** Each role is scored on its own objective at the horizon its strategy pays: the canopy on light captured and persistence; the understory on carbon balance and survival in the dim station; the colonist on seedlings per frame in a disturbed station. The rung check runs per role, and a negative correlation for a trade-off role is information, not a reason to redesign. Rung 2 gets neighbour and shade stations. |
| M4 | MAJOR | Three starting species contradict their roles. Creeper is a ground runner, not shade-tolerant. Grass spreads only by seed (tillers regrow inside one organism) and set 20,188 seeds, the opposite of "few seeds". Shrub is the stall lottery, while tree and conifer persist (10/12) and make shade (0.31). | **Accepted. Remapped:** canopy perennial from **tree or conifer**; clonal ground cover from **creeper**; annual colonist from **herb**; shade-tolerant understory from **shrub**, after A3 fixes its stall and A5 gives shade tolerance a price. Grass stays in the game, outside the trained four. |
| M5 | MAJOR | "Shrubs pass 86%" is an instrument artifact. The bed with the shrub crippled out reads the same (0.919 against 0.889). "Canopy" counted any organism cell, waiting seeds included, and rows were averaged across the flood. | **Accepted; the 86% figure is withdrawn.** Light is re-measured per species, under that species' own leaf, excluding seed organisms, before any flood. |
| M6 | MAJOR | The contact counts behind A6's passage rule count seeds, kin and water. | **Accepted.** Contact is split into seed, own kind and other kind, and leaf against stem, and recounted after the watering fix. If passage is built, it is priced (a tall plant shedding the understory's leaves is not free), shipped as a runtime selector, and judged by pictures. |
| M7 | MAJOR | M0's community beds ran with per-birth jitter (σ 0.08) and founder variants (0.25) still on, and the traces ran with mutation off: two different worlds. | **Accepted.** Re-baseline in M1 step 3. M0's community numbers stand as "today's bed, heredity partly on", and the M0 report says so. |
| M8 | MAJOR | Nothing checks that community-rung scores are heritable. P1's blocker returns at rungs 4–6, and invasion from a few seeds into about 2 gaps per 100 columns is mostly luck. | **Accepted.** Before M4: rung 6's repeatability (same genome, K seed draws, ≥4 resident beds) and its heritability, both gated. The copy test is re-run on a real resident bed at 300,000+ frames, with ants. |
| M9 | MAJOR | The only evidence for the annual role depends on ants, which the training rungs leave out, and it was read from a count rather than traced. | **Accepted.** Herb is traced individually in both arms first. Then a disturbance station, or ants in rungs 5–6, priced at about 2× cost. |
| M10 | MAJOR | "Water is never short anywhere" was read from band means across the full width, including the bare colony band. Mister off was one seed, and intermittent watering is unmeasured. | **Accepted.** Water per column under each species' roots. Watering judged on ≥4 seeds, including a shallow- against deep-rooted plant, before rooting depth counts as a role axis. |
| M11 | MAJOR | The M1 order misses what M0 found: recruitment failure. A1 is unconfirmed and would confound the watering change. "Germinate only on soil" is missing. Moving seed into a numeric bank hides it from ants and favours the heaviest seeder. | **Accepted.** That is the new M1 order above. |
| m12 | MINOR | The cost table is off: rung 6 on a full 512-wide bed is about 96 core-seconds per 80,000 frames, not 25, and rung 3 runs at about 50 generations an hour, not 15. | **Accepted.** §6.2 is corrected below. $40 a month still covers it. |
| m13 | MINOR | The seed and seedling caps were measured on herb; grass saturates them at once. | **Accepted.** Caps are set per role. |
| m14 | MINOR | A6's light-transmission calibration has no evidence behind it and would re-baseline every species. Trees already pass 0.31. | **Accepted.** Dropped unless the per-species light measure calls for it. |
| m15 | MINOR | Several conclusions rest on one seed. | **Accepted.** The n is cited beside each claim in the M0 report. |

**Corrected costs (m12)**, for 12 genomes × 8 stations a generation on a
16-core machine:

| rung | per bed | generations an hour |
|---|---|---|
| 1 | ~0.1 core-s | thousands (overhead-limited) |
| 2 | ~0.8 core-s | ~700 |
| 3 | ~12 core-s | ~50 |
| 6, full 512-wide resident bed | ~96 core-s per 80,000 frames | ~6 |

Rung 6 on a narrower resident bed (192 wide) is roughly 2.5× cheaper, if the
rung check shows it predicts the full bed. Estimates; pinned when the rungs
are built.


## 13. The training-cost problem: harder behaviours, slower generations

**Owner, 2026-10-20:** *"As we progress through our training rungs, we are
trying to train more complex behaviors (which likely require more
generations of training) at the same time our generations/hour decreases as
we need longer runs in larger, more complex ... environments. This might lead
to an exponential problem."* It affects the ant line as much as this one. It
is not solved here; this section is the thinking and the instruments that
will tell us early.

**Why it is real.** Two curves cross:

- generations needed rise with what a rung asks;
- generations per hour fall from about a thousand at rung 1 to about 6 at
  rung 6 (§12's cost table).

At $40 a month (about 180 machine-hours of a 16-core spot VM), a rung needing
500 generations at 6 an hour is 80 hours: half a month for one rung, with no
retries.

**The levers, roughly in order of how much they buy:**

1. **Do not train at the top.** Train each piece where it is cheap, and let
   the top rungs only *select* among a few finished candidates. The ant line
   learned this: the forager was trained in small arenas, and the colony was
   never the training ground. Here, rungs 1–3 (per species, per role) do
   nearly all the generations. Rungs 5–6 run a handful of generations on a
   short list, and rung 7 runs none: it validates.
2. **Warm starts.** Each rung starts from the last rung's champions, never
   from scratch, so a rung's generation count is the cost of the *new*
   behaviour only. Measured per rung: generations to plateau from a warm
   start against a cold one.
3. **Shrink the expensive arena until its ranking breaks.** M0 found cost
   tracks activity, not area. A rung-6 resident bed 192 wide instead of 512
   is about 2.5× cheaper. Keep shrinking while the rung-predicts-rung check
   holds, and stop where it does not.
4. **Grow once, copy many** (M0.8). A pre-grown resident bed is paid once per
   round, not per candidate. Only the window after the copy is paid.
5. **Racing, and stopping early.** Score candidates at a quarter of the
   window and drop the clearly worse ones before paying for the rest. A
   candidate already dead or flooded is not run to the end. Validated like
   any proxy: the early ranking must agree with the full one.
6. **Fewer, larger steps where heritability allows.** P1 found 1–3-gene
   steps invisible and 8-gene steps visible. After Part A shrinks the dice,
   small steps may become visible, which cuts the generations per behaviour.
   Re-measured, not assumed.
7. **Train one species at a time, and only re-score what changed.** In a
   community rung, the residents' own outcomes on a fixed copy are paid once.
8. **A cheaper model of the expensive rung.** If a rung's outcome can be
   predicted from cheap measurements (rungs 1–3 plus a few trait numbers)
   well enough to rank candidates, it becomes a filter. Kept only while it
   ranks as well as the rung on held-out candidates.

**The instrument that tells us early: a compute budget per rung.** Each rung
gets a budget in core-hours, written down before it runs. It also gets a
plateau test: the champion's score on the monitoring hold-out has not improved
for N generations. A rung that hits its budget before its plateau is not run
longer. It is redesigned: split into smaller rungs, given a cheaper arena, or
given a warm start. Every round logs generations-to-plateau and core-hours
per rung, so the curve of "how much does each rung really cost" is measured
as the ladder climbs, not guessed. **If that curve bends upward faster than
the levers above can flatten it, the plan says so early,** at rung 3 or 4,
rather than discovering it at rung 6 with the budget gone.

**Shared with the ant line.** The same budget-per-rung and plateau log fits
the ant trainer as is. When this section is built, it is offered to that lane
as a shared tool, not a plant-only one.
