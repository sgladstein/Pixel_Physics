# Strangers start the fight: a design for touch-triggered alarm (2026-10-10)

*Proposal, reviewed, built behind an off switch, first results in §9. Owner asked for it
on 2026-10-10 ("yes" to writing it up for review). Project rule: trace,
then proposal, then review by another lane, then build behind an off switch,
then test. The trace is §1; the second lane's review (yes, with changes) is
`/mnt/project-files/fight-trace-2026-10-10/review-stranger-alarm-2026-10-10.md`
in the shared project folder, and every change it asked for is folded in
below (§8 lists them). Main `e594d970`.*

**In one paragraph.** Since 2026-10-05 two ant colonies in a lab box never
fight, however often they meet. The only thing that ever started a fight was
an ant trying to *eat* a stranger, and the evolved lab ant's plant gut (-0.8)
no longer counts ant flesh as food. This proposes that **touching a stranger
of your own kind is itself alarming**: an ant whose body touches a living
ant that fails its nestmate test marks the alarm plane where it stands,
**more strongly the more foreign the stranger smells**, and less than a
display at most. Everything after that already exists and is tested: the
`(Alarm, Attack, 2.0)` wire, `nearest_foe`, the contest's assess-then-commit,
and the display that lets a meeting end without a blow. It goes behind
`PIXEL_PHYSICS_STRANGER_ALARM`, off.

## 1. The trace this rests on

Full notes and logs: `/mnt/project-files/fight-trace-2026-10-10/` (shared
project folder). Traced in code, and measured:

- `ant.ron` wires one route to `BrainOutput::Attack`: `(Alarm, Attack, 2.0)`.
  At alarm 0 the urge is `squash(0) = 0` and `act` never enters the fight
  branch (`attack_urge > 0.0` gate in `creature.rs` `act`).
- The alarm plane is written by `cry_alarm` (an animal bitten) and by a
  contest display. A display happens only after an attack roll, and since the
  owner's 2026-09-14 ruling eating a plant writes nothing. So in a box with no
  predator **the first bite of every fight was an ant eating a stranger**
  (`Reports/why-colonies-do-not-fight-2026-09-14.md`: "predation is the
  ignition"; `animal-conflict-research-2026-09-14.md` §8.2).
- `e793e4c5` (2026-10-05) founds every lab box with the evolved ant,
  `scene::LAB_ANT_TRAITS`, gut -0.8. A 480 J cell of ant flesh then yields
  480 x (1 - 1.8/2)^2 = 4.8 J, under `EAT_YIELD_THRESHOLD` (12 J). A
  stranger is not food, is never bitten, and nothing ever writes the alarm.
  Flesh leaves the menu below a gut of about -0.68.

`examples/rivalry`, its default 512x320 two-colony bed, 24,000 frames,
mutation off, `RAYON_NUM_THREADS=1`, seeds 1-4:

| arm | stranger contacts (sampled) | attacks | cross-colony kills |
|---|---|---|---|
| evolved (the lab default) | 424 / 298 / 219 / 660 | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 |
| evolved, gut set to 0 only | 20 / 21 / 23 / 20 | 16 / 17 / 16 / 23 | 51 / 47 / 44 / 44 |
| ancestral ant | 20 / 20 / 22 / 20 | 19 / 17 / 18 / 24 | 43 / 42 / 39 / 43 |

The gut alone accounts for it. The fighting arms show fewer standing contacts
because a contact is consumed by the fight that follows (`rivalry`'s own
note). The owner's 2026-10-10 playtest (herb_ant, two colonies 318 columns
apart, 0 kills) and the 2026-10-09 herb_ant playtest (0 kills of 576 deaths)
both ran this ant.

