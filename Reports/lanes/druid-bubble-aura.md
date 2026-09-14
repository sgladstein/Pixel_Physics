# Lane: the quickening aura (held world)

**Branch** `claude/druid-bubble-aura`, cut from `main` at `f2652979`, `main`
merged back in at 19 behind. **PR [#418](https://github.com/sgladstein/Pixel_Physics/pull/418).**
**The ask**, owner 2026-09-14: *"I want you to improve the bubbles look. They
shouldn't be a solid line it blocks too much. I am thinking hazy shimmering
aura. Think about how to indicate speed visual."*

What the diff shows is in `PR_BODY_LANE_D.md`. This is what it cannot.

## Review cards posted (fire-and-forget, unanswered at hand-off)

| id | what it asks |
|---|---|
| `20260914T053549208Z-49137a` | **blind A/B, 12-frame sequences, x8** — old outline against new haze. Which reads as a bubble of running time rather than a line over the world. |
| `20260914T053802005Z-8310ab` | **four stills, x1/x2/x4/x8, not blind** — does the depth channel read speed from a *paused* screen. I am genuinely unsure and said so on the card. |

Collect with `python3 scripts/review.py inbox`. On the blind card, read the
prose through `blind_was` before believing it — the skill's own worked case.

## The frame cost, and the state it was measured in

`CLAUDE.md` asks for the cost against the state the optimisation exists for.
For this game that is a **held world standing still with a circle placed in
it**, because the whole premise is a world that is not moving and that is
exactly where the dirty-rect skip earns its keep.

**The counter, which is the number to quote** (load-independent, and it is
what a defeated skip actually looks like). Settled held world, one radius-40
circle, `Renderer::draw` with `force_full: false`, guarded by
`the_quickening_haze_keeps_the_dirty_rect_skip`:

| | pixels recomputed per frame |
|---|---|
| aura off (control) | **0** — the skip is alive, so the subject arm is measuring something |
| aura on | **8,649** — the circle's own bounding box, 93x93 |
| a full repaint of that frame | 40,000 |

So the aura repaints its own discs and never the screen. On the game's real
512x320 frame a radius-46 circle is 11,025 px, **6.7%**. It pays that only on
the frames the quantised phase steps: at real time that is every *other* drawn
frame, at x8 every one — which is the right shape, because x8 is the circle
that is supposed to look fast.

**The clock, for scale only, and it is the weaker number of the two.** Whole
frame through `Druid::draw` — the call the game itself makes, HUD and all,
because a subsystem harness overstates — eight alternating blocks of 40 frames,
two settings of one binary, on the shipped 512x320 with a radius-46 circle at
x8: **1.787 -> 1.791 ms mean, +0.004**. The *worst* moved 7.363 -> 5.929, i.e.
the wrong way, and did so in both runs; it does not pin (mean x frames = 572 ms,
far above the worst), so it is an order statistic over many similar frames and
means nothing. The counter beside it is deterministic — **+1,176 px/frame in
both runs** — which is why it is the one to quote.

An earlier build of the same feature measured **+0.051 ms** here. The
difference is the per-disc squared-radius rejection: the union of every
circle's bounding box is one rectangle, so two circles far apart made every
pixel between them pay a square root per circle. Each disc now carries its
band as two squared radii, and the interior and the outside are one compare
each. The render is **byte-identical** before and after — checked by rendering
the same frame twice and `cmp`-ing it, which is the only control a pure
optimisation has.

**`examples/ascii` is unchanged, and by construction rather than by luck**: 31
scenes, 0 skipped, worst render frame 0.468 ms. No `ascii` scene holds a
world, and every path in the aura returns on `!world.held`. The guard
`the_other_games_cannot_see_the_quickening_aura` is what proves that — it
holds the world, checks the picture *moves*, then un-holds it and asserts
byte-identity. A timing that did not move is not evidence.

## Where the brief was wrong

The coordinator asked to be told. Four places, and the first two would each
have cost real work.

1. **"Add a sibling at the END of that chain, after `apply_held_look`"** —
   right, and not enough. `cell_colour` **returns early for empty cells**, at
   the `background_at` branch, long before that chain: the first sheets had
   the haze hugging the ground and stopping dead at the skyline. **A circle of
   running time is mostly air.** Two call sites, not one.

2. **"A shimmer must enter the redraw key or it will smear."** The diagnosis
   is exactly right and the prescription is wrong — following it buys a
   full-screen repaint whenever a circle exists, which is the animated
   grain's ~10 ms. `render.rs` already had the cheaper answer for precisely
   this shape and the brief did not mention it: the animated grain unions the
   *chunks holding liquid* into the dirty region by hand, and `idle_extra`
   does the same one cell at a time. The aura unions the discs' bounding
   boxes. **`held_key` was not touched at all**, so nothing about this change
   can force a full frame.

3. **"Inside a quickening time is running, so those cells are already dirty
   every frame."** Offered as a hypothesis and it is false — which is
   fortunate, because it is *why* the union above is needed. A circle over
   settled rock writes no cell, so `take_touched_chunks` reports nothing about
   it: with the aura's rectangles removed, the guard's settled world
   recomputes **0** pixels and the haze does not animate at all. Put that
   fault back and the guard goes red on "the aura repainted nothing".

4. **The speed dial cannot be read from `render.rs`.** `Druid::speed` lives in
   a file this lane does not own, and `render.rs` is shared by three games.
   It is **measured** instead: `World::frame` advances once per `frame::step`
   and `Druid::update` calls it `speed` times per drawn frame, so the
   difference between two draws *is* the dial. Positive control, because a
   difference is one of the five instrument shapes that has lied in this repo:
   `the_rate_the_aura_draws_is_the_rate_the_world_ran` steps a held world 1,
   2, 4 and 8 times between draws and asserts the renderer reads x1, x2, x4,
   x8 — then that a frame with nothing stepped *holds* the last reading, so a
   pause does not change what the picture says about the speed.

   The brief's own suggestion — phase from `World::frame`, so a x8 circle
   pulses eight times faster because time there is running eight times faster
   — is right and is what channel 1 does. It needed one correction, below.

5. Minor, confirmed: **`scripts/review.py` has no `--meta` on any subcommand**.
   Both cards went through `post --json`.

## Two bugs found on the way, one of them not mine to fix

**The gnome's own circle disappears above x1, and it is not the haze.**
`Druid::step_extra_ticks` lifts the player out of the world for the catch-up
passes; each of those passes runs `frame::step`, which sets
`World::carried = None` because there is no player. The player is put back
afterwards and **`World::carried` is not**. So between the end of an update
and the next one — which is exactly when everything draws — a held world at
speed 2 or more has no carried circle at all. The ground under the gnome greys
out under ONE HUE and his warm halo is absent; the old `RING_CARRIED` outline
vanished the same way, so this predates the aura by however long the dial has
existed. **The fix is one line in `src/druid/mod.rs`** (restore `carried`
beside `player`, or step once more with him present), which is Lane A/B
territory tonight. Visible in the x8 arms of both review cards.

**The carried circle is not on the dial, and a phase read off `World::frame`
alone says it is.** Same root fact that withdrew the per-circle rate
(`dead-ends.md`, `Quickening::rate`): `World::frame` is one global counter. The
carried disc is therefore drawn from `Renderer::frame`, the renderer's own
draw counter, which advances once per drawn frame and so *is* real time — both
its pulse and its depth. Guard:
`the_carried_circle_hazes_at_real_time_whatever_the_dial_says` (standing 22.4
cells against carried 7.0 at x8; watched going red by giving the carried arm
the measured rate).

## Tried and dropped

- **A squared inward fade** (`(1-t)^2`). Puts nearly all of the band's area
  under a very small number: the haze changed ~530 pixels over a radius-46
  circle and, against this game's bright sky, the surviving change was one or
  two units of blue. A mechanism that fires, reports a non-zero count, and
  **cannot be seen** — which is the case the house rule about counters is
  usually pointed the other way at. Linear.
- **Rim roughness 2.2** — a clean-edged disc, i.e. the soap bubble
  `Quickening::contains`' own doc warns about and `dead-ends.md` §1307 records
  the owner rejecting on sight. **7.0** — ragged enough that the circle stops
  reading as a circle. **4.5** shipped; sheets for all three were compared by
  eye before choosing.
- **Putting the phase in `LookKey`** — designed, not built, see 2 above.

## What is still a hard line

The **placement preview** at the gnome's feet is still a 1px circle, and it is
now the most prominent line on screen. It cannot be softened from here: it is
not world state (it is `Druid::place_radius` at the player), so the renderer
cannot see it, and `Interface::draw`'s three-line ring block is Lane A's. If
the owner wants it soft too, the shape of the fix is a
`Renderer::aura_preview: Option<(i32, i32, i32)>` set from `Druid::draw`, drawn
by the same transform at a much lower alpha and with no pulse — "not there
yet" is then *faintness plus stillness* rather than a different kind of mark.

There is also **no runtime selector** for the aura's dials, which `CLAUDE.md`
asks for on a does-this-look-right question. `AuraTuning` is a public field on
`Renderer` and `examples/druid_aura.rs` sweeps every knob from the command
line, but binding a key needs `src/bin/druid.rs` or `src/druid/menu.rs`.

## Files

`src/render.rs` (the aura, seven guards), `src/druid/hud.rs` (`fn rings` only —
Lane A's hunks in that file start at line 897 and do not overlap),
`examples/druid_aura.rs` (new).
