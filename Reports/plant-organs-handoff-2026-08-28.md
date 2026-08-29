# Phase 4 handoff: organs, priced, with their materials (2026-08-28)

**Status: handoff. Read this before starting the organ work.** Written so the
conclusions reached across Phases 0–3 are on disk rather than in a transcript.

The programme is `/root/.claude/plans/` Phase 4 of a six-phase plan; Phases 0–3
are landed on `claude/plant-morphology-evolution-d7i1od` (8 commits, merged up
to `main`, CI-relevant gates green throughout).

## 1. What Phase 4 is

Two new organ cell types with **their own materials**, a determinate axis that
terminates in one, and a carbon price on building them. The acceptance artifact
is a blind review card, not a test.

**It is not a new growth engine.** The production rule is already data
(`SpeciesDef::fates`); organs are new *values* in a table that exists, plus the
materials and the bill.

## 2. What landed in Phases 0–3, and what it gives you

| commit | what |
|---|---|
| `94f3cfd` | corrected a false RNG-confound claim carried by five reports |
| `d3b747b` | founder variance on the four frozen discrete loci |
| `3b4c39e` | **the production rule became data** — `SpeciesDef::fates` |
| `8814f78` | heterogeneous bed (`Relief::Varied`) + neutral `Hazard` |
| `1f0f072`, `d94c7ed`, `76a0097` | gate 1: the viability measurement |

New machinery you should use rather than rebuild:

- **`organism::Fate { when, becomes, child, lateral }`** and
  `FateWhen { Grew, Node, Stale, Flush }`. Lookup is
  `plant::fate_for(world, species, cell_type, when)`; `plant::builtin_fate`
  holds the pre-table behaviour and is the fallback for an unauthored species.
- **`SpeciesRegistry::register_ron`** — register a species from RON at runtime.
  Nothing could create a species before this.
- **`PlantScene::species_ron`** — a runtime species registered *inside* `build`,
  before it plants. See §5.1 for why that ordering is a field and not a habit.
- **`PlantScene::varied()` / `Relief::Varied`**, `common::Hazard` +
  `apply_hazard`, `filmstrip scene=gradient`, `plant_probe relief=varied
  hazard=P`.
- **`examples/fate_viability.rs`** — mutate a production rule N ways and
  classify viable / lethal / **silent**.

## 3. Decisions already taken. Do not re-litigate these.

| decision | who, when |
|---|---|
| Fruit **carries** the seed (dispersal), not converts in place | owner, 2026-08-23 |
| Vines may attach to player-built walls | owner, 2026-08-23 |
| Annuals, on herb archetypes only | owner, 2026-08-23 |
| Structural genome goes as far as **fate table + behaviour sets** | owner, 2026-08-28 |
| Organs are an **additional** seed path; mature-cell `Reproduce` stays | owner, 2026-08-28 |
| Construction is charged **at the decision**, per-tissue coefficient | owner, 2026-08-27 (costs §10a) |

## 4. The two findings that should shape the build

**4a. Gate 1 says organs are aimed at the tolerant half of the rule.**
(`plant-fate-viability-2026-08-28.md`.) Of 40 effective mutations to tree's
production rule, 37 still produced a living plant. Every failure was the same
kind:

| field | mutations | sterile |
|---|---|---|
| `child` | 6 | **5** |
| `becomes` + `lateral` | 34 | **0** |

A determinate axis ending in an organ is a **`becomes`** rule; a truss is a
**`lateral`** rule. Both are in the class that took 34 mutations without one
failure. `child` — the field that kills — is the one organs never need to touch.

**4b. A label change does not read. Five times now.** The owner's verdict on
founder variance (card `20260828T131806922Z-cb5be9`) was *"These look almost
identical. Cannot tell any significant difference"*, joining `weeping` (*"same
plant"*), `prostrate` (*"Not that different"*), and sympody/tropism/acrotony,
all of which fired with counters printed and moved nothing anyone could see.

The one lever that ever read — grass, **4/5** — changed size **and material**
and consequence.

**So the material is not a detail of the organ; it is the organ.** A `Flower`
cell that draws from the existing four-green foliage palette will read as a
leaf, the counters will say it fired, and the card will come back "no
difference" for the sixth time. `assets/materials/flower.ron` and `fruit.ron`,
with their own palette bands, are the load-bearing half of this phase.

## 5. Traps, each already paid for

**5.1 `#[test]` in `examples/` is never collected.** A guard written there is
green because it never ran. Use a runtime `assert!` in the builder instead —
`PlantScene::build` has one for the stone floor, with the reasoning in place.

