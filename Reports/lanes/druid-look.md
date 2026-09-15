# Lane A — "the look" (held world, 2026-09-14 playtest, items 1–3)

**Branch:** `claude/druid-look`. **Owns:** `src/render.rs`.
**Also touched:** `src/druid/mod.rs` (one line in `Druid::new`'s renderer
block, nowhere near the trail functions), `examples/druid_aura.rs`,
`examples/druid_garden.rs`, `README.md`, `Reports/dead-ends.md`,
`Reports/instruments.md`.

## For the coordinator, first

**Item 3 needs a key or a menu row that is not mine to bind.** The switches
are public on the renderer and ready to call — there is nothing left to build:

| what | call |
|---|---|
| debug field channels | `Renderer::cycle_field_overlay()` (or set `renderer.field_overlay`) |
| per-organism channels | `Renderer::cycle_organism_overlay()` |
| animal colour mode | `Renderer::cycle_creature_colour()` |

All three were already `pub` before this lane; `field_overlay` and
`organism_overlay` are public fields too. The lab binds the third as its
`ANIMAL COLOUR` row (`lab::ui`, key `H`). **Lane C owns `menu.rs` and
`bin/druid.rs`** — this is their row to add, and until they do, the only way
in is `examples/druid_garden.rs overlay=… colour=…`.

**`aura_disc_count()` collided.** PR #430 landed it on `main` while this lane
was running and git merged both copies into one file — two identical `pub fn`s
that do not compile. Resolved by keeping #430's and deleting mine. Worth
knowing if another lane was told to "add it yourself if it is missing".

## What landed

**1. Overlapping bubbles merge.** The haze already did "the strongest circle
wins rather than the sum", which is right and was **not** the defect — every
disc still drew its own rim *through its neighbour's interior*, so three
bubbles read as three arcs crossing. The rim now follows the edge of the
**union**: leaving the union means leaving every circle you are in, so a
point's depth is the `max` over the discs, and the disc attaining that maximum
is the one whose arc *is* the boundary there — which settles the tint and the
reach as well. Deep inside any circle the maximum is large, the reach test
fails, nothing draws.

The per-disc `inner2` cull had to go: it skipped exactly the pixel the rule
needs to see. **Cost, at matched repaint cadence on an identical 7,970
px/frame work set** (`druid_aura cost=1 overlap=3 speed=1 perstep=0`):
0.613 → 0.487 ms mean whole-frame. Two binaries on an unpinned box, so read
that as *no measurable cost*, not as a win.

| | hazed pixels |
|---|---|
| three interpenetrating circles, before | 4,419 |
| …after | 2,450 |
| one circle alone, before → after | 1,394 → 1,404 (a no-op, as the geometry requires) |

**2. Speed is the colour.** Both phase clocks are the renderer's own draw
counter now, so the shimmer runs at one rate whatever the dial says. Hue runs
`AURA_STANDING` → `AURA_FAST` on `hud::speed_tint`'s own ramp; `fast_gain`
(1.45) carries density beside it, **because hue alone measured as a mechanism
that fires and cannot be seen** — the cold end is a pale blue against a pale
blue sky. Recovered blend fraction x1→x8: **1.37 with the density channel,
0.94 without.**

`depth_per_step` now defaults to `0.0`. Both withdrawn channels are
`Reports/dead-ends.md` `rendering:001`. Side effect worth having: the phase
off the world clock **halved the repaint rate at speed** — 12,420 → 6,365 px
recomputed/frame on three circles at x4.

**3a. Animals wear colony colour.** One line in `Druid::new`, matching the lab
since 2026-09-06. The outdoor sandbox keeps `Off`.

**3b. The overlays.** They were always routed — `cell_colour` is shared by all
three games. `apply_held_look` is a *full replace* onto one hue and ran
**after** them, so on a held world (nearly the whole map) every ramp arrived
with only its luminance left: a readout whose answer depended on whether time
was running there. It now yields on any cell a debug channel painted, tested
by four bytes against what the chain started with — **not** by keying on
whether an overlay is on, which would drop the held look off the whole world
to make a few hundred ant cells readable. Heat channel: **43,456 cells wrong
→ 0**.

## Guards, and the two that were blind first

Five faults injected, five go red; the clean tree is green (129 render guards).
Two did not go red on the first attempt and both are recorded in the source:

- **The strength assertion summed the raw channel difference.** That number is
  dominated by the hue — blue→near-white moves red by 105 on its own — so it
  reported a healthy rise with `fast_gain` at 1.0, the channel switched off.
  Now it recovers the blend fraction and reads it at a **quantile**, because a
  *sum* is over the pixels that registered a change at all and that set is
  itself a function of the hue.
- **The bar was read back off `fast_gain`.** So the fault moved the
  expectation with it and the guard agreed with whatever the dial said. Fixed
  bar of 1.20, in the measured gap between 1.37 and 0.94.

