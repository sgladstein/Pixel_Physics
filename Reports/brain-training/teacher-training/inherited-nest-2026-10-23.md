# Inherited nests sank into the soil: cause and fix (2026-10-23)

> **Landed 2026-10-24** by the report sweep (thread *Land unlanded thread reports*). The "Inherited nest soil loss"
> thread shipped its code (PR #834, merged to `main` as c08e7f1 on 2026-10-23 17:57Z) but its findings lived only in
> project memory and the shared folder `inherited-nest/`. This is the summary, from the thread's dated notes, **not
> re-measured**. Pictures, patches and the 192-map sweep tables are in
> [`inherited-nest-2026-10-23/`](inherited-nest-2026-10-23/).

**What it is for.** In colony selection a round can start in the previous round's dug nest (`inherit_nest=1`), so a
colony keeps building one nest over many rounds instead of starting from a fresh bed each time. The Teacher lane
found that inherited rounds lost their nests: a flat scraped band appeared under the surface beside the shaft within
10-50 frames, with no ant cutting. Every inherited batch from colsel3 to colsel23 started each round with this, and
colsel17 showed training **without** inheritance gave far bigger nests (794 vs 143).

## Cause

`examples/inrun.rs`'s nest-map reader carved the kept nest cells EMPTY into the fresh world's **loose** soil. Loose
soil is a powder: only packed soil spans a hole. The holes refilled from above within 10 frames, and the void
surfaced as the scraped band. Copying the old map's lining could not help, because the crater rule keeps only the
roofed edges of crater-topped rooms, so their inner wall is fresh loose soil anyway.

## Fix (PR #834, all on `main`)

- **Line every kept cell** (`World::line_dug_cell`, which packs the neighbours) and carve the kept `?` cells too
  (ants and food used to be left as soil plugs). Over 192 end-of-round maps (`nestmaps-1022`) at 3,000 frames:
  refilled cells, median **74 -> 0**; the top-6-row band went 22 -> 66 before, 22 -> 22 after. `INHERIT_LINE=0` /
  `INHERIT_PLUG=1` give the old carve.
- **Join rooms to the door** (`inherit_link`). Most inherited rooms arrived sealed (median 115 cells carved, 30
  joined: the link ran through the uninherited crater). `inherit_link` keeps the cheapest 2-wide lined run of old
  open cells to the door, never the surface row, with the top 6 rows costing 4. Joined/carved went 0.31 -> 1.00 and
  166/192 maps were fully joined.
- **Minimum pocket: use `inherit_link=20`.** Linking every pocket drew made-up tunnel trees on crater-bowl maps (the
  owner asked of one, "is this ant dug?"). At 1, 49 of 192 maps had links bigger than the pockets they joined; at 20,
  none, 86% of pocket cells are kept and 191/192 maps are fully joined. At 50 real rooms start to be dropped.

## Side finding, not changed

The keep flood and `nestcount.py` seed from every open door-column cell, so a buried pocket that crosses the door
column counts as joined.

## What happened next

colsel23 ran inheritance with the fix (`inherit_nest=1 inherit_link=20`): it works mechanically, but nests carried
round to round became a wide shallow food-packed layer (nest 102 vs 208 without inheritance). See
[`../inrun-selection/batches-2026-10-22-to-24.md`](../inrun-selection/batches-2026-10-22-to-24.md).

**Testing rule this cost:** any change to what the nest map keeps changes what is inherited. Check the new world's
first frames, not only the meter that reads the map.
