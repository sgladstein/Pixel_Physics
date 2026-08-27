# Plant evolvability: verified ground truth (2026-08-27)

**Status: facts, not conclusions.** This document exists so that reviewers of
the plant-evolvability design question spend their budget on *judgement*
rather than on re-deriving the same engine facts — and so that no reviewer
has to take a previous reviewer's word for anything.

**It deliberately contains no recommendation, no proposal and no
preference.** Where a fact bears on a design argument, the argument is *not*
stated here. If you find yourself persuaded of a direction by this document,
that is a defect in it; please say so.

Every line below was re-verified in the tree at the commit this was written
on, by reading the source, not by trusting a report. Where a prior document
or review stated something different, the difference is noted with **CORRECTS**.

---

## 1. What the genome is today

| | |
|---|---|
| continuous slots | 10 (`GENOTYPE_TRAITS`, `organism.rs:1964`) |
| discrete loci | 6 (`DISCRETE_LOCI`, `organism.rs:2017`) |
| alleles per locus | `LOCUS_ALLELES = [2,3,3,2,2,3]` (`organism.rs:2059`) |
| discrete mutation rate | `DISCRETE_MUTATION_CHANCE = 0.03` (`organism.rs:2139`) |
| continuous mutation | per-slot additive jitter, `MUTATION_SIGMA`, in `set_seed` (`plant.rs:898-901`) |

The six loci: leaf economy, branch angle, internode, sympodial, tropism,
wood density (`organism.rs:2031-2048`).

**What each genetic channel can reach.** The continuous slots scale numeric
parameters of behaviours that already exist. The discrete loci scale
`branch_angle`/`internode`, flip `sympodial`/`tropism` booleans, and select
wood density and leaf economy. **Neither channel can add or remove a
`Behavior`, add a `CellType`, or change the arity of any table**: verified by
reading every consumer of `alleles` (`plant.rs:499, 508, 718-744, 906,
963-964, 1538-1590`) and of `genotype_draws`.

**Reproduction is asexual.** `set_seed(world, x, y, parent_id, seed_cost,
rng)` (`plant.rs:841`) takes a single parent and copies its species,
`genotype_draws` and `alleles`. There is no crossover machinery anywhere in
`plant.rs`.

**Reproduction is carbon-gated**, so differential reproduction exists:
`Behavior::Reproduce { seed_cost, seed_chance, seed_maturity }`, fired only
where `carbon >= seed_cost` (`plant.rs:4975-4984`).

## 2. `ByOrder` — what it actually is

```rust
pub struct ByOrder<T> { values: [T; BRANCH_ORDERS] }   // organism.rs:2433
pub const BRANCH_ORDERS: usize = 4;                     // organism.rs:2356
```

A fixed-arity array of four values, saturating at the last tier (`at()`,
`organism.rs:2439`). **Nine** `ByOrder` fields exist, all of them inside the
single `Behavior::Grow` variant:

`branch_chance` (:275), `light_weight` (:285), `upward_weight` (:296),
`plastochron` (:346), `branch_priming` (:372), `sympodial` (:555),
`tropism` (:559), `branch_angle` (:587), `internode` (:607).

9 fields x 4 tiers = ~36 values, several of them `bool`, `Tropism` or small
integers.

> **CORRECTS** the 2026-08-26 review, which said eight fields. Nine. Its
> derived figure of "~36 values" is right.

**Two of the nine are measured inert.** `Reports/plant-species-authoring.md`
§lever table: `light_weight` **flat / inert**, `upward_weight` **flat /
inert**. Same table, the two strongest: `plastochron` 3.9x on cells
(2459→634, inverted), `branch_chance` 2.5x (808→2018).

**Two of the nine use `0` as a sentinel for "unset, keep old behaviour"** —
`branch_angle` (`organism.rs:583`) and `internode` (`:607`).

## 3. What is *not* reachable by any genetic channel

`SpeciesDef::cell_types: Vec<(CellType, Vec<Behavior>)>` (`organism.rs:931`)
— which behaviours each cell type carries. Read at runtime through
`Species::behaviors(cell_type)`.

**Species identity is fixed for an organism and for the process.**
`OrganismState::species: SpeciesId` (`organism.rs:1375`). The registry is
built from an `include_str!`'d `EMBEDDED` list (`organism.rs:2149-2177`);
`upsert` is called only from `builtin()` and `reload()` (the F5 asset
reload). Nothing in `src/` or `examples/` creates a species at runtime.

