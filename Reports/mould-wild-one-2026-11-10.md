# The Wild One: a second mould that steals and fights (2026-11-10)

**Status: built, its own dish (the fifth, THE WILD ONE); waits on the owner's play.**
Game: the mould probe (`cargo run --release --bin mould`).
Pictures: https://claude.ai/artifact/QDgynHJg8dPRKuhZf54sf9 (seeds 1 and 4, doing
nothing against the probe's `warden` hand, at 30, 75, 120 and 145 s, with the
counts beside them).

Step 3 of the order the owner agreed after the independent review
([mould-review-2026-11-10.md](mould-review-2026-11-10.md)): (1) the endless
dish and (2) water and sleep are in
[mould-keep-alive-2026-11-10.md](mould-keep-alive-2026-11-10.md). This is (3):
a wild second mould in another colour, under the same rules, nobody steering,
that steals food from the same piles and then fights where the two meet. The
owner's bar: the player's hand must change which mould ends bigger, paired
over seeds; quote the frame cost.

## What it is

- **The wild mould is a whole second `Dish`** (`Dish::rival`), with
  `wild = true`, no level, no storm, no tools and its own rng. Same `Params`,
  so the same walkers, trail, feeding, dividing and starving. `Dish::step` now
  wraps the old body (`step_own`): it copies the lamp, the ground and the
  water to the rival, applies this step's snips to it too, kills its walkers
  standing off agar (salt), marks each mould's cells as occupied for the
  other (`hold_foe_cells`) so they never share a cell, steps both, runs the
  fights and syncs the piles.
- **Stealing is the shared table.** A pile feeds at most `Params::table`
  walkers in total across both moulds, split by head count (`Pile::bite`,
  `Pile::foe_eaters`). The first version gave each mould its own table and
  every hand ended at about half the dish: no tool mattered, because there
  was nothing to contest. With the shared table, whoever has more walkers on
  a pile takes more of it.
- **Each mould's trail repels the other** (`Params::foe_repel`, 1.0 for
  yours, `WILD_REPEL` 0.6 for the wild one). Without fights and with repel
  0.3, the wild one took 90-99% of the dish for every hand, tools included.
- **Fights** (`Params::fight`, 0.02): each step a walker dies with chance
  `fight x (foe neighbours / 8)`, halved on fed tube and doubled off it, so a
  supplied front holds and a stray loses. Counted as `Kill::Fight` /
  `Counts::fought`. About 2,700 walkers a side die fighting in a run when
  nobody plays; 1,300-1,800 under the tool hand.
- **The dish**: seven piles, the wild one starts on the right one
  (`WILD_START` 600 walkers against your `INOCULUM` 400), oats 4 (one back
  per 15 s), salt 1, snips 3, 150 s at 22.5 degrees. Stars by your walkers
  over its walkers at the bell: 1.5x one, 3x two, none of it left three
  (`Win::Rival`). It is open from the start, like Keep It Alive.
- **Drawing**: `render::wild` lays a violet ramp over its tube with pale
  walker dots. A first pass; the looks thread was told so it can redo it.

## Numbers

Your share = your walkers / both moulds' walkers at the bell. Paired seeds,
sign tests, three comparisons in the main table so the bar is p < 0.017.
Scratch census: `mouldprobe level=5 players=idle,random,warden`.

| Hand (16 seeds) | Stars | Your share | Ended bigger |
|---|---|---|---|
| doing nothing | 0 on all 16 | 0.39-0.58 | 9 |
| random | 0 on 14, 1 on 2 | 0.41-0.64 | 12 |
| warden (lamp, salt, snip, oats) | 1 on 2, 2 on 10, 3 on 4 | 0.61-1.00 | 16 |

Warden beats doing nothing 16-0 and random 16-0 on share (p < 0.0001 each);
doing nothing against random 4-12, p 0.08, not significant.

One tool at a time (`WARDEN=lamp` etc.), 8 seeds, against doing nothing:

| Only | Stars | Share | vs doing nothing |
|---|---|---|---|
| lamp | 0-2 | 0.55-0.82 | 8-0, p 0.008 |
| salt wall | 1 on all 8 | 0.64-0.67 | 8-0, p 0.008 |
| oats | 0-1 | 0.50-0.67 | 6-2, p 0.29 |
| snip | 0 on all 8 | 0.44-0.59 | 5-3, p 0.73 |

The lamp and the wall each win alone; only all four reach 2 and 3 stars. The
snip as the warden uses it (every 12 s across the wild tube) does nothing on
its own because the cut heals: the weakest tool here.

**Frame cost** (`RIVALCOST=1 mouldprobe seeds=1,2,3`: the free dish with and
without a rival on alternating frames, 90 s, scale 4): mean 4.3-4.9 ms ->
5.7-6.3 ms, about +1.4 ms a frame; worst frames 15-22 ms either way. The
rival is a full second dish, so it costs about what yours does; it is only
on in this dish.

## Tried and changed

- Per-mould tables: share about 0.5 for every hand, tools irrelevant.
- Shared table, no fights, wild repel 1.0: the warden won 16/16 but doing
  nothing won 9/16 and random 11/16, too generous.
- Wild repel 0.3 or 0.0 without fights: the wild one took 90-99% whatever the
  hand (warden 1 win in 16).
- Fights 0.02 with wild repel 1.0 against 0.6: 0.6 keeps doing nothing at 0
  stars and the warden winning, so it shipped.
- Wild start 400 against 600 made no clear difference to who ended bigger
  before fights; 600 shipped so it starts as the bigger mould.

## Finer tubes: tried, kept at reach 9

The review suggested finer tubes by shortening each walker's sensor reach
(`Params::sensor_offset`, 9). Measured on all five dishes, 8 seeds, with
`mouldprobe level=all sensor_offset=N` and the best hands on dishes 4 and 5;
page https://claude.ai/artifact/5JT3D1sv7CwSRmYfkR6VTW.

| Planned hand, stars per seed | reach 9 | 8 | 7 | 6 |
|---|---|---|---|---|
| Behind the Wall | 33333333 | 33333333 | 13333323 | 20031231 |
| Beat the Ruler | 33333333 | 30330300 | 00000000 | 00000000 |
| Running Dry | 33333333 | 33333333 | 33333333 | 33333033 |

At reach 6, Keep It Alive's lamp+oats+water hand lived 3:04-12:02 (median
3:41) against 4:11-10:42 (median 6:21), shorter on 6 of 8 seeds; The Wild
One's all-tools share fell from 0.67-1.00 to 0.49-0.63. Doing nothing stayed
at 0 stars everywhere. The tubes are only a little thinner (they branch
more), and the mould senses food and oats from less far, so oat trails
that solve each dish stop reaching. Kept at 9; the thin look belongs in the
drawing. Entry in `Reports/dead-ends.md`.

## Next

The independent review of the endless dish, water and sleep, and the wild
one.
