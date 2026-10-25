# Colony mind seat: adversarial feasibility review

*2026-10-25. Reviewer: a fresh agent that did not write the design. Repo `main` at 9e32979e. Nothing changed, no
cargo runs (another build held the tree), so **every behavioural claim below is from code and weights, not from a
run**. Where only a run can answer, the item says so and names the measurement. Under review:
`designs/seat-colony-mind.md` (revised) and its critique `designs/seat-colony-mind-critique.md`.*

---

## The answer in plain words

**Not feasible as written. A smaller demo is feasible if it changes, and only if one day of measurement says so
first.**

The design and its critique were both checked against the **hand-built ant**. The lab no longer runs that ant: since
2026-10-24 every lab ant decides with the **trained mind** (the standard pool) from the first frame. The trained mind
**does not read the forage drive or the "met a returning forager" stamp at all, and a trained ant's delivery does not
even write the stamp.** So the anchor verb (swell the news) and its partner (hush the door) do nothing on the game's
ant. This is not the "it reaches ants that cannot answer" problem the critique found. Nothing reads it, for anyone.
The critique's fixes (mark it new, a recruit path, beat RECRUIT's harm) are real, but they are fixes to a code path
the default game no longer runs.

What the trained ant *does* react to is what it senses each decision: scent A, B and alarm at each option, warmth,
who is beside it (hungry nestmate, larva and how hungry, corpse), its own energy and crop. So the honest version of this
seat is **a brush that turns the volume of those senses up or down in a place** ("signal gain"), plus alarm written
to or wiped from the real alarm plane, plus a favour on who lays. All of these are small hooks. **Whether any of
them makes ants do something you can see is unknown**, and the weights are not promising:

- Read linearly, the pool **steps away from alarm** and its job head turns walking, digging and feeding *down* when
  alarm is present. Echoing a cry probably makes the nest **scatter or freeze**, not mob. That is still a visible
  answer, but not the one the design's 4:30 scene sells.
- The training bed never had a predator, and trained ants now **lay alarm themselves as background chatter**
  (about 280,000 units per colony per 100,000 frames). To these ants alarm has never meant danger.
- The pool's weight on a hungry larva beside it is **negative**, and its weight on a corpse target is strongly
  negative. "Make the brood beg" may push nurses away, and "forbid the carrion" forbids something the ants seldom do.
- In the lab, minds **do not mutate at birth** and the 30 pool members are near-copies (8 to 107 of 3,285 slots differ
  from member 0). Favour can only re-weight 30 near-twins. **The hearing chart cannot fall**: there is no meeting bar
  in the genome, and nothing to select it from.

**Recommended demo, if step 0 passes:** "The colony's voice": four verbs on today's trained ants.
1. **Echo / swallow a cry** (alarm written or wiped over a disc, sustained while held).
2. **Close the shared stomach** (no mouth-to-mouth feeding inside a swept area), with a hunger tint so you can see it.
3. **Swell a trail**: louder food-trail scent laid by a laden forager you press on. This replaces "swell the news".
4. **Favour a line**: lines you favour lay, the others hold their eggs, shown with the lineage overlay.

Every verb has a counter in a corner. Before building any of it, spend about a day measuring whether each verb
visibly moves ants. **If fewer than two verbs pass, do not build a standalone demo.** Fold favour into the queen seat
(who lays) and alarm into the enemy seat, and offer this thread's hands there.

---

## 1. Which ant the lab runs, and why it decides everything

- `src/bin/lab.rs:297-306`: unless `-- oldant` is given, `lab.set_new_ant(true)`, so every ant gets a mind from
  `training::new_ant_minds()` (`src/lab/training.rs:386-394`, the 30-member `assets/minds/std_pool.txt`, id % 30).
  The field doc at `src/lab/mod.rs:308-311` ("Off is the shipped game") is stale for the binary; harnesses that
  build a `Lab` directly still start on the old ant.
