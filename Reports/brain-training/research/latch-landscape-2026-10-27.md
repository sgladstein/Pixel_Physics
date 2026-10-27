# Can selection reach the "home had food" memory? A knock-out landscape (2026-10-27)

**Status:** measurement and recommendation. Owner's question 4 of 2026-10-27 13:45Z, asked through the Teacher lane:
can mutation reach the hand-laid "come home when low, if home had food" memory in small steps that each pay, or
must several weights change together? Data and harness: [`latch-landscape-2026-10-27/`](latch-landscape-2026-10-27/).

## Answer

**Several must change together.** Every piece of the working memory is necessary. Removing any one of them, or
setting it to half strength, loses the whole benefit. Two of those broken versions are far worse than having no
memory: 13 and 14 of 16 colonies die out. There is no slope for mutation to climb, so selection one or two
mutations at a time cannot find it. This is a property of how memory works in this brain, not of the training
settings. Since the Teacher lane's `latch1` batch also lost the habit when it was handed over ready-made, a
habit like this has to be taught and then protected. Or the memory has to change so that holding a fact is the
default rather than a balance of three weights (below).

## What was knocked out

The memory in `scratch/inrun/genomes/latch_pool.txt` (all 30 members), as the Teacher lane built it
(`colony-selection-log-2026-10-24.md`, "the gated brain"):

- **P** (memory unit 8, rate 0): at the nest AND a PickUp target: `20·at_nest + 20·reach[PickUp] - 22`.
- **N** (unit 11, rate 0): at the nest AND no PickUp target AND hungry: `20·at_nest - 80·reach[PickUp] - 60·energy + 2`.
- **L** (unit 9, rate 0), the held fact: `10·L + 15·P - 15·N - 2`.
- **The read:** come-home unit 8's gate gains `+3·L`, its bias `-2.74`.

Each arm changes one or two weights in every member (`mkarms.py`). Each arm ran 16 worlds on the Teacher lane's
standard setting: seeds `s*1000+7`, far moving food, founding grant, teacher off, selection off, 200k frames
(`wl.sh`).

| arm | what changed | lost (of 16) | starved, median of the living | lower than the full memory |
|---|---|---|---|---|
| **full** | nothing | 0 | **45.5** | -- |
| nostart | L starts unset, not "empty" | 0 | 20.5 | 13/16 |
| noread | the gate does not read L | 1 | 309 | 0/16 |
| nounit | come-home unit 8 switched off | 2 | 289.5 | 0/16 |
| noreset | N does not reach L | 0 | 303.5 | 0/16 |
| noself | L does not hold itself | 1 | 284 | 0/16 |
| self1 | L holds itself at 1, not 10 | 0 | 306 | 0/16 |
| read1.5 | the gate reads L at half strength | 1 | 280 | 0/16 |
| **set7.5** | P sets L at half strength | **13** | 7 (3 colonies) | -- |
| **noset** | P does not reach L | **14** | 0.5 (2 colonies) | -- |

The full row reproduces the Teacher lane's own (45.5 starved, 0 lost), which checks the harness. `nostart`
reproduces its "starts unset" row (20.5). The Teacher lane rejected that version because it starved every far
lab box, which this harness does not run.

**Every single change either drops to "no come-home"** (starved 280-309, as with unit 8 off) **or flips to "always
come home"**, which kills colonies. Halving a weight behaves like removing it, because each piece works against a
threshold: the gate's bias of -2.74 must be beaten, the latch's self-weight must be well past 1 to hold, and the set
input must beat the self-weight plus the bias.

## Why breaking the set input kills colonies (a trap in reading knock-outs here)

A memory unit that is "off" reads **-1, not 0**. Away from the nest P and N are both about -1, so L's inputs there
are `15·(-1) - 15·(-1) = 0`, and the pair balances. Remove P's input (`noset`) and N's `-15·(-1) = +15` is
left alone: L turns *on* everywhere except at an empty nest when hungry. The gate opens, and every ant comes home
whenever it is low, whether or not home has food. That is the "gate alone" failure from 2026-10-26, made worse.
Halving P (`set7.5`) leaves the same imbalance.

Two controls confirm this:

- With the gate shut by hand (`noset_shut`, gate bias -100) the same broken memory loses 0 of 4 colonies
  (starved 270.5), like `nounit`.
- With links free (`PIXEL_PHYSICS_LINK_PRICE=0`), `noset` still loses 3 of 4. So it is the wiring, not the
  brain's running cost of unused links.

**The general point for any knock-out or partial lesson in this brain:** zeroing one input of a balanced pair is not
"removing that feature". It hands the other input's -1 a meaning. Check a pair's resting balance before reading
a knock-out.

## Why mutation cannot get there step by step

To carry "home had food" from the nest out to the field, three things must exist at once:

1. a signal that fires only at a nest with food (an AND, here a unit of its own);
2. something that holds it across a trip;
3. something that reads it.

Each is worthless without the other two. The hold also only works past a threshold:

- At rate 0 it needs a self-weight well above 1 *and* set/reset inputs strong enough to flip it (the measured
  cliffs above).
- With the rate gene instead (`0.5 + g`, up to 0.995), one number sets both how fast the unit writes and how long
  it holds. So it cannot write in a few decisions at the nest and then hold for a whole trip.

The brain bills every live link, so the pieces cost something while they wait for the others. And a piece that
is half wired the wrong way, like P missing from a balanced pair, is lethal.

## What would let training find it, or keep it

1. **Teach it, then protect it** (the Teacher lane's direction, no brain change). Lay or copy the habit in, then
   hold those slots out of mutation (`lesson=` protection at a much lower rate than `latch1`'s 0.1, or
   `no_prune`). `latch1` showed this habit is lost under ordinary selection: putting unit 8 and the memory back
   into its best colony cut starvation 291.5 -> 29.5, lower in 16 of 16 worlds.
2. **A memory that holds by default** (a brain change, owner's call). Give each memory unit a write gate,
   `m' = m + z·(squash(x) - m)`, where `z` is computed from the inputs. Keep the old fixed rate at zero gate
   weights, so every saved pool is unchanged. Holding then needs no self-weight balance, and writing only at the
   nest is one weight on `at_nest`. That removes the hold cliff and the balanced-pair trap. It does not remove
   the need for a signal and a reader, so it makes the path shorter and smoother, not one step. Whether
   selection then finds it should be tested by running selection from a pool without the habit, not argued.
3. **A structural mutation that adds a ready-wired memory motif** (fire on X at Y, hold, read): general, but a
   larger design.

Recommended: 1 now, because it needs no brain change and the Teacher lane is already doing it. Test 2 as a
prototype, off by default, judged by whether selection from a pool without the habit finds come-home.
