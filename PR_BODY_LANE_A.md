# Held world: bubbles merge, speed is colour, and the lab's look comes across

Round 37, Lane A — items 1–3 of the owner's 2026-09-14 playtest of
`cargo run --release --bin druid`.

## What changes on screen

**Bubbles placed on top of each other now read as one shape.** *"When you place
multiple overlapping, they should merge instead of just looking like
overlapping circles."* The haze already took the strongest circle rather than
the sum — that was right, and it was not the defect. Every disc still drew its
own rim all the way round, including the stretch buried inside a neighbour, so
three bubbles came out as three arcs crossing. The rim now follows the edge of
the **union**.

**Speed is what colour the bubble is, and nothing else.** *"The speed
indication should just be bubble color, not animation speed (it looks bad when
moving fast)."* The shimmer no longer speeds up with the dial; the bubble runs
cold blue at real time and warms — and thickens in tint, not in width —
towards the top.

**The ants wear their colony's colour, as in the lab**, instead of
disappearing into soil at their own body colour.

**The lab's debug overlays are readable here.** They were always drawn; they
were being painted over.

## The mechanisms

### The union, in one line of geometry

To leave the union of the circles you have to leave every circle you are in,
so a point's depth inside the union is the `max` over the discs of its depth
inside each — and the disc attaining that maximum is the one whose arc *is*
the union boundary there, so it settles the tint and the reach too. A point
deep inside any circle therefore has a large maximum, fails the reach test,
and draws nothing, whichever neighbour's rim it lies under. Over a lone circle
a `max` is the identity, so the single-bubble look the owner approved is
untouched.

The per-disc inner cull had to go: it skipped exactly the pixel the rule needs
to see.

| | hazed pixels |
|---|---|
| three interpenetrating circles, before | 4,419 |
| …after | 2,450 |
| one circle alone, before → after | 1,394 → 1,404 |

**Cost.** Measured at matched repaint cadence on an identical **7,970
px/frame** work set (`druid_aura cost=1 overlap=3 speed=1 perstep=0`): 0.613 →
0.487 ms mean whole-frame. Two binaries on an unpinned box, so that reads as
*no measurable cost*, not as a speed-up. The pixel counter is the
load-independent half and it is identical in both arms.

### Speed, moved into the colour

Both phase clocks are the renderer's own draw counter now — the standing arm's
used to be `World::frame`, which a held world advances up to eight times per
drawn frame, and that *was* the animation speed being complained about. Hue
runs `AURA_STANDING` → `AURA_FAST`, which is `druid::hud::speed_tint`'s own
ramp, so a player who learned it on the ring reads it unchanged.

`AuraTuning::fast_gain` (1.45) carries density beside the hue, **because hue
alone measured as a mechanism that fires and cannot be seen**: the cold end is
a pale blue against a pale blue sky. Recovered blend fraction x1→x8 is **1.37**
with the density channel and **0.94** without.

`depth_per_step` defaults to `0.0` — a band that thickens with speed stacks to
exactly the flat saturated block this change exists to avoid. Both withdrawn
channels are in `Reports/dead-ends.md` (`rendering:001`) with the condition
each rejection depends on.

Side effect worth having: the phase off the world clock **halved the repaint
rate at speed**, 12,420 → 6,365 px recomputed/frame on three circles at x4.

### The overlays were being painted over

`cell_colour` is shared by all three games and nothing ever skipped the
overlays. But `apply_held_look` is a *full replace* onto one hue and ran after
them, so on a held world — nearly the whole map — every ramp arrived at the
screen with only its luminance left. A debug channel whose reading depends on
whether time is running there is precisely the readout that is a function of
the thing it debugs.

The held look now yields on any cell a debug channel painted, tested by
comparing four bytes against what the chain started with. **Not** by keying on
whether an overlay is switched on: a field overlay covers the screen and an
organism overlay covers a few hundred cells, so the mode test would drop the
held look off the whole world to make a handful of ant cells readable, and
where time has stopped is this game's premise. Heat channel, on a world that
is one temperature throughout: **43,456 cells wrong → 0**.

**There is still no key or menu row to turn them on.** `cycle_field_overlay`,
`cycle_organism_overlay` and `cycle_creature_colour` are public and ready;
`menu.rs` and `bin/druid.rs` belong to another lane this round and the row is
theirs to add. Until then `examples/druid_garden.rs overlay=… colour=…`
reaches them.

## Guards

Five faults injected, five go red; the tree is green at 129 render guards.
**Two of the new guards were blind on the first attempt** and both are
recorded in the source rather than quietly fixed:

- the strength assertion summed the raw channel difference, which the hue
  dominates (blue→near-white moves red by 105 on its own), so it reported a
  healthy rise with the density channel switched off. It now recovers the
  blend fraction and reads it at a quantile — a *sum* is over the pixels that
  registered a change at all, and that set is itself a function of the hue;
- the bar was then read back off `fast_gain`, so the fault moved the
  expectation with it. Fixed bar of 1.20, in the measured gap between 1.37 and
  0.94.

`a_faster_quickening_hazes_deeper` asserted the channel the owner removed, so
it is replaced rather than left passing. The dirty-rect guard read a single
frame, which was a parity coin toss that happened to land right while the
phase rode `World::frame`; it now reads a window as long as
`AURA_FRAME_QUANTUM`.

New: `overlapping_quickenings_draw_the_outline_of_their_union`,
`one_quickening_on_its_own_is_untouched_by_the_merge`,
`a_debug_overlay_is_readable_on_held_ground`.

## Instruments

`druid_aura` gains `overlap=N`, `gain=`, and the disc counts on its footprint
line. The whole previous round measured **one** circle, so the defect the
owner reported was invisible to every card the aura had ever been judged on.
Read the disc count *inside* the loop — the control draw's `alpha=0` clears
the disc list, and the first version reported `0 discs` beside a 5,538-pixel
footprint.

`druid_garden` gains `colour=`, `overlay=`, `crop=` and `zoom=`, so both arms
of a look comparison come out of one binary rather than two builds. An ant is
1–2 cells in a 2560-wide world: the same change is 25 pixels whole-frame and
400 cropped and magnified.

## Judged by eye

Four cards on board `druid`:
`20260914T195923263Z-cfdfa0` (the merge),
`20260914T195956622Z-357e6d` (is speed legible from colour?),
`20260914T200008969Z-75f7c5` (the animal colour),
`20260914T200025758Z-8c5504` (the overlays).

The second carries a live question: if the ramp is too weak the lever is
`fast_gain` or the hue endpoints, **not** `depth_per_step`.

## Gates

`cargo clippy --all-targets --release --locked -- -D warnings` clean ·
`cargo test --release` · `bash scripts/docscheck.sh` clean ·
`python3 scripts/deadendindex.py --touching` 0 · `origin/main` merged.

**Note for whoever integrates:** `aura_disc_count()` landed on `main` via
PR #430 while this branch was running, and git merged both copies into one
file. Resolved by keeping #430's and deleting this branch's.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01PTgbJ31QFWqWp3yEXpmzKh
