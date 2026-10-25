# The lone scout: one worker, no royal power, only persuasion

*Game design document for one seat of the "teaching the colony" plan,
2026-10-25. Research only: no code changed, nothing run. Tags: [S] checked
this session (URL in Sources), [K] from memory and not checked, [E] read in
this repository at the file named.*

---

## The pitch

**You are one ordinary worker.** You hatch pale in a colony already
running, with no crown and no command. Whatever the colony does differently
while you live, you persuaded it into, one nestmate at a time.

**The fantasy is being the ant who found the meadow.** You go further than
anyone and come home with proof in your crop. You feed the taste to a
sister, lead her there by touch, and hope she leads the next.

Other scouts argue for other places, and the colony commits only when enough
members agree, possibly with someone else. You can die (hunger, a beetle,
the water), never of old age, by the owner's ruling. You can also choose to
pass on into a daughter or a pupil, and what you taught outlives you or not.

**The answer: a scout binds two things the colony cannot bind alone, the
urge to go and the knowledge of where.** This engine has measured each half
failing alone. Urge without knowledge kills: the returner-contact recruit
starved **442 adults against 298 and 323**, most on the surface [E]
`how-the-ant-works.md` §12.

Knowledge without a carrier does not travel: a hand-laid trail pulled no
extra workers, nor did a far stronger ramp [E]
`play-as-ant-slice0-2026-10-11.md` §2. In group-recruiting carpenter ants,
too, the trail alone recruits nobody; the scout must display inside the nest
and lead [S]. The scout seat is that missing carrier, played by hand.

**Why it might beat the queen and keeper.** It alone teaches ant to ant,
where real ants pass lessons; its loop is the most embodied; and it attacks
the engine's worst problem, colonies starving when food is far.

**Why it might not.** One ant among hundreds is small, and real tandem runs
are four times slower than walking alone [S]. Nothing here works on today's
ants without new work: a follow behaviour, a memory the shipped pool listens
to, and a lead rule every ant has. **Until that last piece lands (build step
5), the seat is an escort mission, which is the owner's rejected "follow me"
with friction.** It lives or dies on step 5.

---

## The biology it rests on

**Tandem running: teaching with feedback both ways.** In *Temnothorax
albipennis* one leader guides one follower to food or a new nest [S]. The
follower keeps touching the leader's hind legs and abdomen. If the gap
grows, the leader slows or stops; if it shrinks, she speeds up [S]. The
follower stops to learn landmarks and taps to say "go on" [S]. A tandem run
takes about **four times** as long as walking alone. The follower still
finds the food in about **two-thirds** the time it would take her alone
(Franks & Richardson 2006) [S].

**It was the first *claimed* case of non-human teaching**, contested the
same year by Leadbeater, Raine and Chittka [S, title only]. The design needs
the mechanics, not the label.