**What this makes of the parked fight recruiting** (`claude/colony-wars-j7j74f`,
`PIXEL_PHYSICS_FIGHT_RECRUIT`): it has two halves, and they behave
differently here (review §4, read on the branch). The v3 call to arms runs
only from a bitten ant, so it is inert while nothing bites. But the branch
still carries v1's alarm climb: `chooser_step` adds `recruit × alarm_rise`
for every empty, unladen ant whenever the alarm plane is live. With this
switch on, every border contact is a mark that climb would draw idle ants up.
So it is a separate second step, tested as two arms (climb only; climb plus
trail), with its `recruit_steps` read against this switch's marks.

## 2. How it should work, from the biology

| rule in real ants | evidence | our ant today |
|---|---|---|
| Nestmates are told apart by a colony odour (cuticular hydrocarbons) against a tolerance threshold, judged on contact by antennation | Vander Meer & Morel 1998; `animal-conflict-research` §2a | **Have, exactly**: `TRAIT_SCENT_A/B/C`, `TRAIT_TOLERANCE`, `scent_accepts`, `blend_with_nest` |
| Recognising a non-nestmate is what starts aggression; hunger is not needed | Hölldobler & Wilson 1990, ch. 5; Czaczkes et al. 2024 (*Lasius niger*: antennation and jerking graded by relatedness) | **Missing**: recognition gates *who may be hit*, but nothing makes recognition *start* anything |
| Most meetings end in display, not a fight; escalation depends on the odds | Czaczkes 2024; *Myrmecocystus* tournaments (Hölldobler 1981); `animal-conflict-research` §2c-2d | **Have**: `sim::contest` assessment, `displays` |
| The alarm pheromone raised in a fight draws and arouses nestmates nearby | Hölldobler & Wilson 1990, ch. 7 | **Have** (local only): the alarm plane, read where the ant stands |
| Alarm is local and fades fast | same | **Have**: `the_alarm_forgets_faster_than_a_trail` |

So the gap is one link: **recognition of a stranger on contact should raise
the alarm**. Every other link is already built and owner-reviewed.

## 3. The rule

**When an ant's body touches a living animal of its own kind that fails its
kin test, it marks the alarm plane at its own cell, topping it up to a level
graded by how far past its tolerance the stranger smells.**

1. **The sense.** A walk of the body's ring, the same deduplicated ring
   `nearest_foe` and `adjacent_food_counted` walk, over attached living
   cells that `is_animal_cell`. A cell is a stranger when it fails the kin
   test but **passes `is_living_kin_id`'s own kind clause**
   (`gut.crosses_kinds || same species`), so recognition and alarm can never
   disagree about who is a candidate (review Q3). Built as its own walk in
   `act`, only when the switch is on, rather than a flag threaded out of
   `sense`: `sense` is called more than once on some paths, and a walk in
   `act` cannot read a different call's answer (review §1). Off, nothing is
   walked and nothing is drawn, so the off game is the shipped game byte for
   byte.
2. **How foreign** (review Q1, after Reeve 1989's threshold on cue distance
   and the ignore / avoid / display / bite ladder assays score). With `d` the
   scent distance and `r` the judge's tolerance radius,
   `foreign = clamp((d - r) / r, 0, 1)`, the largest over the strangers
   touching. A drifted nestmate just past the radius reads near 0; a rival
   colony 1.5-1.8 away (`rivalry`'s gap column) reads 0.5 to 0.8, full at 2.
3. **The mark.** Target `level × foreign`, in alarm units (0-255). If the
   alarm at the ant's cell is below the target, deposit the difference;
   never more. **This tops up one rung, it does not bound the plane**: the
   plane adds (`saturating_add`), so a display adds 40 on top and a bite
   writes 240, and only the alarm's fast decay (`ALARM_RHO`, about 190 frames
   to clear) brings it back down. The ladder is touch < display (40) < bite
   (240), so `level` defaults to **20**, under a display, and is swept
   (10 / 20 / 40) rather than borrowed from the display constant.
