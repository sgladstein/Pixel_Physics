# Play as one ant: slice 0, the switch and three numbers (2026-10-11)

**What this is.** The owner's play-as-an-ant game for the lab: you are a
founding queen who forages, lays and grows a colony, and every other ant
follows its rules. The design is two Claude Docs: the design guide
(https://claude.ai/code/artifact/a7d42c48-a273-4d2b-9ec0-3aee3f793180) and the
implementation design
(https://claude.ai/code/artifact/1b612dde-1225-4539-aa9e-c3071e813fba). An
export of the second, with its second-lane review, is in the project's shared
folder at `game-design/`. This report covers slice 0, the part with no game in
it: the body the player drives, switched off, plus the three numbers the
design said to measure before any game code.

**Where it sits.** Slice 0 is the engine's half. Slice 1 is the first build
you can play: keys in the lab, a `queen_start` scenario and her colour. The
owner chose (card, 2026-10-11 06:53) to finish slice 0 before any of slice 1.

## 1. What was built

`sim::steer` and five hooks in `creature.rs`. The module doc lists them, and
`Reports/how-the-ant-works.md` §16 describes them as they ship. In short:

- With `World::steered` at `None`, nothing changes. Every hook is a branch
  not taken.
- With a body marked and `input` at `None`, her genome drives her. She has
  no old age (owner: *"My player shouldn't die of old age"*), no nest caste,
  and the queen breeding rule applies to her colony only.
- With `input` at `Some`, the keyboard drives her:
  - she stands unless steered;
  - she steps along the usable heading nearest the steer, within 90°;
  - with no usable heading she turns to face the steer;
  - E, Q, F, G, B, L and T are the verbs, with none of the colony's holds
    or refusals between key and verb;
  - T lays trail B wherever she walks;
  - sealed in with a pellet, G packs it behind her as she cuts on.
- `steer::grant` raises her reserve, booked as `Granted`.

`examples/queenprobe.rs` is the harness. It has three modes:

- `trail`: a hand-laid trail, then the workers at food;
- `found`: one queen until she has 20 workers;
- `beetle`: one queen with beetles released at landing.

`play=1` puts a scripted player in her body. The script walks to the heap,
eats at its edge, walks home down the shaft and lays. When stuck it digs and
wiggles, and it drops any pellet it holds. It is a floor on what a person
can do, not a model of one.

**Off is off.** `deeptrace scenario=steady_income food=0 frames=40000 ants=2
seed=3` on this branch against `origin/main` (ffea6583) gives byte-identical
`stats.csv`, `colony.csv`, `events.txt` and `ticks.csv.gz`. That holds at one
thread and at four, and still held after the last change (commit 880440af).
The positive control is `land=6001`, which differs from both.

Guards: seven tests in `creature.rs` and three in `steer.rs`. Each of the six
built around a fix was watched red with its fix taken out:

- the old-age gate;
- the `Move` level;
- `food_trail_lay`;
- the grant's booking;
- the turn in place;
- the steered pack.

## 2. Does a trail laid by hand pull workers to food?

**No, not measurably.** Twelve seeds, steady-income bed, gap 60. The queen
walks out to the heap, then walks home with T held (arm `line`). The count is
distinct workers in the food zone (12 columns of the heap) in the 6,000
frames after she turns for home. The arms were:

- `none` (no trail);
- `none` nudged one frame (`land=6001`);
- `placebo` (the same walk with T held, but toward empty ground on the far
  side);
- `ramp` (a positive control: a strong B ramp laid every 12 frames across the
  whole way for the window).

| arm | median workers at food | line beats it, seeds of 12 |
|---|---|---|
| line | 5.5 | – |
| none | 5.5 | |
| none, nudged | 5.5 | |
| placebo | 6 | |
| line beats both none and nudged | | 3 |
| line beats both nudged and placebo (**the bar: 9**) | | **1** |
| ramp beats both nudged and placebo | | 2 |

The run-to-run spread is as wide as anything a trail does. `none` and
`none, nudged` differ by up to 8 workers on one seed (seed 2: 1 against 9).
**Even the ramp, far stronger than any trail a queen can lay, does not move
the count.** So either B does not recruit empty workers on this bed, or this
count cannot see it. That is inferred, not traced: no worker's decisions were
followed. Started at frame 30,000 instead of 1,000, the line beat both on
3 of 12 and the ramp on 2 of 12, with 5 of 48 walks timing out.

**This fails the gate the design put before slice 1.** The owner chose to
finish slice 0 anyway. What it means for the game is the owner's call: a
stronger trail, a different cue, or a game where the player leads by doing
and not by trails.

## 3. How long does one queen take to reach twenty workers?

Steady-income bed, the queen landed 60 columns from the heap, mutation off.
The bar was 9 of 12 within 432,000 frames.

**Her genome driving her** (`play=0`; she is not steered, but her colony
breeds by the queen rule):

| reserve granted | reached 20 workers | frames, median of those | how she ended |
|---|---|---|---|
| 0 J | 3 / 12 | 51,115 | starved, all but one seed |
| 900 J | 7 / 12 | 54,115 | starved in 12 of 12 (median about 7,600 frames) |
| 3,000 J | 7 / 12 | 52,090 | starved in 12 of 12, about 3,300 frames in |

**Measured with the queen rule held after her death**: in slice 0
`World::steered` stays set when she dies, so her colony keeps the queen rule
for the rest of the run (only one ant lays at a time, the workers who take
over included). Slice 2's hand-off has to decide what a colony with no
player body alive runs.

The genome-driven queen does not feed herself. With a big reserve she lays
18 eggs at once and starves. Where a colony comes up, it is grown by the
workers who take over laying after she dies. One mutation-on run (seed 1,
900 J) reached 20 at 49,490 frames against 49,790 with mutation off.

**A scripted player driving her** (`play=1`):

| reserve granted | reached 20 workers | frames, median of those |
|---|---|---|
| 0 J | 8 / 12 | 14,990 |
| 900 J | **10 / 12** | 11,400 |

**With a player and a 900 J start, founding passes the bar**, about five
times faster than her genome manages. Every miss (seeds 2, 8, 9 and 10 at 0 J;
3 and 4 at 900 J) is the same failure: **the queen is buried at the heap**.
The income lands 40 cells every 1,000 frames, a lone queen eats a fraction of
it, the uneaten heap slumps over her, and the jaw takes no food. So she
stands inside it eating, 60 trips counted and a handful of eggs laid. A
person at the keys can walk away before that happens; nothing in the game
yet makes it obvious they must. **Even after founding, the scripted queen
starves later**: 29k-241k frames in, every seed. That has not been traced.
It is probably the script, which never feeds her while she lays, but it
could be the same burial.

## 4. How often does a beetle kill a lone queen?

**Every time.** Four beetles were released at landing (`one_nest_beetles`).
The bar was at most 3 of 12 killed in 100,000 frames.

| who drives her | killed | frames after release |
|---|---|---|
| her genome, 4 beetles | 12 / 12 | 544-680 |
| her genome, 0 beetles | 0 / 12 (all starved later) | – |
| the scripted player, 4 beetles | 12 / 12 | 192-792 |

The script makes no attempt to flee or hide. A person might, but at 1x there
are 200-800 frames to do it in. Four beetles in the box is not a world a
founding queen can start in.

## 5. What this leaves for slice 1

These are suggestions, not decisions:

- **The trail has to be made to matter, or dropped as the player's lever.**
  It is the design's main way for the player to lead, and right now it does
  nothing measurable. A worker-level trace of empty foragers crossing a hand
  trail would say whether B is read at all.
- **Start the queen with about 900 J.** It is what makes a scripted founding
  pass.
- **Keep beetles out at the start**, or have them arrive with the colony at
  a set size, as the rival does.
- **The food heap buries a lone ant.** Either the player's G takes food (as
  a carried load, so she can dig out of it), or the founding spot's income
  stops growing a heap she can drown in.

## Reproducing

```
cargo build --release --example queenprobe
RAYON_NUM_THREADS=1 ./target/release/examples/queenprobe mode=trail seed=S arm=line|none|placebo|ramp [land=6001] [start=30000]
RAYON_NUM_THREADS=1 ./target/release/examples/queenprobe mode=found seed=S reserve=0|900|3000 [play=1] [mutation=on]
RAYON_NUM_THREADS=1 ./target/release/examples/queenprobe mode=beetle seed=S reserve=900 beetles=4|0 [play=1]
```

Each prints one `RESULT` line; `QP_DEBUG=1` prints the scripted player's
state every 500 frames. A founding run is 438,000 frames, about 10-25
minutes on one thread.