**Runs break, and broken runs still help.** Break rates run from about
**19%** (*T. nylanderi*) to **70%** (*T. rugatulus*), from different studies
[S]; runs between partners more than 10% apart in body length failed, and
about 80% succeeded under 5% (Wagner et al.) [S]. The leader waits, then goes
on; the lost follower searches close, then widens (Franks et al. 2010) [S].
Lost *T. curvispinosus* followers still found the new nest faster than
unguided ants [reviewer's source, abstract not read]. The break rate is a
tuning knob here, not a fact.

**Carrying is fast and teaches nothing.** In house-hunting *Temnothorax*,
scouts first lead active nestmates by tandem run, then switch to **carrying**
the passive majority once enough ants are at the new site [S]. Carrying is
about **three times** faster. Carried ants are held upside down and backwards
and do **not** learn the route; tandem followers do, and later lead [S]. A
real trade-off between speed and teaching.

**Quorum: the colony's built-in argument.** Scouts sense it by how often they
bump into nestmates at the new site; before it, colonies can switch, after
carrying starts they rarely do [S]. Scouts recruit **sooner** to a good site
and **later** to a poor one, tilting the race with no ant comparing sites
(Mallon, Pratt & Franks 2001) [S].

**Scouts are a minority whose information spreads.** In one *Temnothorax*
colony a **single ant moved 57%** of everything carried; honeybee scouts are
**3 to 5%** of a swarm; split colonies reunite through one pivotal carrier
or a few [S]. A lone ant can move a colony.

**Competing scouts argue and inhibit each other.** Honeybee scouts dance
**100 circuits for a first-rate site against 12 for a mediocre one**; at
roughly **15** scouts at one site, the swarm goes [S]. Scouts committed to one site head-butt dancers
for the rival site (the **stop signal**), and each side's inhibition grows
with its size (Seeley et al. 2011) [S]. Nothing shows a signaller must have
visited the rival site.

**The colony is wiser than its scouts.** A lone *Temnothorax* worker shown a
decoy switches preference irrationally; a colony does not (Sasaki & Pratt
2011) [S]. The colony may be right to ignore you.

**Recruitment by contact, not by trail alone.** In *Camponotus socius* a
recruiter shakes in front of nestmates inside the nest; up to about **30**
follow her out along her trail; her trail alone does not recruit [S]. In
*C. sericeus* the recruiter grips a nestmate's jaws and presents her abdomen
to start a tandem; *C. pennsylvanicus* recruits with no leader at all [S].

**Meals carry a lesson.** *Camponotus mus* workers fed one scented meal
mouth to mouth chose that scent in a maze with no reward (Provecho & Josens
2009) [S]. A follow-up found the socially learned smell could override an
ant's own judgement: they collected a deterrent food because it smelled of
the meal [S]. Persuasion can mislead.

**Memory beats the trail, sometimes.** *Lasius niger* learns an odour-food
pair in one visit and prefers its own memory over pheromone; about **20%**
ignore pheromone at any time [S].

**Liberties this design takes, said plainly:**

- It borrows from four animals: *Temnothorax* tandems and quorums,
  *Camponotus* displays, honeybee stop signals, *Lasius* memory. No one
  species does all of it.
- **Reputation with individual nestmates is a liberty.** I found no evidence
  that ants keep track of which leader proved reliable. The design keeps it
  honest by holding "standing" only in the memories of ants that followed
  you, never as a global number.
- **Playing on as a daughter** relies on the engine's own liberty: any ant
  lays, and the egg inherits the layer's brain [E] `brood.rs` (plan §2).
  Real workers mostly lay only sons.
- **Playing on as a pupil** (taking over an ant you taught) is a game
  liberty with no biology under it.
- **"Go and look before you head-butt"** is a game rule, not Seeley's.

---

## Your verbs

| Verb | What you do | What the ant perceives | What the player sees | Works today? |
|---|---|---|---|---|
| **Taste** | Eat at a find; your crop fills with its smell | (you) a crop holding a named food | A coloured glow in your crop, the food's colour | eating: now [E] `steer.rs:69`; a *named* meal: mem/new (the crop holds energy, not a food identity) |
| **Share a taste** | Feed a nestmate mouth to mouth (F) | Energy rises; with memory, the meal's smell becomes "good" to her | A drop passes; she takes on a faint tint of that food | now for the energy [E] `steer.rs:73`; mem for the smell |
| **Tap** | Antennate a nestmate to invite her to follow | A touch from a leader; she accepts or refuses by her own state | She turns and touches your abdomen, or walks off | new |
| **Lead** | Walk to the find; she follows only while touching | Contact with a leader: a hard follow instinct | A nose-to-tail pair in stop-start rhythm | new (follow), mem (for it to stick) |
| **Wait** | Stand still when contact breaks | Her search circle centres on the last contact | A ring widening round her; she finds you, or not | new |
| **Pick a pupil** | Choose whom to tap: idle, empty-handed, your size | A matched partner holds contact | Fewer breaks with a sister your size | new (body size is a gene today) |
| **Display** | Shake inside the nest with a full crop | Raised urge to go, only in ants that already tasted your food, pointed down your trail | A few roused sisters leave by themselves; you need not go | new, mem |
| **Mark** | Lay food trail (T) as you walk | Scent B, weak alone | A faint line on the scent view | now [E] `steer.rs:81`; pulls nobody (slice 0) |
| **Carry a nestmate** | Lift a passive sister and walk her there | Nothing learned; she is simply moved | She curls up in your jaws; she arrives knowing nothing | new |
| **Carry brood** | Move eggs and larvae to a new site | Nurses follow the brood | The nursery moves, and its nurses with it | colony: now, to verify (plan §4a); your body: new |
| **Assess** | Walk the inside of a candidate site | (you) warmth, wetness, room and roof, read from features that exist [E] `options.rs:185-214` | A star rating for the site forms as you pace it | new (UI over existing senses) |
| **Head-butt** | Stop-signal a scout recruiting for a rival site | Her urge to recruit drops, more so if your side is big | She stalls mid-display, then resumes or gives up | new; "only if you visited" is a liberty |
| **Warn** | Release alarm at a hazard or bad find | Alarm scent; with memory, the place becomes "bad" | Ants scatter; later, they skirt the spot | new: the alarm plane exists [E] `how-the-ant-works.md` §7, but the steered body has no alarm key; mem for the lesson |
| **Open a door** | Dig a new exit toward your find (G) | A new short way out | Traffic reroutes through your door | now [E] `steer.rs:75` |
| **Stock a cache** | Drag food closer to the nest, cell by cell | Food in sense, closer to home | A depot by the door that the colony finds itself | now (Q puts down [E] `steer.rs:71`) |
| **Lay** | Lay an egg when you can afford it (L) | Your genome in the brood | An egg with your lineage fleck | now [E] `steer.rs:79`, but see Building it |
| **Sniff** | Read a nestmate | Nothing | What she knows, whom she follows, her faith in you | now in part (ant page exists); new as a quick read |
| **Pass on** | Hand your play to a daughter or pupil you are touching | Nothing; your old body walks on under its genome | The camera moves to her; your old self keeps working | new |

**"Now" means the engine hook exists in `sim::steer`.** No lab binary binds
its keys yet; only `examples/queenprobe.rs` drives it, by script [E]. So
every "now" verb needs build step 2 before a player can press it.

**The best verbs, and why.**

**Lead is the heart, and it is a skill.** The follower is not dragged. If you
walk faster than her, contact breaks, and she searches. Wait and she may find
you; walk on and she is lost. A good run is a rhythm: walk, feel the gap, stop,
feel the tap, walk. It is Shadow of the Colossus's reins applied to a pupil.
Ueda's team made Agro resist the reins and then softened it because
"it got annoying" [S]. That is the tuning warning.

**Share a taste is the cheapest lesson and the riskiest.** One meal teaches a
smell, not a place [S]. Your sister hunts that smell anywhere: a better tree
than yours, or a beetle. Rumour outruns the route, and travels wrong.

**Display binds urge to knowledge, without a column behind you.** It rouses
only sisters who already carry your food's smell from shared meals, and
points them down your trail; they go alone. That is *C. pennsylvanicus*,
not *C. socius*, chosen so Display is not "follow me". It is the late verb,
its strength set by your crop, and it needs memory and a trail that is read.

**Carry versus lead is a real dilemma.** Carrying is three times faster and
teaches nothing; leading is slow and makes teachers. Carry too early and the
colony arrives with no ant who knows the way back.

**Head-butt makes the argument physical.** A game rule, not biology: you can
only stop a rival whose site you have seen, so to argue against a place you
must first go and look, which may change your mind.

**Standing: reputation held in other ants' heads, not in a number.** Each
ant that followed you keeps one value: did following you pay? A run that
ended at good food raises it; a run to nothing, or into a beetle, drops it;
it drifts back toward neutral with time. Only ants that were there change.
This is the design's own rule; Rain World's scavengers keep one regional
number instead [K]. Standing shows as behaviour: how many ants accept your
tap and how long they hold contact. A daughter starting with part of it,
because she smells like you, needs the plan's graded colony odour (P3,
perspectives §0): new.

