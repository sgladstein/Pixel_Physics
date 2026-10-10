# Ant rule audit: hard vetoes, colony loops, and a needs-first restructure

Rule audit thread, 2026-10-06 ~13:45 UTC. Read only; nothing built or run.
Code read at **main 3ba1e7bd** (PR 645 merged, `MOUND_OUT=dig` on), `src/sim/creature.rs`,
with `Reports/how-the-ant-works.md`, the shared findings file and Nest building's
soil-holder inventory (`nest-race/lane3/soil-holder-rules-2026-10-06.md`).

Every claim is marked **traced** (seen decision by decision in a named run, with
where) or **code-read** (from the source only, so a prediction, not a measurement).
Function names, not line numbers; lines in `creature.rs` go stale within a day.

Scott's framing (13:19): hunger and other survival needs are **base needs inside the
ant** that override any other rule, so no state (holding a pellet, a job) can keep a
starving ant from acting on hunger. Separately, features are judged by traces of the
designed behaviour, with colony numbers only as a check. This audit is about the
first; §7 carries the second as a judging rule.

## 0. Short answer

1. **The ant's walk has one pull slot, filled by a first-match ladder** (`home_pull`,
   then `hungry_out_pull`, `mound_out_pull`, `rest_pull` in `chooser_step`). Whatever
   rung matches first silences every rung below it **and** silences scouting, the
   only hunger-driven search an empty ant has. Hunger is not a rung that can win
   against a job; it is only what is left when no job matches. That is the hard veto
   in its general form. (code-read)
2. The ladder is safe today **only where a rung's own entry test implies the ant is
   fed or is eating its cargo** (laden carry, lay walk, nest-worker leash). It is
   lethal where a rung can hold a hungry ant: soil in the jaws (traced, in hand with
   Nest building) and **food in the jaws (store loads ride in the same slot as soil,
   code-read, not in the inventory)**.
3. Besides the ladder there are five more hard vetoes on survival actions (§2): the
   door throttle ignores hunger until half the grant; a lean ant may not dig even to
   get out; the heap cue refuses a breakout to the sky with probability 1 when no
   pellets lie beside it; `FaceTrip`'s `only` refuses door-clearing cuts; and the
   nest worker caste is fixed by id for life. Four of the five are traced killing or
   trapping ants in the findings file.
4. **No live rule draws an idle ant into the nest** (§3, code-read). Every live pull
   aims at the door, out, or the ant's last home contact, except a digger's walk back
   to its face. Time in the dug nest at 4-6% (traced, Deep trace baseline) is what
   that rule set predicts, not a mystery.
5. Colony loops (§4): the strongest **missing** feedbacks are laying keyed to space at
   the brood pile rather than to brood hunger, and the dig urge read from one
   colony-wide number, which gives every ant the same urge everywhere, so nothing can
   make a second chamber. The strongest **self-feeding** loop is the false roads.
   **Correction to the brief:** the "80% of digging is re-digging" figure is from the
   game before `SOIL_WAY`/`WAY_GAPS`/`FACE_TRIP`; with them (now main) re-digs were
   24-53% (traced, Deep trace 2026-10-05).
6. **Restructure, in one line:** needs first, then jobs, then a resting place, with
   needs that rise with the deficit and can never be zeroed by what the ant holds or
   what job it is on. Priority order in §6.

## 1. The ladder (code-read)

`chooser_step` takes the first non-empty of these, in order. "Silences" means: while
this rung matches, every rung below it and the scout term do not act.

