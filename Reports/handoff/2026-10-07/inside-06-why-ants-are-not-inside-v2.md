# Why ants aren't inside the nest (v2, reviewed)

Lane 20 (Nest race), 2026-10-06 00:45. This replaces `02`. It folds in three reviews:
- Nest building: `03-review-nest-building.md`;
- Deep trace: `03-review-deep-trace.md`;
- Laying: its note on nurses, relayed 00:22 and recorded in `04` under "Laying's note".

It also folds in Scott's 00:17 ideas compared with biology (`05`), and his 00:18 choice of a **storeroom**. **Nothing is built.**

**Marks:**
- **traced**: per-decision or per-ant rows.
- **inferred**: reasoned.
- *agreed by* or *disagreed by*: which lanes checked a claim, with their own numbers where they gave them.

Main b081040e (Deep trace checked its 01320ff7-plus-nurses-off build row for row against it), dry goal box, evolved founder, mutation off, seeds 1-4.

## The explanation

**The colony lives where its food is handed out, and that is not inside.** Food reaches ants on the mound and in the top 5 rows of the shaft. The rules that move ants all point out of the nest or stop at its door. So the nest is a place ants pass through.

### 1. No ant lives inside (traced; agreed by all three)
- 2-14% of ants are underground at any 10k census (mine). Deep trace measures 4.6-5.9% at 100-240k.
- 7-18 of about 2,000 ants spend half their time underground (Deep trace).
- **Correction (Nest building):** 28% of my "entries" were soil carriers bouncing back in within about 10 frames. With re-entries under 50 frames merged, a stay lasts a median 190-305 frames for foragers and 280-360 for nest workers (p90 975-2,040). That replaces 02's 40-85.

### 2. Newborns hatch hungry and are walked out (traced; agreed and re-checked by Deep trace)
- Ants hatch in the nest at 0.40-0.53 of the starting energy.
- Their first real exit (bounces merged) comes a median 435-705 frames after hatching. HUNGRY_OUT is the pull on 54-66% of those exits (mine).
- Deep trace measured the same thing from hungry ants' decisions: 92-97% of newborns that are ever hungry leave still hungry, and HUNGRY_OUT is the pull on 70-78% of their last decision inside.
- Nest building: the code agrees. A newborn passes HUNGRY_OUT's gate on its first step.

### 3. Every stay ends on a pull that points out (traced; agreed by Nest building)
With bounces merged:
- **HUNGRY_OUT ends 40-44% of stays.** Half of those are ants that came in fed, gave food away and turned hungry. The other half came in hungry.
- **The soil way out ends 36-39%.** These are diggers carrying a pellet out.
- **No pull ends 15-22%.** These ants wander out fed.

Detail:
- **A fed ant that comes in empty-handed** stays in the top 5 rows (62-67% never go deeper), turns hungry, and is walked out.
  - The morning trace measured 55-70 frames to turning hungry; my 205-235 frames is to the walk out. Deep trace says the two fit.
  - The sharing that empties it is from the morning trace on an older game and **was not re-traced today** (Deep trace).
- **Soil carried in through the door** splits two ways (Nest building, seed 1):
  - 1,631 of 2,881 are the bounces above.
  - 1,155 are pellets cut in the mound's tunnels and carried for a median 1,745 frames, mostly by foragers. They are lost soil carriers. **Why they are lost is open:** Deep trace says nest scent does not hold soil carriers (inferred from the trail weights).
- **Inside is not what drives them out.** An idle fed ant with no pull steps down 55-57% of the time and up 20-22% (seed 1).

