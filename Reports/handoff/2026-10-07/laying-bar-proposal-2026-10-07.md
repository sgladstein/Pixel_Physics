# Egg-laying bar: proposal (Nest race, 2026-10-07 02:45)

**For review by another lane. Nothing here is built. Scott chose "Write proposal" on the decision card at 02:31; he reads this and the review together before anything is built.**

## The defect (traced twice, measured)

**What the code does.** Today an ant lays when this holds:

> its own energy + the food within reach of its head ≥ `lay_at` (1,100 J)

This is `try_bud`, with the `reachable_provision` sum. But `lay_egg` charges the 120 J egg to the ant's **body**. Food in reach pays only when the body holds under 121 J (`shortfall` in `brood.rs` `lay_egg`).

**What that means.** The food that cleared the bar is never spent. An ant holding 200 J, standing beside 900 J of store or heap food, lays and is left with 80 J. It can repeat this as long as its body holds 121 J or more, and the food stays where it is to qualify the next ant.

**Why it contradicts the design.** The authored intent in `assets/species/ant.ron` is: "a breeder at the 1,100 J bar keeps a ~1,000 J reserve and lays one egg per 120 J it earns above it." With food counted, a breeder can lay with almost no reserve.

**Where it has fired (measured):**

1. **Laying, arm 3 (300k).** Held-in ants dug to the heap and laid there. 73% of seed 2's eggs at 50-100k came from layers left under 200 J, against 2% with the store off. The colony boomed past the heap's refill (216 cells per 1k frames) and starved. The store-off arm's seed 4 die-off was the same chain. (`store-arms/arm3/read-300k.md`)
2. **These storeroom runs** (`store-arms/sky-meal/laying.txt`, paidby, 40-150k). Share of eggs laid by ants left under 200 J:

   | Arm | Share |
   |---|---|
   | store off | 2-4% |
   | store on (fixb) | 30-35% |
   | store on (skymeal) | 10-21% |

   Fixb boomed and starved without the nest reaching the heap. In skymeal seeds 1 and 3, the nest broke through to the heap and heap laying went from 0 to 400-680 eggs per 10k. Starvation followed once the heap was bare: 816 and 2,338 starved.
3. **Deep trace, arm 2c (300k).** On seeds 2-4 the far-west deaths began with a boom: births 1,149-2,006 per 25k, against 245-481 before. The boom ate the heap bare (`store-arms/arm2c/farwest-who-300k.md`).

**So any storeroom makes the colony out-breed its food.** That is whatever the store does, because a store is food in reach of ants who are mostly not rich.

## Proposal: `LAY_BAR=body` (one switch, off by default)

**For a species that lays eggs** (`brood` set), the laying bar counts only the ant's own energy:

> lay when energy ≥ `lay_at`

**What does not change:**
- Budding species (no `brood`) keep the food-in-reach rule, which `try_bud`'s comment defends for solitary animals.
- The store-paid birth path (`bud_from_store`).
- `lay_egg`'s payment.
- The egg's price.
- The bar itself (1,100 J).

**Why this one.** It restores the authored "keeps a ~1,000 J reserve". It is local: the ant reads only its own body. Food still caps births, because a rich body comes only from eating. A store or heap then feeds ants, and the fed ants lay.

**What it costs the shipped game.** Little, on these runs. With the store off, 94-97% of eggs already come from layers who still held ≥1,000 J after laying, so they cleared the bar from their bodies. The rule changes only the 2-6% that qualified on food.

## Considered and not proposed

- **Food in reach pays first** (spend the food that cleared the bar before the body). This keeps the general rule, but turns a store or the heap straight into eggs. The store drains into brood, and a colony at the heap still out-breeds the refill. It is the same boom, paid by food instead of bodies. (Inferred, not run.)
- **A colony-wide food brake** (`FOOD_BRAKE`, off). Global, and it is on the off list.
- **The `Lay` brain output as a brake.** It is inert, and moving its hold line did not brake the crash (`lay-hold-line` note, 2026-10-03).

## Risks the reviewer should weigh

- **Fewer births everywhere food lies near ants.**
  - Store-on arms lose 30-40% of eggs if nothing else changes (fixb: 37-40% of eggs from ≥1,000 J layers).
  - Colonies might then be too small to hold a store. Colony size is a floor, not the measure, but it is a floor.
- **Constants calibrated with the food count in play** (the CLAUDE.md "fixing a bug exposes a constant" rule):
  - `egg_frames` 250 was chosen on lab births measured under today's rule.
  - The `LAY_HOME` walk-home pull reads `energy >= birth_bar` from the body only, so it already assumes body-only. That is one more sign the reach count was not intended for eggs. (Inferred from the code.)
- **The lab box (`--bin lab`)** may depend on food-paid laying more than the deeptrace box does. Measure it there too.

## Test, once Scott has read this and the review

- Arm `LAY_BAR=body` on:
  - store off;
  - skymeal (`NEST_STORE=on,pick=20,jaws,sky,meal`, plus `smell=10` if the rerun under way supports it).
- Seeds 1-4 to 150k, then 12 seeds before any default.
- **Lead with:**
  - eggs by layer energy (paidby);
  - eggs laid at the heap (heaplay);
  - births per 25k against the heap's 216-per-1k refill;
  - heap bare or not (heapground).
- **Then:**
  - starvation and where it happens;
  - deep ants as N of M (door column vs off; fed and staying);
  - food fate.
- Trace each death cluster before calling the result.

## Corrections after Laying's review (02:45)
- **The bar is 946 J for the evolved founder** (1,100 x the trait's 0.86), not 1,100 J.
- **Founding:** 17-33% of eggs at 6-20k needed food in reach. Add births by 20k and 50k to the test, and report against a slower start.
- **Not removed:** rich foragers laying at the heap once the nest reaches it. That is about two thirds of off seed 4's boom. It is food-backed and self-limiting.
- **The store with `sated`:** nest ants never get fat, so a full store adds no births. The test names the store setting: skymeal (no `sated`).
