# The plant-engine rethink — lane note

*One unattended overnight session, 2026-09-03, against
[`../plant-engine-rethink-brief-2026-09-03.md`](../plant-engine-rethink-brief-2026-09-03.md).
The findings are in
[`../plant-engine-rethink-2026-09-03.md`](../plant-engine-rethink-2026-09-03.md)
and are not repeated here. **This note holds only what a later session cannot
reconstruct** — what was overturned, and what is waiting on an answer.*

## Waiting on the owner — six review cards, all open at hand-off

| card | asks | what it gates |
|---|---|---|
| `20260903T060437573Z-e132ab` | blind: which of two beds is sixteen clones of one genome | nothing — it is the noise floor shown rather than stated |
| `20260903T060536659Z-91838d` | the genome's two extremes alternating — two kinds of plant, or one kind at two sizes | nothing |
| `20260903T060947894Z-4646f4` | twelve clones one column apart — is that scatter wanted, or to be cut down | **§7 item 2**, below |
| `20260903T120950045Z-4f14bd` | blind A/B of leaf-cluster shape, `leaf_spread` 0 against 1 | whether `leaf_spread` **stays**: if the two stands read the same, retire the lever rather than tune it (§7 item 4) |
| *(posted 2026-09-04)* | blind A/B, shipped against `dev=0` — a lineage with one inherited form | **which end of `shared_development` ships.** H2 on plant size 0.034 → 0.650 and crown width 0.227 → 0.658; the cost is that siblings grow alike |
| *(posted 2026-09-04)* | blind A/B, shipped against `dev=1` — each plant one coherent form | whether coherence alone is worth having. **This card is the primary instrument at that end**, not the H2 table |

**The fourth card was posted after the first three and this table did not have
it for a while**, which is worth a line because the table is the only place a
later session is told to look: a card the lane note does not list is a verdict
nobody collects. Check `python3 scripts/review.py list` against this table
rather than trusting it.

**The third one gates real work.** The report's §7 item 2 proposes keying plant
growth on the plant's own frame with an inherited developmental seed, so that
two plants of one genome grow the same shape wherever they stand. That moves
every plant in both games and it is a design decision, not a repair — some of
the present scatter is what makes a stand read as alive. **Do not start it
before that card is answered.** Collect with
`python3 scripts/review.py inbox`.

## What this session overturned — six things, five of them its own

1. **Slot 1 is not caged on tree/conifer/shrub.** The static reach census read
   one authored base per genome slot and reported three species unable to
   evolve a branching root system. Slot 1 has **two** consumers — it also
   divides the root's `branch_priming`, under a comment in the species file
   reading *"Superseded by `branch_priming` below"*. The widening arm caught
   it. **A reachability census taken by reading one call site is a census of
   that call site.**
2. **`PlantScene::build` does not draw a genome.** It plants through
   `World::plant_tree_species`, which allocates the organism and writes the
   cell and **never calls `plant::seed_genotype`** — only `World::plant_tree`
   does. So at frame 0 every founder in every plant harness in this repo holds
   `genotype_draws = [0.0; 10]`, the species mean. A `ref=` argument selecting
   among founders was therefore inert and produced byte-identical output at
   `ref=0`, `1` and `5`. **Any harness that reads a founder's genome at frame 0
   has this.**
3. **The organism id is not the noise lever.** An earlier draft claimed the
   `organism_id` term in `rng::stream(organism_id, cx, cy, frame)` was a
   per-individual random seed worth removing in one line. The arm that produced
   the number builds a fresh world per run and plants one thing, so the id is
   **constant by construction** and what was measured is position alone. The
   id's contribution is still unmeasured, and the arm that would settle it
   needs a way to advance the organism counter without adding a plant.
4. **Clonal spread is already reachable in the shipped game**, which nothing
   in the record said. No root reaches a `Node` fate, but `FateOp::Retarget`
   can point the root's `Grew` rule's `lateral` at a `GrowingTip`, and then
   every root growth step launches a shoot. Found by accident, while running
   the control for something else.
5. **…and then that finding was cut back by its own follow-up.** The *event*
   is reachable; the growth form is not. 144 launches over eight worlds
   produce no second stem above the background rate and **no extra width at
   all**, on a stand 22% smaller for having tried (report §4b). The draft that
   read §4a as "a lineage discovers clonal spread" overstated it — what a
   lineage discovers is a lateral that goes nowhere. **Reachability was not
   the binding constraint**, which is the load-bearing correction for anything
   built on the brief's *"nothing hard coded, let variety evolve"*: a form
   also has to pay.
6. **The background rate of a census is not zero until you have run the
   negative control on the seeds where the mechanism cannot fire.** All three
   of the shipped species' "second stems" occur on seeds whose launch counter
   reads zero. Had the treated arm been read alone, five second stems would
   have looked like the mechanism working.

## Round two — the owner's direction on PR #225, and what it overturned

All three items built and measured; the lab is wired so they can be played
with (`PARAMS -> HEREDITY`). What a later session cannot reconstruct:

1. **The Spread arm was developmentally uniform and its own control said so.**
   `H2 (control)` read 0.000 on every descriptor at `dev=0`. A founder has no
   lineage seed until something draws one, so reading it off an ungerminated
   founder returns 0 and writing that onto all sixteen collapses `Var(spread)`
   — the denominator of every H2. **The `ref=` failure, in the same file,
   caught by a control rather than by a byte-identical output.** Before
   trusting any H2 row here, check the control row in the same file is
   non-zero.
