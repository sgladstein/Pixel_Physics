# Birth brakes, recruitment and the nest-life switches: what was tried, what it did, and why (2026-10-07 to 10-13)

*Written 2026-10-13 by the session on `claude/eloquent-johnson-axjvu1` (and its
`-nest-life`, `-appetite-sweep`, `-feed-first`, `-mound-trace`, `-mound-dig`,
`-stack-loo`, `-store-heap90`, `-flip-heap30` branches), at Scott's request: a
full account of everything this session tried, what each thing did, why it
worked or failed, and which version of `main` it was tested on. Every number is
marked **measured** (counted in a named run), **traced** (followed ant by ant or
decision by decision) or **inferred** (reasoned from code and numbers, not
followed). Where this report corrects something the session said earlier, the
correction is called out.*

## 0. Summary

Scott's two goals for this work were **a colony that builds up a store of food**
and **a stable colony without boom and bust**. Neither was reached by any birth
brake.

- **Five ways of slowing laying were tried** (the appetite dial, feed-first, the
  lay brake, the food brake, the store brake). None built a store. The ones that
  slowed laying enough to matter either made the colony unstable or killed it.
- **The colonies were not overbreeding.** With food coming in and recruitment
  on, the unbraked colony plateaus at about 1,200-1,600 ants to 240k frames;
  98-99% of deaths are old age. Every brake that cut or moved births made the
  long run worse.
- **What actually helped:** `edible` (the store-crumb trap; earlier sessions'
  work, confirmed here on steady food), **recruitment** (`RECRUIT`, the colony
  roughly doubles) and **spoil off the food trail** (`SPOIL_SIDE=trail`). The
  playtest line at the end of the session was the stack plus those two, no
  brake.
- **Why the lay brake failed, traced on 2026-10-13:** it brought far more of the
  colony into the nest and deep (the goal Scott has named first), and the nest
  had no way to feed its deep residents. 739 adults starved in the nest over
  150-240k against 8 without it.
- **Why the store stays low on the playtest line** (measured): of about 150
  cells of food a 1,000 frames that come home, about 11 are carried into the
  store. On the stack *without* recruitment the store holds food (57-99 cells
  here; 119-213 in [`follow-food-home-2026-10-10`](follow-food-home-2026-10-10/README.md)),
  so the low store is a property of the recruiting colony, not of the store.

## 1. Versions tested

All runs used `examples/deeptrace.rs` on `scenario=nest_goal` (the dry goal box),
evolved founder, mutation off, one thread a run, seeds 1-4 unless said. "Heap
90" is the food heap 90 columns from the door with the harness top-up (`food=120`
target), so food never runs out. "The playtest switches" (or "the line" before
recruitment) are:

```
NEEDS_FIRST=on,backfill CARRY_HOME=on DOOR_COLUMN=on LAY_BAR=body
NEST_STORE=on,pick=20,jaws,sky,meal,smell=10,edible WAY_FOOT=on HUNGRY_OUT=on
```

set as `PIXEL_PHYSICS_*` environment variables, with the switch under test added.

| When | What | Code (build) | Base `main` |
|---|---|---|---|
| 10-07/08 | appetite dial on `boom_bust` and `steady_income` (10 cells), the brakes proposal | PR 661 branch | `f55b33b8` (the stack) and main of 10-07 |
| 10-08 | appetite sweep, steady food 40 cells; floor check on heap 90 | PR 662, 665 | main of 10-08 plus the dial row (PR 661, `6c4bbabc`) |
| 10-08 | `FEED_FIRST` | PR 663 | `6c4bbabc` / `2c7dd4d2` |
| 10-08 | mound diggers trace; `MOUND_DIG` | PR 664, 667 | `f55b33b8` |
| 10-08 | stack leave-one-out; store on heap 90; default flip at heap 30 | PR 666, 668, 669 | `f55b33b8` / `6ffda5db` |
| 10-08 eve - 10-09 | A/B/C/R combinations, `HUNGRY_OUT=off` | `9ef67251` | **`43522586`** (main after PR 669) |
| 10-08 eve | `STORE_READ` | `c525d229` | `43522586` |
| 10-08 eve | `MEET_WAY`, `SPOIL_SIDE` | `f517ce64` | `43522586` |
| 10-09 | `MOUND_IN`, brain census | `772c690f` | `43522586` |
| 10-09 | `RECRUIT` | `a497dbd9` | `43522586` |
| 10-09 | `LAY_BRAKE`; appetite dial on recruitment; both; food brake sweep; long runs to 240k | `1bb916c1` | `43522586` |
| 10-09 | `STORE_BRAKE` | `d7c0f2c9` | `43522586` |
| 10-13 | lay-brake and food-at-home traces (reruns with full logging) | `1bb916c1` | `43522586` |

