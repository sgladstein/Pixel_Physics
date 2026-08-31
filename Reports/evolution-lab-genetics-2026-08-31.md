# The specimen shelf: keeping, cloning and mutating an individual

*Design and build, 2026-08-31, branch `claude/evolution-lab-genetics-gw4cv3`.
Owner brief: **"we need to be able to save genetics of creatures and animals
clone them or mutate. expand on this idea."** This report is the expansion,
and §4 is what shipped with it.*

Reads on: `Reports/lanes/evolution-lab-coordinator.md` (the standing
direction), `Reports/creature-export-design.md` (the other exit, and the
positional-genome law), `Reports/evolution-lab-design-guide-2026-08-30.md`
(decision E8).

---

## 0. The decision, stated first

**A specimen is one individual's heritable identity, kept in a file that
outlives the box, and put back into the world as itself or bred forward.
Clone and mutate are one verb with a number on it, not two verbs.**

Three things follow, and each of them is a decision that could have gone the
other way:

1. **A jar is a genetics record, not a species file.** `species_export`
   already writes an individual out as a whole `assets/species/<name>.ron`.
   That is a different exit and both are wanted — §2.
2. **Both kingdoms.** A plant's genome is a different shape from an animal's
   and the code had no name for either as a unit. §1.
3. **The dial is counted in broods and applies the engine's own per-birth
   mutation once per brood.** No new rate is invented, and none is
   calibrated. §3.

---

## 1. What "the genetics of an individual" actually is

Nothing in the engine had a name for this, and asking the question turns up an
asymmetry that matters more than it looks:

| | a plant carries | a creature carries |
|---|---|---|
| continuous | `genotype_draws`, 10 traits on `-1..=1` | `traits`, 2 (gut bias, birth grant) |
| discrete | `alleles`, 6 loci that **jump** rather than drift | — |
| program | `fates`, its own production rule (`FateGenome`) | `genome`, 12,352 brain weights |
| not heritable, but individual | `flower_band`, `fruit_band` (no locus yet) | — |
| derived, so never stored | `foliage_band`, `bark_band` (from the alleles) | — |

Everything else about the animal — body plan, metabolism, palette, dig force,
half-lives — is its **species'**, and a jar has no business duplicating it.
That is what lets a jar be small and it is also its one real cost: **a jar
whose species has been renamed or removed cannot be released**, and says so.

Two things in that table are worth pausing on.

**A plant's production rule travels in the jar.** `FateGenome` is the layer
`plant-morphology-evolvability-2026-08-26.md` §6 marks as the one to replace —
*"species as an outcome, not an input"* — and it is per-individual. So keeping
a plant keeps its *growth program*, not merely its numbers, and a released
specimen grows the shape the one you pointed at grew. Nothing before this
could hold that outside a running process.

**The organ colours are stored rather than derived, and that is a gap wearing
a workaround.** `flower_band` and `fruit_band` have no locus (their own doc
records the omission), so they are redrawn per individual and cannot be
inherited. Deriving them at release would give a released specimen different
flowers from the plant that was kept — visibly not the same plant — so the jar
carries the bytes. **The day those get a locus, delete the two fields**; until
then they are the honest form of a known hole rather than a design.

---

## 2. Two exits, and why the lab wants both

`species_export` landed 2026-08-29 for decision E8 — *"we can use it to create
new creatures that get saved and added to the game"*. It writes a complete
species file the game's own loader reads. It is the right shape for that
sentence and the wrong shape for a working session:

| | `species_export` | the shelf |
|---|---|---|
| writes | a whole species | one individual's genome |
| covers | creatures only | **plants and creatures** |
| needs | a paired `assets/materials/<name>.ron` before anything hatches | nothing; it reuses its species' |
| reaches the world by | an `include_str!` line and a rebuild, or F5 | **being released, now** |
| on a name collision | refuses, so it cannot eat a hand-authored file | numbers up, because a jar is the player's |