- `src/sim/creature.rs:7998-8022`: an ant with a mind returns into `mind::tick` **before** `sense`, `brain.rs`,
  `act` and `chooser_step`. Only falls, bills, digestion and birth run for it in the body.
- `Reports/how-the-ant-works.md` §4-§12 describe the hand-built ant. §13 calls the mind "harness only" and says
  `MIND_ATTACK` is off. Both are stale: the lab runs minds, and `options::attack_on` defaults on
  (`src/sim/creature/options.rs:397-404`). The design leaned on §6d. Its claims about drives, nurse workers and the
  lean rule are true of an ant the game no longer shows by default.

## 2. Verb by verb

Codes: **hook** = a small hook on behaviour the trained ant already has. **engine** = real new engine work.
**no** = cannot work as described.

| Verb | Verdict on the default (trained) ant | Why, briefly |
|---|---|---|
| Swell the news | **no** (as described); **engine, large** to make it real | Mind never reads drive or stamp; deliveries by minds never stamp |
| Hush the door | **no** | Same: it hushes a signal nobody hears |
| Echo a cry | **hook** (half built) | Lab already has an alarm tool; the response is unmeasured, and the weights point to scatter or freeze |
| Swallow a cry | **hook** | Wipe the plane in a disc while held; visible only if echo shows a response |
| Make the brood beg | **engine, small; probably backwards** | Nurse scent is read only by the old walk; pool's larva-hunger weight is negative |
| Open the shared stomach | **hook, weak** | Can scale the amount given, not the choice to give; needs a hunger tint |
| Close the shared stomach | **hook** | Drop Feed options in the area; clean; needs a hunger tint |
| Forbid the carrion | **hook, low visibility** | Drop corpse pick-ups in the area; ants seldom eat corpses (pool weight on corpse targets is negative) |
| Shift the nursery | **engine, medium** | Minds cannot pick up brood; the body's carry rule is off and has no warmth term |
| Spend the reserve | **no** | The store rules live in `act`; a mind takes store food like any food |
| Lean on a habit | **engine, large** | No habit state; a new sense enters at weight 0, so it needs training |
| Bless the answer | **hook** for laying; **no** for "answerers" | Favour can gate laying per line; there is no "answered a swell" to credit |
| Police an egg | **engine, small** | Remove an egg by hand = a selection click; no worker policing rule exists |
| Call a quorum | **engine, large** | No emigration |
| Step down into one ant | **engine, medium** (shared) | Needs `shared/steer-mind.md` (enemy seat): steer reads nothing in `mind::tick` |

### 2a. Swell the news and hush the door: the stamp and the drive, and who reads them

**Writers of the stamp** (`OrganismState::return_met`):
- `meet_returning_forager` (`creature.rs:26159-26176`), a 7x7 scan at `RETURN_MEET = 3` (`creature.rs:25427`;
  the design cites :25412, drifted). It is called from `act` (`creature.rs:21778`) and from `relay_crop`
  (`creature.rs:16916`, itself called only from `act` at `:21559`).
- `meet_on_way` (`creature.rs:26214-26240`, `PIXEL_PHYSICS_MEET_WAY`, off), called at `creature.rs:8188`, after
  the mind's early return.
- RECRUIT's stamp (`creature.rs:26491`), off.
- **None of these runs for a mind ant.** `mind.rs` and `options.rs` contain no `return_met`, `meet_returning`,
  `forage_drive` or `colony_drive` (grep: zero hits). The mind's put-down sets `state.foraged = true`
  (`mind.rs:2833`), and nothing it runs reads that either.

**Readers of the drive** (`forage_drive_level` at `creature.rs:26097`, then `colony_drive_level` at `:26108-26135`):
- the step chance in `creature_tick`'s hand path (`creature.rs:8380-8400`);
- `act`'s feed urge and dig gate (`:20464`, `:20527`);
- `outward_want` (`:27027`), `throttle_lifted` (`:27071`) and `store_out_pull` (`:15985`).