**What the games teach.** Trico: "the player must learn to read Trico"
(Ueda) [S]; a pupil has her own mind. Journey: one wordless chime carried a
relationship [S]; Tap is that chime. Pikmin's whistle, calling idle
followers into your group [S], is the anti-model.

---

## How a lesson travels

**Five channels, three clocks** (plan §1). The scout leans on ant to ant,
which the other seats barely touch.

| Channel | What carries it | Clock | Today's ants | With the memory layer (plan §7) |
|---|---|---|---|---|
| **The place** | Doors you dug, caches you stocked, trail you laid | Seconds to hours | **Works.** A door changes traffic; a cache is food in sense | Same |
| **The follower's body** | She is *standing at the food* when the run ends | Seconds | **No run exists today.** Once step 3 exists, works once: a fixed-weight ant at food carries it home, laying trail B [E] `how-the-ant-works.md` §7 | Same |
| **One ant's memory** | Where she was led; what her meal smelled of | One trip to a life | **Dead.** The shipped pool's weight on the last-food sense is zeroed on load [E] `mind.rs:539-552` | Works: she returns unaided |
| **Ant to ant** | Pupils leading pupils; meals passed on | Minutes | **Dead** without a lead option every ant has | Works: a teaching tree |
| **The young** | Your eggs; larvae fed the smell you carried | A brood cycle | Your genome spreads if you lay (engine liberty) | Larval smell imprint would need its own work [K] |
| **The bloodline** | Daughters with your genes; scouting traits selected | Lineage | Slow; selection is weak in one colony (critique §4) | The Baldwin drift, possible, not promised |

**Once follow exists, a run on today's weights is worth one delivery.** The run moves the
follower to the food; her own weights pick up, walk home and lay trail once.
Nothing in her remembers, so she does not come back: visible, and not yet
teaching.

**With the memory layer, a run makes a forager for that patch.** The plan's
identity-free "heads toward a place I remember as good", hand-seeded to work
on day one, gives her a return route. It fades with time and empty trips, so
a dead patch un-teaches itself.