**Size is not one of the differences, though the first draft of this said it
was.** A plant jar measures **2,929 bytes** and a *generated* ant species
**2,280** — so "a jar is smaller" is false. `assets/species/ant.ron` is 37 KB
because of its comments, which a generated file has none of; comparing the two
measured the documentation rather than the format. The rows above are the
whole of the difference.

So: **the shelf is the working set and the export is the way out of the lab.**
`PROMOTE` on the shelf page is the seam between them, and it is one button
because they were always the same pipeline with a missing first half.

---

## 3. The dial: one verb with a number, not two verbs

The brief says *"clone them or mutate"*. Built literally that is two buttons,
and two buttons is the binary `CLAUDE.md`'s first law rules against: **an
outcome is a distribution, not a binary.** A rack that offers *exactly this
animal* or *a fresh roll* has the same defect the old rubble had — nothing in
between, and the middle is where the interesting cases live.

So there is one verb and a dial, **counted in broods**:

| dial | means |
|---|---|
| 0 | that exact individual again |
| 1 | as different as its own child would have been |
| 3 | a great-grandchild's worth of drift |
| 8 | the ceiling — past which the relationship is no longer visible |

**Each brood applies the engine's real per-birth mutation once**, and that is
the whole reason the unit is an integer. For a creature: `brain::mutate` at
the species' authored `mutation_rate`, then the trait jitter at its
`trait_variance` — the two lines `creature::bud` runs. For a plant:
`plant::genotype_jitter` per continuous slot, `organism::jump_alleles` over
the loci, and one `FateGenome::mutate` roll at `fate_mutation_chance()` — the
three `plant::bear_seed_at` runs.

**A dial that scaled a rate would have been a new knob with no measured
meaning**, sitting next to a shared budget it reallocates — the failure
`CLAUDE.md` records under *"a term in a weighted sum is not an independent
knob"*, and the one
`Reports/why-changes-cost-so-much-2026-08-27.md` is about. Applying the
shipped operator n times is a quantity the engine already has an answer for,
and the answer moves automatically when the owner turns `mutation_rate` on the
parameters page. That coupling is deliberate: the dial is *how many
generations*, and the parameters page is *how big a generation is*.

**To keep it one operator, two of those were extracted rather than copied.**
`genotype_jitter` and `jump_alleles` were lifted out of `bear_seed_at` and are
now called from both the breeding path and the shelf, so the two cannot drift
apart — the failure `bear_seed_at`'s own doc opens by naming (*"two lineages of
inheritance drifting apart"*). The extraction is behaviour-preserving and §7
records how that was proved rather than asserted.

---

## 4. What shipped

**`src/sim/specimen.rs`** (1,113 lines with tests) — `Specimen`, `Genetics`,
`capture`, `drift`, `release`, and the shelf's file I/O. 19 tests.

**Two engine seams that did not exist**, and both are one-line consequences of
a question nobody had asked:

- `creature::Origin::Stock` — **a founder with a chosen genome.**
  `Origin::Founder` reads `Species::genome`, the ancestral one, so before this
  the *only* way to put a specific animal in the world was `Origin::Bud`,
  which needs a live parent and charges it. A release is economically a
  founder in every respect (fresh lineage, generation 0, a founder's
  endowment) and is booked as a spawn, never a birth — booking it as a birth
  would put energy in the ledger nobody earned and would count a player's
  decision as a reproductive success, which is the one number the lab reads.
- `plant::sow_specimen_seed` — the same for a seed, and the load-bearing line
  is `inherited = true`: `seed_genotype` redraws a genotype from the
  germination coordinate unless told not to, so without it a jar would
  round-trip perfectly into a seed and be erased one tick later. There is a
  test that calls `seed_genotype` directly, because that is the function that
  would do the erasing.
- `OrganismState::stocked` — a third origin, so the cell readout can say
  `RELEASED FROM A JAR` rather than reporting a player's own release as
  something the box bred.

