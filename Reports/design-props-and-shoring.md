# Props and shoring — the verb the delay is already being paid for

**Status: design, nothing built.**

## The finding this starts from

The owner's stated requirement for the collapse delay is:

> collapse must be obvious and delayed, **so the player can get supports in
> first**

`CHAIN_WINDOW_FRAMES` is 600 frames of deliberate generosity in service of
that sentence. **There is no support to get in.** `Tool::{Brush, Rect, Room,
Line}` all do one thing — paint bulk material — and `explosion`, `strike` and
`mine_swept` all do the other. A whole design constraint is being paid for to
enable an action the game does not have.

That is `CLAUDE.md`'s second law, exactly:

> **There must be a verb, and it must deliver something.** … If a system can
> only be changed by the world changing around it, the player is a spectator
> of it.

Destruction has three verbs. Support has none. The structural model's entire
subject is *what holds what up*, and the player can only take away.

## Why painting stone is not the same thing

You *can* draw a column with `Tool::Line` and it does engage the model
properly — `World::paint_capsule` records a disturbance per structural cell
and pays for `relax_region`, so a painted column really does re-route load.
But it is the wrong instrument for the job in three ways:

- **It is masonry, not shoring.** Same material, same weight, same cost. A
  prop is supposed to be cheap, quick and weaker than what it holds.
- **It cannot fail informatively.** Painted stone either holds or breaks like
  any other stone. A prop that is overloaded should *tell you* — bow, creak,
  then snap — which is the first law applied to the constructive side.
- **It reads as building, not as rescuing.** The ten-second window is a panic
  window. Dragging a rectangle is not a panic action.

## The design

**A prop is a one-cell-wide placeable that adds a route to `support_count`.**
Nothing about the load model changes; the whole effect falls out of machinery
that already exists:

- `load::support_count` counts every neighbour strictly closer to an anchor,
  and **load divides among them** — the module's own note says a slab in the
  middle of a bridge span is held by both legs, not by whichever the tie-break
  preferred. So one prop under a sagging span genuinely halves what the
  original carrier holds. That is not scripted; it is `dependants` doing its
  job.
- Placement is a click, not a drag: one column from the cursor down to the
  first thing that can bear it.

### The material

A new `.ron` in `assets/materials/`, alongside `stone`, `wood` and `log`.
Three fields carry the whole design:

| field | what it buys |
|---|---|
| `max_unsupported_span` | small — a prop is a strut, not a beam. It carries in compression and spans nothing. |
| `support_cost_below` / `_beside` / `_above` | cheap below, dear beside and above: a prop that is asked to cantilever should be a bad answer. |
| `breaks_into` | what a failed prop leaves. Splinters, so a failure is legible as *the prop went*, not as *the roof went*. |

**`log` is the closest existing precedent and worth reading first.** It is the
one material in the shipped set that opts out of the span rule
(`max_unsupported_span == u16::MAX`) and spends its life lying on rubble, and
`open-bugs-handoff.md` §T1b records the bug that came of that opt-out holding
against bending and silently not against bearing. A prop wants the opposite
settings, but the same care.

### The failure, which is the point

`CLAUDE.md`'s first law says the outcome must have a middle. A prop that is
merely present or absent has none. The middle is:

1. **carrying** — normal;
2. **strained** — over some fraction of capacity: it emits, exactly as
   `design-load-telegraph.md` describes, and this is where the two designs
   meet;
3. **failed** — it breaks, and what it was holding gets its old load back.

Step 3 is free: a prop is body material, so when it fails, `dependants`
re-routes automatically. There is no bookkeeping to write.

## What this unlocks that nothing else does

- **The ten-second window becomes gameplay** instead of a tuning constant.
- **`arch-vs-lintel-measurement.md` becomes discoverable.** The arch spans
  1.6x further at equal material, and a player will never find that out by
  rebuilding a chamber twice. With a cheap prop they can experiment inside one
  chamber: prop the flat roof, see it hold, pull the prop, watch it go.
- **It is the safe half of a dangerous verb.** Mining games make the player
  brave; this makes them *careful*, which is a different and rarer feeling.

## What it costs

Small, and the risk is not the code.

- A material file, a `Tool` variant, a placement routine, and a HUD entry.
- The placement routine's only real question is *where does the prop stop* —
  down to the first cell that `load::rests_on_ground` or `touches_bedrock`
  accepts, refusing to place if it never finds one. A prop standing on nothing
  must be refused at placement rather than silently placed and then dropped.
- **The frame cost is a wash**: a prop is a handful of cells and it goes
  through `paint_capsule`, which already pays `relax_region` for exactly this.

## The falsifying experiment

| question | how | what falsifies it |
|---|---|---|
| **does one prop actually change the verdict?** | `arch_probe`'s scene shape: take a lintel one span *past* its margin, place a single mid-span prop, census what stands | the roof still failing — if `support_count` does not re-route enough to matter, the whole premise is wrong and it needs a capacity change, not a placeable |
| **does the margin move by a satisfying amount?** | sweep the prop count 0/1/2/3 against the lintel's margin | a prop buying one or two cells of span: technically working, and worthless to a player |
| **can a prop be overloaded rather than binary?** | load the propped span until the prop fails first | the prop never failing before the roof — then it is an invincible crutch and the middle does not exist |

**Run the first one before writing any UI.** It is `arch_probe` with one extra
column of cells and it either says the mechanism carries the feature or kills
it in an afternoon.

## The judge-by-eye question

Two, and they are the ones the numbers cannot answer:

1. **Does placing a prop feel like a rescue?** Blind A/B through
   `scripts/review.py` on the panic case — a roof coming down, propped in time
   against not propped — as frames, with the failure counts in `meta`.
2. **Does a prop failing read as your mistake or as the game cheating?** This
   is the one to get wrong. A prop that snaps with no warning is a punishment;
   one that groans first is a lesson. Which is why this design and
   `design-load-telegraph.md` should probably ship together, and why the
   telegraph should ship first.
