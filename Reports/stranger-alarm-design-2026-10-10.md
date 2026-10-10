# Strangers start the fight: a design for touch-triggered alarm (2026-10-10)

*Proposal, not built. Owner asked for it on 2026-10-10 ("yes" to writing it
up for review). Project rule: trace, then proposal, then review by another
lane, then build behind an off switch, then test. This is the proposal;
the trace it rests on is §1. Main `e594d970`.*

**In one paragraph.** Since 2026-10-05 two ant colonies in a lab box never
fight, however often they meet. The only thing that ever started a fight was
an ant trying to *eat* a stranger, and the evolved lab ant's plant gut (-0.8)
no longer counts ant flesh as food. This proposes that **touching a stranger
of your own species is itself alarming**: an ant whose body touches a living
ant that fails its nestmate test marks the alarm plane where it stands, at
the strength a contest display already writes. Everything after that already
exists and is tested: the `(Alarm, Attack, 2.0)` wire, `nearest_foe`, the
contest's assess-then-commit, and the display that lets most meetings end
without a blow. It goes behind `PIXEL_PHYSICS_STRANGER_ALARM`, off.

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
`PIXEL_PHYSICS_FIGHT_RECRUIT`): its v3 recruits only from a bitten ant, so it
is inert while nothing bites first. It becomes testable once this lands.

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

**When an ant's body touches a living animal of its own species that fails
its kin test, it marks the alarm plane at its own cell, topping it up to the
display level and never above it.**

1. **The sense, for free.** `adjacent_food_counted` already walks the body's
   ring and asks `is_living_kin` of every attached organism cell, every tick
   (it is the walk behind `FoodAdjacent` and `KinNeed`). Where that test
   fails on a cell that `is_animal_cell` and whose species is the walker's
   own, set `FoodScan::stranger = true`. No new ring walk. `sense` must
   still book nothing, so the flag is carried to `act` beside the
   inputs `sense` already hands it, and `act` does the writing.
2. **The act.** At the top of `act`, before the fight branch: if `stranger`
   and the alarm at the ant's cell is below `contest::display_deposit()`,
   deposit the difference. **Topping up, not adding**, so a border of
   touching ants holds the plane at display strength rather than climbing to
   a wound's 240 by repetition. Alarm then reads 40/255 = 0.157, and the
   wire gives `squash(0.31) = 0.24`: about one decision in four rolls
   `Attack`.
3. **What follows is all existing code.** A winning roll walks
   `nearest_foe` (kin skipped), and the contest decides: commit and bite
   (`cry_alarm` at 240, nearby nestmates arouse) or withdraw and display
   (another 40 at the displayer's cell). A lopsided meeting mostly ends in a
   display; an even one or a crowd escalates. That is the graded middle the
   ethos asks for, and it was built and measured on 2026-09-14.
4. **Same species only.** Other species are predators or prey, and that
   question is the diet's (`PreyNear`, `is_visible_prey`). Mixing them in
   would make every ant that brushes a beetle start a brawl with it. A named
   part (`species`) lets a later arm include them.

### The switch

`PIXEL_PHYSICS_STRANGER_ALARM`, off by default, `World::stranger_alarm` for
one world (the house pattern). Parts, so each can be judged alone:

- `touch` — the rule above (the ignition).
- `level=N` — the top-up level, default the display deposit (40).
- `species` — also count other species' animals (off in `touch`).

`on` = `touch`. Counters: `CreatureStats::stranger_touches` (ticks the sense
fired), `stranger_marks` (deposits made). Pair them with the existing
`contests`, `displays`, `attacks`, `attack_kills` and `kills_log`'s
cross-colony split, per `CLAUDE.md`'s fired-counter / effect-counter rule.

## 4. What it could break, and the check for each

| risk | why it is plausible | check |
|---|---|---|
| **Own-colony fights from drift** | `scent_drift` 0.15 can make a lineage a stranger to its own colony; own-label kills were 24-30 per two-colony bed on 2026-10-03 (`killtrace`) | One-colony goal box, seeds 1-4, 300k: `stranger_touches` and own-label kills, on against off. Specificity bar: own kills not higher on any seed. If drift strangers are common the rule needs the `NEST_KIN_GATE` reference, not the body |
| **Border jams** | `Alarm` is `-1.0` on `Move`; ants held at display level step less | Steps per decision of ants within 6 cells of a stranger, on against off; picture of the meeting zone |
| **Nest disruption** | a stranger reaching a door alarms the doorway | Brood and door census on the two-colony bed, against off |
| **Colony collapse from war** | the gut-0 arm killed 44-51 a run on a starving bed | Trace who died, of what, where (`how-we-test.md` §2.3). A colony-killing rule stays off until traced |
| **Frame cost** | first contact allocates the alarm plane (40 MB at the outdoor world size; small in a lab box) | `ascii` worst frame unchanged (no stranger there); lab box tick time on against off |
| **Runaway** | display marks feed more attack rolls | Bounded by the top-up and the alarm's fast decay; confirm `attacks` per contact settles rather than climbs |

**Single-colony boxes, which are most lab boxes**, should be inert apart
from the drift case: no strangers, no touches. That is the first specificity
check (`stranger_touches == 0` on a one-colony bed with drift pinned off).

## 5. How it would be judged

Per `Reports/how-we-test.md` §2, on the evolved founder, mutation off:

1. **Direct metric.** Fraction of stranger contacts that lead to a contest,
   and of contests that lead to a bite, against the gut-0 arm as the
   reference for "fights happen". Off must read exactly 0 attacks (the trace's
   result), on should read attacks on every seed. Beds: `rivalry`'s default,
   `/mnt/project-files/colony-wars/scenarios/war_two.ron` (12 seeds to settle
   a default), and a herb_ant box with the owner's two placements (x 296 and
   614).
2. **Picture.** The meeting zone over time (`rivalry gif=`), so the owner can
   see whether a border forms and moves, which is the readout the tournament
   literature points at.
3. **Cost.** Colony size, deaths by cause and place, nest census, frame time,
   each per seed against off.

Then, as a second step and not part of this switch: the parked fight
recruiting on top of it.

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

## 7. Open questions for the reviewer

1. Is topping up to the display level the right strength, or should the first
   touch write less than a display (a stranger noticed, not yet a display)?
2. Should the touch also fire when the stranger is a *rider* (`riders_at`)?
   `nearest_foe` is deliberately blind to riders (the owner's "one attack
   must not hit 20 creatures"), so a touch-alarm from a rider would arouse an
   ant that then has no target.
3. Is same-species the right boundary, given `kin_crosses_kinds` exists for
   species that recognise across kinds?