| # | Rung (function) | Matches when | Aims at | Can it hold a hungry ant? |
|---|---|---|---|---|
| 1 | Store trip (`store_target`, `harvest_target`, `store_return_target`) | food held in the **jaws** as a store load, or walking back up | storeroom / mouth | **Yes.** Picked up fed (`store_pickup_ok`), but nothing checks hunger during the carry; it ends on 48 still decisions or patience. Shipped storeroom is `pile` (Deneubourg pick/drop near food), so `store_target` is `None`, but the load still sits in `state.spoil` and so meets every pellet gate below |
| 2 | Walk home to lay (`ready_to_lay`) | bank above the lay bar, not at home | anchor | No: its entry test means fed |
| 3 | Nest-worker leash | nest-bound, fed, empty, off home | anchor | No: entry test means fed |
| 4 | Soil out (`soil_way_pull`, else `spoil_haul_target`) | `spoil.is_some()` | door / along the way out | **Yes** (traced: 31-51% and 22-42% of hungry holder decisions, seeds 1-4; Nest building) |
| 5 | Back to the face (`dig_return_target`) | empty digger, energy above `DIG_RETURN_FED` (0.5) | its last cut | Only down to half the grant, then it ends (a floor exists) |
| 6 | Hungry walk home (`HUNGRY_HOME`, off) | empty, latched hungry, `spoil.is_none()` | larder | off on main |
| 7 | Laden carry | food in the crop, not a packed lunch | anchor | It eats its cargo as it walks, so not a starvation route |
| 8 | Hungry out (`hungry_out_pull`) | under grant, nothing in crop or jaws, on the nest's way in | door, along passages | It **is** the hunger rung, and its gain is the throttle's want, not hunger (§2 V2) |
| 9 | Mound out (`MOUND_OUT=way`, off) | as 8, in the mound | open air | off on main |
| 10 | Rest in (`NEST_REST`, off) | idle, want-out under 0.5 | in along the way | off on main |
| — | Scout and away terms | only if **no** rung above matched, not laden, no jaws load, and not a fed nest worker | out from home | — |

So hunger reaches the walk in two places only: rung 8 (inside, empty-handed) and the
scout term (when nothing else matched). Any rung above them that can match a hungry
ant is a veto. Rungs 1 and 4 can. Rung 1 is not in the soil inventory because the
inventory keyed on soil; it keys on the same field, so **the general soil-drop rule
should be written on "anything in the jaws", store loads included** (Nest building to
confirm their switch reads `spoil`, not `spoil` minus store loads).

## 2. Hard vetoes outside the ladder

Soil-holder vetoes (inventory rows 1-33) are in hand with Nest building and not
repeated. These are the rest. "Floor" is whether hunger eventually breaks the veto.

| # | Veto | What it switches off | Floor? | Evidence | Weight |
|---|---|---|---|---|---|
| V1 | **Jaws hold anything** (rungs 1 and 4; `act` returns from the spoil branch before the dig; `hungry_out_gain` returns `None` for `spoil.is_some()`) | dig, hungry out, scouting, hungry walk home | lean drop at half grant, needs an open cell with footing | traced for soil (findings 30, 31, 35); code-read for store loads | high, in hand for soil |
| V2 | **Door throttle** (`outward_want`) | within 16 cells (scaled) of the door, **including the top 16 rows of the nest**, an ant's own hunger is replaced by the colony's want (forage drive `met`, door scent, patrol). Rung 8 takes its gain from the same function, so a hungry ant at 0.5-1.0 of its grant in the upper nest is pulled out only if the colony is getting food | yes, at half grant (`LeanForage::out`): a step from 0 to full hunger | code-read; traced effects: findings 22 (newborns go out via rung 8) and 34 (pulled spells mostly refill without eating) | medium: the shape (colony want, local cue) is right; the cliff at 50% and the 16-row reach are not |
| V3 | **Lean ants may not dig** (`LeanForage::nodig`) | every dig, including the only way out of a sealed pocket | none, except the `MOUND_OUT=dig` exception for `shut_in_mound` (mound only; the shaft below the founding ground is outside it, traced seed 1, finding 36) | traced (findings 28, 57; `MOUND_OUT` 12 seeds: shut-in spells ending starved 1.30% -> 0.50%) | high. The exception proves the rule is wrong in kind: digging to build is a job, digging to get out is a need |
| V4 | **Heap cue at floor 0** (`spoil_cue_factor`) | a cut that opens to the sky with no pellets beside it is refused with probability 1. `DOOR_REOPEN` waives it only inside the founding cut | none | traced: 63 of 63 cuts at a plug refused, colony died (Laying, 10-04, finding 21); 219 of 598 rolls at a plug's inner face refused (seed 4, finding 21) | high for shut-in ants above the founding surface |
| V5 | **`FaceTrip`'s `only`** (`face_trip_refuses`) | a digger walking back refuses any cut more than 3 cells from its face, including the plug of a shut door | the walk back ends below half grant | traced: 737 refusals of the plug against 99 cuts (seed 1, rest pull, Nest building); 11 of 17 reopening rolls (seed 4, finding 19) | medium |
| V6 | **Fixed caste** (`is_nest_bound`: one ant in four by id, for life) | the forage drive (0), the away term when fed; leashed to the anchor | hungry nest workers scout like anyone | code-read | low for survival, high for design (§4 L6) |
| V7 | `Drop` closes below ~40% energy | putting food down | — | code-read | none: this one protects the ant (it keeps and eats its load) |
| V8 | `Feed` reads no hunger | — (the opposite: a missing term) | — | code-read; traced effect: every store load eaten on arrival, store held 0-1 cells (findings 24, 25) | medium for the storeroom goal |

