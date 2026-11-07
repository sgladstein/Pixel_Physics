# Mould probe: herding a slime mould with a lamp (2026-11-07)

**Status: probe built, waiting on the owner's playtest.** Code: `src/mould/`,
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
  2 s later, then watch for 6 s more.
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