**With a lead option every ant has, a run starts a cascade.** A pupil home
with a fresh memory and a full crop may tap someone herself. Knowledge
spreads as a family tree and fades as memories expire; the player starts the
right trees and keeps them alive.

**A trail is a quorum made visible (a hypothesis).** Lab trail B is thin,
10 to 46 out of 255 [E] `how-the-ant-works.md` §7, so one forager cannot
keep it lit; several taught foragers can. That threshold is free physics,
and it matters only if trail B is read at all, which nobody has traced.

**Clocks in frames.** A trip is roughly 1,000 to 2,000 frames (critique §3);
a colony ant lives about 40,000 and a generation is about 12,000 (critique
§1). Minutes depend on the lab's 1x rate, which has a speed ladder and a
minimum-framerate setting of 60/30/20/10 Hz [E] `src/bin/lab.rs:23,32`.
**Measure the 1x rate before setting any chapter length below.**

---

## Playing it

### The first ten minutes, moment by moment

**The setting** is the lab box with an established colony of about 80 from
the specimen shelf. The near food has just been grazed bare. A fruiting plant
stands 200 cells off, past a water channel. The colony's real current
problem, starving with food far away, is the opening situation.

- **0:00.** You hatch pale, deep in the nest. Sniff a nurse: "knows: nothing
  outside. Hungry." Most read that way: nobody knows where food is.
- **0:40.** Outside, the surface is bare and thin ants idle on the mound. The
  store meter is low and falling.
- **1:30 to 2:30.** You find a way round the channel, and fallen fruit. Taste:
  your crop glows orange; a ring marks the find, yours alone.
- **3:30.** Home. Hungry sisters cluster. Share a taste with one (F). She
  takes an orange tint. In the faked slice, her next walk leans toward
  orange smell. Today she just eats.
- **4:30.** Tap an idle forager; she touches your abdomen. You hurry at the
  channel, the gap opens, her contact pip goes grey, and she circles.
- **5:00.** You turn back; her circle crosses your path. Contact, and a soft
  pulse: you learn the rhythm.
- **6:30.** Arrival. She eats, picks up, turns for home alone, and the first
  laden ant from this patch walks a faint line behind her. Counter:
  *tandems 1 completed, 0 broken; pupils 1.*
- **7:30.** You lose a second pupil for good at the channel. Ninety seconds
  later she finds the fruit anyway: *1 broken, still arrived.*
- **8:30.** The payoff moment. Your first pupil, back at the mound with fruit
  in her crop, taps a nestmate. A second pair sets off without you. A thin
  line on the overlay joins her to you: your first grand-pupil.
- **9:30.** A stranger arrives at the argument. A colony scout returns from
  a closer, poorer patch of seed and starts leading pairs the other way. Her
  pupils are tinted grey. The counter now reads *orange 4, grey 3*.

**Learned in ten minutes:** leading is a skill, lessons spread, and you are
not the only voice.

### The minute-to-minute loop

**Find, taste, return, persuade, watch it spread, find again.** The patches
move: plants fruit and drop, the lab day turns, a heap is eaten out. So the
truth you taught goes stale, and today's best pupils are tomorrow's ghost
trail.

**Your hands.** One steers always; the other taps, shares, holds Wait,
toggles the scent view and pins a pupil. About one decision every ten
seconds: whom to tap, when to slow, carry or lead, go and look at a rival.

**Time control is required.** Fast speeds while walking alone, dropping to
1x at every tap, keep the skill and cut the walking.

### The first hour

- **Minutes 10 to 20.** The fruit patch carries the colony. Store rises.
  Fed ants idle again, so your cascade slows: well-fed pupils stop teaching.
  You learn that persuasion needs need.
- **Your first passing, by choice or by violence.** A beetle at the channel,
  or you Pass on to your best pupil to keep a failing patch taught. The
  candidates glow: a daughter (if you laid), pupils, the ant with most faith
  in you. A pupil keeps her memories; a daughter keeps your genome and knows
  nothing. No old age: the owner ruled *"My player shouldn't die of old age"*
  [E] `steer.rs:32-33`.
- **Minutes 20 to 35.** The fruit runs out; pupils keep walking to the bare
  spot. Un-teaching is a new job: Warn there, or lead them past it to the
  next find. First-learned preferences stick hardest [S].
- **Minutes 35 to 60, the house-hunt** (needs build step 7). Rain soaks the
  nest. You and two colony scouts find three sites. You lead ants to yours
  and watch its quorum ring fill; a rival's fills faster. You go and look:
  hers is drier. Head-butt her, or switch sides?

### The fifth hour

**You have worn a dozen bodies, and the colony has a history you made.**
Named lineages and life records exist, so your chain of bodies reads like a
chronicle: pupils, finds, deaths, schisms.

