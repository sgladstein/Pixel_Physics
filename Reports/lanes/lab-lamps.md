# Lane: the lamps light the bed

*Branch `claude/lab-lamps-light-the-bed`, 2026-08-30. Design of record for
this work is [`Reports/lab-lamps-light-the-bed-2026-08-30.md`](../lab-lamps-light-the-bed-2026-08-30.md)
— read that for the measurements. This note is what another lane needs from
me.*

## What I changed, in one line each

| file | what |
|---|---|
| `src/sim/material.rs` | `Material::beam` — light thrown down the emitter's own column. Defaults to 0, so every existing material is unchanged |
| `assets/materials/growlamp.ron` | the fixture. **Appended at the end of `MATERIAL_FILES`**, so no id moved |
| `src/sim/field.rs` | per-block `beam` gathered in `rebuild_blocked`'s existing scan; `apply_sky_to`'s descent re-seeds from it. Gated on `FieldTile::has_beam`, false everywhere outdoors |
| `src/sim/world.rs` | `World::set_sky_lighting(bool)` — the box has no sun |
| `src/lab/scene.rs` | fixtures are `growlamp`, the box is sunless, `lamp_spacing` 128 → 64, and the move API |
| `examples/lamp_probe.rs` | new: granularity, stand, and paired frame cost |
| `examples/labshot.rs` | `movelamp=from,to`; `lamps=0` now goes through the scene's own API |

## What another lane needs to know

**If you are the parameters-panel lane** — the API is in
`src/lab/scene.rs` and the report's §6 spells out the four gotchas. Short
form: `spec.lamp_near(&world, x)` to pick one up, `spec.move_lamp(&mut world,
from, to)` to drag it, and the light follows on the next field step. It
refuses rather than clamps at the wall, so you can call it every frame of a
drag. **You do not need to snap to `FIELD_SCALE`** — sub-block drags are real
and measured.

**If you are the sky-light lane (`claude/lab-skylight-cost`)** — no file
overlap; your change is entirely in `render.rs` and mine does not touch it.
Your finding and mine are the same territory from opposite ends, and there is
something in mine for you: `World::sky_lighting()` is now a world-level "this
room has no sun", so the render-side scan you are optimising may be able to
*read* that rather than derive it. I did not do it because it is your file and
you are mid-rewrite.

**If you are taking `FIELD_SCALE` to 16** — the report's §4 is addressed to
you, and it has a one-line dependency: `LAMP_HALF` is
`max(7, FIELD_SCALE - 1)`, already in place and bit-identical at 8. **Take it
with the constant.** Without it a 15-cell fixture fits inside a 16-cell block
and dragging a lamp does nothing for ten columns and then lurches 4.9 cells,
dimming as it goes. With it the sweep is as clean at 16 as at 8.

**If you compare `field::field_hash` across this change** — it now covers the
beam array, so the digest moved for every world even where no field value did.
A mismatch across 2026-08-30 is not evidence of a behaviour change.

## What I deliberately did not do

**The light pool draws on the back wall, not on the ground.** The bench light
is real — the plants respond to it, and moving a fixture kills the plant it
leaves behind — but `render.rs`'s field-light read is gated on `glow_tiles`, a
glowing tile plus its 3x3 neighbours, so nothing samples the field nineteen
blocks under a fixture. Ungating it for a beam-lit column is small, and it is
in the 722 lines the sky-light lane is rewriting. Left for whoever lands
second, and put to the owner as a question on the review card rather than
decided here.

**I did not touch `src/lab/ui.rs`, `src/lab/mod.rs` or `src/bin/lab.rs`.**

## Open, and stated rather than tidied away

The pool's *total* flux ripples about ±11% with sub-block phase as a fixture is
dragged, from the `max` in the descent rather than from the emitter model. The
peak and the position are clean, so this is a brightness shimmer on a drag and
not a position error. Not chased.
