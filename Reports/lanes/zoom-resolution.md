# Lane: zoom-out resolution (round 32)

*Round 32 closed 2026-09-13 (#385, #389, both merged). Round 33 follow-on
below: the lab half. Coordinator owns every merge.*

Distinct from `evolution-lab-zoom-out.md`, which is round 31's lane about how
*far* out the view may go. This one is about how many pixels the view is drawn
with once it gets there, and it deliberately reopens none of that.

## The two PRs, and the order

| | |
|---|---|
| **#385** `claude/zoom-out-resolution-measure` | The price. Instrument + report, no production change. |
| **#389** `claude/zoom-out-resolution` | The thing. **Contains #385**, merged in, identical content. |

**Either order works and there is nothing to reconcile** — whichever lands
first, the other's copy of those files is the same. #389 alone is sufficient;
#385 exists because the measurement was worth landing on its own while the
implementation was still being written.

## What it does

`App::pixel_budget` (`+` while zoomed out, x1 / x2 / x4, **default x1**) grows
the render buffer at zoom-out so more of the view reaches a pixel of its own.
The renderer draws into a fixed 512x320 back-buffer that the window magnifies,
so at the widest rung 94% of the cells in view could not reach a pixel at all.

`ZoomOutFilter::Coverage` (round 31) already stopped things *disappearing*. What
it could not stop is a one-cell stem being **drawn four cells wide**. That is
what this fixes.

## The numbers, if you are quoting them

Measured on the shipped path (`App::update` + `App::draw`, HUD included),
`zoomout_pixels game=app`, median of 11 at `RAYON_NUM_THREADS=4`:
**x2 costs 1.29x the whole frame, x4 costs 1.66x** — for four and sixteen times
the cells respectively. Sub-linear because `pixels x stride²` is constant, so
cell reads do not move and only per-pixel colour work does.

Quote the app figure, not the pre-build one (1.31-1.37x / 1.79-2.07x). Both are
in `Reports/zoom-out-resolution-2026-09-13.md` and the gap between them is
itself the point.

## What the coordinator should know

- **The default is off on purpose, and the verdicts decide it, not me.** Three
  cards on `board=zoom`, all pending: lab `20260913T083914900Z-764956`, outdoor
  `20260913T083948135Z-0f4767`, the real game `20260913T100843436Z-de27a0`.
  The lab plainly gains; **outdoor rock arguably loses its grain to smoothness**,
  which cuts against a house style that is chunky on purpose. If the owner likes
  it, flipping the default is a one-line change to `App::pixel_budget`'s
  initialiser plus the README and wiki lines that say it is off.
- **`bin/lab.rs` does not have this yet** and the lab is where the win is
  clearest. It has its own draw path and its own HUD (~35 call sites in
  `src/lab/ui.rs`), so it is a real piece of work rather than a switch — and
  `src/lab/ui.rs` is another lane's file, which is why this lane did not take
  it. That is the obvious follow-up.
- **`src/app.rs` is the contested file here, not `src/render.rs`** — 85 HUD call
  sites were rewritten to go through `render::Hud`. Mechanical, but anyone
  holding a large `app.rs` diff will feel it. Landed the same day it was written
  for that reason.
- **`PIXEL_PHYSICS_ZOOM_OUT=<rung>,<budget>`** starts the app zoomed out. Any
  later session needing to screenshot the real app at zoom-out wants this; the
  sandbox has no keyboard and without it the check has to go through a harness
  that reimplements the app.

## The one that cost the afternoon

The feature was correct in twelve guards and **panicked on the first frame of
the real app** — `index out of bounds: the len is 655360 but the index is
700428`, in a HUD blend a hundred lines from its cause. `main.rs` sized the
buffer from `App::viewport()` and `App::draw` pushed the budget at the renderer
afterwards, so the two disagreed for one frame after any change. Every test
passed through it because every test applied the budget before drawing.

Two sources of truth for one derived quantity, and the tests all happened to
observe them in the agreeing order. Fixed by deriving `viewport()` from the
authoritative pair so there is no ordering at all. Recorded here because the
*shape* recurs: a value pushed from an owner into a consumer, read back from the
consumer by a third party.


---

## Round 33: the lab half

**Why it existed.** #392 flipping the sandbox default to x2 made the two games
*diverge* rather than agree: `src/lab/mod.rs` re-exports the sandbox's
`WIDTH`/`HEIGHT` and drives the same `Renderer`, so the lab had the identical
15-in-16 discard and none of the fix — and the lab is where the owner picked the
**finest** setting (*"C is best"* = x4, decoded through `blind_was: [1, 0, 2]`;
I re-checked the decode rather than taking it on trust, and it holds).

**Shipped:** `Lab::pixel_budget`, default **x4** — his pick, not overridden,
and **the same default as the sandbox**: consistency between the two games is a
stated requirement, not a nice-to-have (#392 defaults `App::pixel_budget` to 4
to match).
`+` cycles it, the window caps it, `render::Hud` (from #389) keeps the bar at
its logical size, and `Lab::to_logical` converts the cursor once at the window
boundary.

### The three things the next person should know

1. **The shipped 512x320 bed cannot zoom out at all**, so the budget buys
   nothing and costs nothing there — 1.00x achieved at every budget, measured.
   Anyone reading "the lab defaults to x4" and expecting a cost on the default
   bed is wrong. The feature is for beds bigger than the viewport, and beds up
   to **2048x1280** show whole at one cell per pixel (`MAX_BOX` is 4096, so the
   top of the range does not — the round-32 brief's claim needed that
   qualification).

2. **The speed dial does not amortise the render — it competes with it**, which
   is the opposite of the round-33 brief's reasoning and the opposite of what I
   expected. `TimeControl` gives each pass one wall-clock budget and the render
   comes out of it: at dial 1 the bigger buffer is **free (1.00x)**, at
   fast-forward it costs **0.45-0.65x** of the achieved rate. It costs most
   exactly where it was expected to cost least. `examples/labzoom_cost.rs`, and
   **`achieved` is the unit** — frame milliseconds answer the wrong question
   about a box you run fast.

3. **`visible_span`'s contract changed**: it takes the **logical** viewport and
   multiplies by the *ladder* stride, where it used to take the buffer and use
   the sampling stride. Same number whenever the two agree; still right when
   they do not. Every camera function (`follow`, `pan`, `set_camera`,
   `zoom_within`) now takes the logical viewport and touches `pixel_scale`
   nowhere. If you are adding a camera call, pass `(WIDTH, HEIGHT)`, not the
   buffer.

### The failure worth carrying, because it is the same one twice

Round 32's panic and round 33's silent bug are **one shape**: two sources of
truth for a derived quantity, observed by different parties in different orders.
Round 32, `viewport()` read the renderer's pushed copy while `draw` pushed it
afterwards — it panicked. Round 33, `zoom_within` derived the scale from the
renderer while its viewport came from the caller — it did not panic, it clamped
the widest zoom-out to rung 2, and **the tell was a number that could not be
true: the bigger buffer measured *faster*.** It was not faster; it was showing
half as much world.

Both fixes were the same: delete the ambiguity rather than order the operations.

**And its guard had to be written twice.** The obvious one — assert on
`Renderer` that the rung is budget-independent — **passed with the fault
reinstated**, because the fault needs a caller whose viewport is derived from
the budget. Blind, so replaced rather than widened, per `CLAUDE.md`. The one
that works is at `Lab` level and pushes the budget at the renderer first,
because that is the order the app runs in.

### The mistake in the cards, which is mine and worth not repeating

The round-32 cards offered **different menus** — the lab card was x1/x2/x4, the
real-game card was two panes, x1 against x2 — so **x4 was never on offer in the
game**. That made the verdicts look like a preference for different settings in
the two games, and the owner corrected it: *"I am not sure what questions that I
answered that suggests zoom should be different between the games, but that
doesn't seem like what I want."* Given the full range he picked the finest in
both.

**A comparison can only return a verdict about the options it contains.** That
is the review-queue form of *ask what your number counts*, and the failure mode
is specific: a difference between two cards' *constructions* reads exactly like
a difference in the thing being judged. If two cards are going to be compared
against each other, give them the same menu.

### The bar verdict, and the fact hiding in its decode

Card `20260913T170211133Z-af41c9`: *"I think this is fine. If there are other
better looking options, we can explore them"* — the block-drawn glyphs are
**accepted**. The second clause is an opening, not a task; nobody should spend a
lane on typography off it.

**The fact worth carrying is in the decode, not the prose.** `blind_was: [1, 0]`
puts the **x2** arm in front of him as pane A, because the card was rendered at
the 1024x640 default window where `pixel_scale_cap` resolves a request of x4
down to x2. **So x4 in the lab is unseen and, at that window, unreachable.**
Do not report "the lab ships at x4" as though his eye has backed it: the default
*asks* for x4, the cap gives x2 there, and x4 arrives if the window grows.

That also qualifies the consistency claim in both directions, and the honest
version is the one to state: both games **request** the same budget; what either
**displays** depends on its window.

**Acted on:** a refused request now says so on screen — a line under the lab's
clock naming the request, what it resolved to, and whether the window or the
zoom rung refused it, drawn only when the request is not met. Not on the bar:
row 0 measures 508 of 508 pixels at its tightest spacing.

Its guard is worth reading before writing another like it. The first version
counted opaque pixels in the notice's band and compared x4 against x2 — which
measured the **buffer size**, because the world is drawn behind the notice so
every pixel in the band is opaque either way. It read 14,336 against 57,344 and
said nothing about the notice. The version that works is a **paired diff at one
buffer size**: both arms at scale 2, one meeting the request and one refused,
asserting the notice band differs and everything else is byte-identical.

### Open
- **`src/lab/ui.rs` is the contested file** here — ~175 call sites went through a
  mechanical `hc` parameter. `claude/lab-chronicle-log-ring` also touches it
  (only `chronicle_text` and tests, so the overlap is small). Land promptly.