- **A scouting lineage.** If the lead gain is heritable, your line leads
  more than average: an emerged scout minority, partly yours. The old trust
  you; the young do not know you.
- **A rival colony** (the two-colony bed exists [E] `examples/rivalbed.rs`).
  Whose scouts reach a patch first decides whose trail holds.
- **Born knowing, maybe.** With the memory layer's heritable priors, a line
  that kept being taught "orange is food" may drift toward starting there.
  It would show on the page as "her grandmother was taught this; she was
  born leaning toward it". Possible in dozens of generations, never promised.

---

## What the colony might do

### Behaviours the player will aim for

- **A teaching tree.** Ten minutes after your first run, eleven ants know the
  fruit patch, and you led only three. The overlay draws the tree: you, your
  pupils, theirs. A thick branch is a pupil who teaches well.
- **A highway.** Enough taught foragers keep a route's trail lit, and it
  stops flickering.
- **A move by quorum.** Tandems, then a sudden switch to carrying, brood and
  all, into the dry site you chose.
- **A door and a depot.** You dig an exit toward the east patch and drag
  food to it; traffic reroutes, and idle ants on the mound find work.

### Surprising behaviours, each with its cause

- **The rumour outruns the route.** Ants you never met head for orange smells
  everywhere, because your meals passed mouth to mouth faster than any
  tandem. One finds an orange-fruited plant you never knew. Cause: smell
  learned by meal, place not.
- **A false prophet.** A colony scout leads eight sisters to a dead beetle,
  eaten in a minute. Her pupils come back empty and their faith in her falls.
  Yours rises by comparison, though you did nothing.
- **The ghost trail.** Your old pupils keep marching to a patch that died an
  hour ago, each lap relaying the trail. Cause: first-learned preference
  plus a trail that traffic keeps alive.
- **The schism.** Two equal sites, two factions, brood in both. Then one
  pivotal carrier, sometimes you, moves the losing side over, as in real
  reunifications [S].
- **The colony overrules you.** You push a big warm site; the colony picks a
  smaller, drier one. Twenty minutes later yours floods. You were one
  irrational ant (Sasaki & Pratt [S]).
- **The deterrent harvest.** Pupils fetch a bitter food you shared, having
  learned its smell, not its taste (the *Camponotus* override [S]).
- **Grief as a gap.** You die or pass on. The patch you served empties within a cycle,
  because you were its only teacher. Or it does not, because you built a
  tree. The difference is your score, and nobody has to tell you.

---

## Fun and boring

**What kills this seat, ranked.**

1. **The first lesson lands on an ant that cannot hear it.** On today's pool
   a pupil does not come back. If the faked slice hides that and the real
   build ships it, the seat is slice 0's dead trail again, at the centre.
2. **An escort mission.** At the walkthrough's own pace a run takes a minute
   or more plus the walk home: 3 or 4 runs in ten minutes. Without the
   cascade the scout is a delivery service, and the cascade has no build
   route until step 5. It is not a feature; it is the game.
3. **Tandem tedium.** Real runs are four times slower than walking alone [S].
   Ueda softened Agro's resistance because it annoyed players [S]. Keep runs
   short, give time control, and make the skill the pacing, not the walking.
4. **Follow-me collapse.** Any verb that puts a column behind you becomes
   Pikmin with a lag. Display rouses ants that go alone, capped by your crop.
5. **Losing your pupils among 300 ants.** Tint by knowledge, draw the tree,
   pin and follow a pupil; text on a page is not enough (critique §1).
6. **Rivals that read as bugs.** A rival needs her own tint and counter, so
   she reads as an opinion.
7. **Death as punishment.** Death must be a chapter: what you taught stays,
   and you return as someone who carries part of it.
8. **The colony fine without you, or doomed without you.** Target: colony
   scouts find some food; you find better and spread it faster.

**What keeps it alive past hours.**

- **Discovery.** Patches move, plants fruit, rain floods. There is always
  something no ant knows yet.
- **The tree you grew.** Knowledge reaching ants you never touched is the
  seat's deepest pleasure: your influence, drawn.
- **Arguing.** Rivals, quorums and a colony that can say no make every
  persuasion a contest.
- **Bodies as chapters**, ended by violence or by choice, with a real choice
  of the next body.
- **Un-teaching.** The colony's mistakes are often your old lessons; no other
  seat has that puzzle.

---

## Building it

### Fit with each owner rule

