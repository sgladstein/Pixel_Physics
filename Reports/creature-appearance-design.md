# Why you cannot find an ant, and what actually makes a creature visible

Written for the owner's E5 open question — *"We definitely want new creatures
to not look like a recoloured ant. Not sure how we get good looking creatures
with our pixel resolution or how we do that by evolution rather than direct
design."*

`Reports/plant-appearance-design.md` is the input this was built on and its
finding transfers, but **not in the direction it points for plants**. That
report found a silhouette is set by **extent, composition and palette**, and
that every architectural lever the plant line built moved only *which cell
gets a label*. A creature sits at the opposite end of the same axis: an ant is
a two-cell chain, so it has no composition to move at all, and of the two
levers that remain, one of them is measurably already at its best.

**The measurement is `examples/creature_look.rs`.** Review card
`20260829T045336581Z-34c3d3` is the five body plans it was built to compare.
The card's images and its `meta` counts were rendered on `ba6fc98`; every
number in this report is the re-measurement on `f96c08d` after merging `main`
in (§2 says how far they moved, which is not far).

## 1. The finding, stated first

**Extent is the only lever, and it has to roughly quadruple.**

- **Palette has nothing left in it.** The shipped dark ant already achieves
  the best contrast of the three values tested, in the world it actually
  walks in. A pale body at **9 cells puts less luminance on screen than the
  dark 2-cell ant does** — 249 against 282, at 4.5x the cells (§3).
- **Shape at constant extent is a small effect and it is bought expensively.**
  Two 9-cell bodies, one a compact 3x3 block and one a waisted 5x2, score
  within 0.1% of each other on every appearance number and differ by **13
  percentage points of blocked movement** (§4).
- **Size moves everything.** Holding contrast fixed and moving only the body's
  size, the number of places in one frame that look as much like the animal as
  the animal does falls **126 → 13 → 0** across 2, 9 and 16 cells (§2).

## 2. The number that explains the picture: decoys

An ant standing on a lit skyline has enormous contrast. Measured on the
`rolling` preset at noon, a two-cell ant against sky reads **body luma 39
against a surround of 143** — a Weber contrast of 0.72, higher than any other
arm in the grid. And you still cannot find it. The contrast number is real,
arithmetically correct, and answers the wrong question — `CLAUDE.md`'s single
worst-recurring failure, in its "metric" costume.

What it misses is that **a pixel-art world's texture is 1–2 cell luminance
noise**. Every rock edge, every leaf, every grain of a speckled soil is a
small high-contrast feature. So the question is not *how different is the
animal from its background* but **how many other things in this picture are
equally different** — how many candidates the eye must reject before it
reaches the animal.

`decoys()` measures exactly that: it slides the body's own bounding box over
the frame *the body is not in*, scores every position with the same
body-against-surround statistic, and counts the positions that beat a
threshold. Holding the threshold fixed and moving only the window size, over
one 512x320 frame of the `rolling` preset at noon:

| body size | ≥40 | ≥60 | **≥80** | ≥100 |
|---|---|---|---|---|
| 1 cell | 3,184 | 1,165 | 342 | 97 |
| **2 cells — ships today** | 2,075 | 566 | **127** | 56 |
| 4 cells — the shipped beetle | 1,358 | 252 | **55** | 26 |
| 6 cells (3x2) | 1,119 | 182 | **32** | 18 |
| 9 cells (3x3) | 858 | 98 | **15** | 0 |
| 16 cells (4x4) | 618 | 48 | **0** | 0 |

**These figures are from `main` at `f96c08d`, and they are the second
measurement, not the first.** Everything here was originally measured on
`ba6fc98`, a few hours older — and `main` then landed two worldgen changes
that deepen the soil blanket about tenfold between them, which is exactly the
kind of drift `CLAUDE.md` warns makes a baseline "a measurement of a tree
nobody else has". Re-run after the merge, the ≥80 column reads **342 / 127 /
55 / 32 / 15 / 0** against the pre-merge **337 / 126 / 57 / 33 / 13 / 0**, and
every live figure in §5 moves by under 3%. That is a robustness result worth
having rather than a formality: the effect is a property of the world's
*texture grain*, which a soil-depth change does not touch.

Read the ≥80 column: **80 is about the contrast a dark body actually achieves
against ordinary ground here**, so that column is the live one. At the shipped
two cells the picture contains 126 things that look as much like an ant as an
ant does. At nine it contains 13. At sixteen it contains none.

