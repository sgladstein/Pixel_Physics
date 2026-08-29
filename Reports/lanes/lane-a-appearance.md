# Lane A — creature appearance

Branch `claude/creature-appearance-lane-a`.

## 2026-08-29 — extent is the only lever; the palette is already right

**Branch `claude/creature-appearance-lane-a`.** `main` is merged in at
`f96c08d`; the PR body is `PR-BODY-lane-a-appearance.md` on the branch. The
head SHA is at the bottom of this note.

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
  cells 15, 16 cells 0.** Measured on three trees — `ba6fc98`, `f96c08d`
  (two worldgen changes deepening the soil ~10x) and `a7b2dd9` (#103's
  placement fix and #106) — and the table is 337 / 126 / 57 / 33 / 13 / 0 on
  the first and *identical* on the last two. It is a property of the world's
  texture grain, which none of those changes touch.
- **Palette has nothing left in it, and this is now measured twice.** Arm E of
  the card is arm C's body repainted pale — bit-identical run, `moves 535 /
  blocked 400` on both — and it scores **ink/creature 251 against C's 1,288**.
  A nine-cell pale animal puts less on screen than the shipped **two**-cell
  dark one (285). `render.rs`'s standing suggestion to try a *brightness* axis
  rather than a hue axis was tried here and loses too.
- **Shape at constant extent does nothing.** Two nine-cell bodies, a 3x3
  block and a waisted 5x2: ink within 0.8%, contrast within noise, on all
  three trees. **A mobility claim here is withdrawn** — the pair read 26/39%,
  25/44% and then **43/41%** blocked once #103 changed where a colony lands,
  so the ordering was placement luck. What survives is the coarse gap: rigid
  25–43%, chain 2–6%.
- **A chain is the cheapest extent there is.** `Chain(6)` gets 2.9x the ink of
  the shipped ant for *less* blocked movement (4% vs 5%) — a chain follows its
  head, so it flows over anything a two-cell chain does. It reads as a worm,
  which is the cost.

## → coordinator, and → lane B: the two findings are one constraint

**Your first ask is a no-op and here is why**, so it can be closed rather than
carried: **no number of mine was ever taken on `scene=colony`.**
`examples/creature_look.rs` builds its own world from a `WorldgenPresets`
preset (`rolling`, never `wetland`), so nothing here was measured on the
pre-#103 scene. Everything has now been re-run on `a7b2dd9` anyway; the decoy
table is byte-identical to the `f96c08d` run and one mobility figure moved a
long way (below).

**Your second ask, answered.** Yes — they are the same constraint, and it is
now §7 of `Reports/creature-appearance-design.md`, stated once:

> **The two things that decide whether a creature is worth looking at —
> extent and palette — are exactly the two things an individual cannot own.**
> Evolution moves the brain and the gut. It cannot move either half of the
> silhouette. So "evolve creatures and add the good ones to the game" today
> means a person hand-authors the appearance of every one, and an evolved
> creature is a recoloured ant by construction — in fact not even recoloured,
> because the colour is inherited too.

Both ends are one line of source each, and I checked lane B's rather than
taking it from the poke:

- **Palette** — `plant_creature_seed` resolves the material as
  `materials.id_of(species_name)` and writes every body cell as
  `Cell::new(material_id, shade)` with `shade` an independent
  `rng::stream(..).below(palette.len())`. One palette *is* the whole
  appearance. Lane B's far end: an exported species with no hand-authored
  material hatches nothing, silently.
- **Extent** — `species_export::individual_as_species` copies the parent's
  `body` verbatim, and says so in its own header. `genome` and `traits` are
  the only things an individual owns. **So the one lever I measure as working
  is not reachable by selection either**, which lane B's write-up does not
  say and is the sharper half.

**The smallest change that opens any of it** is shade-by-cell-type, one line
at that same seam — the `CellType` is computed on the very next line and
thrown away. It buys countershading, which is the only thing that holds
contrast against both the sky a creature is silhouetted on and the ground it
tunnels in, and it is the first step toward an appearance a genome could
reach, because a cell-type assignment is the kind of thing a genome can carry
and a material *name* is not. Worth having exactly when a body is big enough
to have a top and a bottom.

**I did not touch `creature.rs`.** Nothing on this branch changes a shipped
creature: `ant.ron`, species and material, is byte-identical. And **never
widen a creature palette to add variety** — a wide spread is speckle and
speckle destroys a small body's shape (`dead-ends.md`'s `log.ron` entry, the
same finding on debris).

## → coordinator: a claim of mine is withdrawn, and #103 is why

Arms C and D (both nine cells, block vs waisted) read **26%/39%** blocked
movement on `ba6fc98` and **25%/44%** on `f96c08d`, and I wrote that the
waisted outline costs "three quarters again" in mobility. On `a7b2dd9`, with
`colony_ant_site` finding sites the old predicate missed, the same pair reads
**43%/41%** — C nearly doubled and the ordering is gone. It was placement
luck. The coarse gap survives all three runs and is what §5 now claims:
**rigid 25–43%, chain 2–6%.** The appearance numbers did not move at all.

## → any lane taking a luminance or visibility measurement

Two traps, both hit here before the numbers were right:

- **Pin the daylight.** The first run on a generated world landed at night and
  reported a surround luma of **28** where noon gives **153**. Every contrast
  figure would have been a statement about the hour.
- **Your predicate will find the canopy.** I hit R's root cause
  independently, before #103 named it: written as "topmost cell that is not
  air", it finds a `Plant` under any tree and placed **9 of 24 probes**. Two
  harnesses reaching for the same wrong predicate is the argument for #103's
  other half — this one now calls `creature::colony_ant_site` rather than
  keeping a third copy.
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

**Head SHA: `d7f7d2373620ca7ef66df1ecf0f97fadb15e7823` plus exactly one commit on top of it** — the one
that stamped this line, which a note can never name without changing it.
`git log -1 origin/claude/creature-appearance-lane-a` is the head; everything
below it is the work. `main` is merged in at `a7b2dd9`. Gates: `cargo test
--lib` 1,005 passed / 0 failed / 54 ignored, `clippy --all-targets -D
warnings` green on 1.94.1 and on CI's 1.98.0, `docscheck` clean.
