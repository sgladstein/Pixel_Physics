# Zoom for the held world: what it costs, what it buys, and the rung it cannot reach

*2026-09-13. Written by the round-32/33 evolution-lab coordinator, at the
owner's instruction ("Yes druid should get zoom. plan your recommendation").
Status: **plan, not built**. The files it touches belong to the held-world
line (`src/bin/druid.rs`, `src/druid/*`), so §6 is about who does it.*

The owner's standing requirement, 2026-09-13: *"I am not sure what questions
that I answered that suggest zoom should be different between the games, but
that doesn't seem like what I want."* This plan exists to stop the held world
becoming the third divergence rather than the third agreement.

## 1. Where the three games stand

The sandbox (`App`) and the lab (`--bin lab`) both zoom out, and both now
default to a x4 pixel budget — PR #392 and PR #395, both from the owner's own
verdict on the same menu. The held world (`--bin druid`) has **no zoom control
at all**: `src/bin/druid.rs` runs its own `winit` loop, allocates one fixed
`Pixels::new(WIDTH, HEIGHT, ...)` buffer, and drives the camera with
`Renderer::follow` and nothing else. It never calls `zoom_within` or
`adjust_zoom`, and the string `pixel_budget` does not appear in it.

So the three do not currently *disagree* about zoom; the held world simply has
no opinion. Adding one is what this plan is for.

## 2. The number that decides whether this is worth building

The held world is `2560 x 960` (`druid::WORLD_WIDTH`/`WORLD_HEIGHT`) against a
`512 x 320` logical viewport — about five screens of walking.
`render::max_zoom_out_stride` over that pair:

```
stride_w = ceil(2560 / 512) = 5
stride_h = ceil( 960 / 320) = 3        # width wants more than height needs
cap      = 2560 / 320 = 8   (floor)    # the width-driven temper
stride   = max(3, min(5, 8)) = 5  ->  clamp(1, MAX_ZOOM_OUT_STRIDE) = 4
```

**The held world reaches rung 4**, the widest rung on the ladder. That matters
because `Renderer::pixel_scale_for` only ever returns a power of two that
*divides* the rung: at rung 4 a budget of 4 is spent in full, and the widest
view draws every one of its cells into its own pixel instead of discarding
fifteen in sixteen.

Had the cap landed on 3 this plan would be a no-op wearing a feature.

**Run rather than asserted**, with a control either side — both functions
copied verbatim out of `render.rs` into a standalone binary, so this executes
the repo's own arithmetic rather than a restatement of it. (It is not the
crate: a `cargo test` reaching the real `Renderer` is the stronger check and is
worth doing when someone builds this.)

```
control   512x320 world -> max stride 1     # fits one screen: must not zoom out
control  8192x2560 world -> max stride 4    # the sandbox, known to reach the cap
DRUID     2560x960 world -> max stride 4

  rung 1: budget 4 -> pixel_scale 1   (view  512x320  cells)
  rung 2: budget 4 -> pixel_scale 2   (view 1024x640  cells)
  rung 3: budget 4 -> pixel_scale 1   (view 1536x960  cells)   <- the hole
  rung 4: budget 4 -> pixel_scale 4   (view 2048x1280 cells)
```

**And the hole is not where it was expected to be.** §6 raises rung 3 as a
general defect of the ladder; this table makes it a *specific* one for this
world. Rung 3 spans `1536x960` — the first rung showing **the world's entire
height**, 960 rows exactly, and the one a player wanting to see where they are
will stop at. It is also the only rung of the four where the budget buys
nothing. Rung 4 is no substitute: it overshoots vertically by 320 rows of void
and still shows only 2048 of 2560 columns, so it is a different, more
letterboxed picture rather than a better version of the same one.

## 3. What the held world does *not* need

Two of the three costs the lab paid do not arise here, which is most of why
this is small:

- **No cursor conversion.** `Lab::to_logical` exists because the lab's bar is
  clickable and at x4 every button would be four screens off. The held world
  is keyboard-only — `src/bin/druid.rs` binds no mouse event — so there is
  nothing to convert.
