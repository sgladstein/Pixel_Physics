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


## Making a cut matter, and food seen moving (2026-11-08, overnight)

A brainstorm round on where the game goes (five write-ups in the project's
`game-design/mould-game-brainstorm-2026-11-08/`) agreed, separately, that
food should be seen moving, and found that a cut cost nothing: a snip
healed in about 0.5 s and a whole scripted game drew 0 grey tube cells.
The owner chose "each piece eats its own food" on a card.

- **Each piece eats its own food** (`Params::pieces`, on). The feeding
  search now remembers which pile each fed cell's food comes from; where
  two piles' searches meet they are one piece. Every walker carries its
  share of its piece's food, so a split or a join neither makes nor loses
  food. A piece earns from its own piles, pays its own upkeep and births,
  and goes hungry (farthest first) on its own. Off reproduces the shared
  store exactly: 7 hands x 5 seeds, every star identical to before.
- **A cut stays open** (`Params::wound`, 4 s). Snipped cells become
  `Ground::Wound`, which the mould cannot cross, drawn dark wine and fading
  back to agar over the last second.
- **Joins flash** (`Dish::snaps`): where two pieces meet and become one, a
  burst that spreads over 0.8 s, unless those two piles were joined within
  the last 2 s (a thin tube flickering across the threshold would otherwise
  flash every census). 1 to 23 per scripted run, median 4.
- **The pulse**: a bright band rides out along fed tube from its food,
  read off the feeding search's distances, every 1.3 s. Starving tube has
  no distance, so its beat stops. Drawing only.
- **Toughness now needs 3 piles joined**, like thrift: with pieces on, a
  do-nothing 2-pile network regrew round the scar within the watch and took
  the star on 2 of 5 seeds.

Stars with all of it on (5 seeds): idle 1 (seed 1's network happened to be
1.13x, under the thrift bar), random 3, plan 10, fresh 7, shepherd 4,
lamp-only 2, dump 3 (before: 0, 3, 8, 9, 3, 3, 2). `mouldprobe cut=DIR`
snips across the trunk at 40 s: the most grey tube in the next 12 s was
14, 139, 308, 40, 12 cells against 26, 80, 74, 31, 21 for the old snip,
but the arms cut different places (the trunk differs between them), so
this is not yet a paired answer. **A trunk between two fed piles is the
wrong test of starving**: both sides keep their food. What a per-piece
store changes is a piece with more mould than its food can feed; a branch
beyond the leash starved before and still does. Frame mean 2.81 ms,
worst 14.62 ms (2.63 / 17.38 before, same session).

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

## Review 1 fixes (2026-11-08)

An independent bug check of aeae6d29 (`game-design/mould-game-brainstorm-
2026-11-08/review-1.md`) found 13 things; all are fixed on `challenges`, each
game finding a test can reach has a regression test, and each of those was
run red with its fix put back.

- **A dish whose food has all run out could not be rescued** (Running Dry:
  oats on 950 just-starved walkers never eaten, 85 s left unwinnable). Mould
  fed in the last `Params::stray_grace` = 2 s now counts as fed for the stray
  rule, **only while nothing on the dish is fed**. Applied at all times it
  let Beat the Ruler's intended hand join round the side on seed 7 (1.10x,
  1 star, at every grace from 1 to 3 s), and at 5 s a random hand took 3
  stars on Running Dry seed 4.
- **Join flashes** fire only between two piles that were both being eaten,
  in different pieces, at the census before. Three oats on fed tube: 6
  flashes before, 0 now; a hand-laid bridge between two fed pieces still
  flashes. Joins per run fall by about half for the scripted hands (Running
  Dry intended 19 to 8-12, Beat the Ruler 11-17 to 4-10, the free dish's
  plan hand 12-21 to 6-12); what is gone is oats eaten by the network
  already there.
- **Free dish stars are a ladder**: thrift and toughness need all the food
  joined. Stars over 8 seeds, before to after: idle 2 to 0, random 5 to 0,
  shepherd 5 to 0, lamp only 3 to 0, dump 3 to 0; plan 14 and fresh 13
  unchanged (every one of their shape stars was on a full join).
