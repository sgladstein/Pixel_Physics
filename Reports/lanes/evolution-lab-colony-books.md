# Lane B — the colony's books

*Round 35, 2026-09-14. Brief: `Reports/colony-food-economy-design-2026-09-14.md`
§3, Lane B. Branch `claude/evolution-lab-colony-books`, **PR #419**, head
`4254ba97`. The lane opened its own PR — it has the GitHub tools — so the
coordinator owns only the merge.*

## What landed

Three commits, each standing alone.

1. **`World::colony_books`** — `EnergyLedger`'s accounts kept per
   `OrganismState::colony`, plus the diet band and a raid pair. Guards in
   `src/sim/creature.rs`.
2. **`examples/colonybooks`** — read the books before drawing them.
3. **The lab's FOOD page** — `Panel::Food` in `src/lab/ui.rs`, reached from
   ANTS and MENU.

Plus a README status section, a row in `Reports/instruments.md`, and a FOOD
tile in `examples/labui`.

## The three findings a later session should not have to re-derive

**The design trap in the brief was real, and here is its size.** Splitting a
closed ledger per colony breaks closure unless transfers between colonies are
their own account, and the transfer is **trophallaxis**: `neediest_kin` goes
through `creature::is_living_kin`, which is *species* identity unless
`World::colony_rivalry` is on — and §5 of the design report says the shipped
bed holds no rival colonies at all, only one extended family, which is exactly
why the sharing crosses. Measured on the guard's own bed over 12,000 frames:
colony 1 gave 1,122.75 J and took 1,207.65 J, **84.9 J across the line**.
Remove the `SharedOut`/`SharedIn` pair and `the_books_close_for_every_colony`
goes red at 243.03 held against 158.10 booked, against a world-ledger drift of
**0.049** — three orders of magnitude, which is the gap a free term would have
been invented to close.

**The colonies do not feed themselves, and this is the headline the books
were built to produce.** Shipped bed, seed 1, two colonies of 8 founders, over
20,000 frames: ANT 1 took in 10,400 J of founding grant and foraged **399 J**;
both colonies together foraged **485 J against 18,800 J granted — 2.6%**. They
ate 84% and 95% **corpse**. That is the mechanism under "70–90% of ant deaths
in the lab bed are starvation", in joules: the colony is not failing to find
its dead, it is that plant income is approximately zero. Walking is 27% of
everything spent; brains 8.3%; upkeep 46–48%.

**A scene fact that cost a measurement.** Found the two colonies **30 cells
apart, not 120**. At 120 they never meet, every share stays inside one colony,
and the cross-colony transfer the whole split was designed around is untested
while its equality passes perfectly. The guard bed founds at 85 and 115 and
asserts the crossing happens.

## What this cost, and what it did not

**Nothing measurable in frame cost.** `antcost ants=0,400 frames=200 reps=3`,
`RAYON_NUM_THREADS=4`, four alternating paired runs of two fixed binaries:
per-ant 3.24 µs before against 3.20 after (medians), inside a before-arm
spread of 2.93–3.49 on its own. **The world hash is bit-identical in every
arm** (`0x5b579f9aaadf8840` empty, `0x557d82fc0aebf6cb` at 400 ants) — this
books what already happened and changes no behaviour.

## Sensitivity, since green was going to be cited

Every guard here was written after the code, so each had the fault put back:

| mutation | what goes red |
|---|---|
| book every colony to bucket 0 | `the_books_close_for_every_colony`, `the_diet_band_sums_to_the_harvest_accounts`, `the_books_are_not_all_zero` — and `every_account_sums_over_the_colonies` stays **green**, which is the pair being independent as intended |
| drop the material record in `book_meal` | `the_diet_band_sums_to_the_harvest_accounts` |
| remove the `SharedOut`/`SharedIn` pair | `the_books_close_for_every_colony`, by 84.9 J |
| remove the page's fit test | `the_food_page_stays_on_the_screen`, 241 px of rows against a 228 px budget at 3 colonies |

