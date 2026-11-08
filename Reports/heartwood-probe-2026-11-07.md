# Heartwood probe: a card-grown tree against a free one (2026-11-07)

**Status: round 2 built (the 2026-11-08 playtest's six fixes), waiting on
the owner's play.** Careful play now beats every lazy strategy but one on
final light (6 of 7 in the same direction, none past the corrected bar);
"Leaf Out only" ties it. Before/after page:
https://claude.ai/artifact/E8tN9PvQ8cCMfiawmioD9N. Round 1 below is kept as
measured. Code: `src/heartwood/`,
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

## Round 2: the playtest's six fixes (2026-11-08)

The playtest (`/mnt/project-files/game-design/new-game-brainstorm-2026-11-07/playtest-2026-11-08/heartwood-review.md`)
found choosing a card was not a decision: shift-click sent one card to every
picked tip and paid per tip, so "pick every tip, play anything" beat the plan
bot; sugar stopped binding after season one; cards were small and many; light
fell straight down, so two crowns side by side could not shade each other; a
third of cards were refunded after the fact; and each seed drew a different
rival. Its fixes, as built:

- **One card, one tip.** `select` replaces the pick; shift-click is gone. Fork
  is the only multicast (the fork child does not join the selection; later
  cards on the stem go down both halves as before).
- **Sugar binds all year**: earnings minus `UPKEEP_PER_WOOD` (0.002) per wood
  cell above ground per second; the HUD shows the net rate.
- **Fewer, bigger cards**: Grow Up 40 cells (6 sugar), Lean 36 (6), Leaf Out
  14 (8), Fork 8 then splits (8), Bud (2); one drawn every 200 frames. Card
  tips step every `CARD_STEP_FRAMES` (2) through the new
  `TipCards::step_frames`, and are topped up to `RESOURCE_SCALE` carbon each
  frame, so a paid card grows its full length rather than stalling on the
  tree's purse.
- **Two low suns**, game-side (`sun_on`/`sun_at`): a ray march from each top
  corner at a 3:2 slope, a leaf passing 0.8, anything else stopping it,
  averaged. Score, sugar and Lean's "brighter side" use it; the engine still
  grows by its vertical light. Leaves under 0.3 are drawn dark. One sun from
  the left left the plan only edging key-mashing (67.5 vs 60.8, 4-2 on 6
  games); one from the right gave everyone 0%.
- **An honest outline**, checked before paying (`verdict`): red with the
  reason (not enough sugar, too tall for water, no empty neighbour), amber
  when water reaches only part of the length (reach = 0.9 of the species'
  height ceiling minus the tip's path length). A tip within 3 of its reach
  with nothing queued retires and a bud breaks lower.
- **The same valley and rival every game** (`VALLEY_SEED = 1`); the game
  number shuffles the deck only.

**Replay, seeds 1-12 paired, eight strategies** (the playtest's cheese bots
added to `heartwoodprobe`: `mash`, `top`, `overhang`, `leafonly`,
`selectall`). Final-season light, mean; careful play's wins-losses against
each on the same seed. The score changed (low suns, fixed rival), so compare
within a column.

| strategy | before | plan vs it | after | plan vs it |
|---|---|---|---|---|
| do nothing | 0.0 | 12-0 | 0.0 | 12-0 |
| plan | 42.0 (82 cards/yr) | -- | 51.9 (22 cards/yr) | -- |
| random | 38.8 | 8-4, p 0.39 | 41.9 | 9-2, p 0.065 |
| mash | 37.8 | 7-5, p 0.77 | 42.0 | 10-1, p 0.012 |
| top | 28.9 | 11-1, p 0.006 | 40.2 | 10-2, p 0.039 |
| overhang | 25.6 | 10-2, p 0.039 | 35.1 | 10-2, p 0.039 |
| leafonly | 41.8 | 6-6, p 1.0 | 51.3 | 6-4, p 0.75 |
| selectall | 45.8 | 3-9, p 0.15 | 36.8 | 10-2, p 0.039 |

Seven comparisons, so the corrected bar is about p 0.007 and none passes but
doing nothing; mash is closest. Stars over the 12 seeds: plan 11 -> 16,
selectall 14 -> 7, leafonly 8 -> 14. The playtest's own before numbers differ
for selectall (51.8) and its cheese bots; both have selectall ahead of plan.

- **Refused before paying**: plan 0 a year; mash 2,861 and selectall 6,328
  over 12 years (they press regardless). Stuck after paying: plan 7.7 a year
  (was 26).
- **Purse at year end**: mash 2-132, random 0-102, plan 7-223, leafonly
  73-429 -- Leaf Out alone does not spend what it earns.
- **Losses are graded**: plan loses 0-79 limbs a year (1,143 cells at worst,
  seed 3); round 1 dropped 1,790 cells in one starvation.
- **Prediction**: 136 plan cards ran their course; per-seed median miss 3.0-7.6
  cells over 14-36-cell outlines; 41 within 3 cells, 88 within 6.
- **Frame cost**: mean tick 0.46 ms idle, 0.66-0.68 playing (was 0.43-0.59);
  p99 at most 18.2 ms; worst 40-57 ms (was 32-48); idle's own worst is
  41.8 ms, the rival. Draw worst 48 ms.

**Still wrong**: Leaf Out only ties careful play and banks its sugar, so
the next lever is its price (rising with each play, or with how much of
your crown is already dark).

**Lab check (round 2)**: `labshot frames=0,3000,12000` at main 4e0cf639
and on this tree: PNG md5 `53565093abd9020d842557802f60ede8` both, printed
counts identical except the output path.

## Results, round 1 (seeds 1-12, paired)

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