| Rule | How the seat fits |
|---|---|
| **One brain, no castes, roles emerge** | Accept, lead and display are body rules every ant has, with heritable gains seeded above zero. Who leads is set by state and genes; a scout minority emerges |
| **Dig a multi-room nest and forage all game** | The scout serves foraging and the nest's moves; digging is untouched |
| **Graded, visible, satisfying** | Runs break and still half-work; a tree's reach is continuous; a quorum ring fills; faith rises and falls by ant |
| **Frame cost is hard** | No new scent planes for the core; tens of bytes per ant; contact checks only for ants in a tandem |
| **Features default on** | Each piece ships on only if it beats its off arm on paired seeds, never because a sweep reads neutral (plan §7) |
| **Not "walk and they follow"** | Followers come one at a time, by touch, and can be lost; the colony commits by quorum; rivals argue; what you taught outlives you |

### Build steps, in order, each with what it proves

0. **Look and trace first.** (a) A worker-level trace of trail B. Slice 0's
   bed already had workers at food in every arm (median 5.5), so a trail
   there carried no new information; repeat where there is a gap. (b) Print
   the pool's last-food weight, hand-seed it as a positive control (a unit
   test sets 8.0, critique §3), and check an ant carried to food comes back.
   (c) Does a scripted follower moved to far food deliver once? *Proves the
   floor each verb stands on.*
1. **The faked ten-minute slice** (below). *Proves whether persuading is
   more fun than being followed, before any learning exists.*
2. **The player as a worker, with keys.** Bind `sim::steer` in the lab app.
   Change two hard-coded rules for the scout: the steered body puts its
   whole colony on the queen rule [E] `creature.rs:23015-23020`, which in an
   established colony stops *every* member laying while any other has young
   [E] `creature.rs:22951-22963`; the colony needs its process rule back. No
   old age stays, per the ruling [E] `creature.rs:7943-7947`. *Proves the
   body is an ordinary worker.*
3. **Follow and lead.** Follow is an instinct once a tap is accepted, the
   body bypassing the brain. *Proves a run puts the follower at the food.*
4. **The memory layer** of plan §7, two or three remembered places,
   hand-seeded or a heritable-gain pull, trained on the moving-food bed.
   *Proves pupils return unaided; learners beat frozen ants on paired seeds.*
5. **Lead as a rule every ant has.** *Proves the cascade: knowledge reaches
   ants the player never touched. Gate: far-food deliveries beat no-tandem on
   paired seeds, never "neutral". This is the engine's side benefit.*
6. **Display, coupled to a trail.** The risk is the recruit harm measured
   on the lab ant: roused ants starving on the surface (442 against
   298/323). (The "363 recruits, 11 surfaced" finding and its fix,
   `forage_out`, both belong to the needs walk, which no game runs [E]
   `how-the-ant-works.md` §17, so neither is cited here.) *Proves urge bound
   to a remembered route beats that harm on paired seeds.*
7. **Carrying nestmates, emigration and quorum.** Large and new. *Proves the
   colony moves by quorum and sometimes splits.*
8. **Pass on, standing and graded colony odour.** *Proves a change of body
   reads as a chapter, not a reset.*
9. **Head-butts and rival scouts as colony ants**, nearly free once lead and
   display are rules every ant has.

### How each new piece gets weight (or it ships dead)

**Nothing new enters the brain at weight zero.** Accepting a tap, leading,
displaying and faith are *body rules with heritable gains*, as the walk's
named gains are, and the founder's gains are hand-seeded above zero with a
mutation width above zero. (The six existing walk gains ship at allele 0 and
width 0 [E] `queen.md` §1, which is exactly the trap.) Faith multiplies the
accept chance; it is not a brain input.

**Teaching is altruistic.** The leader loses foraging time; the colony
gains. Offline training selects whole colonies (engine brief), which can keep
a costly gain if teaching colonies out-forage: train on far, moving food
(`foodflip=`). Inside one played colony the gain may erode; log its drift
over five hours.

### Frame cost

Per-ant state and a few contact checks are small against "every pixel
simulated". Watch the tandem pairs (each checks contact every tick) and
emigration. A taste palette costs one scent plane per smell (plan §7: 3 to
4 at most). Quote each step's worst frame from `examples/ascii.rs`.

### The cheapest faked ten-minute slice

**One question:** is persuading one ant at a time more fun than the design
the owner rejected? Build nothing that learns.

- **Scene and body:** the opening box above; a steered worker, with two
  code edits, not switches: keys bound in the lab, and the colony's own
  breeding rule kept (the queen rule would freeze breeding and confound
  "adults starved").
- **Tap:** the nearest idle, not nest-bound nestmate within 3 cells.
- **Follow (scripted):** within 2 cells, the follower steps toward the
  leader and ignores her mind. A gap over 3 cells for 60 frames breaks it;
  she spirals out for 300 frames, then returns to her mind.
- **"Learned" (scripted):** on arrival, "knows patch P" pulls her to P for 3
  trips, losing one each time P is empty.
