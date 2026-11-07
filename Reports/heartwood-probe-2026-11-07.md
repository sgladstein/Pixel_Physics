# Heartwood probe: a card-grown tree against a free one (2026-11-07)

**Status: probe built, waiting on the owner's playtest.** Code: `src/heartwood/`,
`src/bin/heartwood.rs`, engine hook `src/sim/tip_cards.rs`; harness
`examples/heartwoodprobe.rs`. Brief:
`/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/prototypes/8-heartwood.md`
with `common.md` and `plant-engine-rules.md` beside it (probe 8 of the
2026-11-07 new-game brainstorm). Picture page:
https://claude.ai/artifact/R68Fub2suZdXumdS6M9pkP

**The question:** is "your deck is your tree's genes" a game -- does choosing
which tip gets which card matter, or is it a spectator sim with a clicker on
top? Only a person playing it can say whether it feels like a decision. This
report holds what the measurements can say.

```
cargo run --release --bin heartwood                         # -- seed=N for a valley
cargo run --release --example heartwoodprobe -- play=idle,random,plan seeds=1-12
```

## What was built

- **A 256x160 valley, two trees.** Yours at x 80 in deep soil; the rival, a
  shipped `tree` with no cards, at x 176 on a stone shelf with 8 rows of soil,
  planted 600 frames earlier. Clear weather, the sky held at noon, seeds off
  (the rival's seed maturity is pushed past the year).
- **Plants run six times faster here only**: `Clock::growth_speedup` (new,
  default 1, `#[serde(skip)]`, not a panel knob) divides the organism interval.
  `ORGANISM_TICK_INTERVAL` and the shipped species are untouched. Bending is off
  in this world, and big plants' upkeep runs on the size cadence.
- **The hook, `sim::tip_cards`**: a card table on `World` (`None` everywhere
  else). For a listed organism's shoot tips only, plant.rs's `Grow` arm asks it
  first. No card: the tip holds -- no step and no ageing out. A card: its aim
  and keep-going replace the four steering weights, `Fork` forces the engine's
  own lateral (cap and roll aside; the flag waits for a step that can afford
  it), `Leaf Out` makes every step a leaf node at the engine's leaf price, the
  tip cap does not refuse it, and if every open cell scores against it the tip
  takes the least-bad one rather than none. The census tells the card why a
  visit did not step.
- **Five cards**, a 16-card deck, five in hand, one drawn every 1.5 s:
  Grow Up (16 cells up, 2 sugar), Lean to Sun (14 cells curving to the brighter
  side, 2), Fork (splits; every card after it goes to both halves -- the
  multicast, 4), Leaf Out (6 cells with leaves at every step, 3), Bud (keeps the
  next card on the stem; it grows if the branch above is lost -- the trigger, 1).
- **Sugar** is earned from your leaves' light (0.008 per unit of lit leaf
  light per second); cards cost it. **Prunes**: right-click cuts your own tree,
  two held, one back every 20 s; the engine drops what that leaves unsupported.
- **Score**: your share of the two trees' leaf light at the end of each of
  three 45 s seasons; stars at 40/55/70% of the last.
- **Readability**: gold ring on the picked tip, blinking white on idle ones,
  the dotted outline of the hovered card, the card's plain sentence, the tip's
  queue ("THIS TIP: UP > FORK") or why it is stuck, a fed/starving line, and
  each event as a note.
- **Left out, per the brief**: the dry third season, seedlings and Drop Seed,
  the run between levels.

## Results (seeds 1-12, paired)

Final-season light share, mean over 12 seeds: **doing nothing 0%** (12 of 12
seeds: the tree never grows without a card), **random cards 38.8%**, **plan
42.0%**.

| comparison | measure | wins | losses | ties | sign p |
|---|---|---|---|---|---|
| plan vs random | final share | 8 | 4 | 0 | 0.39 |
| plan vs random | stars | 5 | 0 | 7 | 0.06 |
| plan vs random | mean share over the year | 7 | 4 | 1 | 0.55 |
| plan vs nothing | final share | 12 | 0 | 0 | 0.0005 |

Three plan-vs-random comparisons were made, so none of them passes; the stars
row is the closest and is not significant at a corrected bar. Seeds with at
least one star: plan 9, random 5, nothing 0.

**Prediction test.** For every Grow Up / Lean / Leaf Out card that ran its
course, the cells between the outline's end (drawn as the card was fed to the
tip) and where the tip stood: plan, 461 cards, median 3.2 cells over a median
13.3-cell outline; 54% within 3 cells, 79% within 6. Random play: 536 cards,
49% / 78%. The control is the outline's own length: a tip that did not move
would miss by ~13.

**Frame cost.** Mean tick 0.45 ms (nobody playing), 0.64 (random), 0.66
(plan); p99 at most 17 ms; worst tick 41-43 ms in all three arms, including
idle, so the spikes are the rival's economy, not the player's tips (the worst
plan tick came with 25 tips). Drawing the window is 21-29 ms at worst.

**Stuck cards.** About one card in three is called off (plan: 312 of 988),
mostly "no room to grow" (a tip boxed by its own leaf spray or the rival's) and
"too tall for water to reach" (the engine's path-length turgor bound).

## What changed on the way, and why

- **Wood 21,000 -> 40,000 in this valley.** Idle, 6 seeds: at the shipped
  strength the rival snapped at the base and was lost whole on 2 of 6 (2,056
  cells in one snap, seed 2); at 40,000 it stood on all six; 60,000 changed
  nothing more.
- **Card tips stalled.** `why=1` on seed 3: 2,047 card-tip visits ended
  `NoGoodDirection` against 215 that grew, plus 922 `Boxed` and 918 at the
  height bound. Fixed with the least-bad step, a card called off after 8
  failed visits (with the reason on screen and the unspent share of its price
  refunded), and **a stuck tip retiring while the best-lit stem cell 20+ cells
  of path nearer the roots breaks into a new tip** -- without that, every tip
  was stuck by frame ~2,100 on seed 3, the hand sat full, roots doubled on the
  unspent income, and the crown fell whole when the trunk base starved.
- **Random cards first beat the plan bot**, because the bot stacked cards on
  the brightest tip, which was the one at the height bound. Rewritten to keep
  every tip busy and leave tips the screen calls stuck alone.
- **Sugar 0.02 -> 0.008 per light**: at 0.02 it piled past 400 unspent and
  every card was free.
- **Fork's flag** used to be spent by the first step whether or not the engine
  could afford a lateral, so forks failed silently; it now waits.

## What is still wrong

- **Order barely matters yet.** Breadth (cards on many tips at once) is most of
  what a plan buys over random. The obvious next lever is making cards differ
  more in what they are good for, or charging per tip, so that *where* a card
  goes is a real choice.
- **Bottom-up death is all or nothing.** A crown that outgrows its leaves can
  starve at the trunk base and fall whole (seed 3, plan: 1,790 cells lost over
  the year). This is the engine's starvation, not a snap, and it breaks the
  graded-outcome law when it happens.
- **Sugar binds only in the first season**; a grown tree out-earns the draw.
- **The outline does not warn that a tip will be stuck**, so a third of cards
  are refunded rather than chosen against.

## Lab check

The hook is inert without a card table, and `growth_speedup` defaults to 1
(`base * slowdown / 1`). `labshot frames=0,3000,12000` before (main 1474c46)
and after: PNG md5 `53565093abd9020d842557802f60ede8` both, and the printed
counts identical line for line.