All of these are hand-built-ant code. The mind's context senses are `mind.rs:72-150` (energy, crop, spoil, at nest,
three scents and their changes, home, since nest, age, self, crowd, depth, roofed, roof cut, hole drop). There is
**no drive, no met, no recruit**.

**So on the game's ant there is neither a writer nor a reader.** Three ways to make a swell real:
1. **Run the demo on `-- oldant`.** The drive exists there, but the critique's item 1 still holds: the mound's idlers
   are never-foraged ants and nest workers who read 0 (`creature.rs:26111-26119`). The owner made the trained ant the
   default the day before this design, so a demo on the old ant shows a game the owner has moved away from.
2. **Give the mind a "met" sense.** A new input enters every saved pool at weight 0. This is the `FOOD_WAY`
   precedent (`mind.rs:528-545`): zeroed so stale weights cannot act at random. Nothing responds until a training
   batch selects for it, and selection needs a bed where meetings carry information. That is days of compute and
   design, with an unknown result. **Large.**
3. **Override the mind's pick** (`shared/steer-mind.md`'s forced pick) for ants in the ring. That is a puppet, and it
   contradicts the seat's own premise ("you never give an order").

**The substitute recommended here: "swell a trail".** A laden mind ant lays scent B through its emit row (mind
`EMITS` = A, B, alarm, `mind.rs:65-69`; member 0's B emit bias is -2.0, so it lays B only when its inputs lift it).
Pressing on a laden forager multiplies the B she lays for a while. That swells a signal she is already giving, which
is exactly the seat's fiction.

The pool does read B: its options carry B strength and B two cells ahead. Linear contributions over all members:
B strength -49, B rising two cells ahead +68 (`options.rs:1019-1023`). The context B weight is +24 into the
scorer, and member 0's Eat job weight on it is +3.4. **Whether a louder trail sends anyone out is unknown.** Slice 0
found a hand-laid trail pulled no workers, but that was 2026-10-11, on the old ant. It must be measured on minds.

### 2b. Echo and swallow a cry: the alarm plane, and whether trained ants answer it

- **The plane.** A bite writes 240 of 255 (`pheromone.rs:347`, `cry_alarm` at `creature.rs:11222`). It spreads by
  falloff of 12 per cell, reading 171/119/80/51/12 at 1/2/3/4/6 cells (`pheromone.rs:349-392`). It decays by
  `ALARM_RHO` 0.35 and **empties in about 144-190 frames** (`pheromone.rs:324-336`, `:381-383`). One echo is a
  1.5-second flash. "Hold to echo" has to re-deposit every frame while held, over a disc. That is cheap.
- **Already in the lab.** `Lab::alarm_at` deposits `ALARM_DEPOSIT` at a click (`src/lab/mod.rs:2937-2945`). Echo is
  that tool with a radius and a hold. Swallow means writing 0 into a disc every frame while held. Bites landing
  inside the disc are wiped the next frame, so swallow is a mask, not a fight with the bite.
- **Does the pool answer?** All 30 members wire alarm in. Linear contribution (sum over scorer units of output
  weight times input weight):
  - alarm strength at the option: -19.6;
  - alarm rising one cell on: -16.8;
  - alarm rising two cells on: about 0 (one member +50);
  - member 0's job rows on alarm here: Walk -1.14, Dig -2.81, Feed -1.08.

  Read plainly: **step away from alarm, and do less while standing in it.** That is a scatter or a freeze, not a mob.
  Squashes make the linear reading approximate, and only a run can say. The bite itself is a mind verb (Attack, on),
  but every pool member's job head has constant 0 on Attack, and no feature into it was selected against predators.
- **What alarm means to these ants.** The training bed has `predators: 0` (`src/train/challenges.rs:113`). With the
  bite on, the colony bed's minds emit alarm (`challenges.rs:2703-2709`), and the colony-vs-colony report §7 measured
  about 280k alarm per colony per 100k frames whether or not a neighbour was there. Alarm is chatter. **An echo will
  not "teach" anything that lasts**: lab minds do not mutate (§2e).

### 2c. Feeding and a per-area gain (the shared stomach)

- **The Feed act** (`mind.rs:3026-3103`). To a poorer adult it gives `SHARE_FRACTION` of the **bank** gap (energy
  bank to bank, not crop food). To a hungry larva it gives crop food, else bank. The option is listed only when it
  would give something (`options.rs:880-905`). "Who, and whether, is the mind's; the amount is fixed"
  (`mind.rs:3031-3033`).
- **When the pool feeds.** Member 0's Feed job row: energy +12.45, crop -11.37, at nest -11.22. It feeds when rich,
  with an empty crop, away from the nest. An older pool spent 24% of its decisions on adult sharing
  (`options.rs:892-894`). The hungry-nestmate feature defaults to a flag (hunger above 0.5, `options.rs:1080-1086`),
  and on the training bed it "fired on no option at all", so the preference for feeding the hungry was barely
  selected.
- **Hooks.** *Close*: skip pushing `Verb::Feed` for an ant whose head is inside a player zone (one predicate at
  `options.rs:903`). That is clean, because it is a legal-move filter and the body already filters. *Open*: scale
  `SHARE_FRACTION` in the zone (`mind.rs:3068`). That changes amounts, never choices, so it is weak. A third,
  seat-true form is a gain on the **hungry** feature for targets inside the zone, which swells the begging signal.
- **Visibility is the real cost.** Bank energy has no picture. The lab's organism overlays (`render.rs:1071` on:
  plant health, sap, cell type, resource, canopy, veins, moisture, food value, gut bias, stress, lineage) have no
  per-ant hunger tint. One is needed: about a day, and drawn only while toggled (dirty-rect cost).