- **Cascade (scripted):** a flagged ant home with food taps a neighbour with
  chance *c* per visit.
- **Share (scripted):** F gives "knows the smell", biasing her walk toward
  that food's kind.
- **Rival (scripted):** from frame 4,000, a colony scout leads runs to a near,
  poorer seed patch, pupils tinted grey.
- **Overlays:** tint by flag, the teaching tree, contact pips, follow-cam.

**Three arms, all with the same scripted "knows patch" reward:**

- **A, the scout:** everything above.
- **B, the owner's rejected design, as the control:** you walk to food and
  the colony follows you. Every idle nestmate you pass within 6 cells falls
  in behind you and follows to the food, then forages it. No taps, no
  breaks, no cascade, no rival.
- **C, the escort:** A minus cascade, rival and shared smells. Timed: if it
  is dull by minute five, the seat rests on step 5, and the owner knows it.

Post A and B as GIFs on a **blind** review card for the owner to judge; C
goes beside them as a labelled timing check. **Meta:** the cascade chance
*c*; tandems started, completed and broken; pupils making a second trip;
pupils who led; ants knowing each patch at 10 minutes; adults starved.

If A does not beat B by eye, the premise is wrong, at the cost of days. If
A beats B but not C, friction alone was the fun, and the escape layer has
not earned its build cost.

---

## Risks and open questions

### Biggest risks to fun

Ranked under Fun and boring. The top three: **an escort mission** until the
cascade is real, **tandem tedium**, and **follow-me by the back door**. Also
untested: Pass on could make death meaningless if it is free; it should cost
the standing your old body held.

### Biggest risks to feasibility

1. **Weight zero, twice.** Memory the shipped pool ignores [E]
   `mind.rs:539-552`, and the new accept, lead and faith rules, which need
   seeded heritable gains (Building it).
2. **Teaching is altruistic** and may erode inside one colony.
3. **The trail is untraced.** The highway and Display inherit slice 0's null.
4. **Recruit harm.** Display ships only if it beats 442 against 298/323.
5. **Emigration is a whole subsystem**: the best set piece, the dearest.
6. **The cascade could boom** and end the player's role. Teaching needs a
   cost (time off foraging) and a need (hungry nestmates).

### Decisions only the owner can make

1. **A seat of its own, or the middle stage of queen, scout, colony mind**
   (perspectives §7)?
2. **Your ruling holds by default:** *"My player shouldn't die of old age"*.
   Would you ever want old age on for a scout body, as an option?
3. **May the player pass on into a pupil, not only a daughter?** No biology
   under it; the most game-like liberty here.
4. **Is per-ant standing (faith in one leader) an acceptable liberty?**
5. **May following be a hard instinct** once a tap is accepted, inside "one
   brain"?
6. **May the player lie?** Leading pupils to a beetle or bitter food: is it
   supported, recorded on graves, and allowed to hurt the colony?

### What the review changed

1. Cascade had no route to weight: **fixed**. Accept, lead, display and faith
   are seeded heritable body rules; teaching's altruism and step 5's gate
   are stated.
2. Owner ruling misread: **fixed**. Quoted exactly; no old age by default;
   bodies change by violence or the new Pass on verb.
3. A/B could not detect the objection: **fixed**. Arm B is now the rejected
   design itself; arm C is the escort; *c* is in meta.
4. "Now" labels needing work: **fixed**. Follow, Warn, Taste, daughter
   standing relabelled; "now" defined as an engine hook with no lab keys.
5. Head-butt misread of Seeley: **fixed**. Biology restated; "go and look"
   marked a liberty.
6. Slice switches do not exist: **fixed**. Listed as code edits, with the
   queen rule's breeding freeze.
7. Stale dead-ends citation: **disputed in part**. `forage_out` and the
   "11 of 363" finding are both the needs walk, which no game runs (§17), so
   neither is cited; the risk rests on the lab ant's recruit harm.
8. Pace had no mechanism: **fixed**. Replaced by Pick a pupil (size match).
9. Break rate mis-sourced: **fixed**. 19% to 70% range, a tuning knob.
10. Rain World precedent: **fixed**. Tagged [K], dropped as precedent.
11. Escort-mission arithmetic: **fixed**. Fun #2 and arm C say so.
12. Unchecked frame rate: **fixed**. Minutes removed; measure 1x first.

---

## Sources

**Biology (search summaries and abstracts, read this session; WebFetch of full papers mostly failed):**

