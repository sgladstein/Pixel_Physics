# Brood and breeding in the training bed: plan and adversarial review (2026-10-23)

*Landed 2026-10-27 from the thread's Claude Doc (https://claude.ai/artifact/D8oCnZn32kaD2AsV1b5uxR, rev 14), which a
new account cannot open. Kept as written: it addresses the owner ("you") and describes the state on 2026-10-23, when
`brood=lay` was still off by default on a branch. What happened after -- breeding on by default, the crumbs being the
working larder, the food-distance test and the learned lay gate -- is in
[`brood-in-training-2026-10-24.md`](brood-in-training-2026-10-24.md). Paths under `/mnt/project-files/brood/` below
are now `Reports/brood-in-training-2026-10-24/pictures/` (pictures) and `scratch/inrun/brood/` (scripts, carries).*


Oct 23, 2026, written for the owner

Turn the game's own egg-laying on in the training world and let selection choose each ant's genome the moment it hatches. The pilot shows this empties the larder in every arm, but no brake yet stops the colony from booming and busting, so regulation is the next piece of work.

## Why the nest fills with food

The training world never lets an ant lay. Its births are made by the selection harness, each handed 600 J (3 grants) from nothing, and the colony is held at 24 to 36 ants. So food has no use beyond feeding the adults it already has.

- One food cell holds 960 J, and a grown ant costs about 1,040 J, so a cell is worth about one ant.
- In colsel20's gardens the median nest held 389 stored cells, which is roughly 350 ants never made. Each ant also carried about 87 grants (17,000 J) in its bank.
- The lab game does not have this problem: its brood eats the surplus. The training world has the same brood code switched off (`births_paused`).

## The design

One life cycle for every world, with selection deciding only whose genome each new ant carries. The switch is `brood=lay` in `examples/inrun.rs`, on branch `claude/project-thread-6einf2`, off by default. With it off, a run is byte-identical to today's (checked on a full 200k-frame round).

1. **Laying is the game's own.** A fed ant lays a 120 J egg onto the brood pile at home once its bank reaches the laying bar (1,100 J, scaled by a gene). This runs in the body after every tick, so mind ants lay exactly as the old ant does.
2. **Brood must be fed.** A larva has to reach an adult's price (about 1,040 J). Nestmates feed it by touch from their crops and banks, and it eats food lying beside it. When food runs short, larvae starve before workers do.
3. **Selection picks the genome at hatching.** Each judgement queues selection's parents, best first. The frame a pupa hatches, the harness takes the next queued parent, mutates its genome as it would for a quota child, and writes it into the new ant. There is no gap between the pick and the ant.
4. **Food decides how many are born, selection decides who.** Selection's own made-from-nothing births are switched off. Its culls still wait for births, as they do today.
5. **The lab is unchanged.** With no selection running, a hatchling keeps its layer's mind, exactly as in the lab today.

Body genes still come from the layer, because body genes do not mutate in training yet. Once they do, they should be written at hatching from the same parent.

## What the pilot showed

Laying empties the nest floor in every arm, but only uncapped laying actually spends the surplus, and it does so by starving adults. Setup: today's founder (hcs), first round, the Teacher lane's colsel21 settings, 3 seeds of 200,000 frames each, run locally. Ranges are across the 3 seeds; ant counts are the lowest and highest seen after frame 20,000. The colony's reserve is stored food plus every ant's bank, in kJ, averaged after frame 20,000.

| Arm | Ants (low–high) | Food on nest floor (cells) | Reserve, store + banks (kJ) | Adults starved per round | Food carried home | Cells dug | Frame cost vs today |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Today (no laying) | 24 | 15–56 | 230–336 | 0–24 | 896–973 | 235–273 | 1x |
| Laying, no brake | 94–204 | 2–5 | 129–148 | 602–682 | 1,685–1,890 | 283–354 | about 5x |
| Laying + size cap 48 | 26–90 | 5–7 | 262–528 | 95–232 | 1,061–1,789 | 267–270 | about 2x |
| Laying + store brake | 36–175 | 6–7 | 317–394 | 264–569 | 1,102–2,526 | 286–358 | about 3x |
| Laying + food brake | 40–207 | 3–8 | 145–193 | 522–1,341 | 2,059–3,418 | 303–416 | about 4x |
| Cap + eggs kept out of first chamber | 0–78 | 2–8 | 119–343 | 102–155 | 260–1,037 | 253–275 |  |
| Cap + brood spread out | 21–114 | 1–6 | 241–639 | 74–169 | 562–958 | 266–410 |  |
| Softer cap (36, ramp 16) | 1–129 | 2–9 | 168–546 | 43–225 | 636–2,489 | 301–501 |  |
| Cap + no egg near a starving larva | same as cap on 2 of 3 seeds |  |  |  |  |  |  |