**5.2 A species `.ron` edit does nothing until the next build** (`include_str!`).
Identical output across settings is the tell. `cargo build --release
--examples` with `set -o pipefail` and read `${PIPESTATUS[0]}`.

**5.3 Ripening does not go in `aux`, and `aux` is not a colour readout.**
`render.rs:3676` reads `palette[cell.shade …]`. `shade` is already spent on
band identity plus grain (`plant.rs`'s `banded_shade`). Repacking `aux` is
against standing policy (`organism.rs:2522-2528`). Put a ripening stage in the
`OrganismCell` sidecar and get colour from the ripe fruit being its own
material. **Reach report §2a says otherwise and is wrong on this point.**

**5.4 Cover `FateWhen::Stale`, or determinacy silently produces no organ.**
A terminal tip more often ages out than grows one last time, and staleness is a
different site (`plant.rs`'s `organism_tick`). The symptom is a low counter with
no visible cause.

**5.5 `FateWhen::AfterMetamers` does not exist yet, deliberately.** Adding a
condition no species uses would be a channel with a writer and no reader — the
failure this repo has shipped three times. Add it *with* the species that needs
it.

**5.6 Inserting a doc block above a struct can orphan its `#[derive]`.** Hit
twice in Phase 1. Check what precedes your anchor.

**5.7 Controls caught two harness bugs that inspection did not.** In one, the
*positive* control reported the unmutated table as 0/3 — a registration
ordering bug that would have published a decisive, false 0%. Run the control
that must be non-zero, every time.

## 6. The retune budget, named rather than asserted

Costs §9a is the precedent: *"absorbed is not calibrated."* Charging carbon for
organ construction and paying upkeep on organ cells reallocates:

| constant | site |
|---|---|
| `INCOME_PER_NODE` | `plant.rs` |
| `MAINTENANCE_PER_NODE` | **shoot organs pay the girth term too** — `maintenance_cost` |
| `MAINTENANCE_PER_CELL` | the pooled bill everything is calibrated against |
| `LEAF_CONSTRUCTION_MULTIPLE` | denominate the organ charge against it |
| `Grow.cost`, `max_active_tips` | per species; determinacy raises per-tip share |
| `REPRODUCTIVE_BUDGET_CAP`, `seed_cost` | organs are a second draw |
| `RESOURCE_SCALE` | charge-at-the-decision must be affordable from one cell |

**And the sharpest, which no report names:** `shoot_cells` counts every non-root
organism cell and feeds **both** `seed_maturity_met` **and** the juvenile check
**and** the per-cell bearer denominator. Adding `Flower`/`Fruit` cells therefore
*advances* the `seed_maturity` fence and *dilutes* the seed rate at once — a
two-sided reallocation of an economy PR #84 calibrated to bind at ~79%. Either
exempt organs from `shoot_cells` and say so in source, or budget the
re-derivation. (`seed_maturity`'s fence is itself still open — costs §15d.)

## 7. What is NOT established

- **Gate 2, generation throughput, is open and is called the biggest practical
  gate.** Measured depth is 1–2; `genome_drift` warns below 2. Phase 6 cannot be
  judged until this moves. Phase 5 is the annual/senescence work that addresses
  it. Encouraging sign from Phase 2: with `hazard=0.06` the established plants
  carrying an *inherited* genome went 1 → 3 of 16, because gaps are where
  recruits establish.
- **Gate 3, whether selection discriminates by morphology, is unmeasured.**
- **Several mutants out-produced the base (109 seeds against 80). That is one
  world seed and is a hypothesis, not a result** — identical genomes span
  31–153 cells here. Comparing arms needs an order statistic over seeds.

## 8. Verification

1. `cargo test --lib` — baseline **956 passed / 0 failed / 54 ignored**.
2. `cargo clippy --all-targets -- -D warnings`.
3. Shared-budget instrument is **`plant_probe`, not `seedsweep`** —
   `seedsweep.sh`'s own header scopes it to the load/bearing/fracture model and
   its `cells lost` column rides the water cycle at ±1,700. Use
   `plant_probe species=tree trees=24 width=512 worldseed=N` over 8 seeds, and
   read the Gini/top-share who-won lines.
4. `cargo run --release --example ascii` for worst-frame; `bash
   scripts/acceptance.sh`; `bash scripts/docscheck.sh` after every merge.
5. **A blind review card before calling it done** — an authored
   determinate-plus-organ species against a shrub, asking the owner's own
   question verbatim: *"are these different plants, or one plant in several
   sizes?"* Counts in `meta`: organs built, axes terminated, fruit dropped.
   The bed card (`20260828T181052911Z-85e5e2`) came back *"I think it could be
   realistic"*, so `scene=gradient` is cleared for use.