Checks that make the comparisons fair (measured): a later build with its new
switches off plays the same game as the earlier one (recruitment seed 1 on
`a497dbd9` and on `1bb916c1`: 115 checkpoints, 0 differences); the 10-13 logged
reruns are the same worlds as the runs they trace (235, 235 and 115
checkpoints, 0 differences).

**Since then.** All of the 10-08/09 code landed on main in PR 670
(`e5810aed`), every switch off. Main has since taken 24 merges; the one that
matters most here is **PR 675** (2026-10-10), which turned the stack on by
default, with the store on and `NEEDS_FIRST`'s `job` part off. **None of the
results below has been re-run on today's main** (`9b0e401c` at writing). The
switches they are about (`RECRUIT`, `SPOIL_SIDE`, the brakes) are still off on
main.

## 2. The birth brakes

### 2.1 The appetite dial (`digest_hunger_weight`, a lab parameter)

**What it does:** a full ant stops digesting and keeps its surplus in the crop,
so less is burned and less is banked for eggs.

| Test | Result (measured) |
|---|---|
| `boom_bust`, 0.5, 4 seeds | eggs 649-724 -> 603-617; food lasted 10-17k frames longer; larvae starved 112-117 -> 65-85. 1.0 stops breeding |
| `steady_income` at 10 cells (~100 ants), 0.5, 4 seeds, 300k | no seed fell below 20 ants (unbraked fell to 2 and 6); adults starved 208-416 -> 104-244 |
| `steady_income` at 40 cells (200-300 ants), 0-0.75, 4 seeds, 200k | fewer eggs and starving larvae on every seed, but at 0.5 one seed fell to 18 ants; on main 0.75 adults starved 53-98 -> 202-267 |
| heap 90 floor check, 0.5, playtest switches, 4 seeds, 200k | **adults starved 21-29 -> 275-575** (10-20x) while the colony grew 10-29% |
| heap 90, recruitment, 0.5, 4 seeds, 120k | idle on the mound 23-28% -> 12-17% (the lowest of any arm); larvae starved per egg 0.02-0.14 -> 0.00-0.01; one seed fell 1,041 -> 802 with 505 starved |
| recruitment + lay brake + dial | worst fall 21-198, adults starved 12-287 (worse than the lay brake alone) |

**Why it is not the answer.** The first test's success (no near-collapse at
10 cells) was against crashes that the 40-cell sweep showed to be the
**store-crumb trap**: hungry ants held beside crumbs too small to eat, 242 of
243 deaths in the worst pulse inside the nest (traced). `edible` removes the
trap (deaths in the nest 222-591 -> 1-9 a run, no seed below 221). The dial's
one consistent effect, fewer starving larvae, holds on every bed; its cost,
adults running lean and starving on endless food, is **inferred** (surplus
held as crop cargo instead of body) and not traced.

**Correction:** the session once told Scott the dial "clearly stabilizes
colonies on steady food". That was the 10-cell result alone; its own report
called it indicative, and it did not replicate at 40 cells.

**Never tested:** the dial over a long run on today's line (`edible`, recruitment,
spoil off the trail).

### 2.2 Feed first (`FEED_FIRST`)

**What it does:** no egg while a starving larva of the colony is within reach.

**Result (measured, steady food, 4 seeds, 200k):** held eggs on 1 seed of 4;
the rest bit-identical to off.