### 2d. Eating the dead, and a forbid hook

- Corpse pick-up is on with the bite (`options.rs:410-413`, gate at `:846-853`). The hook is to skip `PickUp` on a
  corpse cell inside a zone. **Trivial.**
- **But the ants seldom do it.** The pool's linear weight on a corpse target is -40.6, and on `DEAD` -1.75. With
  corpse pick-up off, garden food moved only 1-13% (lone-predator report, lines 116-117). The verb forbids something
  barely happening, so the player sees a body lying where it would have lain anyway. **Include it only if step 0
  counts real corpse pick-ups near the nest.**

### 2e. Who lays, and favour per line

- **Laying is the body's.** In `try_bud`, an ant whose bank plus reachable provision meets the bar lays
  (`creature.rs:~5727-5760`). The mind's lay gate (`PP_LAYGENE`, on) raises its own bar (`mind.rs:1825`). The
  precedent for an outside brake sits right beside it: `World::colony_lay_scale`, per colony, harness-only
  (`world.rs:4885-4890`, read at `creature.rs:5749-5755`).
- **A favour hook** keyed by line (the inherited `Arc<MindGenome>` pointer, or the lab's lineage id) in the same spot
  is **about 30 lines, clean, None = off**. Disfavoured lines need, say, 8x the bar and in practice hold their eggs.
  That is graded, and visible with the existing lineage overlay. The `shared/who-lays.md` father-store (queen seat)
  is the other route, and the mind seat agreed to use it.
- **The catch, for the slow clock.** Lab minds **do not mutate at a birth**: "a mind does not mutate at a birth in
  the box" (`src/lab/mod.rs:1026-1030`), and `inherit` copies the genome unchanged (`mind.rs:1727-1740`).
  `shared/who-lays.md` says "with mutation", which is wrong for lab minds. The 30 members are deltas of member 0
  (`training.rs:96-97`) differing in 8-107 of 3,285 slots. Favour therefore shifts a mixture of near-twins, and
  **no genome slot holds a meeting bar**, so the design's hearing chart has nothing to plot. To make selection show,
  `egg_genome` would have to mutate (or cross), which is the queen plan's call, and the effect needs paired-seed
  measurement before it is promised.
- **No "answerer" to credit.** With no swell, there is no "ant that answered". Favour has to be pointed at directly
  (click a line or an ant) or tied to a measurable act (deliveries). The design's own surprise warns about the second:
  courtiers at the door, and carry-out cheats.

### 2f. Brood carrying and the pile

- Minds **cannot pick up brood**: `PickUp` needs a food cell with no organism in it (`options.rs:838`, `:846-853`).
- The body's carry rule `brood::carry` (`brood.rs:480-520`, called from the brood tick at `brood.rs:938`) uses any
  grown kin nestmate touching the item, so it would apply under minds too. It is **off**, as "close to inert"
  (`brood.rs:454-466`), and its "better place" is next to more brood, with no warmth or player term. Eggs are placed by
  `pile_site` (`brood.rs:397`), which has no warmth term either.
- "Shift the nursery" means a new carry target, which is a body rule doing the player's will. **Medium work, and a
  puppet**, unless it reads a real quantity.
- **The interesting fact: the pool's largest single linear weight is on warmth, +91.** The feature is "warmer there
  than here", squashed (`options.rs:1139-1144`). Trained ants probably drift toward heat. A real heat source (one
  physics) might gather ants without any hook. That belongs to the keeper's lamp more than to the mind, but it is the
  cheapest visible "voice" in the box, and step 0 should test it.
