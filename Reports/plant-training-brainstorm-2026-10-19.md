# What a plant trainer could train — from one dial to a new kind of plant

*2026-10-19. Step 1 of 5 of the plant-trainer line: brainstorm → engine review
([`plant-engine-trainability-review-2026-10-19.md`](plant-engine-trainability-review-2026-10-19.md))
→ plan ([`plant-trainer-plan-2026-10-19.md`](plant-trainer-plan-2026-10-19.md))
→ adversarial review of the plan → build. This page is the menu, not the
choice: the plan picks from it.*

**Where this sits.** On 2026-10-17 the ant line stopped writing behaviour by
hand and built a trainer (`src/train/`): small beds that each pay for one
skill, a population of genomes, keep the best quarter, mutate, repeat, with
the controls that keep a score honest. This line asks the same of plants. The
reason it should work better for plants than for ants is recorded twice
already: plant evolution *in the bed* is starved of generations (8–17 seed
generations in 300,000 frames, `dead-ends.md`'s defence-step entry,
2026-10-03; a tree takes ~15,000 frames a generation, `plant_probe`'s
`GENERATION CLOCK`), and a trainer does not wait for the bed to breed.

## The owner's rulings that frame this (2026-10-19)

1. **A trained plant is a species file.** What the trainer produces is an
   `assets/species/<name>.ron` the game loads like any other — not a genome
   sidecar, not a lab-only object. (The creature line's `species_export.rs`
   made the same call for animals, decision E8.)
2. **The trainer may change a plant's shape and look.** Not only its
   physiology. *"We will need to think carefully what we are scoring, how are
   we measuring and optimizing looks, and how that could lead to improved
   looks from our current plants or new interesting looks in new plants."*
3. **Herb goes first.**
4. **A plant-only test bed may shed creature-side overhead** to train faster.
   Not a high priority.

## Measured before writing this

One generation of `selection_arena` on this container, one thread, live
weather (`RAYON_NUM_THREADS=1`):

| bed | frames | wall |
|---|---|---|
| herb, 4 founders, 256 wide | 6,000 (one herb generation) | **1.3 s** |
| herb, 16 founders, 1,024 wide | 6,000 | 6.1 s |
| tree, 4 founders, 256 wide | 15,000 (one tree generation) | **13.8 s** |

The ant trainer's beds run 0.07 s (`fetch`), 0.3 s (a colony) and 2.2 s
(`dig`). **Herb is in the ant's price range; a tree is about 11x a herb per
generation.** These numbers are why rule 3 is cheap to keep.

## How to read the menu

Each option says **what it scores**, **how it will be cheated** (the ant
trainer's first probe had every score cheated inside three generations —
heap scattered, food flown home, colony let shrink — so assume the same),
**what it touches** in the species file, and **roughly what one evaluation
costs**. The ladder runs from things whose right answer we already know
(useful as proof the trainer works) to things nobody has seen.

---

## Rung A — one dial, a known answer (the trainer's own positive controls)

These are worth doing *because* the answer is known: each was tuned by hand,
the record of the hand-tuning is in the species file's comments, and a
trainer that cannot rediscover it is broken.

- **A1. Herb's turgor ceiling against its metamer count.** `herb.ron`
  records that `turgor_source: 0.32` put the height ceiling exactly at the
  ninth metamer, so only **9 axes of 32 established plants terminated in a
  flower**; 0.46 fixed it by hand. Score: share of axes that end in a flower.
  Touches one number. *Cheat:* a plant that flowers at once on a one-cell
  stalk if `after_metamers` is free too — freeze it.
- **A2. Seed maturity and reproductive allocation.** Score: seedlings of the
  second generation established (not seeds set — see the cheats list).
  Known region from `plant-mutation-rate`/`throughput` reports.
- **A3. Flower head size against the stand.** `organ_cluster` 9 → 12 was
  chosen by the owner's eye (blind cards e1ee88/83eaba) after population
  checks. Score: population held *and* flower cells on screen. The first
  rung where a look term enters.
- **A4. Germination thresholds against a dry bed.**
  `plant-water-scarcity-2026-08-30.md` found every species' germination floor
  sits *above* the water level at which its own uptake limits, "so below the
  floor you get an empty bed". Score: plants established in a bed at a pinned
  dry sky. A trainer should lower the floor; if it cannot, the report's
  explanation is wrong.

## Rung B — one behaviour block (a shape)

- **B1. The shoot's `Grow` block alone**: steering weights, `branch_chance`,
  `branch_angle`, `internode`, `plastochron`, `leaf_cluster`,
  `heading_inertia`, `max_active_tips`. This is where silhouette lives.
  Score: a look target (Rung E) with a viability floor.
- **B2. The root's `Grow` block alone**, scored on water uptake in a bed
  where rooting volume limits (the one lever that bit in the scarcity
  report). *Cheat:* roots out into the air (bug W6's chain: the root tip
  takes water's direction *instead of* gravity's) — score must count root
  cells above the soil line as a penalty.
