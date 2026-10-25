# The enemy: teaching the colony by being what it has to survive

*One seat's game design document, 2026-10-25, revised the same day after an independent review (listed at the end of
"Risks and open questions"). Design research only: no code, no runs. Tags: [S] checked this session (URL in Sources),
[K] from memory and unchecked, [E] read in this repository (file:line or report).*

## The pitch

**You are the thing the colony evolves against.** You play an antlion in a sand pit, a centipede that slips into the
nest, or the queen of a raiding colony next door. You never lead the ants and they never follow you. They flee you,
fight you, wall you out, and in time are born better at all three.

**The answer in one paragraph.** A predator teaches through the one channel that already acts on today's ants: who
survives to lay. Every ant you eat is a genome removed, and the eggs come from the ants that answered you. Every bite
also leaves a mark: alarm is laid where a bite lands [E] `mind.rs:2604`, and every kill leaves a corpse. So the colony
has a short memory of you in the place and a long one in the bloodline. The central readout is **the price of an ant:
how many loads of food the colony carries home for each ant you manage to eat.** A colony that forages freely and still
starves you has learned. A colony that hides scores badly, so the score never rewards hiding.

**Why this seat might beat the queen and the keeper.**

- It is the furthest thing from "walk to food and they follow", which the owner rejected.
- Its verbs deliver at once: sand avalanches, a bite, a corpse, a bitten-off segment.
- Its teaching needs no learned weights to start. The predator bodies, the bite, the alarm plane and the colony's own
  bite all exist [E]. **What does not exist is a predator the player can steer** (section 8, step 2).

**Why it might not.** It inverts the emotional contract of a game about raising a colony. Its lesson vocabulary is
narrower: avoid, flee, fight, hide, rebuild. Selection inside one colony is weak and slow, so the Red Queen may not show
inside an evening without several colonies (section 4). And a sit-and-wait hunter can mean a lot of waiting.

**My honest read: the enemy is the best second seat in the game, not the best first one.** It works as seasons inside
the queen game ("switch sides"), and as the sparring partner who makes the player's own colony hard.

## The biology it rests on

Real colonies answer enemies on every clock: seconds (alarm), hours (closing doors), days (moving house) and
generations (arms races). The design borrows each one.

**Alarm is graded by dose.** Low concentrations of alarm pheromone attract and arouse ants; high concentrations repel
them. This is shown in the clonal raider ant and described for weaver ants [S]. Harvester ants show a low-intensity
response (looping runs) and a high-intensity one (tighter circling, open jaws) [S]. **Design use:** a light touch draws
defenders; a massacre clears the area. The engine's alarm response has no such threshold yet (section 7).

**Some species recruit soldiers to one specific enemy.** *Pheidole dentata* minors meeting a single fire ant scout lay
a trail that calls majors to kill it, and the response is narrowly specific to *Solenopsis* [S]. Wilson's 1976 paper
describes three graded phases: fight far out, fight near the nest, then flee with the brood [S]. **Design use:** the
answer scales with your pressure and ends in evacuation, not extinction.

**Army ant raids trigger evacuations.** *Aphaenogaster cockerelli* evacuates workers, brood and queen when
*Neivamyrmex* raids. About 100 to 150 army ants dropped into a nest set it off [S]. *Eciton* raids take much of the
brood but rarely kill workers or queens [S]. **Design use:** a raid that takes brood is a loss the colony recovers from.

**Doors are a defence.** Answers to army ants include evacuation, rebuilt architecture, doors blocked with stones, and
fighting outside [S]. Two *Stenamma* species keep a "door pebble" to plug the hole when army ants come [S].
*Cephalotes* soldiers block doors with flat heads [S]. Harvester ants raided by Argentine ants plug their doors [S].
*Conomyrma bicolor* drops stones into a rival's entrances to stop it foraging [S].

**Parasitoid flies teach by fear more than by killing.** *Pseudacteon* phorid flies cut fire ant foraging, during and
for a while after their presence [S]. Workers curl, freeze and guard baits [S]. Field parasitism is only 0 to 2.4% per
colony, so the response is far larger than the deaths [S]. Exposed colonies shifted the size mix of their foragers [S].
That shift is a nutrition effect inside one colony (less protein, fewer large workers later), not heritable change; I
read the 2010 abstract, and the reviewer adds a 2015 source. Night foraging under flies came up in the brief, and my
search did not confirm it [K].

