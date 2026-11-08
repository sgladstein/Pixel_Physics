# Mould probe: herding a slime mould with a lamp (2026-11-07)

**Status: the owner played it (2026-11-08): "the most promise so far ...
the core mechanism is inherently just kind of fun ... we definitely want to
dig deeper", but it needs instruction, a clearer interface and a real
challenge. The lesson and briefing card are in (see *What the owner's play
found*, at the end); the challenge is next.**
Code: `src/mould/`,
`src/bin/mould.rs`; harness `examples/mouldprobe.rs`. Brief:
`/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/prototypes/2-mould.md`
(probe 2 of the 2026-11-07 new-game brainstorm).

**The question:** does herding a slime mould feel alive, or is it watching?
Only a person playing it can answer that. This report holds what the
measurements can say: whether the hand gets an answer fast enough, whether
doing nothing loses, and what a frame costs.

```
cargo run --release --bin mould          # dish 1; `-- 3` for dish 3
cargo run --release --example mouldprobe # the census below
```

## What was built

- **Jones (2010) agent slime mould** on a private 256x160 dish: walkers sense
  trail at three points 9 cells ahead (45 degrees apart), turn toward the
  strongest, step, and deposit. One walker per cell. Trail spreads (3x3
  mean) and fades. Food gives off a separate **scent** field that is summed
  into every sensor but is never drawn or counted as tube.
- **No shared engine file is touched.** `sim::pheromone` was sized against a
  Jones follower, but it is world-sized, `u16`, chunk-bound and stepped on the
  world's interval. A private field kept the probe from being able to move a
  lab, outdoor or held-world result, and lets the Seep probe lift the module
  whole.
- **Economy: food joined = mould alive.** One shared store pays for births
  and upkeep. Each pile feeds at most 40 eaters, so one pile supports about
  300 walkers. Food only reaches walkers through tube: a breadth-first search
  from each pile the mould is on, out 70 cells along tube. Beyond that a
  walker is *unfed*. It cannot divide, slowly dies, and its tube is drawn
  pale, so a cut branch fades over seconds rather than vanishing.
- **Tools:** Lamp (held, unlimited), Oats x5, Salt x1 line, Snip x3.
- **The storm:** at 90 s, one strike across the thickest tube. Read the score
  2 s later, then watch for 6 s more. *(Changed 2026-11-08: aimed at the
  worst cut, warned 5 s ahead, read after the full 8 s. See the end.)*
- **Three stars.** *Reach*: all 5 piles joined. *Thrift*: network at most 1.5x
  the shortest possible, with at least 3 piles joined. *Tough*: nothing lost
  to the storm, with at least 2 piles joined.

## Measurements

**Hand to answer** (`mouldprobe lean=1`, paired against the same dish with
the tool withheld, 5 seeds x 2 moments):

- **Oats to a visible lean** (15 or more walkers within 15 cells, against the
  no-oats twin): **0.45 to 0.85 s** in 9 of 9 placements. Target: 3 s or less.
  The tenth placement fell outside the dish. The first walker reaches the
  oats in 0.30 to 0.55 s.
- **Lamp to recoil** (the mould under the bright core halves against the dark
  twin): **0.03 to 0.08 s** in 10 of 10.
- **Positive control:** the same run with the lamp's three levers zeroed
  (`light_weight=0 light_fade=0 burn=0`) never recoils in 10 s. The guard
  test `the_lamp_clears_the_mould_under_it` has a dark-twin control too.

**Do nothing against a plan** (5 seeds, the census in `mouldprobe`):

| seed | do nothing: stars (joined, after storm) | random hand | simple plan |
|---|---|---|---|
| 1 | 0 (2/5, 1) | 0 (3/5, 2) | 0 (3/5, 2) |
| 2 | 1 (3/5, 2) | 2 (4/5, 4) | 2 (5/5, 4) |
| 3 | 1 (3/5, 2) | 0 (2/5, 1) | 1 (4/5, 2) |
| 4 | 0 (2/5, 1) | 2 (3/5, 3) | 1 (4/5, 3) |
| 5 | 0 (2/5, 1) | 2 (3/5, 3) | 2 (5/5, 4) |

