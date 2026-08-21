# Round 6, overnight: what landed, what I ruled, what is still yours

*Written for the owner to read cold on waking. Branch:
`claude/game-world-gen-planning-h12713`. Nothing reached master.*

## The short version

Everything in round 6's plan landed except round-5 task 6 (ceiling grain),
plus one thing that was not in the plan and turned out to matter more than
most of it: **open bug 0c, the grey light blocks**, is fixed.

Four review cards are waiting for you. Two of them (the formation A/B and
the cave-size A/B) are the ones I would most like an eye on, because the
numbers say yes and numbers have been wrong about this three times tonight.

## The night's actual lesson

**Six of the round's problems were the ruler, not the cave.** Not one of
them announced itself as a measurement bug; each arrived as a confident
number that sent work in the wrong direction, and in two cases the code had
already been *shaped* around the bad number before anyone checked it.

| The number | What it said | What was true |
|---|---|---|
| `cave_probe` reachability | round-5 caves 0-8% reachable, "cannot enter at any point" | 33-37%, **one connected walkable region**, traversable end to end. It counted walk-through formations as walls. |
| The `>= 50% reachable` bar | a retune was needed | it was set against the broken number; the retune it justified made the cave *worse* |
| `cave_probe` formation height | A3's taper made formations shorter | it required air on both flanks, which a taper fills. Height was unchanged; A3 had already confined its flare to the bottom fifth of each trunk **to keep this number readable** |
| `vaults detail` base width | formations 3-8 wide | the *drawn* width, not the written one. The cone was gated off and the pass was reporting its own intentions |
| B2's prominence p90 | the residual pass was failing its bar | p90 over every column in the world; reaching 20 needs a tenth of the world to be pinnacle. p99 was 29 against a bar of 20 all along |
| `speleothems_never_bridge` | the fused column was vacuous, "never appeared across 14 systems" | it appears in every system with a chamber. Same both-flanks test, copied |

The pattern in all six: **a rule adopted without asking which object it
evaluates** — a cell, a column, a formation, a world. That question is
already in `CLAUDE.md` twice and I still missed it three times myself.

Every one of these rulers is now fixed and, where possible, sanity-checked
against a case known to be fine before being trusted — round 5's formations
measure base width median 1, p90 1, which is your *"they are all 1 pixel
thick"* in a number.

## What landed

### Caves you can walk into (Track A)

**A1 — rejected.** Its own finding unmade it. Once the reachability ruler
knew about `Material::scenery`, round 5's caves already measured one
connected walkable region at 33-39%; A1's retune reached 96% by dissolving
the network into a single rounded bubble (span across 136 -> 70, contrast
5.4x -> 2.1x). That is your *"looks like a single room instead of a cave
system"* reproduced by the change meant to answer it.

**A2 — bigger caves, with a range.** The envelope was one fixed 181x71 for
every cave in every world; it is now a per-system heavy-tailed draw up to
401x161.

    span across          was always 181    now median 73-157, max 368
    span down            was always 71     now median 57-83,  max 148
    tallest open column  max 56            now max 117

The part that needed measuring: growing the envelope alone does **not** make
a bigger cave. The lattice is a density, so a double-size box gets twice as
many rooms of the same size, and the extra area goes into cracks the gnome
cannot fit in — measured, walkable fell 38% -> 23%. The lattice, the edge
fades and the chamber all scale with the envelope now.

**A3 — fewer, thicker, tapered**, and then a second pass because the taper
was gated off for exactly the formations that carry the silhouette (of ~20
placements probed, only 7 ran the cone, and every trunk over 8 cells long
was in the excluded class).

    formations/system    45.8  ->  15.4      bar 12-20
    base width median    1     ->  3         bar >= 3
    base width p90       1     ->  6
    height p90           19    ->  18        bar >= 10, unchanged

**Phase 0** made formations walk-through and still minable, and gave them
their own materials. That exposed a render bug worth its own note: the
gnome's front/behind hash was keyed on the world *column*, so from width
three up **no formation ever put him wholly on one side** — measured 0% at
widths 3, 5, 8 and 12. Every formation A3 makes would have shipped him
sliced into vertical stripes.

### Rock at the player's scale (Track B, merged earlier)

Residual landforms fill the empty 12-120 cell band. The cleanest statement
of why they were needed is the paired control now in the acceptance test:
**strip the pass and the whole world's 99th-percentile relief at a 15-cell
reach is 3 cells**, against a 14-cell character. Nothing else in the
generator makes rock at your scale.

### The grey light blocks (open bug 0c)

You named these twice, unprompted, on cards that were about something else.
Not a smoothing problem: the light field stores one value per 8x8 cells and
quantises the *emitter* to that grid before diffusion runs, so a two-cell
crystal is a filled 8x8 square before anything smooths it. Each glowing cell
now gets its own short-range falloff, with the coarse field carrying the far
tail. Costs nothing per frame on a settled world.

## What I ruled, so you can overrule it

1. **A1 rejected and reverted**, on the numbers and a paired render.
2. **The `reachable >= 50%` bar retired**, replaced with *walkable regions
   == 1 at p90, largest walkable >= 30% median, contrast >= 5.0x*.
3. **The glow fix defaults to on** rather than being left as a question,
   because you had already called the old one bad twice. `'` switches.
4. **A scratch test dropped** in the merge (`tmp_find_waterline_shot`,
   left behind by a stalled agent, and stale besides).

## Reported, not smoothed

- **Contrast misses its bar in canyon**: 4.83x against 5.0x. The other
  presets make it. That bar came from a population where every cave was the
  same size, so it may be the wrong bar now — but the gap is the honest
  record, not a relabel.
- **Slightly fewer caves**: 5-6 of 16 worlds hold none, against 4-5 before.
  Shrink-to-fit recovered most of what a 200-cell half-width would have cost
  to world-edge rejection, not all of it.
- **Residual coarse blocking** is still faintly visible far out from a large
  halo. Chasing it would cost the point of a *short*-range term.

## Still open, in the order I would take them

1. **Palette dither (open bug 0b).** With the light blocks gone, the static
   in the rock is what is left to look at. You named the two together and
   only one is done.
2. **Round-5 task 6, ceiling grain.** Was blocked behind A1; A1 is settled,
   so it is unblocked and unstarted.
3. **The vertical banding question** on the cave-size card. Broad light and
   dark swathes run the full height of a world render. They are not from
   tonight's lighting work — identical with the new glow term and the old
   one — so they are either the rock's regional variation working as
   intended or something nobody has looked at. I could not tell which, and
   it is a cheap thing for you to settle by eye.
4. **Untouched from the original list**: roofed water / ponds (open bug 0),
   springs placement (needs your flow-budget ruling), presets -> biomes,
   the grain-mode default.

## Housekeeping

- One stranded commit, `e94fca2` on `claude/worldgen-caves-r6`: a task-file
  edit about A1 that this round's ruling supersedes. Deliberately not
  cherry-picked.
- The implementation agent stalled twice mid-gate and, on being woken,
  restored a pre-A3 backup over `passes.rs` **during my measurement run** —
  so a commit of mine claimed A3 and shipped only its tests. Caught by the
  render coming out byte-identical before and after. Two sessions writing
  one worktree is the failure `CLAUDE.md` opens with, and it cost a rebuild
  and a re-measure. I stopped the agent and finished the work myself.
