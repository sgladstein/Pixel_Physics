# Review: the egg-laying bar proposal (`LAY_BAR=body`) (Laying, 2026-10-07 02:40)

Reviewing `/mnt/project-files/nest-race/laying-bar-proposal-2026-10-07.md`. All numbers below come from Laying's saved
runs: arm 3 seeds 1-4 to 300k; the off arm's seed 2 to 60k and seed 4 to 169k. Those reruns match Nest race's `off/`
row for row. No new runs were made.

The new tool is `bodyonly.py`, in `store-arms/arm3/why/`. For every egg it asks whether the layer's body alone cleared
the bar: energy just after laying, plus the 120 J egg, at or above the bar. It cannot say what an ant refused under
body-only would have done instead (eat more, then lay later). So "eggs it would block" is an upper bound on what
the switch removes.

## Verdict

**Build it as proposed, behind its switch. It removes the part of the chain that kills.** What it does to that chain:
- **Removes:** the births that continue after the heap is bare, and the births by hungry ants beside a store.
- **Leaves:** the boom that rich foragers cause by laying at the heap once the nest reaches it. That one is
  food-backed and self-limiting, and on the off arm's seed 4 it is about two thirds of the boom.
- **Gaps to fix before the test:** two gaps in the proposal (founding, and the store with `sated`) and one wrong
  number (the bar is 946 J, not 1,100 J).

## 1. Does it fix the chain? Mostly, and the part it misses is named