- **Smaller ones.** The organ heals its snips and its window lets go on
  release. A tool a dish does not give is never selected, and the keys for
  it do nothing. A dish with no mould left ends then ("THE MOULD DIED AT
  1:05"), and "CAME WITHIN 0 CELLS" now reads "ONLY STARVING STRAYS GOT
  THERE". Beat the Ruler's join no longer hops a one-cell break: no long step
  squeezes past a pinch, and with fewer than all three joined there is no
  join. Escape quits only on a second press within 2 s. The probe prints its
  parameters in every mode, builds every dish from them, honours an explicit
  7-hand list on `wander=1`, and reads `oats=` in `level=`, `wander=` and
  `cut=` (refusing it elsewhere).

**The ship bar did not move**: on seeds 1-8 every one of the 120 star cells
of `level=all` is the same before and after (intended 3 on all; Beat the
Ruler 0.87-0.92x, Running Dry 354-411 alive). **Not new, but found here:**
on 8 seeds Running Dry's random hand takes 3 stars on seed 7, before as
after (its mould grew onto the new piles by itself, helped by a stray oat),
so the 5-seed bar it passes is not an order statistic yet. Frame cost (dish
1, intended hand, back to back): mean 2.71 ms, worst 4.85 ms, against 3.08 /
5.70 before.

## Review 2 fixes (2026-11-08)

A second independent check, of 754853039
(`game-design/mould-game-brainstorm-2026-11-08/review-2.md`), found 2 high,
2 medium, 5 low and 3 probe-only findings; all are fixed, with 13 new tests.
The 12 that guard a fix were run red with their faults put back at once,
12 of 12. The card's "inf" guard was blind at first, because Rust writes an
infinite float "inf" and the test looked for "INF". The thirteenth guards
the two-press Escape that the organ now shares. Two window-only changes,
the organ's scaling and its title, have no test.

- **Food still due when a dish ends (H1).** Running Dry's losing hands end
  before its 80 and 100 s piles, and the Wanderer's Go comes before its
  55 s pile. That food stayed due for ever: the built look counted down to
  it and then wrapped to `153722867280912906` (a panic in a debug build).
  Now the dish's end and Go drop it, and the look draws food due only while
  the dish grows. At Running Dry's "mould died" moment the bar reads food
  1/6 rather than 1/8.
- **Oats on just-starved mould (H2, M2).** The 2 s grace rescued only oats
  laid within about 1.5 s of the last fed cell going, and none while
  another piece was still eating. Meanwhile the preview said "It will reach
  these". The ground *under the oats* now counts as fed for 8 s after it
  was (`Params::oats_grace`), whatever else is fed. Two oats on the
  mould's middle walker, Running Dry left alone, seeds 1-3:

  | laid after the last fed cell went | +0 s | +1 s | +2 s | +3 s | +5 s | +8 s |
  |---|---|---|---|---|---|---|
  | before (review 2's table) | eaten | eaten | never | never | never | - |
  | after | eaten | eaten | eaten | eaten | eaten | never |

  Mould alive at those times: 950, 763, 604, 459, 280 and 139 on seed 1.
  The M2 case (random hand, seed 4, a branch starved far from home while
  home still ate) is eaten now.
  - Where oats would not be started, the preview now says "Only starving
    strays here: too far from fed mould", reading the same rule
    (`Dish::oats_would_start`).
  - **Two wider rules failed the ship bar on 8 seeds.**
    - Oats exempt from the leash outright (the review's option a) brought
      the beachhead back. A random hand took 3 stars on Behind the Wall on
      seeds 1, 6 and 8, and on Running Dry on seeds 2, 4 and 7.
    - An 8 s oats grace read over the whole 27-cell leash let strays start
      the intended hand's oats, laid ahead of the tube on Beat the Ruler.
      It joined round the side on seed 7 (1.16x, 1 star) at graces of 2, 4
      and 8 s.

  **Star table, `level=all` seeds 1-8, before (754853039) / after:**

  | dish | idle | random | intended |
  |---|---|---|---|
  | Behind the Wall | 0 0 0 0 0 0 0 0 / same | 0 0 0 0 1 0 0 0 / same | 3 on all 8 / same |
  | Beat the Ruler | 0 on all 8 / same | 0 on all 8 / same | 3 on all 8 / same |
  | Running Dry | 0 on all 8 / same | 0 0 0 0 0 0 3 0 / same | 3 on all 8 / same |

  - Every star cell is the same before and after. Five random-hand runs
    differ in detail (mould left, how near), so the rule does fire.
  - The bar (idle 0, random at most 1, intended 3) holds on seeds 1-5.
  - Running Dry's random hand on seed 7 still takes 3 stars, 723 alive at
    the bell, the same before as after. Review 1 found it, and it is not
    from this rule.
- **Tool tiles (M1).** A click on a tile in the built look or C (and on
  any look's Wanderer bar) now selects that tool. Only a press on the dish
  holds the button. Before, the tiles did nothing, and a press on the bar
  slid up onto the dish lit the lamp. The window and the tests share one
  hit test (`render::press_at`, `look::tile_at`).
- **Lows.**
  - L1: the refusal under a tool names the phase: "Dish over", "Not now:
    she's on her way", and "the storm" only in the storm.
  - L2: the glow cache is keyed by the dish as well as its address. The
    lesson, M and a challenge dish showed 41,888-50,857 pixels of the
    lesson's glow behind the briefing card; now 0.
  - L3: the armed Escape's window title goes back after 2 s.
  - L4: the organ shrinks to fit a short screen and quits only on a second
    Escape within 2 s. Both windows share the rule (`render::QuitArm`,
    `render::frame_fits`).
  - L5: Behind the Wall with both foods eaten apart now reads "NOT JOINED:
    BOTH FOODS EATEN, APART", not "CAME WITHIN inf CELLS". This state was
    met for real on random seed 5 under option a.
- **Probe.**
  - P1: a chip wider than the frame no longer panics, so `level=3` at the
    default scale 2 draws.
  - P2: `pull=` and `looks=` refuse `oats=`.
  - P3: `lesson=`, `looks=` and `menu=` build their dishes from the printed
    parameters (`Lesson::dish_with`).

**Frame cost, back to back** (dish 1, intended hand, steps plus draw at
scale 4, 664 frames; run order Night, Classic, Night, Classic):

| look | mean, run 1 | mean, run 2 | worst, run 1 | worst, run 2 |
|---|---|---|---|---|
| built look (Night) | 4.02 ms | 4.27 ms | 8.65 ms | 12.57 ms |
| old look (Classic) | 2.73 ms | 2.68 ms | 4.50 ms | 14.10 ms |

The built look costs about 1.3-1.6 ms more on average.

## Food that flows (2026-11-09)

**The owner's ask:** food that stopped at a tube distance "clearly is just like a distance metric", the drawn pulse was disliked ("everything we're seeing should be based on real pixel simulations"), and all three levels of real flow were asked for, each behind a switch or a number to play with.

**What it is** (`Params::flow`, off by default until the owner has played it; the dishes were tuned against the old rule):

1. **Food carried by the mould.** Walkers carry a load (`Walker::food`), pay upkeep from it, divide from it and starve when it is empty; neighbours even out (`share`). There is no reach: the census still runs, uncapped, only for the score, joins, the stray rule and which way along a tube is outward.
2. **Food in the tube, a stream and a pulse** (`sap`). Piles pump into the tube cells at their rim on the beat (`pulse_seconds`, `pulse_strength`); the tube's food is carried outward and back by a stream (`stream`, `stream_bias`, scaled by tube thickness) and soaks along it (`sap_speed`); a walker draws from its cell only what it is short of `want`. The beat is the real organism's shuttle streaming, sped up.
3. **Tubes that remember** (`wall_grow`, `wall_half_life`, `wall_pull`). Food moving through a cell builds a wall (capped at `WALL_MAX`) that fades slowly, widens the tube's conductance and is sensed by walkers on top of the trail.

What is drawn with flow on is the walkers' average load (`Dish::carried` over `Dish::walkers_seen`) plus the tube's food (`Dish::sap`), graded; the drawn-only pulse is off. T in the window opens the tuning panel (`mould::tune`): 19 rows, live, saved to `mould-tune.txt`; the last four (2026-11-09) are Jones's sensor angle, turn angle, sensor reach and the piles' smell.

**Measured** (free dish, seeds 1-4, old rule against flow at its defaults):

| | Old rule | Flow |
|---|---|---|
| Plan: food joined | 6 6 6 6 | 6 6 6 6 |
| Plan: stars | 2 2 2 2 | 1 1 2 1 |
| Plan: storm cost a pile | 2 of 4 | 4 of 4 |
| Plan: peak mould | 1,857-2,199 | 1,566-1,664 |
| Do nothing / random: stars | 0 on all | 0 on all |
| Frame, mean (sim + draw) | 3.33 ms (1.13 + 2.21) | 5.01 ms (2.57 + 2.44) |

The storm now costs a pile every time because a cut-off piece really starves; toughness is the star lost. **What was tried first:** food handed to walkers on the pile (level 1 alone) left 90% of walkers within 20 cells of food carrying it and the middle of long tubes starving (load 0.6 near food, 0.02 at 40+ cells), and a total of carried food drew every pile bright and every tube dim (median tube cell 0.6 against 8 at p90), hence the average load; spreading alone (`sap_speed` 12-24, no stream) was absorbed within about 5 cells of the rim. The stream plus demand-only drawing is what moved food down the tube: far walkers' mean load 0.076 with stream and spreading on against 0.010 with both off (seeds 1-4, 40 s; the test runs seeds 1-2; `flowing_food_reaches_down_the_tube`). Mid-tube walkers are still lean: the next tuning question for the owner's panel.

