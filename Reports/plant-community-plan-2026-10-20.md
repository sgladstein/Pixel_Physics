# Plants that grow like plants, and a bed where many kinds thrive: plan (2026-10-20)

**Status: plan, awaiting the owner's decisions in §8. Nothing here is built
yet.** It supersedes P2–P6 of
[`plant-trainer-plan-2026-10-19.md`](plant-trainer-plan-2026-10-19.md); that
plan's P1, the trainer core, is on `main` (PR #731) and is reused here.

## 0. In one paragraph

The goal is a lab bed where many different kinds of plant live together and
keep thriving over a long run. The trainer gets there in small, measured
steps, each one a species file that makes the bed more diverse without
pushing another kind out.

That cannot start yet, for one reason measured below: **in today's engine
most of what makes one plant differ from another is dice.** Clones of one
herb planted a column apart range 134–387 cells. Selection on noise selects
luck, and every small step would be lost in it.

So the plan has three parts, in this order:

- **A. Make plants develop like real ones.** Genes and surroundings decide
  what a plant becomes, and dice only add a small individual touch. This is
  engine work, done before any training.
- **B. Define "a diverse bed that thrives" as a number, and measure today's
  bed.** The number stays graded, never pass/fail.
- **C. Train species one small step at a time towards it.** Long beds run
  on Google Cloud, launched from a new session that has the credentials.

## 1. Rulings this plan is built on

**Owner, 2026-10-20:**

1. **Development (was D2), reframed.** Plants should resemble real plants:
   environment plays a role and genetics play a role. Roots grow towards
   water, leaves towards light, and less light gives bigger leaves. *"If
   cloned plants all look very different even in almost identical
   conditions then nothing we are doing matters."* Today *"most of the
   variation comes from randomness … and the differences we do see seem
   unrelated to the environment."*
2. **Colour (D1): agreed, with limits.**
   - A species may carry colours of its own.
   - Each colour is built from one base colour with the existing 4-tone
     shading.
   - Each is kept within a natural range for its plant part.
   - None ships without the owner's pick.
3. **Not looks yet.** The goal is *"a diverse bed of different types of
   plants that thrive over a long run"*, and *"each training step should be
   a small step towards that goal."*
4. **Compute.** Google Cloud is available. It needs a new session with the
   credentials, since this one has none.
5. **Images go in the chat**, or in a report readable in the chat, for now,
   not through the review queue.

**Standing rulings that still bind:**

- **Features ship on** unless a harm is measured (`CLAUDE.md`). A change that
  alters looks waits for the owner's eye.