2. **`dev=1` moves heritability, which the review predicted it would not.**
   ~0.32 on `cells` against a shipped 0.034. The reasoning — germination
   position still fully selects the form — was sound and the conclusion was
   wrong. The rendered comparison is still the primary instrument at that end.
3. **Composition gets *worse* under `dev=0`** (foliage share 0.61 → 0.48)
   while size and width get much better. Developmental noise was masking shape,
   not composition. Do not report the change as a uniform improvement.
4. **Age-neutral mortality is not a generation lever.** `Hazard.chance` 0.02
   adds nothing to depth (5 against 5, 8 against 8) and costs 11% of the
   births. The intuition that more death means deeper lineages is wrong here
   because the hazard is deliberately independent of age.
5. **The report's §5.4 free-lever list was stale** — three of nine are priced
   now. Genuinely free: `turgor_source`, `turgor_yield`, `heading_inertia`,
   `seed_maturity`, `branch_angle`.
6. **Pricing `seed_launch` retired §6.1's own +38% headline.** Priced, the
   far-dispersal column is flat; coverage and establishment survive. An
   unpriced lever's headline number is a measurement of the free lunch.

## Traps in the lab UI that cost a build each

- **`OrganismState::collar_y` is not an origin.** It is recomputed every
  organism tick as the plant's *lowest shoot row*, so keying a growth draw on
  it re-keys every cell the moment the collar settles a row lower. The
  developmental key uses a separate immutable `origin`.
- **The PLANT params page is capped at 20 rows** and was at 21 with one dial
  added. `no_page_is_longer_than_two_screens` catches it; the fix was a
  HEREDITY page, not a raised cap.
- **The LIFE panel has a fixed border** and a second generation row painted
  past it. `the_page_stays_inside_its_own_border` catches it. Both numbers now
  share one line.
- **`Knob::Heredity` runs everything through `settable_rate`**, which bounds
  0..=1. A row that is a *mode* rather than a rate has to be handled ahead of
  that guard or every setting above 1 is silently refused.

## The blocker a later session will hit on underground shoots

**A shoot below ground cannot be followed past maturity, and the reason is a
missing distinction rather than a missing instrument.** `CellType::MatureBody`
is shared by root and shoot, and `plant::organ_material` gives a distinct
material only to `Flower` and `Fruit` — so a matured sucker is identical, by
cell type *and* by material, to the root it came off. That is why §4b can say
*nothing surfaces* and cannot say *why*. Anything that proposes to make
underground shoots work needs that distinction to exist **before** it can
measure whether it worked.

## Traps this bed has that are not in `CLAUDE.md`

- **The plant bed buries its own collars.** Shoot tissue below the ground line
  reads 1 / 3 / 2 in the *unmodified* species over three world seeds, and up to
  **nine rows** down on one. So no census of "is there shoot tissue below
  ground" can answer "did a root put a shoot up" — that took two failed
  discriminators and an event counter (`World::root_shoots_launched`).
- **A widening arm at a short horizon reads as a dead slot.** At 4,000 frames
  five of eight live genome slots came back byte-identical; at 16,000 all eight
  moved. The stand had not yet thickened or rooted to depth.
- **`grass` has no `CellType::Leaf`**, so every composition descriptor is
  structurally zero for it, and it establishes ~2 plants in the standard bed.
  Any number measured on `grass` in this bed is a number about the scene.
- **`tree` reaches generation 1** in a 20,000-frame run, so the clone/population
  split is not measurable on it — 23 established plants and four variance ratios
  above 1.0. The woody species need a horizon nobody has run.
- **`moss` is outside evolution entirely**, and nothing in the record said so.
  It has no `Grow`, so none of the ten continuous slots is expressed; and
  `Divide` writes **the same organism id**, so a patch is one individual
  spreading and never bears. Every heredity channel hangs off
  `plant::bear_seed_at`, so none of them reaches it — the parameter genome
  included. Any evolution result that says "across the shipped species" has
  been excluding one of them.

## The limitation to hit first, in the mechanism this session built

**A parameter no species has ever authored is evolvable only within four units
of zero.** `param_scale` takes units from the corpus and falls back to 1.0 when
the corpus is empty, so `clamp_param` bounds a `Magnitude` at 4. The first new
channel built that way, `seed_launch`, is measured with reach **4 inside the
spread** and reach **12** at +38% — so it is heritable and its evolvable range
is below its useful range. **Do not fix this with a per-parameter range
table**: that is the hardcode `ParamKind` exists to remove. Authoring a
non-zero value on one species raises the scale for every lineage at once, which
is the corpus doing its job. Applies to every future `ParamId` whose corpus is
all zeros — check the reachable range against the range the mechanism needs
before calling the channel evolvable.

## The two things shipped inert, and what would turn them on

`plant::PARAM_MUTATION_CHANCE = 0.0` and `Behavior::Reproduce::seed_launch`
at 0 in every species. For the rate: the mechanism is measured, the rate is
not. **Run `genome_reach -- drift=1` before raising it** — the addresses that
pile up at their `clamp_param` bound are the free-lever list measured rather
than inventoried, and pricing those is the prerequisite. At rate 0.3 nothing
piles up except `juvenile_size`, because `herb`'s pedigree is ~2.3 generations
deep and cannot accumulate the coordinated set a degenerate optimum needs; that
changes when generation depth rises.