4. **What follows is existing code.** Alarm 20/255 reads 0.078 and the wire
   gives `squash(0.157) = 0.14` per decision. A winning roll walks
   `nearest_foe` (kin skipped), and the contest decides: commit and bite
   (`cry_alarm`, 240) or withdraw and display (+40). **Per contest a
   lopsided meeting mostly displays; per contact, a held meeting probably
   escalates**: deciding every 5 frames, a 50-frame contact rolls at least
   once with probability 1 - 0.86^10 = 78% at level 20 (94% at 40), and
   stacked displays raise the next roll. That is the number §5's funnel
   measures, not one this design asserts.
5. **`fed`: two holds in one part**, each applying only while a same-kind
   stranger touches the ant (§10); its `Attack` urge is then read as 0 for
   the tick, and it neither marks nor answers.
   - **Lean: hunger overrides it** (owner's rule; Grover et al. 2007,
     carbohydrate-starved Argentine ant colonies were less aggressive). Lean
     is energy under `LeanForage::line` x start energy.
   - **Laden: a forager keeps its load.** A separate rule, not justified by
     Grover: any ant with crop cells or food in its jaws, *however well
     fed*, is held, even while the stranger is biting it. It is held too
     when a beetle bites it while a stranger touches it (rare; the hold
     reads the touch, not the biter).
   A named part, so "fights while starving or carrying" can be judged
   apart.