**Why it failed (traced, every starved larva):** eggs are laid at the old ground
line, brood is a powder, and a larva that starves has fallen a median 38-40 rows
down the column under the door; the nearest egg laid in its last ~7,200 frames
was a median 31 cells away. No layer ever stands near a starving larva.
`Reports/feed-first-2026-10-08/README.md`.

### 2.3 The lay brake (`LAY_BRAKE`, built 10-09)

**What it does:** no egg while the layer stands crowded (half or more of the
cells round it hold nestmates), or while its own last reading of the store was
low.

**Why it was built (traced):** recruitment's seed 1 jammed its one shaft at
37.5k: 211 outbound ants packed the door column, no food came home for 20k
frames, and the colony fell 687 -> 405.

**Results (measured, recruitment, heap 90):**

| | no brake | lay brake |
|---|---|---|
| ants at 120k (4 seeds) | 1,226-1,481 | 1,423-1,613 |
| worst fall inside 60-120k | 41-362 | 49-137 |
| adults starved 60-120k | 1-539 | 2-30 |
| eggs 60-120k | 2,057-2,426 | 2,087-2,191 |
| **adults starved 120-240k** (2 seeds) | **8-12** | **488-792** |
| store at its lowest, 120-240k | 2-9 | 0 |

A stronger crowd line (40%) was worse on every measure. The crowding cue did
nearly all the holding (22,000-32,000 checks against 13-31 for the store cue,
which a layer rarely reads).

**Why it failed (traced 2026-10-13, seed 1 rerun with full logging, same world
as the original):**

- **It brought the colony into the nest, and deep.** About 294 ants in the dug
  nest at a time against 175 without it; 45% of them 60+ rows down against 30%.
  Half of all hungry-ant decisions in the nest were 60+ rows down (52% against
  32%).
- **The ones who starved were those deep ants.** 739 starved over 150-240k
  against 8; 736 died in the nest, 534 of them nest workers. Their last 2,000
  frames were spent empty-handed in the nest, mostly 60-79 rows down, moving
  (62% of decisions were steps).
- **Nothing led them to food.** The pull that sends hungry ants out reached 6-8%
  of decisions 60+ rows down, against 26-35% near the top; 39% of their
  decisions had no pull at all. The store ran empty from about 160k
  (22 -> 0-2 cells) though more food came home than without the brake
  (trips home 85-97 against 59-65 a 1,000 frames), so the store pull, which
  without the brake carried 6% of deep hungry decisions, was gone too.
- **Ruled out (measured):** eggs laid deeper (median row 167 against 166); the
  starvers drained by feeding larvae (a median 77 J given in their last 5,000
  frames; body-feeding totals similar in both runs); the store readings the
  brake added (they are only recorded; nothing but the brake acts on them).
- **Not explained:** *why* holding eggs in crowds brings more of the colony into
  the nest. Not traced.

**Correction:** the session first told Scott the starvation came from eggs laid
deeper and nest workers drained feeding larvae, then that the store readings
were the cause. Both were wrong; this section replaces them, and the
`Reports/dead-ends.md` entry is corrected with it.

### 2.4 The food brake (`FOOD_BRAKE`, built 2026-10-02 by an earlier session)

**What it does:** raises the bar for laying as the colony's smoothed income
(food digested) falls toward its burn, and stops laying at or below a ratio.

**Results (measured, recruitment + spoil off the trail, 4 seeds, 240k):**

| Setting | Egg checks held 120-240k | Ants at 240k | Adults starved 120-240k |
|---|---|---|---|
| shipped `on` (1.0-1.5) | 162-1,213 | 1,286-1,647 | 3-71 |
| 1.5-2.5 | 32,285-157,822 | 0-0 | 74-129 |
| 2.0-3.0 | 0-264 (the colonies had stopped laying before 120k) | 0-3 | 10-26 |
| 2.5-4.0 | 0-135,520 | 0-2 | 1-132 |

"Egg checks held" counts ticks on which a layer cleared every bar but the
brake's, not eggs. Seed 1 at 2.0-3.0: 847 ants at 40k, no eggs from 40k to
100k, 78 at 100k.