### 4. Outside, nothing points in, and the fog is how the colony eats (traced except where marked)
- **Every home target from outside is the door anchor.** That covers laden home, the leash, the laying walk and the rest pull's outside leg (Nest building).
  - "Walk home to lay" never acts underground: 0 of 415-584k decisions (mine).
  - **Inside, `NEST_HOME=dug` makes the whole dug nest count as arrived.** Any home pull ends at the first dug cell (Nest building's sharper wording, adopted).
- **Nest scent sits at 0.98-0.99 across the whole mound**, so it holds a food carrier's heading about four times over. 84-89% of laden carries lose the pull home (Deep trace).
- **That fog is today's feeding system** (Deep trace):
  - Lost carriers are the heap eaters: about 100 decisions at the heap per carry, and 44-52% fill their crop.
  - Hungry ants on the mound are fed where they stand; under 1% of mound hunger spells reach the heap. That they are fed by passing carriers is *inferred*.
- **Fed ants on the mound stand on 66-73% of their decisions** (traced). *Corrected:* the cause, the brain reading energy capped at start, was traced in the shaft's top rows; on the mound it is *inferred*.
- **About half of larva food comes from passing ants' crops** (Laying: shared 50%, nursed 37%, floor 12%). Larvae with a fed adult within 2 cells: 69-80% in the top 5 rows, 1-2% in the room (morning trace).

### 5. Why every single fix failed (inferred; agreed by Deep trace and Nest building)
**Ants and food have to move together.**
- Moving ants in without food starved them: Nest building's stay, beg and home; `stay` even lowered nest workers' time underground, 7.2% → 5.4%.
- Moving food home without feeding the mound starved the mound: the full homing fix halved colonies and killed seed 4.
- Carrying food in without homing lost 78-82% of loads on the mound (my probe; re-run by Nest building).
- The nurses' door hand-off took 36-42% of forager loads away from the door-eaters, who starved (Laying).

*Corrected (Nest building):* `stay` acted on hungry nest workers too, not only diggers.

## What real ants do (05, sources there)
- The nest is home, and young workers stay in.
- Hungry ants are fed inside, but a worker that stays lean for a long time becomes a forager (Bernadou 2020).
- Foragers are sent out by contact with returning laden foragers (Gordon's lab) or by colony need (Greenwald 2018).
- Waiting foragers rest in an entrance chamber just inside the nest, and go deeper when foraging stops.

## The plan, as agreed (storeroom)

| step | what | owner | agreed by | notes |
|---|---|---|---|---|
| 1 | **Homing (D)** as an off switch: trip loads turn for home by scent contrast, and fill up at the food first | Deep trace | all | Lost carriers 45-58% → 15-22% (trip-only). On its own, underground share rises only 4.6-5.9% → 6.6-8.7%, and it risks moving the feeding to the door. Seed 1 under fill-and-turn fell to 20 ants at 300k; being traced. |
| 2 | **The storeroom set, as one off switch**, scored with step 1 on as its base arm and today's game as control (`04`) | lanes 20, 3 and 2 (to split) | all | See the parts below |
| 3 | Rest pull `workers` (built, held), then `on` measured | Nest building | all | `workers` alone: nest workers' time underground 8.3-10.3% → 9.3-12.3%, colony larger 4/4 |
| 4 | Home as a gradient outside, through the heap (B) | Nest building | all | It needs a cue that does not saturate where ants crowd; nest scent A can't serve. It also changes the doorstep drop, which is Scott's design. |

**Parts of step 2:**
- **Home inside follows `NestWay` deeper**, replacing "arrived at the first dug cell".
- **Hungry ants go to the store and eat there**, replacing HUNGRY_OUT. A, ants stay in, only with feeding below the top rows; agreed by all three. A worker that stays lean for a long time may still become a forager (biology).
- **Carry-in from the doorstep** (my probe) with a drop place below. The founding room alone failed, so try depth.
- **`keep` with Laying's nurse gate:** a fed nest worker may fill its crop at the store only while a hungry larva is within scent reach, and that crop goes only to larvae.
- **Nurses eat at the store**, with `nurse` and `stay` on, and the door hand-off (`relay`, `down`) dropped. An empty nurse needs a pull to the store.
- **Forage drive `larder`** reading the same store the nurses draw from. Gordon's contact cue is a later option.

**Scored on:**
- share of ants underground;
- food standing in the store;
- larvae starved per egg (Laying);
- ants starved, and where;
- colony size;
- young ants' share of time below 5 rows (Nest building);
- a picture.

## Where the lanes disagree, or something is still open
1. **Order.**
   - I first proposed A alone, first.
   - Nest building: debounce, then A with feeding, then C, then B.
   - Deep trace: D first, then the 04 set with D as its base.
   - **Resolved:** all agree A only works with feeding, and feeding below the top rows is the store plus nurses plus rest. So A belongs inside step 2, and step 2 waits on D.
2. **Does D ship on by itself?** Open. It waits on Deep trace's 4 seeds of the real switch, now running.
3. **Why mound soil carriers are lost** (1,155 pellets on seed 1). Open; not the nest-scent fog (inferred).
4. **The top-rows sharing on today's game.** Not re-traced; it is the morning trace's.
5. **Doorstep drop.** Scott's design was "foragers drop at the door". B would change that, and that is his call (Nest building).