V7 is listed to show the line: a veto that **keeps a hungry ant alive** is fine. The
test for a bad one is whether it can hold a hungry ant away from food or out of open
air with no floor short of death.

## 3. Why ants don't live in the nest, from the rules (code-read)

Scott's number-one issue. Every live pull on main aims at one of: the door (rungs 4,
8), out (rung 8, scouting, door reader, forage drive), or the ant's anchor (rungs 2, 3,
7). The anchor is the last cell the ant stood on beside home; inside the dug nest it
is the ant's own cell (traced, finding 1), and for a forager coming in from outside it
is the doorstep. The only live pulls that end deep in the nest are rung 5 (back to a
dig face, nest diggers only) and store trips. Everything that drew ants inward is
off: `NEST_REST`, `NURSE_SEEK`, `CROP_NURSE=on`, `CROP_DOWN`, `NURSE_STAY`. Each was
turned off because it killed or shrank colonies, and every one of those deaths traced
so far ran through V1 or V3 (findings 27, 31).

So the 4-6% in the dug nest is the expected output of the rule set: nothing an idle
ant feels says "inside". In real colonies a large share of workers at any moment are
inactive: "most social insect colonies contain large numbers of highly inactive
workers", which act as a reserve (Charbonneau, Sasaki & Dornhaus 2017, PLoS ONE
12:e0184074, doi 10.1371/journal.pone.0184074, via PubMed), and workers hold spatial
zones in the nest (Mersch et al. 2013). That the inactive ones sit inside is my
reading, not the paper's claim. **The designed behaviour needs a resting place, and the resting
place can only be safe once needs can override it**, which is why the order in §6
puts needs first and the rest pull second.

## 4. Colony loops

For each: what it is, the evidence, the **local** signal an ant could really sense,
and a global stopgap only where I could not find a local one.

**L1. Laying follows space at the brood pile, not food or brood hunger (missing
negative feedback).** Traced: about 50 ready layers wait at every frame
(`buds_held_for_nest`), and 65-76% of eggs go into a cell brood just fell out of or
hatched from; eggs track brood falls (r 0.81 over 34 runs), causal link inferred
(Deep trace, `nest-plan-switches-2026-10-05.md`). Larvae starved 295-471 per 100k
against 1.7-1.9k eggs on the baseline (traced). The `Lay` output is inert (memory:
lay-hold-line) and `FOOD_BRAKE` stayed off. **Local:** a layer at the pile already has
a reading of how hungry the larvae round it are (`brood::larva_scent`, hunger-weighted,
built for nurses). Lay less where that scent is strong: hungry larvae near the pile
mean food is not reaching brood. This is close to what real colonies do, where brood
hunger is signalled to workers and laying follows what the layer is fed (stated from
the general literature, not checked against a paper here). No global needed.

**L2. The dig urge is one colony-wide number, so the nest cannot branch (missing
spatial feedback, global signal).** At the nest `Crowding` is the whole nest's roofed
void over its ants (`NestRoom::occupancy`); every ant reads the same value to four
decimals (code-read, the doc on `crowding_local`). That is a real negative feedback
(more room, less digging) but it has no place in it, and
`Reports/nest-biology-2026-09-19.md` §4.1 already names local worker density at the
digging face as what turns a round cavity into a branched nest. **Local version
already built:** `PIXEL_PHYSICS_CROWDING_LOCAL=near|wide`, off. Its doc says why it
was not shipped: `(Crowding, Move, -0.3)` reads the same slot, so changing the slot at
the nest re-points that weight too. The fix is a separate input for the dig
(local density at the head), leaving `Crowding` alone. This is the most direct lever
on "separate chambers" in the audit.