- **Plan against do nothing:** 6 stars against 2. Better on 3 seeds, worse on
  0, level on 2 (sign test p 0.25). Food joined: better on 5 of 5 (p 0.062).
  Five pairs cannot reach p < 0.05 on a sign test, so nothing here survives a
  correction. It is a direction, not a result.
- **Random against plan:** level on stars (6 against 6). On food joined the
  plan is better on 4 seeds and level on 1. **Random's stars are mostly
  toughness on small networks**: when its oats sit off the network, the
  thickest tube in the best-joined piece is sometimes a spur to them, and
  cutting that loses no pile. The bar "random play must lose to a plan" is
  therefore met on reach and not on stars. That is a scoring weakness to fix
  if the probe goes on. One candidate: strike the tube whose loss would cost
  the most piles, rather than the thickest.
- The plan bot is crude: oats as stepping stones toward the nearest unjoined
  pile, and the lamp on stray growth away from every corridor. A person
  should beat it, and the playtest is what tests that.

**Frame cost** (the window's frame, simulation plus drawing at 4x, plan bot,
whole dish awake, about 1,500 walkers): **mean 2.70 ms (simulation 1.37,
drawing 1.34), worst 8.84 ms.** The worst frame is the storm frame, which
labels every tube piece. A dish never settles, so the whole frame is redrawn
every frame. The 1.34 ms of drawing is the price of having no dirty-rect
skip. The dish is bounded, so this never grows with a bigger world.

## What was tried and changed