6. **Riders: counted, never acted on** (review Q2). `nearest_foe` folds no
   riders (the owner's "one attack must not hit 20 creatures"), so a rider
   alarm would arouse an ant with nobody to reach. A diagnostic counter only;
   if it turns out large, `nearest_foe` learns riders first.
7. **Brood is safe by construction.** `brood.ron` is `kind: Powder`, so a
   larva is not an animal cell: it neither raises the touch nor is a target.
8. **No memory.** The rule is neither dear enemy nor nasty neighbour (both
   reported in ants: Heinze et al. 1996; Newey et al. 2010); contests per
   episode are read over the run to see whether anything like either emerges.

### The switch

`PIXEL_PHYSICS_STRANGER_ALARM`, off by default, `World::stranger_alarm` for
one world (the house pattern). `on` is `touch,fed,level=20`. Parts:

- `touch` - the rule above (the ignition).
- `level=N` - the top-up level at full foreignness, 0-255 (default 20).
- `fed` - while a stranger touches, lean ants (hunger) and laden ants (crop
  cells or food in the jaws, however well fed) neither mark nor answer.
- `species` - also count other kinds' animals (not in `on`; predator and
  prey stay the diet's question).

Counters on `CreatureStats`: `stranger_touches` (decisions that touched a
stranger), `stranger_marks` (deposits made), `stranger_fed_skips` (touches a
lean or laden ant ignored), `stranger_rider_touches` (diagnostic). Paired
with `contests`, `displays`, `attacks`, `attack_kills` and `kills_log`.

**The nest kin gate (PR 569) stays on in every arm.** It protects nest
odour from blending across colonies; nothing in the attack path reads colony
or nest, by design, so recognition is all scent and the gate is what keeps a
colony's scent its own. No arm runs this switch with the gate off.

## 4. What it could break, and the check for each

| risk | why it is plausible | check |
|---|---|---|
| **Own-colony fights from drift** | with mutation off `mutate_newborn` freezes scent and tolerance, but nest odour still drifts (`step_nest_scents`, about 120k frames to the tolerance radius), and long-absent foragers carry stale odour; with mutation on (the game) scent drifts per birth and tolerance at a third of that | (a) one-colony goal box, mutation off: **on must be byte-identical to off**, not just "no touches"; (b) mutation on, 2-4 seeds to 150k-300k: touches and kills where attacker and victim share a lineage by `World::descends_from`, never the label (`regroup_by_scent` mints labels for split clusters, so a colony fighting its own fission daughter books as cross-colony); (c) heritable `TRAIT_TOLERANCE` over the run |
| **The shared heap becomes a permanent border** | both colonies' foragers meet at shared food every trip; `(Alarm, Move, -1.0)` slows them, fights cost jaw work, and at gut -0.8 a kill pays nothing | `trip_deliveries` per colony and time at the heap, on against off |
| **Colonies wiping each other** | held contacts likely escalate, and the trace saw 219-660 sampled stranger contacts per seed against about 20 in the fighting arms | per seed: does either colony reach 0, when, and of what |
| **Border jams** | alarm lowers `Move` | steps per decision near a stranger, on against off; the meeting zone in a picture |
| **Nest disruption** | a stranger at a door alarms the doorway | brood and door census, against off |
| **Frame cost** | a ring walk per decision when on; first contact allocates the alarm plane (40 MB at outdoor size, small in a lab box) | lab tick time on against off; `ascii` unchanged (off) |

## 5. How it would be judged

Per `Reports/how-we-test.md` §2, on the evolved founder, mutation off,
paired by seed on one build.

**Beds.** All three first proposed starve the evolved ant whether or not it
fights (`rivalry`: 112 of 113 animal deaths starved; `war_two` is planted;
the owner's herb_ant box starved by 40k), so deaths by cause would drown in
starvation and `fed` would read as doing nothing. So the main bed is **a
two-nest dry goal box**: `nest_goal.ron`'s ground with two colonies, each with
its own endless heap, plus an arm with one shared heap between them, read over
100k-300k. `rivalry` stays the fast iteration bed, `war_two` the 12-seed
settle bed.

**Arms:** off (byte-identical to main); on at level 10 / 20 / 40; on without
`fed`; gut 0 with the rule off (the predation reference); a mutation-on pair.

**The direct metric is per encounter**, not a ratio of counters: for each
stranger pair, first touch frame, then the outcome - parted with no roll,
display only, bite, kill - and the time to first bite. That is the
literature's escalation rate. Things that would mislead:

- the gut-0 arm's cross-colony kills are mostly *eating* (seed 1: 16 attacks,
  11 attack kills, 51 cross-colony kills), so compare attacks and attack kills,
  never `xcol`;
- `stranger_touches` counts decisions, so a jam inflates it: report pairs and
  episodes;
- contacts fall once fighting starts, so any rate per standing contact moves
  between arms;
- own-label kills are not own-lineage kills (§4);
- a zero-attack seed may be one whose colonies founded alike: print the
  founding gap per seed first.

**Picture.** The meeting zone over time (`rivalry gif=`) with the alarm
plane overlaid, so a standing border can be told from a moving one.

**Then** the parked recruiting on top, as two arms (§1).

## 6. Alternatives considered

- **Put the gut back to -0.5 or 0 in the lab.** Restores predation as the
  ignition at once (measured, §1). It would undo the evolved ant: in the goal
  box the gut was the trait that cost most when reverted (mean
  100k-200k 303-358 against 384-467, `scene::LAB_ANT_TRAITS` doc). It also
  keeps war tied to diet, so any future plant-eating lineage stops fighting
  again. Not recommended.
- **Hunger-gated predation** (eat strangers only when starving). Biologically
  sourced (`animal-conflict-research` §5), but still diet-coupled, and a fed
  colony would never fight.
- **A new brain input, `StrangerNear`, wired to `Attack`.** Evolvable, which
  the owner wants in the long run. Costs a genome slot, changes every species'
  layout, and its weight would start at whatever we author, so it is the same
  rule with more moving parts. Better as a later step, once the engine-level
  rule shows the behaviour is wanted.
- **`sight_range` and `ThreatNear`.** Closed with the alarm wire in 2026-09-14
  and did not move attacks (`why-colonies-do-not-fight-2026-09-14.md`); the binding link
  was the ignition, which is still the case.

## 7. Open questions, as the review answered them

1. **Strength**: less than a display, graded by foreignness (§3.2-3.3).
2. **Riders**: a counter only (§3.6).
3. **Boundary**: `is_living_kin_id`'s own kind clause (§3.1).

## 8. What the review changed

- The mark is graded by scent distance past tolerance, defaults to 20 under
  a display, and is swept; "bounded by the top-up" was wrong and is gone.
- Riders are a diagnostic; the kind boundary reuses the kin test's clause.
- `fed` added: lean or laden ants neither mark nor answer.
- The drift check is byte-identity with mutation off, a mutation-on pair, and
  kills split by lineage.
- Main bed is a two-nest dry goal box; the direct metric is per encounter;
  gut-0 cross-colony kills are no longer the reference.
- The parked recruiting's alarm climb fires on any live alarm, so it is
  tested as two arms.

## 9. First results (2026-10-10, branch head after `687033e7`)

Logs, the bed files and run scripts are in the shared project folder,
`/mnt/project-files/fight-trace-2026-10-10/` (`two-nests-steady/`,
`two-nests-12seeds/`, `mutation-on/`). `rivalry scenario=two_nests_own
encounters=<csv>`, 150k frames, evolved founder, mutation off,
`RAYON_NUM_THREADS=1`, switch `on` (touch, fed, level 20) against off.

**The bed had to change first.** The two-nest box as first written dropped
one finite heap per colony, and every arm, off and on, starved out by 24k.
Both beds now feed like `steady_income`: 40 cells every 1,000 frames per
colony 30 columns outward (`two_nests_own`), or 80 at the midpoint
(`two_nests_shared`).

**Off is main.** Every off run is byte-identical to the same run before the
encounter log existed, and one colony alone (`steady_income`, seed 1) is
identical on and off, every sample line. *An earlier "identical" for that
pair compared two empty logs (the scenario override directory hides the
built-in beds); it was caught and rerun.*

**On, 12 seeds of `two_nests_own`:**

| | off | on |
|---|---|---|
| fight kills (bite) | 0 every seed | 126-1,571; 0 on seed 9 |
| kills inside one founding line | 0 | 0 every seed |
| cross-colony contact samples | 55,630-110,754 | 69-570 |
| living at 150k, median (range) | 540 (483-585) | 486 (375-616) |
| starved, median | 103 | 30 |
| how it ended | two colonies mixed over the whole box | 8 borders, 3 conquests, 1 peace |

- **Borders (8 seeds):** each colony's heads hold its own side, e.g. seed 7
  columns 5-251 against 253-507; with the switch off both span 4-500. The
  fighting never settles into a quiet line: kills per 25k frames stay
  roughly level through 150k.
- **Conquests (seeds 3, 11, 12):** one colony is gone by 30k-78k and the
  winner grows to 527-616, as many as both colonies together with the switch
  off.
- **Peace (seed 9):** the two colonies founded smelling alike (founding gap
  0.57 against 1.4-2.7 elsewhere), accept each other, and the run is
  byte-identical to off. This is the rule working, not failing.

**Per encounter (pooled, 62,691 stranger pairs that touched):** 67% parted
with no contest, 7.5% displayed only, 6% bit without killing, 19% ended in
a kill. First touch to first bite: median 10-15 frames per seed, quartiles
4-5 and 24-60. Pairs that went to a contest without a logged touch: 0-15
per run, so the fights come from touches, not from alarm drifting in.

**Mutation on (seeds 1-2, `two_nests_own`):** main already fights here
(974-1,106 bite kills, 307-358 eaten), because the gut drifts back toward
meat and predation lights the old wire. With the switch on, kills are about
the same (785-1,127), cross-colony contact falls 1,121-1,142 -> 171-275,
and kills inside one founding line stay 0 in all four runs: scent drift does
not start a civil war over 150k.

**Not yet measured:** the shared-heap bed past 2 seeds (2 seeds: 311-736
kills, both colonies alive, split at the heap); levels 10 and 40 and `fed`
off on this bed; the parked recruiting arms (§1); frame cost.

## 10. After the results review (2026-10-10)

The second lane reviewed §9 (`review-results-pr673-2026-10-10.md` in the
shared folder): **not yet** for default on, three blockers. What each became.
Logs and per-meeting CSVs: `/mnt/project-files/fight-trace-2026-10-10/review-blockers/`.

**1. `fed` reached past strangers: fixed.** The hold now applies only while
a same-kind stranger touches (`lean_or_laden && touch.foreign.is_some()`),
and lean is read off `LeanForage::line`. Guard
`a_lean_ant_still_answers_another_kind` fails with the old hold put back.
In a one-colony box with four beetles (`one_nest_beetles`, seeds 1-2,
150k), the switch on now plays byte-identical to off (alive 295 / 305,
attacks on beetles 38 / 45); the old hold gave 277 / 237 alive and 31
attacks on both seeds.

**2. The conquests: all three losers died fighting, by two routes.** The
§9 on arm changed with the fix, so 12 seeds were rerun: 8 borders, 3
conquests, 1 peace again, but the conquest seeds moved (2, 7, 12, against 3,
11, 12 before) -- which colony wins is chaotic, not a property of a seed's
opening. Per-lineage deaths by cause over time (seeds 2, 7, 12):

| seed | route | loser's dead, starved / killed (any lost head) | what happened |
|---|---|---|---|
| 12 | landing clash | 0 / 67 | 58 of the 52 landers and their first young killed in the first 6k frames |
| 7 | runaway | 7 / 481 | even exchange to 42k (230 vs 257 killed), then the side behind loses 5:3 (42-54k: 97 vs 161) and is gone by 66k |
| 2 | attrition | 5 / 354 | an exactly even exchange (354 killed each side), but the loser stopped growing from 36k and went 118 -> 0 by 66k |

Starvation killed no loser. Seed 7 fits the contest's `numbers` term: the
side that falls behind meets more foes per encounter and backs off or loses
more [inferred from the code; not traced per meeting]. Seed 2's loser had
no fewer kills but no births to replace them [inferred; births per line are
not logged].

**The landing is most of the opening massacre.** Two controls, seeds 1, 3,
11, 12:
- *Far* (240 columns apart): 4 of 4 hold a border; 40-58 kills by 12k
  (against 56-78 at 120 apart, 11 fighting seeds); living at 100-150k 471-527 on against
  534-559 off.
- *Late* (the second colony lands at 30k): the newcomer is wiped out at
  landing on 3 of 4 seeds (45-94 kills, no war after), as an incipient
  colony beside an established one would be; seed 11 holds a border.

**3. Lineage path untested: still so.** `splits=0` in every run (mutation
off); the 4-seed mutation-on rerun prints it.

**Level and `fed` (seeds 1-4, per meeting, 300-frame gap):**

| arm | living 100-150k | kill / meeting | display only | held (hunger) | parted |
|---|---|---|---|---|---|
| level 10 | 417-623 | 11-18% | 4-6% | 13-30% | 42-66% |
| level 20 (`on`) | 439-610 | 14-22% | 6-9% | 12-22% | 50-59% |
| level 40 | 387-453 | 21-29% | 8-10% | 14-19% | 37-50% |
| `fed` off | 431-473 | 20-29% | 7-10% | 0 | 53-67% |

Level moves how often a meeting turns lethal, not how many die: kills per
run stay 1,200-1,650 at every level on the seeds that hold a border
(level 10 has more meetings: 9,600-13,100, against 7,300-9,700 at 20 and
5,100-6,900 at 40 [inferred:
the sides mix more before they separate]). **No level brings displays near
"most meetings"**: display share is set by the contest's assessment
(`contest::boldness`), which the alarm level does not reach. Ten stays the
gentlest per meeting; 20 is kept for now.

**Pooled meetings, 12 seeds, level 20:** 74,602 meetings: 52% parted, 16%
held by hunger, 7.4% display only, 5.9% bite, 18% kill. Living at 100-150k,
median 547 off against 475 on (-13%); on the 8 border seeds -5% to -30%,
mean 538 -> 452 (-16%). Frame cost +1.5-4% (the review's measurement).

**To 300k, 12 seeds** (`long-300k/` in the shared folder): living at
100-300k, median 534 off against 463 on (-13%). At 300k: **5 borders**
(seeds 1, 3, 4, 5, 8), **3 conquests** (2, 7, 12) and **4 one-family**
(seed 9 from the start; seeds 6, 10 and 11 merge by scent late, cross-colony
strangers falling to 0-1.6% and the fighting stopping, seed 6 from about
264k and seed 11 from 288k). Off, seeds 2 and 6 merge the same way by 294k,
so the late merge is main's. Starved deaths fall on border seeds (e.g. 188
-> 43) and rise on conquest seeds, where the winner fills the box.

**Mutation on, seeds 1-4, 150k:** main fights already (693-1,106 bite kills,
219-358 eaten); on, 933-1,237 bite kills, 222-287 eaten, cross-colony
contact 912-1,288 -> 204-301, living at 100-150k 398-466 on against 404-491
off. Kills inside one line: 0 on every seed with the switch on (4 on seed 4
with it off, by the mouth).

**The lineage path is now exercised, and it fires once.** Seed 1 at 300k,
mutation off: 70 kills inside a line (of 2,504 in that run; victims healthy,
median 456 J), **66 in line 1 and 4 in line 2**. No other run of the 24 at
300k, or of the 8 mutation-on runs, kills inside a line. The second lane
traced it (`review-default-on-pr673-2026-10-10.md` §2, from
`long-on-1.csv` and a local scent dump):

- The two nests' odours drift toward each other (mean cross-line distance
  1.334 at 222k, 1.067 at 240k, 1.11-1.17 over 246-264k, just past the
  tolerance radius of 1.0); 7-24 line-1 ants sit nearest line 2's nest
  throughout.