- **B3. Leaf economy**: `Photosynthesize` rate, `shade_death`,
  `drought_death`, `stomatal_reserve`. Score: biomass held through a dry
  spell. *Cheat:* shed every leaf and stand as a stump (bug 0-z: *"a tree
  escapes dying of thirst by starving first"*).
- **B4. The organ clocks**: `Ripen` rates and costs, `rebloom_after`. Score:
  standing flowers over time (the played bed ran out of flowers on its own,
  81 → 25 → 8, pollinator design §1.2).

## Rung C — a whole species file, for a niche

The trainer owns every field of `herb.ron` but the materials it names.

- **C1. A better herb on the shipped bed.** Score: share of the bed held
  against the shipped herb in one world (`selection_arena`'s design, mirrored
  pairs). *Cheat:* a herb that wins by smothering — fine if it looks right;
  that is what the look floor is for.
- **C2. Herbs for places the shipped herb fails**: a dry bed (`sky=clear`), a
  shade bed (under a pinned canopy), a shallow-soil bed (`soil=4`), a wet bed
  (standing water — reeds are the only plant that lives wet). Each winner is a
  new species file. This is the most direct route to "new interesting plants"
  that also *mean* something in the world.
- **C3. Life-history strategies** that are hand rules today: annual vs
  perennial (`annual_half_life`), seed bank longevity (`seed_half_life`),
  regrowth after grazing, resprouting. Score: persistence across a bed with
  a disturbance (drought pulse, a cut). The owner has already shipped several
  of these by hand (`ANNUAL`, `GRASS_REGROW`, `GRAZE_REGROW`); a trainer could
  find which strategy each niche wants.

## Rung D — the development program (new body plans)

The fate table — what a growing cell becomes at each event — is heritable
already (`FateGenome`), and it is where *kinds* of plant differ rather than
sizes.

- **D1. Determinate vs indeterminate, and where organs go**: flowers at every
  node (a spike), only at the apex (the herb), at lateral tips (a branching
  head). Score: a look target for inflorescence shape.
- **D2. Rosette and basal habit**: leaves clustered at the base with a bare
  flowering stalk. Whether the fate vocabulary can express this at all is a
  question for the engine review.
- **D3. Clonal spread**: a plant that sends runners. Needs a fate that makes
  a new rooted tip at a distance — probably not expressible today; the review
  should say.
- *Cheat for all of D:* the viability gate found `child` mutations on a
  frontier type kill 5 of 6 plants while `becomes`/`lateral` took 34 without
  one failure (`plant-fate-viability-2026-08-28.md`) — so a D-rung trainer
  needs mutation operators weighted by that record, or it wastes most of its
  budget on dead plants.

## Rung E — looks: what can be scored, and how

This is the hard rung, and the owner asked for it to be thought through. A
plant's job here is to look like a plant, and an optimiser finds the
high-scoring ugly answer before anything else. Eight ways to put a number on
a look, roughly cheap to dear:

- **E1. Shape descriptors from cells** — height, width, height:width, crown
  depth, leaf share, flower share, wood share, number of visible branches,
  branch angle distribution, gap fraction inside the crown, box-counting
  dimension of the outline, vertical leaf profile, left/right asymmetry.
  Cheap (a pass over the plant's cells). **The house trap applies in full**
  (`.claude/rules/plants-and-organisms.md`): sympody, tropism and acrotony all
  fired and moved no pixel, because they relabel cells while the silhouette
  is set by material and palette. So descriptors must be computed from
  **materials as rendered**, not from cell labels.
- **E2. Pixel descriptors from the shipped renderer** — `Renderer` draws on
  the CPU (`filmstrip`, `labshot` use it with no GPU), so a trainer can render
  each champion and measure colour histogram, contrast against soil and sky,
  `creature_look`'s *ink* and *decoys* (can you find it in the picture), and
  `world_look`'s between-group/within-group colour separation.
- **E3. Distance to a target.** The owner picks an exemplar — an existing
  plant at its best, a drawing, a described silhouette — and the score is
  descriptor distance to it. Good for "make the current herb look better",
  if the owner can say what better is.
- **E4. Recognisability.** A species should look like itself: the
  `clone_variance` instrument measured herb size heritability at
  **0.00–0.05**, i.e. where a plant stands decides its look more than its
  genome does — the owner's *"clones of the same plant end up growing very
  different"*. Score: within-species spread of descriptors small against the
  distance to every other species. Directly answers the owner's standing
  complaint that species *"differ in size and colour and nothing else"*.
- **E5. Novelty and quality-diversity.** Instead of one champion, keep an
  **atlas**: a grid over two or three look descriptors (say height × spread ×
  flower share), and keep the fittest viable plant in each cell
  (MAP-Elites). The output is a sheet of distinct living herbs, and the owner
  picks the interesting ones. This is the most direct answer to *"new
  interesting looks in new plants"*: the trainer is asked for variety, not
  for one optimum, so it cannot converge on one ugly winner.
- **E6. The owner in the loop.** Interactive evolution (Picbreeder's method):
  each round the review queue shows a grid of nine, the owner picks the ones
  they like, those breed. Slow (one human verdict a round) but the only
  score that *is* the bar. `scripts/review.py` already carries cards to a
  phone. Best used to steer an E5 atlas rather than to drive every
  generation.
- **E7. A learned taste.** Every owner verdict (E6, blind A/B cards) is a
  labelled pair; a small preference model over E1/E2 descriptors, refitted
  as verdicts come in, becomes a score the trainer can use between verdicts.
  Dear in owner time to train, cheap to run, and it can be checked: it must
  predict held-out verdicts better than chance before it is allowed to steer.
- **E8. In motion.** The house rule is *looks good in motion*. A plant sways
  in wind (`Bending in the wind`) and grows over time; a score over a short
  frame sequence — sway visible but no snap, growth that reads as growth
  rather than flicker — is possible from E1 descriptors per frame. Later.

**Penalties every look score should carry**, each a defect this repo has
already met: root cells in open air (W6); cells not connected to the plant;
one-pixel whips (a plant that is a single line reads as a glitch); wood share
near 90% with a one-cell green skin (`plant-appearance-design.md` §2.1, the
reason three species looked like one); a flat mat across the ground; a
leafless standing stump (0-z).

## Rung F — competition and community

- **F1. Invade the shipped stand**: a trained herb sown into a bed of shipped
  herbs must spread. `selection_arena` does this today.
- **F2. Coexist**: two trained species that both persist in one bed for N
  generations — a community rather than a winner. Score: the rarer one's
  share at the end. Needs many generations per evaluation; dear.
- **F3. A whole bed's look**: train a *set* of species together so a lab bed
  reads as varied (E4 between species, E2 over the whole frame). The most
  complex target here, and the one closest to what the player sees.

## Rung G — a plant "mind" (decisions, not dials)

The ant's general brain lists every legal option and lets one trained scorer
choose. A plant tip already decides that way — it scores its open neighbour
cells by four weights (keep going, toward light, with wind, up or toward
water), divides by crowding and draws (`plant.rs`, `Grow`) — so the plant
version is a scorer over more features per neighbour: light there, water
there, how crowded, how far from the plant's other tips, how much load the
branch already carries. Further decisions could follow: which bud breaks,
how carbon is split between root, shoot, seed and defence, when to flower.
**Only worth building if Rungs B–D show the dials are the limit.**

## Rung H — with animals (out of scope while this line is plant-only)

Defence against grazing, seeds carried by ants (`seed_gut_survival`),
nectar for pollinators, a garden a colony can live on. Recorded so the
plant-only arena (ruling 4) is designed not to rule them out.

---

## What every rung needs, borrowed from the ant trainer

- **The shipped species as the control**, scored on the same beds every
  generation, so a rising best can be told from easier beds.
- **Beds training never sees** (a hold-out), scored every generation.
- **Mirrored pairs scored on the worse side**: a trained forager once
  averaged 14 by scoring 21–38 on every east bed and 0 on every west.
- **A "blind" control**: the champion with its light and water sensing
  zeroed (`light_weight`, `wind_weight`, the hydrotropic pull). If it scores
  as well blind, what was learned is a fixed habit, not a response.
- **Proof the score has teeth**: `selection_arena`'s `lethal` and `nobranch`
  arms must score below the shipped herb, or the score is blind.
- **Scores in steps**: germinate → establish → survive a dry spell → flower →
  set seed → those seeds establish, each worth more than everything below
  it. "Food delivered" alone gave the ant trainer nothing to climb; "seed set"
  alone will do the same to a plant.
- **The cheats to design against from the start**: many tiny seeds that
  never germinate (score establishment, not seeds); seeds thrown out of the
  bed; a plant that wins by being too short to be shaded; parameters run to
  their clamp (the `PARAM_MUTATION_CHANCE` doc predicts *"the first thing a
  population would do is run the free ones to their bound"*); and every look
  penalty above.
- **A trainer win is not a game win**: the ant's 8x forager kept a colony
  half the game ant's size in the lab (153 ants against 319). Every
  champion is checked in the lab bed over seeds before it is called a
  species.

## The cheap arena (ruling 4, low priority)

What a plant-only bed could shed, to be priced rather than assumed: the
creature pass and its pheromone planes, live weather (pin the sky per bed,
which `selection_arena sky=clear` already does), a smaller world, the
chunk-wake cost of a bed with nothing moving but plants. **Measure each
against the 1.3 s herb generation before building any of it** — a cost that
vanishes may be work that vanished.
