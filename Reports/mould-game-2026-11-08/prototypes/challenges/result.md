# Challenges: the first three dishes and the dish picker

Built for real on branch `challenges` (worktree
`/home/user/Pixel_Physics/.claude/worktrees/agent-a3b4054aec7d193f0`, not
pushed). The mould game now has something to beat: three hand-made dishes,
each asking one question, a score card that says *why* you missed a star, and
a picker (M) that unlocks the next dish on a star from the one before.

## Does each dish meet the ship bar?

`cargo run --release --example mouldprobe -- level=all`, seeds 1-5 (stars per
seed; the layout is fixed, the seed moves the mould and the random hand).
Bar: do nothing 0, random at most 1, intended 3.

| dish | do nothing | random | simple plan | fresh player | intended hand | bar |
|---|---|---|---|---|---|---|
| 1 Behind the Wall | 0 0 0 0 0 | 0 0 0 0 1 | 0 0 0 0 0 | 0 0 0 0 0 | 3 3 3 3 3 (joined 0:10-0:11) | met |
| 2 Beat the Ruler | 0 0 0 0 0 | 0 0 0 0 0 | 0 0 0 0 0 | 1 0 0 0 0 | 3 3 3 3 3 (0.87-0.91x ruler) | met |
| 3 Running Dry | 0 0 0 0 0 | 0 0 0 0 0 | 2 0 1 0 0 | 0 0 0 0 0 | 3 3 3 3 3 (354-411 alive) | met |

"Simple plan" and "fresh player" are the free dish's existing scripted hands,
run on the levels for interest; they are not part of the bar. Beat the Ruler
on 8 seeds: intended 0.87-0.92x, all 3 stars.

## The dishes

- **Behind the Wall** (90 s, lamp + oats, 5 oats back every 15 s). Food
  behind a pre-drawn salt wall with one gap at the bottom; scent does not
  cross salt. Stars: joined at all / by 1:00 / by 0:45.
  Why-lines: "SCENT CAN'T CROSS SALT: LEAD IT TO THE GAP", "PAST THE WALL,
  BUT OUT OF REACH: LAY OATS".
- **Beat the Ruler** (100 s, lamp + oats, no snips). Three foods in a
  triangle; the ruler (two short sides) is drawn dotted. Scored on the
  shortest join through the tube against the ruler. Stars: all joined /
  under 0.98x / under 0.95x. After the bell the best possible join (0.87x) is
  drawn faintly. Why-line e.g. "UNDER 0.95X FOR 3: MEET NEARER THE MIDDLE".
- **Running Dry** (120 s, lamp + oats, 5 back every 10 s). Small piles that
  last ~35 s; a new pile appears every 20 s, further on. Stars: 100 / 200 /
  300 mould alive at the bell. Why-lines "IT KEPT MOVING HOUSE, AND IT LIVED",
  "THE FOOD RAN OUT; NOTHING HEADED FOR MORE".
