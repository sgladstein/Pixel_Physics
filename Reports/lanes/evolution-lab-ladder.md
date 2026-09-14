# Round 34, lane B — the soft rung of the zoom ladder, put to the owner

*2026-09-14. Branch `claude/lab-zoom-rung-three`, **PR #406**, code at
`39170d31`. **Deliverable: one blind review card,
`20260914T000047337Z-693fcc`, board `druid` — answered 2026-09-14, and the
answer is below.** Nothing is built on it and nothing should be until whoever
takes the next step reads what it does and does not settle. The coordinator
owns the merge.*

The lane's whole job was to stop asking a look question in prose. It has been
asked three times in writing — `Reports/held-world-zoom-plan-2026-09-13.md` §6
is the third — and not answered, which is this repo's own evidence about what
prose is for.

## The defect, and why it is the held world's problem in particular

The zoom ladder is `1, 2, 3, 4` and `Renderer::pixel_scale_for` only ever
returns a **power of two that divides the rung**, so rung 3 can absorb no
budget at all. Walking outwards a player gets sharp, sharp, *soft*, sharp.
`src/app.rs` already names it in a guard.

Run against the real crate rather than a copy of its arithmetic — the plan's
§2 ran two functions lifted out of `render.rs` into a standalone binary, and
this is the shipped `Renderer` driven at the real viewport over the real
2560x960 held world:

```
RUNG 1  VIEW 512x320    PIXELS X1   VOID 0%
RUNG 2  VIEW 1024x640   PIXELS X2   VOID 0%
RUNG 3  VIEW 1536x960   PIXELS X1   VOID 0%    <- the hole
RUNG 4  VIEW 2048x1280  PIXELS X4   VOID 25%
```

`VOID` is the share of the screen the world does not reach, and it is the
number the plan could not state: **rung 3 is the only rung that frames the
land exactly**, 960 rows of world in 960 rows of view. Rung 4 spends the
budget in full and pays a quarter of the screen for it, in black bars above
and below. So the soft rung is also the good one.

## What shipped in `examples/labzoom.rs`

Three knobs, and nothing outside that file:

- **`budget=N`** — the pixel budget, which `labzoom` predated. Each tile's
  label gains `PIXELS X{n}`, the scale *actually in force*, so a rung that
  refuses the budget says so in its own caption.
- **`world=lab|druid`** — `druid` photographs the real held world through
  `Druid::new()`, centred on the gnome. `src/druid/` is read, never written.
- **`spend=pow2|any`** — `any` takes the largest divisor of the rung rather
  than the largest power-of-two divisor, so rung 3 spends 3 of a budget of 4.

**`spend=any` is the proposal's own output, not a mock-up of it, and it needed
no change to `pixel_scale_for`.** An unrestricted budget of 4 at rung 3 draws
1536x960 cells into 1536x960 buffer pixels — `sampling_stride` exactly 1 — and
the shipped renderer already draws precisely that when `zoom_out_stride` is 1
and the viewport it is handed is 1536x960. Same span, same clamped camera, same
cell walk, and `minifying()` false in both because that stride is 1. That is
the finding worth carrying out of this lane: **option 3 of the plan's §6 can be
photographed from an example**, so nobody has to land a change to `render.rs`
to find out whether it is wanted.

Every tile is normalised to one display size before it is written, because a
x4 tile holds 2048x1280 buffer pixels and a x1 tile holds 512x320 and the
window upscales both to the *same* rectangle. A sheet laid out at buffer sizes
would compare a big picture with a small one and call the difference
resolution.

## The controls, which are the reason to believe the numbers

Same binary, same world, three settings; the per-rung tiles compared byte for
byte:

| | rung 1 | rung 2 | rung 3 | rung 4 |
|---|---|---|---|---|
| `budget=1` | X1 | X1 | X1 | X1 |
| `budget=2` | X1 | X2 | X1 | X2 |
| `budget=4` | X1 | X2 | **X1** | X4 |
| `budget=4 spend=any` | X1 | X2 | **X3** | X4 |

- **Specificity.** At `budget=1` nothing spends anything, which is the
  pre-2026-09-13 behaviour the renderer's own doc promises.
- **Sensitivity.** `spend=any` changes rung 3 and **nothing else**: the rung
  1, 2 and 4 PNGs are md5-identical across the two runs, and only rung 3
  differs. So the knob is connected, the world is the same world in both arms,
  and the pane the card is judging is the only thing that moved.
- The span never moves with the budget in any row, which is the safety claim
  `pixel_scale_for`'s doc makes.

## The card

Three panes, blind, board `druid`, `gallery` kind so all three fit — **the
three options of §6 are three pictures, and a two-pane A/B cannot separate
"leave rung 3 soft" from "make rung 3 sharp"**, which is the one the plan
prefers. Same world, same centre cell, same on-screen size, each pane carrying
its own span, its own pixels-per-cell and its own black-border share in `meta`.

One artifact is mine rather than the engine's and is declared on the card: to
reach a common size, the x3 pane is stretched 4:3 (every third row and column
doubled). No common size below 6144 px is a whole multiple of 1, 3 and 4, and
every alternative *removed* detail from one of the panes instead of duplicating
rows in one.

The card weighs 7.4 MB of PNG, which is heavy for the queue — the panes are
2048x1280 because the question is sharpness and a smaller pane answers it
wrongly.

## The verdict — answered 2026-09-14T00:38Z

**He chose rung 4: the wider view, sharp, with a quarter of the screen in black
bars.** Not the rung that frames the land exactly, in either of its two forms.

Decoded rather than read raw, because the panes were shuffled — he saw **A** as
rung 4, **B** as rung 3 made sharp, **C** as rung 3 as it ships. He clicked A.
`scripts/review.py get` prints this map itself under `blind_decoded`; do not
hand-derive it.

**No comment, no rating, no pins — this is the click alone.** What it settles
and what it does not:

- It settles that **the 25% letterboxing is not the problem I assumed it was**.
  The plan's §2 reasoned that rung 4 "is no substitute" because it overshoots by
  320 rows of void; the owner picked it over both full-height framings anyway.
  Seeing 80% of the world's width beat framing 100% of its height.
- It is therefore evidence **for §6's option 2, dropping rung 3 off the ladder**
  (`1, 2, 4`), whose only stated cost was losing the full-height view he has
  just declined twice — once soft and once sharp.
- It is evidence **against option 3**, the non-power-of-two budget, which is the
  expensive one: he was shown exactly what it buys, beside what it costs
  nothing to have, and did not take it.
- **What it does not settle is whether rung 3 should be *removed* or merely
  passed through.** The card asked which picture he wants at rest, not what the
  ladder should do on the way there. If dropping 3 is proposed, that is worth
  one more card — two rungs against three, walked — rather than inferred from
  this one.

## For whoever picks this up

- The three outcomes are priced in `Reports/held-world-zoom-plan-2026-09-13.md`
  §6. Nothing in this lane touched `src/render.rs`, `src/app.rs`, `src/lab/`
  or `src/bin/`; the shared-budget extraction of §5 is still the next lane and
  is unaffected by which way the verdict goes.
- **What the card cannot answer is cost.** `spend=any` at rung 3 draws nine
  times the pixels of today's rung 3, against sixteen at rung 4; the sandbox's
  measured whole-frame figure at x4 is 1.66x (PR #395) and nobody has measured
  x3. If the verdict is the sharp rung 3, that measurement is part of the work.