**`CellType` is packed into 4 bits of `Cell::aux`** (`organism.rs:59-64`):
16 possible variants, **8 currently used** — `Seed=0, GrowingTip=1,
MatureBody=2, Leaf=3, RootTip=4, DormantBud=5, Head=6, Segment=7`
(`organism.rs:73-159`), two of them creature-owned. **8 free.**

> The doc comment at `organism.rs:64` says "room for 11 more variants than
> are named yet". That is **stale** — it was written when 5 were named. 8 are
> named now, so 8 remain.

## 4. Blast radius, measured

| | count |
|---|---|
| `.species.get(` in `plant.rs` | 31 |
| `.species.get(` across `src/` | 50 |
| `plant.rs` total lines | 11,387 |
| `plant.rs` landings (CLAUDE.md collision table) | 60 |

**Generic behaviour dispatch — where `cell_type` is a variable — is two
sites**: `plant.rs:1351` (active-site path) and `plant.rs:4855` (sweep path).
Both are `world.species.get(species_id).behaviors(cell_type)`.

> **QUALIFIES** the 2026-08-26 review's "the dispatch is only two sites".
> True for the generic tick dispatch. But a further ~18 sites read a
> species' behaviours for a *named* `CellType` (`plant.rs:734, 3897, 4036,
> 4040, 4221, 4391, 4489, 4502, 4512, 4578, 5729, 7767, 8056, 9622`, and
> `organism.rs:1253, 1303, 3122`). A change to where behaviours come from
> reaches those too.

`MAX_BEHAVIORS_PER_CELL_TYPE = 8` (`plant.rs:1053`), and `Behavior` is
`Copy` specifically so the dispatch buffer is a stack array
(`plant.rs:1349`); the `Vec` it replaced cost "roughly 350,000 allocations
over a 6,000-frame run" (`plant.rs:1342-1349`).

## 5. Population and persistence limits

- **4,095 concurrent organisms.** `ORGANISM_INDEX_BITS = 12`
  (`world.rs:29-39`). Births past it are refused and counted
  (`World::organisms_refused`), not fatal.
- **Slot generation wraps after 16 reuses** (`GENERATION_MASK`,
  `world.rs:48`).
- **There is no world save/load.** The only `save()` paths are tunables
  (`clock.rs`, `player.rs`, `explosion.rs`, driven from `app.rs:1044`).
  Worlds are generated from a seed. **No persisted format exists to
  migrate.**

## 6. Determinism constraints

- `PLAN.md` requires same-build determinism.
- `set_seed` consumes **one shared-`Rng` draw per genome slot**, so genome
  width changes shift every later draw. `SEQUENCED_TRAITS` (`plant.rs:816`)
  freezes the measured prefix; an appended slot draws after the loci, and
  `APPENDED_JITTER_SALT` (`plant.rs:826`) gives it its own substream.
- `Chunk::rng` is seeded from chunk coordinates, so **the same genome planted
  in two places draws a different sequence** — position is a hidden inherited
  variable (`plant-simulation-research.md` §7d; independently recorded as
  creature gotcha **P-21**: "a fitness-relevant RNG keyed on position makes
  location heritable and manufactures fake selection results").
- **An unstable sort's tie order depends on the element type**, and
  `plant.rs`'s `allocate_to_frontier` is subject to it — caching a sort key
  changed tree heights 101→103 and root depth histograms (CLAUDE.md gotcha).

## 7. What has been put in front of the owner, and the verdicts

**WP-C's three form probes, 2026-08-21** (`plant-evolution-design.md:467-515`),
two posted blind:

| card | verdict | outcome |
|---|---|---|
| form probes (gallery) | *"Tree and weeping look the same. creeper and prostrate look the same"* | — |
| `tree` vs `weeping` (**blind**) | *"same plant"* | retired |
| `creeper` vs `prostrate` (**blind**) | *"Not that different"* (2/5) | prostrate retired, file + 4-line code change |

What each probe moved: `weeping` moved **only** `upward_weight` on orders ≥1
(a `ByOrder` field). `prostrate` moved order-0 `Plagiotropic` + tiny
`internode` (both `ByOrder`). `creeper`/`prostrate` both cut `turgor_source`
to a 5-8 row budget.

The report's own summary: *"across three probes, **every group change came
from the size budget and none came from an architectural knob**."*

**Earlier, `plant-appearance-design.md`:** sympody, tropism and acrotony all
fired with counters printed (46-186 sympodial forks/shrub; 1,797-2,750
plagiotropic steps/conifer) and the owner's reading was that nothing had
changed. All species measured ~90% wood / ~5% leaf, drawing from one
four-brown and one four-green palette.

**`grass`**, which moved size *and* material *and* consequence, came back
*"looks different from trees"* at **4/5**.

**Also measured** (`dead-ends.md:567`): fighting canopy fusion by raising
upward bias or adding trunk branching made the slab *worse*; only lateral
crowding opposed it, at `crowding_weight` 0.5 → 6.0.

## 8. Prior decisions on the shelf

**D4** (`dead-ends.md`, creatures): NEAT-style topology evolution rejected
for the creature brain — "hours-of-noise bootstrap, illegibility, and every
downside traced to topology mutation". Re-test line: *"recorded as settled
with the owner (2026-08-17); re-litigating requires an owner decision, not
new measurement."*

**The reason D4 gives, in `brain.rs`'s own words** (quoted at
`creature-direction.md:663`): *"No crossover in v1: reproduction is asexual
(queens). Crossover is compatible later because every genome shares one
scaffold — that compatibility is the entire reason topology mutation was
rejected."*

**P-20** (`creature-direction.md:848`): *"degenerate attractors are the
default outcome, not an edge case — spinning in place, sessile freeloading,
exploit-the-energy-bug. Build [the instrumentation] before mutation is
switched on, because afterward every anomaly is ambiguous between 'bug' and
'adaptation.'"*

## 9. Literature already surveyed in-repo

`Reports/plant-simulation-research.md` §7. Reviewers should read it rather
than this summary; three points are recorded here only because they have been
misquoted at least once:

- **§7a names two genome levels.** Parameter vector ("safe, always produces a
  viable organism") and **structural** ("which `Behavior`s each `CellType`
  carries, and what the cell types transition into. Richer, and where
  genuinely novel body plans would come from, **but most mutations produce
  nonviable organisms**"). Reference given for the latter: Ochoa, *On Genetic
  Algorithms and Lindenmayer Systems* (PPSN 1998).
- **§7b: Niklas's adaptive walks** — "the canonical plant-evolution
  simulation" — run on a **six-variable** morphospace (two branching
  probabilities, two rotation angles, two bifurcation angles), i.e. a
  *parametric* genome. Its stated most valuable result: *"multi-task fitness
  landscapes have many near-equal optima; single-task landscapes have one …
  selecting on at least three conflicting tasks is not a nice-to-have; it is
  the mechanism."*
- **§7c: Bornhofen & Lattaud (2009)** is called *"essentially the target
  system, minus the physics"* — L-system morphology plus transport-resistance
  physiology, mutation on both the L-system and a parameter set.
  **Bornhofen, Barot & Lattaud (2011)** got Grime's CSR strategies to
  *emerge* given heterogeneous resources and varying disturbance. These are
  **one group, two papers**, not two independent replications.

`Reports/tree-procedural-prior-art.md` §3 is headed *"L-systems — least
transferable, because the engine already is one"*; its "nothing to port"
refers to L-system *theory* buying this engine nothing. Its §4 records that
*"every model surveyed starts from an authored seedling, not a single cell"*
and that trunk/crown separation is *"enforced by construction in every case,
never emergent"*.

## 10. Instruments that already exist

- **`divergence`** — *"Does one environmental difference produce two
  different-shaped plants?"* **Axis-agnostic**: "adding an axis is one arm on
  `Axis` and nothing else". Already answers *"does a new genome locus move
  morphology at all?"* (`instruments.md:92`). Has a `control=1` exact-zero
  check.
- **`genome_drift`** — per-slot population mean over generations; warns below
  generation 2.
- **`plant_probe`**, **`crown_census`**, **`flora_census where=/at=`**,
  **`litter_probe`**, **`root_contact`**. `flora_census where=` exists
  because a card once came back "I don't see a difference" and the window
  held 125 grass cells against 7,853 woody.

**Within-stand spread is large**: one measured stand ran root cells min 90,
median 438, max 1,435; twelve identical genomes span 31-153 cells. Stand-
against-stand comparison of a morphology claim sits inside that variance.

## 11. An existing identity-guard precedent

`plant.rs:7709` `expressing_the_appended_genome_slot_changes_no_plant` grows
one stand twice in a process — appended slot expressed, then at zero width —
and requires the two bit-identical (hashing material/aux/organism_id per
cell, plus `genotype_draws[..9]`, `alleles`, `generation`).

It carries its own anti-vacuity control (`plant.rs:7696`): *"**Confirmed not
vacuous** … pointing the turgor read at slot 9 (one character) makes the arms
disagree and the test red. Re-run that if this is rewritten."*

> **CORRECTS** a claim made by the lead session on 2026-08-26 that this guard
> was the blind shape CLAUDE.md warns about and needed re-arming. It is the
> *cured* one and names its own re-arming recipe. CLAUDE.md's general warning
> about stand-fingerprint guards stands; this particular test already answers it.
