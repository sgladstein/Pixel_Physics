# The guest: you live inside the nest, and the colony learns to keep you

*Seat design, 2026-10-25, revised the same day after an independent review
(listed at the end of "Risks and open questions"). Design research only: no
code changed, no runs. Marks: [S] checked this session (URL in Sources), [K]
from memory and not checked, [E] read in this repository (file:line, under
`src/sim/` unless named).*

---

## The pitch

**You are a small animal that lives in the ant nest, and the whole game is
getting the colony to keep you.** The default body is a rove beetle the size
of a worker. You smell wrong and you are not family. Every ant you meet
decides what you are. Get it right and they groom you, feed you mouth to
mouth and carry you to the nursery. Get it wrong and they bite, drag and kill
you.

**The fantasy is the con artist in a city of blind giants who smell.** You
never command an ant; it acts on what it believes you are. The colony "learns"
to keep you on three clocks: each ant decides in a second, the nest's smell
drifts over minutes, and perhaps the bloodline turns gullible or suspicious
over hours.

**The answer in one paragraph.** A guest teaches the colony two things. First,
*how to treat you*: ignore, inspect, tolerate, feed, carry, defend or kill,
read straight off ant behaviour. Second, *side effects* of living with you: a
mutualist that pays sugar pulls foraging to where it sits, and a parasite that
eats brood may breed a more suspicious colony. Two playable guests carry this:
**the beggar** (a parasite, after *Lomechusa*/*Atemeles*) and **the herd** (a
mutualist, after aphids and treehoppers). **The gardener as symbiont** gives
the keeper gnome the same odour model.

**What the engine gives, said honestly.** Every animal carries a three-number
scent and a heritable tolerance radius, and both drift with each birth [E
`organism.rs:7758-7787`, `assets/species/ant.ron:996`]. An ant on its nest
blends toward the nest's odour [E `creature.rs:11349`]. Larvae are fed by
touch through a body rule that checks kin by scent [E `brood.rs:1051-1091`].
**But today's lab ants never consult that scent model about a stranger.**
Their mind sorts nestmate from stranger by colony number [E
`creature/options.rs:1059-1061`]. Feed is offered only to the same colony
[E `options.rs:877-907`]. Kin needs the same species unless a switch that is
off for every shipped kind is set [E `creature.rs:11819`, `organism.rs:4282-4293`].
Mind ants skip the old hand-built stranger alarm [E `creature.rs:8022`], but
they bite through their own Attack verb. Every trained pool member has it
switched on, and it is offered against any animal of another colony in reach
[E `assets/minds/std_pool.txt` member 0 slot 8; `options.rs:905-907`;
`mind.rs:2648-2665`]. So today a guest with its own colony number is never fed
and may be bitten by any ant it touches, whatever it smells like. The danger
is real; the middle is missing.

**So the seat is new body rules plus one owner switch.** The rules are
contact rules that read the existing scent model: an inspection ladder, a
begging share and a host-judged soak. They sit beside nursing by touch, which
is already such a rule. Because they are body rules, they do not need trained
brain weights. That keeps the seat clear of the weight-zero trap. **The price
is hand-made behaviour**, which this project usually avoids. That is the
owner's call (decision 5).

**Why it is still my favourite seat, and where it fails.** The data it needs
(scent, tolerance, drift, blend) already exists, and its reactions are local
and cheap. Each one is visible on an ant's body. It fails in three places.
The colony mostly learns about *you*, not about digging and foraging. The
parasite feeds on the owner's main colony by design. And nothing plays until
a creature, two switches and three rules are built.

---

## The biology it rests on

**Myrmecophiles are the many animals that live with ants, from tolerated
scavengers to full impostors [K].** Their common trick is the one the engine
models: ants know family by smell, and smell can be borrowed.

**The beggar beetles (*Lomechusa*, *Atemeles*).** Their larvae live in
*Formica* nests and are treated as the ants' own brood. They beg like ant
larvae, but harder, and tracer experiments showed they win food from the host
larvae [S, Hölldobler 1967 abstract]. Adoption depends on chemical signals
[S]. Adult glands appease hosts and stimulate adoption, so the beetles can
live and lay eggs in a defended nest [S, PLoS ONE 2018]. *Wasmann's
"pseudogynes"* in infested colonies were once blamed on the beetles. That idea
has been dropped: the deformed workers come from a gland disease [S, AntWiki,
Labial Gland Disease].

**The large blue caterpillars (*Phengaris*/*Maculinea*).** They smell like
ant brood and are carried into *Myrmica* nests. Two strategies exist side by
side, and the difference is this seat's scoring:

- **Cuckoo** (*M. rebeli*, *M. alcon*): fed by worker regurgitation. Late
  larvae imitate the queen's sounds and are fed before the ants' own larvae
  and rescued first [S, Barbero 2009, via perspectives.md].
- **Predator** (*M. arion*): eats the brood. A *M. sabuleti* nest needs at
  least 354 workers to rear one [S, Thomas & Wardlaw 1992].
- **The trade**: one nest can support about six times more cuckoo
  caterpillars than predatory ones. Cuckoos must match one host closely;
  predators are generalists [S].
- **The arms race**: the closer the chemical match, the better the
  exploitation, and host populations show signs of counter-adapting [S, Nash
  2008, via perspectives.md].
- **The wasp**: *Ichneumon eumerus* hunts the caterpillars inside the nest.
  Its secretions make *Myrmica* workers attack each other while it slips by
  [S, Thomas 2002, via The Scientist].

**The singing beetle (*Paussus*).** Lives with *Pheidole* and makes sounds
resembling three ant castes, the queen included [S, Di Giulio 2015].

**The thieves and janitors.** The silverfish *Atelura* eats nest debris and
sometimes steals food during trophallaxis [S]. The woodlouse *Platyarthrus*
eats droppings and mould, but hungry captive ants will kill and eat it [S,
anecdotal]. Ant crickets beg by mimicking grooming and trophallaxis, and
escape by jumping. Specialists take colony odour by grooming ants and are
rarely attacked; generalists often are [S, Kyoto seminar].

**The farmed guests (aphids, treehoppers, lycaenid caterpillars).**

- Ant footprints slow aphids and suppress wing growth [S, Oliver 2007].
- *Lasius niger* ate aphids eight times faster when given another sugar
  source, and tended them less; extra protein changed nothing [S, Offenberg
  2001]. A 2024 preprint suggests ants cull low-honeydew aphids [S, preprint].
- Tended treehoppers do better even with predators excluded. Modelling
  suggests guarding lets them overexploit their plant and cycle [S, Morales].
- *Lasius flavus* carries root-aphid eggs into its brood chambers over winter,
  piles and grooms them [S, 2025 study].
- *Acropyga* queens fly to found a colony carrying a mealybug in their jaws
  [S].
- Lycaenid secretion lowers attending ants' dopamine, so they walk less and
  stay, and fight harder when the caterpillar signals. The authors suggest
  such "mutualisms" may be partly parasitic [S, Hojo, Pierce & Tsuji 2015].

**Liberties this design takes.**

1. **One player animal does several species' tricks**, a kit drawn from the
   whole guild.
2. **The guest can move the colony's odour.** Rubbing is symmetric in the
   engine, so a rubbed worker drifts toward you too. Real guests mostly copy
   the host; shifting its odour is a stretch [K].
3. **Generations are compressed.** A real arms race runs over decades. Here
   it must show inside an hour of fast-forward, or not at all.
4. **"Learning" is smell and body rules, not memory.** Real ants learn odours
   in one to three trials [K]; on today's ants that channel is dead.

---

## Your verbs

Codes: **now** = works on today's lab ants with only a guest body added (rare);
**switch** = needs the owner's guest-kin switch (decision 2); **rule** = needs
a new body rule, named; **mem** = needs the memory layer of plan §7;
**new** = other new engine work. A guest added today with no rules is never
fed, and every mind ant that touches it is offered a bite, by colony number.

