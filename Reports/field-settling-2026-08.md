# What the coarse field costs, and what actually decides it

*Round 7, sizing the 4x world. **This file has now been wrong twice and
rewritten twice**, in opposite directions, and both corrections are kept in
full at the bottom — how it went wrong is more useful than the numbers it got
wrong.*

## Measure it against the clock, not across it

The field's cost is governed by **two designed oscillators**: the day/night
cycle (`DAY_NIGHT_PERIOD_FRAMES = 3600`) and the weather, which is a pure
function of `(seed, frame)` and turns gusts on and off over thousands of
frames. Any mean taken over a window shorter than a cycle is a sample of an
oscillator at an arbitrary phase, not a cost.

That is not a theoretical worry. Three 600-frame windows on the **same**
2048x640 world, differing only in the frame they started at, measured
**0.00, 4.98 and 7.04 ms/frame** — each offered as "the settled field cost".

So `examples/field_cost.rs` classifies every frame by both oscillators and
buckets over two full cycles. Quote its `amortised` row and nothing shorter.

## The instruments, and what each is for

| | Answers |
|---|---|
| `field_cost.rs` | what a frame costs, bucketed by sky-step and gusting |
| `FIELD_PASS=N` | which of the eight passes the cost is in |
| `FIELD_DRIFT=N` | which *channel* is unsettled, and which of the three seeds woke each tile |
| `field_hash` | is this build's field bit-identical to that one's |
| `field_channels` + `FIELD_DUMP` | if not, how far apart, per channel, against the settle epsilons |
| `FIELD_CARRY` / `FIELD_SKYFAST` | run the paired baseline in the same session, without a `git stash` |

The last one matters more than it looks. The first reading of the
carry-forward change showed a 29.5 ms win **alongside a 50% regression in
every other pass**; the regression was entirely the machine having slowed
between two runs an hour apart.

## Where the cost was, at 8192x2560

| | solved | total | blocked | advect | diffuse | velocity | sky | pressure |
|---|---|---|---|---|---|---|---|---|
| sky step | 1482 | 59 ms | **29.5** | 7.8 | 7.6 | 6.4 | 3.4 | 2.4 |
| the echo after it | 555 | 34 ms | **11.2** | 5.7 | 7.4 | 3.8 | 3.2 | 2.2 |

Every other frame hits the early-out at the top of `step` and is free. The
"echo" is the frame after a sky step, re-solving the ~95 tiles the step left
marked unsettled; it is not an independent defect.

**Amortised, and what the two changes bought** (paired, same machine):

    30.36 -> 17.17 ms   carrying the CA-derived arrays forward
    18.44 -> 16.71 ms   skipping the momentum passes once they can write only zero
    worst frame 132 -> 72 ms

Both are **bit-identical**, verified by hash and by byte-comparing all six
channels at matched frames — not by a green suite.

### An awake *tile* is not an awake *chunk*

`rebuild_blocked` rescans all 4096 CA cells of every solved tile, with a
material-registry lookup each, to rederive `blocked`, `transmission`,
`moisture_source` and `glow`. Its own comment already called it "the busiest
standing cost in the field". What nobody noticed is that the sun wakes tiles
over rock that has not moved in ten thousand frames — `FIELD_DRIFT` reports
`chunk 0` on those frames — and a settled chunk's occupancy is unchanged *by
definition of settled*. Carrying the arrays forward takes the pass to 0.00 ms.

### Settled means changing slowly, and slowly is not never

The momentum passes are geometric decays, so once a tile's pressure and
velocity are **exactly** zero they can only write zero again. Keying the skip
on `settled` instead froze a gale's residue at 0.32 pressure units forever,
against a settle epsilon of 0.01.

## Rejected, with numbers, because each measured better

- **Subsetting `apply_sky_to` by column.** 8.26 -> 6.55 ms, and it was
  measuring a bug: a tile in the solve set is rebuilt from a fresh
  `FieldTile`, so a skipped column does not keep stale light, it **loses** it
  (mid-air cells going 2.43 -> 0.0 and staying there). Corrected to "solved
  OR drifted" it covered the whole world anyway and cost 8.26 -> 9.06 ms.
