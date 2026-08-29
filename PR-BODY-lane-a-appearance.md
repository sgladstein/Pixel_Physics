# Creature appearance: extent is the only lever, and the palette is already right

**What this does.** It answers the owner's E5 question — *"we definitely want
new creatures to not look like a recoloured ant"* — with a measurement and a
rendered choice, and it changes no shipped creature. `ant.ron`, species and
material, is byte-identical; `src/sim/creature.rs` is untouched.

**Where it sits.** `Reports/plant-appearance-design.md` found that a
silhouette is set by extent, composition and palette, and that every
architectural lever the plant line built moved only which cell gets a label.
This is the same question asked at the other end of the scale: a plant is
4,700 cells and an ant is **two**, so the creature line cannot lose the way
the plant line did — it has no composition to move at all. Of the two levers
that remain, one turns out to be already at its best.

## The finding

**A two-cell ant is not hard to see because it lacks contrast.** On a lit
skyline it reads body luma 39 against a surround of 143 — the highest contrast
of any arm measured. It is unfindable because **a pixel-art world's texture is
1–2 cell luminance noise**, so the ant competes with every rock edge and every
leaf in the frame.

The number that says so is `decoys`: how many *other* places in the picture are
at least as different from their surroundings as the animal is. Holding
contrast fixed and moving only the body's size, over one frame:

| body size | places in the frame that look as much like the animal as the animal does |
|---|---|
| 1 cell | 342 |
| **2 cells — ships today** | **127** |
| 4 cells — the shipped beetle | 55 |
| 9 cells | 15 |
| 16 cells | 0 |

Measured on three trees — `ba6fc98`, `f96c08d` (two worldgen changes
deepening the soil ~10x) and `a7b2dd9` (#103's colony fix and #106's export).
The first reads 337 / 126 / 57 / 33 / 13 / 0 and the last two are identical
to each other: the effect is a property of the world's *texture grain*, and
none of those changes touch it. **No number here was taken on
`scene=colony`** — this harness builds its own world from a `WorldgenPresets`
preset — so nothing in it was measured on the pre-#103 scene.

**Palette has nothing left in it, measured twice.** `render.rs`'s standing
suggestion — try a *brightness* axis rather than the hue axis that lost a blind
A/B — was tried here and loses too. Arm E of the card is arm C's nine-cell body
repainted pale, bit-identical run (`moves 535 / blocked 400` on both arms), and
it scores **ink per creature 251 against C's 1,288**. A nine-cell pale animal
puts less on screen than the shipped **two**-cell dark one (285).

**Shape at constant extent does nothing.** Two nine-cell bodies — a 3x3 block
and a waisted 5x2 — score within 0.8% of each other on every appearance
number, on all three trees. A mobility claim between them is **withdrawn** in
the report: the pair read 26/39%, 25/44% and then 43/41% blocked once #103
changed where a colony lands, so the ordering was placement luck. The coarse
gap survives — rigid 25–43% blocked, chain 2–6%. That is
`plant-appearance-design.md` §5 reproduced in another subsystem.

**A chain is the cheapest extent there is.** `Chain(6)` gets 2.9x the shipped
ant's ink for *less* blocked movement (4% against 5%) — a chain follows its
head, so it flows over anything. It reads as a worm, which is the cost.

## The finding that outlives the body plan

**Appearance is not in the genome, and cannot be.** This is lane A's and lane
B's findings turning out to be one constraint, stated once in the report's §7:

> **The two things that decide whether a creature is worth looking at —
> extent and palette — are exactly the two things an individual cannot own.**

`plant_creature_seed` resolves a body's material as
`materials.id_of(species_name)`, so a creature's whole appearance is one
hand-authored palette — lane B reached the same seam from the far side, where
an exported species with no material hatches nothing. And
`species_export::individual_as_species` copies the parent's `body` verbatim;
only `genome` and `traits` are an individual's own. So *"how do we do that by
evolution rather than direct design"* has a measured answer today, and it is
**you cannot**. The smallest change that opens any of it is
**shade-by-cell-type**, one line at that seam.

## The deliverable

Review card **`20260829T045336581Z-34c3d3`** (board `creatures`): five body
plans, one world, one seed, one colony, 12-frame sequences so the motion is
scrubbable. **The shipped body plan is the owner's decision and this branch
does not make it.**

## What is here

| | |
|---|---|
| `examples/creature_look.rs` | The instrument. **Worth landing on its own** — `ink` and `decoys` are not creature-specific and answer *"is this findable"* for anything drawn into this world, which is the question `plant-appearance-design.md` had no instrument for |
| `Reports/creature-appearance-design.md` | The report, with its `Reports/README.md` line and its `Reports/instruments.md` row |
| `Reports/lanes/lane-a-appearance.md` | The lane note |
| `assets/materials/chitin_{mid,pale}.ron` | The value axis. Probe materials; nothing in the world is made of them |
| `assets/species/ant_{long,block,wide}.ron`, `chitin_pale.ron` | The four candidate bodies. **Each is `ant.ron` with one line changed** — `body:`. They exist to be rendered |
| `src/sim/{material,organism}.rs` | One `include_str!` line each per asset, appended at the end of the lists per those lists' own standing rule |

**At most one candidate should survive the owner's choice, and none of them
should land as-is.** If the answer is "none of the above", the two files worth
keeping are the instrument and the report.

## What it does not do

No engine behaviour changed, and nothing here touches the contested
`src/sim/creature.rs`. The one code change the report argues for —
**shade-by-cell-type**, which is what buys countershading and is the only way
to hold contrast against both the sky a creature is silhouetted on and the
ground it tunnels in — is deliberately not built, because it is only worth
having once a body is large enough to have a top and a bottom, and that is the
decision on the card.

## Gates

`cargo test --lib`, `cargo clippy
--all-targets -- -D warnings` on both 1.94.1 and CI's 1.98.0,
`bash scripts/docscheck.sh` clean. `main` merged in at `a7b2dd9`; the two
`include_str!` conflicts resolved trunk-first, per the rule those two lists
state themselves. Exact counts are at the bottom of
`Reports/lanes/lane-a-appearance.md`.