**The positive control is the threshold-0 column**, which is not printed above
because it is uninteresting and is the whole point: at a threshold of zero
every window must count, and it does — 158,065 down to 155,620 as the window
grows and the margin shrinks. A row that was not the full window population
would mean the counter never fired. `CLAUDE.md`'s rule about running the case
whose answer you know is non-zero, applied to a counter written the same hour.

**What this number is not.** It is a count of *distractors*, not a model of
human search. It says the eye has 126 candidates rather than 13; it does not
say a person takes ten times as long. The claim it supports is ordinal and the
review card is what settles the rest.

## 3. Palette: there is nothing left to win, and the arm that proves it

The prior evidence was one blind A/B — tinting ants by their diet gene, in
which **untinted won** — and `render.rs`'s own note draws the right lesson
from it: *"the readable signal at that size is contrast against the ground
rather than hue"*, and *"the thing to try is a brightness axis rather than a
hue axis (brightness being the one channel that reads at 1–2 px)"*.

**That recommendation was tried here and it loses too.** Three values, hue
held to the shipped ant's R:G:B ratio so that only value moves:

| material | body luma | contrast vs surround | decoys at that contrast |
|---|---|---|---|
| `ant` (shipped) | 33 | **103** | **44** |
| `chitin_mid` (120,104,92) | 114 | 32 | 3,465 |
| `chitin_pale` (214,202,188) | 212 | 48 | 1,180 |

The mid value is catastrophic because it *is* the colour of the stone. The
pale value is worse than the dark one because this world is bright: sky reads
176 and lit rock reads 120–160, so a pale body has less headroom above the
ground than a dark body has below it.

**The clean version of that result is arm E of the review card.** Same seed,
same 9-cell body, same 600 frames — the run is bit-identical, `moves 598 /
blocked 197` on both arms, so nothing but the paint changed:

| | `ant_block` (dark) | `chitin_pale` (pale) |
|---|---|---|
| cells per creature | 9.0 | 9.0 |
| **ink per creature** | **1,285** | **249** |

A nine-cell pale animal puts **less on screen than the shipped two-cell dark
one** (282). Rendered, they read as lichen on the rock.

**So the shipped ant's palette is not a thing to fix.** `ant.ron`'s own
comment — *"Dark, and deliberately not red… the readable signal at that size
is contrast against the ground"* — is correct and now has a second,
independent measurement behind it.

## 4. Shape at constant extent: the plant report's prediction holds

Arms C and D of the card are both **nine cells** and differ only in
arrangement: C is a filled 3x3, D is a 5x2 with a one-cell waist notch, which
is an insect outline rather than a brick.

| | C, 3x3 block | D, 5x2 waisted |
|---|---|---|
| cells per creature | 9.0 | 9.0 |
| ink per creature | 1,285 | 1,288 |
| \|contrast\| | 119.0 | 116.7 |
| **moves blocked** | **25%** | **44%** |

**Every appearance number is the same to within noise (0.2% on ink) and the
mobility cost is three quarters again as large.** This is `plant-appearance-design.md` §5 reproduced in a
different subsystem: rearranging which cell is where, at constant extent, does
not move a silhouette — and here it is not even free, because a wider footprint
has fewer legal positions on rough ground.

The one thing D does buy is legible at magnification and is worth the owner's
eye rather than this report's assertion, which is why both arms are on the
card.

## 5. What extent costs

Measured on one seed, 40 attempted placements, 600 frames, `rolling` preset:

| arm | body | cells | placed | ink/creature | **moves blocked** |
|---|---|---|---|---|---|
| A | `Chain(2)` — ships today | 2 | 24/40 | 282 | **4%** |
| B | `Chain(6)` | 5.7 | 18/40 | 823 | **2%** |
| C | `Rigid` 3x3 | 9 | 19/40 | 1,285 | **25%** |
| D | `Rigid` 5x2 waisted | 9 | 18/40 | 1,288 | **44%** |
| E | `Rigid` 3x3, pale | 9 | 19/40 | 249 | 25% |

Three costs, in the order they matter:

- **Mobility, and it is the expensive one.** `BodyPlan`'s own doc predicted
  it — *"a wide body handles rough ground badly — often no legal position at
  all"* — and the number is 6–11x the shipped ant's blocked rate. **A chain
  pays none of it**: `Chain(6)` is blocked *less* than `Chain(2)` (2% against
  4%), because a chain follows its head and flows over anything.
- **Cells per creature**, which is 4.5x at nine cells. That is the frame cost
  (creature cells are relocated explicitly from the active-site schedule, not
  swept), and it is also 4.5x the `body_energy` the ledger stamps at every
  hatch — a real change to the colony's economy, not only its picture.
