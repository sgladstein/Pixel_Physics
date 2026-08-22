# Pass interference: the defect class this generator keeps shipping

2026-08-20. A measurement, an instrument, and the argument for why this is
the highest-leverage thing in worldgen right now — ahead of any new
mechanism.

## The pattern

Worldgen is a pipeline of twelve passes that each write cells into a shared
world, later ones overwriting earlier ones. Each pass reports how many cells
it wrote, and that counter exists for a good reason (`mod.rs`: *a picture
cannot show whether the feature that produced it is the one you built*).

**But the counter cannot see the failure this generator actually has.** A
pass that wrote nothing *because an earlier pass took its cells* reports the
same number as a pass whose noise draw came up empty: zero, or a small
number, with no indication which. So the counters stayed green, the tests
stayed green, and features were absent from the screen.

Five of these have been found. Every one by accident, one per round, each
costing most of a session to diagnose:

| eater | eaten | found in |
|---|---|---|
| `pockets` | `vaults` — one sand grain rejected an entire cave system | round-5 review |
| `brows` | `boulders` — a lip already occupied the dome's air | round-4 finding R4-1 |
| `soil_blanket` | `talus` — erosion's apron folded into cover first | round-4 finding R4-2 |
| `brows` | `ponds` — a lip roofed water that filled from both sides | round-4 finding R4-3 / open bug 0 |
| erosion | formation-scale relief — flattened, never rebuilt | `worldgen-erosion-design.md` |

Finding these one at a time is the process defect. The instrument below
finds them all in one command.

## The instrument

`examples/pass_ablation.rs`, on `worldgen::generate_ablated`. Build the
world once per pass with that pass switched off; difference the whole
report vector against the full build. Read it as a matrix: **row = the pass
switched off, column = what that did to another pass's output.** Positive
means the switched-off pass was *suppressing* that column; negative means it
was *feeding* it. Median over seeds, never a single seed.

The harness names passes through `worldgen::pass_names()` rather than
carrying its own copy of the table — a duplicated pass list goes stale
silently the moment a pass is added or reordered, and an ablation run
against a stale list reports interference between the wrong pair.

**It validated itself on the first run**, which is the check CLAUDE.md asks
for before trusting a new metric: without being told about them, it
reproduced `pockets → vaults` and `brows → boulders` at the right
magnitudes, and it reports the degenerate `without stone_massif` row
(`pockets -100%`, `vaults -100%`) that any correct version must.

## What it found — 6 seeds, all presets, 2048x640

    without pockets      : vaults +112% (canyon)  +87% (rolling)  +86% (terraced)
                           vaults APPEARS, was zero (arid)
    without brows        : boulders APPEARS, was zero (arid, canyon, rolling, terraced)
                           talus +9% (canyon)
    without ponds        : life_scatter +49% (wetland)  +37% (rolling)
                           soil_moisture -79% (rolling)  -81% (terraced)
    without talus        : life_scatter +10% (canyon)   ponds +6% (terraced)
    without soil_blanket : life_scatter -44% to -62%    soil_moisture -100%

**Two new findings, both first-order:**

1. **`ponds` eats up to half of `life_scatter`.** Removing standing water
   raises scattered life by **49% in `wetland`** and 37% in `rolling`.
   `wetland` is the preset whose whole identity is lushness and which the
   world review called "a mud flat with a pond" and the dullest world in
   the set. Half its missing vegetation is its own water covering the
   ground the seeds would have gone into. Nobody had connected those two
   facts, because each pass's counter looked fine.

2. **`pockets` deletes *every* cave in `arid`.** Elsewhere the loss is 46-53%
   of cave cells; in `arid` the baseline is zero and switching pockets off
   makes caves appear. That is the strongest form of the round-5 task-1
   finding and it explains arid's 13-of-16 empty worlds exactly.

**Confirmations, now quantified rather than anecdotal:** `brows` deletes
100% of boulders in four of six presets — R4-1 said "about one boulder in
30 seeds seats" and this says why, in one line, for every preset at once.

**Relationships that are correct and worth having on record anyway**, so a
future change that breaks one is visible: `soil_blanket` feeds
`life_scatter` and `soil_moisture`; `ponds` feeds `soil_moisture`;
`stone_massif` feeds everything that carves rock.

## Limits of the instrument — read before trusting a row

- **It measures cell counts, so it sees suppression and not reshaping.**
  Erosion flattening formation-scale relief does not appear here at all,
  both because erosion runs in the plan phase (no row in `PASSES`) and
  because it changes the *shape* of what other passes write, not how many
  cells they write. Shape questions need the shape probes
  (`cave_probe`, `viewshot`'s prominence-at-four-reaches).
- **`without stone_massif` is a degenerate world**, not a finding. It is
  the sanity anchor: any version of this instrument that does not report
  `-100%` there is broken.
- **A percentage on a pass writing 92 cells is not the finding a
  percentage on a pass writing a million is**, which is why the baseline
  row prints beside the matrix.
- One row is unexplained and probably harmless: `without stone_massif`
  raises `brows` by 7-37%, i.e. `brows` writes *more* into a world with no
  massif. Only reachable in the degenerate ablation, but it hints that
  `brows`' write test does not require stone beneath the lip — which is
  the same shape as R4-3's unanchored-lip bug. Worth one look during the
  round-6 `brows` work; not worth a session on its own.

## Why this comes before new mechanisms

Every cave, boulder and formation defect found in this planning block was
either a constant that had never been checked against its outcome, or a
pass quietly eating another's cells. Neither is an architectural failure —
the pipeline's shape (pure plan, declared margins, collect-verify-write,
at-rest by construction) is what made all of this measurable in the first
place, and none of it argues for rebuilding the generator.

What was missing is a **feedback loop**: nothing read the world back and
asked whether the mechanisms produced what they claimed. Three instruments
now do (`cave_probe`, `pass_ablation`, `viewshot`'s prominence and boulder
finders), and the next step is to make them gates rather than things a
session remembers to run.