What to take from it:

- **The size cap hides the surplus rather than spending it.** Its reserve is higher than today's, because the ants bank it instead of piling it on the floor.
- **The food and store brakes boom and bust**, swinging between about 40 and 200 ants, as you predicted.
- **Capped colonies still crash. In 8 of 15 capped runs the colony fell to 26 ants or fewer, and 3 of those ended with 0 or 1 ant**. Three seeds is enough to show direction, not to rank arms.
- **Brood stands in columns on the chamber floor and in the shaft.** It never sits in a room of its own.
- Frame costs come from 4 runs sharing 4 cores, so treat them as rough.

Pictures: `/mnt/project-files/brood/pilot-curves.png` (ants and stored food over the round), `pilot-nests-end.png` and `pilot-nests-placement.png` (end-of-round nests with food, eggs, larvae and pupae marked).

## What the adversarial review found

An independent reviewer read the draft, the code and the pilot data and tried to break them. It found one real bug, fixed since, and several claims the pilot cannot support yet. The bookkeeping checks out: every hatch was booked exactly once, and no ant went unjudged.

| # | Finding | What changed |
| --- | --- | --- |
| 1 | When the parent queue ran dry, 25–42% of hatchlings got their mind from a fallback that skipped every selection rule. It mostly picked ants a few hundred frames old, and it ran in the random-selection control too. | **Fixed.** The queue now cycles again; with no queued parent alive, the egg's own layer is the parent, as in the lab. The source of each pick is logged. |
| 2 | The pilot never reproduced the complaint. Round-one nests hold 15–56 stored cells, not 389. | Next batch starts from late colsel20/21 nests and founders. I'm asking the Teacher lane for them. |
| 3 | The size cap hides the surplus in bank accounts instead of spending it. | The reserve (store plus banks, kJ) is now in the table and is the larder measure from here on. |
| 4 | 51–61% of starved adults die before they are ever judged; up to 43% die within 2,000 frames of hatching. Newborns hatch with about half a grant, against 3 grants today. Parents are still charged a child, and retired, for children that die young. | New arm: a larger hatch bank. Count only children that survive to judging toward a parent's limit. |
| 5 | Selection's culls kill more ants than starvation does (929 against 651 in one run), and culled ants become food that pays for eggs. With refill off, an extinct colony stays extinct. | Every regulation arm gets a no-culls control, and what colony selection does with a dead colony gets decided. |
| 6 | The brood-per-nurse brake I proposed probably would not work. Larvae were near zero right up to the crash it was meant to stop. Uncapped larvae pile up because they lie out of reach, which the brake would read as too much brood. A ratio is also the same scale-free signal that sank the food brake in an earlier attempt. | **Dropped** as the lead idea. Any brake gets tested first against the pilot's own time series: does it rise before the crashes? |
| 7 | A hand-made caste is still in the game: 1 ant in 4, chosen by id, is a nest worker for life. Only nest workers feed larvae from their bank, carry store food or dig the nest face. | A question for you (below). It is shipped game physics and it reaches the training world. |
| 8 | The no-egg-near-a-starving-larva rule did fire (seed 3 diverges). An earlier session already found it nearly inert, because larvae fall out of reach. | Corrected; its counter goes in the brood log. |
| 9 | Food/brood separation has almost nothing to measure once laying is on: 2–8 food cells on the floor. | The separation score is gated on enough stored food, and weighed in joules. |
| 10 | Selection now sees 14x more births per round (1,600–2,200 against 119), so mutation per round rose 14x. | Any comparison against today's colsel21 has to control for it, for example by lowering the mutation rate per child. |

## The open problem: boom and bust

Every brake tried so far either overshoots and starves, or holds the colony on fat banks until it dies in lockstep. My reading is that this is a delay problem. Laying answers today's wealth, but an egg takes 500 frames or more to become a worker, so by the time the brake bites the colony has already committed to too many mouths. The game's food and store brakes count adults only, so they never see the brood coming through.

