# Brood in the training bed: breeding on, and why the nest still fills with food

*Brood and breeding thread, 2026-10-23/24. Owner's ask (2026-10-23 16:24Z): "I don't love that the nest is 100% full
of food." Owner rulings in the thread: brood should work as it would in the full game, with the genome chosen by
selection **at hatching**; reward food and brood kept apart, ideally in separate rooms; boom and bust is expected;
"breeding should be on by default" (2026-10-24 00:47Z). The working plan and its adversarial review are a Claude Doc,
https://claude.ai/artifact/D8oCnZn32kaD2AsV1b5uxR. This report is the record of what was built and measured.*

## What shipped

- **`examples/inrun.rs` breeds by default** (PR #839 built it, PR #845 made it the default, PR #849 folded it into the
  standard settings). `brood=lay` turns the game's own laying on in the training bed: the body lays into
  `src/sim/brood.rs` brood (egg, larva fed by nurses, pupa) and, at hatching, the harness writes a child chosen by
  selection from its sire queue into the new ant (`inrun::set_trait`, `mind::genome_of`). If the queue is empty the
  hatchling keeps the layer's mind. `brood=off` gives the old bed (headcount held by selection births with grants);
  it was checked byte-identical to the pre-brood base (seed 1, 200k frames).
- **`lay_cap=T:D`** (default `48:8`, `off` disables): the laying bar doubles for every D ants (adults plus brood)
  above T. Without it the bed booms to 150-200 ants, about 650 adults starve per round, the larder empties and
  frame cost goes up 5x. With 48:8 the colony holds 40-90 ants at about 2x cost.
- **Probes in inrun**: `brood.csv` (laid, hatched, brood on hand), `occupants_f*.txt` beside each nest map,
  `larder.csv` (stored kJ by kind: provisions, crumbs, banks), and `PP_HUNGRYPROBE=1` → `hungry.csv` (for each hungry
  ant: crop, jaws, food beside it, pick-ups it was offered, gut, last verb; `mind::last_verb_of`).
- **`PP_HOMEEAT=H` crutch** (`creature::options::home_eat_crutch`, off by default): a hungry ant in the nest bowl
  (within 12 columns of a nest site, below its surface) eats from stored food. **Judged not needed and not used**:
  in the colsel `brood1` batch the crutch arm dug craters of 200-371 cells; without it, 3-8.
- Tools: `scratch/inrun/brood/garden.sh` (one evolved colony on the garden bed, `DIST=` moves the food) and
  `scratch/inrun/brood/separation.py` (how close brood sits to stored food, against an open-cell control).

## What the packed nest is

The stored food filling the nest is almost all **crumbs**: part-eaten provisions put back at home, about 0.5 kJ a
cell, worth carried in aux, never rotting. Whole provisions are 0-2 cells of it.

