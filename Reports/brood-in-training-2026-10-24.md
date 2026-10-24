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