**Antlions are a physics trap.** Ants near a pit rim slide but rarely start avalanches; the antlion throws sand to set
off slides toward the centre [S]. Trap-jaw ants sometimes outrun it or fling themselves out with their jaws [S].
*Conomyrma* ants steal prey from about 10% of antlion pits holding prey [S]. *Cataglyphis* workers dig out a trapped
nestmate and bite the snare (Nowbahari 2009, via perspectives.md [S]). Desert ants learn to avoid pit traps over
successive trials, and trapped nestmates nearby *increase* falls [S].

**Slave-makers raid for pupae, and hosts fight back.** *Polyergus* scouts find a *Formica* nest and lead a raiding
column back [S]. The raiders release formic acid, which panics defenders into fleeing and leaving pupae [S]. In
*Temnothorax*, enslaved workers kill about two thirds of the slave-maker's female pupae (Achenbach and Foitzik 2009,
via perspectives.md [S]). *Temnothorax* slave-maker queens avoid founding near aggressive hosts, which selects for
aggressive hosts [S], and these slave-makers raid five to ten times a year [S].

**Raids are led by a few.** *Megaponera* scouts lead columns of 200 to 500, and recruits cannot follow the trail
without the scout [S]. That is why the Slaver's raid is a colony-level call, not a follow-me.

**The Red Queen.** Van Valen (1973) proposed that species must keep evolving just to hold their place against
co-evolving rivals [S]. Read as an arms race (Dawkins and Krebs 1979), each side's gain selects for the other's counter,
and neither's fitness need rise [S].

**Liberties, said plainly:** the sealed box means the keeper lets the enemy in, or it is the box's own predator line.
One antlion feeding all season is denser predation than nature gives. A raiding queen who scouts the neighbour herself
is a liberty: real slave-maker queens invade hosts to found, but workers run the raids.

## Your verbs

Four enemy bodies, one readout. **The Pit** (antlion) teaches routes. **The Raider** (centipede) teaches doors and the
nest. **The Slaver** (a rival queen) teaches recognition and the cost of war. **The Fly** (phorid) teaches by fear.

**Codes, kept strict.** **now** = a player can do it on today's engine. **mech** = the mechanic exists but the player
verb needs the steered predator body of step 2 (`sim::steer` drives only an ant, through five hooks in the ant's tick
[E] `steer.rs:1-27`). **mem** = needs the memory layer of plan §7. **new** = new engine work.

| Verb | What you do | What the ant perceives | What the player sees | Works today? |
|---|---|---|---|---|
| **Dig the cone** (Pit) | Spiral backwards in sand, throwing it out | A sloped powder ahead; its stand/fall/climb features change | Arcs of thrown sand; a cone | new (body); sand physics exists |
| **Flick sand** (Pit) | Throw sand at an ant on the slope | Footing lost; it falls | A small avalanche carrying the ant down | new (verb); avalanche physics exists |
| **Reshape the rim** (Pit) | Steepen or widen the cone while you wait | A steeper slope | The pit changing shape, sand streaming | new |
| **Seize and drag** (Pit) | Bite from below, pull under | Being bitten; alarm laid at the bite [E] | A struggle, then a red haze | bite and alarm mech; drag new |
| **Let one go** | Release a bitten ant, wounded | It lives with a wound; alarm stays at the bite | A limping ant heading home | mech (wounds heal, per critique.md) |
| **Creep** (Raider) | Steer the 5-cell centipede | Another colony's animal in reach: the `STRANGER` flag [E] `options.rs:1059-1061` | A one-cell chain snaking along a tunnel | mech (body exists [E] nest-raider report) |
| **Try the door** (Raider) | Enter by an open door; a packed one stops you | An intruder in the shaft | You stuck at a plug, or inside | mech: jaw 0.5 cannot cut soil 0.8 or packed lining 0.95 [E] |
| **Shove the larder** (Raider) | Burrow through stored crumbs, packing them behind you | Its store moved, not lost | Crumbs shifting as you tunnel | mech [E] `organism.rs:4224` |
| **Snatch brood** (Raider, Slaver) | Grab a larva or pupa and leave | Brood gone; nurses search | A white speck carried out | new |
| **Coil and wait** | Ambush still at a door or a trail | Nothing until contact | You still; your hunger rising | mech (Rest) |
| **Strike and withdraw** | Bite once, retreat up a side tunnel | A bite, alarm, then no foe | A short alarm flash; the colony settling | mech |
| **Scout their door** (Slaver) | Walk your queen body to the neighbour's nest | Trained ants: a stranger by colony label, no grading. Hand-built ants: alarm graded by scent distance [E] | A ring of defenders closing in | new, small: steer is off and drives one queen of one colony |
| **Mark the target** (Slaver) | Drop alarm at their door before a raid | Alarm features in each option [E] `options.rs:253-255` | Defenders gathering where you chose, if they read it | lab tool now; effect untraced |
| **Call the raid** (Slaver) | Send your colony to war for one cycle | A raid target that pulls workers out | A column of your ants crossing the box | new: a separate raid target. Moving the forage anchor is not a shortcut: it is home for `f::HOME`, so laden workers would carry food to the enemy [E] `creature.rs:2293-2297` |
| **Panic gas** (Slaver) | A heavy alarm burst inside their nest | High dose repels: defenders flee, leaving brood | Their nest emptying; pupae left lying | new (dose response) |
| **Carry pupae home** (Slaver) | Your raiders bring brood back | A hatchling of their stock in your nest | Strangers among your workers | new |
| **Hover** (Fly) | Hang over foragers and stab one | A threat sense (new), or only the deaths | Foragers freezing; traffic thinning | new |
| **Stock the box** (Director) | Release AI spiders or centipedes when you are away | Trained predator minds [E] `lab/mod.rs:1113` | The colony's stress over days | now by hand; scheduling new |
| **Wake as the colony** | Switch sides and read what they learned of you | n/a | The colony's dossier on you | new (interface) |