- **Placement**, which is nearly free: 23 → 18 of 40 across the whole ladder,
  and the shipped ant already refuses 16.

**`Chain(6)` is therefore the cheapest thing on this table by a wide margin**:
2.9x the ink for *less* blocked movement than the body that ships today, and
no engine change at all. What it costs is elsewhere — rendered, six cells in a
following chain read as a **worm**, not an ant, because the chain's shape is
wherever the head has been.

## 6. The recommendation

**Ship a body of about nine cells and keep the dark palette.** The choice
between the arms is the owner's and is on the card; the measurement says:

1. **Two cells cannot be made to work by any means available today.** Not by
   hue (already lost a blind A/B), not by value (§3), not by arrangement
   (there is none at two cells). It is below the world's own texture grain.
2. **Nine cells clears the texture** — 15 decoys against 127 — and four cells,
   the shipped beetle, only halves it (55). The step that pays is the large
   one.
3. **The palette should not move.** It is already the best of the values
   tested, and the pale arm is a measured regression.

**What this does not settle, and should not be read as settling.** These are
five *body plans*, not five species. Nothing here says an ant should be nine
cells; it says an animal the player is supposed to see must be. A colony of
nine-cell ants is a different economy, and §5's ledger note is where that
starts.

## 7. Composition is structurally unavailable to a creature, and that is the
next thing worth building

`plant-appearance-design.md`'s three silhouette-setters are extent,
composition and palette. **Creatures have two of them.**

`creature::plant_creature_seed` writes every body cell as
`Cell::new(material_id, shade)` where `material_id` is the species' single
material and `shade` is `rng::stream(..).below(palette.len())` — an
independent uniform draw per cell. So a creature is **one material with random
per-cell shade**, and there is no way for a species to say *this cell is dark
and that one is pale*. The `CellType` is already computed on the very next
line (`if i == 0 { Head } else { Segment }`) and is thrown away as far as
appearance is concerned.

Two consequences, and the second is the interesting one:

- **The palette must be narrow, or the body dissolves.** This is
  `dead-ends.md`'s `log.ron` entry — a wide spread makes a field of cells draw
  randomly across a range, which is *speckle*, and speckle destroys the one
  thing a small body has, its shape. The shipped ant's three shades span 24
  luma units and that is about right. **Do not widen a creature palette to add
  variety**; it subtracts the silhouette.
- **The one code change worth making is shade-by-cell-type.** A body that
  could put its pale cells on top and its dark cells underneath is
  countershaded, which is what guarantees contrast against *both* the sky it
  is silhouetted against on a ridge and the ground it is on in a tunnel —
  the two backgrounds §3 shows no single flat value can serve at once. It is
  one line at the seam quoted above, it costs no per-cell state (the shade
  byte is already there), and it becomes worth having **exactly when the body
  is large enough to have a top and a bottom**, which is what §6 recommends.

That is deliberately not built here: the shipped body plan is an owner
decision (E5) and `src/sim/creature.rs` is the creature line's most contested
file. It is the thing to do *after* the card comes back.

## 8. What is on this branch, and what should land

Nothing here changes a shipped creature. `ant.ron` — species and material —
is untouched, and no engine behaviour moved.

| | |
|---|---|
| `examples/creature_look.rs` | The instrument. Worth landing on its own — `ink` and `decoys` are not creature-specific and answer *"is this thing findable"* for anything drawn into this world |
| `assets/materials/chitin_{mid,pale}.ron` | The value axis. Probe materials, nothing is made of them |
| `assets/species/ant_{long,block,wide}.ron`, `chitin_pale.ron` | The four candidate bodies. **Each is `ant.ron` with one line changed.** They exist to be rendered, and at most one of them should survive the owner's choice |

## 9. Two things the instrument got wrong first, both worth carrying

**A scene that is not the game will flatter every arm.** The first version
built its world with `app::build_terrain`, which is `Spec::Legacy` — thin bare
platforms over open sky. Every probe in it stood on a clean skyline against a
flat gradient, so `surround luma` came back **176.5 to one decimal place for
every single probe**. That tidiness is `CLAUDE.md`'s own tell: a clean first
result is evidence of an artifact rather than of a strong effect. The number
was arithmetically correct and it was the luminance of the sky.

**And the day/night cycle aliases straight into a luminance measurement.** The
first run on a generated world landed at night and reported a surround luma of
**28** where the same world at noon gives **153**. Every contrast figure in
this report would have been a statement about the hour. `render` pins
`sky::frame_for_daylight(1.0)` for exactly this reason — the oscillator rule,
applied to a measurement rather than to a threshold.
