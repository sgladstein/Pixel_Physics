# Foraging food loop: old ant vs new ant, and a carry-home teacher (2026-10-22)

**Setup.** The Teacher lane's in-run training bed (compute branch 9d19121c plus the local patch here). 24 ants, food
topped up 25 columns out on both sides, 200k frames, no selection (`sel=off`), seeds 1-8 per arm, so 192 ants per arm.
"Old ant" = the game's own hand-built ant (chooser + act, no mind; `PP_NOMIND=1`). "New ant" = h1 founders. Same body
in every arm. Raw table: `arms-200k-8seeds.txt`.

## Where the food goes

| Arm | Bites delivered to the nest | Cells delivered (8 colonies) | Eaten whole by choice | Put back down in the field | Starved of 192 |
|---|---|---|---|---|---|
| Old ant | 25% | 5,899 | 0% | 8% | **0** |
| New ant (h1) | 3% | 638 | 38% | 41% | 98 |
| h1, Eat only under 2 tanks | 7% | 1,667 | 0% | 54% | 149 |
| h1, Eat only under 5.5 tanks | 7% | 1,395 | 2% | 54% | 141 |
| h1 + carry-home teacher | 36% | 3,218 | 33%* | 16% | 152 |
| h1 + carry-home + hungry rules | 31% | 4,901 | 39%* | 22% | 55 |
| h1 + the pack move (no teacher) | 4% | 656 | 39% | 39% | 93 |
| h1 + carry-home + hungry rules + the pack move | 29% | 5,239 | 41%* | 29% | **25** |

The percentages are of cells bitten in the field. \*In the teacher arms, the "by choice" column also counts the
teacher's own eat label and meals at home just after a delivery: the trace that marks a load is sampled every 50
frames, so treat it as rough there. Food put down in the field gets bitten again, so the new ant's bite
counts are inflated.

- **Digestion is not the bottleneck.** 3.3 face per tick at 6 frames per tick is about 1,500 frames for an 827 J cell.
  Both ants walk a load home in a few hundred frames.
- **The new ant shuffles food in the field.** Its field drops sit p50 22 columns out, 120 frames after the bite, and
  0 columns nearer home.
- **The new ant also eats its load by choice**, with the Eat move, mostly while already over 20 tanks. Gating Eat
  turns that food into field drops instead.
- **Both ants get very fat here** (old median ~150 tanks), because the bed turns breeding off. The old ant spreads
  the food: all 24 forage (the top 5 take ~28% of bites against 55% for h1), and none starve.

## The teacher (scratch `PP_CARRY=q` in mind.rs, acting on every decision it labels at q=1)

- **Carry home**: holding food taken away from home, at 1 tank or more:
  - at the nest, put it down;
  - else, with food in reach, pick it up (fill before walking);
  - else, take the step nearest home.
- **Hungry rules** (`PP_CARRY_HUNGRY=2.5`, under 2.5 tanks):
  - holding soil, put it down off the hole (`ANYCUT`: any spot at or above the ground);
  - else, on the surface, carry it away from home, or below ground, carry it up;
  - food in the crop, eat it;
  - empty, pick up food in reach, else go out (up, then away, to 40 columns), never dig.

Carry-home alone takes delivery past the old ant's share, but more ants starve, because the ants at home still don't
eat. With the hungry rules added, deliveries reach 83% of the old ant's and starvation drops from 98 to 55. Every arm's
remaining deaths are the hungry lane's die-off: ants holding a soil pellet, mostly on the surface 10-15 columns from
the nest, often stacked on each other. They are offered no put-down (the spoil footing rule wants ground, not an
animal, under the pellet). Seed 5 lost 18 the same way in every hungry-rule arm.

## The pack move (scratch `PP_PACK=1`, options.rs and mind::dig)

