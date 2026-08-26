# The arch outspans the lintel, and the margin is 1.6x at equal material

**Status: measured, 2026-08-26. Nothing built, nothing changed.** The
instrument is `examples/arch_probe.rs`, new and read-only — it builds scenes,
runs them, and censuses. No default moves.

The claim came out of `structural-support-model.md`'s design follow-up and was
explicitly flagged there as a prediction rather than a measurement:
`load::capacity` scales with section depth *squared* measured perpendicular to
wherever support arrives, and `stone.ron` prices a step from *below* at 1
against 3 from *above*. That predicts a curved roof spans further than a flat
one **with nothing added to the engine** — which would mean a player can
discover real structural engineering inside a falling-sand game.

**It holds, and by more than expected.** At the same material budget an arch
spans **63% further**; at the widest span it can hold, a flat roof needs
**3.1x the stone** to do the same job.

## The result

Four arms over identical piers and an identical opening, swept until every arm
had both a span it held and a span it dropped. A roof "holds" if it kept ≥90%
of its cells above the springing line after settling.

| arm | thickness | margin (span / clear) | cells at margin |
|---|---|---|---|
| `lintel` | 3 | 56 / **55** | 219 |
| `lintel=` — thickened to the arch's cell count | 4 | 64 / **63** | 332 |
| `lintel3T` — the depth control | 9 | 96 / **95** | 1,125 |
| **`arch`** | **3** | **104 / 103** | **493** |

- **Against the same thickness: 1.86x** the span (104 against 56).
- **Against the same material: 1.63x** the span (104 against 64). This is the
  one that matters — see the confound below.
- **Against three times the depth: 1.08x** the span, using **44%** of the
  stone (493 against 1,125).

## What it costs to roof the same hole flat

Read the other way round — hold the span fixed and ask what a flat roof
costs. Thickness bisected until the lintel holds:

| clear span | arch (T=3) | thinnest lintel that holds | flat costs |
|---|---|---|---|
| 79 | **389 cells** | T=6, 618 cells (T=5 falls at 24.6%) | **1.59x** |
| 103 | **493 cells** | T=11, 1,507 cells (T=10 falls at 24.4%) | **3.06x** |

**The gap widens with span** — 1.6x at 79 cells of opening, 3.1x at 103. That
is the same shape masonry has, arrived at for a different reason (below), and
it is the property that makes it a *discovery* rather than a fact: the wider
the player builds, the more curving pays.

## The controls, because the headline is the kind that flatters itself

**The confound this was built to remove.** A semicircular ring is longer than
the chord it spans, so an arch of equal thickness is simply more stone — the
arch runs 1.26–1.43x the plain lintel's cell count across the sweep. "More
material holds better" is not a discovery. `lintel=` thickens the flat slab
until it matches the arch's own cell count, and at the decisive spans it
matches or **exceeds** it: at span 80, `lintel=` places **396 cells against
the arch's 389** and falls to 28.0% while the arch holds 100%. With material
held equal, and in fact tilted against the arch, the geometry alone decides
it.

**The alternative explanation, tested rather than argued.** `capacity` goes as
depth squared, so "the arch is really just depth where it counts" is the
obvious rival reading. `lintel3T` is that reading taken seriously — three
times the thickness, a 9x capacity term against a 3x load term. **It works**:
the margin goes 56 → 96. So section depth is a real lever and a player who
thickens a flat roof is not wrong. The arch still beats it, on 44% of the
material.

**The null guard fired on the first attempt, which is why the number is
trusted.** `anchor_probe`'s recorded lesson is that its own first run put the
subject where the margin could not reach it and produced a null that said
nothing. The first sweeps here (spans 8–48) had every arm at 100% and the probe
refused to report a margin, printing *"the sweep never reached its margin"*
instead. The reported margins are only printed once every arm has both a
standing span and a fallen one.

**Settling, checked rather than assumed.** `CLAUDE.md`: a cascade censused
before it settles reads a delay as damage. Spans 80 and 120, all four arms, at
frame budgets 800 / 2,400 / 4,800: **byte-identical at all three**. The scenes
are at rest well inside the budget.

**The scene is a constant.** Both forms bear on the same piers, at the same
springing row, over an opening the probe *measures* (the `clear` column) rather
than assumes — the widest empty run on the row below the springing line. At
span 104 both roof a 103-cell hole.

## Why it wins here, which is not why it wins in stone

A real arch works by putting its ring into pure compression and throwing the
thrust sideways into its abutments. **This engine has no lateral thrust** —
support is a DAG over four neighbours and nothing pushes outward. The arch
wins for a different reason:

- Every voussoir is held from **below-ish** by the next one down. That is the
  cheap direction, and the lever arm is one cell.
- A lintel's midspan is held only from the **side**, across half the opening,
  so it carries a bending moment with a lever arm of tens of cells.
- And the two are measured by `capacity` in different regimes: the arch's
  section is the ring depth, perpendicular to a support arriving from below;
  the lintel's is a horizontal run that `section_cells` walks and
  **`MAX_SECTION` caps at 40**. At a 103-cell clear span the lintel's capacity
  is computed over a 40-cell window of a 103-cell member.

**That last point is a caveat, not a defect, and it should be stated when the
result is quoted.** The margin is what a player experiences and it is measured
honestly; the *explanation* is partly a statement about a work cap. If someone
wants the explanation to stand alone, the measurement that isolates it is
`MAX_SECTION` swept against the lintel's margin — if the lintel's margin moves
with it, the cap is part of the answer.

This is not a claim that the engine is a masonry simulator. It is a claim that
**the shape a player builds changes what stands, in the direction real
buildings go, without a line of special-case code** — which is the interesting
thing either way.

## What to do with it

1. **Show it before building anything on it.** Two contact sheets — a 103-span
   arch standing and a 103-span lintel in pieces — with the failure counts in
   the card's `meta`, through `scripts/review.py`. This report is numbers; the
   owner judges by eye, and *"the roof you curved is still there"* is a
   judge-by-eye claim.
2. **It is a tutorial, not a feature.** Nothing needs adding to the engine.
   What is missing is any reason for a player to try the second shape — see
   `design-load-telegraph.md` (they cannot see what is straining) and
   `design-props-and-shoring.md` (they have no cheap way to experiment).
3. **A `Tool::Arch` is optional and probably second.** Curving a roof with the
   existing `Line` tool is fiddly but possible; the discovery matters more than
   the convenience, and a tool that draws arches for you removes the discovery.

## Reproducing

```
cargo run --release --example arch_probe
cargo run --release --example arch_probe -- spans=48,56,64,72,80,88,96,104,112,120 frames=1200
cargo run --release --example arch_probe -- spans=104 thickness=11    # the cost bisect
```

The default sweep is 48–120, which brackets all four margins. The *first*
sweep was 8–48 — inside every arm's margin — and it printed the null rather
than a number, which is the behaviour the guard exists for.