`a_faster_quickening_hazes_deeper` asserted the channel the owner removed, so
it is **replaced**, not left passing. `the_quickening_haze_keeps_the_dirty_
rect_skip` read one frame, which was a parity coin toss that happened to land
right while the phase rode `World::frame`; it now reads a window as long as
`AURA_FRAME_QUANTUM`.

## Instruments

`druid_aura` gains `overlap=N` (a cluster that genuinely interpenetrates),
`gain=`, and the disc counts on its footprint line. **Read the disc count
inside the loop** — the control draw's `alpha=0` clears the disc list, and the
first version reported `0 discs` beside a 5,538-pixel footprint.

`druid_garden` gains `colour=`, `overlay=`, `crop=` and `zoom=`, so both arms
of a look comparison come out of one binary. An ant is 1–2 cells: the same
change is 25 pixels whole-frame and 400 cropped and magnified.

## Cards posted (board `druid`)

| card | asks |
|---|---|
| `20260914T195923263Z-cfdfa0` | does the cluster read as one shape |
| `20260914T195956622Z-357e6d` | is speed legible from colour, or does the ramp need widening |
| `20260914T200008969Z-75f7c5` | is colony colour the animal colour he meant |
| `20260914T200025758Z-8c5504` | does the overlay read the same everywhere now |

**The second one is the one with a live question in it.** If he says the
colour ramp is too weak, the lever is `fast_gain` or the hue endpoints —
**not** `depth_per_step`, which is what he removed.

**Corroboration worth knowing before reading his answer.** On the previous
aura card (`20260914T053549208Z-49137a`) he wrote *"Looks much better static
in animation, I am worried that it is too fast/rapid, but I will playtest"* —
the worry came first, and the playtest is what turned it into item 1. This
lane removed only the part that **accelerated with the dial**; the *base*
shimmer rate is untouched, so it still runs at the speed he called much
better. The speed card was amended (unanswered) to say so and to name
`AuraTuning::period` as the dial if he still finds it too rapid.

## Gates

`cargo clippy --all-targets --release --locked -- -D warnings` clean ·
`cargo test --release` · `scripts/docscheck.sh` clean ·
`deadendindex.py --touching` 0 · merged `origin/main` (36 behind at the time).

## The owner's verdicts, and what is left

Four of the five cards came back on 2026-09-14. **Two were answered by the
choice alone, with no comment** — worth saying, because an empty `comment` in
`review.py inbox` reads exactly like an unanswered card and neither of these
was:

| card | verdict |
|---|---|
| the merge (item 1a) | **after**, rated **5** |
| animal colour (item 2) | **after** |

Neither was blind, so those labels are literal and need no `blind_was`
translation. Items 1a and 2 are accepted as they stand.

**Speed colour — *"This is the idea, but make the color change a little more
visible. You are close."*** Acted on in this branch: `AURA_FAST` warmed from
`[255, 250, 225]` to `[255, 243, 185]` and `fast_gain` raised 1.45 → 2.0. A x8
bubble now carries **1.72x** the tint of a x1 one, against 1.37x before.
Re-posted as card `20260914T223428380Z-a1c555`.

**The ceiling on that lever is worth knowing before anyone pushes it again.**
`AURA_CARRIED` is `[255, 214, 140]`. A hot end much past `[255, 243, 185]`
stops reading as *a fast circle* and starts reading as *his circle* — one
distinction bought with another. If more heat is wanted, the carried colour
has to move first. `fast_gain` has no such ceiling and is the lever to reach
for.

**Overlays — *"'there is no key or menu row to turn these on yet.' - this was
the main issue, but the after does look better."*** The rendering half is
accepted; **the switch is what he actually wants, and it is not in this
lane.** Re-derived from the branch list rather than from the brief:
`origin/claude/druid-founding` is unlanded and is editing both
`src/druid/menu.rs` and `src/bin/druid.rs` right now, so a row added here
would land in a file someone else has open. **Coordinator: this is one menu
row, and it is the difference between item 3 being done and not.** The three
`cycle_*` calls are in the table at the top of this note.

## Status for the coordinator

**PR [#437](https://github.com/sgladstein/Pixel_Physics/pull/437) — open,
`mergeable_state: clean`, all 9 CI checks green, no review comments.** You
opened it from this branch before my own `create_pull_request` landed, so
there is one PR and not two; I updated its body to the final version, because
the one you picked up predated the commit carrying the measured gate numbers.

**CI-green head: `3451869a94628e59e9fbdd1f8d15485295702740`** — every check on
run `34897262000` succeeded, including `cargo test (release)`, `cargo test
(debug, compiles the debug_assert guards)`, `structural acceptance cases`,
`worldgen pass interference`, `ascii`, `clippy`, `fmt --check` and `docscheck`.
That is the SHA of the commit *before* this paragraph, for the obvious reason;
this note's own commit is documentation only and re-runs the same suite.

**The merge is yours, not mine** — lane, not independent session. Nothing here
is waiting on me.
