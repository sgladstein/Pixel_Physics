## Making a bubble of running time look like one

Owner, 2026-09-14: *"I want you to improve the bubbles look. They shouldn't be
a solid line it blocks too much. I am thinking hazy shimmering aura. Think
about how to indicate speed visual."*

A quickening — a circle where time still runs in a world the druid has held —
used to be drawn as a hard 1px outline over the world. It is now a **hazy,
shimmering aura**: a per-cell tint in the world pass, with a ragged rim and
pulses that wash inward. Nothing is painted over; the ground under the haze is
still the ground.

**Where this sits.** Third of the held world's three ways of saying *this is
where you spent your power* — the readout says how much, the preview says
where the next one goes, and this is the one you actually look at while
playing. It is also the last place the game was still telling the player
something by drawing a line across the thing he is watching.

## The complaint and the speed readout were the same object

That is why this is one change and not two. `hud::rings` drew one outline per
circle **plus one more inside it per two steps of the speed dial** — at the top
of the dial, five concentric hard circles three pixels apart over the very
ground the player came to watch. Removing the occlusion removes the speed cue,
so the haze has to carry speed itself, and it has to carry it twice or a
screenshot says nothing (`rings`' own doc: *colour alone is the single-channel
readout this repo keeps learning not to rely on*).

- **In motion: how fast the haze pulses.** Not a mapping at all — the pulse
  phase is `World::frame`, which advances once per `frame::step`, and
  `Druid::update` calls it `speed` times per drawn frame. A x8 circle pulses
  eight times faster **because time in there is running eight times faster**.
- **In a still: how far the haze reaches inward.** 7 cells at x1, 22 at x8, so
  a fast circle reads as full of running time rather than merely rimmed with
  it. This is the half a paused screen, a contact sheet or a review card keeps.

The dial itself is **measured, not told**: `Druid::speed` lives in a file this
lane does not own and `render.rs` is shared by three games, so the renderer
takes the difference between two readings of `World::frame` — which *is* the
dial. `the_rate_the_aura_draws_is_the_rate_the_world_ran` is the positive
control (1, 2, 4, 8 steps between draws -> x1, x2, x4, x8), and a paused frame
holds the last reading rather than collapsing the picture to real time.

**The circle he carries is exempt**, and that needed its own clock.
`step_extra_ticks` lifts the player out of the world for the catch-up passes,
so his own ground genuinely runs at real time however fast the paid circles are
set; `World::frame` is one global counter (the fact that withdrew the
per-circle rate), so the carried disc is drawn from the renderer's own draw
counter instead. Guarded.

## Why it is in the world pass and not in the HUD

`druid::hud` cannot blend — a `Hud::blend` into a region the renderer skipped
compounds frame on frame and oscillates as chunks repaint underneath, which is
recorded at the top of that module and is why every mark it makes is a `put`.
A cell in the world pass is written from scratch this frame, so a computed tint
is one-pass by construction. **And it tints the cell instead of drawing over
it, which is the literal answer to "it blocks too much."**

The rim is deliberately ragged, on `Quickening::contains`' own instruction: a
constant-level disc reads as a soap bubble, which the owner rejected on sight
for foliage, and the repair recorded in `dead-ends.md` is coherent value noise
**keyed to world position so it does not crawl with the camera**.

## What it costs

Measured on the state the dirty-rect skip exists for — a settled held world
with a circle placed in it, which is this game's whole premise. **A counter,
not a clock**, because a counter does not move with whatever else the box is
doing:

| settled held world, one radius-40 circle | pixels recomputed per frame |
|---|---|
| aura off (control) | **0** — the skip is alive, so the arm below measures something |
| aura on | **8,649** — the circle's own bounding box |
| full repaint of that frame | 40,000 |

