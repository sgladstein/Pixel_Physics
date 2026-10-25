# The enemy seat: adversarial review and feasibility

*2026-10-25. Reviewer: an agent that did not write `designs/seat-enemy.md` or its critique. Read against the
working tree of branch `claude/project-thread-1ekeut` at `9e32979e` plus the thread owner's uncommitted steer hook
(`src/sim/steer.rs`, `src/sim/creature/mind.rs`, `src/sim/world.rs`). Line numbers in `mind.rs` are the working
tree's, which sit 29 lines below `main` past line 2300. One short headless run was made (two arms, one seed,
60,000 frames). Nothing in the repository was changed.*

## Verdict

**Feasible with changes, as the Raider (a steered centipede) and not as the Pit.** The body, the bite, the
segment loss, the colony's bite back and the counters for the "price of an ant" all exist or are a day's work. But
the design's central promise, that the colony visibly *learns* you inside a session, has no engine channel today:
the alarm lasts three seconds, the trained lab colony cannot evolve in the box, and the Pit's sand physics does not
do what the design says.

## Must-fix findings

**1. Alarm is a three-second shout, not a lasting mark.** The alarm plane is built as a volatile "active space".
It carries about six cells, reading 171, 119, 80, 51 and 12 at one to six cells out. Once biting stops, it empties
in "about 16 passes, ~190 frames" (`src/sim/pheromone.rs:336-383`, `ALARM_RHO` and `ALARM_DEPOSIT`). That is about
3 s at 1x and 0.2 s at 16x. Several claims rest on a lasting alarm, and all of them fail:

- "a short memory of you in the place" (the pitch);
- "Alarm stays at the bite" (Let one go);
- "The haze has faded" by 8:00, which implies it lasted minutes;
- the haunted pit ("corpses at the old rim");
- "Read the traffic, the haze".

What does persist is **corpses** (`assets/materials/corpse.ron`: a powder that decays into soil, worth 120 J of
food). They do not last either. With the bite switch on, a mind ant is offered a corpse to pick up
(`options::corpse_on`, `src/sim/creature/options.rs:410-413`), so the colony eats or carries away the evidence.
**Fix:** the place channel is the corpse and the shape of the ground, not the alarm. The alarm is a flash at the
moment of the bite. Either show it as that, or the design needs a long-lived alarm. A long-lived alarm would change
every fight in the engine, so it would have to be a new plane and not a retune.

**2. In the lab box, the trained colony cannot inherit anything new.** A newborn inherits its parent's mind
unchanged: "mutation is frozen, so the genome is the parent's own" (`mind.rs:1728-1730`). The lab says so too: "a
mind does not mutate at a birth in the box; minds evolve on the TRAINING page" (`src/lab/mod.rs:1024-1030`). So
"who survives to lay" can only change how common each of the 30 starting genomes is (`assets/minds/std_pool.txt`,
handed out by id in `src/lab/training.rs:386-394`).

Those 30 are near-clones. Member 0 is whole. The other 29 are written as changes to it, of 8 to 107 slots each,
against about 1,950 slots in member 0. I decoded the scorer's weight on the option feature `f::STRANGER` (slot 31)
through each unit's output weight: it is **identical in all 30 members** (+4.8). The weight on the three alarm
features takes only two or three values across the pool.

So in a trained-ant box, three things in the design cannot happen at all:

- "Selection is the channel that acts today";
- "the bloodline answers in hours";
- the Range's Red Queen.

This holds at any session length, not only a short one. Both lab birth paths copy the parent's mind: the egg route at `brood.rs:1595` and the bud route at
`creature.rs:6087-6089`. The only place a mind genome mutates is the TRAINING page's founders (`train::mutate`,
`lab/training.rs:376`). One thing does still vary from birth to birth: `mutate_newborn` (`creature.rs:6085`) runs
on every birth, mind or not, so **body traits** do mutate in the box. Those are the jaw, the armour, the gut and the
scent. A predator could select on armour in principle, but slowly, and nobody has measured it. The hand-built ant's
brain also mutates at birth by default (`mutation_rate` in `assets/species/ant.ron:883`;
`creature::mutation_off_for_measuring`, `creature.rs:6160-6172`, exists because measuring runs must turn it off).
The design never says which brain its selection claims are about. **Fix:** say plainly that the trained box does not evolve. Learning in a demo
has three possible sources, and the design should name which one it means:

- selection off-screen, between seasons, on the TRAINING page or in a harness;
- mutation at birth for minds, switched on in demo mode only (new code, and a behaviour change, so off by default);
- the hand-built ant.

**3. The Range is a war, and its per-colony laying cap does not exist.** A mind bites any animal of another colony
that is within reach. The colony check is `mind.rs:2692` and the option is offered at `options.rs:907`. So three or
four colonies in one box fight each other. In the colony-war report, a rival colony that could not be bitten back
killed 25 ants in 700 frames, and champions won 24 of 24 matches. The player's selection would be swamped by
selection between the colonies themselves.

The cited "per-colony laying caps ... `world.rs:4890`" is `encounter_log`. The real brake is
`colony_lay_scale` (`src/sim/world.rs:4891-4896`). It is marked SCRATCH, and it is `None` everywhere except
`examples/rivalbed.rs`. Refounding a fallen colony from a daughter of another does not exist anywhere. **Fix:** the
Range needs four things: colonies kept apart (separate chambers, or walls between them); mind mutation in the box;
a refounding step; and elitism. That is a project, not a demo step.

**4. The trained lab colony hardly fights a raider.** I ran `spiderlab` (scenario lab_mid_fruit, species
centipede, the lab pool, 2 raiders landed at frame 26,000, seed 3, 60,000 frames), once per ant brain:

| colony brain | raiders at 60k (peak) | raiders killed by ants | raider births | ants at 60k |
|---|---|---|---|---|
| trained (`ant=std`) | 72 (72) | 4 | 83 | 162 |
| hand-built (`ant=old`) | 14 (15) | 14 | 27 | 387 |

This is one seed per arm, so it is a first look and not a ranking. But it agrees with
`Reports/nest-raider-2026-10-25.md:38-40`: three blank-minded raiders took a trained colony from 30 ants to 8 in
20,000 frames. It also agrees with `Reports/lone-predator-2026-10-24.md:312`: with the trained ant, lab_mid_fruit
"dies in 3 of 4 seeds with no spiders at all".