- By 270k `regroup_by_scent` mints label 4 out of line 1: 23 ants at 270k,
  55 at 300k, all nearest line 2's nest. They **wear line 2's nest odour**
  (mean distance to it 0.022 at 270k, 0.008 at 300k; to their own line's
  nest 1.33 and 1.70). The mirror happens once: label 3 is one line-2 ant at
  x 146 wearing line 1's odour, and it accounts for the 4 kills in line 2
  (267-277k).
- Every one of the 70 pairs is a scent stranger by the shipped test (pair
  distance 1.065-1.683, median 1.449). The kills go both ways: 13 label 1 ->
  label 1 before the mint, 30 label 1 -> label 4, 23 label 4 -> label 1.
  A two-sided fight at a scent line, not a colony eating its own; line 1
  ends with 294 ants, the largest group, and label 4 grows while fought.
- **Per-ant path not traced:** which of main's three paths moved those ants
  onto line 2's odour (`blend_with_nest`, the kin share blend, or
  `carry_nest_wander`). The exact endpoint points to a blend rather than
  wander [inferred]. Either way it is main's: off seeds 2 and 6 and on seeds
  6, 10 and 11 merge by the same machinery with no kill inside a line;
  seed 1 is a merge that stalled halfway (the gap reopens to 1.71 by 300k),
  and the switch makes the stranded pocket a fight.

