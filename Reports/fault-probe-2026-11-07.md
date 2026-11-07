# Fault probe: is a running crack satisfying to cause, and can you read where it goes? (2026-11-07)

**Status: first playable version built and measured; the owner's verdict
pending. A plan does not yet beat random play.**
Probe 6 of the prototype lanes from the 2026-11-07 new-game brainstorm
(brief: `/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/prototypes/6-fault.md`).
Play it with `cargo run --release --bin fault`. Harness: `examples/faultprobe.rs`.

## The game

An ice wall holds back a lake that rises on a clock (the thaw, 120 s). If
the lake reaches the crest, the wall bursts. The player cuts a spillway
instead, with four tools: **Tap** (start a crack; 10 a wall), **Drill**
(bore a hole that stops a crack; 5 a wall), **Meltwater** (pour water that
wets the cracks it reaches) and **Frost** (freeze wet cracks, which wakes
the dead crack tips that touch them and runs them on). Cut a piece free and
the lake shoves it off the crest when the water reaches its back; the lake
drains through the gap. A mill downstream must not be hit by ice or flooded.
Stars: one for no burst, one for draining at least half the lake, one for
the mill left alone.

## What is new and what is shipped

- **New, game-only** (`src/fault/`): a tension field (a screened relaxation
  fed by the lake's depth and the weight of ice above, which does not cross a
  crack), crack tips that walk the **ice's own grain** (joints between
  `fracture_field::domain` cells at scale 10) towards the most strained
  neighbour, fork when strained, and stop at a hole, a face or another crack;
  severance of a closed ring; the lake's shove (a cut piece slides along its
  crack, grinding the step corners to snow, and peels off the crest a grain
  block at a time); a support sweep that lets fall any ice nothing holds up.
- **Shipped**: the crack bits on `Cell`, the body code (`rigid.rs`) the
  blocks fall through and break on, water, the renderer and the HUD.
- **One shared edit**, additive: `rigid::throw_block`, a public way to hand a
  block of cells to the body code with a push, for a caller that decided
  severance itself. Nothing else calls it.
- **Per-world material edits** (the registry is owned by the world): ice
  melts at 400 rather than 4 (no weather here keeps it cold), is recoloured
  so a white crack reads, uses stone's fragment ladder, and water never
  freezes (a cold front skinned seed 3's lake with single ice cells).

## Results (10 walls, paired by seed; `faultprobe` with no plan)

| | burst | stars (total of 30) | runs with 2+ stars |
|---|---|---|---|
| do nothing | 10 of 10 | 0 | 0 |
| random (a tool every 2 s, 70% taps) | 5 of 10 | 7 | 2 |
| planned (look-ahead, staged notches) | 5 of 10 | 5 | 0 |

- **Doing nothing loses**, every wall.
- **Random loses to nothing it shouldn't, and the planner does not beat
  it**: planned against random by stars, 3 better, 3 worse, 4 tied; sign
  test p = 1.0. Each beats doing nothing on 5 walls with the rest tied
  (p = 0.06 each, two comparisons).
- **Before the tap stock**, random tapping (about 45 taps a game) kept the
  lake under the crest on 7 of 9 walls by dicing the wall down; the stock of
  10 halved that.
- No run earns three stars: every run that drains half the lake throws ice
  at the mill.

The planner (`Planner` in the harness) tries every tap in a band in its head
(`Game::trial` + `settle_cracks`) and keeps the one whose cracks best cut
across the wall at the target row. It stands in for a player who can read
where a crack will run, **which is the question the probe asks of a person,
not something this answers.** Its failure says a straight look-ahead on
crack geometry does not plan this level well; it does not say a person
cannot.

## Repeatability (`faultprobe repeat=1`)

One tap at the same spot and moment:

- **The same wall twice: identical**, edge for edge (48 edges).
- **Ten walls**: the crack stays near where you tapped (its rows span 42..76
  for a tap at row 60) but turns a different way on each wall, following
  that wall's grain. Overlap with wall 1's crack is 0-6%. Only one of ten
  crossed the wall. So "roughly where" is readable; "which way" is not, from
  the ice alone. That is what the shimmer (`S` key) is for.

## Frame cost

Single runs of seed 6 with nothing else running (`faultprobe
plan=nothing|planned seed=6`), one game step (tension, tips, slides, support
sweep and the shipped `frame::step`) at 256x160, drawing excluded:

| | mean | worst | worst within a second of a slab breaking |
|---|---|---|---|
| do nothing | 1.42 ms | 6.8 ms | (no slab broke) |
| planned | 1.83 ms | 11.4 ms | 11.4 ms |

The four-wide table runs, contended with each other and a build, saw worst
frames up to 44 ms; treat those as noise. The heavy frames are slab breaks
(many bodies at once) and the support sweep's flood, every 6 frames.

## Tried and rejected along the way

- **Handing a closed ring to the shipped load model**: a ring inside the
  wall reported "confined" every 5 frames (43 failures, 2,924 cells over 240
  frames) and released nothing; one at the crest "crumbled" to snow. Severance
  is decided in `tips::find_severed` instead, and crack cuts schedule no
  structural check.
- **Releasing a piece with no push**: the body code wrote it back where it
  was, crack bits lost (192 cells back in place 30 frames later).
- **Throwing the piece** (a heave out of the wall): 1.6/0.9 fell back and
  dammed the notch; 2.6/1.4 and 2.2/1.3 landed slabs on the mill. Replaced by
  the slide along the crack.
- **Dropping the whole slab once half overhung**: the upstream half fell
  onto the stub and welded into a lump. **Toppling it at its centre of
  mass**: ten blocks fell together onto the crest. Replaced by per-block
  peeling.
- **Dropping a jammed slab's remainder as resting bodies**: they set on a
  neighbouring slab that then slid away, leaving lumps hanging off the crest
  for the rest of the game. Replaced by `tips::support_sweep`, which also
  needed: a floe on water is held (one was re-dropped 177 times); a lump the
  body code puts back is trusted for 2 s; a lump wedged by its sides (the
  body code cannot drop it) is let down by hand.

## Open

- Make a plan beat random: likely the level (a staged spillway the mill
  can survive) more than the planner.
- Cracks curl into hooks on some walls (wall 1's repeat crack): the step rule
  prefers joints over tension strongly enough to wander.
- The shimmer is faint; the blind A/B on the page asks whether it helps.