So for the trained ant (the lab's default, `src/bin/lab.rs:297-307`), "They mob the raider at the door" is a hope
and not the expected outcome. The hand-built ant has a real crude mob: its brain wires `Attack: Alarm +2.0`, so a
bite raises the urge to bite in the ants within about six cells. That is graded by the alarm's falloff
(`Reports/how-the-ant-works.md:301, 311`; `pheromone.rs` near `ALARM_FALL`). **Fix:** the demo has to choose its
opponent's brain knowingly (see the owner questions). It also needs a scenario in which a trained colony survives
at all; the pool header lists boxes measured alive to 15,000 frames (`std_pool.txt:3`).

**5. The Pit's sand does not trap an ant, and there is no antlion.** Four parts of the engine stand in the way:

- **Powder piles up on an animal and never carries it.** A grain falls past living tissue only if its material is
  marked `falls_through_organisms` (`src/sim/update.rs:1929-1957`). Sand is not.
- **An ant stays standing while any of the eight cells around its body is solid, powder or plant**
  (`touches_ground`, `creature.rs:23927-23933`, checked before every mind's fall at `mind.rs:2300`). An ant on a
  sand slope therefore never falls unless the sand is cleared from around its whole body. When it does fall, it
  walks back up a 34-degree slope (`assets/materials/sand.ron`: friction angle 34).
- **The ant's jaw (1.0) cannot cut sand (1.4)**, and the centipede's (0.5) cannot either (`dig_force` in
  `assets/species/*.ron`). So an antlion body needs a jaw above 1.4, and that jaw would also cut soil (0.8) and
  lining (0.95).
- **Nothing in the engine throws a cell.** The only impulse is `creature::launch` (`creature.rs:28669`), a
  ballistic hop for an animal standing on something, which the lab's FLING tool calls (`lab/mod.rs:2963`).

"Avalanche physics exists" is true of sand, but not of an ant on sand. The Pit needs four new things: a species, a
throw verb, an animal-on-moving-powder rule in the hottest loop in the engine, and a scenario with a sand bar (the
`Fill` placement can lay one, `src/lab/scenario.rs:127`).

**6. No ant plugs a door, and the hand-built ant is built not to.** The hand-built ant refuses to drop spoil in a
doorway or anywhere that closes a way through (`closes_a_way` and `in_a_doorway`, `creature.rs:14659-14736`). The
trained colony's nest is already closed, by packed crumbs (`Reports/nest-raider-2026-10-25.md:28-35`), but crumbs
are exactly what the raider shoves through (`digs_inedible_food`, `assets/species/centipede.ron`). So two
consequences of Try the door are not reachable on today's engine: "The door you used twice is packed the third
time" (the expected list) and "the colony's whole problem is plug against forage" (the best verbs). **Fix:** move
both to hopes, next to "brood moves deep". I found no code path that ties where brood is laid (`brood::pile_site`)
to a threat.

**7. The faked slice is built on the two parts that do not work.** Its body was "the steered queen, re-skinned".
That is now unnecessary: the thread owner's hook lets a mind body be led (`steer::take_forced`, called at
`mind.rs:2310`, applied at `mind.rs:2530`), so the real centipede can be steered. Its weapon was flicked sand,
which does not trap anything (finding 5). Its answers were a scripted repulsive pull and a scripted mob, which are
hand-made behaviour. That is acceptable in a throwaway blind A/B, but never in a demo the owner plays. **Fix:**
retire the Pit slice. What it was meant to test, whether being answered is fun, can be tested on the Raider
against the real colony.

**Smaller corrections.**

- The doc comment at `options.rs:392-395` says "every trained pool, the lab's new ant included ... holds Attack
  banned". It is stale: `std_pool` member 0 has slot 8 (the Attack switch; `Verb` order `options.rs:73-106`) set
  to 1, and the other 29 inherit it.
- "Four bites take a cell", "67% of bites kill" and "a sated predator cannot bite" are all real. But the 67% is ant
  on ant, from the colony-war report, and does not apply to the centipede.
- "Wake as the colony" and "switch sides" depend on the queen seat's work (`shared/steered-ant.md`).

## Feasibility per body and per verb

S means under a day, M one to three days, L more than three days or new physics. "Exists" means a player could get
the effect today once the steer hook lands.

| Body / verb | Exists? | Cost to make playable | Evidence |
|---|---|---|---|
| **Raider body** (centipede, `Chain(5)`, mind) | Yes | S: mark one centipede and route the keys | `assets/species/centipede.ron`; `Lab::give_predator_minds`, `lab/mod.rs:1106-1130` |
| Creep (steer) | Being built | S: arrows choose the Walk landing nearest the arrow, from the 8 cells round the head | `steer::choose`; Walk targets `options.rs:773-800` |
| Bite (B) | Yes | S: needs a stranger in one of the 8 cells round the head | `options.rs:907`; `mind::attack` `mind.rs:2677-2710` |
| Segment loss, your length as health | Yes | S (HUD) | `reconcile_chain`; the plate 2.0 note in the nest-raider report |
| Sated: cannot bite | Yes | S: HUD "full, digesting". A forced Attack otherwise counts as a failed act | `mind.rs:2695-2698` |
| Hunger clock | Yes | S (HUD) | 1,600 J start, "about 6,000 frames of steady walking on a full tank" (`centipede.ron`), so about 100 s at 1x |
| Shove the larder | Yes | S | `digs_inedible_food`; `options::pack_on` |
| Try the door (a packed one stops you) | Yes for soil and lining | S | jaw 0.5 against soil 0.8 and packedsoil 0.95 |
| Coil and wait; strike and withdraw | Yes (Rest, Walk, TurnAround) | S | `train/predator.rs:22` `RAIDER_VERBS` |
| Let one go | Partly: a bite that does not finish a cell leaves the ant wounded, and wounds heal | S | `gnawed`; `wound_heal_frames`, `creature.rs:7966-7972` |
| Snatch brood and leave | No: PickUp takes a cell into the crop, and nothing carries a whole larva | M | Verb docs `options.rs:86-89` |
| Molt a longer body | No | M | none |
| Your own budding | Yes, unwanted: above 3,200 J the player buds an AI raider | S to turn off or keep | `reproduce_threshold` in `centipede.ron`; my run went from 2 raiders to 72 |
| **Pit body** (antlion) | No species | L | finding 5 |
| Dig the cone | No: no jaw cuts sand | M (species) plus L (trap rule) | sand 1.4 against `dig_force` |
| Flick sand | No | L (cell throw plus animal-on-moving-powder rule) | `update.rs:1929-1957`; `creature.rs:23927` |
| Flick the ant (FLING, a ballistic hop) | Yes as a lab tool | S | `creature.rs:28669`; `lab/mod.rs:2963` |
| Seize and drag under | No | L | none |
| **Slaver body** (a rival queen) | An ant of a second colony; two-nest scenarios exist | M to steer a second colony's queen (queen rule per colony) | `steer.rs` header; `assets/lab_scenarios/two_nests_*.ron` |
| Scout their door | Yes once steered | S | the mind bites by colony number |
| Mark the target | Yes (lab ALARM tool, J) | S | `lab/mod.rs:2942-2946`, `ALARM_DEPOSIT`, gone in about 3 s |
| Call the raid | No | L | none |
| Panic gas (high dose repels) | No: the response is linear | L | the critique's item 12 |
| Carry pupae home; the assimilation race | No | L | no brood carry for minds |
| **Fly body** (phorid) | No: the flitter floats, but an airborne animal "cannot eat, dig, drop, steer" | L | `creature.rs:28700-28712` (`step_flight` doc); `assets/species/flitter.ron` |
| Stock the box (Director) | Yes by hand (COLONY chip) | S; scheduling it is M | `lab/mod.rs:1106-1130` |
| Wake as the colony | Queen seat's piece | M | `shared/steered-ant.md` |

**What a steered centipede does through its option list.** Its verbs are Walk, TurnAround, Rest, Attack, PickUp,
Eat, Dig and Drop (`train/predator.rs:22`). Every target is one of the eight cells round the head (`options.rs`
target ring; `REACH` is 6 for the senses at `options.rs:339`), so here is what each key does:

- **Arrows** choose among at most eight landings, plus a trunk crossing, and the chain follows its head.
- **B** bites only a foe touching the head.
- **G** cuts only what a 0.5 jaw cuts, plus crumbs.
- **E** takes corpse or flesh into the crop.
- **Q** puts down a pellet or a cell from the crop.

A fall is physics and comes before the decision (`mind.rs:2300`), so the player loses control while falling, which
is right. Every standing bill is charged whether steered or not (`mind.rs:2238-2295`). Old age is skipped for the
steered body (`creature.rs:7947`); without that, the centipede's 40,000-frame half-life would kill the player in
about eleven minutes at 1x.

Three things the steer hook should handle for a predator: `steer::mark` applies the queen breeding rule to the
body's colony (`steer.rs` header), which means nothing for a budding centipede; the player's budding; and HUD text
when a pressed verb had no option (sated, nothing in reach). Frame cost of the off path is two reads per mind
decision (`steer::take_forced`).

## What the colony can visibly do in a session today, and what is hope

**Time scale.** The lab ticks 60 times a second at 1x (`src/lab/time.rs:285`). The dial's presets are 1, 2, 4,
16, 64, 256 and 1024 (`time.rs:740`). Above 12 ticks per displayed frame the screen stops reading as motion
(`MOTION_TICKS_PER_FRAME`, `time.rs:341`). With the display at 60 Hz up to 4x and 30 Hz at 16x
(`AUTO_DISPLAY`, `time.rs:777`), a player can steer at 1x to 4x and not above.

My headless runs did 60,000 frames in 49 s (trained colony of 100-160 ants, up to 72 raiders) and 60 s (hand-built
colony of 300-400 ants). That is about 1,000-1,200 frames a second, measured while another release build was
running. So the box can reach 16x headless; with drawing it is less, and `examples/labdial mode=rate` is the
instrument that would say how much.

An egg takes 250 frames and a pupa 250, plus the larva's feeding (`ant.ron:126-128`). A generation is about 12,000
frames (`designs/critique.md:71`), which is 200 s at 1x. **A 20-minute session at 1x is 72,000 frames, about six
generations**, or about 24 at 4x with no steering.

**Real now, in seconds:**

- The colony bites back.
- A bite costs the raider a quarter of a cell, and four take a segment.
- Alarm flashes for about 3 s within about 6 cells.
- With the hand-built colony, alarm drives more biting nearby: a crude, graded mob.
- Corpses lie where an ant died, until the colony eats or carries them.
- Food stops coming home when you sit on a route.
- A raider that eats enough buds.

**Real now, in minutes:** the predator and prey numbers swing (the nest-raider report's 2 to 34 to 1 to 23 cycle,
hand-built ant).

**Hopes:**

- Trained ants avoiding alarm, or moving toward it. Untraced. I tried a crude linear read of the scorer weights, and
  it failed its own positive control: it rates food *negatively*, which no foraging ant does. So it says nothing,
  and step 0a of the design (a trace) is still needed.
- Route shifts that last after the threat is gone.
- Door plugging and brood moved deep (finding 6).
- Rescue.
- Any heritable answer in a trained box (finding 2).

## The price of an ant: can it be computed live?

Yes, at small cost (S). Three counters exist:

- **Food home:** `CreatureStats::deliveries` (`world.rs:2530-2541`). It is world-wide; a mind ant's drop at the
  nest adds to it, and also to that ant's own `life.deliveries` (`mind.rs:3022-3031`), so a per-colony count is a
  sum over its ants.
- **Each kill:** `World::kills_log` keeps one `KillRecord` per kill, with the victim's colony, the attacker's
  species and colony, and a `KillDetail` naming both individuals (`world.rs:565-590`, filled at
  `tally_kill_detailed`, `world.rs:10175-10195`). The player's kills are the records whose `detail.attacker` is the
  steered id.
- **Bites taken:** `encounter_log` is `None` by default. The demo would set it to `Some` (bounded at 2 M events,
  `world.rs:641-664`), or simply watch the body's length.

Kills per 1,000 deliveries, the lone-predator report's own metric, is then one division. A breakdown by line is
harder, because a dead ant's mind leaves with it; recording the victim's pool member at the kill is M.

## Recommended demo scope

The thread owner's fear is the right one. On today's engine, an honest demo of this seat shows a colony that
**fights back**. It does not show a colony that **learns**, unless the learning happens somewhere the player can be
told about plainly. I rank three candidates on three things: agent days to a playable demo, how satisfying it is to
play (by the repo's two laws: graded outcomes, and a verb that delivers), and honesty about learning.

**A. "The Raider": one steered centipede against a live colony.** (1 to 2 days. Satisfying: good. Honesty: high,
learning: none.)

- Launched with `lab -- demo=enemy`, which uses the shared launcher and `assets/lab_scenarios/demo_enemy.ron`.
- One centipede is marked as the player; no AI raiders, and the player's budding is off unless the owner chooses
  otherwise.
- Arrows creep, B bites, G shoves, E eats, Q drops.
- The HUD shows: length (your health), crop and digestion, hunger, kills, wounds taken, and **kills per 1,000
  loads home**, live.
- Every verb has a consequence: a corpse, a flash of alarm, a segment lost, a store moved.
- The outcome is graded: a bite wounds before it kills; the colony's bites take your length a quarter-cell at a
  time.

It fails the seat's thesis on purpose, and it says so on screen. Its risk is the trained colony's weak defence
(finding 4): the player may simply eat it. Against the hand-built colony the fight is real.

**B. "Seasons": A, plus a colony that has met raiders before.** (A plus 1 to 2 days, plus hours of CPU.
Satisfying: better. Honesty: good, if both arms are shown.)

- **Before the demo, off-screen:** breed a line of the trained ant on the colony bed with raiders
  (`examples/inrun.rs spider_species=centipede`, nest-raider report line 141). Breed a **control line for the same
  rounds with no raider**, because the lone-predator report found the spider-free line improved just as much (4.6
  against 3.2-3.8 killed per 1,000 deliveries).
- **In the demo:** Season 1 is today's colony. Season 2 is "the daughters of colonies that survived raiders".
  The HUD shows each season's price per delivery side by side, and the season card says where the change came from.
- **Gate:** if the raided line does not beat the control line on paired seeds against the player's own recorded
  hunt, then Season 2 is not called "learned you". Ship A and report the finding.

This is the smallest scope in which the colony really answers the player's kind of threat, with no scripting. It
also matches the design's own best fit, "seasons inside the queen game".

**C. "The Pit": an antlion in sand.** (5 or more days. Satisfying: possibly the best. Honesty: the same as A.)

It needs a species, a jaw for sand, a throw verb, and a rule that moves an animal standing on moving powder, inside
`update.rs`'s hottest path. That last part is a frame-cost and determinism risk for every world, and the "one
physics for every world" rule means it changes the outdoor game too. It answers nothing about learning that A does
not. A cheap hybrid (the Raider plus the lab's FLING as a "flick") buys spectacle, not an answer, and FLING is a
hand tool rather than a predator's verb.

**Recommendation: build A as the playable demo, with B's Season 2 as a gated stretch.** Start B's off-screen
training run on day 1, so its CPU hours overlap A's build.

- A is the only scope that fits in 1 to 3 days.
- Its fun is movement and a bite, not waiting.
- Every consequence in it is the engine's own.
- It respects every owner rule: one physics; one brain design, with no scripted answers; ants dig and forage
  throughout; every verb has a visible consequence; graded outcomes; one creature's frame cost; and the shipped lab
  unchanged, because it runs only under `demo=enemy`.

Two measurements before building:

1. **Step 0a, the trace.** Lay alarm beside working trained ants and trace each one, with a large dose as the
   positive control. The `funnel` method applies.
2. **The scenario.** Find a lab scenario where a trained colony is still alive at 60,000 frames on most seeds
   (`labsurvive`, `std_pool.txt:3`). Without it, the demo is a box that dies whether you hunt or not.

## Open questions only the owner can decide

1. **Which colony does the player hunt?** The trained ant is the lab's default by the owner's ruling, but it barely
   fights a raider (finding 4). The hand-built ant fights, and also evolves in the box. Options: trained only;
   hand-built only; or a runtime selector showing the active brain on screen, which is what `CLAUDE.md` advises for
   "does this look right".
2. **Is learning off-screen, between seasons, honest enough to count as "the colony learned you"?** It is real
   selection, but the player does not watch it happen. The alternative is mind mutation at birth in demo mode,
   which is real and on-screen but probably invisible within a session.
3. **Does the player's centipede bud?** If it does, eating well seeds AI raiders (2 became 72 in 34,000 frames in my
   trained-colony run), and the box can be lost to your own brood. That is graded and dramatic, but it is no longer
   the player against the colony.
4. **May the player wipe out the colony?** Three raiders took a trained colony from 30 ants to 8 in 20,000 frames.
   Should the demo stop, refound, or let it happen?
5. **Is the Pit wanted badly enough** to put a new rule for animals standing on moving powder into the shared
   physics? If so, it is its own project, with its own frame-cost and determinism checks.
6. **Is the Range worth building at all,** given that it means a war between colonies (finding 3) unless they are
   walled apart?