**The best verbs, briefly:**

- **Flick sand** is the seat's signature. The falling-sand engine *is* the antlion's weapon, so the kill is physical,
  graded (a small slide or a big one) and different every time.
- **Try the door** turns nest architecture into the lesson. The centipede is one cell wide and fits any tunnel, so
  narrowing a door does nothing. Only a plug stops it, and a plug also stops foraging. That trade is the colony's whole
  problem, and the material numbers that make it true exist [E]. Spoil is also 0.95, so a spoil plug holds (critique).
- **Strike and withdraw** and **Let one go** teach without killing. They lay alarm and leave a wounded witness, at the
  cost of a meal.
- **Reshape the rim** and **Flick sand** are what the hands do while waiting (section 7).

## How a lesson travels

**Each lesson has one channel and one clock.** The place teaches in seconds. One ant's memory needs the memory layer.
Ant-to-ant help needs trained responses. The young need the provision hook switched on. The bloodline acts today, but
slowly, and strongly only across several colonies.

| Lesson the colony learns | Channel | Clock | Today? |
|---|---|---|---|
| "Something bites here" | **Place**: alarm at the bite; corpses; the cone | seconds to minutes | **now** to lay; whether lab ants move away from it is untraced |
| "I fell in at that rim" | **One ant's memory**: a bad place with a value | one trip (1,000 to 2,000 frames, per critique.md) | **mem** |
| "Help the bitten one" (mob, rescue) | **Ant to ant**: low alarm draws nestmates | seconds | **new**: needs a trained response; alarm has been rare in training |
| "Raise bigger defenders" | **The young**: provision moves body traits through each line's developmental block | one brood cycle | **new**: the brood path sets provision to 0 (plan §2) |
| "Plug the door", "keep brood deep", "forage the far side" | **Bloodline**: the ants you eat do not lay | ~12,000 frames a generation (critique.md) | **now**, weak in one colony; strong across several |

**What today's lab colony can do, in session.** The lab opens on the trained ant by default [E] `bin/lab.rs:297-307`.
Each result below names its brain.

- **It bites back (trained ant).** Lab minds come from `std_pool` cut to the colony bed's verbs, which include Attack
  [E] `lab/training.rs:386-388`, `train/challenges.rs:2719-2730`. Member 0 sets slot 8 (the Attack switch) to 1, and
  the other 29 are written as changes to member 0, none touching slot 8 [E] `std_pool.txt`, `lab/training.rs:94-95`.
  A mind picks only switched-on verbs [E] `mind.rs:1983`, and bites any animal of another colony in reach [E]
  `mind.rs:2575-2591`. (`options.rs:395`, which says trained pools never bite, is stale.) On the training bed the trained colony
  killed about one spider a round [E] lone-predator report.
- **It smells where you were (both brains, response untraced).** Every bite cries alarm. The trained ant has three
  alarm features per option [E] `options.rs:253-255`. Alarm was rare in its training: on the colony bed, mutants laid
  2,000 to 13,000 alarm a run against about 300,000 of the home trail (lone-predator report; `how-the-ant-works.md`
  `MIND_ATTACK` row). The hand-built ant's `Move: Alarm -1.0` *lowers* its chance of moving, so it lingers in alarm
  rather than avoiding it [E] `how-the-ant-works.md:301`. **Whether lab ants avoid an alarm patch is untraced, like the
  slice-0 trail.** It is step 0.
- **It shifts where it works (hand-built ant, once, a correlation).** In the one centipede population that took hold,
  on `lab_mid_fruit` with the hand-built ant, the raiders held the west side and the ants foraged east [E] nest-raider
  report. The food may simply have lain east.