Arms for the next batch, in the order I'd run them:

1. **Brakes that count the brood.** The food brake and store brake as they ship, but with eggs, larvae and pupae counted as future mouths. If the delay reading is right, this is the one that damps the swings. These would be game rules, off by default.
2. **A larger hatch bank.** It tests whether much of the starvation is simply newborns hatching too poor to survive their first trip.
3. **A no-culls control** for each arm, so selection's removals are not mistaken for the colony regulating itself.
4. **The training-only size cap**, kept only to bound frame cost. It is not physics and should not decide the shape of the curve.

What counts as solved: across 16 or more paired seeds, the colony's lowest point stays well above zero, the swing between its highest and lowest size shrinks against today's laying, starvation falls, and digging and food carried home hold all round. Colony selection then does the rest, because a colony that busts ranks low.

## Keeping food and brood apart

Reward the ant that makes the choice, measure the nest, and only then consider a game rule. Today brood stands in columns on the chamber floor and in the shaft, and the game's two placement rules do not help. Keeping eggs out of the first chamber changes nothing while that chamber is all the colony has dug. Spreading brood turns one column into several short ones.

1. **A graded separation score**, counted per brood cell as the distance to the nearest stored food, and per nest as the share of brood with no food within 3 cells. A later version will ask whether brood and food lie in different rooms, using the dig thread's room-shape tool. It only counts when the nest holds enough food to be worth separating, and is shown with every nest picture.
2. **Pay the ant that drops the food.** Ants decide where food goes, so food put down close to brood pays less and food put down in the nest away from brood pays in full. This lives in the training score.
3. **A colony-level case** for the Teacher lane's colony selection, read from the end-of-round map.
4. **Possibly a game rule:** an egg prefers an open home cell with no food beside it. It would be off by default until measured.

## Laying genes

Laying already lives in the body's genes: how full a bank must be before laying, how much a parent hands its child, and how far eggs keep from the door. With the genome chosen at hatching, the per-ant score never rewards laying or nursing. Only colony selection can judge those genes, because a colony that breeds well grows, digs and forages more. So the order is:

1. Laying genes stay fixed at today's values while the switch is tested.
2. They mutate alongside the other body genes once the Body genes lane lands.
3. Then check whether colony selection moves them.

## Next steps

1. (Done) The `brood=lay` switch, genome chosen at hatching, the size cap, brood logs and nest maps, and the 9-arm pilot. The review's bug fix is in.
2. Reproduce the complaint: start from late colsel20/21 nests and founders, and confirm the nest is packed before changing anything.
3. Build the brakes that count brood, the hatch-bank arm and the no-culls control. Run them on 16 or more paired seeds, on GitHub Actions or a VM slot from the Teacher lane.
4. Build the separation score and the food-drop pay, and hand the Teacher lane a colony case.
5. Multi-round colony selection with `brood=lay` on the current founder (hcs with the dh7 dig head, light-reading dig head, home sense, weaned body rules), against today's colsel21, with mutation per child lowered to match turnover.
6. Land it. The harness switches go onto the Teacher lane's compute branch. Any game rule (a brake, an egg-site preference) goes to main through the Merge desk, off by default, with its measurement.

The Ants feeding on plants lane is also running `brood=lay` on its plant garden, where food is limited by what grows. That is the natural place to see boom and bust.

## Questions for you

- **The nest-worker caste.** The shipped game makes 1 ant in 4, chosen by id, a nest worker for life. Only those ants feed larvae from their own bank, carry store food and dig the nest face. That is a hand-made caste, and with laying on it decides who can raise brood. Should it come off in the training world, or in every world, and be left for ants to evolve roles themselves? Removing it changes game results, so it would go through the Merge desk with a measurement.
- **Starvation as regulation.** Today's training world loses almost no adults. Any version with real breeding will lose some, as real colonies do. Is a few dozen deaths a round acceptable if the colony is steady and the larder is spent?
- **Frame cost.** Real breeding means bigger colonies, about 2x the frame cost at 60 ants and 5x at 165. Is it acceptable to cap training colonies near 50 for speed, if that cap is kept out of the game?

## What the evolved colonies showed (added 2026-10-23, evening)

Tested on four late colonies from the Teacher lane's colsel20 run (round 47, the packed-nest arm), in a fresh garden with selection off, 200,000 frames.

