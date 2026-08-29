# Lane A — creature appearance

Branch `claude/creature-appearance-lane-a`.

## 2026-08-29 — extent is the only lever; the palette is already right

**Branch `claude/creature-appearance-lane-a`. Work commit `fb71294eb8a2`** — the
commit adding this note sits directly on top of it, so `origin/claude/creature-appearance-lane-a`
is the head to read.

**The deliverable is review card `20260829T045336581Z-34c3d3`** (board
`creatures`): five body plans, same world, same seed, same colony, 12-frame
sequences. A is the shipped `Chain(2)` ant. Posted, not blocking.

The written finding is `Reports/creature-appearance-design.md`. The headline,
for anyone who reads only this:

- **Contrast is not what is wrong with an ant.** A two-cell ant on a lit
  skyline reads body luma 39 against a surround of 143 — the *highest*
  contrast of any arm measured. It is still unfindable, because a pixel-art
  world's texture is 1–2 cell luminance noise and the ant is competing with
  every rock edge and every leaf.
- **The number that works is `decoys`** — how many other places in the frame
  are at least as different from their surroundings as the animal is. At fixed
  contrast, moving only body size: **1 cell 342, 2 cells 127, 4 cells 55, 9
  cells 15, 16 cells 0.** Those are *after* merging `main` at `f96c08d` in;
  before it, on `ba6fc98`, the same table read 337 / 126 / 57 / 33 / 13 / 0.
  Two worldgen changes deepening the soil roughly tenfold landed in between
  and the effect did not notice — it is a property of the world's texture
  grain, not of its soil.
- **Palette has nothing left in it, and this is now measured twice.** Arm E of
  the card is arm C's body repainted pale — bit-identical run, `moves 598 /
  blocked 197` on both — and it scores **ink/creature 249 against C's 1,285**.
  A nine-cell pale animal puts less on screen than the shipped **two**-cell
  dark one (282). `render.rs`'s standing suggestion to try a *brightness* axis
  rather than a hue axis was tried here and loses too.
- **Shape at constant extent does nothing and is not free.** Two nine-cell
  bodies, a 3x3 block and a waisted 5x2: ink within 0.2% of each other,
  contrast within noise, and **25% vs 44% blocked movement**.
- **A chain is the cheapest extent there is.** `Chain(6)` gets 2.9x the ink of
  the shipped ant for *less* blocked movement (2% vs 4%) — a chain follows its
  head, so it flows over anything a two-cell chain does. It reads as a worm,
  which is the cost.

## → any lane touching `src/sim/creature.rs`

**Composition is structurally unavailable to a creature**, and the seam is
`creature::plant_creature_seed`: every body cell is written as
`Cell::new(material_id, shade)` with one material per species and an
independent `rng::stream(..).below(palette.len())` per cell. The `CellType` is
computed on the next line and thrown away as far as appearance goes.

Two things follow, and if you are editing that function they are cheap to fold
in:

- **Never widen a creature palette to add variety.** A wide spread is speckle
  and speckle destroys a small body's shape — `dead-ends.md`'s `log.ron` entry
  is the same finding on debris. The shipped ant's 24-luma spread is about
  right.
- **Shade-by-cell-type is the one code change worth making**, and only *after*
  the body gets big enough to have a top and a bottom. It is what buys
  countershading, which is the only thing that can hold contrast against both
  the sky a creature is silhouetted on and the ground it tunnels in. §3 of the
  report shows no single flat value serves both.

**I did not touch `creature.rs`.** Nothing on this branch changes a shipped
creature: `ant.ron`, species and material, is byte-identical.

## → any lane taking a luminance or visibility measurement

Two traps, both hit here before the numbers were right:

- **Pin the daylight.** The first run on a generated world landed at night and
  reported a surround luma of **28** where noon gives **153**. Every contrast
  figure would have been a statement about the hour.
- **`app::build_terrain` is not the game.** It is `Spec::Legacy` — thin bare
  platforms over open sky. Every probe in it stood against a flat gradient and
  `surround luma` came back **176.5 to one decimal for every probe**, which is
  the sky. Use a `WorldgenPresets` preset. Not `wetland` — that is the colony
  scene's, and both of its open bugs are about creatures being put on water.

## Files

- `examples/creature_look.rs` — the instrument (`ink`, `decoys`, paired
  with/without render). Not creature-specific; it answers "is this findable"
  for anything drawn into this world.
- `Reports/creature-appearance-design.md` (+ its `Reports/README.md` line)
- `assets/materials/chitin_{mid,pale}.ron`, `assets/species/ant_{long,block,wide}.ron`,
  `assets/species/chitin_pale.ron` — candidates and probes. At most one should
  survive the owner's choice; **none of them should land as-is.**

**Work commit: `fb71294eb8a2c5006a37aa007c6181a0fa4f7bfd`.** The head of the branch is the commit that added this
note, one above it.