**Selection is the channel that acts today, and it has two honest limits.**

1. **One colony is a weak selector.** On the training bed, plain selection with a spider cut ants killed per 1,000
   deliveries from about 17 to 3 to 5. A line selected with *no* spider lost ants at the same rate (4.6 against 3.2 to
   3.8) [E] lone-predator report. The colony got better at everything, not at the spider.
2. **Several colonies make it strong.** That is how the offline trainer works: many colonies, the best seed the next
   round (engine-brief.md). The late game, "the Range" (section 5), uses the player's hunting to decide which colonies
   found the next ones.

**The Baldwin caveat.** With the memory layer, an ant that learns the rim survives and lays, and selection can then
favour lines that start closer to avoiding it. This is slow and partial, and learning can shield genes from selection
(plan §7). Do not promise "born knowing" inside a session.

## Playing it

**The shape:** you hunt in seconds, the colony answers in minutes, and the bloodline answers in hours. Your hands are
always on one of three things: steering a body, shaping the trap, or timing a strike. Times below are **design
targets**, not measurements.

**The first ten minutes (as the Pit):**

- **0:00.** The keeper's hand has laid a sand bar across a busy foraging path. You land in it as an antlion larva.
- **0:20.** Hold dig. You spiral backwards and sand arcs out. By 1:00 a cone is open; ants walk past its rim.
- **1:10.** A forager crosses the slope. You flick; a slide carries it down; you seize it. A red haze blooms, and a
  corpse lies where it died.
- **1:30.** Your crop is full. A sated predator cannot bite until it digests [E] `mind.rs:2592-2595`. You spend the
  wait reshaping the rim and watching who walks where.
- **2:00 to 4:00.** You flick two more ants in. One nestmate walks into the haze and bites your jaw. You lose a quarter
  of a cell; four bites take a cell [E].
- **4:30.** You choose: stay and risk another bite, or move the pit. You stay.
- **6:00.** The dossier opens its first named entry: "They walk round the west rim", with a first-seen time and a count
  of foragers that skirted the cone.
- **8:00.** Fast-forward one brood cycle. The haze has faded; the corpse has been carried off.
- **10:00.** The season card: kills, wounds, loads carried home per ant you ate, and which lineages you ate most.

**An alternative opening is the Raider,** whose minute is movement, not waiting. The Pit is listed first because its
verb is the engine's own physics; the slice in section 8 should test which opening holds attention.

**The minute-to-minute loop: wait and shape, read, strike, get away, digest.**

1. **Wait and shape**: reshape the cone, pick a door, herd with flicks.
2. **Read** the traffic, the haze and the colony's stress meter.
3. **Strike**: one bite and withdraw, a full kill, or a raid.
4. **Get away** before the mob arrives, or pay in body cells.
5. **Digest**, and watch what the colony does with the gap.

**The first hour: three enemies in turn, one colony.**

- **The Pit** (minutes 0 to 15) teaches routes.
- **The Raider** (minutes 15 to 35) teaches doors. You shove through the larder and snatch a larva. The colony moves its
  brood deeper; a door you used twice is plugged.
- **The Slaver** (minutes 35 to 60). You take a rival queen's body next door. You scout their door, call a raid, panic
  their nest and bring pupae home. Then you watch what hatches in your own colony.
- **Between enemies,** the stress meter shows the learning band (some alarm, foraging continues) or collapse (foraging
  stopped).

**The fifth hour: the Range.** One colony cannot show the Red Queen (section 4), so the late game is several.

- The box holds three or four small colonies from the same founding stock, each named.
- A colony that falls below a floor is refounded by a daughter of the colony with **the highest price per delivery**:
  the most food carried home per ant you ate. Hiding does not win; foraging safely does. Per-colony laying caps, built for
  the colony-war bed, exist [E] `world.rs:4890`.
- The Red Queen chart, price per delivery over hours, per colony, is the headline. It saws up as colonies learn and
  drops when you find a new trick.
- You have counters too. As the Raider you can molt a longer body (six cells): deeper reach, more food needed. A
  bitten-off segment is a shorter centipede [E], so your length is your health bar.
- At the hour's end, **switch sides**: take the toughest colony as its queen, and inherit the defences you put there.

## What the colony might do

**Expected (the player aims for these):**

- **They walk round the pit.** The path bends away from the cone; corpses and haze mark the bend.
- **They mob the raider at the door.** Five ants on the centipede's head; a segment bitten off; you back out shorter.
- **They plug a door.** The door you used twice is packed the third time. Your jaw cannot cut it [E]. There may be no
  other way in.