Eggs that needed food in reach to clear the bar (`bodyonly.py`, evolved founder's bar 946 J):

| Run, window | Eggs | Needed food in reach | Of those, at the heap |
|---|---|---|---|
| arm 3 seed 2, 30-55k (heap still full) | 1,934 | 1,283 (66%) | 521 |
| arm 3 seed 2, 55-150k (heap bare) | 8,239 | **8,042 (98%)** | 4,874 |
| arm 3 seed 2, 150-300k | 10,261 | **10,047 (98%)** | 4,357 |
| arm 3 seed 1, 240-300k (heap bare from 250k) | 4,970 | **4,526 (91%)** | 2,056 |
| off seed 4, 130-169k (heap not bare, 86-114 cells) | 2,238 | 825 (37%) | 245 |

- **The bust: fixed.** Once the heap is bare, 91-98% of eggs come from ants whose bodies could not clear the bar.
  The freshly dropped 54 cells, or nest food, qualify them, and they lay down to near nothing. Body-only stops
  these. Without them seed 2 lays about 2% as many eggs during its die-off, so the young ants born into famine,
  half of seed 2's starvers, are not born. This restores the brake the proposal names: no fat body, no egg.
- **The boom at a full heap: only partly fixed.** Rich foragers who eat at the heap and stand within 4 steps of a
  nest cell still qualify, and lay there.
  - Off seed 4 at 130-169k: 1,413 of 2,238 eggs (63%) would still be laid, 964 of them at the heap by 291 layers
    (up to 23 eggs each).
  - Arm 3 seed 2 at 30-55k: 318 heap eggs would still be laid.
  - So contact with the heap still roughly doubles laying (off seed 4: about 180 eggs per 10k before contact,
    about 360 per 10k on body-qualified eggs after).
  - This boom stops by itself when the heap can no longer fatten ants. Off seed 4's 701 deaths at 140-169k came
    with the heap still holding 86-114 cells, and Nest building traced them to the way-out map. So they are not
    this rule's.
- **Upstream links untouched:** held-in ants digging 3.5-11x more, and the nest reaching the heap. Body-only is right
  not to try to fix those, but the test should still count heap contact.

## 2. What could it break

- **Founding: a slower start, not a stall. The proposal does not mention this.**
  - In the first 14,000 frames of the colony (6-20k), 17-33% of eggs needed food in reach (off s2 17%, off s4 30%,
    arm 3 24-33%). Afterwards it falls to 4-5% (off s2 20-60k, off s4 20-130k, arm 3 seeds 3 and 4).
  - The first egg on off seed 4 (7,654) came from an ant at 878 J, which needed 68 J more.
  - 68-80% of founding eggs come from fat ants on the mound top, so laying starts anyway.
  - **Measure:** births by 20k and 50k, and the founding die-off, beside the off arm.
- **The store with `sated`: births beside a full store go to zero.** That may be what Scott wants, but it is his
  call.
  - Under body-only, a nest ant lays only if its body reaches 946 J.
  - With `sated` (fed ants leave home food alone), nest ants never eat past their grant at the store. So no store,
    however full, raises births. All births then come from foragers who fattened at the heap.
  - Without `sated` (skymeal), ants eating at home do fatten: Nest race's seed 1 trace has doorway eaters at 5-6x
    their start energy. So a rich store still raises births, at one egg per 120 J above an 826 J reserve.
  - If Scott wants a rich store to drive births directly, the existing `BUD_STORE` path does that and the proposal
    leaves it alone.
  - **The test should state which store arm it uses and report births beside the store.**
- **The number is wrong, not the logic.** The evolved founder's bar is 1,100 x (1 + TRAIT_REPRODUCE_AT -0.14) =
  **946 J**, not 1,100 J. The reserve kept after an egg is about 826 J, not about 1,000 J.
- **When mutation is turned on, the reserve becomes a gene again.**
  - Today any food in reach bypasses `TRAIT_REPRODUCE_AT`. Under body-only the trait decides.
  - Selection may push it down: the floor is the birth cost plus 1. A lineage could then evolve back towards poor
    layers, in its own bodies.
  - That is the design's stated intent, "a gene rather than a constant". It needs watching once mutation is on.
- **Code:** for a laying species, `eat_toward_birth` stops being reached on the lay path, because a body at 946 J
  has no 121 J shortfall.
  - `a_funded_colony_buds_and_an_unfunded_one_does_not` (creature.rs:48417) covers budding. Check whether it runs
    with brood on.
  - `brood.rs` tests call `lay_egg` directly and bypass `try_bud`, so they will not see the change.
  - The new test should watch red: an ant at 300 J beside 1,000 J of food lays under today's rule and not under
    `body`.

## 3. A cheaper or more local cue?

- **Body-only is already the most local cue there is:** the layer's own body.
  - The change is one condition in `try_bud`, for species with `brood`.
  - It matches `ant.ron`'s stated intent ("keeps a reserve") and `LAY_HOME`'s existing body-only read.
- **I agree with the rejected options.**
  - Food in reach paying first would let every ant at a full heap clear the bar whatever it holds. That is a boom
    limited only by the refill (inferred).
  - `FOOD_BRAKE` is colony-wide.
  - The `Lay` output is inert.
- **One middle option, not recommended:** keep counting food in reach, but only for a body that still holds its 200 J
  grant after the egg.
  - On seed 2 at 55-150k, that blocks the 76% of eggs left under 200 J, but lets 23% (layers at 200-1,000 J) through.
    It is a weaker brake with an extra constant.
  - Its one gain is a smaller founding cost: 5-14% of founding eggs, against 17-33%.
- **If the full-heap boom still matters after the test, the next local cue is where the egg may land, not the bar.**
  - Rule: an egg may not go into a nest cell within reach of open ground.
  - The proposal does not cover this, and it should not be built before body-only is measured.

## 4. Changes to the test plan

1. Add **founding**: births by 20k and 50k, the first egg's frame, and the founding die-off, against the off arm.
2. Add **heap contact**: the frame at which heap laying starts (`heaplay.py`), and eggs per 10k laid at the heap by
   body-qualified ants, so the leftover boom is measured rather than assumed.
3. Use **946 J** in every bar read, and name the store arm's `sated` setting.
4. The off-arm seed 4 rerun to 169k is in Laying's container if a paired trace is needed.

Files: `bodyonly.py` is in `store-arms/arm3/why/`. The read behind sections 1 and 2 is `store-arms/arm3/read-300k.md`.

## 5. Addendum (02:47): Nest race's two further questions

**The verdict does not change.**

### Which constants were tuned while food in reach counted

- **The evolved founder's six traits** (`src/lab/scene.rs` `LAB_ANT_TRAITS`), including the laying bar's
  `TRAIT_REPRODUCE_AT` -0.14 (946 J).
  - They evolved in the no-plant goal box under today's rule.
  - The note there says that putting the bar alone back to `ant.ron`'s "killed seed 1 outright".
  - Under body-only the bar does more of the braking. If births stall in the test, check this trait first. (Inferred.)
- **`egg_frames` and `pupa_frames` 250** (`ant.ron`).
  - Picked on lab births and colony survival, measured under today's rule: 250 against 100 and 500.
  - The note says 100 lifted births but the colonies "crash harder". That crash may be the same boom. (Inferred, not
    checked.)
  - If body-only slows births, re-sweep these before blaming the bar.
- **Not tuned against the reach count:**
  - `egg_cost` 120 and `lay_at` 1,100 are marked "first guesses". Their comment states the body reserve that the
    proposal restores.
  - `LAY_HOME` already reads the body.
  - The egg pile reach (4) decides where an egg goes, not whether it is laid.

### Does the lab box rely on food-paid laying? No, not on a planted bed

The saved runs could not answer this, so I made one new run:

- **Setup:**
  - the played bed (`played_bed`: plants, no heap, `food=0`);
  - shipped switches (every `PIXEL_PHYSICS_*` unset);
  - the evolved founder;
  - seeds 1-4 to 60k;
  - the same binary (c9e8a860b).
- **Reader:** `store-arms/arm3/why/bedread.py`. Table: `why/bed-60k.txt`.

| Seed | Eggs that needed food in reach | Eggs that left the layer under 200 J | Founding, to 10k | Starved by 60k |
|---|---|---|---|---|
| 1 | 10 of 482 (2%) | 0 | 6 of 8 | 63 |
| 2 | 3 of 487 (1%) | 0 | 1 of 25 | 110 |
| 3 | 39 of 474 (8%) | 0 | 4 of 25 | 114 |
| 4 | 12 of 501 (2%) | 0 | 2 of 17 | 147 |

- **A planted bed already lays from bodies.** Body-only would cost it a few eggs.
  - Founding is slower only on seed 1, and by a handful of eggs.
  - The worst stretch is seed 3 at 40-50k: 19 of 51 eggs, mostly layers in the spoil mound.
- **Food-paid laying fires where food lies in a pile beside the ants:**
  - at the heap (the player's food spot), as in arm 3 and the store-off seed 4;
  - at a store.
  - That is the case the proposal is for.
- **The bed still starves 63-147 adults by 60k** while its layers are rich. The bar change will not touch that. It is
  not traced here.
- **The tool can see the effect:** the same test (needed food in reach, at 946 J), on the same binary, reads 98% on arm 3 seed 2 at 55-150k and 37% on store-off
  seed 4 at 130-169k. So the low numbers here describe the box, not a blind instrument.
