# Handoff: can an individual reach as far as a species?

*Written 2026-09-05 at the end of the `plant-engine-rethink` session that
measured developmental noise. The prompt below is the owner's question,
written to be pasted into a fresh session.*

---

## The prompt

> Read `Reports/plant-engine-rethink-2026-09-03.md` §6.13 and
> `Reports/lanes/plant-genome-reach-handoff-2026-09-05.md`, then take on the
> question below. Same standing rules as the plant-engine brief: act
> autonomously, reconsider closed decisions, measure before you build, land
> what you are sure of, and post review cards rather than blocking.
>
> **The owner's question, in their words:** *"If cloned genomes grow in
> identical environments and have so much variation, then we will never be
> able to see the variation from actually intentionally changing the genome or
> the environment. I saw a test (the two most different herbs this genome can
> make, alternating). They looked different, but not that different. Not as
> different as an herb and a tree in the game. Aren't trees and herbs based on
> the same genome plant engine?"*
>
> **Two claims are packed in there and both are testable.**
>
> 1. **Developmental noise swamps genetic signal.** Measured last session:
>    twelve clones of one genome, each alone in an identical flat bed with
>    pinned clear weather and no breeding, differing only by which column the
>    seed landed in, span **151 to 3,368 living cells at 20,000 frames** — and
>    two clones in the *same* column are cell-for-cell identical (Jaccard
>    1.0000). So the spread is real, it is produced by a one-column move, and
>    it is larger than what the widest within-species genetic contrast
>    produces. If that holds, **selection cannot see the genome and neither
>    can the player**, which is the whole objective failing.
> 2. **Species differ enormously; individuals barely.** Both use the same
>    engine, so the owner's inference is that individuals *should* be able to
>    reach further than they do.
>
> **Your first deliverable is one table nobody has made: the between-species
> span and the within-species span, on the same quantities, from the same
> instrument.** Do not propose a fix before that table exists.

---

## What was verified on 2026-09-04, so you need not re-derive it

**The ten genome slots are rates and biases, and none of them is what makes a
tree a tree.** `organism::GENOTYPE_TRAITS == 10`, and the slot map (positional
forever, `Reports/plant-genome-design.md`) is:

```
0 shoot branch chance        5 root tropism gain
1 root branch chance         6 root:shoot allocation bias
2 shoot plastochron          7 stomatal closure point
3 turgor per cell            8 root penetration force
4 pipe ratio                 9 strain-response gain
```

**`genotype_variance` is near-identical across every plant species.** herb's
shoot `Grow` carries `(0.4, 0.0, 0.35, 0.18, 0.6, 0.0, 0.4, 0.5, 0.0, 0.7)`
and tree's `(0.5, 0.0, 0.4, 0.18, 0.7, 0.0, 0.4, 0.5, 0.0, 0.7)`. So the
*width of individuality* is authored the same for a herb and a tree, and it
multiplies each species' own mean.

**What actually separates a herb from a tree is the authored species table** —
the `Behavior` graph and its parameters, `seed_maturity` 600 against 60, the
cell types, the architecture. **None of that is in the genotype.** That is the
mechanical answer to the owner's question, and it is the thing to put in front
of them early.

**`ParamGenome` is the machinery that could close the gap, and it is shipped
inert.** ~43 parameters over 804 heritable addresses, per-individual overrides
on the species behaviour table, with `param_scale` taking its units from the
**corpus max across species** — i.e. it is already dimensioned to let an
individual reach toward another species' authored value.
`plant::PARAM_MUTATION_CHANCE` is **0.0**.

**A slot can have a width and do nothing.** Slots 1 and 5 were
`upward_weight` and `light_weight`, **measured inert across 1,024 genomes at
±40% / ±50%**, and are held at zero width in every species today. Assume
nothing about a slot's reach without measuring it.

## The laws this question runs into

- **A free lever made heritable produces uniformity, not diversity**
  (`plant-heritability-survey-design-2026-08-27.md` §2). A quantity with a
  benefit and no counterweight has one optimum and selection pins every
  individual at it. So *widening* `genotype_variance` or raising
  `param_mutation_chance` is not automatically more variety — it can be less.
  The free-lever audit completed 2026-09-04: `seed_launch`, `seed_maturity`
  and the turgor ceiling are now priced, `heading_inertia` was declined with
  reasons. Nothing on §5.4's list is unpriced.
- **Do not raise `param_mutation_chance` off 0 without first running
  `genome_reach -- drift=1`.** The addresses that pile up at their
  `clamp_param` bound are the free-lever list *measured* rather than
  inventoried.
- **A lever that relabels a cell cannot move a silhouette that texture and
  colour set** (`Reports/plant-appearance-design.md`). Three architectural
  levers fired, were counted, and changed nothing anyone could see. Before
  ranking any lever by expected visual impact, ask which *pixels* it moves.

## Instruments that already exist — read `Reports/instruments.md` first

- **`clone_identity`** (new 2026-09-04) — asks identity rather than variance.
  Divergence frame plus a translation-invariant Jaccard over each plant's
  cells relative to its own collar. Modes: `solo=1` (each clone in its own
  world), `clear=1` (`Pin::Clear`), `sterile=1` (no breeding), `dev=`,
  `columns=a,b,c` (one tree per world per column, organism id held fixed),
  `png=`. **Run `columns=256,256` first** — two arms at one column must read
  1.0000 or every number after it is noise.
- **`clone_variance`** — broad-sense heritability per shape descriptor, with
  `spread` as the estimator's sensitivity control. **Check the `H2 (control)`
  row is non-zero before believing any `H2` row.**
- **`genome_reach`** — what phenotypes a species' genome can actually reach.
  This is the instrument the owner's question is really about.
- **`plant_probe`** — per-cell channels, the generation clock, effect counters.

## Four traps that cost real time on 2026-09-04

1. **A render window that cropped the roots** turned a 22x difference in cells
   into about 1.5x on screen, and the panels were read as "these all look the
   same". Render the whole plant before comparing plants.
2. **A census reported "unowned plant cells: 0" and could not have returned
   anything else** — `deadwood` and `deadleaf` are `MaterialKind::Powder`, not
   `Plant`. An unvalidated null over exactly the tissue in question.
3. **Measuring at 6,000 frames, where a `tree` is a seedling.** It produced a
   tight band (28 of 33 within 15%) that vanished entirely at 20,000 frames (3
   of 12). Age the plants before quoting a distribution.
4. **A small cell count beside a big picture is a plant that died back**, not a
   bookkeeping fault: `cells` counts living organism tissue, the render shows
   living and dead, and `rot_remains` leaves a standing skeleton.

## Open, and the owner's to judge

Review cards `20260904T175736809Z-b982e8` (twelve clones one column apart),
`20260904T071211409Z-ad25be` and `20260904T054136294Z-99adef`. The question on
the first is whether the variation reads as a bug or as a species that varies
— **if the owner says it reads as a species, this whole line stops there.**
Collect with `python3 scripts/review.py inbox` before doing anything else.