- **The why-line** now shows on every score card, free dishes included
  ("A BRIDGE WHERE NO FOOD WAS. BEST 0.87X", "THE STORM CUT OFF 3 PILES: A
  LOOP HOLDS").
- **Dish picker (M)**: the lesson, the three dishes, FREE DISH (N cycles the
  seeds). A dish opens on 1+ star of the one before; the lesson counts once
  finished. Best stars saved to `mould-progress.txt` beside the executable; a
  missing or garbled file reads as fresh (tested). The sim pauses while the
  picker is open. Keys: 1-5, arrows, Enter/Space, M/Esc; mouse click.
- **Briefing card per dish** (H): its goal sentence and that dish's tools.
  The tools line hides tools the dish does not give.

## How to run

```
cargo run --release --bin mould              # the lesson, as before
cargo run --release --bin mould -- d1        # a dish directly (d1, d2, d3); M for the picker
cargo run --release --bin mould -- 7         # free seeded dish 7, as before
cargo run --release --example mouldprobe -- level=all              # the table above
cargo run --release --example mouldprobe -- level=2 look=out/ gif=out/
cargo run --release --example mouldprobe -- menu=out/              # the picker, as drawn
cargo run --release --example mouldprobe -- level=2 stray_reach=inf  # the control (below)
```

## Pictures, with their counts

| file | what | counts |
|---|---|---|
| `sheet-dishes.png` | per dish: briefing card, start, intended hand's end | wall seed 1: joined 0:11, 3 stars, oats 5, joins 6. Ruler seed 1: 0.88x, 3 stars, oats 7, joins 11, mould 944. Dry seed 1: 400 alive at bell, 3 stars, oats 12, joins 19 |
| `dish1-wall-intended.gif` | Behind the Wall, intended hand, seed 1 (568 KB) | joined 0:11, oats 5, joins 6 |
| `dish2-ruler-intended.gif` | Beat the Ruler, intended hand, seed 1 (2.7 MB, every 60th step) | 0.88x, oats 7, joins 11 |
| `dish3-dry-intended.gif` | Running Dry, intended hand, seed 1 (2.4 MB) | 400 alive, oats 12, joins 19 |
| `sheet-ruler-steps.png` | Beat the Ruler at 5, 10, 15, 30, 60, 95 s | as the GIF |
| `sheet-dry-walk.png` | Running Dry at 15, 30, 50, 70, 90, 115 s: the mould moving house | as the GIF |
| `free-dish-why-line.png` | a free dish's score card with its why-line | plan hand seed 1: 2 stars, joined 6/6, 3 left after the storm, 1.12x, joins 15 |
| `menu-fresh.png` | the picker, nothing won (dishes 2-3 locked) | progress 0/0/0/0 |
| `menu-some.png` | the picker with some stars | lesson done, wall 3, ruler 1 |
| `window-ruler-brief.png` | the real window (xvfb), Beat the Ruler's briefing card | - |

## Where I departed from the design

- **No storms on any of the three.** A Steiner join is a tree, which the
  storm always splits, so Beat the Ruler would score the storm, not the
  shape; Behind the Wall is a single route; on Running Dry the only tube
  worth cutting is the finger to the next food. Storm Season (dish 5) is
  where it belongs.
- **Behind the Wall layout moved closer** (home (62,80), food (194,80), not
  (40,80)/(210,80)). At the brainstorm's spacing the way round was 207 cells
  and an oat relay (each oat drains ~3 s once eaten) died after carrying the
  front 98 cells; the intended hand never arrived. Now 157 cells.
- **Beat the Ruler scores the shortest join through the tube, not tube
  area.** Area is set by food joined, not shape: on the first run doing
  nothing read 1.40x and a good meeting point 1.39-1.75x. `Dish::best_join`
  is the least, over tube cells, of the summed tube distance to the three
  piles. Bars 0.98 / 0.95 (best seen 0.87, so headroom both ways).
- **Running Dry bars are 100 / 200 / 300 alive, not 300 / 600 / 900.** The
  bell always finds the mould between piles; the intended hand ends 354-411.
- **A model change, hand-made dishes only: `Params::stray_reach`.** Unfed
  walkers wander off the leash, and any walker on food became a source of
  feeding, so strays colonised far food on their own. Control
  (`stray_reach=inf`, the old rule) on Beat the Ruler, seeds 1-3: do nothing
  1, 2, 2 stars; random 3, 3, 1. On levels a stray can now start eating only
  within 24 cells of fed mould. Free dishes keep the old rule (infinite) and
  are unchanged. Shutting strays out entirely was tried first: an oat just
  past the leash could never be eaten and every relay died.

## Frame cost

Steps + draw at scale 4, intended hand, one process alone: Behind the Wall
mean 3.32 ms / worst 12.43 ms (664 frames); Beat the Ruler 2.52 / 9.11;
Running Dry 2.36-2.47 / 7.26-10.46. Base (free dish, before) 2.6 / 17.

## What did not work, and what is not fixed

- Relays fell apart until the intended hand **saved up oats before starting
  one** and laid them in one go; a player has to learn this too. On Beat the
  Ruler the junction slides to a side if left alone (one seed in five went
  0.87x -> 0.99x by the bell), so the hand **keeps feeding the meeting
  point**. Both are worth a line in the briefings or a later dish.
- Early route versions were fooled by a lone walker sitting on an oat
  (now: the front is home's fed tube only) and wasted oats on a pile.
- **Not fixed, noted:** a dish whose mould has all died steps about 3x slower
  than a live one (idle Running Dry: 27 s for 120 s of dish against 9 s with
  the intended hand), probably subnormal floats in the fading trail. About
  5 ms a frame, inside budget; unverified.

## Next steps

- Owner playtest of all three dishes by hand: the scripted hand proves each
  is winnable, not that it is fun or that the briefing is enough to find the
  route.
- The remaining brainstorm dishes (2, 4, 5 Storm Season) fit `Level` as it
  is; the struct has room for per-dish storms.
- Fold the picker's drawing into the interface rework (`look.rs`) when it
  lands; the additions here are self-contained functions at the end of
  `src/mould/render.rs` (`level_status`, `ruler_overlay`, `level_brief`,
  `menu`).
- Check the subnormal suspicion (flush the trail to zero below a floor) if
  the dead-dish cost ever matters.

## Checks

`cargo test --release --lib mould`: 15 passed, 2 ignored (the heavy
Running Dry do-nothing and the ruler/dry intended-hand runs; with
`--include-ignored` 17 pass in 34 s). Clippy -D warnings clean on `--lib
--bin mould --example mouldprobe --tests`. `bash scripts/docscheck.sh` clean.