**Why it failed (inferred from the code and all 12 runs; not traced per
ant):** income over burn does not move with colony size. Fewer ants is fewer
foragers *and* fewer mouths, so once the ratio sits below the stop line it stays
there and laying never restarts. Its income also counts food eaten out of the
store, so a colony living off savings reads as healthy.

### 2.5 The store brake (`STORE_BRAKE`, built 10-09)

**What it does:** raises the bar for laying as the layer's nest store falls
below `hi` cells per 100 adults, and stops laying at `lo`; colonies under 30
adults are exempt.

**Results (measured, recruitment + spoil off the trail, 4 seeds, 240k):**
0.5-3 left 25-1,548 ants; 1-5 left 0-1,277; 2-8 left 0-9; adults starved 2-718 a
seed against 3-71 unbraked.

**Why it failed (measured):** the design assumed that a shrinking colony's store
fills, so laying resumes. It did not: on seed 1 at 1-5 the store stayed 2-14
cells from 1,545 ants at 80k down to 16 at 200k, while old-age deaths ran at
600-730 per 20k and adults starving rose (27 -> 288 per 20k), because fewer
foragers bring less food.

## 3. The nest-life and recruitment switches

Scored together after the session on one setup (heap 90, the playtest
switches, seeds 1-4, 60-120k). *A* = `NURSE=workers` (only nest workers feed
larvae from their own body), *B* = `FORAGER_NODIG` (a forager the colony is
calling does not dig), *C* = `LAY_IN` (a ready layer is pulled along the nest's
way in), *R* = `NEST_REST=all` (on main). "Food/ant" is trips home with food per
ant over the window; "idle on mound" is fed ants standing on the mound (29% on
the line).

| Arm | ants at 120k | worst fall | food/ant | adults starved | larvae starved/egg | store mean | idle on mound |
|---|---|---|---|---|---|---|---|
| **the line** | 690-729 | 23-49 | 2.06-3.28 | 1-3 | 0.03-0.13 | 57-99 | 27-29% |
| A+B+C+R | 558-677 | 24-122 | 1.92-2.32 | 9-104 | 0.00-0.11 | 7-32 | 24-30% |
| without A | 613-675 | 43-195 | 1.39-1.87 | 2-102 | 0.07-0.20 | 13-32 | 25-34% |
| without B | 637-732 | 33-44 | 2.01-2.97 | 0-1 | 0.03-0.12 | 7-36 | 25-31% |
| without C | 494-619 | 35-266 | 1.48-2.11 | 1-231 | 0.03-0.15 | 10-22 | 25-31% |
| without R | 614-726 | 17-113 | 1.51-2.56 | 1-58 | 0.02-0.16 | 8-47 | 23-30% |
| B only | 581-727 | 34-100 | 1.66-1.94 | 13-132 | 0.04-0.14 | 16-25 | 27-30% |
| B + `HUNGRY_OUT=lean` | 577-773 | 70-107 | 1.40-2.00 | 36-181 | 0.16-0.23 | 3-8 | 19-22% |
| A+B+C+R + lean | 563-733 | 67-200 | 1.45-2.06 | 37-297 | 0.16-0.22 | 3-4 | 15-18% |
| `MOUND_IN` | 327-749 | 61-650 | 2.66-4.17 | 1-321 | 0.00-0.14 | 12-86 | 19-30% |
| `MOUND_IN` + B | 278-511 | 68-338 | 2.95-3.57 | 16-207 | 0.00-0.02 | 8-58 | 16-19% |
| `SPOIL_SIDE` | 620-725 | 19-52 | 1.52-2.98 | 0-6 | 0.06-0.18 | 13-107 | 26-33% |
| `SPOIL_SIDE` + B | 574-800 | 16-141 | 1.76-2.56 | 1-17 | 0.02-0.06 | 10-25 | 32-34% |
| **`RECRUIT`** | **1,226-1,481** | 41-362 | 1.96-3.99 | 1-539 | 0.02-0.14 | 22-38 | 23-28% |
| `RECRUIT` + `SPOIL_SIDE` | 1,348-1,619 | 32-75 | -- | 1-16 | -- | 5-51 (min) | -- |
| `RECRUIT` + `SPOIL_SIDE` + B | 1,047-1,123 | 38-382 | 1.19-1.84 | 25-190 | 0.08-0.15 | 7-24 | 16-22% |
| `HUNGRY_OUT=off` (base for the next three) | 306-354 | 213-320 | 5.01-5.29 | 1,131-2,317 | 0.09-0.14 | 1-3 | 7-11% |
| `STORE_READ` | 745-1,025 | 288-441 | 2.43-3.45 | 331-823 | 0.08-0.16 | 0-45 | 24-30% |
| `STORE_READ` + B | 637-754 | 310-476 | 1.40-2.03 | 606-783 | 0.09-0.11 | 6-8 | 18-24% |
| `STORE_READ` + B + rest | 572-1,072 | 38-515 | 1.07-3.17 | 269-918 | 0.06-0.14 | 0-7 | 16-28% |