**The lab**: two tools (`KEEP` M, `FREE` ,), a rack page (`G`), a brood dial
(`;` `'`), and four page verbs — `COPY` (breed the armed jar on the shelf,
without releasing it), `DISCARD`, `PROMOTE`, `RELOAD`. Plus a jar chip on the
bar that names what `FREE` will put back, on the species chip's own argument:
the design guide says planting has to show what you are about to plant *"or
planting is a slot machine"*, and that is true with more force here, because
two jars are the same two dark cells on screen and differ only in the genome
nobody can see.

**One law across all of it: nothing destroys a specimen except `DISCARD`.**
Keeping never overwrites (the save refuses, and `next_free_name` picks the
next stem), `COPY` leaves its parent standing and armed, and `FREE` does not
consume the jar. A kept specimen is the one thing in the lab a player cannot
regenerate — the box moves on and the individual dies — so the only way to
lose one is to say so.

Measured through the real click path (`examples/labui.rs`, counters beside
each tile because a jar is a file and a freed ant is two dark cells):

```
KEEP:   jars 0 -> 1   -- "KEPT HERB -- PLANT G0 -- 1 ON THE SHELF"
COPY:   jars 1 -> 2   -- "HERB DRIFTED 3 BROODS INTO HERB_2 -- 31 GENOME SLOTS MOVED"
FREE:   organisms 90 -> 91 -- "RELEASED HERB -- 3 BROODS, 32 GENOME SLOTS MOVED"
```

---

## 5. What this makes possible that nothing else did

This is the part that is worth more than the feature, and it is why the brief
was worth expanding rather than implementing literally.

**5a. An experiment in this bed can now be repeated.** Today it cannot be, and
the reason is structural rather than incidental: `seed_genotype` keys a
founder's genome on `(world seed, germination coordinate)`, so two runs of
"the same" experiment start from *different plants* unless every seed lands on
the identical cell. A jar fixes the founder. Two boxes seeded from one jar
differ only in what the experimenter changed — which is the definition of a
control arm, and the lab has not had one.

**This bears directly on Gate 2**, which the coordinator note records as never
run and as invalidating every evolution result in the bed until it is: *does
selection have teeth here?* `selection_arena`'s finding is that a null there
is a statement about the *world* rather than the genome — and telling those
apart needs the genome held fixed while the world changes, which is exactly
what a jar is for.

**5b. Artificial selection, by hand, with no further engine support.** Keep
the best of a generation, `COPY` it a few times at one brood, release the
copies, run, keep the best again. That is a breeding programme, it is the
owner's own *"I can figure it out myself"*, and every piece of it now exists.
`from_jar` records what was selected at each step, so the rack **is** the
pedigree — and it is the only place the player's own selection decisions
survive a reset of the box.

**5c. The loop back to the outdoor game.** E8 in one sentence: evolve, keep
the good ones, promote them, and they are the outdoor world's animals.
`PROMOTE` closes it for creatures.

---

## 6. What is deliberately not built, ranked by what the machinery supports

**6.1 `CROSS` — breed two jars together. Cheapest real expansion, and the
architecture was built for it.** Everything on the shelf today is asexual: a
jar drifts from one parent. But `creature-direction.md` D4 caged the brain's
topology on **one shared scaffold** and says so in as many words —
*"crossover compatibility across one shared scaffold is the entire reason"* —
so two creature genomes are aligned by construction and a uniform crossover is
a per-slot coin flip over two `Vec<f32>`s. The plant side is the same shape:
positional draws, positional loci, and a fate table where the honest operator
is picking whole rules from one parent or the other. This is a third verb on a
page that already has the rack, the two jars and the dial, and it is the one
that turns a rack into a population.

**6.2 Promoting a plant.** `PROMOTE` refuses a plant today and says so.
`species_export::individual_as_species` copies the parent species' `fates`
rather than the individual's, so promoting a plant would write a file that
parses and grows the *wrong* plant — a silent failure, which is why it refuses
loudly instead. The fix is small (write `state.fates.to_table()`) and it needs
a round-trip test of a *drifted* rule table, which is the part that has never
been exercised.

**6.3 Comparing two jars.** The rack shows a name, a species and a generation.
It cannot say how far apart two jars are, which is the question a breeder asks
constantly. A count of differing slots is arithmetic the shelf already does —
`drift` returns exactly that number — pointed at two jars instead of one.