**The crumbs are the colony's working larder, not dead weight.** 75-83% of all digestion happens in the nest, and it
is eaten at the surface of the pile while the buried part keeps growing. This was first read the other way (that the
pile is never eaten) and corrected by experiment: a switch that let crumbs rot emptied the pile from ~300 cells to
about 0 by 100k frames and killed 4 of 4 colonies, against 2 of 4 alive with it off. The rot switch was reverted;
`Reports/dead-ends.md` carries the entry (PR #850).

## Why breeding does not empty it

Breeding is stable under colony selection and roughly halves the store (colsel `brood1`, 64 colonies: store at round
31 was 260 cells against 514 without breeding; gut reserve 28-39 grants against 107; food home and digging held, 0-3
of 64 extinct; the common-garden figures are over surviving colonies only, since `cg.csv` skips a colony that died out, but
with 0-3 of 64 lost the bias is small). But the nest is still packed, because **the size cap stops laying while income is far above upkeep**:
at about 87 ants the laying bar is 32 times its base, so surplus food has nowhere to go but the pile.

Everything tried on the supply side either leaves the pile or kills the colony:

| Lever | Result (4 evolved colsel20 carries on the garden, 200k frames unless stated) |
|---|---|
| looser cap `lay_cap=64:8` | 2/4 alive, crumbs 373-491 |
| looser cap `lay_cap=96:8` | 0/4 alive, 94-240 crumbs left |
| crumbs rot | 0/4 alive, pile ~0 |
| `lay_reserve=3` (lay only from store) | 0/4: inherited fat banks (~87 grants) pay for a boom |
| plant-food bed (limited income, `broodplant1`) | nest still packed |
| store pays per birth (`colsel store=4`, earlier batch) | larder 36 against 452, but craters 310 against 10 |
| food moved from 25 to 45 columns | 2/4 alive; the survivor still packs (c59: 422 crumbs); the weak die sooner with 87-113 left |
| food at 65 columns | 0/4, all dead by 50k frames (46-198 deliveries in total) |

Live ants at 50k/100k/150k/200k, by food distance (garden, breeding default, one seed per carry):

| dist | c3 | c27 | c43 | c59 |
|---|---|---|---|---|
| 25 | 67 71 1 0 | 86 83 90 90 | 72 20 0 0 | 69 84 81 81 |
| 45 | 51 53 16 10 | 64 24 0 0 | 69 2 0 0 | 64 69 70 72 |
| 65 | 1 0 0 0 | 3 0 0 0 | 1 0 0 0 | 2 0 0 0 |

So the training ants cannot forage much past their training distance (25 columns); a food-distance ramp has to train
them in steps of about 20 columns, not jump. Distance shrinks the pile mainly by killing colonies.

**Starving beside food.** On the evolved carries, when a colony does starve it dies with 106-188 kJ of crumbs left. The
hungry probe found starving ants at home touching crumbs, crop empty, jaws free, choosing Walk 200 times out of 214
and PickUp 4: the evolved mind never learned to eat at home, because the old bed never let an ant go hungry. Late in a
collapse 78-81% of hungry ants are on the mound, carrying soil pellets.

## Brood and food in separate rooms

`separation.py` on capped evolved nests: 1-3 brood cells at a time, and 73-100% of every open nest cell already
touches food. Separation cannot be judged, or selected for, until the food is limited; it waits on the levers above.

## What is left, and what the owner has been asked

- The remaining levers are behavioural, not supply: (a) a selection case that ranks down colonies that starve beside
  stored food (teaches eating the buried pile), or (b) stop part-eaten loads being put back at home. Asked of the owner
  2026-10-24, not yet answered.
- Open questions in the plan doc: the shipped Storeroom caste (`caste=4`) is a hand-made caste and conflicts with the
  no-castes rule; how much starvation is acceptable; a training headcount cap near 50.
- Review cautions still standing: a cap hides surplus in gut banks, so judge reserve as store plus banks in kJ; 51-61%
  of starved adults die before `min_age` (newborns carry about half a grant); culls outnumber starvation; 14x more
  births per round raises the mutation load, a confound when comparing against `brood=off` arms.

## Learned laying: a brake the colony can evolve (2026-10-24, later)

Asked for by the Teacher lane after the far64 pool boomed to ~710 ants in the owner's lab box and then starved 817 in
10k frames with 2.4 MJ of food still in the box. The lab has no `lay_cap`, and a mind ant's laying was the body's
alone. The owner asked whether the brood rules are in the genome and said they should balance through training,
and ruled that the brake may read only what the layer itself can sense, not the colony's store.

- **`PP_LAYGENE` (on by default, `=0` for none)**: six genome slots after the dig head (`mind::LAY_LEN`) that raise
  the ant's own laying bar by about `2^-x`, up to 64 times, where `x` is a bias plus weights on the layer's own crop,
  free food cells, nestmates and brood within 6 cells, and how much nestmates have fed it lately (the real queen's
  signal). The bias alone is the `TRAIT_REPRODUCE_AT` arm (`PP_LAYGENE_SLOTS=0`). The gate slots mutate on their own
  schedule (`PP_LAYGENE_PM`, `PP_LAYGENE_SIGMA`); a founder without them gets zeros (`PP_LAYGENE_INIT`).
- **Neutral as it starts**: with selection off, a zero gate is byte-identical to main, and so is `PP_LAYGENE=0` with
  selection on. With selection on, the gate's mutation draws change which children are picked, and nothing else.
  Pools saved now carry the gate (genome 3285); the lab keeps it (`training::fit_head`).
- **`lay_cap_p=P`** in inrun scales the cap's exponent, from 1 (as before) down to 0 (no cap). **colsel
  `capwean=R0:R1`** fades it over rounds the way `wean=` fades the body rules, and runs the common garden at 0, so
  colonies are judged on their own brake. It stays opt-in: an uncapped bed booms to ~170-200 ants at about 5x frame
  cost.
- **A hold does not brake, and neither does a hand-set bar.** The first gate held the egg, as the game ant's `Lay`
  output does. A ready ant just lays a few ticks later, once its surroundings change. Uncapped bed, std_pool, seed 1,
  adults at 20k-60k frames: 192-161 with no gate, and 178-195 at their peaks with holds keyed on crowd, wealth or
  being fed. A crowd trace found 43% of gate reads passing, from the same nest cells as the holds. A bar a rich ant
  can reach only slows the climb to the ~150-230 ants the bed's food carries. With bias -3 the colony went
  24/47/94/145/194/198; crowd-keyed exponential bars boomed and crashed (208 then 90). The open question is whether
  selection finds a brake that hand-setting did not. Jobs: `scratch/inrun/jobs/laygene1.txt` (gate, bias only, and no
  gate, all with `capwean=4:16` and the lab case). Dead end recorded: `Reports/dead-ends.md`, `lay_gate`.

**Result, laygene2-1024 (Teacher lane, 2026-10-24 evening): selection found no brake.** Founder ls12, 32 colonies x 24
rounds, `capwean=4:16`, common garden at `lay_cap_p=0`. Round 23: gene on (lg) 175 ants in the garden, ~960 starved
per round, nest 144, lab case 1,244; no gene (lgctl) 182, ~1,030, 114, 1,289; cap kept (lgkeep) 80, 63, 317, 1,210.
Paired lg against lgctl, lg ahead on the lab case in 13 colonies of 32 and on garden alive in 14: no win. The gate
slots did drift (medians bias -0.03, crop 0.03, food -1.36, crowd 0.08, brood 0.65, fed 0.86, spread about +-2), so
the genes were moving, but nothing selected them toward restraint. Uncapped colonies settle at 160-200 ants with
constant starvation churn and do not die out (0 of 32 in lg), so **the bed never presents the lab box's failure**
(a boom to ~710 and a crash with food left): a colony that starves a thousand a round and refills is not ranked down
by any case, and the lab case moved little. The Teacher lane keeps `lay_cap` in training; the gate stays on by
default, neutral at zeros. *Next, if retried:* a colony case that pays for low starvation per birth, or a bed where
overshoot kills (food far, as in the lab box), so restraint has something to buy.