| Verb | What you do | What the ant perceives | What the player sees | Needs |
|---|---|---|---|---|
| **Rub** | Groom a worker you touch | Her scent and yours move toward each other | Your tint shifts toward the colony's; hers shifts a shade toward yours | switch + rule: `blend_with_kin` (`creature.rs:11459`) runs only on a same-colony Feed today |
| **Soak** | Lie on the nest patch or brood pile | Your scent moves toward the nest's; the nest's a little toward yours | Your tint settles; the pile's haze drifts toward you | rule: the nest blend's foreign-visitor gate is judged by the visitor (`creature.rs:11397-11406`); flip it to the host |
| **Beg** | Rear up and tap a nurse's mouth | A guest asking for a share, accepted if your scent is within her tolerance | The nurse stops, a droplet passes, your belly swells | switch + rule: a nurse-side share; `nurse()` has no step where a nurse chooses whom to feed (`brood.rs:1051-1120`) |
| **Beg harder** | Hold the beg | A stronger ask; she gives a bigger share | You fill faster; nurses near you run low | rule: share size scales with beg strength (absorbs the cut "Sing") |
| **Appease** | Offer a droplet from your gland to an ant inspecting you | Food in reach; energy rises | The inspector stops, eats and walks off slower | rule: a calm step in the ladder |
| **Steal** | Slip between two ants sharing food | Nothing; the meal shrinks | A droplet diverted to you; one ant shakes its head | new |
| **Eat a larva** | Take a brood item in the pile | A missing larva, a corpse, alarm at the spot | A small corpse, an alarm cloud, ants converging | new (brood is a powder-like stage cell, `creature.rs:26906`) |
| **Go limp** | Fold up in a tunnel | Brood lying out of place | A worker lifts you and carries you inward | new: brood carry is off and only re-piles brood (`brood.rs:448-456`); lifting a multi-cell body is new |
| **Hide** | Wedge into a crevice or under the pile | You are out of reach | Inspections stop while you are hidden | now (body and dig physics) |
| **Jump** | Spring away from a biting ant | A target gone | A ballistic hop out of trouble | now: the ballistic jump exists (`creature.rs:28654`) |
| **Lay** | Put your egg in the brood pile | Brood | Your larva among theirs | new |
| **Sweeten** (herd) | Raise or cut honeydew output | Food on the plant | Droplets on the stem; tenders gather or drift off | new: honeydew liquid |
| **Walk the herd** (herd) | Move the herd one stem or plant | Food has moved | A procession up the stalk; traffic may follow | new (creature); the follow depends on trail B, untraced |
| **Cry** (herd) | Drop alarm when a beetle comes | Alarm scent | An alarm cloud; whether a ring forms is the test | new: alarm at will, and a response rule |
| **Grow wings** (herd) | Raise a winged generation | Nothing until they leave | Winged young; ants biting the wings off, or a fly-off | new |

**The one rule under most verbs: the inspection ladder.** On contact between a
mind ant and a non-nestmate, a body rule reads the scent distance against the
ant's tolerance and rolls one rung: walk on, pause and antennate, display (an
alarm puff), nip, drag, kill. The odds shift with the distance and with alarm
already in the air. It reuses the contest module's floor-bounded commit [E
`contest.rs` header], so no rung is ever certain. It must also run between
nestmates if the drift scenes in section 6 are wanted.

**Notes on the best ones.**

- **Rub and soak are the core verb.** An ant on its nest takes 10% of the
  gap to the nest's odour per exchange and gives back 2% [E
  `creature.rs:3639,4992`]. A guest joining that exchange is the specialist ant
  cricket exactly. Rub has a cost the player can see: the rubbed worker drifts
  toward you and becomes a shade foreign to her sisters.
- **Beg is the cleanest verb that delivers.** It needs a new rule in which a
  nest-bound ant shares from her bank or crop when an accepted guest asks.
  Acceptance is graded: the probability falls as your scent nears the edge of
  her tolerance, so the same disguise passes some nurses and fails others.
- **Eat a larva carries the parasite's moral weight.** It fills you fast but
  leaves a corpse and an alarm cloud that raises the ladder's odds nearby.
- **Walk the herd is the mutualist's only teaching verb**, and the weakest
  link: it pays off only if trail B recruits, which slice 0 could not show.

---

## How a lesson travels

**The guest's lessons travel through smell, body rules and perhaps selection.
None of them runs on today's lab ants without the new rules; the per-ant memory
channel is dead until plan §7.**

| Lesson | Channel | Clock | Today? |
|---|---|---|---|
| "This one is family" | **Place**: your scent against each ant's tolerance, judged at contact | seconds | **No.** The mind uses colony number, so every guest is a stranger it may bite; kin needs the same species. Needs the switch and the ladder |
| "The nest smells a bit like you" | **Ant to ant**, through the nest's shared odour | minutes | Blend runs for ants; a guest needs the host-judged soak rule |
| "Feed it", "carry it" | **Place**: body rules for brood | seconds | **No.** Needs the begging share; carry needs new work |
| "There is food on that plant" | **Place**: laden returners lay trail B | minutes | Untraced: a hand-laid trail recruited nobody in slice 0 |
| "Children are born smelling like this" | **The young**: a child inherits its parent's *current* scent [E `creature.rs:11336-11340`] | brood cycle | Yes, for ants |
| "Be more or less trusting" | **Bloodline**: tolerance drifts per birth at a third of `scent_drift` [E `creature.rs:11653`] | lineage | A hypothesis (below) |
| "That smell meant sugar" | **One ant's memory** | seconds | No. Needs plan §7's valence inputs |

**The bloodline channel is a hypothesis, not a promise.** Feeding a guest
costs the feeder energy. In this engine laying is an energy decision any ant
can make [E brief]. So lines that feed you could lay fewer eggs, and selection
could narrow tolerance. Three things argue against it showing in a session:

- one guest drains a few nurses in a colony of about 400;
- tolerance drifts only about 0.05 a birth (0.15 / 3), against a radius
  near 1.0;