- **The brood moves deep.** After a snatch, the nursery is three rooms further down.
- **They forage the far side.** With you holding one side of the box, the colony works the other.
- **They evacuate.** Push hard and the colony carries its brood to a new hole. Wilson's third phase [S], and the seat's
  softest "loss".

**Surprising (with their causes):**

- **They fill your pit.** A hole near the nest is a fine dump; one morning your cone is half-full of packed pellets.
  Cause: spoil drop sites, with *Conomyrma*'s stone-dropping as precedent [S].
- **They steal your kill.** While you hold a forager, three ants drag the corpse up the slope. Cause: mind ants may pick
  up a corpse they can eat [E] `options.rs:406-412`; *Conomyrma* robs about 10% of antlion pits [S].
- **The haunted pit.** You moved an hour ago, and the colony still walks round the empty cone. Cause: corpses at the old
  rim, and lines selected to avoid that slope. A visible superstition.
- **The fortress that starves.** Score a colony on fighting alone and its lines stop carrying food. In the colony-war
  tournament, champions won 24 of 24 matches and brought home about a tenth of the food [E] colony-vs-colony report.
  The price-per-delivery score exists to stop this.
- **The assimilation race.** Pupae you stole hatch in your nest. What happens depends on a choice the build must make:
  which colony a stolen hatchling belongs to. Trained ants bite by colony label [E] `mind.rs:2586-2591`. Odour drifts
  toward the nest's at every tick at home [E] `creature.rs:11349`, `mind.rs:2218`, but a nest that is not the ant's own
  is refused beyond its tolerance gene [E] `creature.rs:11372-11400`. Label it yours, and it blends in; label it theirs,
  and it stays a stranger your workers bite. Real slave rebellion (enslaved workers killing the slave-maker's pupae [S])
  would need new brood-killing behaviour.
- **Soldiers appear.** Under steady raids, lines that provision some eggs richly produce bigger ants that hold the door.
  No caste was designed; the threat made it pay. Needs the provision hook live.
- **Doors that close at dusk.** You always raid when the lamps dim, and the surviving colony packs its doors at that
  hour. Needs a sense of the lab's day as an input: a hope, not a promise.
- **You starve because they learned.** The price climbs past your hunger rate; your AI centipede line crashes and
  recovers. The lab has shown such a cycle (hand-built ant): raiders 2 to 34 to 1 to 23 while the colony held at 165 to
  228 [E] nest-raider report.

## Fun and boring

**What kills this seat, ranked.**

1. **The colony cannot answer inside a session.** You farm a herd that never changes. Weights are fixed, and one-colony
   selection is weak and noisy (section 4). Fix: make the place channel legible (haze, corpses, routes), time control,
   the Range, and the faked slice before anything else.
2. **Waiting.** A sit-and-wait hunter waits. Fix: give the wait verbs (reshape, herd with flicks, pick out one ant to
   follow), put the Pit on a busy path, and test the Raider as the opening.
3. **Wiping them out is too easy, or surviving you is impossible.** Both are binary. A lone *rival ant* that the nest
   could not bite back killed 25 in 700 frames [E] colony-vs-colony report. Four spiders from frame 0 left no colony in
   12 runs [E] lone-predator report. Fix: a graded bite (a quarter cell a bite; a scratch switch exists, read only by
   the rival bed [E] `mind.rs:2943`), satiation, and your body as something the colony can chew.
4. **Fear is the only lesson.** Fix: four enemies that each demand a different answer (route, door, recognition, fear),
   and a dossier that names each answer as it emerges.
5. **Villain fatigue.** Killing the creatures the game is about numbs. Fix: the sparring partner frame, and switch sides
   so the hardening pays off for the player.
6. **The colony hides and starves.** Foraging collapses under fear, as with phorids [S]. Fix: the price-per-delivery
   score, the stress meter, and your own hunger forcing you to let them forage.

**How to keep the player from just wiping the colony out.** Four layers that exist or are small, and one that is new:

- **Hunger with a ceiling.** A sated predator cannot bite until it digests [E]. You cannot eat 40 ants in a minute.
- **The colony is your only food.** Plague Inc's rule: killing hosts too fast loses the game (player reports [S]).
- **Risk to yourself.** Four bites take one of your cells. Wounds heal slowly. A mob costs you length, speed or a life.
- **The score pays the price per delivery, not the kill count.**
- **New: alarm by dose.** A massacre that clears the area for minutes needs a repel response at high alarm. Today the
  engine's alarm weights are linear with no threshold, so this layer is new work.

**What keeps it alive past hours.**

- **The sparring partner.** Your goal is the colony that beats you. A fencing master scores by how hard the pupil is to
  touch: "you win when you can no longer win."
- **The Red Queen chart.** Every trick you find drops the price; every answer they evolve raises it.
- **The dossier.** Defences nobody authored, named on first sight: "First rescue", "Door packed", "Pit filled". The
  *Evolution* board game's arms race of hard shell against bigger carnivores [S] is the model, except that neither
  side's cards are printed in advance.
- **Switch sides.** Hunt a season, then raise the colony you hardened. Or raise a colony as queen, then hunt your own
  daughters. *Dungeon Keeper* puts the player in the villain's chair [S]; switching gives you both chairs.
- **The Director role.** *Left 4 Dead*'s director paces threat in peaks and valleys by tracking player stress [S]. Here
  you are the colony's director, and keeping it in the learning band is a skill.

**What other games warn.** *Alien: Isolation*'s alien only appears to learn: behaviour-tree branches unlock when the
player repeats a tactic, paced by a "menace" gauge [S]. **Here the learning would be real but slow**: the selling point
and the danger. The faked slice is Alien's trick, used honestly as a prototype.

## Building it

**Fit with each owner rule.**

| Rule | Fit |
|---|---|
| One brain; roles emerge | Strong: guards, pluggers and soldiers are what threat selects for. Soldiers need the provision hook |
| Dig and forage all game | At risk: fear suppresses foraging, and defence selection cost food in every arm [E] lone-predator report. Guarded by the price-per-delivery score and hunger |
| Graded; visible | Strong once the bite is graded. Today 67% of bites kill [E] colony-vs-colony report |
| Frame cost | Low for bodies; the Range multiplies ants |
| Realistic, one physics | Excellent: the antlion is the sand physics |
| Not "walk and they follow" | Opposite by design: you are fled from |

**Build steps, in order, each with what it proves.**

| Step | What | What it proves |
|---|---|---|
| **0. Look and trace** | (a) Drop alarm beside a lab colony (trained ant) and trace workers: away, toward, or no change? Positive control: a huge dose. (b) Does an ant on a sand cone slide when sand is flicked? (c) Does a packed door stop the centipede live? (d) Where do ants forage when AI centipedes hold one side, with food placed evenly? | Whether the place channel works today. If alarm moves nobody, everything else waits |
| **1. The faked slice** | Below | Whether being answered is fun, before learning is built |
| **2. A steered enemy body** | Generalise `sim::steer` to the centipede; its hooks are written for the ant's tick [E] `steer.rs:5-27` | Every "mech" verb becomes playable |
| **3. A graded fight** | Quarter-cell bites; hunger, satiation and wounds on the player's body | No massacres, no instant deaths |
| **4. The readouts** | Price per delivery, stress meter, dossier detectors, per-lineage kills. `World::kills_log`, `encounter_log` and wound events exist [E] | The player can see the colony learning |
| **5. The Range** | Three or four colonies; daughters of the best price per delivery refound fallen ones | Player-driven selection shows inside hours; needs elitism (section 9) |
| **6. The Pit body** | Antlion: dig cone, flick, seize | The signature verb on the engine's own physics |
| **7. Bad places** | The memory layer's place values (plan §7), trained on a bed with a predator | Survivors avoid in one life; must beat frozen ants on paired seeds |
| **8. The Slaver** | Raid target, brood theft, the hatchling's colony rule | A rival game |
| **9. Trained answers** | Rescue and mobbing as lessons; provision on the brood path | Rescue and soldiers can appear at all |

**Frame cost.** The player's body is one creature among hundreds. The alarm plane exists; no new plane is needed. The
cone is sand the engine already simulates. Detectors are counters. **The Range is the real cost**: four colonies of 80
is 320 ants, against one colony of about 200 in today's lab box. Quote `examples/ascii.rs` worst-frame before committing,
and shrink colonies rather than slow the frame.

**The cheapest faked slice: ten minutes of the Pit alone, paired seeds, a blind A/B.**

- **Body.** A stand-in: the steered queen (an ant, so `sim::steer` works), re-skinned, biting by key, with flicking as a
  scripted impulse on sand cells. The cone is pre-dug across the colony's busiest path, so kills come every minute or so.
- **Both arms keep the real consequences**: corpses, alarm and haze. Only the scripted answers differ.
- **Scripted answers (arm A only), each a hand-set term, graded with kills:**
  - after each kill within 15 columns of the pit, foragers get a repulsive pull around it, adding up per kill;
  - ants within 10 cells of a bitten nestmate are pushed toward the bite for a few seconds (a scripted mob).
- **The test.** Paired seeds, same hunt plan, posted with `review.py ab --blind`. Card meta: kills, loads home per kill,
  mobs. The question: **"In which run did the colony feel like it was learning you, and was hunting fun?"**
- **What it decides.** Arms that cannot be told apart kill the premise cheaply. Arms that can, with fun hunting, justify
  steps 2 to 5. A second slice can then test the Raider and door plugging.

## Risks and open questions

**Biggest risks to fun.**

- **Slow selection.** Without the memory layer, a single colony's answer takes hours and may be generic (section 4).
- **The tone.** A seat about killing the colony may sit badly in a calm lab game about raising one.
- **Narrow vocabulary.** The queen and keeper teach building; the enemy teaches surviving.
- **Waiting.** The Pit's natural pace is slow; the slice must test it.

**Biggest risks to feasibility.**

- **Alarm may not be read.** Alarm was rare in training (section 4). Step 0 decides this.
- **Co-evolution has been noisy here.** No trained centipede line beat its founder, as with the spider [E] nest-raider
  report. Defence cases needed the 64-colony scale to rank [E] lone-predator report. Expect the Range to need elitism.
- **The bite's middle is unproven in the lab.** The graded bite was tested on the colony bed; the spider was tuned
  around one-bite kills [E] colony-vs-colony report.

**Hopes, unmeasured.**

- **Use it or lose it.** Every live brain link costs energy [E] `creature.rs:11596-11613`. One might expect a colony
  that stops being hunted to shed its defensive wiring. But that same doc comment says the training bed never feels the
  bill, so selection has not shed links. Forgetting the enemy in the genes is a hypothesis, and improbable inside a session.
- **Born to the threat.** Heritable shrinking under the Fly, and dusk door-closing, are liberties until a channel for
  them (provision; a day sense) exists.

**Decisions only the owner can make.**

1. **Seat, mode or season?** Recommended: seasons inside the queen game ("switch sides"), plus a trainer mode.
2. **May the player end a colony?** Recommended: as evacuation and refounding (the Range), never a blank box.
3. **How much cruelty is supported?** Drowning, starving and massacre are all possible. Decide whether they are recorded
   (graves name the cause today) and whether a shared colony can be harmed by another player.
4. **Does the player's enemy evolve too?** A longer molt, or an AI predator line selected on the player's grounds, makes
   a two-sided Red Queen, at more cost.
5. **Is the Range's frame cost acceptable** in the lab box, or does it wait for streaming?

### What the review changed

- **Verbs re-coded.** Anything needing a steered predator is now "mech", not "now"; `sim::steer` drives only an ant.
  Scout their door is "new, small".
- **Scent and strangers corrected.** Trained ants judge strangers by colony label, not scent [E] `options.rs:1059-1061`,
  `mind.rs:2586-2591`. Scent-graded alarm belongs to the hand-built ant's act, which mind ants skip [E]
  `creature.rs:8022`. "Rebellion" is now an assimilation race decided by the hatchling's colony rule. **Partly disputed:**
  the reviewer said a stolen hatchling always converges on your odour; the nest kin gate refuses a foreign nest beyond
  the ant's tolerance [E] `creature.rs:11372-11400`, so it depends on the label it gets.
- **Link-price forgetting** moved from engine fact to unmeasured hope.
- **The score is now per delivery**, so the Range cannot breed hiders.
- **Alarm citation fixed**: the option-level alarm features did see alarm, rarely; the `options.rs:491` context slot is
  overwritten by the default home ramp.
- **Each lab result now names its brain**; the hand-built ant lingers in alarm; the 25-kill raider was a rival ant.
- **Waiting addressed** with wait verbs and a Raider opening; unsourced times relabelled as design targets; satiation
  replaces an invented thirty-second lock.
- **Slice narrowed** to the Pit alone, paired seeds, real corpses and haze in both arms, no generation swap.
- **Biology fixed**: phorid forager size is a nutrition effect, not heritable; "five to ten raids a year" is
  *Temnothorax* slave-makers; alarm by dose is marked new; the forage-anchor raid shortcut is dropped.

## Sources

**Biology, read this session (search summaries and abstracts, not full papers):**

- Phorids and fire ants: [Insectes Sociaux 2023](https://link.springer.com/10.1007/s00040-023-00930-7),
  [Environmental Entomology 2010](https://academic.oup.com/ee/article/39/5/1593/479596),
  [UT Austin fire ant program](https://web.biosci.utexas.edu/fireant/Research%20Program.html),
  [ESA 2013](https://esa.confex.com/esa/2013/webprogram/Paper73944.html),
  [Pheidole susannae parasitoids](https://hal-univ-tlse3.archives-ouvertes.fr/hal-02350175)
- Army ants and evacuation: [Smith and Haight 2008](https://pmc.ncbi.nlm.nih.gov/articles/PMC3127389),
  [Pheidole obtusospinosa defence](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC3014660/)
- Doors: [Stenamma alas](https://ants.biology.utah.edu/genera/stenamma/species/alas/alas.html),
  [ESA 2007](https://esa.confex.com/esa/2007/techprogram/paper_29757.htm),
  [Phragmosis (AntWiki)](https://antwiki.org/wiki/Phragmosis)
- Slave-makers: [Polyergus breviceps](https://en.wikipedia.org/wiki/Polyergus_breviceps),
  [Psyche 94](https://groups.csail.mit.edu/mac/projects/psyche/94/94-261.html),
  [Mainz news](https://idw-online.de/de/news863420), [JGU magazine](https://www.blogs.uni-mainz.de/jgu-magazine/?p=1191)
- *Pheidole dentata*: [Pheidole (Wikipedia)](https://en.wikipedia.org/wiki/Pheidole),
  [keyapa.com summary](https://keyapa.com/pheidole/2024/05/29/pheidole-dentata-coping-with-fire-ants/)
- Antlions: [JEB 2020](https://cob.silverchair.com/jeb/article/223/15/jeb224626/224521/Fine-sand-particles-enable-antlions-to-build),
  [Büsse et al. (bioRxiv)](https://www.biorxiv.org/content/10.1101/2020.11.25.398073v1.full.pdf),
  [Trap-jaw escape](https://www.nationalgeographic.com/animals/article/150513-ants-animals-science-insects-trap-jaw),
  [Conomyrma kleptoparasitism](https://journals.flvc.org/flaent/article/view/58214),
  [Cataglyphis and pit traps (JoVE)](https://visualize.jove.com/39698156)
- Predator cues: [UCF thesis](https://stars.library.ucf.edu/etd/6704) (colony foraging was risk-insensitive to an ant-eating spider)
- Alarm by dose: [clonal raider ant (bioRxiv)](https://www.biorxiv.org/content/10.1101/2022.12.04.518909.full.pdf),
  [PMC 9941220](https://pmc.ncbi.nlm.nih.gov/articles/PMC9941220)
- *Megaponera*: [Wikipedia](https://en.wikipedia.org/wiki/Megaponera), [AntWiki](https://antwiki.org/wiki/Megaponera_analis)
- Red Queen: [Wikipedia](https://en.wikipedia.org/wiki/Red_Queen_hypothesis), [UVM](https://go.uvm.edu/rqh)

**Games, read this session:** [Left 4 Dead](https://en.wikipedia.org/wiki/Left_4_Dead),
[Evolve and the director](https://shacknews.com/article/82787/how-evolve-assures-action-peaks-and-valleys),
[Alien: Isolation AI](https://www.gamedeveloper.com/design/revisiting-the-ai-of-alien-isolation),
[The perfect organism](https://www.gamedeveloper.com/design/the-perfect-organism-the-ai-of-alien-isolation),
[Dungeon Keeper](https://en.wikipedia.org/wiki/Dungeon_Keeper),
[Evolution board game](https://en.wikipedia.org/wiki/Evolution_(board_game)),
[Plague Inc. player reports](https://steamcommunity.com/app/246620/discussions/0/215439774873667976)

**Cited by the reviewer, not read by me:** [Puckett 2015](https://urbanentomology.tamu.edu/wp-content/uploads/sites/19/2022/05/nvv118_puckett_2015.pdf),
[Achenbach and Foitzik 2009](https://bioone.org/journals/Evolution/volume-63/issue-4/j.1558-5646.2009.00591.x/First-Evidence-for-Slave-Rebellion--Enslaved-Ant-Workers-Systematically/10.1111/j.1558-5646.2009.00591.x.full).

**From memory [K]:** Wilson 1975/1976 beyond the summaries; Dawkins and Krebs 1979; phorid night foraging.

**Repository [E]:** the teaching-the-colony plan and folder (engine brief, perspectives, critique, queen) and
`enemy.critique.md`; the lone-predator, nest-raider and colony-vs-colony reports; `how-the-ant-works.md`;
`dead-ends.md`; the source files cited inline (`options.rs`, `mind.rs`, `creature.rs`, `steer.rs`, `organism.rs`,
`world.rs`, `lab/training.rs`, `lab/mod.rs`, `bin/lab.rs`, `train/challenges.rs`), `assets/minds/std_pool.txt`,
`examples/rivalbed.rs`, `examples/rivalry.rs`.
