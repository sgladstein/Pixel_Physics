# Handoff: the production rule is heritable now (2026-08-29)

**Status: handoff. Read this before continuing the plant-evolution line.**
Written because the mechanism landed and the *measurements it needs* did not,
and the gap between those two is the whole content of this document.

The code is on `claude/plant-morphology-organs-phase-4-uubdv1`
(PR sgladstein/Pixel_Physics#96), which also carries Phase 4 (the organ
package) and Phases 0–3 beneath it.

## 1. What changed

`SpeciesDef::fates` used to live on `Species` in the registry, and a seed
inherited its parent's **species id** unchanged plus ten continuous draws and
six discrete alleles. So nothing a genome carried could reach the production
rule: a plant could not mutate into a determinate one, or into one that bears
an organ, at any rate whatsoever. Hand-authoring a species file was the only
channel to that vocabulary.

Now every organism carries its own `organism::FateGenome` — a fixed 16-slot
array of `PackedFate`, one `u32` each — founded from its species table at
`World::push_organism`, read by `plant::fate_for` ahead of the species table,
and copied-then-mutated at `plant::bear_seed_at`.

| | |
|---|---|
| capacity | `MAX_FATES = 16`, against the widest shipped species at 9 |
| founded | `World::push_organism`, from the species table, no founder variance |
| read | `plant::fate_for`: individual → species table → `builtin_fate` |
| inherited | `plant::bear_seed_at`, from a keyed substream |
| rate | `FATE_MUTATION_CHANCE = 0.01` per birth |

## 2. The operator, and which quarter of it is measured

**Owner's call, 2026-08-29: the most flexible operator available**, on the
reasoning that it can be narrowed later.