The aura repaints its own discs and **never** takes the full-redraw path: the
phase is not in the `LookKey`, it unions rectangles into the dirty region the
way the animated liquid grain and `idle_extra` already do. On the game's real
512x320 frame a radius-46 circle is 6.7% of the screen, and it is paid only on
the frames the quantised phase steps — every *other* frame at real time.

Whole frame through `Druid::draw` on the shipped 512x320, radius-46 circle at
x8, eight alternating blocks of 40 frames, **after merging `main`** so Lane A's
button bar and biosphere page are in the frame being compared: **1.699 -> 1.727
ms mean, +0.028**. The worst-frame figure moved the *other* way in all three
runs and does not pin (mean x frames far exceeds it) — noise wearing a number.
The counter beside it is **+1,176 px/frame, identical across all three runs and
both sides of the merge**, which is why it is the one quoted.

`examples/ascii`: 31 scenes, 0 skipped, worst render frame **0.453 ms after
the merge, 0.468 before it** — unchanged **by construction** rather than by
luck, since no `ascii` scene holds a world and every path here returns on
`!world.held`. The guard is what proves that, not the timing; a number that
did not move is not evidence on its own.

## Guards

Seven, in `src/render.rs`. Every count has a known non-zero answer in the arm
that should have one, because a haze reading 0 pixels looks the same whether
the mechanism is quiet or the probe never reached it.

- `the_other_games_cannot_see_the_quickening_aura` — control first (hold the
  world, the picture must move), then byte-identity on an unheld one.
- `the_rate_the_aura_draws_is_the_rate_the_world_ran` — the positive control
  on the instrument the depth channel is built from.
- `a_faster_quickening_hazes_deeper` — scored on the *count*, not on a verdict.
- `the_carried_circle_hazes_at_real_time_whatever_the_dial_says`.
- `the_quickening_haze_moves_with_the_world_clock`.
- `the_quickening_haze_never_covers_what_is_under_it` — no pixel is carried all
  the way to the aura's colour. A hard outline fails this by construction.
- `the_quickening_haze_keeps_the_dirty_rect_skip`.

**All five injected faults were watched going red** (a frozen phase, a depth
that ignores the rate, a replace instead of a blend, a carried disc on the
dial, and removing the dirty union). None of these guards was cited green
without that.

## Tried and dropped

- A **squared inward fade**: the haze changed ~530 pixels over a radius-46
  circle and, against this game's bright sky, one or two units of blue
  survived. It fired, it counted non-zero, and it could not be seen.
- **Rim roughness 2.2** (a clean-edged disc — the soap bubble) and **7.0** (the
  circle stops reading as a circle). 4.5 shipped, chosen off rendered sheets.
- Putting the phase in the `LookKey`: designed, then not built, because
  `render.rs` already had the cheaper device.

## Found, not fixed — one line in a file this lane does not own

**Above x1 the gnome's own circle disappears from the picture.**
`step_extra_ticks` puts the player back after the catch-up passes and does not
put `World::carried` back, so between two updates a held world at speed 2+ has
no carried circle at all: the ground under him greys out under ONE HUE and his
halo is absent. This predates the aura — the old `RING_CARRIED` outline
vanished the same way — and it needs a line in `src/druid/mod.rs`, which
another session holds tonight. Details in the lane note.

The **placement preview** is still a hard 1px circle and is now the most
prominent line on screen. It is not world state, so the renderer cannot see it,
and the ring draw block is another lane's; the lane note has the shape of the
fix.

## Judgement asked for

Two cards are in the review queue, both unanswered at hand-off:

- `20260914T053549208Z-49137a` — **blind A/B, 12-frame sequences at x8**: old
  outline against new haze.
- `20260914T053802005Z-8310ab` — **x1/x2/x4/x8 stills**: does the depth channel
  read speed from a paused screen? I said on the card that I am unsure, and
  why.

Full account, including everything the brief got wrong, in
`Reports/lanes/druid-bubble-aura.md`.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01ACu1rTADqs6PGfwdwFnYBS
