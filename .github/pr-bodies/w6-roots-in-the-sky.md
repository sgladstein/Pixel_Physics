**The owner's pale-cream plant is root wood standing eighty cells up in the
canopy, and this branch reproduces it, names the cause, and shows that the
rule shipped for it three days ago is not what fixes it.** No behaviour the
player sees changes here — this is the diagnosis and the two guards that stop
it being re-derived.

Bug register: [`Reports/open-bugs-handoff.md`](../Reports/open-bugs-handoff.md) §W6, still **OPEN**.

## 1. What the owner actually meant

They were asked which feature in their review sheet was *"the roots growing
into the tree/sky"* and answered **"the left image"** — a whole plant rendered
pale cream instead of brown. Pale cream is `rootwood` (`168,146,112` …
`200,180,144`); `wood` is `92,64,40`. So the complaint means root **material**
standing above ground, in and through the crown.

Censused on the two PNGs the card itself stored:

| arm | root pixels | above the soil line | highest |
|---|---|---|---|
| roots **off** | 284 | 4 — 1.4% | 1 cell up |
| roots **on** (what landed) | 1,312 | 180 — **13.7%** | **80 cells up** |

The root work grows 4.6x more root tissue, which is what it was for. The
*fraction* standing above ground went up tenfold and the height from one cell
to eighty, all in one plant.

## 2. Reproduced in-engine, on the world the sighting came from

`examples/root_sky.rs` gains `defaultseed=1`, and that is the point of it. A
numbered seed sweep is a **different world** from the one the complaint came
from, and this chain needs rain, which is a pure function of `(seed, frame)`.
An earlier six-seed `grove` sweep read 387 → 377 cells under open sky and was
reported as a null. It was — those seeds do not have the defect.

Paired from **one binary**, `PIXEL_PHYSICS_ROOT_SUBSTRATE` as the switch:

| | rules OFF | growth **and** thickening gates ON |
|---|---|---|
| root cells above the soil line | 791 (12.35%) | 785 (12.16%) |
| worst rise | 79 | 85 |
| `DormantBud` in root material | 198 | 250 |

**A null.** Both rules are correct locally and both are guarded; they do not
govern this population.

## 3. The cause: material marks lineage, not role

`germinate`'s own comment states the design and its assumption in one breath —
*"The companion root is `rootwood`, and that choice propagates for free: every
cell `Grow` creates copies its parent's material, so the whole root system
below ground comes out as rootwood while the shoot above stays wood, with no
cell-type-to-material table anywhere."* The root/shoot split is held up only by
*where growth happens*.

The unambiguous evidence is **198–250 `DormantBud` cells made of root
material**. A bud is placed only at a node, and all seven shipped species give
`RootTip` `plastochron: [0]`, so a root can never reach one. Those cells are
shoot machinery wearing root wood.

It is not only cosmetic: `update.rs::root_reinforced` keys on the *material*,
so a rootwood cell standing above the soil line glues loose powder to itself
in mid-air.

The fix shape is recorded in §W6 and in `dead-ends.md`. It recolours every
plant in the world, so it wants the owner's eye and a blind A/B rather than a
quiet landing.

## 4. What ships

- **`thicken` gets the substrate rule `growable` already had.** It was the
  other site that writes cells and was never gated, because thickening lays a
  cell *beside* an existing one rather than advancing a frontier — CLAUDE.md's
  *which object does this rule evaluate*. Keyed on `reinforces_powder`, a `Vec`
  index on a `Cell` the site already holds, not an `id_of` string hash in the
  sweep. Shares `roots_need_substrate()` with the growth gate so the pair is
  one ablation.
- **`a_root_may_thicken_into_a_cavity_but_not_into_the_sky`** — three arms:
  the refusal, a shoot left untouched, and an underground cavity the rule must
  not over-reach into. Verified **sensitive**: red under
  `PIXEL_PHYSICS_ROOT_SUBSTRATE=off`, green with the rule.
- **A `dead-ends.md` entry** so the next session reading §W6 does not spend an
  evening re-deriving "gate the root's step", which is the obvious move, is
  right, and is not the answer.

### Checked for what the change was not measured on

A substrate test can misfire inside a dense root ball, where every neighbour
of an interior cell is the plant's own tissue and the soil it grew through was
displaced on the way in — that is the false positive that sank an earlier
version of this census, at 23 of 55 cells. If the new gate were suppressing
legitimate thickening there, total root tissue would fall. It does not:
**6,405 root cells with the rules off, 6,453 with them on.**

## 5. Two instrument corrections, both caught before publishing

- The census filed `MatureBody` under *"root work"*. It is the one cell type
  shared by root and shoot, so that column answered a different question than
  its label; it now prints as AMBIGUOUS.
- `grassroot` is also `kind: Plant` and also `reinforces_powder` — it is what
  soil *becomes* under grass — and its cells carry `organism_id == 0`, where
  `aux` is **moisture, not a packed cell type**. Decoding it manufactures cell
  types out of soil wetness. Both are now split out, and both came back
  **zero** on this world, so the bud counts survive the correction. Recorded
  because the check cleared a real result rather than because it found a fault.

## Gates

`cargo test --lib` 1,472 passed / 0 failed / 65 ignored · `cargo clippy
--all-targets --release --locked -- -D warnings` clean · `scripts/docscheck.sh`
clean · `scripts/acceptance.sh` green.