| operator | weight | measured? |
|---|---|---|
| retarget a cell-type field | 60% | **yes**, and re-measured: 74% woody / 97% determinate through the *shipped* operator (the 92% on the left was a lookalike's) |
| change `when` or `after_metamers` | 15% | yes — 2% of draws do anything on woody, 42% on determinate; all that land, live |
| insert a drawn rule at a drawn position | 15% | yes — 8% woody, 18% determinate |
| delete a rule | 10% | yes — **0% woody**, 48% determinate |

**Insert is what the flexibility actually buys.** With retargeting alone a
`tree` lineage could never acquire a flower: `tree.ron` has no `FateWhen::Ripe`
rule to retarget and nothing could create one, so the organ vocabulary would
stay reachable only by lineages whose species already had it.

**Two structural properties that are not tuning and should not be relaxed
without reading why.** Insert draws its *position*, not just its content,
because lookup is first-match-wins — a determinate rule only works listed
above the ordinary one, which is how every authored species writes it. And
delete stops at one rule, because an empty genome falls back to the species
table: a lineage that deleted to zero would silently revert to being
non-heritable, which reads as "evolution stopped" with nothing broken
anywhere.

## 3. What is NOT established — in priority order

**3a. CLOSED, 2026-08-29 — and the answer inverts the premise.** See
`plant-fate-operator-gate-2026-08-29.md`. All four operators are now gated, and
the three unmeasured ones are not the hazard this section assumed: 46 of 46
effective mutants lived. They are **inert** — on the woody base `delete` is 0%
effective in 40 draws, `recondition` 2%, `insert` 8%, because `fate_for` falls
back *per query* and the layer beneath refills any slot a mutation vacates.

Two things this section got wrong, both worth carrying forward. It was **not**
only a harness change: the harness had been measuring a *lookalike* of the
operator rather than `FateGenome`'s own, drawing from six replacement cell types
on the woody base where the engine draws from eight — so the 92% below is about
a mutation nothing in the engine performs. And it is not true that the operators
merely lacked a number; three of them barely function.

**3b. `FATE_MUTATION_CHANCE = 0.01` is a guess.** Gate 1 measured what *one*
mutation does to a fresh table. A rate compounds over generations and nobody
has measured that. Two things argued for starting low rather than high: the
failures are concentrated (`child` on a frontier type killed 5 of 6, and
roughly a third of retargets land there), and a rule table is a program rather
than a dial — a retargeted rule can make a lineage that never grows, with
nothing pulling it back. **The number to replace it with comes from a lineage
census at several rates**, not from an argument.

**3c. Throughput is the blocker, and this change does not touch it.**
`plant-recruitment-measurement-2026-08-27.md`, 16 paired runs: **a tree
reaches generation 1 in 8 of 8 seeds and never more; grass reaches generation
2 in 7 of 8.** So on the woody species a heritable table has almost nothing to
act on — it will drift approximately zero times before a run ends. Making
fates heritable creates the channel; turnover has to use it. **The first
species that could exercise this is grass or the two new herbs, not trees**,
and 3b's census has to be run on one of those or it measures nothing.

**3d. Nobody has watched a lineage drift.** There is no probe that prints a
genome's rule table, and no census of how tables differ across a population.
`plant_probe`'s allele census is the model for what that should look like.
Until it exists, "the production rule is evolving" is an inference from the
mechanism rather than an observation.

## 4. Traps already paid for

**4a. The obvious guards are blind to the whole mechanism, and this was
established by putting the fault back rather than by reasoning.** With the
genome lookup in `fate_for` disabled outright — the mechanism completely dead
— both `a_founders_genome_answers_as_its_species_file_does` and
`a_determinate_species_terminates_its_axes_in_organs_and_an_indeterminate_one_does_not`
stayed **green**. `fate_for` falls through to the species table, and a
founder's two tables agree by construction, so every answer is identical
whichever source produced it.

The guard that is not blind is
`an_individuals_genome_shadows_its_species_table`, and it works by
constructing the case where the two *disagree*: a `tree` organism whose own
genome says a shoot node becomes a `Flower`, which `tree.ron` cannot say
anywhere. **Any future assertion about the genome has to be built the same
way** — agreement between a founder and its species proves nothing about
which one was read.

**4b. The caller's `Rng` position is a measured property with a guard over
it.** `bear_seed_at` draws its fate mutation from a keyed substream, never
from the shared `rng`, because that stream is `&mut`, outlives the call, and
one extra draw shifts everything the caller does afterwards — observable or
not depending on the order behaviours happen to sit in a `.ron`.
`set_seed_leaves_the_callers_rng_position_alone` is the guard; it is green and
must stay green.

**4c. `after_metamers: Some(0)` and `None` are the same rule.** The field
gates on `metamers >= n` and every lineage satisfies `>= 0`, so the packing
folds them and nothing can tell them apart. This is lossless, and it is pinned
by a test so a later reader does not "fix" it.

## 5. Where the disagreement stands

The direction is contested and the record should not be read as settled. In
`plant-evolvability-three-reviews-2026-08-27.md`, reviewers **A and B called
the structural-genome direction spent** — B proposed an organ-allocation
budget instead — while **C argued that verdict was never tested**, because
four of six discrete loci were hardcoded monomorphic in every founder. C's
prerequisite has since landed (Phase 0 of this programme), and the review's
unanimous "nothing can evolve yet" was partly overturned by the recruitment
measurement above. **This work backs C's side**, on the owner's instruction of
2026-08-29.

## 6. Also open on this branch

- The blind acceptance card for the organ package
  (`20260829T005132631Z-0b56d4`), asking *"are these different plants, or one
  plant in several sizes?"* over four panes, is **answered**: *"Different
  (although not very pretty)."* The organ package's core claim passes — four
  species read as four kinds of plant rather than one plant at four sizes.
  The caveat is aesthetic and is not this line's to close.

  **Read it against the rest of the plants board, though, because the pattern
  there is older and it is not aesthetic.** Across the earlier four-species
  cards the owner says the same thing repeatedly — *"the biggest differences
  are still size and color"*, *"most of the difference between all of these is
  just size and color"*, and, on the fourth attempt, *"I think the issue is in
  the base design of the random walk growth."* Organs moved that verdict where
  five successive label changes did not, which is `plant-appearance-design.md`'s
  finding restated: **the levers that read are the ones that change material or
  size, not the ones that relabel a cell.**

  That is worth carrying into any decision about the mutation operators
  (`plant-fate-operator-gate-2026-08-29.md`), because it cuts both ways. The
  three inert operators are the ones that would let a lineage *acquire* an
  organ — the one class of change the owner's eye has actually credited — so
  their inertness costs more than their share of the budget suggests. And
  equally: making them effective moves labels, and a label change has failed to
  read here six times now.
- ~~`bramble` may be renamed `scrambler`~~ — **decided: `scrambler`**, owner,
  2026-08-29. The argument that settled it belongs to this document's own
  subject rather than to botanical style: a species file is now a *starting
  point rather than an identity*, because rule tables mutate and a lineage
  keeps the name it was founded under. A growth-form name degrades gracefully
  when a lineage drifts; a taxon name (`bramble` is *Rubus*) becomes a false
  claim about what the thing is. `assets/species/scrambler.ron`'s header
  carries it.
- `windfall`'s `falls_through_organisms` is on and its effect is **unmeasured**
  — neither authored fruiting species has a crown deep enough to lodge a
  fruit in. The first tall fruiting species should measure it.