**L3. False roads (self-feeding).** Traced: over the mound, nest scent lies even and
the trail hold keeps a carrier's heading about four times over, so 84-89% of carries
lose the pull home at least once and 51-67% of the food trail is laid by carriers
that have lost it (finding 16). That trail then recruits: empty ants follow it (away
term), the door reader turns toward it, and door scent raises the throttle's want
(code-read). Bringing lost carriers home cost colonies, because the lost ones are the
ones who filled up (finding 16). **Local:** (a) lay the food trail in proportion to the
carrier's own home patience, its sense of making progress home, so a lost carrier
stops marking (its patience is its own state; trail laying on the way home is the
real pattern). (b) Later, a "nothing here" mark: Pharaoh's ants lay a repellent at
unrewarding branches (Robinson et al. 2005, Nature 438:442, doi 10.1038/438442a, via PubMed). Our scouts
already give up at a lit trail's dark end (`giveup`); that moment is where the mark
would go. (c) `CARRY_HOME=turn` (off) is the existing patch on the hold itself.

**L4. Mound churn and re-digging (self-feeding, smaller than briefed).** Pellets
leave by the door, the mound grows, loose soil slides back over the mouth and into
pockets, and is cut again (traced: 4,775 cells filled by falling or sliding soil over
176-200k against 546 pellets put down, seed 4, finding 17). With the three nest-plan
switches now on main, re-digs are 24-53% (not 80-85%) and new-ground cuts rose
(traced, 2026-10-05). **Local:** deposited soil carrying a building scent that attracts
more deposits nearby and fades (Khuong et al. 2016, PNAS 113:1303, doi 10.1073/pnas.1509829113, via PubMed; the paper also finds the scent's lifetime controls the nest's form), so pellets
pile where pellets are, away from the mouth, rather than wherever the carrier gave up.
The ring (`SPOIL_RING`) is the existing geometric stand-in. Lower priority than L1-L3.

**L5. Food recruitment (healthy, the model to copy).** The `met` forage drive is
local (an ant counts its own meetings with returning foragers), graded (plateau then
fade), and has its own negative feedback (no returns, no drive). Gordon 2013 is the
biology. Keep it; use it as the template for L1-L3.

**L6. Roles are assigned, not earned (missing feedback, design).** One ant in four
by id is a nest worker for life (V6). The response-threshold model (Bonabeau,
Theraulaz & Deneubourg 1996, in project refs) and age polyethism (Mersch 2013) both
let the share of inside workers follow what the colony needs. Not urgent; listed
because chambers and a nest population will want it later.

**L7. Adults and larvae draw on one pool with no priority (missing feedback).**
Traced: nurses moved crop food from adults to larvae, adults were topped up half as
often, and the hungry then died in V1 and the false roads (finding 27). Shares are
local and graded already; what is missing is the donor weighing an adult's hunger
against a larva's. Once V1 and V3 are fixed this may matter less, which is the point
of fixing them first.

### Global signals in use today, and their local versions

| Signal | Used by | Local version |
|---|---|---|
| Nest room census (one number per nest) | `Crowding` at the nest, so the dig urge | local density at the head, built and off (`CROWDING_LOCAL`), needs its own input slot |
| Door scent read from anywhere in a 33x33 box | throttle want, so rung 8's gain | read trail B at the ant's own sensors, as the door reader already does |
| Nest's way in / dug home / soil way (breadth-first maps) | rungs 4, 8, 10, `AtNest` | already argued as a stand-in for the nest-air gradient (Cox & Blanchard 2000 in `hungry_out_of`'s doc). Accept as a labelled stand-in |
| Distance to the nearest breeder | graded breeding suppression | a fading breeder scent; the distance already behaves like one. Accept |
| Food distance from every door (trip reach) | marks a trip load | the ant's own distance from its last nest contact, which it already keeps (`forage_max`) |
| `nest_need` hunger / larder | forage drive `hunger`/`larder` | off on main; `met` replaced it |

## 5. The rule I'd add for every new rule: no veto without a floor

A rule may **gate** a survival action (eat, walk to food or out, drop what is held,
dig out of an enclosure) only if, in the same change, it names the floor at which
hunger (or being shut in) breaks the gate, and that floor is above death by a margin
the ant can act on. Concretely, two checks:

1. **Review check.** Any new `return None`, gain 0, or skip keyed on what the ant
   holds, its role or its job states its hunger exit in its doc comment.