- **An outcome is a distribution, not a binary** (`CLAUDE.md`'s ethos). A
  plant "thriving or gone" is the defect.
- **"Give me the tools… a wrong default is reported, not tuned"**
  (`Reports/lanes/evolution-lab-coordinator.md`). The trainer is the tool:
  it finds species files, and the owner promotes them.
- **The trainer never overwrites a shipped species.** A trained species is a
  new file beside it, promoted by the owner (trainer plan §1).
- **A bed is judged over a few hundred thousand frames, as an order
  statistic over seeds** (lab coordinator note).

## 2. Where things stand, measured

### 2.1 Development: mostly dice

| what | number | source |
|---|---|---|
| One herb genome, 8 spots one column apart, 12,000 frames, today's key | **134–387 cells**, 29–67 rows tall (size CV 0.41) | `clone_variance shift=1`, 2026-10-20 |
| The same, with dice keyed to the plant (`Plant{coarseness:0}`) | **128–142 cells**, 30–31 rows (CV 0.04) | same run |
| Herb size heritability, today / per-plant key | **0.034 / 0.650** (crown width 0.227 / 0.658) | `plant-engine-rethink-2026-09-03.md` §6.8 |
| Tree clones one column apart **with** the per-plant key | still diverge from frame 1,500–2,550, reaching 151–3,368 cells at 20,000. The amplifier was not identified | rethink §6.13 |
| Trainer children 1–3 genes from the herb | heritability of score ≤ 0.21 at every step size, so selection is blind to small steps | trainer plan §13 |
| A bed that reseeds itself | size heritability **0.000**: offspring crowding hides it | review card `ad25be` |

**Where the dice are** (from a code survey of `src/sim/plant.rs`,
2026-10-20; ranked by estimated effect, the ranking INFERRED). Four rolls
change a plant for good:

1. **The taper roll** (`plant.rs:5863`): whether a tip near its turgor bound
   extends this tick. Four misses in a row retire the axis, so a lateral's
   length is a lottery.
2. **The step sample** (`:6328`): which neighbour a tip grows into, sampled
   in proportion to score. A vertical tip steps diagonally 59% of the time.
3. **Leaf placement** (`:6806`, `:6915`): which side a leaf goes. Crowding
   then steers later growth away from it, and its light decides which bud
   breaks.
4. **Bud kill** (`:14520`): thickening kills each neighbouring bud with
   chance 0.5. For a herb, buds are its only branches.

All are drawn from `(organism, x, y, world frame)` (`:2278`). With the
per-plant key they are drawn from `(dev_seed, x, y, age)`, which is still
keyed by **age**. So a clone that falls one tick behind re-rolls everything
after: the likely source of §6.13's unexplained divergence (INFERRED).

The survey also reports that the leaf-litter shade and the rain mister draw
from one shared generator (`plant.rs:13623`, `lab/rain.rs:332`). If so, one
fallen leaf moves every later raindrop (INFERRED; to be confirmed in A1).

**What the plant senses today** (same survey):

| sense | response | state |
|---|---|---|
| light | income per leaf, graded; the strongest response there is | live |
| light, direction | **up or nothing** (`organism::phototropism_dir`), so a plant cannot lean towards a lamp. Measured: light steering as shipped, off, or ×4 gives the same plant to the cell through frame 8,000 | effectively inert |
| water, soil | roots switch to the wetter side once a gradient passes 0.05 | an on/off switch, not a lean |
| water status | income and root share shift with drought | live, graded |
| crowding | steers a tip away | live, steering only |
| nutrients | whole-plant income and price | live, never spatial |
| wind | weight 0.03; the lab has none | inert in the lab |

**Missing:**

- shade leaves (bigger leaves in low light);
- shade avoidance (longer, thinner stems in shade);
- root:shoot balance responding to light;
- roots foraging into wet or rich patches;
- a sideways lean to light. That patch exists but is withdrawn
  (`plant-phototropism-lateral-2026-08-27.md`).

`World::plasticity` is for ants only.

### 2.2 The community today

- **The bed of record ("mix7")**: herb, scrambler, shrub, creeper, grass,
  conifer and tree. Light rain, half pace, mutation off, 450,000 frames, 12
  world seeds.
- **About 4 of the 7 species hold 100+ leaf cells per seed** (47 → 50
  summed over 12 seeds after grass was given a lifespan, `grass.ron`'s
  record).
- **Grass is the ratchet.** It once took 21% of the low plants' columns
  every 50,000 frames and gave none back.
- **The low plants lose to seed heaps and burial**: 76 of 89 covered
  sprouts died. Every lever tried that thinned the seed grew more grass
  (`dead-ends.md`, the seed-heap entries).
- **Herbs are annual**: 8 of 12 beds were still declining at frame 450,000.
  With the rain off they die out, and a 49.7M-frame playtest ended as grass
  alone.
- **Ants change the bed.** By energy their diet is 60% dead leaf, 8% live
  leaf and 7% seed, and they carried 188 seeds home in one playtest.
  Inedible leaves gave 83 plants against 36.

### 2.3 What a long bed costs

Measured 2026-10-20 on this machine (4 vCPU), `latecensus scenario=played_bed`:

| run | time |
|---|---|
| 10,000 frames, no ants, **1 thread** | **6.0 s** |
| 10,000 frames, no ants, 4 threads | 7.4 s |
| 10,000 frames, with ants, 4 threads | 9.0 s |
| 60,000 frames, no ants, 4 threads | 62 s (cost rises as the bed fills) |

**One bed per core beats one bed on four threads**, so long-bed work is
embarrassingly parallel over beds. A 450,000-frame bed is roughly **8–12
core-minutes**, to be pinned in M0.

## 3. Part A: plants that grow like plants

### 3.1 The target, as three numbers

A plant's form should be decided mostly by its genes and its surroundings,
and only a little by dice. **Variation from dice** is small, **variation
from surroundings** is large and goes in the direction a real plant goes, and
**variation from genes** is visible. One instrument measures all three. Its
starting bars are below; M0 replaces them with measured ones, with headroom
(`CLAUDE.md`):

| number | how | today (herb) | first bar |
|---|---|---|---|
| **Dice** | clones, same surroundings, different spots *and* different developmental seeds; spread of size, height, width and leaf share | size CV 0.26–0.41 | ≤ 0.10 |
| **Surroundings** | clones across set gradients (under a lamp ↔ between lamps, wet ↔ dry soil, alone ↔ crowded); the shift in each trait divided by the dice spread | unmeasured; sideways light provably zero | ≥ 3 in the expected direction, for every response §3.3 adds |
| **Genes** | different genomes, same surroundings: heritability | 0.03 (size) | ≥ 0.5 for genomes the trainer's step size apart |

The "dice" arm varies the developmental seed on purpose. **A per-plant key
alone only freezes the dice; it does not shrink them.** Today two herb
lineages with different seeds would still differ as much as two spots do. A
real plant's development is canalised: the same genes in the same place give
nearly the same plant, whatever the seed. That is what the bar asks for.

Each arm also renders a strip, the same genome across spots, gradients and
seeds, sent to the owner in the chat.

### 3.2 The instrument (A0)

Extend `examples/clone_variance.rs`, which already has the spot arm, the
key switch and the heritability arms, with:

- a **seed arm**: one spot, N developmental seeds;
- an **environment arm**: clones across lamp offset, soil water and a
  neighbour planted beside them;
- a **one-line summary** of the three numbers;
- a **strip** per arm.

**Positive controls:** a gene known to change size must read heritable, a
lamp moved off-axis must show any lean that exists, and the
seed arm must read non-zero today. Run it on herb, shrub and grass, which
are three growth forms.

### 3.3 The changes, in order, one PR each

Every change gets the three numbers before and after, a picture in the chat,
and a frame-cost check on `ascii`. The guard for each fails if its
replacement artifact appears (`CLAUDE.md`).

1. **A1: separate the plants' dice from the weather's.** If the survey's
   reading is right, give the litter shade its own stream so a fallen leaf no
   longer moves the rain. Small, and it makes every paired comparison after
   it cleaner.
2. **A2: key dice on where a plant is in its own development, not on the
   clock.** The key becomes (the plant's seed, which axis, which node along
   it), not (seed, x, y, age). A plant that falls one tick behind then rolls
   the same numbers when it gets there. Expected: §6.13's tree divergence
   largely goes; checked by its own scene.
3. **A3: replace the four lasting rolls with rules driven by the plant's
   state**, and measure the dice number after each:
   - **taper**: the tip carries a growth fraction and extends when it
     reaches one (the way `render_step` already carries a residual), instead
     of a 4-miss lottery;
   - **leaf side**: a fixed alternation, which is how real phyllotaxis
     works. A small jitter stays so leaves are not stamped;
   - **bud kill**: a threshold on how far thickening has reached, not a
     coin;
   - **step choice**: the best-scoring neighbour, with error diffusion
     carrying the remainder (sideways intent becomes a diagonal every few
     steps, not a coin each step). A small temperature keeps some
     individuality;
   - **near-ties**: a margin on the bud-break and root-face argmaxes, so a
     tiny light difference stops deciding which bud breaks.
4. **A4: which key ships.** Once A2–A3 have shrunk the dice, the key matters
   much less. Ship each plant its own seed drawn at germination, so every
   plant is slightly individual ("alive rather than stamped", rethink §7),
   with the dice too small to swamp genes or surroundings. That end is
   measured, not assumed. If the dice number still fails, revisit sharing a
   lineage seed.
5. **A5: responses to the surroundings**, each graded, each with its
   environment arm:
   - **roots to water**: the on/off switch becomes a lean proportional to
     the gradient. Roots branch more in wet soil (keying root branching on
     local water), and a nutrient sensor copies `moisture_pull` onto soil
     nutrients;
   - **leaves and shoots to light**: the withdrawn sideways-light patch,
     opted into per species (U7). The trainer re-finds each species' light
     weight under it, which is the re-derivation that report said the fix
     needs;
   - **less light, bigger leaves**: leaf cluster size scales up as light
     falls. Income must be counted per leaf, not per node at the genome's
     cluster size (`plant.rs:8160–8210`), or a bigger cluster in shade earns
     nothing;
   - **shade avoidance**: low light or crowding lengthens the interval
     between nodes and thins the stem;
   - **root:shoot balance follows light** as it follows water already.

   Each response's strength is a species gene, so species can differ in how
   plastic they are. That is a niche axis Part C can use.

**Risks:**

- Every plant in both games moves, so each species' constants, tuned
  against dice, may need re-deriving. That is the trainer's first job after
  Part A, not hand-tuning.
- Too little dice can look stamped. A3 keeps a small jitter, and the strips
  decide.
- Frame cost should be flat: A2–A3 replace draws with arithmetic, and A5
  adds a few field reads per growing tip.

## 4. Part B: "a diverse bed that thrives", as a number

### 4.1 Which bed (owner decision, §8)

**Proposed:** a lab bed sown with all nine plant types: herb, grass, shrub,
scrambler, creeper, tree, conifer, reed, moss. Train on it **without ants**,
because it is about 20% cheaper and one moving part fewer. Check it **with
ants** at the end of every round, because the lab is the game and ants are in
it. The played bed and mix7 stay as regression beds.

### 4.2 The score: graded, per bed, then over seeds

For each world seed, from censuses every 25,000 frames over the last half of
a 450,000-frame run:

| part | what it asks | graded how |
|---|---|---|
| **Persistence** | how many kinds are still there | each kind scores min(1, its leaf cells / 100) at the end, summed |
| **Thriving** | is each one reproducing, not hanging on | the share of present kinds with seedlings established in the last 100,000 frames |
| **Trend** | is anyone fading out | each kind's leaf share slope over the last half; a falling kind pays in proportion |
| **Evenness** | many kinds, not one carpet with tokens | effective number of kinds (exp of Shannon on leaf shares) over the number sown |

**The bed's score** is a weighted sum. **The run's score** is the **25th
percentile over 12 seeds**: an order statistic, so a bed that thrives on half
its seeds is not called diverse. The weights and the 100-cell floor are set
after M0 shows today's bed and its spread. The noise bar is that spread,
measured on the shipped species, the trainer plan's §7.5 rule.

**Cheaper proxy, validated first.** A 150,000-frame bed would cut cost by
three. It is used to filter only if its ranking of candidates agrees with the
450,000-frame ranking (rank correlation measured on the M0 candidates, as
mean-against-min was in P1). Otherwise every step pays full length.

## 5. Part C: training in small steps

### 5.1 What is trained

**One species file at a time, with the rest of the bed held as it is.** A
step is accepted only if the bed's score rises beyond the noise bar
(paired, same seeds) **and no other kind falls below its floor**. Then the
next kind is trained. The species are visited worst first. Each accepted
step is one small, inspectable change: a new file beside the shipped one.

### 5.2 The step

- **Children:** take the species, propose children with the P1 genome
  machinery (use-site bounds, sign-locked ranges, validated, renamed), and
  sow each in the community bed in place of its parent.
- **Step size:** small *once Part A is in*. P1 measured that 1–3-gene
  children are invisible today, so the heritability gate is re-measured
  after Part A and sets the step size. That is the concrete reason Part A
  comes first.
- **Selection:** parents are the best quarter, with in-bed mutation off
  while scoring, as in P1.
- **Champion:** chosen on a validation set of seeds that is neither the
  hold-out nor the final set. This is P1's seed-13 lesson.

### 5.3 A ladder of beds

Each rung is a small step, and earlier rungs become a regression suite:

1. **Alone** (P1's bed): the species lives its whole life. It exists.
2. **Beside grass**, the dominant: both persist over 150,000 frames.
3. **The low plants and grass**: herb, scrambler, shrub, creeper and grass.
4. **All nine kinds.**
5. **All nine with ants**: the game.

### 5.4 Controls, every round

- **A known answer.** Cripple one species in the community bed (halve its
  photosynthesis until it dies out); the trainer must bring it back, beside
  random selection at equal compute. This is P1's acceptance task, one rung
  up.
- **A final set read once**, a monitoring hold-out, and paired bars, as in
  P1.
- **Determinism**: the same settings give the same bytes.
- **A cheat watch.** A gene that pins against its range, or a species that
  wins by a lever with no cost, is reported as an engine pricing gap and
  never tuned around (`plant-equilibrium-costs-2026-08-27.md`: levers with a
  benefit and no cost run away).
- **The owner's eye.** At the end of each round, a time-lapse GIF of a bed
  and the score table come to the chat.

## 6. Compute

### 6.1 Where to run what

| work | where |
|---|---|
| Part A instruments, P1-size runs, tests | here, or any session |
| small trainer rounds (≤ 6 h, 4 vCPU = 2 cores) | GitHub Actions `train.yml`, which exists. It shares the CI pool, so not for sustained load |
| community rounds (Part C) | **Google Cloud, spot VMs** |

### 6.2 Cost

A step is 8 children and the parent, on 12 seeds: 108 beds × about 10
core-minutes, so **about 18 core-hours**.

- **Here:** about 4.5 hours a step.
- **One spot `c4d-highcpu-32`** (16 cores, about $0.51 an hour, from
  `Reports/brain-training/cloud-compute/cost-research-2026-10-18.md`; list
  prices, not from Google's own page): about 1–1.5 hours and **under $1 a
  step**.
- **A round** of every kind, 2–3 steps each, is about 25 steps: about $25
  and a day on one VM, less with several VMs in parallel.

All of this is estimated from §2.3 and is pinned in M0.

### 6.3 What the new session needs

**From the owner, in the new session's environment settings:**

- a service-account key (or workload identity) as a secret, with Compute
  Engine and Cloud Storage access;
- the project ID;
- network access to `*.googleapis.com`;
- a budget alert. Hard caps on Compute Engine were in preview at the last
  check, so the launcher enforces its own limits as well (below).

**Built by that session** (`scripts/gcp_round.sh` or similar):

1. Create a spot VM from a startup script. The script fetches the repo at a
   pinned commit, builds release, and runs one process per core over a job
   list (child × seed).
2. Each process writes its census to Cloud Storage as it finishes.
3. The VM deletes itself when the list is done, or after a hard maximum
   runtime, whichever comes first.
4. A preempted VM loses only its unfinished beds, which are re-queued.
   Results are deterministic, so a re-run gives the same bytes.
5. Results come back into the repo as data, as `train.yml`'s results branch
   does.

## 7. Milestones

| | what | done when |
|---|---|---|
| **M0** | **Baselines, no engine change.** The development report (§3.2) on herb, shrub and grass. The nine-kind bed and mix7, 12 seeds × 450,000 frames, with and without ants: score parts, spread, and the 150k-vs-450k proxy check. Exact per-bed timings. | today's numbers and pictures are in the chat, and the §3.1 and §4.2 bars are set from them |
| **M1** | **Part A**, A1–A5, one PR each | each meets its bar and the owner has seen its strips. Then M0's community baseline is re-run on the new engine |
| **M2** | the community score and harness (`plantrain mode=community`), the validation-set champion pick, the known-answer task on the community bed | the crippled species comes back and random selection does not |
| **M3** | the GCP launcher (new session) | the M0 baseline reproduces byte for byte on GCP |
| **M4** | **training rounds** up the ladder | each round's step is above the noise bar, with no kind lost, and the GIF is shown |
| **M5** | promotion: the owner picks which trained files replace the shipped ones; lab check with ants; README, wiki and freshness notes | owner's pick |

M0 needs nothing from the owner and runs in a few hours here. M1 and M2 can
run here too. M3 onwards needs the GCP session.

## 8. Decisions for the owner

1. **The bed:** all nine kinds, trained without ants and checked with them
   (§4.1)? Or the played bed, or mix7?
2. **What "thrive" means:** the four parts of §4.2, weighted after M0's
   numbers are seen? Is any kind allowed to be rare but present, like moss
   or reed?
3. **Part A ships on for every plant in both games**, each step shown as
   pictures first. It changes how every plant grows, which is the point, and
   is why it comes before training.
4. **GCP:** a daily spend ceiling, and which machine size (the 16-core spot
   VM is the proposal).

## 9. Risks

- **Coexistence needs trade-offs the engine may not price.** If one lever
  is free, every species converges on it and diversity falls; dead-ends
  records that *"a free lever made heritable produces uniformity, not
  diversity."* Answered by the cheat watch (§5.4) and by Part A's plasticity
  genes, which give species different niches (shade-tolerant against
  light-hungry, deep against shallow roots).
- **A score can be met without the look of a thriving bed.** Answered by
  the owner's GIF every round. The score decides steps; the owner decides
  rounds.
- **Part A moves everything, both games included.** Answered by one PR per
  change, each with numbers and pictures, and the M0 baselines to compare
  against.
- **Cost overrun on GCP.** Answered by the launcher's own deadline, one
  machine at a time to start, and the M0 timings before any round.

## 10. Sources

- The code survey (randomness and responses, with `plant.rs` line numbers)
  and the records survey: research agents, 2026-10-20, summarised in §2. The
  INFERRED items are marked there.
- `Reports/plant-engine-rethink-2026-09-03.md` §2.3, §6.8, §6.13 and §7.
- `Reports/plant-trainer-plan-2026-10-19.md` §13 (P1 measurements).
- `Reports/plant-phototropism-lateral-2026-08-27.md`.
- `Reports/plant-equilibrium-costs-2026-08-27.md`.
- `Reports/brain-training/cloud-compute/cost-research-2026-10-18.md`.
- `assets/species/grass.ron` (community record).
- `Reports/playtest-2026-10-09-herb-ant/README.md` (ants and plants).