- ant-to-ant spread grows roughly as 0.05 × √generations, so after 16
  generations, about 48 minutes at 1x, ants differ by only about 0.2.

So early on, **ants differ in the moment, not by gene**: the ladder's dice,
the alarm around them, hunger. The "enforcer" lines in section 5 need step 6
to show real spread, or a wider tolerance drift (an owner dial, narrowed on
2026-10-03 because lines bit their own nestmates [E `organism.rs:7778-7781`]).
**First check**: print the per-ant tolerance spread from an existing long run.

---

## Playing it

### The first ten minutes (the beggar)

**The opening goal is one verb that delivers in the first minute: rub an ant
and watch the next ant treat you differently.** Everything below assumes the
inspection ladder, the begging share and the guest-kin switch are built.

- **0:00.** You drop from a leaf onto the soil, ten cells from the mound. A
  ring on your back shows your colour; the colony's hangs as a haze over the
  door. They are far apart.
- **0:20.** A forager bumps you. She stops dead and jerks: a display, with a
  puff of alarm. You jump clear. Lesson one, with no text: *you smell wrong*.
- **1:00.** A returner comes by. You **rub** her for three seconds. Your ring
  warms one step toward the haze, and hers cools a shade toward you. The next
  ant to bump you pauses, antennates and walks on. That pause is the first
  payoff.
- **2:00.** You rub two more, then **soak** on the edge of the mound. At the
  door a guard antennates you longer than the others. She nips once; you limp.
  Some ants, on some rolls, are pickier: Hitman's "enforcers", but carried by
  chance and alarm, not a uniform [S, Hitman wiki].
- **4:00.** Down to the brood pile. You **soak** among the larvae. Nurses
  step over you.
- **5:00.** You **beg**. A nurse stops; a droplet passes; your belly bar
  swells. The first time an ant *gave* you something.
- **6:30.** You **beg harder** and fill faster. The nurses near you run low,
  and the larvae they would have fed next go pale. The first graded harm,
  shown on bodies, not a meter.
- **8:00.** Greed: you **eat a larva**. Your belly fills at once. A corpse, an
  alarm cloud, and four ants converge. You **hide** under the pile; one finds
  you and nips, and you **jump** clear.
- **9:30.** You **lay** your egg in the pile.
- **10:00.** The host ledger opens, in the lab's life-record style: *fed 9
  times, bitten twice, larvae lost near you 2, your young 1*. It names the
  guard who bit you.

### The minute-to-minute loop

**Three needs pull against each other: your belly, the ants' reactions, and
the host's health.** A minute goes like this:

1. Read the ants near you. Each shows its reaction on its body: walks on,
   pauses, jerks, nips, drags.
2. Pick a source: beg a nurse, steal from a pair, or eat a larva.
3. Pay the cost: begging is quiet, stealing is noticed, eating is loud.
4. Repair your disguise: rub or soak somewhere safe.
5. Watch the host: pale larvae, a thinner brood pile, alarm at the door.

**Your hands, every minute.** Arrow keys through a living crowd; one key each
for rub, beg, steal, eat, hide, jump. You choose *which ant* to rub (a nurse
smells most like home) and *when* to beg (when her crop is full). Each choice
answers within seconds. There is no suspicion bar: the ring on your back and
the ants' bodies are the only gauges.

### The first hour

**You play a lineage, not a beetle.** When you die you take over your eldest
larva in the pile, as in Rogue Legacy or Crusader Kings, where you play the
heir [K]. Over the hour:

- **Minutes 10–25.** Your larvae grow on ant food; brood near you thins; the
  lab's population graph bends down slightly.
- **Minutes 25–40.** Fast-forward through a few ant generations. The nest's
  haze creeps toward your colour, because you soak there every day.
- **Minutes 40–60.** The first crisis. Either the colony shrinks (you took too
  much, and your own young starve with it), or alarm builds so often near the
  pile that inspections turn to bites. The choice: tighten your disguise, eat
  less, or leave for another nest. The two-colony bed is the second host [E
  `examples/rivalbed.rs`].

### The fifth hour

**If the arms race shows (step 6), the fifth hour is reading it off the
colony and choosing hosts.** By then:

- **Your host has a culture**: more displays at the door, fewer begs answered,
  smaller tolerance numbers on every page. If step 6 shows no drift, this
  hour falls back to host switching and the herd.
- **You switch hosts as a strategy.** The *Nash refuge* idea: a second colony,
  less adapted to you, lets the first one relax [S, Nash 2008]. With you away,
  selection on its tolerance stops; whether it drifts back is unmeasured.
- **You can go straight.** Switch to the herd, and the same colony must learn
  to farm you.

### Playing the herd (mutualist), briefly