- **No ring arithmetic.** `druid::hud::rings` already routes every circle
  through `Renderer::world_to_screen`, which divides by `sampling_stride()`
  and therefore already answers in **buffer** pixels. The rings follow a grown
  buffer for free. That was a deliberate choice by the held-world line ("a
  second copy of it here is the side table that goes stale") and it pays off
  exactly here.

## 4. What it does need

1. **A zoom control, which does not exist yet.** Two separate things, and they
   should be bound the way the other two games bind them rather than invented:
   a rung control (`Renderer::zoom_within`, so it cannot open past the world
   and keeps the centre cell put) and a budget cycle.
2. **`Druid::pixel_budget` + `Druid::viewport()` + `cycle_pixel_budget()`**,
   and a `pixels.resize_buffer(want)` **before** `frame_mut()` in the event
   loop. This is the third verbatim copy of a shape `src/app.rs` and
   `src/bin/lab.rs` already carry — see §5.
3. **The HUD scale.** `druid::hud::Interface::draw` hardcodes
   `Hud::new(viewport.0, viewport.1, 1)` under a comment that says *"this game
   never does"* grow its buffer. That comment becomes false; pass
   `renderer.pixel_scale()`, which is what `render::Hud`'s scale parameter
   (PR #389) is for. Without it the readout draws at a quarter size in the
   corner of a 2048x1280 buffer.
4. **`KEYS` in `druid/hud.rs`.** Its own test asserts every key the binary
   binds is named on screen — the guard written for the bug where eight keys
   were bound, working and invisible. A new key that is not added there turns
   that test red, which is the guard doing its job.

## 5. The recommendation proper: extract before adding the third copy

`App` and `Lab` each hold `pixel_budget`, a `viewport()` that derives the
buffer size from `min(player choice, window cap)`, a `cycle_pixel_budget()`,
and a caller that calls `resize_buffer` before `frame_mut`. They are the same
mechanism written twice. **Writing it a third time is how the lab inherited
the sandbox's 15-in-16 discard and none of its fix** — PR #395's own account
of why it existed.

So: **lift the budget into one shared type in `render.rs`** — holding *both*
halves, the player's choice and the window's cap, deriving the viewport once —
and let each game keep only its key binding and its cap. Then the held world
is a wiring job rather than a reimplementation, and the next game after it is
free.

The constraint this must respect, and the reason "just store it on
`Renderer`" is the wrong shape: `Renderer::pixel_scale_for`'s doc records that
the renderer's copy is *stale by design*, and that sizing a buffer from it
"shipped for one afternoon and panicked in the HUD 100 lines from its cause".
The authoritative pair must live in one place that owns both halves — not in
the renderer with callers guessing when to push.

## 6. Two things to settle before building

**The rung-3 hole, which is not druid's and should be decided once.** The
ladder is `1, 2, 3, 4` and the budget is a power of two, so **at rung 3 the
budget buys nothing** — `pixel_scale_for` returns 1 because 3 has no power-of-
two divisor above 1. `src/app.rs` already guards this ("rung 3 cannot absorb a
power-of-two budget"). A player walking out from rung 1 therefore gets sharp,
sharp, *blurry*, sharp. The held world is where this will be most visible, and
§2's table says why it is worse there than a general blemish: **rung 3 is the
rung that first shows the whole world top to bottom**, so it is at once the
most useful rung and the only one the budget cannot reach.

Three ways out, and it is a look question rather than an argument:

- **Leave it.** Rung 3 stays a soft rung. Cheapest, and it is what ships in
  two games already.
- **Drop 3 from the ladder** (`1, 2, 4`). Every rung then absorbs the budget.
  Costs the one view that frames this world's full height — the very thing §2
  says a held-world player will want.
- **Let the budget be non-power-of-two**, permitting 3 where the rung is 3.
  The only option that keeps the useful view *and* makes it sharp, and the
  largest change: `pixel_scale_for`'s power-of-two walk is load-bearing
  elsewhere, and CLAUDE.md's shared-budget rule says to budget re-deriving
  that as part of the work rather than discover it during.

The cheap move is a blind A/B of rung 3 at scale 1 against rung 4 at scale 4:
the question is which picture a player actually wants out of a five-screen
world, and that is not answerable from the numbers.

**That card cannot be made with `zoomout_pixels`, which is the harness it looks
like it wants.** Checked before promising it. `Arm::new` derives its stride as
`SPAN_STRIDE / scale` and holds `scale * stride == SPAN_STRIDE` — the
**constant-reads construction** that is the whole reason that instrument
answers the question it was built for, since it makes cell reads identical by
arithmetic so the delta is per-pixel work alone. Rung 3 and rung 4 differ in
**span** (`1536x960` against `2048x1280`), so expressing them as two arms means
breaking the invariant the harness exists to hold. Do not "just add an arm".

**`labzoom` is the right instrument and it becomes right once #395 lands.** It
already renders one tile per zoom step at the real 512x320 viewport, with VOID
and MID beside each — which is exactly a rung ladder. What it lacks today is a
budget: it predates `Lab::pixel_budget`. Once #395 is on `main`, a `budget=`
arm over the existing step loop produces the rung-by-rung sheet at scale,
and the card is then near-free.

So the sequencing is: #395 lands → `labzoom budget=` → post the blind card →
decide the ladder → then build §4. Posting a card whose construction does not
match the question is the failure this round already paid for once, when two
review cards offered *different menus* and the difference in construction read
as a difference in the thing being judged.

**The default, which must be measured rather than inherited.** x4 is the
owner's pick in both other games and consistency is a stated requirement, so
x4 is the presumption. But the held world's premise is *a world standing
still*, which is exactly where the dirty-rect render skip earns its keep, and
its `draw` says so in as many words. The honest version of the argument is
that the budget costs **nothing at rung 1** — `pixel_scale_for` returns 1
whenever the stride is 1, which is all ordinary walking — so the cost appears
only while zoomed out, which is when you are reading the map rather than
playing. That should be confirmed with the whole-frame figure, not a subsystem
harness: the sandbox's measured whole-frame cost at x4 is **1.66x** (PR #395),
and per CLAUDE.md a subsystem harness aims the work rather than sizing it.

## 7. Ownership

`src/bin/druid.rs` and `src/druid/*` belong to the held-world line, which has
PR #399 open over them. The extraction in §5 touches `src/render.rs`,
`src/app.rs` and `src/bin/lab.rs` — all three contested. The sequencing that
avoids a three-way collision:

1. Land #392 and #395 (both green, both waiting on trunk churn only).
2. Extract the shared budget (§5) on its own small branch, landed quickly —
   `src/render.rs` is the #7 contested file in the repo and a large diff held
   across a session there is the window everyone else cannot compile in.
3. Hand the held-world wiring (§4) to the held-world line on top of it.