**The owner's box (review item 3).** Logs: `herb-box/` in the shared folder.
Run on the branch before PR 675 merged, so main's switches here are the
pre-flip ones.

- *The 10-09 planted box* (`rivalry herbbox=1`: 13 plants, colonies landing
  at 83k and 85k, x 728 and x 284), seeds 1-2 to 145k: **the colonies never
  meet.** Each keeps to its own plants (columns 605-908 against 210-407),
  `touches=0`, and on plays byte-identical to off. Nothing for the switch to
  do there.
- *The 10-10 box* (`herb_box_fed`: colonies of 52 at x 296 and x 614,
  frame 58). Its food was painted with the FOOD brush, which the actions log
  does not record; one 3,000-cell provisions heap at x 455 is **a guess** at
  where. Finite food, so every arm starves out by 42-60k, as the playtest
  did. Seeds 1-2 to 60k:

| switches | arm | kills | meetings: parted / held / display / bite / kill | dead per line, starved / killed |
|---|---|---|---|---|
| main's | off | 0, 0 | none logged (259-290 cross contacts sampled) | 188/0 + 207/0; 140/0 + 125/0 |
| main's | on | 62, 67 | 136/10/18/20/62; 127/4/20/17/66 | 206/32 + 206/31; 136/30 + 146/37 |
| the playtest's | off | 0, 0 | none logged (1,153-1,306 sampled) | 454/0 + 351/0; 347/0 + 406/0 |
| the playtest's | on | 456, 356 | 446/61/116/79/446; 415/1,104/66/63/345 | 269/193 + 75/295; 202/167 + 198/211 |

  Off reproduces the complaint: the colonies meet and nobody bites. On, the
  kills come between the 24k and 30k samples (playtest switches) and the 42k
  and 54k samples (main's); the colonies' first touches are late too (27-29k;
  main's 8k on seed 1, one held touch, then 49k on seed 2) [inferred: they
  meet as the heap runs down and foragers range]. The playtest's set holds 1,104 of
  1,993 meetings on seed 2 by `fed` (8,058 held touches): its storeroom
  keeps food in the jaws, so many ants are laden. *Laden* is the part of
  `fed` this set leans on, which is why §3 now names it.
- Kept in the shared folder, not `assets/`: `herb_box_fed` (its heap is a
  guess) and `two_nests_own_late` (its 30k landing would lengthen the CI
  arrival test). The four beds in `assets/lab_scenarios/` re-run §9-§10.

## 11. On main with the nest stack on (2026-10-10, branch `63e7c502`)

Review item 2: PR 675 shipped the nest stack on (main `214311ff`), so the
whole set was rerun on the merged branch. Logs, CSVs and `sum300.py`:
`rebased-63e7c502/` in the shared folder. Same beds, same seeds, mutation
off unless stated.

**12 seeds to 300k** (`two_nests_own`):

| | before the stack (§10) | stack on |
|---|---|---|
| living 100-300k, median of seeds, off / on | 532 / 463 (-13%) | 512 / 502 (-2%) |
| paired, on against off, per seed | | -6.5% to +6.7% |
| kills on the five seeds that were borders before (1, 3, 4, 5, 8) | 2,241-2,800 | 1,402-1,930 (-27% to -46%) |
| at 300k, on | 5 borders, 3 conquests, 4 one-family | 8 borders, 2 conquests, 2 one-family |
| starved, on / off | | 64-400 / 99-477 |

- *Borders:* seeds 1, 2, 3, 4, 5, 7, 8, 10. Seed 1 still ends at 88%
  strangers between (a pocket mixing), as before.
- *Conquests:* seeds 11 and 12, both **landing clashes**: the second colony
  is gone by the 30k and 60k samples, 53 and 144 of its ants killed and none
  starved. No runaway or attrition conquest this time.
- *One family:* seed 9 from the start (no fighting, on = off), seed 6 merged
  by scent between 240k and 270k.
- Off, no colony is squeezed now: the smallest line at 300k is 188 ants
  (before the stack, 5 of 12 off seeds squeezed one colony to 9-41).

**Mutation on, seeds 1-4, 150k:** living 100-150k 452-576 on against
501-550 off; bite kills 942-1,276 on against 645-1,090 off, except seed 4,
where the second colony was wiped out at landing (55 killed, 0 starved,
63 kills in all) and the winner alone starved 898. Kills inside one line: 0
with the switch on; 2 with it off (seed 1).

**The owner's 10-10 box** (`herb_box_fed`, heap guessed, seeds 1-2, 60k):
off, 0 kills and every ant starved by 42k (359 + 376, 359 + 345); on, 264
and 190 kills from about 36k, everyone dead by 42k too. Meetings, parted /
held / display / bite / kill: 1,051 / 234 / 111 / 66 / 255 and 718 /
1,670 / 79 / 62 / 189. Dead per line, starved / killed: 286/93 + 196/189;
275/93 + 246/104. The stack's storeroom makes more ants laden, so `fed`
holds 9-61% of meetings here.