25 of the 55 deaths in the hungry-rules arm were ants walled in holding a pellet: their only options were stepping into
their own body, turning round, or resting. With jaws full there's no Dig, and with no room there's no Drop. The old ant
(`NEEDS_FIRST`'s `pack`) and the player's ant (G) both have `pack_behind` for exactly this: cut ahead, step in, and pack
the pellet where the tail was. In scratch, a mind walled in with a pellet is now offered that cut. With the teacher,
starvation fell from 55 to 25 of 192 and walled-in deaths from 25 to 6. Without the teacher, h1 barely uses the move
(93 starved against 98), so it has to be taught. Scotttt approved it for the new ant's body (2026-10-22): PR #794,
`PIXEL_PHYSICS_MIND_PACK`, handed to the Merge desk.

## Copy c1 (first copy of the teacher into h1)

`founders/c1.raw.txt` (copier_c.py, config c1.json). It was trained on the teacher's calls from 8 teacher-acting colonies
plus the soil, fetch and trail keep logs. Results on the training logs: carry-home overlap went from 0.22 to 0.83, and
the kept skills held at 0.92 to 0.99.

In the bed, 8 seeds × 24 ants, 200k frames, pack move on:

| arm | delivered | starved |
|---|---|---|
| h1 | 638 | 98 |
| copy c1 | 4,145 | 96 |
| h1 with the teacher acting | 5,239 | 25 |
| old ant | 5,899 | 0 |

The copy learned carrying home: deliveries rose 6.5x. It did not learn the hungry rules. Its deaths come early, in
bursts (seeds 1–4 die together at frames 6k–13k), at the nest and holding a pellet, the same trap as h1's. In c1's own
colony the teacher was logged as a labeller only (`PP_CARRY=0`). The copy then agreed with the teacher on only 17–37%
of decisions, about the same as h1 when hungry. That is the usual problem with copying a teacher: the training rows
came from states the acting teacher led ants into, and the copy's own colony visits different states. The second round
logs the teacher's labels in the copy's own colonies (`dlog.sh`, seeds 31–38) and retrains on them.

## Correction: "delivered" counted food put down at field cuts (found 2026-10-22 ~05:10Z)

The bed books a trip load as delivered when it goes down anywhere `nest_within_reach` says is home. Under the default
`NestHome::Dug`, home is beside any cell the colony ever cut, and h1 cuts surface soil out at the food. Counting only
loads put down within 8 columns of the hole (the old ant puts 5,865 of its 5,899 there and none beyond 12):

| arm | home deliveries | all "deliver" events | died of 192 |
|---|---|---|---|
| old ant | 5,865 | 5,899 | 0 |
| h1 | 503 | 638 | 98 |
| copy c1 | 2,469 | 4,145 | 96 |
| teacher + pack (px2.5) | 2,283 | 5,239 | 25 |
| teacher homed on the founding hole (ph8) | 4,263 | 4,605 | 125 |
| ph8 + a step down off gallery roofs, any drop when hungry (pa8) | 4,559 | 5,227 | 102 |
| pa8 + the way out through the tunnels (pw8) | 3,555 | 3,900 | 93 |
| pw8 + walk to the store when hungry (ps8) | 3,532 | 3,900 | 90 |

The teacher's error was walking loads toward `forage_anchor`, which follows every nest contact and so drifts out to
the field cuts. Homing it on the founding site (`PP_CARRY_HOME_R=8`) nearly doubles real deliveries, but deaths go from
25 to 125. Hungry ants die holding a pellet or empty, mostly underground within 15 columns of the hole, beside a store
that still holds food. Meanwhile the colony mean tank is around 30, and the fattest ant holds 88 tanks. Foragers
digest their loads on the long walk home. Patching the hand rules one trap at a time (ANYDROP, WAY, STORE) has stopped
paying: the last three patches moved deaths only from 102 to 90.

## How the old ant actually feeds itself (2026-10-22 ~05:40Z)

Bed tanks at frame 200k, seeds 1–4: every old ant is very fat. The colony mean is 131–272 tanks and the thinnest ant
holds 25–135. The old ant never chooses to eat (`ateByChoice` 0). About 70% of everything it bites is digested
passively, from the crop it carries and holds, and it delivers 22–28%. It also bites about twice as often as the
teacher colonies (2,200–3,800 bites per colony against 1,000–1,800).

So the old ant's economy is "always have food in the crop": it fills, digests on the way, and puts down at home what
is left. The new ant's Eat verb finishes a crop cell in one tick, so a choice to eat empties the crop at once, and
there is nothing left to carry home. My teacher's hungry rule (eat the crop under 2.5 tanks) copies that h1 habit
rather than the old ant. The next teacher drops it: never Eat, as the old ant. The body then feeds the ant from the
crop it carries, and a hungry ant with an empty crop goes out to the food.

## The old ant as teacher (PP_SHADOW=1), and what kills the copies (2026-10-22 ~06:00–06:45Z)

**Shadow teacher.** In a colony of old ants, every ant carries a mind that watches but never acts. Before each tick the
mind sees the options and context it would have had and decides, but only its memory moves. After the tick, the old
ant's actual move is read off its body as the matching option indices: a cut, a put-down, a pick-up, a step, a
turn or a rest. Positive control: events.csv is byte-identical to the PP_NOMIND old ant. Labels on seed 1: Walk 48%,
Rest 44%, PickUp 3%, Drop 3%, Turn 2%, Dig 0.1%. Patch: `shadow-rim-vs-9d19121c.patch`; logging: `shlogc.sh`.

**Test copy s4** went into c1, from 4 old-ant colonies plus 3 c1 colonies carrying the crater refusal labels. It fit
the old ant's moves only partly (overlap 0.11 to 0.36). The refusal anchor drifted (0.96 to 0.91). In the bed, 8 seeds
× 24 ants with the refusal off: home deliveries 2,743, deaths 133 of 192, against c1's 2,469 and 96. Not handed on.
The old ant's states are not the copy's states, so its labels never reach the trap below.

**What kills c1 (traced, seed 3, 15 ants dead at frame ~12,950).** A jam: the colony packs into its small pit holding
pellets. In their last 1,500 frames the dying ants picked Walk 2,770 times and Feed 1,688 times. A Drop was offered in
6% of their decisions and a PickUp in none. The feeding passes their banks around until they all hit zero together.
That is the burst: whole colonies die within ~100 frames, beside a store that holds food. Picture:
`jam-c1-vs-old-seed3.png` (top row c1, bottom row old ant, at frames 4.3k, 8.6k and 12.9k).

Body levers tried as scratch, on c1 and h1, 8 seeds:

| arm | home deliveries | died of 192 |
|---|---|---|
| c1 | 2,469 | 96 |
| c1 + the old ant's hungry put-down (`PP_NEEDDROP`) | 2,347 | 132 |
| c1 + no adult-to-adult Feed (`PP_NOADULTFEED`) | 2,356 | 118 |
| h1 + pack | 535 | 93 |
| h1 + hungry put-down | 447 | 96 |
| h1 + no adult Feed | 1,127 | 104 |

Neither lever saves the colony. Deaths come in bursts of 15–20, so 8 seeds is really "how many of 8 colonies
collapsed", and differences of ±30 are noise.

**The best survival teacher is still px2.5** (25 died). Its hungry rules were never learned by c1: in c1's own colony
they agree on only ~30% of decisions. The cause was the usual one, training rows taken from states only the acting
teacher reached. Running now: a DAgger round, with the px2.5 teacher labelling c1's own colonies (6 logs) and a copy
into c1.

**DAgger round d1 (result, 06:50Z).** px2.5 labelled c1's own colonies (6 logs, `dlog.sh`), and the copy went into c1
(`d1.json`). On the training logs the carry KL fell from 1.62 to 0.96. In the bed: home deliveries 2,233, died 130 of
192, against c1's 96. Agreement with the teacher in a fresh c1-or-d1 colony barely moved: hungry ants holding a pellet
went from 31% to 32%. Neither s4 nor d1 is recommended (both are in `founders/` under `-not-recommended`).

**Reading:** three copies of three different teachers (px2.5, the old ant, px2.5 again on the copy's own states) all
fail the same way. They leave the jam untouched, or worse. That is the sign of a missing input rather than a missing
label. In the dig ladder, the Teacher lane found a teacher rule that would not copy until the ant had a body sense the
job stage reads directly ("bottom of my hole in reach"). The likely equivalent here is a sense like "I'm in a crowd at
the bottom holding a pellet, and hungry". That would be a body change, so it is Scotttt's call. It has not been built.

## 32-seed results, the control, and the lesson that works (2026-10-22 ~09:00–10:30Z)

**8 seeds was noise.** Making c1's brain bill as cheap as h1's raised deaths from 96 to 133, and making h1's as dear as
c1's raised them from 93 to 116. Everything below is 32 seeds × 24 ants, 200k frames, sel=off, pack on. "Home" counts
food put down within 8 columns of the hole; "collapsed" means 12 or more of the 24 died.

| arm | home/colony | died/24 | collapsed of 32 |
|---|---|---|---|
| h1 | 57 | 10.7 | 14 |
| c0: control copy, no labels | 64 | 9.5 | 14 |
| c1 (founder) | 253 | 14.6 | 26 |
| c6: carry home from h1, no down-with-pellet labels | 225 | 18.2 | 30 |
| s4: c1 + old-ant shadow + rim | 373 | 15.9 | 28 |
| d1: c1 + DAgger, px2.5 | 304 | 17.4 | 30 |
| h1 + px2.5 teacher acting | 308 | 5.2 | 4 |
| hA: hungry rules only, acting on c1 | 240 | 6.7 | 4 |
| hB: hA + store walk + NEEDDROP (a body change) | 233 | 5.3 | 1 |
| **hA1: hungry-only lesson copied into c1, acting alone** | **269** | **4.3** | **2** |

**The control** (c0) has the same copy machinery and grown links as c1 but no labels, and it collapses like h1. So
the carry-home lesson itself raised collapse. It moves food out of the ants' bodies into a store that hungry ants
can't reach. The old ant instead lives on what it carries.

**hA1** is a DAgger copy: c1 acts in 6 colonies (seeds 41–46, `dlogA.sh`), the hungry-only teacher labels its own
states, and every labelled row is kept (`hA1.json`). Acting alone, it keeps c1's food home (269) and cuts collapse
from 26 to 2 of 32. Paired against c1: fewer deaths on 29 seeds, more on 3. It needs no body change. It went to the
Teacher lane for 64×6 with selection. Old-ant bar for comparison: 5,865 home over 8 colonies (about 733/colony),
0 died.

## Next

1. Second copy round (in progress), then hand the founders to the Teacher lane for its 64x6 judge.
2. Trace the stuck pellet carriers. If they are stuck because a stacked ant can't put a pellet down, that's a body
   rule, and it's Scotttt's call.

## PP_NEEDDROP on hA1 (2026-10-22 ~13:45Z, test only)

32 seeds, 200k frames, sel=off, pack on, hA1 acting alone:

| Arm | Home per colony | Died of 24 | Collapsed of 32 |
|---|---|---|---|
| hA1 | 269 | 4.3 | 2 |
| hA1 + NEEDDROP | 269 | 4.0 | 2 |

Paired: fewer deaths on 3 seeds, more on 2, identical on 27. Why it moves nothing: hA1 almost never holds a pellet while hungry (PP_DROPWHY, seed 7: 2 such decisions in 200k frames), so the jam NEEDDROP was built for is already gone. Not worth a body change; drop it.