**6.4 A jar of a colony.** Everything here is one individual. A colony's
interesting property is the *distribution* of its genomes, and keeping one ant
out of eighty says nothing about it.

**6.5 Auto-keep.** "Keep the best N each generation" is 5b with the hand taken
out. Deliberately last: the owner's direction is *"give me the tools… I do
that testing myself"*, and automating the selection decision is taking back
the one thing they asked for.

---

## 7. Things that cost time, recorded so they do not again

**A guard that is green can be blind, and two of the three here were —
measured, not assumed.** The extraction in §3 changes code inside
`bear_seed_at`, whose draw order is a measured record with a guard over it. The
first fault injected to test that guard *was itself wrong*: it moved the
`below()` draw into an `if` where it was already conditional, changed nothing,
and both guards stayed green — which reads exactly like "these guards are
blind". Injected properly (drawing unconditionally, one extra draw per locus),
**both went red**, which is what makes the extraction's green worth citing.
`CLAUDE.md`: *before you cite a guard's green as evidence, put the fault it is
named for back and watch it go red* — and the corollary this adds is that **a
fault injection is itself a thing that can silently not fire**, and the tell is
the same one everywhere else: the number did not move.

**The contact sheet found a bug no test would have.** `COPY` writes a jar and
reloads the rack, and the first `reload_shelf` cleared the selection — on the
true observation that an index is not a name. So `COPY` disarmed the jar it had
just bred from, and the next `FREE` refused. The tile showed a bed with a plant
in it either way; only `FREE: organisms 89 -> 89` said the release had done
nothing. Re-finding the armed jar **by name** fixes it and is strictly better:
the selection survives any change to the rack that keeps the jar, and clears
exactly when the jar is gone.

**The bar is now full, and this is a handoff item.** Measured with
`PIXEL_PHYSICS_BAR_TRACE=1`: row 1 sat at **exactly** its own width (508 of
508) before this work and still does; row 0 had **76 px** of slack at
comfortable spacing and this feature needed more than that. `RELEASE` as a
label plus a jar chip did not fit at any spacing — hence `FREE`, which is also
better vocabulary — and the bar now fits **only at its tightest padding, with
1 px to spare**. `SPACINGS` is the mechanism for exactly this and the guard
passes, but the next lab feature genuinely cannot go on this bar. It needs a
third row, a page, or something taken off.

**Two things a rendered page said and no assertion could**: `DRIFT` appeared
as both the dial's label and a button and read as one control drawn twice (the
button is now `COPY`), and the dial's `+` sat flush against the first verb —
fixed by *measuring* the strip's own width into the page width rather than
eyeballing it, which is `param_page_width`'s rule.

**The shelf directory is overridable (`PIXEL_PHYSICS_SHELF_DIR`) and the tests
that use it hold a mutex.** An environment is per *process*, and `cargo test`
runs a module's tests on several threads in one — two shelf tests without the
lock would each point the override at their own directory and then read each
other's rack, which is a flake that depends on thread scheduling. The override
exists at all because `/tmp` and the checkout are both shared between agents in
this project's containers.

---

## 8. Open questions for the owner

1. **Is the dial the right way to offer clone-or-mutate**, against separate
   `CLONE` and `MUTATE` buttons? Posted as review card
   `20260831T024841134Z-52020a`.
2. **`CROSS` (§6.1) next, or `PROMOTE` for plants (§6.2)?** The first turns
   the rack into a population; the second closes E8's loop for the half of the
   biosphere it currently excludes.
3. **Should a released specimen keep its recorded generation** rather than
   restarting at 0? It restarts today, because it is a founder and a fresh
   line. The argument the other way is that a breeder's generation count is
   the number they care about, and the shelf currently keeps it only in the
   jar's provenance.

---

*Freshness: written 2026-08-31 against `claude/evolution-lab-genetics-gw4cv3`.
Every number in §4 and §7 was measured on that branch on that day.*