- **Food scent in the trail field** (Jones's single channel): the halo round
  each pile was drawn as tube and counted as tube, so every pile wore a pale
  ring and joined nothing. Split into its own field.
- **A pile that keeps its full smell once eaten** pulled every walker back
  onto the first pile; the mould never explored (do-nothing joined 1 of 5).
  A pile now gives off 15% of its scent once the mould is on it.
- **Unbounded eating** (no table limit) and **eat 0.06**: the home pile was
  eaten out in 2 s, and the whole mould starved by 30 s. Now each pile has a
  table of 40 eaters, eat 0.03, and 20,000 food.
- **A disc-shaped storm cut** (radius 6): the mould flowed round it within the
  watch, so every run scored toughness. Replaced by a 40-cell line across the
  tube.
- **Three cuts**, then **two** (the brief asks for three): the toughness star
  was earned in **0 of 20** runs at either, including a plan bot with 10 oats
  that spends its spares building loops. At **one** cut it was earned 4 of 20,
  never by doing nothing. One cut shipped. It also matches the synthesis
  review's graft (one strike picks the trunk).
- **Reading "after" at 2, 5 and 8 s after the storm** gave identical joins on
  every run. Pieces that each hold a pile are each fed and never rejoin, so
  nothing heals in that window. 2 s shipped.
- **Thrift at `TUBE_WIDTH` 4**: a straight tube read as 3.8x its own length.
  Tubes are about 12 cells wide at the tube threshold. Recalibrated to 12, and
  pile discs are left out of the count.
- **The plan bot's lamp** cut its own trunk on seed 2 (joined 3, then 2). It
  now keeps the light clear of piles and corridors.

## Left out, as the brief says (next steps if it is fun)

The pulse beat, spores and fruiting (starve-to-jump over salt), dye riding the
tubes, lightning as the finale, a rival mould, more dishes, and between-dish
perks. If it is fun, this module becomes the mould half of Seep
(`review-3-synthesis.md` §4a). Seep's own probe is building a graph mould,
and the two should be compared before either is kept.

## Playtest fixes (2026-11-08)

The demo playtest (`/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/playtest-2026-11-08/mould-review.md`)
ranked Mould 4th of 8 and asked for five fixes, in this order. All five are
in, and all ship on. Before/after page:
https://claude.ai/artifact/52KtqyTGjyhKMb7L14Eqq6

1. **The lamp pushes instead of erasing** (`Params::push`, `push_light`). The
   lamp was a repulsive term in every sensor plus a trail fade and a burn, so
   held on a tube it emptied that stretch and the tube came apart. It now
   carries trail and walkers outward one cell at a time; the old three levers
   default to 0. Measured (`mouldprobe bend=1`, lamp held 3 s at 25 s, against
   the no-lamp twin, 5 seeds):

   | lamp held | old lamp: food join lost while lit | new lamp |
   |---|---|---|
   | on the tube | 5 of 5 | **0 of 5** (the tube parts round the light) |
   | 8 cells beside it | 2 of 5 (tube moved 6.5-8.1 cells) | **0 of 5** (moved 5.2-6.6 cells) |

   The new lamp swings a tube a little less far, and never cuts it. Lamp to
   recoil is still **0.02-0.07 s** (`lean=1`, 9 of 9 placements with mould
   under the lamp); with `push=0` it never recoils in 10 s (control). Oats to
   a lean: **0.50-0.70 s**, 10 of 10. A sensor-only lamp at every lighter
   setting tried also lost the join: `Reports/dead-ends.md`, `other`.
2. **The middle minute has news.** An oat comes back every 15 s (up to the 5
   held). The far pile is not there at the start and turns up at 0:30, and a
   sixth pile turns up at 0:55 somewhere at least 45 cells from every pile.
   Each is announced: a dotted ring for 5 s before, a flash ring when it
   lands, and the time on the bar.
3. **The storm is warned and aimed.** 5 s ahead, a purple dashed line shows
   where it will cut. It is aimed at the cut that leaves the fewest piles
   joined (thickest tube among equals), not the thickest tube, so a spur out
   to stray oats no longer earns a random hand a free star. Toughness is
   read at the end of the 8 s watch, so a tube that grows back round the
   scar, or a bypass built during the warning, counts.
4. **Thrift bar 1.15** (was 1.5), so a branchy network has to be trimmed.
5. **The score is off the dish**: two lines of the bar under it, which grew
   from 3 lines to 5. The bar's lines are kept to about 40 characters so
   they read whole in the half-size pictures too.

**The same seven hands, before and after** (`mouldprobe`, seeds 1-5; the
hands in `mould::play` are this probe's own versions of the reviewer's, so
the before column is our replay, not the reviewer's numbers, which differ
for fresh 7, shepherd 4 and dump 5; the reviewer's plan-plus-decoy was not
rebuilt). After the fix there are 6 piles, not 5.

| hand | stars before | stars after | reach / thrift / tough after | food joined at 30 s = at 80 s (before -> after) |
|---|---|---|---|---|
| do nothing | 2 | **0** | 0 / 0 / 0 | 4/5 -> 3/5 |
| random hand | 6 | **3** | 0 / 2 / 1 | 3/5 -> 3/5 |
| simple plan | 6 | **8** | 5 / 1 / 2 | 1/5 -> 0/5 |
| fresh player | 6 | **9** | 4 / 3 / 2 | 3/5 -> 0/5 |
| lamp shepherd | 3 | 3 | 0 / 0 / 3 | 3/5 -> 2/5 |
| plan's lamp, no oats | 1 | 3 | 0 / 1 / 2 | 4/5 -> 3/5 |
| dump all oats at 1 s | 1 | 2 | 0 / 1 / 1 | 5/5 -> 3/5 |

- **Doing nothing now loses outright**: plan and fresh player each beat it
  on 5 of 5 seeds (sign test p 0.062, the floor for five pairs). Before,
  3 of 5 and level on 2 (p 0.25).
- **Random no longer ties the plan**: 3 stars against 8. Plan better on 3
  seeds, level on 2 (p 0.25); fresh player better on 4, level on 1
  (p 0.125). Random's toughness went from 3 stars to 1, as the aimed storm
  intended.
- **Reach is what the oats bought**: plan reached all piles on 5 of 5 seeds
  (2 of 5 before), fresh 4 of 5 (0 before). Thrift got harder for everyone
  (plan 4 -> 1, fresh 5 -> 3), which is the point of fix 4.
- **The lamp alone still cannot win**: shepherd 3 stars, lamp-only plan 3,
  and neither ever reached all food. The lamp steers and shapes; oats are
  still the only thing that leads the mould somewhere new.
- The storm still costs a pile in 24 of 35 runs (27 before): an aimed cut
  hits harder, and the warning only helps a hand that answers it. None of
  the scripted hands answer it, so the number to watch is the owner's.
- None of these comparisons survives a multiple-comparison correction: five
  seeds. Read them as a direction.

**Frame cost** (window frame, plan bot, 4x): mean 2.32 -> **2.56 ms**,
worst 13.38 -> **16.72 ms**. The worst is now the frame that aims the storm
(several hundred trial cuts, once per dish). Its first version flooded the
whole dish per trial cut and cost **36 ms**; flooding only the piece's own
cells gives identical aims on all 35 runs at 16.7 ms. Peak walkers rose
1,500 -> 1,800 with the sixth pile.

## What the owner's play found (2026-11-08)

The owner's verdict, from playing it: the mechanism is fun to play with and
looks good, but "I didn't really know what I was doing", the salt and the
snip were mysteries, the storm's scar read as "a weird red rectangle ... a
bug", "things were flashing gray", and there "needs to be a challenge".

- **The grey flashing was a drawing bug.** The feeding search walks only
  cells over the tube threshold, but tube is drawn from half of it, so the
  soft edge of every fed tube was drawn grey, and edge cells whose trail
  wobbled across the threshold switched yellow-grey-yellow every few
  frames. `mouldprobe flicker=1` counts cells that switch yellow<->grey 4+
  times in a second, lamp-lit cells left out: **about 420 a second** on a
  working network (seeds 1-3: 427, 417, 426), **0** after. A cell now shows
  fed if it or a neighbour was fed in the last half second
  (`Dish::looks_fed`, `FED_LOOK_REACH` 1, `FED_LOOK_SECONDS` 0.5); the
  simulation still reads `fed` alone, so every census is unchanged (all 35
  runs identical). Widening to 2 cells or 1 s changed nothing.
- **The game now explains itself.** Every dish opens behind a briefing card
  (the goal, what tan, yellow and grey mean, the three stars, what each
  tool does), held until a key or click; H brings it back. The tool hints
  say what each tool does to the mould. The storm's line is labelled
  "STORM WILL CUT HERE" and its scar "STORM SCAR".
- **The game opens on a lesson** (`mould::lesson`, seed 0; N goes on to
  dish 1). Six steps, each judged from the dish's own counters: hold the
  lamp near the mould (2 s); put oats down and let the mould eat them; join
  all the food; snip a tube; draw salt; then the storm, aimed and shown for
  the whole 20 s it gives. The storm is held back until that step
  (`Dish::storm_at`). Each step stays up at least 4 s: on the first
  scripted run the mould joined three piles 1.6 s into "join 3 piles",
  before the banner could be read, and the oats step passed with no oats
  eaten because the home blob touched a second pile on its own; the join
  step now asks for all the food, and the oats step for oats eaten.
  `mouldprobe lesson=DIR` plays it with a scripted hand: all six steps pass,
  the lesson ends at 51 s, 2 stars.
- **An independent bug check of the lesson** (2026-11-08, full write-up in
  the project's `game-design/mould-game-brainstorm-2026-11-08/bug-check.md`)
  found two ways a new player gets stuck, both now fixed:
  - *A slow lesson starved.* The lesson has no clock, but the mould still
    ate: a 20,000 pile lasts about 139 s at a full table. Reading for 60 s a
    step left 20 walkers at the storm; left alone, 0 of 400 walkers at
    200 s, and Join could then never pass. The dish's own piles now stay
    full until the storm step (oats still run out).
    `a_slow_lesson_does_not_starve` goes red with the refill switched off
    (0 of 400) and green with it.
  - *Spent tools made a step impossible.* Salt (1) and snips (3) never come
    back, so salt drawn during step 1 left the salt step stuck for good.
    The snip and salt steps now hand one back whenever the stock is empty,
    and the snip step needs a snip that cut mould, not a click on agar.
  - Smaller fixes from the same check: a stroke released over the HUD
    strip stayed live for the next press; the lamp could stay on after H
    or alt-tab; a window shorter than 726 px cropped the frame (it now
    shrinks to fit, crisp whole-pixel scaling whenever it fits); a score
    with fewer than 3 piles joined read "THRIFT 0.00X" (now "--"); R and N
    kept fast and pause on; salt and snip worked during the storm watch,
    letting a player cut their own network after the strike (now refused,
    like oats already were).


## Challenge dishes and the dish picker (2026-11-08)

The owner's "there needs to be a challenge", answered with the first three
dishes of the near-term-challenges brainstorm (`game-design/mould-game-
brainstorm-2026-11-08/near-term-challenges.md`), built for real in
`src/mould/level.rs`. Each asks one question and gives only the tools it
needs; none has a storm (a Steiner tree is a tree, which the storm always
splits, and Running Dry's only tube worth cutting is the finger to the next
food; Storm Season, dish 5, is where it belongs).

- **Behind the Wall** (90 s, lamp + oats): the food is behind a level-drawn
  salt wall with one gap at the bottom; scent does not cross salt. Stars by
  when the food is joined: at all, by 1:00, by 0:45. Moved from the
  brainstorm's layout: at (40,80)/(210,80) the way round was 207 cells and a
  relay of oats (each drains in ~3 s once eaten) died out after carrying the
  front 98 cells, so the scripted hand never arrived. Now 157 cells.
- **Beat the Ruler** (100 s, lamp + oats): three foods in a triangle and a
  dotted ruler (the two short sides). Scored on **the shortest join through
  the tube** (`Dish::best_join`: least over tube cells of the summed tube
  distance to the three piles), not tube area: area is set by food joined,
  not shape, and on the first run doing nothing read 1.40x and the meeting
  point 1.39-1.75x. Stars: all joined, 0.98x, 0.95x. The best possible
  (0.87x) is drawn faintly after.
- **Running Dry** (120 s, lamp + oats back every 10 s): small piles (~35 s
  of food each), a new one every 20 s further on. Stars at 100, 200, 300
  mould alive at the bell. The brainstorm's 300/600/900 is out of reach: the
  bell always finds the mould between piles (intended hand 354-411).

**A model change, hand-made dishes only: strays.** An unfed walker can walk
off the leash (a cell a step, ~5 s to live), and any walker on food made it
a source of the feeding search, so strays colonised food far past the leash.
With that, doing nothing joined all three in Beat the Ruler (1-2 stars on 3
of 3 seeds, `mouldprobe level=2 stray_reach=inf`) and a random hand took 3
stars there and on Behind the Wall. `Params::stray_reach` now limits it: on
a hand-made dish a stray can start eating only within 24 cells of fed mould
(infinite, the old rule, on the seeded dishes, which are unchanged). Shut
out entirely, an oat just past the leash could never be eaten and every
relay died.

**Ship bar** (`mouldprobe level=all`, seeds 1-5; stars per seed; the layout
is fixed, the seed moves the mould and the random hand):

| dish | do nothing | random | simple plan | fresh player | intended hand | bar |
|---|---|---|---|---|---|---|
| Behind the Wall | 0 0 0 0 0 | 0 0 0 0 1 | 0 0 0 0 0 | 0 0 0 0 0 | 3 3 3 3 3 (0:10-0:11) | met |
| Beat the Ruler | 0 0 0 0 0 | 0 0 0 0 0 | 0 0 0 0 0 | 1 0 0 0 0 | 3 3 3 3 3 (0.87-0.91x) | met |
| Running Dry | 0 0 0 0 0 | 0 0 0 0 0 | 2 0 1 0 0 | 0 0 0 0 0 | 3 3 3 3 3 (354-411 alive) | met |

The intended hand is `play::Route`: oats one at a time, a short step ahead
of home's fed tube along a route (down to the gap and up; home to the empty
middle, then the middle out to each far food; on Running Dry, from the fed
mould toward each new food as its ring shows). Two things it needed are
things a player needs too: **lay a relay in one go** (it saves up before
starting one), and in Beat the Ruler **keep feeding the meeting point**: left
alone, a junction slides toward a side (the mould straightens any bend it is
not pulled into), and one seed in five went from 0.87x to 0.99x by the bell.

The score card now says **why** in one line, from the failed condition, on
every dish including the free ones ("THE STORM CUT OFF 3 PILES: A LOOP
HOLDS", "SCENT CAN'T CROSS SALT: LEAD IT TO THE GAP"). **M** opens the dish
picker: the lesson, the three dishes (each opens on a star from the one
before; the lesson counts once finished) and FREE DISH, with best stars
saved to `mould-progress.txt` beside the executable.

Frame cost (steps + draw at scale 4, intended hand, one process alone): mean
2.4-3.3 ms, worst 7-12 ms, against 2.6 / 17 ms before. **Not fixed, noted:**
a dish whose mould has all died steps about 3x slower than a live one (idle
Running Dry: 27 s for 120 s of dish against 9 s with the intended hand),
probably subnormal floats in the fading trail; about 5 ms a frame, inside
budget, unverified.