- **Skipping the momentum passes per tile.** Bit-identical on a calm world,
  and on a windy one pressure diverged **11.04** against an epsilon of 0.01:
  with the sun up, sky-woken tiles worldwide were being pressure-stepped, so
  a gust relaxed through all of them rather than advancing one ring a frame.
  The old behaviour is arguably the accident — a gust that spreads further at
  noon than at midnight — but it is wind the player sees, and changing it is
  the owner's call, not a side effect of a performance pass. **Open question
  for the owner, not a closed door.**

## Two bugs the counters caught and no timing could

- **The momentum skip never fired at all.** `any_fluid` counted
  `tile_unsettled`, and ~95 tiles are unsettled after every sky step purely
  because their *light* moved — so it was true on every frame of a dead-calm
  world. The timings looked like an ordinary frame; `momentum == solved` on
  every line of `FIELD_PASS` said otherwise. This is `CLAUDE.md`'s "did it
  fire at all needs a counter, not a picture", arriving as a pass that costs
  0.00 ms looking exactly like a pass that was skipped.
- **"A previous tile exists" is not "a previous tile was ever scanned".**
  `World::ensure_chunks_for` eagerly inserts a blank `FieldTile` for every
  chunk, so the first carry-forward version carried an all-default scan
  forward permanently for any chunk already settled when the field first
  stepped. A whole-field hash over 3,600 frames of the `rolling` preset said
  **identical** — worldgen leaves every chunk dirty, so those first solves all
  rescanned. Every hand-built test scene hit it instantly. Fixed by stating
  the difference as data: `derived_valid`, set by the scan itself.

## What is left

1. **`step_diffusion`, ~11-14 ms**, now the largest single pass. It is what
   makes shade soft rather than a hard stencil at field resolution, so it is
   not skippable the way the others were — a canopy's whole appearance rests
   on it.
2. **`apply_sky_to`, ~4-6 ms**, walks every column of the world whenever the
   step runs at all. Small, and the obvious subset is the one reverted above.
3. **What a gust costs.** Diluted at 4x, large at 1x (78 of 320 chunks woken
   by a +-34 dipole 52 cells across), and unhelped by either change here,
   because a gust genuinely disturbs the fluid channels.

---

## Correction 1: "the field never settles"

The first version of this file led with *"pressure never converges"*. It does,
at about frame 4500. Three instruments each gave a confident wrong answer, and
each was wrong the same way — **it could be satisfied without the world being
quiet**: a fixed 400-frame count; `world.fields_settled()`, which latched true
at frame 47 because it is a verdict over the tiles *actually solved* that
frame; and a `field_at_bilinear` lattice that reported quiet at frame 0
because points spread evenly over a world mostly land in solid rock.

The rule earned: **ask what a metric says when the thing it measures has not
happened yet.**

## Correction 2: "a ~4500-frame load transient"

The second version attributed 30-50 ms/frame over the first ~4500 frames to
generated terrain starting far from field equilibrium, and planned to seed the
field at load. **There is no load transient. It is the wind.** Seed 1 opens on
a gale — wind 0.963, below `GUST_THRESHOLD` at frame **3704** — gusting a +-34
dipole every 26 frames throughout. The field goes quiet at 4501, about 800
frames after the last gust, which is one gust's dispersal time. Three
ablations pin it: terrain is at rest by frame 7 (sweep-only run, zero awake
chunks), so there was never anything to seed; per-channel attribution puts
every unsettled tile on pressure with peak swings of 10.7, 13.5 and **14.7 at
frame 2400 — larger than at frame 1200**, and a decaying transient does not
get louder; and past the gale pressure peaks at 0.007 against an epsilon of
0.01.

The rule earned: **a cost that tracks the frame number is not necessarily a
function of the frame number.** Everything in `weather::at` is, and it was
sitting between the generator and the solver the whole time.