**The herd is played as a unit of five to twenty bodies on one plant.** You
choose the stem, the sweetness and when to move:

- **Sweeten** to draw tenders; every droplet costs your growth.
- **Cry** when a beetle comes, and watch whether a guard ring forms.
- **Walk the herd** to a fresh plant before yours is sucked dry.
- **Grow wings** when milked too hard, and watch them bite the wings off [K;
  S Oliver 2007 on wing growth].

**Its honest weakness: it is mostly waiting, on an untested signal.** Moving
the herd pays off only if trail B pulls workers, which slice 0 could not show.
It needs a tending rule (ants near a sweet herd stay and collect) before any
of this plays.

---

## What the colony might do

### Behaviours the player aims for

- **Adoption.** A nurse lifts you in a tunnel and carries you to the brood
  pile, the *Maculinea* moment. Needs multi-cell carry (new).
- **Feeding you first.** In a lean minute, nurses empty their crops into you
  and the larvae go short. Satisfying, then troubling.
- **A guard ring.** The herd cries alarm as a beetle climbs the stem; six
  tenders turn outward. Needs a defend response.
- **A road to your plant.** Laden returners from the herd lay trail B, and the
  busiest path runs to your stem. Depends on trail B recruiting.
- **Indoors for the night.** Tenders carry the herd's young into a brood
  chamber, as *Lasius flavus* does with aphid eggs [S].

### Surprising behaviours, each with its cause

- **The rubbed nurse becomes a stranger.** Rubbing is symmetric, so the nurse
  you groomed most drifts toward your odour. If the ladder runs between
  nestmates, her sisters start antennating her. Cause: `blend_with_kin` moves
  both sides [E `creature.rs:11459-11466`].
- **Feeding you sinks the colony's next generation.** Every share you take is
  energy a nurse cannot spend on an egg. The births line falls a brood cycle
  after you arrive.
- **A civil split, if the ladder covers nestmates.** A narrow line turns on
  its drifted sisters, as lines have before [E `organism.rs:7778-7781`], and
  the census names a new kind [E `world.rs:10267`]. Note that an ant at its own
  nest always rejoins its odour [E `creature.rs:11389-11401`], which damps this.
- **Sugar elsewhere gets the herd eaten.** The gnome leaves sugar out; the
  colony tends the herd less and eats it more, eight times faster in the real
  study [S, Offenberg 2001]. Two players' goods compete.
- **The herd kills its plant.** Guarded aphids multiply, suck the stem dry and
  crash, as Morales modelled [S], on top of the colony's own boom-bust.
- **Honeydew floods a gallery.** Tenders carry the herd underground, it keeps
  dripping, and a sticky pool closes a low tunnel. Free from one physics.
- **Rescued before their own young.** Rain floods the nursery; carriers lift
  you first and larvae drown. Real biology [S, Barbero 2009], and a harm the
  player can cause or avoid.
- **Idle ants find a job, perhaps.** Fed ants that idle on the mound could
  stand around the herd instead. That only happens if the tending rule exists.
- **The wasp's trick, a dark variant.** A discord scent makes nearby workers
  bite each other while you slip by [S, Thomas 2002].

---

## Fun and boring

### What kills this seat, ranked

1. **The colony reacts by number, not smell.** Today a guest is a
   colony-number stranger: offered as a bite on every touch, never fed, blind
   to the disguise (section 1). If the new rules cannot override that, the
   seat is binary: hidden or bitten.
2. **Meter-watching stealth.** Every state must show on an ant's body: pause,
   jerk, nip, drag.
3. **The herd as a screensaver.** An aphid that sits and sucks has no verbs,
   and the herd's one teaching verb waits on trail B.
4. **Harm that feels bad, not clever.** Scoring by how many guests the host
   can carry fixes most of it, as biology does: a nest supports about six
   times more cuckoos than predators [S].
5. **The arms race is invisible.** Tolerance barely drifts in an hour
   (section 4). The deepest hook may not exist in play.
6. **It sidelines the main game.** A seat where the colony is your food or
   backdrop may be wrong for the lab, whatever its charm.

### What keeps it alive past hours

- **A relationship read off behaviour**, ant by ant.
- **A lineage you play through**, so a run is a family story.
- **The moral spectrum.** Beg, steal, appease, eat: four ways to take, graded
  by their cost to the host. Hojo's "reward that is also a drug" makes even
  the mutualist grey [S].
- **Host switching.** Two colonies, each at its own pace, is a strategy layer
  no other seat has.

### Is exploiting the colony fun, and how is it graded?

**Yes, if the fun is the con and the restraint, not the cruelty.** This is the
opposite of Plague Inc., whose goal is extinction [K]. Here the score is
**your young raised, times the host still standing**, which is the cuckoo's
logic. Three grades show at the end of each life:

- **Cuckoo**: you took food the colony could spare; its population held.
- **Thief**: you took more; the brood thinned but recovered when you left.
- **Predator**: you ate brood; the colony shrank, and so did your yield.

The colony must recover when the guest leaves (critique §5). A host that
collapses to zero is a design failure, not a player win.

---

## Building it

### Fit with each owner rule

- **One brain, no castes**: the brain is untouched. But the seat adds body
  rules that decide behaviour, which is hand-made by this project's standard.
- **Dig and forage all game**: the herd drives foraging; the beggar touches
  neither, which is its weak fit.
- **Graded and visible**: ignore → pause → feed, or display → nip → drag →
  kill, each on an ant's body.
- **Realistic, one physics, features on**: documented biology; honeydew is a
  liquid. A guest-kin switch would be a new default, so it must measure
  neutral on a box with no guest.
- **Not "follow me"**: you never lead. The colony comes to you, or kills you.

### Build steps, in order, each with what it proves

0. **Look at the gate before building.** Drop a beetle-shaped body into a
   grown lab colony and count Feed and Attack options offered on it, and bites
   landed (expected: Feed zero, Attack on every touch, `options.rs:877-907`).
   Then give it the host's colony number and count again: Feed should be
   offered as to any poorer nestmate (`options.rs:977-981` admits any
   creature), and Attack never; whether the share act completes is untested.
   *Proves*: which route is cheap. Owner chooses: (a) give the guest the
   host's colony number, so the mind feeds it and never bites it, but the
   disguise means nothing to the mind; (b) retrain the mind's kin sense on
   scent (a pool retrain); or (c) keep the mind colony-gated, put every guest
   reaction in body rules, and let the ladder decide whether the mind's
   colony-number bite may land on a guest (this doc's default).
1. **The guest body.** A small rigid body cut from `assets/species/beetle.ron`,
   with its own scent. Positive control: it is **bitten either way**, near or
   far in scent, because colony number decides. *Proves*: the baseline the
   rules must move.
2. **The guest-kin switch.** A guest-only exception to the species gate, the
   host judging by its own tolerance. *Proves*: the kin test sees the guest,
   with a test beside `a_beetle_is_never_family_unless_its_kind_crosses_kinds`
   [E `creature.rs:49601`].
3. **The inspection ladder.** *Proves*: the middle exists between ignored and
   killed. Count each rung. Cover nestmates only if the owner wants the drift
   scenes.
4. **Rub, soak and beg.** Host-judged soak, symmetric rub, nurse-side share.
   *Proves*: a disguise can be earned and pays. Count blends and feeds; show
   the rubbed worker's drift.
5. **The herd.** First run the worker-level trace of trail B. Then honeydew,
   the herd body and a tending rule. *Proves*: foraging follows a moved food
   source, or names why not.
6. **The arms race.** 24 seeds, guest against no-guest, ten generations, the
   tolerance spread printed. *Proves*: the lineage clock shows, or it is
   dropped from the pitch.
7. **The memory layer (plan §7)**, only after the rest plays well.

### Frame cost

**The guest is one to twenty bodies, so its cost is small.** The kin test is
three subtractions per contact [E `creature.rs:11486`]. The ladder runs only
on contact with a non-nestmate, a handful of checks a tick. Honeydew adds a few
hundred liquid cells. Two costs to watch. **If the ladder runs between
nestmates**, it is a check on every ant contact. **A drifted colony is a scent
cloud**, and the census pairing, once 28–35% of a grown colony's frame, scales
with ants outside the cloud's core [E `world.rs:10250-10264`]. Quote
`examples/ascii.rs` worst-frame before and after step 3.

### The cheapest faked ten-minute slice

**Script the colony's reactions by hand and see if the con is fun.** No
learning, no evolution, no honeydew physics:

- The player drives a beetle body (reuse `sim::steer`'s keyboard hooks, which
  already replace a body's outputs [E `steer.rs:1-30`]).
- On contact, a hand-written state machine picks the reaction from the
  distance between the beetle's ring value and the ant's tolerance, plus a
  dice roll: walk on, pause, jerk, nip, drag.
- Rub moves the ring value 0.1 toward the colony per second. Beg in the pile
  transfers a fixed meal. Eating a larva spawns a corpse and an alarm puff,
  and raises the odds of every nearby ant for a minute.
- Run it in the lab box with a grown colony. Post a review card per run with
  the counts in `meta`: *feeds, inspections, nips, drags, larvae eaten, your
  young*.
- Pair it with a faked herd as a blind A/B card. The herd gets two arms:
  scripted tenders that walk to the herd while it is sweet, and **today's
  engine with the herd moved**, so the owner sees the gap.

**What it answers**: whether conning a crowd of ants is fun in the hand, and
which guest the owner prefers. Whether the colony can learn is steps 3 to 6.

---

## Risks and open questions

### Biggest risks to fun

- **The seat is about you, not the colony.** A guest teaches tolerance, care
  and suspicion; only the herd teaches foraging. That may be too narrow.
- **Parasitism fights the owner's colony**, and **the arms race may be
  invisible** (section 7, items 4 to 6).

### Biggest risks to feasibility

- **Everything rests on new body rules.** Today's mind ants never feed a
  guest and may bite it by colony number. The ladder, the share and the soak
  are all new, hand-made, and must override that bite.
- **The species gate is guarded by a test and an owner ruling.** Loosening it,
  even for guests, touches every kin check: mouth, eye, nursing, nest blend.
- **Trail B is untraced.** The herd's road depends on it.
- **Odour drift could destabilise the colony.** Symmetric rubbing pulls
  workers toward the guest; it needs a cap.

### Decisions only the owner can make

1. **Is a parasite seat welcome in the lab, or only in a side box?**
2. **May a guest be family? A precondition, not an open question.** Beg, rub,
   soak and the whole ladder need it. Options: a guest-only exception to the
   species gate, or `kin_crosses_kinds` on ants (off for every shipped kind,
   flagged as the owner's call [E `organism.rs:4282-4293`]).
3. **Which route through the mind**: host's colony number, a retrained
   scent-reading kin sense, or body rules only (build step 0).
4. **Cruelty.** Eating brood, drowning larvae by rescue priority and the
   wasp's discord scent are real biology. Which ship?
5. **Body rules versus mind.** Are contact rules like nursing by touch
   acceptable for the ladder and the share, or must every reaction evolve?
6. **Is the guest a main seat, a mode, or a graft** onto the keeper (below)?

### The gardener as symbiont (perspectives.md §7)

**Give the keeper gnome the guest's odour model, and the colony's relationship
with him becomes graded and readable with no trust meter.** The gnome gets a
scent vector (new), and the same ladder reads it:

- **Rubbing nest soil on his boots** soaks his scent toward the nest (critique
  §5's "becoming a nestmate").
- **Keeping a herd** on the plants he grows pays the colony sugar, and may
  pull its traffic to his beds.
- **Digging up the nest or flinging ants** leaves alarm on his hands, so ants
  near him display and nip.

The ladder the player reads: ignore him → walk past → tend his herd → guard
his plants → swarm his boots. It reuses steps 2 to 4, with no new brain input.
**It is the cheapest way to give the keeper a relationship; I would build it
even if the guest never becomes a seat.** Grafted on, the guest supplies what
the keeper lacks, a colony that notices *you*. The keeper supplies what the
guest lacks: teaching the colony to dig, farm and forage.

### What the review changed

1. Mind judges by colony number, Feed is colony-gated: **fixed** (pitch, step 0, decision 3).
2. Species gate makes "family: Today? Yes" false: **fixed** (now decision 2, a precondition).
3. Mind ants never react to strangers: **partly reversed by item 13** (no [E] in the walkthrough; the stranger-touch alarm is indeed skipped, but the mind's own bite is live).
4. Soak judged by the visitor; rub is `blend_with_kin` and symmetric: **fixed**, and the symmetric drift is now a visible cost and a surprise scene.
5. Beg harder, Sing, brood registration and limp-carry are not small: **fixed** (Sing cut; beg is a nurse-side share rule; carry marked new).
6. Bloodline overclaimed: **fixed** (a hypothesis, with the spread arithmetic and a first check).
7. Pseudogynes outdated: **fixed** (marked as a dropped hypothesis; scene cut). Checked: AntWiki confirms.
8. "Six times more" misread: **fixed** (nest capacity, not per animal).
9. Rain World and Plague Inc. references: **fixed** (Rogue Legacy / Crusader Kings; Plague Inc. as the opposite).
10. Contradictions (cheapness, suspicion halo, Cry "now"): **fixed**.
11. Herd loop waits on an untested signal: **fixed** (trace before step 5; honest arm in the faked slice; tending rule named).
12. Line numbers: **fixed** (drift `creature.rs:11653`; jump `creature.rs:28654`).
13. Later correction (enemy seat's author, checked here): trained lab ants **can** bite. Every `std_pool.txt` member has the Attack switch on, Attack is offered against any other-colony animal, and the comment at `options.rs:395` is stale. **Fixed** in the pitch, codes, lesson table, kill list, steps 0 and 1, and feasibility: a guest today is bitten by colony number, not ignored.

Disputed: none in substance. One addition to item 1: a third route, keeping the mind colony-gated and doing every guest reaction in body rules, is this doc's default, since nursing already works that way.

---

## Sources

**Web, read this session (search summaries and abstracts, not full papers):**

- Lomechusa/Atemeles: [Wikipedia, Lomechusa pubicollis](https://en.wikipedia.org/wiki/Lomechusa_pubicollis); [Hölldobler 1967 abstract](https://experts.azregents.edu/en/publications/zur-physiologie-der-gast-wirt-beziehungen-myrmecophilie-bei-ameis/); [Lomechusoides strumosus glands, PMC6059432](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC6059432/)
- Pseudogynes: [UK Beetles](https://www.ukbeetles.co.uk/lomechusoides-strumosus); [AntWiki, Labial Gland Disease](https://www.antwiki.org/wiki/Labial_Gland_Disease) (search summary)
- Maculinea: [Thomas & Wardlaw 1992, Springer](https://link.springer.com/doi/10.1007/BF00317247); [PMC3981827](https://pmc.ncbi.nlm.nih.gov/articles/PMC3981827); [Mimetic host shifts dataset](https://ckan.publishing.service.gov.uk/dataset/mimetic-host-shifts-in-a-social-parasite-of-ants-caterpillar-survivors-in-myrmica-nests); [PMC6367357](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC6367357/)
- Ichneumon eumerus: [The Scientist](https://www.the-scientist.com/wily-wasps-ferment-ant-civil-war-53237); [Wikipedia](https://en.wikipedia.org/wiki/Ichneumon_eumerus)
- Paussus: [Di Giulio et al. 2015, PMC4496082](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC4496082/)
- Ant crickets: [Wikipedia, Ant cricket](https://en.wikipedia.org/wiki/Ant_cricket); [Kyoto RISH seminar](https://www.rish.kyoto-u.ac.jp/logos/wp-content/uploads/2018/02/Open_Seminar_RISH_20180327_PoWei-HSU.pdf)
- Platyarthrus and Atelura: [Wikipedia, Platyarthrus](https://en.wikipedia.org/wiki/Platyarthrus_hoffmannseggii); [Scholtz 2021](https://www.biologie.hu-berlin.de/de/gruppenseiten/compzool/res/publications/scholtz-2021-atelura.pdf/@@download/file/Scholtz%202021%20Atelura.pdf)
- Aphids eaten by tenders: [Offenberg 2001](https://antbase.net/pdf/offenberg-2001.pdf); [bioRxiv 2024 preprint](https://www.biorxiv.org/content/10.1101/2024.05.02.592168.full.pdf)
- Treehoppers: [Wikipedia, Treehopper](https://en.wikipedia.org/wiki/Treehopper); [Morales 2000](https://mutualism.williams.edu/wp-content/uploads/pdf/ecology81_482.pdf); [Ecosphere 2022](https://doaj.org/article/9f7c435deff1449fb6b1c35c3c98189c)
- Lasius flavus root aphid eggs: [PMC12308343](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC12308343/)
- Acropyga: [Wikipedia, Acropyga epedana](https://en.wikipedia.org/wiki/Acropyga_epedana); [Smith et al. 2007](https://complete.bioone.org/journals/journal-of-insect-science/volume-7/issue-11/031.007.1101/First-Recorded-Mating-Flight-of-the-Hypogeic-Ant-Acropyga-epedana/10.1673/031.007.1101.full)
- Lycaenid dopamine: [Pierce Lab, Hojo et al. 2015](https://piercelab.oeb.harvard.edu/publications/lycaenid-caterpillar-secretions-manipulate-attendant-ant-behavior); [Kobe University](https://www.kobe-u.ac.jp/en/news/article/2015_08_19_03/)
- Hitman disguises: [Hitman wiki, Disguise](https://hitman.fandom.com/wiki/Disguise); [Hitman (2016), Wikipedia](https://en.wikipedia.org/wiki/Hitman_(2016_video_game))
- Via perspectives.md's sources (not re-read): Barbero 2009, Nash 2008, Oliver 2007.

**Repository files relied on (read only):**

- `Reports/teaching-the-colony-2026-10-25.md` (§1, §2, §4, §7, §10); its folder's `engine-brief.md`, `perspectives.md`, `critique.md`, `queen.md` (skimmed); `guest.critique.md` (the review of this doc)
- `src/sim/creature.rs`, `src/sim/creature/options.rs`, `src/sim/creature/mind.rs`, `src/sim/organism.rs`, `src/sim/brood.rs`, `src/sim/contest.rs`, `src/sim/steer.rs`, `src/sim/world.rs` (lines cited inline)
- `assets/species/ant.ron`, `beetle.ron`, `hopper.ron`, `flitter.ron`
