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
number in this report is the third measurement, on `a7b2dd9` — after the
colony fix (#103) and the species export (#106) both landed. §2 and §5 say
how far the numbers moved between trees, which for the appearance figures is
barely at all and for one mobility figure is a long way.

**No number here was taken on `scene=colony`.** This harness builds its own
world from a `WorldgenPresets` preset, so nothing in it was ever measured on
the pre-#103 scene that stood ants on lakes. What it *did* share with that
scene is the bug: §9.

## 1. The finding, stated first

**Extent is the only lever, and it has to roughly quadruple.**

- **Palette has nothing left in it.** The shipped dark ant already achieves
  the best contrast of the three values tested, in the world it actually
  walks in. A pale body at **9 cells puts less luminance on screen than the
  dark 2-cell ant does** — 251 against 285, at 4.5x the cells (§3).
- **Shape at constant extent moves nothing measurable.** Two 9-cell bodies,
  one a compact 3x3 block and one a waisted 5x2, score within 0.8% of each
  other on every appearance number, over three runs on three trees (§4).
- **Size moves everything.** Holding contrast fixed and moving only the body's
  size, the number of places in one frame that look as much like the animal as
  the animal does falls **127 → 15 → 0** across 2, 9 and 16 cells (§2).
- **And neither lever is reachable by evolution** — §7, which is the half of
  the E5 question this report can answer with a flat no. Palette lives in a
  hand-authored material file keyed by species name; body plan is copied from
  the parent by `individual_as_species`. Only `genome` and `traits` are an
  individual's own.

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

**These figures are from `main` at `a7b2dd9`, and they are the third
measurement, not the first.** Everything here was originally measured on
`ba6fc98`, a few hours older — and `main` then landed two worldgen changes
that deepen the soil blanket about tenfold between them, which is exactly the
kind of drift `CLAUDE.md` warns makes a baseline "a measurement of a tree
nobody else has". Re-run, the ≥80 column reads **342 / 127 / 55 / 32 / 15 / 0**
against the pre-merge **337 / 126 / 57 / 33 / 13 / 0** — and reads **the same
342 / 127 / 55 / 32 / 15 / 0 again** after #103 and #106, which change where
a body may stand but not what the ground looks like. That is a robustness
result worth having rather than a formality: the effect is a property of the
world's *texture grain*, and neither a soil-depth change nor a placement-rule
change touches it.

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
same 9-cell body, same 600 frames — the run is bit-identical, `moves 535 /
blocked 400` on both arms, so nothing but the paint changed:

| | `ant_block` (dark) | `chitin_pale` (pale) |
|---|---|---|
| cells per creature | 9.0 | 9.0 |
| **ink per creature** | **1,288** | **251** |

A nine-cell pale animal puts **less on screen than the shipped two-cell dark
one** (285). Rendered, they read as lichen on the rock.

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
| ink per creature | 1,288 | 1,278 |
| \|contrast\| | 119.0 | 116.7 |

**0.8% apart on ink, and inside the noise on contrast** — and that holds on
all three trees this was measured on. This is
`plant-appearance-design.md` §5 reproduced in a different subsystem:
rearranging which cell is where, at constant extent, does not move a
silhouette.

**A mobility claim was made here and is withdrawn.** Two earlier runs put C
and D at 26%/39% and 25%/44% blocked movement, and this section said the
waisted body pays "three quarters again" for its outline. On the third tree —
after #103 replaced the placement predicate — the same pair reads **43% / 41%**,
with C nearly doubling and the ordering gone:

| tree | C blocked | D blocked |
|---|---|---|
| `ba6fc98` | 26% | 39% |
| `f96c08d` | 25% | 44% |
| `a7b2dd9` (better predicate, 21–22 of 40 placed) | **43%** | **41%** |

The blocked rate is a function of **where the colony happened to land**, which
is exactly what the predicate change moved, and one run of it is a sample from
a wide distribution — `CLAUDE.md`'s own warning, arriving as a correction
rather than as a precaution. What survives all three runs is the coarse fact
in §5: **a rigid body is blocked 25–43% of the time and a chain 2–6%**, an
order-of-magnitude gap that no reshuffling of the runs closes. What does not
survive is any ranking *within* the rigid pair.

## 5. What extent costs

Measured on `a7b2dd9`, one seed, 40 attempted placements, 600 frames,
`rolling` preset:

| arm | body | cells | placed | ink/creature | **moves blocked** |
|---|---|---|---|---|---|
| A | `Chain(2)` — ships today | 2 | 31/40 | 285 | **5%** |
| B | `Chain(6)` | 5.8 | 22/40 | 836 | **4%** |
| C | `Rigid` 3x3 | 9 | 21/40 | 1,288 | **43%** |
| D | `Rigid` 5x2 waisted | 9 | 22/40 | 1,278 | **41%** |
| E | `Rigid` 3x3, pale | 9 | 21/40 | 251 | 43% |

Three costs, in the order they matter:

- **Mobility, and it is the expensive one.** `BodyPlan`'s own doc predicted
  it — *"a wide body handles rough ground badly — often no legal position at
  all"* — and a rigid body is blocked **8–10x** as often as the shipped ant.
  Read that gap as an order of magnitude rather than as a number: §4 shows the
  per-arm figure moving 25 → 43 on one arm between trees. **A chain pays none
  of it**: `Chain(6)` is blocked *less* than `Chain(2)` (4% against 5%),
  because a chain follows its head and flows over anything.
- **Cells per creature**, which is 4.5x at nine cells. That is the frame cost
  (creature cells are relocated explicitly from the active-site schedule, not
  swept), and it is also 4.5x the `body_energy` the ledger stamps at every
  hatch — a real change to the colony's economy, not only its picture.
- **Placement**, which is real but modest and is the one figure #103 improved
  outright: the shipped two-cell ant now places 31 of 40 where it placed 23,
  and the nine-cell bodies 21–22 where they placed 18–20. **A bigger body
  costs about a third of the sites a two-cell one gets**, on this seed.

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

## 7. Appearance is not in the genome, and cannot be

**This is the single sentence the creature line most needs, and it is two
lanes' findings turning out to be one constraint.** Stated once, sharply:

> **The two things that decide whether a creature is worth looking at —
> extent and palette — are exactly the two things an individual cannot own.**
> Evolution moves the brain and the gut. It cannot move either half of the
> silhouette. So *"evolve creatures and add the good ones to the game"* today
> means a person hand-authors the appearance of every one, and an evolved
> creature is a recoloured ant by construction — in fact not even recoloured,
> because the colour is inherited too.

The two ends of it, each checkable in one line of source:

- **Palette.** `creature::plant_creature_seed` resolves a body's material as
  `world.materials.id_of(species_name)` — one material per species, keyed by
  name — and writes every body cell as `Cell::new(material_id, shade)` with
  `shade` an independent `rng::stream(..).below(palette.len())` draw. So a
  creature's whole appearance is *one palette*, and a species that has no
  hand-authored `assets/materials/<name>.ron` **hatches nothing, silently**.
  Lane B reached the identical seam from the far side: `species_export`
  writes a species file and deliberately does not write a material, because
  "a material is a palette" and E8 asks a person to choose it
  (`Reports/creature-export-design.md`, `Reports/lanes/lane-b-serialisation.md`).
- **Extent.** `species_export::individual_as_species` copies the parent's
  `body` verbatim; its own header says so — *"a trait vector; everything else
  about it — body plan, metabolism — [is the parent's]"*. `genome` and
  `traits` are the only things an individual owns. So the one lever this
  report measures as working is not reachable by selection either.

**What that costs, and what it does not.** It does not make the E8 workflow
useless — a dev tool where a person picks a palette is a perfectly good dev
tool, and it is what the owner asked for. What it costs is the *other* half
of the E5 question: *"how do we do that by evolution rather than direct
design"* has a measured answer today, and the answer is **you cannot**.

**Two things follow for whoever next opens `creature.rs`.**

- **Never widen a creature palette to add variety.** This is
  `dead-ends.md`'s `log.ron` entry — a wide spread makes a field of cells draw
  randomly across a range, which is *speckle*, and speckle destroys the one
  thing a small body has, its shape. The shipped ant's three shades span 24
  luma units and that is about right.
- **Shade-by-cell-type is the smallest change that opens any of this**, and it
  is one line at the seam quoted above: the `CellType` is already computed on
  the very next line (`if i == 0 { Head } else { Segment }`) and thrown away as
  far as appearance goes. A body that could put pale cells on top and dark
  cells underneath is countershaded, which is the only thing that holds
  contrast against *both* the sky it is silhouetted on and the ground it
  tunnels in — the two backgrounds §3 shows no single flat value can serve at
  once. It costs no per-cell state: the shade byte is already there.

  It becomes worth having **exactly when the body is large enough to have a
  top and a bottom**, which is what §6 recommends — and it is also the first
  step toward an appearance that a genome could reach, because a cell-type
  assignment is the kind of thing a genome can carry and a material name is
  not.

That is deliberately not built here: the shipped body plan is an owner
decision (E5) and `src/sim/creature.rs` is the creature line's most contested
file. It is the thing to do *after* the card comes back.

## 8. What is on this branch, and what should land

Nothing here changes a shipped creature. `ant.ron` — species and material —
is untouched, and no engine behaviour moved.

| | |
|---|---|
| `examples/creature_look.rs` | The instrument. Worth landing on its own — `ink` and `decoys` are not creature-specific and answer *"is this thing findable"* for anything drawn into this world. Stands on `creature::colony_ant_site` rather than a private predicate |
| `assets/materials/chitin_{mid,pale}.ron` | The value axis. Probe materials, nothing is made of them |
| `assets/species/ant_{long,block,wide}.ron`, `chitin_pale.ron` | The four candidate bodies. **Each is `ant.ron` with one line changed.** They exist to be rendered, and at most one of them should survive the owner's choice |

## 9. Three things the instrument got wrong first, all worth carrying

**A scene that is not the game will flatter every arm.** The first version
built its world with `app::build_terrain`, which is `Spec::Legacy` — thin bare
platforms over open sky. Every probe in it stood on a clean skyline against a
flat gradient, so `surround luma` came back **176.5 to one decimal place for
every single probe**. That tidiness is `CLAUDE.md`'s own tell: a clean first
result is evidence of an artifact rather than of a strong effect. The number
was arithmetically correct and it was the luminance of the sky.

**And this harness hit `open-bugs-handoff.md` R's root cause independently,
before #103 named it.** Its own "where can a body stand" predicate was
written as *topmost cell that is not air* — which on a vegetated world finds
the **canopy**, because the topmost non-air cell under a tree is a `Plant`.
It placed **9 of 24 probes** and the missing ones were the two smallest
shapes, which are the arms the whole question is about. Two independent
harnesses reaching for the same wrong predicate is the argument for #103's
other half: `creature::colony_surface` and `colony_ant_site` are now the
single definition, and this harness calls them rather than keeping a third
copy.

**And the day/night cycle aliases straight into a luminance measurement.** The
first run on a generated world landed at night and reported a surround luma of
**28** where the same world at noon gives **153**. Every contrast figure in
this report would have been a statement about the hour. `render` pins
`sky::frame_for_daylight(1.0)` for exactly this reason — the oscillator rule,
applied to a measurement rather than to a threshold.