**The packed nest is crumbs.** Every stored cell in these nests is crumbs, the part-eaten food an ant puts back, worth about half a food cell each. There are only 0 to 2 whole food cells. One nest held 475 cells, about 227 kJ.

**With laying on, all 4 colonies died out, and 106 to 188 kJ of crumbs were still in the nest.** The probe shows why the crumbs went uneaten. Starving ants at home were standing on them with empty crops and free jaws, and the body offered them 1 to 7 pick-ups. They chose Walk 58 times out of 59. This is the ant's own choice, not a rule blocking it. Training never let an ant go hungry, so it never learned to eat at home.

**But the larder is smaller than it looks.** These colonies boomed from 25 to 150 to 200 ants in 40,000 frames. The inherited banks, about 87 meals an ant, paid for that boom. At the peak the crumbs come to less than one meal a head. Late in the crash, most hungry ants are on the mound near the door, and 78 to 81% of them are holding a soil pellet, so they cannot eat.

| arm (4 colonies × 3 seeds) | alive at 200k | notes |
| --- | --- | --- |
| laying, no brake | 0 of 4 (one seed) | crash by about 100k |
| + home-eat crutch | 0 of 4 | twice the meals at home, and the crash comes about 10k frames later |
| + reserve brake (3 meals a head) | 0 of 4 | too permissive while the banks are fat |
| + size cap 48:8 | 7 of 12 |  |
| + size cap + crutch | 9 of 12 (3 of 4 on every seed) | one of the 9 was dying, at 15 ants |

Colony c43 dies in every arm. It brings home a third of the food the others do. Most of its hungry ants are at home, holding pellets.

**Built** (branch claude/project-thread-6einf2, head 7108ed0): `larder.csv` gives the stored food in kJ by kind. `PP_HUNGRYPROBE=1` logs every hungry ant: crop, jaws, the food touching it, the pick-ups offered, and its choice. `PP_HOMEEAT=H` is the crutch: a hungry ant in the nest is offered only pick-up, and it is weaned with `PP_HOMEEAT_P`. The arms `lay_reserve`, `nocull` and `hatch_grant` are built too.

**What this changes in the plan.** Real breeding needs two things first:

- A growth brake. For now that is the size cap: the reserve brake reacts too late while the banks are fat.
- Hunger skills the training bed never exercised: eat at home, put the pellet down, come home.

The Teacher lane will run size cap + crutch against the size cap alone in colony selection, with the crutch weaned and judged switched off.

## Colony selection with breeding on: brood1 (2026-10-23, night)

The Teacher lane ran this on the VM. Each arm was 64 colonies × 32 rounds from the hcs founder, using colsel21's rules plus `brood=lay lay_cap=48:8`, with lower mutation per child (kpm 0.013, 4 events). Body rules were weaned over rounds 4 to 20. One arm also had the home-eat crutch. Every common garden ran with the scaffolds off and breeding on.

| measure (no-crutch arm, common garden median) | round 1 | rounds 15–31 |
| --- | --- | --- |
| food brought home | 608 | 16,800–24,900 |
| adults starved | 120 | 1–2 |
| ants alive at the end | 13 | 84–89 |
| colonies extinct (of 64) | 15 | 0–3 |
| crater | 3 | 3–8 |

**Breeding is stable under selection.** No colony crashed. Starvation all but disappeared, and food brought home rose about 30x. At round 31 the stored food was 260 cells (320 with the crutch), against 514 without breeding. Ant banks held 28 to 39 meals, against 107.

**The crutch is not needed.** The arm without it starves no more. The crutch arm's craters swing up to 200–371 in rounds 23 to 27, and its big nests are open pits.

**The nests are still packed.** At about 87 ants the size cap has raised the laying bar about 32 times, so laying all but stops while income runs far above upkeep. The surplus has nowhere to go but the bowl. Rooms fell to 0 once the body rules were weaned, which is the digging line's problem, not breeding.

**Decision (with the Teacher lane):** `brood=lay lay_cap=48:8` becomes the standard training bed, without the crutch. To take the larder down further, the lever left is limiting the food supply, which is the plant-food lane's work. A looser cap would also help, but it costs frame time, because cost grows with the number of ants. The Teacher lane's `store=S` pay rule has already been tried and failed. At store=4 (colsel22, food 45 away) it emptied the larder, 36 cells against 452, but the crater came back as open pits, 310 against 10 (`/mnt/project-files/teacher-lane/gardens-colsel22.png`).