(The `RECRUIT` + `SPOIL_SIDE` row is from a separate scoring of the 120k batch;
"--" marks columns that scoring did not compute.)

### 3.1 Why each was built, and what it did

- **A, B, C** came from the **doorway trace** (heap-90 stack, seed 1, 95-110k;
  traced): rich foragers were drained at the door (fed ants in the top 5 rows
  gave 9.22 per 1,000 ant-frames to larvae against an upkeep of 0.49; 63-69% of
  their nest visits turned hungry 70-75 frames in), cut the mound after
  delivering (91-93% of mound cuts were foragers'), and the egg-cap trace had
  counted 320 of 846 mound ants walking home to lay. **In combination they never
  beat the line**; the store emptied on every combination.
- **B (`FORAGER_NODIG`) halves the food that comes home (traced, 60 carriers per
  arm through every tick).** Foragers' cuts are the passages through the mound's
  food-side flank. With B the flank went solid (open inside 28-50% -> 14-20%),
  carriers stranded on it (33 -> 88 at a time), could neither hand over nor put
  down there (both happen only beside the nest), and their loads digested into
  them (median energy 884 -> 1,741 J over a trip). Seed 1, 60-100k: food home
  2,068 (recruitment) / 2,452 (+ `SPOIL_SIDE`) / 1,029 (+ B) / 1,453 (both).
  The same cause as `MOUND_DIG`'s failure (§3.2).
- **`STORE_READ`** came from the trace of who starves with `HUNGRY_OUT=off`
  (traced): 98-99% died inside the dug nest, half never having foraged, with no
  pull in 64% of their last decisions. Reading the store themselves rescued that
  colony (245-406 -> 769-1,279 ants on average) but the line's `HUNGRY_OUT=on`
  does better than either. A reading saw a median 14% of the store and read low
  60% of the time: it needs calibration.
- **`MOUND_IN`** came from the carrier trace (traced): 81% of laden foragers
  stood on the mound; they were wandering, not stuck (a step toward home open on
  81-88% of decisions, 27-29% of steps taken toward home), because the pull home
  had given up (median gain 0.05). More food home per ant (2.66-4.17), but the
  colony was smaller and boom-and-bust (one seed fell 650).
- **`RECRUIT`** came from the **brain census** (every ant on the mound with its
  Move output broken into terms; traced): the mound's idlers are never-foraged
  foragers and nest workers at a Move sum of about -0.05, whom the forage drive
  never reaches; 70% had a laden carrier within 2 cells. Recruitment counts the
  rate of returning carriers touching a newcomer, as harvester ants do. **The one
  clear gain of the session.**
- **`SPOIL_SIDE=trail`** puts spoil on the side of the door with less food trail.
  Neutral on the line; on recruitment, twice as many carriers got inside the
  mound (119 against 57 at a time, seed 1, traced).
- **`MEET_WAY`** (a carrier stamps the ants it passes on the way home) was built
  and traced (only 18-19% of mound idlers' decisions came within 3 cells of a
  delivery) but **never run as its own arm**.

### 3.2 Earlier the same session (10-07/08, on main)

- **Mound diggers (PR 664, traced):** the mound's holes are cut by idle, fed
  foragers in its hollows. **`MOUND_DIG` (PR 667, measured)**: every part that
  stops the mound digging seals the colony's passage to its food (colony mean
  268-275 -> 134-198; 101-178 adults starving inside the mound).