- The nurse scent (`brood::larva_scent`, `brood.rs:1392`) is read only by the hand-built walk (`creature.rs:16162`,
  `:28024`). The minds' only larva sense is per option, on the cell beside them (`LARVA`, `LARVA_HUNGER`,
  `options.rs:1071-1074`), with a linear weight of -26.

### 2g. Verified critique items, re-read for the trained ant

- Item 3 (the caste ships on): **true for minds too**. `PP_MIND_CASTELESS` defaults off (`mind.rs:1642-1670`), so mind
  ants are still stamped one in four.
- Items 1, 2, 4, 5, 8, 9: the fixes are correct statements about the hand-built ant, and **moot for the default
  game**, where the deeper fact is §2a.
- Item 6 (echo by area or decay): correct. Add that the plane empties in about 150 frames, so a hold has to sustain it.

## 3. The smallest honest demo, and its work

The rule for "visible": a verb passes only if, in a paired run (verb on against off, same seed, about 8 seeds), the
ants in or near the zone **change a counted behaviour by an amount you can see in a GIF**, and the change grades with
hold time or radius.

| # | Verb | Hook | Counter shown | Work |
|---|---|---|---|---|
| 0 | **Measure first** | `examples/` harness on the lab bed with minds: alarm disc held for 0/60/240 frames; trail-B gain x1/x3/x10 on laden ants; Feed filter in a chamber; corpse pick-ups near the nest; a heat patch | ants leaving or entering the zone per 100 frames, walks/rests/digs in the zone, feeds, corpse pick-ups | **~1 day** |
| 1 | Echo / swallow a cry | generalise `Lab::alarm_at`: disc radius, re-deposit while held; swallow writes 0 in the disc while held | ants in the disc, and their verbs, before and after | ~0.5 day |
| 2 | Swell a trail | `World::signal_gain` zones/ids (None = off), multiplying a laden ant's B emit; the same struct can gain sensed features | ants walking out along it in the next 2,000 frames | ~1 day with tests and a byte-identical guard |
| 3 | Close the stomach | skip `Verb::Feed` for heads in the zone (`options.rs:903`) | feeds in the zone; the spread of energy in the zone | ~0.5 day, plus **~1 day** for a hunger tint |
| 4 | Favour a line | per-line bar scale beside `colony_lay_scale`, or the who-lays store | eggs per line; lineage overlay | ~1 day (less if who-lays lands first) |
| - | Voice cost, HUD, demo scenario | delete food cells per second of hold; `demo=mind` via `shared/launcher.md` | store size | ~1 day |