2. **A guard test** (to build, one table): for each state on the ladder and in §2
   (holding soil, holding a store load, on a face trip, nest worker, in the door
   zone, encased in the mound, below the founding ground), make an ant at 30% of its
   grant and assert that within a bounded number of ticks it has dropped the load,
   or has a pull or dig toward open air or food with gain above 0. Put the fault back
   (restore one veto) and watch the row go red. This is the sensitivity control the
   repo asks of every guard.

And a **rule card** for each new rule: which designed behaviour it serves (in the
nest, chambers, foraging), the trace that shows it working, and which base needs it
can delay. A rule whose only card line is "colony survives" gets questioned.

## 6. Restructure and priority order

**Shape: needs, then jobs, then a resting place.**

- **Needs** (hunger now; escape from an enclosure; later heat and water). Each is an
  urge that rises with the deficit, smoothly, never a step at one line. While a need
  outweighs the current job, the ant first **puts down what its jaws hold** (where it
  stands; packed into a wall if there is no open cell, as Nest building is building),
  then acts on the need: hungry out, scout, eat, or dig out.
- **Jobs** (carry soil out, carry food in, dig, nurse, walk home to lay). Weighted by
  their own urges as now, and they **yield** to needs instead of being checked first.
- **Resting place** when no need or job pulls: inside the nest, by cues an ant can
  sense (where nestmates rest, the nest air, brood nearby).

The smallest change to the code that gets this shape is not a rewrite: give
`chooser_step` a hunger urge that is compared with whichever rung matched, and skip
the rung (and drop the jaws load) when hunger wins. The ladder stays for jobs.

**Priority, each step judged on its own designed metric, traced, before the next:**

1. **General drop rule for anything in the jaws** (in hand, Nest building). Ask them
   to cover store loads, which share the slot. Metric: hungry holders with a pull to
   food; shut-in spells ending starved; door seals per seed.
2. **Needs floor on the other vetoes**, one switch with named parts: `throttle`
   (hunger blended with the colony's want instead of a step at 50%, and the door read
   at the ant's own sensors); `escape` (a dig to get out, keyed on being enclosed with
   no open way, allowed when lean and exempt from the heap cue's zero and from
   `FaceTrip`'s `only`; generalises `MOUND_OUT=dig` to the shaft, where seed 1's 22
   died). Metric: shut-in spells ending starved, per seed, against the dig-on baseline.
3. **The no-veto guard test** (§5). Small, and it stops the next switch reintroducing
   the pattern.
4. **Resting place inside**: re-test `NEST_REST` (and nurse stay-in) on top of 1-2, as
   already planned, scored on time in the dug nest. This is the designed behaviour;
   1-3 make it safe to try.
5. **Local dig density** (L2) for chambers: a new dig input reading density at the
   head, `Crowding` untouched. Scored on separate rooms (30+ cells apart) and a picture.
6. **Laying keyed to larval hunger at the pile** (L1). Scored on larvae starved per
   egg and eggs per food brought home.
7. **Trail laid by patience** (L3a). Scored on the share of the food trail laid by
   lost carriers, and heap intake.
8. Later: building scent on soil (L4), thresholds instead of a fixed caste (L6),
   donor weighing adult against larva (L7).

**Robustness score, for steps that flip a default.** Four seeds at the standard setup,
plus each of the rule's two or three key constants at ±10%, to 300k. Report the
designed metric per run and flag the rule fragile if any perturbed run collapses
(under 300 ants) or flips the verdict. Cost: about 4 x 7 = 28 runs, roughly two to
three hours on four threads at today's 300k run times; so only for defaults, not
for iteration.

## 7. Judging rule (Scott's first point, kept separate)

A feature is judged by traces of the behaviour it was designed for; colony size is a
check that must not fall beyond noise, never the target. Needs-first is what makes
that safe: once no state can hold a starving ant, a feature that shrinks the colony
has a cause worth tracing rather than a trap that ate it.

## Corrections this audit makes to the brief

- "80% of digging is re-digging" is the pre-`SOIL_WAY` game. On main's switches,
  24-53% (traced, `deep-trace/nest-plan-switches-2026-10-05.md`).
- The soil inventory keys on soil; store loads ride in the same `spoil` field and
  meet the same vetoes (code-read).