- **Stack leave-one-out (PR 666), store on heap 90 (PR 668), flip at heap 30
  (PR 669):** the evidence behind PR 675's default flip; the store starves
  foragers on steady food and helps on endless food. Their reports are on main.

## 4. The long-run colony and food at home

### 4.1 What the colony does to 240k (measured, 10 runs, no brake or the default food brake)

It plateaus at about 1,200-1,600 ants and drifts down 15-30% over 100k frames;
98-99% of deaths are old age. Eggs hold at about 700 per 20k, but larvae get
steadily hungrier: hungry larva-ticks per egg 112 -> 352, larvae starved 6 ->
about 110 per 20k (about 15% of eggs), brood piling up 295 -> 723 (seed 1).

**Correction:** the session first reported "crashes" of 163-556 ants over
120-240k. Those falls were measured from 40k and included the 40-60k doorway
jam; inside the window it is a slow drift.

### 4.2 Where food goes on the playtest line (measured, seed 1, 100-160k, per 1,000 frames)

| Flow | Cells of food (960 J) |
|---|---|
| put down at home straight off a trip | about 150 |
| carried into the store | about 11 |
| fed to larvae | about 39 (15 eaten from food lying beside them; the rest shared or given from adults' bodies) |
| eaten by adults (the remainder) | about 100 |

Cells are put down and picked up again about four times each near home
(deliveries 525-626 against 118-155 straight off trips, 4 seeds).

**How this fits [`follow-food-home-2026-10-10`](follow-food-home-2026-10-10/README.md)**
(a later session, main `e594d970`, every food cell followed): there the store
holds 119-213 cells on heap 90 with the stack, and 18-95 on the playtest line
with recruitment; here 57-99 on the line before recruitment and 9-38 with it.
**The low store belongs to the recruiting colony**, which carries about as much
into the store but has far more mouths. That report also traced why larvae
starve (they lie deep, and nothing brings food to them), which matches §2.3:
the deep nest has no supply line, for larvae or for adults.

**Not traced:** which adults eat the arriving food, and how hungry they are when
they do.

## 5. The lab's `DIG` brush (built 10-09, PR 670)

Scott asked for a way to dig a standing tunnel by hand. `B` then `B` arms
**DIG**; dragging clears diggable ground (soil, spoil, packed soil; never stone,
walls, plants, animals or corpses) and packs the soil round each cleared cell,
the lining an ant's cut uses. Guarded: a 60x3 tunnel stays at least 90% open
after 120 frames, and its unlined control arm falls in.

## 6. What is left open (suggestions, not instructions)

- **The deep nest has no supply line.** Both the lay brake's starving adults and
  the follow-food-home report's starving larvae die deep with food in the nest
  above them. The hungry walk-out pull reaches 6-8% of decisions 60+ rows down.
  A supply line (food carried down, or hungry ants led up from any depth) seems
  the shared lever.
- **Why the lay brake brings ants inside** is untraced, and that is the effect
  Scott wants.
- **Which adults eat arriving food**, as a step toward any rule that banks a
  surplus on a recruiting colony.
- **The brakes on finite food.** The boom-and-bust Scott saw was on finite food;
  the lay, food and store brakes were tested only with a heap that never runs
  out.
- **The appetite dial over a long run on today's line** (it does cut larval
  starvation; its cost on endless food is the open question).
- **Everything here is on main `43522586`'s game.** PR 675 turned the stack on by
  default since; a re-check of `RECRUIT` and `SPOIL_SIDE` on today's main would
  say whether the gains carry over.

Raw runs lived in the session's container and are not kept; the switch commits
and the `Reports/dead-ends.md` entries carry the numbers. The scoring scripts
were throwaway.