**About 5-6 working days after step 0**, if step 0 passes. Forbid carrion (~0.5 day) is added only if step 0 counts
corpse eating. Swell the news as designed, habits, quorum, policing and the nursery stay out of the demo.

## 4. Risks the design understates

1. **The design is written for an ant the game no longer runs** (§1). Any plan built from it inherits the error.
2. **The anchor verb sends nobody, on any ant.** On minds nothing hears it. On the old ant only ants already
   stirred hear it.
3. **The reactions the remaining verbs rely on were never selected.** Alarm never meant danger in training, and
   the hungry-nestmate flag fired on no option on the training bed. A response may exist and still be noise:
   it differs member to member, it is not graded, or it runs the wrong way (larva hunger, corpse).
4. **Nothing persists.** Minds do not learn in life and do not mutate in the lab. When you let go, the colony is
   exactly what it was, except for who laid. "What you amplify is what you breed" reduces to "who you favour lays",
   which is the queen's verb.
5. **Minute to minute: one short flash and a lot of watching.** An echo lasts about 1.5 s unless held. The stomach's
   effect plays out over thousands of frames. Favour shows a brood cycle later. Without the swell there is no
   "whistle and the crowd answers" (the Pikmin beat the design leans on).
6. **Frame cost.** The hooks are cheap when `None` (one Option read per option build). The **hum overlay** is the
   cost: animated over the whole box, it defeats the dirty-rect skip on a settled world. Draw it only while a key is
   held, and quote `examples/ascii.rs` worst-frame before and after. A per-ant hunger tint has the same rule.
7. **One recipe.** With alarm likely to mean "scatter", the dominant play is "swallow everything, favour the
   biggest deliverers". Nothing in the demo punishes it.

## 5. Verdict

**Not feasible as written. Feasible with changes, as a four-verb "colony's voice" demo on the trained ant, gated on
a one-day measurement.** The changes:

1. Re-base the seat on the trained ant. Drop swell-the-news, hush, spend-the-reserve, habits, quorum and policing
   from the demo. List "a met sense for minds plus a training batch" as the long-term path, if the owner wants the
   recruitment loop.
2. Replace the swell with **swell a trail** (a louder B emit from a pressed laden forager). It is the only swell with
   a sense on the other end.
3. Build the verbs as **gains on signals the ants already sense** (the alarm plane, B, the Feed legal move), plus
   **favour on laying** shared with the queen's who-lays piece.
4. Drop the hearing chart from the demo. Lab minds do not mutate, and no gene holds a meeting bar. Show favour as eggs
   by line instead.
5. **Run step 0 first.** If fewer than two of {echo, swell a trail, close the stomach} change counted behaviour
   visibly and gradedly, stop. Write it up, give favour to the queen seat and alarm to the enemy seat, and help
   there.
6. Fix the stale docs this found: `how-the-ant-works.md` §13 (minds run in the lab, `MIND_ATTACK` defaults on),
   `src/lab/mod.rs:308-311`, and `shared/who-lays.md`'s "(with mutation)".

*Method notes: weights were read from `assets/minds/std_pool.txt` (30 members, genome 3285, layout 4; members are
deltas of member 0) with the slot layout of `mind.rs:160-238` (scorer S_CTX=23, S_FEAT=583, S_VERB=1303,
S_OUT=1447, JOB=1464; 2,797 core slots, the rest dig-head riders). "Linear contribution" = sum over the 16 scorer
units of (unit output weight x input weight). It ignores the squash and the sharpness of 8, so it gives sign and rough
size, not behaviour. Script kept outside the repo.*