- Franks & Richardson 2006: [Bristol portal](https://research-information.bris.ac.uk/en/publications/teaching-in-tandem-running-ants/), [Bristol news](https://www.bristol.ac.uk/news/2006/879.html), [Nature](https://www.nature.com/articles/439153a)
- Leadbeater, Raine & Chittka 2006 (title only): [Royal Holloway record](https://pure.royalholloway.ac.uk/en/publications/social-learning-ants-and-the-meaning-of-teaching/)
- Break rates and size matching: [Wagner et al. 2020/21](https://link.springer.com/article/10.1007/s00265-020-02941-x); lost followers, *T. curvispinosus*: [reviewer's link, not read](https://link.springer.com/article/10.1007/s00265-008-0565-9)
- Interrupted runs: [Basari, Bruendl et al. 2014](https://antwiki.org/wiki/images/2/23/Basari%2C_N.%2C_Bruendl%2C_A.C._et_al._2014._Landmarks_and_ant_search_strategies_after_interrupted_tandem_running_%2810.1242%40jeb.087296%29.pdf)
- Routes, carrying, reverse tandems: [bioRxiv 2024, *T. nylanderi*](https://www.biorxiv.org/content/10.1101/2024.06.05.597530.full.pdf), [PMC 2039849](https://pmc.ncbi.nlm.nih.gov/articles/PMC2039849), [Naturwissenschaften 2007](https://link.springer.com/article/10.1007/s00114-007-0273-8)
- Quorum and latency: [Pratt 2005](https://ideas.repec.org/a/oup/beheco/v16y2005i2p488-496.html), [Mallon, Pratt & Franks 2001](https://asu.elsevierpure.com/en/publications/individual-and-collective-decision-making-during-nest-site-select)
- Group rationality: [Sasaki & Pratt 2011](https://ideas.repec.org/a/oup/beheco/v22y2011i2p276-281.html)
- Reunification: [Doering & Pratt 2019](https://cob.silverchair.com/jeb/article/222/5/jeb194019/20655/Symmetry-breaking-and-pivotal-individuals-during)
- Honeybees: [Cornell, quorum](https://news.cornell.edu/node/276432), [Cornell, stop signals](https://news.cornell.edu/node/271186), [Live Science](https://www.livescience.com/17395-honeybees-head-butts-decisions.html), [PMC 3758014](https://pmc.ncbi.nlm.nih.gov/articles/PMC3758014/)
- *Camponotus* recruitment: [*C. socius*](https://asu.elsevierpure.com/en/publications/recruitment-behavior-in-camponotus-socius-hym-formicidae)
- Trophallaxis: [Provecho & Josens 2009](https://repositoriosdigitales.mincyt.gob.ar/vufind/Record/BDUBAFCEN_243b5d76a152c5da53b03c93959a158b), [social info overrides own](https://repositoriosdigitales.mincyt.gob.ar/vufind/Record/CONICETDig_ebd4dc53e7dd2736eb6ff25795e85387)
- *Lasius niger*: [Koch, Kabas & Czaczkes 2024](https://link.springer.com/article/10.1007/s00265-024-03430-1)
- Social carrying: [AntWiki](https://antwiki.org/wiki/Social_Carrying)

**Games (searched this session):**

- The Last Guardian: [Siliconera, Ueda](https://www.siliconera.com/fumita-ueda-shares-more-about-the-last-guardians-main-hero/)
- Agro: [Glitterberri interview](https://glitterberri.com/extended-developer-interview/the-graphic-designers-tell-it-straight-4/)
- Rain World (now [K] in the text): [Scavenger](https://rainworld.miraheze.org/wiki/Scavenger)
- Journey: [Wikipedia](https://en.wikipedia.org/wiki/Journey_(2012_video_game)); Pikmin: [Idle](https://pikminwiki.com/Idle); Spirit of the North: [WellPlayed](https://www.well-played.com.au/spirit-of-the-north-review/)
- From memory [K]: Pikmin stragglers lost at sundown.

**Repository files relied on (read only):**

- `Reports/teaching-the-colony-2026-10-25.md` §1, §2, §4, §7-§10
- `Reports/teaching-the-colony-2026-10-25/` `engine-brief.md`, `perspectives.md`, `critique.md`, `queen.md`
- `Reports/play-as-ant-slice0-2026-10-11.md` §2, §4, §5
- `Reports/how-the-ant-works.md` §7, §12 (`RECRUIT` row), §16, §17
- `Reports/dead-ends.md` (recruit, resting-forager reserve and give-up entries)
- `src/sim/steer.rs` (module doc, `SteerInput`), `src/sim/creature.rs:7943-7947` (old age), `:22951-22963` (queen rule), `:23015-23020` (`regime_of_colony`), `src/bin/lab.rs:20-34` (speed keys), `examples/queenprobe.rs` (only steer driver), `src/sim/creature/mind.rs:539-552` (`enter_last_food`), `src/sim/creature/options.rs` (verbs, feature list)