## Open, and deliberately not done

- **No key opens the FOOD page.** The bar has been measured full twice and
  `src/bin/lab.rs` was Lane A's file this round. One line there (`F6`, the next
  free stop after `F5`) finishes it.
- **`CARRYING` is a standing count**, so per the design report §4 it cannot
  tell a store from a conveyor. It is labelled as food *in transit* and says
  so in its note. **Do not build a harness for the store question — one
  exists**: `larder_probe mode=turnover` tracks the band as a set of
  positions and reports entries, exits and residents, and is what turned "a
  granary of ten cells" into "ten cells in transit", `resident` 0 from frame
  200. The genuinely unmeasured thing is dwell time *inside a crop*, which is
  what would separate a carrier that is ferrying from one that is hoarding;
  it wants a frame stamp on `Crop` and is not in this branch.
- **Meat is not attributed and must not be without a label to attribute it
  by.** A corpse cell has nowhere to carry a colony — `Cell::aux` is its
  worth. Colonies pay into one world pool and draw out of it; `meat_lost`
  stays world-wide. Inventing an attributed meat account is how the 300
  conjured joules happened the first time.
- **The raid pair books only where a mouth swallowed.** The `Attack` verb
  destroys flesh without gaining it, and counting it would make the two sides
  of a matched pair stop matching; `CreatureStats::attack_cells` is that
  number.

## After #417 landed — a stranger is food, and the page says so

The coordinator's sequencing message arrived after this branch had already
merged `main` at `c5a77513`, so the merge it asked for was done (that merge
is where the README conflict below came from). What was **not** done was the
question inside it, and it was a real gap.

**#417 makes a stranger edible through the ordinary mouth** — `ant` material
carries `food_class: 1.0` against the shipped neutral gut, so two colonies
outside each other's tolerance eat each other with no `Attack` weight
involved. In these books that lands in **`HarvestedPlant`**, because living
flesh is not `worth_in_aux`. Arithmetically right, and on the page it read as
*"they found some plants"*.

Two things came out of checking rather than assuming:

- **`raided` does fire on the new path**, and the guard for it is a different
  door from the beetle one: there `is_living_kin` is false because a beetle is
  not an ant, here both are ants and the predicate turns on **smell**.
  `a_stranger_colonys_ant_is_booked_as_a_raid_and_as_food` asserts the pair
  *and* that the mouthful is filed under the `ant` material, which is the half
  the page needs — a raid that books joules without naming its source draws as
  an unexplained rise.
- **The obvious scene was the wrong one.** Making the eater hungry, on the
  reasoning that `Feed` is an urge, starved it: at a quarter bank it walked off
  looking for food and was dead inside the window, and the null read as
  "strangers do not eat each other". The hunger wire makes a *full* ant rest,
  so two rich strangers stay adjacent long enough for the mouth to find flesh
  that is already touching it — 163 eats and 54 cells taken, against zero.

The page gains one conditional row, `ATE RIVALS / EATEN BY`, inserted third so
the block still reads bank → sources → bill → state. It is only drawn when it
is happening, and the page guards book a raid for every colony precisely so
the fit test measures the **tall** block rather than the short one.

**And a stale reference this found, which is the more useful half.**
`World::colony_rivalry` is **retired** — the live mechanism is the narrow end
of `Behavior::scent_spread`. Three doc comments named the retired switch as
the live rule, one of them `OrganismState::colony`'s own, which is the doc
anybody asking "what is a colony" reads first. All three now say what actually
decides it, with the switch kept only as history.

## The review card

`20260914T063026275Z-e7b8f3` (board `lab`) — the FOOD page at rest, asking
whether it reads cold and what is missing. **Unanswered as of this writing;
read it with `review.py get 20260914T063026275Z-e7b8f3`, never off `inbox`.**
